"""graft sb's host-side broker for one run: the status.relay and git.fetch bindings (design §6.3, §8).

Usage: broker.py --context NAME --denials FILE [--status SOCK --workmux BIN [--pane PANE]]
                 [--git SOCK --ssh BIN [--key-file F] --remote PATTERN...]

status.relay: the run's `workmux` shim sends one JSON line, its argv. Only `set-window-status` and
  `register-agent` are forwarded, to host workmux, aimed at the pane that launched the run (its
  TMUX_PANE, read by sb at start, never from the run). Each argument is cut to printable text.
git.fetch: the run's ssh shim (GIT_SSH_COMMAND) sends one JSON line, {argv, protocol}; the broker
  parses ssh's arguments itself, allows only `git-upload-pack '<path>'` to a URL matching a bound
  remote pattern, answers "OK" or "ERR <why>", then runs host ssh (the agent's keys, or the
  binding's key file) and relays the stream. Push is from the host until phase 2.
It exits when sb's backend, its parent, is gone. Sockets are relative paths in the run directory
(AF_UNIX allows 108 bytes).
"""
import asyncio
import fnmatch
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from compose import normalise_origin, pattern_key  # noqa: E402

A = {"remote": []}
args = sys.argv[1:]
while args:
    k = args.pop(0)
    if k == "--remote":
        A["remote"].append(args.pop(0))
    else:
        A[k[2:]] = args.pop(0)
STATUS_ALLOW = ("set-window-status", "register-agent")
SSH_OPTS_WITH_VALUE = {"-p", "-o", "-l", "-i", "-F", "-J", "-c", "-m", "-b", "-E"}


def clean(s, n=200):
    return "".join(c for c in str(s) if c.isprintable())[:n]


def denied(what):
    if A.get("denials"):
        with open(A["denials"], "a") as f:
            f.write(clean(f"graft sandbox ({A.get('context')}): {what}", 500) + "\n")


async def status(r, w):
    try:
        argv = json.loads(await r.readline())
        ok = isinstance(argv, list) and argv and argv[0] in STATUS_ALLOW and len(argv) <= 8
        if ok and A.get("pane"):
            p = await asyncio.create_subprocess_exec(A["workmux"], *[clean(a) for a in argv],
                                                     env={**os.environ, "TMUX_PANE": A["pane"]},
                                                     stdin=asyncio.subprocess.DEVNULL, stdout=asyncio.subprocess.DEVNULL,
                                                     stderr=asyncio.subprocess.DEVNULL)
            await p.wait()
        w.write(b"OK\n" if ok else b"ERR only set-window-status and register-agent are relayed\n")
        await w.drain()
    except (ValueError, OSError):
        pass
    w.close()


def parse_ssh(argv):
    """(user@host, port, command) from ssh's arguments as git gives them."""
    port, dest, i = None, None, 0
    while i < len(argv):
        a = argv[i]
        if dest is None and a in SSH_OPTS_WITH_VALUE:
            if a == "-p":
                port = argv[i + 1]
            i += 2
            continue
        if dest is None and a.startswith("-"):
            i += 1
            continue
        if dest is None:
            dest = a
            i += 1
            continue
        return dest, port, " ".join(argv[i:])
    raise ValueError("no command")


async def pipe(r, w, eof=None):
    try:
        while data := await r.read(65536):
            w.write(data)
            await w.drain()
    except (OSError, ConnectionError):
        pass
    finally:
        if eof:
            eof()
        else:
            w.close()


async def git(r, w):
    try:
        head = json.loads(await r.readline())
        dest, port, command = parse_ssh([str(a) for a in head["argv"]])
        m = re.fullmatch(r"git-upload-pack '([^']+)'", command)
        if not m:
            raise PermissionError(f"the run tried ssh {clean(dest)} {clean(command)}: only git-upload-pack (fetch) is brokered; push from the host")
        path = m.group(1) if m.group(1).startswith("/") else "/" + m.group(1)
        if not re.fullmatch(r"[A-Za-z0-9._-]+@[A-Za-z0-9.-]+|[A-Za-z0-9.-]+", dest) or (port and not port.isdigit()):
            raise PermissionError(f"the run tried ssh to {clean(dest)}: not a host")
        url = f"ssh://{dest}{':' + port if port else ''}{path}"
        key = normalise_origin(url)
        if not any(fnmatch.fnmatchcase(key, pattern_key(p)) for p in A["remote"]):
            raise PermissionError(f"the run tried git fetch {clean(url)} (not a bound git.fetch remote): sb why {clean(dest.split('@')[-1])}")
        proto = head.get("protocol") or ""
        env = dict(os.environ)
        if re.fullmatch(r"version=[0-9]", proto):
            env["GIT_PROTOCOL"] = proto
        cmd = [A["ssh"], "-o", "BatchMode=yes", "-o", "SendEnv=GIT_PROTOCOL"]
        if port:
            cmd += ["-p", port]
        if A.get("key-file"):
            cmd += ["-i", A["key-file"], "-o", "IdentitiesOnly=yes"]
        cmd += [dest, f"git-upload-pack '{path}'"]
        p = await asyncio.create_subprocess_exec(*cmd, env=env, stdin=asyncio.subprocess.PIPE,
                                                 stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.DEVNULL)
        w.write(b"OK\n")
        await w.drain()
        await asyncio.gather(pipe(r, p.stdin), pipe(p.stdout, w))
        await p.wait()
    except PermissionError as e:
        denied(str(e))
        w.write(f"ERR {e}\n".encode())
        await w.drain()
        w.close()
    except (ValueError, KeyError, TypeError, OSError):
        w.write(b"ERR malformed request\n")
        w.close()


async def orphaned(parent):
    while os.getppid() == parent:
        await asyncio.sleep(1)


async def main():
    parent = os.getppid()
    servers = []
    if A.get("status"):
        servers.append(await asyncio.start_unix_server(status, A["status"]))
    if A.get("git"):
        servers.append(await asyncio.start_unix_server(git, A["git"]))
    print("LISTENING", flush=True)
    await orphaned(parent)


asyncio.run(main())
