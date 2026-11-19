"""pot's proxy, on the host: a run's only way out to the network.

Usage: proxy.py LISTEN [--denials FILE --context NAME] [--tmux BIN --pane PANE] HOST:PORT...

LISTEN is a unix socket path (bubblewrap: the launcher binds it into the run, where socat serves it
as 127.0.0.1:3128), or tcp:ADDRESS:PORT (Seatbelt: 127.0.0.1, port 0 for a free one; the proxy
writes "LISTENING PORT" first, and the run's profile allows connecting to that port alone). It
forwards CONNECT requests (HTTPS) and absolute-form HTTP requests to the HOST:PORT pairs given (the
policy's network.allow, each host with its ports), and refuses the rest. A host may use fnmatch
patterns ("*.example.com"). Writes one line per request: ALLOW or DENY, then host:port. With
--denials, it also appends each newly refused host:port to FILE, once, as the message the run reads
(/run/pot/denials; an agent kit's hook passes new lines to the model). With --tmux,
it shows each new one on the launching pane with `tmux display-message` (never a write to the run's
terminal: that corrupts a full-screen program). It exits when
the launcher, its parent, is gone.

The host's loopback is never reached (v2.19.0): a name is resolved once, here, and refused when any address it
has is loopback or unspecified (127/8, ::1, 0/8, ::, and their IPv4-mapped forms), so the host's own name,
localhost. and a name that rebinds to 127.0.0.1 are refused like a literal; the connection goes to a checked
address. host-loopback:PORT, the form that would forward one port, is reserved for a later release.

POT_PROXY_TOKEN in its environment (Seatbelt: the port is on the host's loopback, where any local process
can connect): every request must carry Proxy-Authorization: Basic with that token as its password (the run's
proxy URL is http://pot:TOKEN@127.0.0.1:PORT); one without it is refused, 407, and logged NOTOKEN.
"""
import asyncio
import base64
import fnmatch
import hmac
import ipaddress
import os
import socket
import re
import sys
import urllib.parse


MAIN = __name__ == "__main__"
ARGS = sys.argv[2:] if MAIN else []
OPT = {}
while ARGS and ARGS[0] in ("--denials", "--context", "--tmux", "--pane", "--fix", "--upstream", "--nested"):
    OPT[ARGS[0][2:]] = ARGS[1]
    ARGS = ARGS[2:]
DENIALS, CONTEXT = OPT.get("denials"), OPT.get("context")
LISTEN, RULES = sys.argv[1] if MAIN else None, [e.rpartition(":")[::2] for e in ARGS]
DENIED = set()
TOKEN = os.environ.pop("POT_PROXY_TOKEN", None) if MAIN else None
ZERO = ipaddress.ip_network("0.0.0.0/8")


class Loopback(Exception):
    """A host that resolves to the host's own loopback (or an unspecified address): never connected to."""
    def __init__(self, address):
        super().__init__(address)
        self.address = address


def host_local(address):
    """An address on the host itself: loopback or unspecified, IPv4, IPv6 or IPv4-mapped."""
    a = ipaddress.ip_address(address.split("%", 1)[0])
    if a.version == 6 and a.ipv4_mapped is not None:
        a = a.ipv4_mapped
    return a.is_loopback or a.is_unspecified or (a.version == 4 and a in ZERO)


async def resolve(host, port, getaddrinfo=None):
    """The addresses to connect to, resolved once and checked: Loopback when any is the host's own."""
    infos = await (getaddrinfo or asyncio.get_running_loop().getaddrinfo)(host, port, type=socket.SOCK_STREAM)
    out = []
    for info in infos:
        a = info[4][0]
        if host_local(a):
            raise Loopback(a)
        if a not in out:
            out.append(a)
    return out


def loopback_text(context, what, address):
    return (f"pot ({context}): the run tried {what}, the host's loopback ({address}): a run cannot reach it "
            f"(host-loopback is reserved for a later release).\n")


def authorized(headers):
    """With a token: the request's Proxy-Authorization holds it (Basic, any user name)."""
    if TOKEN is None:
        return True
    for h in headers:
        k, _, v = h.partition(":")
        if k.strip().lower() != "proxy-authorization":
            continue
        scheme, _, cred = v.strip().partition(" ")
        if scheme.lower() != "basic":
            continue
        try:
            _, _, pw = base64.b64decode(cred.strip(), validate=True).decode("utf-8").partition(":")
        except ValueError:
            continue
        if hmac.compare_digest(pw.encode(), TOKEN.encode()):
            return True
    return False
# a host name, an IPv4 literal, or an IPv6 literal in brackets: nothing else is looked up, or logged
HOST = re.compile(r"[A-Za-z0-9_.-]+|\[?[0-9A-Fa-f:.]+\]?")


def denied(what, loopback=None):
    """The run's own record of a refusal: one line per new host:port, never the raw bytes."""
    if DENIALS is None or what in DENIED:
        return
    DENIED.add(what)
    host = what.rpartition(":")[0] if what != "(malformed)" else what
    with open(DENIALS, "a") as f:
        if loopback is not None:
            f.write(loopback_text(CONTEXT, what, loopback))
        elif what == "(malformed)":
            f.write(f"pot ({CONTEXT}): the run sent the proxy a malformed request\n")
        else:
            flag = host if what.endswith(":443") else what
            # compose.denial_text's form: what was refused, then the one-run and permanent fixes
            alts = [a for a in OPT.get("fix", "the repo kit").split("\n") if a]
            once = "" if OPT.get("nested") else f"  one run:   pot {'' if CONTEXT == 'work' else CONTEXT + ' '}--net {flag} …\n"
            f.write(f"pot ({CONTEXT}): the run tried {what} (not in this run).\n{once}"
                    f"  permanent: {alts[0]}\n" + "".join(f"             {a}\n" for a in alts[1:]))
    if OPT.get("tmux") and OPT.get("pane"):
        import subprocess
        why = "the host's loopback" if loopback is not None else "not in this run"
        subprocess.Popen([OPT["tmux"], "display-message", "-t", OPT["pane"], "-d", "5000",
                          f"pot ({CONTEXT}): the run tried {what} ({why}) · pot why {host}".replace("#", "##")],
                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def allowed(host, port):
    return any(fnmatch.fnmatchcase(host.lower(), h.lower()) and str(port) == p for h, p in RULES)


async def pipe(r, w):
    try:
        while data := await r.read(65536):
            w.write(data)
            await w.drain()
    except OSError:
        pass
    finally:
        w.close()


async def refuse(w, status):
    w.write(f"HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".encode())
    await w.drain()
    w.close()


async def handle(r, w):
    try:
        head = (await r.readuntil(b"\r\n\r\n")).decode("latin-1")
    except (asyncio.IncompleteReadError, asyncio.LimitOverrunError, OSError):
        w.close()
        return
    first, *headers = [h for h in head.split("\r\n") if h]
    if not authorized(headers):
        # not the run's (no token): nothing is looked up, nor written to the run's denials
        print("NOTOKEN", flush=True)     # not a DENY: the after event counts only the run's own
        w.write(b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"pot\"\r\n"
                b"Content-Length: 0\r\nConnection: close\r\n\r\n")
        await w.drain()
        w.close()
        return
    try:
        method, target, version = first.split()
        if method == "CONNECT":
            host, _, port = target.rpartition(":")
            port, forward = int(port), None
        else:
            # plain HTTP: the request in origin form, alone on its connection
            url = urllib.parse.urlsplit(target)
            if url.scheme != "http" or not url.hostname:
                raise ValueError(target)
            host, port = url.hostname, url.port or 80
            path = (url.path or "/") + (f"?{url.query}" if url.query else "")
            kept = [h for h in headers if not h.lower().startswith(("proxy-", "connection:"))]
            forward = "\r\n".join([f"{method} {path} {version}", *kept, "Connection: close", "", ""])
        if not HOST.fullmatch(host):
            raise ValueError(host)
    except ValueError:
        # never the raw bytes: the launcher prints this log on the host's terminal
        print("DENY (malformed)", flush=True)
        denied("(malformed)")
        await refuse(w, "400 Bad Request")
        return
    ok = allowed(host, port)
    print(f"{'ALLOW' if ok else 'DENY'} {host}:{port}", flush=True)
    if not ok:
        denied(f"{host}:{port}")
        await refuse(w, "403 Forbidden (pot: not in this run)")
        return
    try:
        if OPT.get("upstream"):
            # a nested child's proxy (v2.18.0): out through the parent's proxy, which allows what the parent reaches
            ur, uw = await asyncio.open_unix_connection(OPT["upstream"])
            uw.write(f"CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n".encode())
            await uw.drain()
            head = await ur.readuntil(b"\r\n\r\n")
            if not head.split(b" ", 2)[1:2] == [b"200"]:
                raise OSError("upstream refused")
        else:
            # resolved once and checked; the connection goes to a checked address (no second lookup to rebind)
            ur = None
            for a in await resolve(host.strip("[]"), port):
                try:
                    ur, uw = await asyncio.open_connection(a, port)
                    break
                except OSError:
                    continue
            if ur is None:
                raise OSError("no address connected")
    except Loopback as e:
        # its own log word: the exit line says what was refused and why, with no --net fix (none applies)
        print(f"LOOPBACK {host}:{port} {e.address}", flush=True)
        denied(f"{host}:{port}", loopback=e.address)
        await refuse(w, "403 Forbidden (pot: the host's loopback)")
        return
    except (OSError, asyncio.IncompleteReadError, asyncio.LimitOverrunError):
        await refuse(w, "502 Bad Gateway")
        return
    if forward is None:
        w.write(b"HTTP/1.1 200 Connection established\r\n\r\n")
        await w.drain()
    else:
        uw.write(forward.encode("latin-1"))
    await asyncio.gather(pipe(r, uw), pipe(ur, w))


async def orphaned(parent):
    while os.getppid() == parent:
        await asyncio.sleep(1)


async def main():
    parent = os.getppid()
    if LISTEN.startswith("tcp:"):
        address, _, port = LISTEN[4:].rpartition(":")
        server = await asyncio.start_server(handle, address, int(port))
        print(f"LISTENING {server.sockets[0].getsockname()[1]}", flush=True)
    else:
        server = await asyncio.start_unix_server(handle, LISTEN)
    async with server:
        # the launcher's cleanup stops the proxy; when the launcher is killed, it stops itself
        await asyncio.wait([asyncio.ensure_future(server.serve_forever()), asyncio.ensure_future(orphaned(parent))],
                           return_when=asyncio.FIRST_COMPLETED)


if MAIN:
    asyncio.run(main())
