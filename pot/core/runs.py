"""pot: runs/1, a run's record, computed from its run log (runs/<id>/log.jsonl) alone.

The log is host-written; the record is never stored, so it always agrees with the events. States:
running, exited, stopped (exit 125), review (a clone's commits, neither fetched nor discarded) and pending
(reserved for phase 3, never written). Pure functions here; pot.py reads the files and the processes.
"""
import datetime
import json
import os
import re

RECORD_VERSION = 1


def read_log(path):
    """The events of one log.jsonl; a torn last line (a crash while writing) is skipped."""
    out = []
    try:
        with open(path) as f:
            for line in f:
                try:
                    e = json.loads(line)
                except ValueError:
                    continue
                if isinstance(e, dict) and isinstance(e.get("ev"), str):
                    out.append(e)
    except OSError:
        pass
    return out


def first(events, ev):
    return next((e for e in events if e.get("ev") == ev), None)


def last(events, ev):
    return next((e for e in reversed(events) if e.get("ev") == ev), None)


def ended(events):
    return any(e.get("ev") in ("exit", "stop") for e in events)


def needs_after(events, alive):
    """A clone run whose pot died before its after event: the next reader completes the staging."""
    s = first(events, "start")
    if not s or s.get("mode") != "clone" or first(events, "after") or first(events, "stop") or first(events, "discard"):
        return False
    return not alive(s)


def plural(n, word):
    return f"{n} {word}" + ("" if n == 1 else "s")


def attention(after, fetched=False):
    """The ATTENTION items (design §9): the host's after findings. A commit count alone is not one; unsigned
    commits are, when the host signs, until they are fetched."""
    out = []
    if not after:
        return out
    if after.get("fetch_failed"):
        out.append("fetch failed: " + after["fetch_failed"])
    if after.get("unsigned") and after.get("signs") and not fetched:
        out.append(f"{after['unsigned']} unsigned")
    if after.get("root_attention"):
        out.append("root: " + ", ".join(after["root_attention"]))
    elif after.get("root_repo_change"):
        out.append("root changed")
    if after.get("touches_host_run"):
        out.append("host-run: " + ", ".join(after["touches_host_run"][:4]))
    if after.get("repo_kit_change"):
        out.append("repo kit: " + ", ".join(after["repo_kit_change"][:4]))
    if after.get("gitlinks"):
        out.append("gitlinks: " + ", ".join(after["gitlinks"][:4]))
    if after.get("new_git_metadata"):
        out.append(plural(len(after["new_git_metadata"]), "new git metadata path"))
    d = after.get("denials") or []
    if d:
        out.append(f"{sum(x.get('count', 1) for x in d)} denied: " + ", ".join(x["host"] for x in d[:3]))
    return out


LOST = "pot ended without an exit (killed or crashed)"


def note(rec):
    """pot runs' NOTE column."""
    c = rec.get("clone") or {}
    if rec["state"] == "review" and not (c.get("commits", 0) + c.get("scionCommits", 0)):
        return f"fetch failed · pot review {rec['id']}"
    if rec["state"] == "review":
        n = c.get("commits", 0) + c.get("scionCommits", 0)
        return f"{plural(n, 'commit')} · pot review {rec['id']}"
    if rec["state"] == "stopped":
        if (rec.get("stop") or {}).get("grantedSince"):
            return "stopped · granted since: start it again"
        return (rec.get("stop") or {}).get("fix") or "stopped"
    if rec["state"] == "running":
        return "running"
    if c.get("discarded"):
        return "discarded" + (" (pot ended without an exit: killed or crashed)" if rec["lost"] else "")
    if rec["lost"]:
        return "lost: " + LOST
    if c.get("fetched"):
        return f"exit {rec['exit']} · fetched"
    return f"exit {rec['exit']}"


def workspace_name(rec):
    """One rule for a workspace's name: the one its start event records (the origin's last path part, else
    the checkout's folder: compose.repo_short_name); a log written without one shows its folder."""
    w = rec.get("workspace") or []
    return " ".join(x.get("name") or os.path.basename(x["path"]) for x in w) if w else "-"


def record(rid, events, alive):
    """The runs/1 record of run RID. alive(start event) says whether its process still runs."""
    s = first(events, "start") or {}
    ex = last(events, "exit")
    stop = last(events, "stop")
    af = last(events, "after")
    fetch = last(events, "fetch")
    disc = last(events, "discard")
    mode = s.get("mode") or "in-place"
    clone = None
    if mode == "clone" or s.get("clone"):
        c = s.get("clone") or {}
        commits = (af or {}).get("commits") or []
        clone = {"base": c.get("base") or (af or {}).get("base"), "branch": c.get("branch"),
                 "commits": len(commits), "scionCommits": sum(len(v.get("commits") or []) for v in ((af or {}).get("scions") or {}).values()),
                 "unsigned": (af or {}).get("unsigned", 0),
                 "refs": (af or {}).get("refs") or {}, "uncommitted": (af or {}).get("uncommitted"),
                 "nested": c.get("nested") or [],
                 "size": (af or {}).get("size"), "fetched": (fetch or {}).get("refs"),
                 "discarded": (disc or {}).get("time"), "keptUntil": None}
    if stop:
        state = "stopped"
    elif not ended(events) and s and alive(s):
        state = "running"
    elif clone and (clone["commits"] or clone["scionCommits"] or (af or {}).get("fetch_failed")) and not fetch and not disc:
        state = "review"
    else:
        state = "exited"
    lost = bool(s) and not stop and ex is None and state != "running"
    return {
        "version": RECORD_VERSION, "id": rid, "state": state, "context": s.get("context"), "mode": mode,
        "command": s.get("command"), "argv": s.get("argv") or [], "workspace": s.get("workspace") or [],
        "invoker": s.get("invoker"), "detached": bool(s.get("detached")), "pane": s.get("pane"),
        "parent": s.get("parent"), "started": s.get("time"), "ended": (ex or stop or {}).get("time"),
        "exit": (125 if stop else (ex or {}).get("status")), "lost": lost,
        "stop": {k: stop.get(k) for k in ("reason", "kits", "fix")} if stop else None,
        "clone": clone, "attention": attention(af, fetched=bool(fetch)), "children": [],
    }


DUR = re.compile(r"([0-9]+)([mhd])")


def parse_time(t, now=None):
    """--since/--until: 2026-11-17, 2026-11-17T14:00, or 30m, 2h, 3d before now."""
    now = now or datetime.datetime.now()
    m = DUR.fullmatch(t)
    if m:
        n, u = int(m.group(1)), m.group(2)
        return now - datetime.timedelta(**{{"m": "minutes", "h": "hours", "d": "days"}[u]: n})
    for fmt in ("%Y-%m-%d", "%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S"):
        try:
            return datetime.datetime.strptime(t, fmt)
        except ValueError:
            pass
    raise ValueError(f"{t}: a time is 2026-11-17, 2026-11-17T14:00, or 30m, 2h, 3d")


def when(t):
    try:
        return datetime.datetime.fromisoformat(t) if t else None
    except ValueError:
        return None


def process_alive(start):
    """Whether a start event's process still runs: the same pid, start time and boot."""
    pid = start.get("pid")
    if not isinstance(pid, int):
        return False
    if start.get("boot") and start["boot"] != boot_id():
        return False
    st = proc_start(pid)
    if st is None:
        return False
    return start.get("pidStart") in (None, st)


def boot_id():
    try:
        with open("/proc/sys/kernel/random/boot_id") as f:
            return f.read().strip()
    except OSError:
        return None


def proc_start(pid):
    """Field 22 of /proc/PID/stat, the process's start time in clock ticks; None when it is gone."""
    try:
        with open(f"/proc/{pid}/stat") as f:
            s = f.read()
    except OSError:
        return None
    try:
        return int(s[s.rindex(")") + 2:].split()[19])
    except (ValueError, IndexError):
        return None


def run_dirs(state):
    d = os.path.join(state, "runs")
    try:
        names = os.listdir(d)
    except OSError:
        return []
    return [(n, os.path.join(d, n)) for n in names if re.fullmatch(r"[0-9a-f]{4}", n)]
