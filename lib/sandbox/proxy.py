"""graft's sandbox proxy, on the host: a run's only way out to the network.

Usage: proxy.py LISTEN HOST:PORT...

LISTEN is a unix socket path (bubblewrap: the launcher binds it into the run, where socat serves it
as 127.0.0.1:3128), or tcp:ADDRESS:PORT (Seatbelt: 127.0.0.1, port 0 for a free one; the proxy
writes "LISTENING PORT" first, and the run's profile allows connecting to that port alone). It
forwards CONNECT requests (HTTPS) and absolute-form HTTP requests to the HOST:PORT pairs given (the
policy's network.allow, each host with its ports), and refuses the rest. A host may use fnmatch
patterns ("*.example.com"). Writes one line per request: ALLOW or DENY, then host:port. It exits
when the launcher, its parent, is gone.
"""
import asyncio
import fnmatch
import os
import re
import sys
import urllib.parse


LISTEN, RULES = sys.argv[1], [e.rpartition(":")[::2] for e in sys.argv[2:]]
# a host name, an IPv4 literal, or an IPv6 literal in brackets: nothing else is looked up, or logged
HOST = re.compile(r"[A-Za-z0-9_.-]+|\[?[0-9A-Fa-f:.]+\]?")


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
        await refuse(w, "400 Bad Request")
        return
    ok = allowed(host, port)
    print(f"{'ALLOW' if ok else 'DENY'} {host}:{port}", flush=True)
    if not ok:
        await refuse(w, "403 Forbidden (graft sandbox: not in network.allow)")
        return
    try:
        ur, uw = await asyncio.open_connection(host.strip("[]"), port)
    except OSError:
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


asyncio.run(main())
