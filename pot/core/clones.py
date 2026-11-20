"""pot: clone mode on the host (design §9): making a run's clone, keeping it (keepDays, maxSize), and taking
its commits back (pot review RUN --fetch, --discard).

Host git here runs only in the person's own repository and in folders pot creates, never in a clone a run
has written (P4, §7.2): the run's commits reach staging through a sandboxed upload-pack (the backend's
after step), and --fetch copies them from staging. Every call is without the user's global and system
config, hooks or fsmonitor, so the host's filters (git-lfs's smudge) never run on a clone.
"""
import datetime
import os
import re
import shutil
import stat
import subprocess

import runs as R


def clean_env():
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1"}
    for k in [k for k in env if k.startswith("GIT_CONFIG_KEY_") or k.startswith("GIT_CONFIG_VALUE_") or k == "GIT_CONFIG_COUNT"]:
        del env[k]
    return env


def git_lines(s):
    """K1: git's output as lines, split at newlines only (str.splitlines also splits at U+0085, U+2028 and the
    other line breaks a run-chosen name may hold)."""
    return [l for l in s.split("\n") if l]


def git(gitbin, cwd, *args, check=False):
    """K1: bytes, decoded with surrogateescape; K3: literal pathspecs."""
    r = subprocess.run([gitbin, "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false", "-c", "safe.directory=*",
                        "-C", cwd] + list(args), capture_output=True, env={**clean_env(), "GIT_LITERAL_PATHSPECS": "1"})
    r.stdout = r.stdout.decode("utf-8", "surrogateescape")
    r.stderr = r.stderr.decode("utf-8", "surrogateescape")
    if check:
        r.check_returncode()
    return r


class CloneError(Exception):
    pass


def base_of(gitbin, top, ref):
    """--from REF (default HEAD), resolved in the source: (commit, branch for the clone)."""
    r = git(gitbin, top, "rev-parse", "--verify", "--quiet", f"{ref or 'HEAD'}^{{commit}}")
    if r.returncode != 0:
        raise CloneError(f"--from {ref}: not a commit in {top}")
    commit = r.stdout.strip()
    branch = None
    if not ref or ref == "HEAD":
        b = git(gitbin, top, "symbolic-ref", "--quiet", "--short", "HEAD").stdout.strip()
        branch = b or None
    elif git(gitbin, top, "show-ref", "--verify", "--quiet", f"refs/heads/{ref}").returncode == 0:
        branch = ref
    elif git(gitbin, top, "show-ref", "--verify", "--quiet", f"refs/remotes/{ref}").returncode == 0 and "/" in ref:
        branch = ref.split("/", 1)[1]
    return commit, branch


def dirty_count(gitbin, top):
    out = git(gitbin, top, "status", "--porcelain", "--ignore-submodules=dirty").stdout
    return len([l for l in out.splitlines() if l.strip()])


def safe_rel(p):
    """A .gitmodules path or name pot may use: relative, with no . or .. part (R9, G4)."""
    parts = p.split("/")
    return bool(p) and not p.startswith("/") and all(x not in ("", ".", "..") for x in parts)


def submodules_at(gitbin, top, commit):
    """The submodules .gitmodules names at COMMIT whose gitlink is a commit there: [{name, path, url, commit}]."""
    r = git(gitbin, top, "config", "--blob", f"{commit}:.gitmodules", "--get-regexp", r"^submodule\..*\.(path|url)$")
    subs = {}
    for line in git_lines(r.stdout):
        k, _, v = line.partition(" ")
        name, field = k[len("submodule."):].rsplit(".", 1)
        subs.setdefault(name, {"name": name})[field] = v
    out = []
    for s in subs.values():
        # R9, G4: the path is where pot clones to, the name where it clones from (.git/modules/NAME): neither may
        # leave the repository (git's own rule for names)
        if "path" not in s or not safe_rel(s["path"]) or not safe_rel(s["name"]):
            continue
        ls = git(gitbin, top, "ls-tree", commit, "--", s["path"]).stdout.split()
        if len(ls) >= 3 and ls[1] == "commit":
            out.append({**s, "commit": ls[2]})
    return sorted(out, key=lambda s: s["path"])


def drop_heads(gitbin, repo, keep=None):
    """Delete every local branch of a fresh clone but KEEP, so only refs a run moves come back."""
    out = git(gitbin, repo, "for-each-ref", "--format=%(refname)", "refs/heads").stdout.split()
    for ref in out:
        if ref != f"refs/heads/{keep}":
            git(gitbin, repo, "update-ref", "-d", ref)


def tracked_hooks(gitbin, top, base, value):
    """H9 (v2.18.0): the source's core.hooksPath, when it is a relative path naming a tracked folder at BASE (such as
    .githooks, protected in every run); else None. Only the run itself runs those hooks: every host git call on a
    clone passes -c core.hooksPath=/dev/null."""
    v = (value or "").strip().rstrip("/")
    if not v or v.startswith(("/", "~")) or ".." in v.split("/") or "\n" in v:
        return None
    r = git(gitbin, top, "ls-tree", "--format=%(objecttype)", base, "--", v)
    return v if r.returncode == 0 and r.stdout.strip() == "tree" else None


def make_clone(gitbin, top, common, origin, dest, base, branch, rid, user, hooks=None):
    """runs/<id>/clone: a full clone of TOP at BASE (no hard links), its submodules from the source's own
    module directories, carrying user.* and the raw origin URLs only (§9.1, §9.2)."""
    r = git(gitbin, os.path.dirname(dest), "clone", "-q", "--no-hardlinks", "--no-checkout", top, dest)
    if r.returncode != 0:
        raise CloneError(f"git clone failed: {r.stderr.strip()}")
    name = branch or f"pot-{rid}"
    if origin:
        git(gitbin, dest, "remote", "set-url", "origin", origin, check=True)
    else:
        git(gitbin, dest, "remote", "remove", "origin")
    for k, v in (user or {}).items():
        git(gitbin, dest, "config", f"user.{k}", v)
    if hooks:
        # set before the checkout: from here on the clone names hooks, and every host git call below (checkout,
        # update-ref, submodule) must keep core.hooksPath=/dev/null for them never to run on the host (ruling 7)
        git(gitbin, dest, "config", "core.hooksPath", hooks)
    r = git(gitbin, dest, "checkout", "-q", "-B", name, base)
    if r.returncode != 0:
        raise CloneError(f"git checkout of {base[:12]} failed: {r.stderr.strip()}")
    drop_heads(gitbin, dest, keep=name)     # F1: the clone has the base's branch only
    scions, nested = [], []
    subs = submodules_at(gitbin, dest, base)
    if subs:
        git(gitbin, dest, "submodule", "--quiet", "init")
    for s in subs:
        src = os.path.join(common, "modules", s["name"])
        if not os.path.isdir(src):
            continue      # the source never checked it out: left uninitialised, as there
        wt = os.path.join(dest, s["path"])
        if os.path.isdir(wt):
            os.rmdir(wt) if not os.listdir(wt) else None
        r = git(gitbin, dest, "clone", "-q", "--no-hardlinks", "--no-checkout", src, wt)
        if r.returncode != 0:
            continue
        git(gitbin, wt, "checkout", "-q", "--detach", s["commit"])
        drop_heads(gitbin, wt)                  # F1: a scion's clone has its gitlink only (a detached HEAD)
        if s.get("url"):
            git(gitbin, wt, "remote", "set-url", "origin", s["url"])
        scions.append({"name": s["name"], "path": s["path"], "commit": s["commit"], "objects": os.path.join(src, "objects")})
        if submodules_at(gitbin, wt, s["commit"]):
            nested.append(s["path"])
    if scions:
        # the submodules' git directories into .git/modules, linked by relative paths, so they resolve at the
        # original path inside the run
        git(gitbin, dest, "submodule", "--quiet", "absorbgitdirs")
    return {"base": base, "branch": name, "path": dest, "scions": scions, "nested": nested,
            "objects": os.path.join(common, "objects")}


def parse_size(s):
    s = str(s).strip()
    mult = {"K": 1 << 10, "M": 1 << 20, "G": 1 << 30, "T": 1 << 40}
    if s and s[-1].upper() in mult:
        return int(float(s[:-1]) * mult[s[-1].upper()])
    return int(s)


def human(n):
    for u in ("B", "KB", "MB", "GB", "TB"):
        if n < 1024 or u == "TB":
            return f"{n:.0f} {u}" if u == "B" else f"{n:.1f} {u}".replace(".0 ", " ")
        n /= 1024


def size_of(rundir, after):
    if after and isinstance(after.get("size"), int):
        return after["size"]
    n = 0
    for d in ("clone", "staging"):
        for root, _, fs in os.walk(os.path.join(rundir, d)):
            for f in fs:
                try:
                    n += os.lstat(os.path.join(root, f)).st_blocks * 512
                except OSError:
                    pass
    return n


def writable_tree(top):
    """Make every real folder below TOP writable (the run may have left read-only ones), never through a link: each
    folder is opened with O_NOFOLLOW | O_DIRECTORY and changed with fchmod, and links are never entered (F4/R1)."""
    stack = [top]
    while stack:
        d = stack.pop()
        try:
            fd = os.open(d, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        except OSError:
            continue        # a link, or gone: never followed
        try:
            os.fchmod(fd, 0o700)
            with os.scandir(fd) as it:
                for e in it:
                    if e.is_dir(follow_symlinks=False):
                        stack.append(os.path.join(d, e.name))
        except OSError:
            pass
        finally:
            os.close(fd)


def remove_clone(state, rundir):
    """clone/ and staging/ of one run, only under pot's own runs folder; links inside are removed, never followed."""
    runs = os.path.join(os.path.realpath(state), "runs") + "/"
    rd = os.path.realpath(rundir)
    if not rd.startswith(runs) or os.path.dirname(rd) + "/" != runs:
        raise CloneError(f"refusing to remove {rundir}: not a run folder of pot's state")
    for d in ("clone", "staging"):
        p = os.path.join(rd, d)
        if os.path.islink(p):
            os.unlink(p)
            continue
        if os.path.lexists(p):
            shutil.rmtree(p, ignore_errors=True)       # rmtree unlinks links, never follows them
            if os.path.lexists(p):
                writable_tree(p)
                shutil.rmtree(p, ignore_errors=True)


def writable_at(dfd, name):
    """writable_tree by directory fds: every real folder below DFD/NAME made writable, links never entered."""
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
    try:
        fd = os.open(name, flags, dir_fd=dfd)
    except OSError:
        return
    try:
        os.fchmod(fd, 0o700)
        with os.scandir(fd) as it:
            subs = [e.name for e in it if e.is_dir(follow_symlinks=False)]
        for e in subs:
            writable_at(fd, e)
    except OSError:
        pass
    finally:
        os.close(fd)


def remove_run_dir(state, rundir):
    """C1: a run folder with no log (the run was refused before its start), only under pot's own runs folder; links
    inside are removed, never followed."""
    runs = os.path.join(os.path.realpath(state), "runs") + "/"
    rd = os.path.realpath(rundir)
    if not rd.startswith(runs) or os.path.dirname(rd) + "/" != runs or os.path.islink(rundir):
        raise CloneError(f"refusing to remove {rundir}: not a run folder of pot's state")
    if os.path.lexists(os.path.join(rd, "log.jsonl")):
        raise CloneError(f"refusing to remove {rundir}: it has a log")
    shutil.rmtree(rd, ignore_errors=True)
    if os.path.lexists(rd):
        writable_tree(rd)
        shutil.rmtree(rd, ignore_errors=True)


def prune(state, clones_cfg, log, alive, now=None):
    """keepDays and maxSize (Daniel's rule: pot never deletes unfetched commits on its own): only clones that
    are fetched, discarded or hold no commits are removed. Returns the review runs to name when the clones
    still exceed maxSize: [(id, size)], oldest first, and the total."""
    now = now or datetime.datetime.now()
    keep = int(clones_cfg.get("keepDays", 14))
    cap = parse_size(clones_cfg.get("maxSize", "20G"))
    kept = []
    for rid, rd in R.run_dirs(state):
        if not os.path.isdir(os.path.join(rd, "clone")) and not os.path.isdir(os.path.join(rd, "staging")):
            continue
        ev = R.read_log(os.path.join(rd, "log.jsonl"))
        if not R.first(ev, "start"):
            continue    # R9: a clone being prepared (no start yet), never taken for an old one
        rec = R.record(rid, ev, alive)
        if rec["state"] == "running" or R.needs_after(ev, alive):
            continue
        end = R.when(rec["ended"] or rec["started"]) or now
        removable = rec["state"] != "review"
        size = size_of(rd, R.last(ev, "after"))
        if removable and now - end >= datetime.timedelta(days=keep):
            remove_clone(state, rd)
            log(rd, {"ev": "discard", "by": "host", "why": "keepDays", "time": now.isoformat(timespec="seconds")})
            continue
        kept.append((end, rid, rd, size, removable))
    kept.sort()
    total = sum(k[3] for k in kept)
    for end, rid, rd, size, removable in list(kept):
        if total <= cap:
            break
        if removable:
            remove_clone(state, rd)
            log(rd, {"ev": "discard", "by": "host", "why": "maxSize", "time": now.isoformat(timespec="seconds")})
            total -= size
            kept.remove((end, rid, rd, size, removable))
    over = [(rid, size) for _, rid, _, size, removable in kept if not removable] if total > cap else []
    return over, total, cap
