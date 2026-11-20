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
  - the socket closing (the client killed, its tool timeout, its SIGTERM): SIGTERM to the session, 10 s, SIGKILL;
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
GRACE = 10.0               # SIGTERM to SIGKILL: longer than a sibling's own cleanup (its backend's helpers, after, cleanup)
PENDING = threading.BoundedSemaphore(64)   # connections still sending their request: more are closed at once
live = {}                  # pid -> Popen
lock = threading.Lock()
closing = False            # set by ending(): no sibling starts after it


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
    """SIGTERM to the sibling's session, again each second (one that came while the sibling was starting is not
    lost: the re-review's M1), up to GRACE seconds; then SIGKILL."""
    end = time.time() + GRACE
    while p.poll() is None and time.time() < end:
        try:
            os.killpg(p.pid, signal.SIGTERM)
        except (ProcessLookupError, PermissionError):
            return
        for _ in range(20):
            if p.poll() is not None:
                break
            time.sleep(0.05)
    try:
        os.killpg(p.pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def read_head(c):
    """The request line and the fds that came with it, within HEAD_TIME and HEAD_MAX. On any failure the fds that
    came are closed (no leak from a partial or malformed request)."""
    c.settimeout(HEAD_TIME)
    end = time.time() + HEAD_TIME
    fds = []
    try:
        msg, fds, _, _ = socket.recv_fds(c, 65536, 3)
        buf = msg
        while b"\n" not in buf:
            if len(buf) > HEAD_MAX or time.time() > end or not msg:
                raise ValueError("no request")
            msg = c.recv(65536)
            buf += msg
        c.settimeout(None)
        line = buf.split(b"\n", 1)[0]
        if len(line) > HEAD_MAX:
            raise ValueError("a request over 256 KiB")
        return json.loads(line), fds
    except BaseException:
        for f in fds:
            os.close(f)
        raise


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
        except (ValueError, OSError, TypeError, RecursionError) as e:
            if isinstance(e, OSError) and not isinstance(e, ValueError):
                e = "no request in time" if isinstance(e, socket.timeout) else e.strerror or e
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
        req = {"parent": A["run"], "broker": os.getpid(), "key": head["key"], "args": head["args"], "cwd": head["cwd"],
               "hook": {k: v for k, v in (head.get("hook") or {}).items() if k in ("GIT_INDEX_FILE", "GIT_PREFIX")
                        and isinstance(v, str)}}
        with lock:
            if closing:
                os.close(r)
                os.close(w)
                answer(c, {"refused": f"pot ({A['context']}): {name}: this run is ending", "exit": 1})
                return
            # the entry holds the slot too (pass_fds), so the slot stays taken while it lives, whatever happens here
            try:
                p = subprocess.Popen(ENTRY + ["run", "--sibling", str(r)], stdin=fds[0], stdout=fds[1], stderr=fds[2],
                                     pass_fds=(r, slot), start_new_session=True, close_fds=True)
            except OSError as e:
                os.close(r)
                os.close(w)
                answer(c, {"refused": f"pot ({A['context']}): {name}: the host could not start it ({e.strerror})", "exit": 1})
                return
            live[p.pid] = p
        os.close(r)
        for f in fds:
            os.close(f)
        fds = []
        try:
            with os.fdopen(w, "wb") as f:
                f.write(json.dumps(req).encode())
        except OSError:
            pass            # the entry went first; its status comes back below
        log(f"start {name} (pid {p.pid})")
        gone = False
        # poll, not select: no ceiling on the fd's number (the review's F2); any failure here stops the sibling
        poller = select.poll()
        poller.register(c, select.POLLIN | select.POLLHUP | select.POLLERR)
        try:
            while p.poll() is None:
                if not poller.poll(100):
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
        except BaseException:
            gone = True
            stop(p)
            p.wait()
            with lock:
                live.pop(p.pid, None)
            raise
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
    """The run is ending: every sibling stops first, and none starts after this."""
    global closing
    with lock:
        closing = True
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
