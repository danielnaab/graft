"""pot's host-side broker for a run's siblings (design §10, v2.20.0; docs/reference/pot-siblings.md).

Usage: siblings.py --socket SOCK --run DIR --tree DIR --context NAME --entry JSON

The backend starts it beside the run's other bindings when the run's children table has a sibling row. A wrapper
in the run (the in-run `pot run --command`) connects to SOCK, sends one JSON line ({v, key, args, cwd, hook}) with
its fds 0-2 by SCM_RIGHTS, and blocks. The broker checks the request (its size and time, the fds' kinds, the row's
key in the run's table), takes one of the tree's slots (flocked files in --tree, outside every run folder, freed
when their holder ends however it ends), and starts the host entry (`pot run --sibling FD`, the JSON --entry) in
a session of its own, with the caller's fds 0-2 and the request on FD. Then:
  - a line `INT` from the client: SIGINT to the sibling's session (pot and the backend ignore it; the command's
    is the default again);
  - the socket closing (the client killed, its tool timeout, its SIGTERM): SIGTERM to the session, 2 s, SIGKILL;
  - the sibling ending: one JSON line back, {"exit": N}.
A refusal before the start is {"refused": TEXT, "exit": N}. On SIGTERM (the run is ending) the broker stops every
sibling first, the same way. It exits when the backend, its parent, is gone.
"""
import fcntl
import json
import os
import select
import signal
import socket
import stat
import subprocess
import sys
import threading
import time

A = {}
args = sys.argv[1:]
while args:
    k = args.pop(0)
    A[k[2:]] = args.pop(0)
HEAD_MAX = 256 << 10       # the request line
HEAD_TIME = 5.0            # seconds for the request to arrive
SLOTS = int(A.get("slots") or 16)      # decision 18: per tree
GRACE = 2.0
PENDING = threading.BoundedSemaphore(64)   # connections still sending their request: more are closed at once
live = {}                  # pid -> Popen
lock = threading.Lock()


def log(what):
    print("".join(c for c in str(what) if c.isprintable())[:500], file=sys.stderr, flush=True)


def null_rdev():
    try:
        return os.stat(os.devnull).st_rdev
    except OSError:
        return None


def crossable(fd):
    """A terminal, a pipe or FIFO, a socket, or /dev/null; never O_PATH (the design note: nothing whose path a
    sibling could reopen with its caller's authority)."""
    try:
        st = os.fstat(fd)
        if fcntl.fcntl(fd, fcntl.F_GETFL) & getattr(os, "O_PATH", 0):
            return False
    except OSError:
        return False
    if stat.S_ISFIFO(st.st_mode) or stat.S_ISSOCK(st.st_mode):
        return True
    if stat.S_ISCHR(st.st_mode):
        return os.isatty(fd) or st.st_rdev == null_rdev()
    return False


def take_slot():
    """One of the tree's SLOTS, as a held flock (the fd), or None when all are taken."""
    d = A["tree"]
    os.makedirs(d, mode=0o700, exist_ok=True)
    for n in range(SLOTS):
        try:
            fd = os.open(os.path.join(d, f"slot-{n}"), os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
        except OSError:
            continue
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return fd
        except OSError:
            os.close(fd)
    return None


def answer(c, obj):
    try:
        c.sendall(json.dumps(obj).encode() + b"\n")
    except OSError:
        pass


def stop(p):
    """SIGTERM to the sibling's session, GRACE seconds, then SIGKILL."""
    try:
        os.killpg(p.pid, signal.SIGTERM)
    except (ProcessLookupError, PermissionError):
        return
    end = time.time() + GRACE
    while time.time() < end and p.poll() is None:
        time.sleep(0.05)
    try:
        os.killpg(p.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def read_head(c):
    """The request line and the fds that came with it, within HEAD_TIME and HEAD_MAX."""
    c.settimeout(HEAD_TIME)
    end = time.time() + HEAD_TIME
    msg, fds, _, _ = socket.recv_fds(c, 65536, 3)
    buf = msg
    while b"\n" not in buf:
        if len(buf) > HEAD_MAX or time.time() > end or not msg:
            for f in fds:
                os.close(f)
            raise ValueError("no request")
        msg = c.recv(65536)
        buf += msg
    c.settimeout(None)
    line = buf.split(b"\n", 1)[0]
    if len(line) > HEAD_MAX:
        for f in fds:
            os.close(f)
        raise ValueError("a request over 256 KiB")
    return json.loads(line), fds


def serve(c):
    fds, slot = [], None
    if not PENDING.acquire(blocking=False):
        c.close()
        return
    try:
        try:
            try:
                head, fds = read_head(c)
            finally:
                PENDING.release()
            ok = isinstance(head, dict) and head.get("v") == 1 and isinstance(head.get("key"), str) \
                and isinstance(head.get("args"), list) and all(isinstance(a, str) for a in head["args"]) \
                and isinstance(head.get("cwd"), str) and isinstance(head.get("hook") or {}, dict)
            if not ok:
                raise ValueError("a malformed request")
        except (ValueError, OSError, TypeError) as e:
            log(f"refused: {e}")
            answer(c, {"refused": f"pot ({A['context']}): the run's broker refused the request ({e})", "exit": 2})
            return
        row = (TABLE.get(head["key"]) or {})
        name = row.get("command") or head["key"]
        if row.get("how") != "sibling":
            answer(c, {"refused": f"pot ({A['context']}): {name}: not a sibling row in this run's children table: run it on "
                                  "the host", "exit": 126})
            return
        if len(fds) != 3 or not all(crossable(f) for f in fds):
            answer(c, {"refused": f"pot ({A['context']}): {name}: fds 0-2 go to a sibling only as a terminal, a pipe, a socket "
                                  "or /dev/null", "exit": 2})
            return
        slot = take_slot()
        if slot is None:
            answer(c, {"refused": f"pot ({A['context']}): {name}: {SLOTS} commands already run in this run's tree (pot's cap): "
                                  "wait for one to end, or run it on the host", "exit": 1})
            return
        r, w = os.pipe()
        req = {"parent": A["run"], "key": head["key"], "args": head["args"], "cwd": head["cwd"],
               "hook": {k: v for k, v in (head.get("hook") or {}).items() if k in ("GIT_INDEX_FILE", "GIT_PREFIX")
                        and isinstance(v, str)}}
        p = subprocess.Popen(ENTRY + ["run", "--sibling", str(r)], stdin=fds[0], stdout=fds[1], stderr=fds[2],
                             pass_fds=(r,), start_new_session=True, close_fds=True)
        os.close(r)
        for f in fds:
            os.close(f)
        fds = []
        with os.fdopen(w, "wb") as f:
            f.write(json.dumps(req).encode())
        with lock:
            live[p.pid] = p
        log(f"start {name} (pid {p.pid})")
        gone = False
        while p.poll() is None:
            rd, _, _ = select.select([c], [], [], 0.1)
            if not rd:
                continue
            try:
                d = c.recv(64)
            except OSError:
                d = b""
            if not d:
                log(f"stop {name} (pid {p.pid}): the request went away")
                gone = True
                stop(p)
                break
            if b"INT" in d:
                try:
                    os.killpg(p.pid, signal.SIGINT)
                except (ProcessLookupError, PermissionError):
                    pass
        st = p.wait()
        with lock:
            live.pop(p.pid, None)
        log(f"exit {name} (pid {p.pid}): {st}")
        if not gone:
            answer(c, {"exit": 128 - st if st < 0 else st})
    finally:
        for f in fds:
            os.close(f)
        if slot is not None:
            os.close(slot)
        c.close()


def ending(*_):
    """The run is ending: every sibling stops first."""
    with lock:
        ps = list(live.values())
    ts = [threading.Thread(target=stop, args=(p,)) for p in ps]
    for t in ts:
        t.start()
    for t in ts:
        t.join()
    os._exit(0)


def orphaned(parent):
    while os.getppid() == parent:
        time.sleep(1)
    ending()


with open(os.path.join(A["run"], "policy.json")) as f:
    TABLE = json.load(f).get("children") or {}
ENTRY = json.loads(A["entry"])
signal.signal(signal.SIGTERM, ending)
signal.signal(signal.SIGINT, signal.SIG_IGN)
threading.Thread(target=orphaned, args=(os.getppid(),), daemon=True).start()
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.bind(A["socket"])
os.chmod(A["socket"], 0o600)
s.listen(SLOTS)
print("LISTENING", flush=True)
while True:
    try:
        conn, _ = s.accept()
    except InterruptedError:
        continue
    threading.Thread(target=serve, args=(conn,), daemon=True).start()
