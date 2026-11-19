"""pot: the Seatbelt backend, for macOS (design §11, Appendix B, §16 residual 12).

    backend probe|prepare|run|after|cleanup <run dir>

The same contract as bwrap.py: it reads <run dir>/policy.json and launch.json, and carries the policy
out with /usr/bin/sandbox-exec and a profile that denies by default. The profile is written at start,
from policy.json (v2.14.0's was built with the root): macOS's own needs (BASE), the closure, the run's
paths, then the deny rules nothing above may reopen. Every host path reaches the profile as a
parameter (-DPn=PATH, (param "Pn")), never as text in it, so no name can add a rule; store paths are
checked against a pattern and written as text.

Seatbelt has no mounts, so (the rework list, design §11):
- HOME is a new folder, <run dir>/home; links stand in for mounts (a state request's `at`, a file, a
  fixed directory, a ~/ read request), and deny rules keep each link from being removed or replaced.
- inspect is allow-then-deny: the whole file system read, then the home, temporary folders and the
  hidden list denied, then what inspect.home names allowed again, then the hidden list denied again.
- the run's folder is the run's /run/pot: POT_POLICY names <run dir>/policy.json, and /run/pot/… in
  the command's arguments is rewritten to it.
- no same-path clone: a dispatch clone runs at its own path (compose says so on the startup line).
- the network is a port on the host's loopback, the proxy's, plus a per-run token
  (http://pot:TOKEN@127.0.0.1:PORT); the profile allows connecting to that port alone.
- the run is its own process group; what is left of it when the command exits is stopped by group,
  then by its TMPDIR (a process that left the group with setsid), as v2.14.0 did.
- with the nix capability, builds run in the host's nix daemon, which on macOS runs them unsandboxed by
  default (stated on the startup line by pot).

Guarantees it proves (probe): closure-store no-tiocsti in-place private-proxy. It lacks own-netns
own-pidns protected-as-mounts local-listen same-path-clone nest resource-limits root-files
separate-kernel. Nesting is refused (pot inside a run on macOS refuses; a narrower child is never
flattened into its parent).

Residuals (design §16, 12): no process isolation (a run lists this user's processes, and Seatbelt does
not check KERN_PROCARGS2, so a run can read the arguments and environment of every process of the same
user, including credentials another program holds in its environment); sockets a run creates are on
the host's file system; a protected folder is a path rule, not a mount point.
"""
import json
import os
import re
import secrets
import shutil
import signal
import stat
import subprocess
import sys
import termios
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bwrap as B  # noqa: E402  (the backend-neutral parts: pins, the after event, staging, the run's own files)

SANDBOX_EXEC = "/usr/bin/sandbox-exec"
GUARANTEES = ["closure-store", "no-tiocsti", "in-place", "private-proxy"]
STORE_RE = re.compile(r"^/nix/store/[0-9a-z]{32}-[A-Za-z0-9+._?=-]+$")
# the run's own files in its folder, read-only in the run (never launch.json, log.jsonl, before.json or the logs)
OWN_FILES = ("policy.json", "launcher", "README.md", "denials", "explain.txt", "why.json", "mcp.json", "bash_env", "hidden.txt",
             "gitconfig")
OWN_DIRS = ("bin", "notfound")
Stop = B.Stop

# What a process needs from macOS, read-only, and nothing of the user's: no home, no /Users, no /tmp, no
# /Volumes, no other store path. v2.14.0's seatbelt.sb, from Nix's darwin build sandbox
# (sandbox-defaults.sb), OpenAI codex's seatbelt_base_policy.sbpl and Anthropic's sandbox-runtime, after
# Chromium's mac/common.sb.
BASE = r'''(version 1)
; pot's Seatbelt profile (pot/core/seatbelt.py), written at start from the run's policy.json
(deny default)

; processes: its own; no process-info and no signal toward a process outside the sandbox
(allow process-fork)
(allow signal (target same-sandbox))
(allow process-info* (target same-sandbox))

; sysctls that libc, Python, Node and Go read (no kern.proc.all: no listing every process)
(allow sysctl-read
  (sysctl-name "hw.activecpu") (sysctl-name "hw.busfrequency_compat") (sysctl-name "hw.byteorder")
  (sysctl-name "hw.cacheconfig") (sysctl-name "hw.cachelinesize_compat") (sysctl-name "hw.cpufamily")
  (sysctl-name "hw.cpufrequency") (sysctl-name "hw.cpufrequency_compat") (sysctl-name "hw.cputype")
  (sysctl-name "hw.l1dcachesize_compat") (sysctl-name "hw.l1icachesize_compat") (sysctl-name "hw.l2cachesize_compat")
  (sysctl-name "hw.l3cachesize_compat") (sysctl-name "hw.logicalcpu") (sysctl-name "hw.logicalcpu_max")
  (sysctl-name "hw.machine") (sysctl-name "hw.memsize") (sysctl-name "hw.model") (sysctl-name "hw.ncpu")
  (sysctl-name "hw.nperflevels") (sysctl-name "hw.packages") (sysctl-name "hw.pagesize")
  (sysctl-name "hw.pagesize_compat") (sysctl-name "hw.physicalcpu") (sysctl-name "hw.physicalcpu_max")
  (sysctl-name "hw.tbfrequency_compat") (sysctl-name "hw.vectorunit") (sysctl-name "kern.argmax")
  (sysctl-name "kern.hostname") (sysctl-name "kern.maxfiles") (sysctl-name "kern.maxfilesperproc")
  (sysctl-name "kern.maxproc") (sysctl-name "kern.ngroups") (sysctl-name "kern.osproductversion")
  (sysctl-name "kern.osrelease") (sysctl-name "kern.ostype") (sysctl-name "kern.osvariant_status")
  (sysctl-name "kern.osversion") (sysctl-name "kern.secure_kernel") (sysctl-name "kern.sysv.semmns")
  (sysctl-name "kern.usrstack64") (sysctl-name "kern.version") (sysctl-name "machdep.cpu.brand_string")
  (sysctl-name "machdep.ptrauth_enabled") (sysctl-name "sysctl.proc_cputype") (sysctl-name "vm.loadavg")
  (sysctl-name-prefix "hw.optional.") (sysctl-name-prefix "hw.perflevel") (sysctl-name-prefix "kern.proc.pgrp.")
  (sysctl-name-prefix "kern.proc.pid.") (sysctl-name-prefix "machdep.cpu.") (sysctl-name-prefix "net.routetable."))

; Mach services: getpwuid and group membership, confstr's user directories, logging, notify(3). No
; keychain (SecurityServer), no trustd, no DNS (mDNSResponder), no launch services, no pasteboard.
(allow mach-lookup
  (global-name "com.apple.system.opendirectoryd.libinfo")
  (global-name "com.apple.system.opendirectoryd.membership")
  (global-name "com.apple.bsd.dirhelper")
  (global-name "com.apple.system.logger")
  (global-name "com.apple.logd")
  (global-name "com.apple.system.notification_center"))
(allow ipc-posix-sem)
; AF_UNIX: socketpair(2) (libuv's child pipes). Binding or connecting to a socket path is network-bind
; and network-outbound, allowed below for the run's own sockets and pot's
(allow system-socket (socket-domain AF_UNIX))

; macOS, read-only
(allow file-read*
  (literal "/")
  (subpath "/usr/lib") (subpath "/usr/share") (subpath "/System/Library") (subpath "/System/Cryptexes")
  (subpath "/System/Volumes/Preboot/Cryptexes") (subpath "/Library/Apple") (subpath "/private/etc")
  (subpath "/private/var/db/timezone") (subpath "/private/var/db/dyld") (literal "/private/var/select/sh"))
; /bin/sh and /usr/bin/env, for scripts' first lines
(allow file-read* process-exec
  (literal "/bin/sh") (literal "/bin/bash") (literal "/bin/dash") (literal "/bin/zsh") (literal "/usr/bin/env"))
; what realpath(3) and getcwd(3) stat on the way (names and attributes only, no listing)
(allow file-read-metadata
  (literal "/bin") (literal "/dev") (literal "/etc") (literal "/Library") (literal "/nix") (literal "/nix/store")
  (literal "/private") (literal "/private/var") (literal "/private/var/db") (literal "/private/var/select")
  (literal "/System") (literal "/System/Volumes") (literal "/System/Volumes/Preboot") (literal "/tmp")
  (literal "/usr") (literal "/usr/bin") (literal "/var") (literal "/Users"))

; devices. ttyname(3) lists /dev and stats its entries: names and attributes only, no device's data
(allow file-read-data (literal "/dev"))
(allow file-read-metadata (subpath "/dev"))
(allow file-read* file-write-data file-ioctl (literal "/dev/null") (literal "/dev/zero") (literal "/dev/dtracehelper"))
(allow file-read* (literal "/dev/random") (literal "/dev/urandom"))
(allow file-read* file-write-data (literal "/dev/stdin") (literal "/dev/stdout") (literal "/dev/stderr") (subpath "/dev/fd"))
; the controlling terminal (the caller's terminal device is allowed by name below). Not every
; /dev/ttys*: another terminal of the same user could be read.
(allow file-read* file-write-data file-ioctl (literal "/dev/tty"))
; pseudo-terminals the run opens itself (script, tmux, a Node pty): new ones only
(allow pseudo-tty)
(allow file-read* file-write* file-ioctl (literal "/dev/ptmx"))
(allow file-read* file-write* file-ioctl (require-all (regex #"^/dev/ttys[0-9]+$") (extension "com.apple.sandbox.pty")))
'''


def real(p):
    """P as Seatbelt sees it: resolved (/tmp is /private/tmp), also when it does not exist yet."""
    return os.path.realpath(p)


class Profile:
    """The run's rules, after BASE: each path a parameter."""

    def __init__(self):
        self.lines, self.params, self.keys = [], [], {}

    def p(self, path):
        if path not in self.keys:
            self.keys[path] = f"P{len(self.keys)}"
            self.params.append(f"-D{self.keys[path]}={path}")
        return f'(param "{self.keys[path]}")'

    def rule(self, head, filters):
        if filters:
            self.lines.append(f"({head}\n  " + "\n  ".join(filters) + ")")

    def paths(self, head, kind, paths):
        self.rule(head, [f"({kind} {self.p(x)})" for x in dict.fromkeys(paths)])

    def note(self, text):
        self.lines.append("; " + text)

    def text(self):
        return BASE + "\n".join(self.lines) + "\n"


RUN_RULES = "; (the run's own rules, added at run)"


def store_rules(prof, paths):
    out = []
    for p in sorted(set(paths)):
        if not STORE_RE.match(p):
            raise Stop(f"not a store path, so the profile can't name it: {B.visible(p)}")
        out.append(f'(subpath "{p}")')
    prof.rule("allow file-read* process-exec", out)


def ancestors(paths):
    up = set()
    for p in paths:
        d = p
        while True:
            d = os.path.dirname(d)
            if d in ("", "/"):
                break
            up.add(d)
    return sorted(up)


def ttys():
    out = []
    for fd in (0, 1, 2):
        try:
            if os.isatty(fd):
                t = os.ttyname(fd)
                if t not in out:
                    out.append(t)
        except OSError:
            pass
    return out


def redirects():
    """The regular files on fds 0, 1 and 2 (`pot CMD > out.txt`): the caller chose them. The run writes them through
    the descriptor it inherits; some writes (GNU cat's) are checked against the file's path too, so the path is
    allowed: reading for fd 0, writing for 1 and 2. A protected file stays refused (its deny comes last)."""
    import fcntl
    out = {"read": [], "write": []}
    for fd in (0, 1, 2):
        try:
            if not stat.S_ISREG(os.fstat(fd).st_mode):
                continue
            p = os.fsdecode(fcntl.fcntl(fd, fcntl.F_GETPATH, bytes(1024)).rstrip(b"\0"))
        except (OSError, AttributeError):
            continue
        if p:
            out["read" if fd == 0 else "write"].append(p)
    return out


# ---------------------------------------------------------------- probe

def probe(cfg):
    """sandbox-exec works when an allowed command runs and a denied read is refused."""
    ok = subprocess.run([SANDBOX_EXEC, "-p", "(version 1)(allow default)", "/usr/bin/true"], capture_output=True, text=True)
    deny = subprocess.run([SANDBOX_EXEC, "-p", '(version 1)(allow default)(deny file-read* (literal "/private/etc/hosts"))',
                           "/bin/cat", "/etc/hosts"], capture_output=True, text=True)
    if ok.returncode != 0 or deny.returncode == 0:
        print(json.dumps({"guarantees": [], "error": (ok.stderr or deny.stderr or "a denied read was not refused").strip()}))
        return
    print(json.dumps({"guarantees": GUARANTEES}))


# ---------------------------------------------------------------- prepare

def load(rundir):
    policy, launch = B.load(rundir)
    policy["mounts"]["run"] = real(rundir)          # the run's /run/pot is its folder
    return policy, launch


def link(root, rel, target, made):
    """A link at ROOT/REL (ROOT: the run's HOME or a state directory) to TARGET: what Seatbelt has for a mount.
    Never through a link below ROOT (bwrap.safe_dest): a run writes its state, and an earlier run could have planted
    one there (P3); in HOME, a link pot made for a read request leads to a host folder."""
    # F4 (fix round 4): the folders safe_dest creates on the way are pot's too, kept as the link is (no rename,
    # removal or replacement by the run)
    parts = rel.strip("/").split("/")[:-1]
    new = [os.path.join(root, *parts[:i + 1]) for i in range(len(parts)) if not os.path.lexists(os.path.join(root, *parts[:i + 1]))]
    at = B.safe_dest(root, rel.strip("/"))
    made.extend(d for d in new if os.path.isdir(d) and not os.path.islink(d))
    if os.path.islink(at):
        os.unlink(at)
    elif os.path.lexists(at):
        kind = "folder" if os.path.isdir(at) else "file"
        raise Stop(f"{at} is a {kind} of the run's state, where the policy places {target}: move it out of the way "
                   "(the run's state is the host's to tidy), then run again")
    os.symlink(target, at)
    made.append(at)


def place(policy, home, runhome, value):
    """Where a ~/ entry goes: (root, rel), in the state directory whose `at` holds it, else in the run's HOME."""
    dest = home + value[1:]
    st = next((s for s in policy["mounts"]["state"] if dest.startswith(home + s["at"][1:] + "/")), None)
    if st:
        return real(st["dir"]), dest[len(home + st["at"][1:]) + 1:]
    return runhome, value[2:]


def in_dir(p, dirs):
    return any(p == d or p.startswith(d.rstrip("/") + "/") for d in dirs)


def prepare(cfg, rundir, print_cmd=False):
    policy, launch = load(rundir)
    rundir = real(rundir)
    B.EMPTY = os.path.join(rundir, "empty")
    open(B.EMPTY, "w").close()
    os.chmod(B.EMPTY, 0o444)
    home, runhome, runtmp = launch["home"], os.path.join(rundir, "home"), os.path.join(rundir, "tmp")
    for d in (runhome, runtmp):
        if os.path.lexists(d):
            shutil.rmtree(d) if os.path.isdir(d) and not os.path.islink(d) else os.unlink(d)
        os.mkdir(d, 0o700)
    view = next((i for i in B.eff(policy, "view")), None)
    hidden = [real(p) for i in B.eff(policy, "hidden") for p in i.get("paths", [])]
    net = [i for i in B.eff(policy, "network") if i["class"] == "reach"]
    prof = Profile()
    made = []                        # the links pot made: none may be removed or replaced
    keep = [runhome, runtmp]         # folders that may not be renamed, removed or replaced
    ro, ro_files = [], []            # protected: no write, no new hard link
    reads, writes = [], [runhome, runtmp]
    own_reads = []                   # read after the hidden list's denies (the run's own, granted explicitly)
    own_writes = []

    def home_link(value, target):
        # a ~/ entry of the policy, where a program looks for it: in the run's HOME
        if value.startswith("~/"):
            if not os.path.lexists(os.path.join(runhome, value[2:])):
                link(runhome, value[2:], target, made)

    # the store: the closure, or the whole store (the nix capability, and inspect)
    if view or B.binding(policy, "nix"):
        prof.note("the store, whole (inspect, or the nix capability: closure-store is dropped)")
        prof.rule("allow file-read* process-exec", ['(subpath "/nix/store")'])
        launch["closureCount"] = "the whole store"
    else:
        with open(cfg["baseClosure"]) as f:
            closure = set(launch.get("closure") or []) | {l.strip() for l in f if l.strip()}
        prof.note(f"the closure: {len(closure)} store paths")
        store_rules(prof, closure)
        launch["closureCount"] = len(closure)

    if view:
        # inspect (allow-then-deny): the file system read, then the home and temporary folders denied, then
        # inspect.home and the workspace allowed again; the hidden list and secret files are denied after
        prof.note("inspect: the whole file system, read-only, but the home and temporary folders")
        prof.rule("allow file-read* process-exec", ['(subpath "/")'])
        prof.paths("deny file-read* process-exec", "subpath",
                   [real(home), "/private/tmp", "/private/var/tmp", "/private/var/folders", "/Volumes"])
        for h in view["home"]:
            p = os.path.join(home, h[2:]) if h.startswith("~/") else h
            if os.path.exists(p):
                reads.append(real(p))
                home_link(h if h.startswith("~/") else "", p)
        if view.get("homeManager", True):
            for ln, target in B.hm_links(home):
                if not in_dir(ln, hidden):
                    rel = ln[len(home) + 1:]
                    if not os.path.lexists(os.path.join(runhome, rel)):
                        try:
                            link(runhome, rel, target, made)
                        except Stop:
                            pass     # below a read request's link: the host folder shows its own
        for w in policy["workspace"]:
            reads.append(real(w["path"]))
        t = time.time()
        found, _ = B.secret_walk(cfg, [r for r in reads if os.path.isdir(r)])
        found = [real(s) for s in found if not in_dir(real(s), hidden)]
        launch["walk"] = {"secrets": len(found), "sockets": 0, "seconds": round(time.time() - t, 2)}
    else:
        found = []
    for i in B.eff(policy, "read"):
        if os.path.exists(i["path"]):
            reads.append(real(i["path"]))
            home_link(i.get("value", ""), i["path"])
    for i in B.eff(policy, "write"):
        if os.path.exists(i["path"]):
            writes.append(real(i["path"]))
            keep.append(real(i["path"]))
            home_link(i.get("value", ""), i["path"])

    # the workspace
    pinsets, clones = [], []
    cwd = launch["cwd"]
    for w in policy["workspace"]:
        top, common = real(w["path"]), real(w["common"])
        if w.get("clone"):
            # §11: no same-path clone: the run works in the clone, at its own path
            src = real(w["clone"])
            pins = B.Pins(src, os.path.join(src, ".git"), w["protected"], cfg)
            pins.compute(os.path.join(src, ".git", "hooks"))
            clones.append((w, pins))
            own_writes.append(src)
            keep.append(src)
            if cwd == top or cwd.startswith(top + "/"):
                cwd = src + cwd[len(top):]
        elif view or policy["access"] != "write":
            reads += [top] + ([] if common.startswith(top + "/") else [common])
            continue
        else:
            pins = B.Pins(top, common, w["protected"], cfg)
            hooks = B.git(cfg, top, "rev-parse", "--path-format=absolute", "--git-path", "hooks").stdout.strip()
            pins.compute(hooks)
            pinsets.append(pins)
            writes += [top] + ([] if common.startswith(top + "/") else [common])
            keep += [top, common]
        keep += pins.dirs
        ro += pins.ro
        # a file below a protected folder with a second hard link: as for a protected file (Pins.compute)
        for d in [p for p in pins.ro if os.path.isdir(p) and not os.path.islink(p)]:
            for dp, _, fs in os.walk(d):
                for n in fs:
                    f = os.path.join(dp, n)
                    st = os.lstat(f)
                    if stat.S_ISREG(st.st_mode) and st.st_nlink > 1:
                        raise Stop(f"{pins.rel(f)} has another hard link, so a run could write it through that link (a "
                                   "run must not write what decides the sandbox or what the host runs): replace it "
                                   "with a copy of its own (cp -p F F.copy && mv F.copy F), then run again")
        ro_files += [p for p in pins.ro if os.path.isfile(p)]

    # state: a link in HOME at each request's `at`, to its folder
    state_dirs = []
    for s in policy["mounts"]["state"]:
        os.makedirs(s["dir"], exist_ok=True)
        d = real(s["dir"])
        state_dirs.append(d)
        own_writes.append(d)
        keep.append(d)
        if not os.path.lexists(runhome + s["at"][1:]):
            link(*place(policy, home, runhome, s["at"]), d, made)     # a nested state's, in the outer one
    # files: fixed ones a link (the run can't replace it); seeded and reasserted ones written into state
    ssl = cfg["cacert"]
    for i in B.eff(policy, "file"):
        v = i["value"]
        if v == "/etc/ssl/certs/ca-certificates.crt":
            ssl = i["source"]                      # the root's caBundle: SSL_CERT_FILE names it
            own_reads.append(real(i["source"]))
            continue
        if not v.startswith("~/"):
            raise Stop(f"{v}: Seatbelt can't place a file outside the run's home (root-files)")
        if i.get("mode", "fixed") != "fixed":
            B.into_state(policy, launch, i)
            continue
        link(*place(policy, home, runhome, v), i["source"], made)
        own_reads.append(real(i["source"]))
    # fixed directories (skills, agents, …): a folder of links in the run's folder, linked at its place
    for n, i in enumerate(B.eff(policy, "dir")):
        d = os.path.join(rundir, "fixed", str(n))
        os.makedirs(d, exist_ok=True)
        own_reads.append(d)
        for entry in i.get("links") or []:
            name, target = entry[0], entry[1]
            os.symlink(target, os.path.join(d, name))
            if not target.startswith("/nix/store/"):
                own_reads.append(real(target))
        v = i["value"]
        link(*place(policy, home, runhome, v), d, made)

    # pot's own: shims, the not-found handler, the MCP configuration; the run's .gitconfig and hidden list
    B.write_own(cfg, rundir, policy, launch)
    # the launcher pointer (v2.18.0): <run dir>/launcher, next to POT_POLICY, names this run's own pot, which a kit
    # command's wrapper runs inside (its children table: Seatbelt cannot nest, so a row runs in place or is refused);
    # the run reads it and cannot write it (the run folder has no write rule)
    if launch.get("potBin") and not os.path.lexists(os.path.join(rundir, "launcher")):
        os.symlink(os.path.join(launch["potBin"], "pot"), os.path.join(rundir, "launcher"))
    for n in ("explain.txt", "why.json"):
        if not os.path.exists(os.path.join(rundir, n)):
            with open(os.path.join(rundir, n), "w") as f:
                f.write("{}" if n.endswith(".json") else "")
    with open(os.path.join(rundir, "hidden.txt"), "w") as f:
        f.write("".join(i["value"] + "\n" for i in B.eff(policy, "hidden")))
    g = launch.get("git") or {}
    with open(os.path.join(rundir, "gitconfig"), "w") as f:
        f.write("# generated by pot: user.* from the host, no signing, no includes or helpers\n[user]\n")
        for k in ("name", "email"):
            if g.get(k):
                f.write(f"\t{k} = {json.dumps(g[k])}\n")
        f.write("[commit]\n\tgpgsign = false\n[tag]\n\tgpgsign = false\n[safe]\n\tdirectory = *\n")
    link(runhome, ".pot-hidden", os.path.join(rundir, "hidden.txt"), made)
    if not view and not os.path.lexists(os.path.join(runhome, ".gitconfig")):
        link(runhome, ".gitconfig", os.path.join(rundir, "gitconfig"), made)
    with open(os.path.join(rundir, "README.md"), "w") as f:
        f.write(readme(policy, launch, rundir, runhome))
    open(os.path.join(rundir, "denials"), "a").close()
    for n in OWN_FILES:
        own_reads.append(os.path.join(rundir, n))
    own_dirs = [os.path.join(rundir, n) for n in OWN_DIRS]

    # the rules, in order: Seatbelt applies the last rule that matches
    prof.note("the run's reads and writes")
    prof.paths("allow file-read* process-exec", "subpath", reads)
    prof.paths("allow file-read* file-write* process-exec", "subpath", writes)
    prof.note("the hidden list, and secret files in what inspect reads: denied over every allow above")
    prof.paths("deny file-read* file-write* process-exec", "subpath", hidden)
    prof.paths("deny file-read* file-write*", "literal", found)
    prof.note("the run's own, granted explicitly (its folder is in pot's state, which is hidden)")
    prof.paths("allow file-read* file-write* process-exec", "subpath", writes[:2] + own_writes)
    prof.paths("allow file-read*", "literal", [p for p in own_reads if not os.path.isdir(p)])
    prof.paths("allow file-read* process-exec", "subpath", [p for p in own_reads if os.path.isdir(p)] + own_dirs)
    # what realpath(3) and getcwd(3) stat on the way to each of them
    prof.paths("allow file-read-metadata", "literal",
               ancestors(reads + writes + own_writes + own_reads + own_dirs + [runhome]))
    # sockets: the run's own (in HOME and TMPDIR), pot's bindings (the run's folder), and socket requests
    socks = [os.path.join(rundir, os.path.basename(b["socket"])) for b in policy.get("bindings") or [] if b.get("socket")]
    socks += [real(i["path"]) for i in B.eff(policy, "socket") if os.path.exists(i["path"])]
    prof.rule("allow network-bind network-outbound",
              [f"(local unix-socket (subpath {prof.p(d)}))" for d in (runhome, runtmp)]
              + [f"(remote unix-socket (subpath {prof.p(d)}))" for d in (runhome, runtmp)])
    prof.rule("allow network-outbound", [f"(remote unix-socket (path-literal {prof.p(s)}))" for s in dict.fromkeys(socks)])
    prof.paths("allow file-read* file-write-data file-ioctl", "literal", ttys())
    prof.lines.append(RUN_RULES)          # run() puts its rules here: the redirects, the proxy's port
    prof.note("last: what no rule above may reopen")
    # file-link: no new hard link to a protected file (link(2) checks the file it links, and the new name)
    prof.paths("deny file-write* file-link", "subpath", ro)
    # no rename, removal or replacement: the workspace's pins, the run's HOME, TMPDIR and state, and each link
    prof.paths("deny file-write-unlink file-write-create", "literal", keep + made)
    # F_MAKECOMPRESSED and F_TRANSFEREXTENTS change a file through a read-only descriptor (codex's policy)
    prof.rule("deny system-fcntl", ["(fcntl-command 80 110)"])
    prof.rule("deny file-ioctl", [f"(ioctl-command {termios.TIOCSTI})"])
    with open(os.path.join(rundir, "profile.sb"), "w") as f:
        f.write(prof.text())
    with open(os.path.join(rundir, "profile.params"), "w") as f:
        json.dump(prof.params, f)
    with open(os.path.join(rundir, "sb.json"), "w") as f:
        json.dump({"cwd": cwd, "home": runhome, "tmp": runtmp, "ssl": ssl, "ro": ro_files, "links": made}, f)

    # before the run: what the after event compares (as bwrap.prepare)
    before = {"pins": [], "hostrun": {}, "refs": {}}
    for pins in pinsets:
        before["pins"].append({"top": pins.top, "fingerprint": B.fingerprint(cfg, pins)})
        before["hostrun"][pins.top] = B.hostrun_hashes(pins.top, pins.protect)
        before["refs"][pins.top] = B.refs(cfg, pins.top)
    for w, pins in clones:
        before.setdefault("clones", []).append({"top": w["path"], "clone": w["clone"], "gits": sorted(pins.dotgits)})
    with open(os.path.join(rundir, "before.json"), "w") as f:
        json.dump(before, f)
    with open(os.path.join(rundir, "launch.json"), "w") as f:
        json.dump(launch, f)
    if print_cmd:
        print_command(cfg, rundir, policy, launch, prof)


def readme(policy, launch, rundir, runhome):
    text = B.readme(policy, launch).replace("/run/pot/", rundir + "/")
    text = text.replace("- HOME is empty but for what the policy places there.",
                        f"- HOME is {runhome}, empty but for links to what the policy places there.")
    return text + f"""
On macOS (Seatbelt), this run differs from a Linux one:
- This folder ({rundir}) is the run's /run/pot: POT_POLICY names its policy.json.
- There is no process or network isolation: the run sees this user's processes, and its network is
  the proxy's port on the host's loopback ($HTTPS_PROXY, with the run's token). Listening on a port
  is refused.
- A dispatch clone runs at its own path, not at the repository's.
- Protected files and folders are path rules: writes, new hard links, renames and removals are refused.
"""


def run_env(cfg, policy, launch, sb):
    env = B.run_env(cfg, policy, {**launch, "home": sb["home"]})
    env.update({"TMPDIR": sb["tmp"] + "/", "SSL_CERT_FILE": sb["ssl"], "NIX_SSL_CERT_FILE": sb["ssl"], "SHLVL": "1"})
    return env


def run_argv(policy, launch):
    """The command, with /run/pot/… (a kit's argument: --mcp-config /run/pot/mcp.json) at the run's folder."""
    rd = policy["mounts"]["run"]
    return [rd + a[len("/run/pot"):] if isinstance(a, str) and (a == "/run/pot" or a.startswith("/run/pot/")) else a
            for a in launch["argv"]]


def print_command(cfg, rundir, policy, launch, prof):
    import shlex
    with open(os.path.join(rundir, "sb.json")) as f:
        sb = json.load(f)
    env = run_env(cfg, policy, launch, sb)
    for b in B.binding(policy, "credential") + B.binding(policy, "mcp"):
        for n in b.get("env") or []:
            env[n] = "(the run holds it)"
    print("# the Seatbelt command for this run (pot explain --backend-command)")
    print("cd " + shlex.quote(sb["cwd"]) + " && env -i \\")
    for k in sorted(env):
        print(f"  {k}={shlex.quote(env[k])} \\")
    print(f"  {SANDBOX_EXEC} -f {shlex.quote(os.path.join(rundir, 'profile.sb'))} \\")
    for p in prof.params:
        print("  " + shlex.quote(p) + " \\")
    print("  " + " ".join(shlex.quote(x) for x in [cfg["env"], "--default-signal=INT"] + run_argv(policy, launch)))
    if [i for i in B.eff(policy, "network") if i["class"] == "reach"]:
        print("# with network, the run adds (allow network-outbound (remote ip \"localhost:PORT\")), the proxy's port")
    print("# the profile, after macOS's part:")
    for line in prof.lines:
        print("#   " + line.replace("\n", "\n#   "))


# ---------------------------------------------------------------- run

def start_proxy(cfg, rundir, policy, launch, net):
    token = secrets.token_urlsafe(32)
    allow = [i["value"] for i in net]
    logp = os.path.join(rundir, "proxy.log")
    log = open(logp, "w")
    tmux = (launch.get("hostTools") or {}).get("tmux")
    live = ["--tmux", tmux, "--pane", launch["pane"]] if tmux and launch.get("pane") else []
    live += ["--fix", launch.get("netFix") or "the repo kit's requests.network"]
    proxy = subprocess.Popen([cfg["python"], "-I", "-S", cfg["proxy"], "tcp:127.0.0.1:0", "--denials",
                              os.path.join(rundir, "denials"), "--context", policy["context"]] + live + allow,
                             cwd=rundir, stdout=log, stderr=subprocess.STDOUT, env={**os.environ, "POT_PROXY_TOKEN": token})
    for _ in range(300):
        with open(logp) as f:
            first = f.readline().split()
        if first[:1] == ["LISTENING"] and first[1:2] and first[1].isdigit():
            return proxy, int(first[1]), token
        if proxy.poll() is not None:
            break
        time.sleep(0.01)
    log.close()
    with open(logp) as f:
        raise Stop("the network proxy did not start: " + f.read())


SANDBOX_FILTER_PATH = 1     # sandbox_check(2)'s filter type: a path


def sandbox_checker():
    """sandbox_check(pid, op, type, path) from libsystem_sandbox (ctypes): 0 allowed, 1 denied; with no operation, 1
    when the process is sandboxed. None where there is no such call (Linux).
    v2.19.2: the call is variadic after TYPE, so argtypes names the three fixed arguments and the path goes after
    them, as a variadic one: arm64 macOS passes those on the stack, where ctypes puts them only for arguments past
    argtypes. Without argtypes the path went in a register, every path check failed (-1, EINVAL), and a process
    that left the run's group without its TMPDIR outlived the run (F2)."""
    import ctypes
    try:
        lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib")
    except OSError:
        return None
    f = lib.sandbox_check
    f.restype = ctypes.c_int
    f.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int]

    def check(pid, op=None, path=None):
        if op is None:
            return f(pid, None, 0)
        return f(pid, op.encode(), SANDBOX_FILTER_PATH, ctypes.c_char_p(os.fsencode(path)))
    return check


def ps_lines(env):
    r = subprocess.run(["/bin/ps", "-A", "-ww"] + (["-E"] if env else []) + ["-o", "pid=", "-o", "command="],
                       capture_output=True, text=True)
    out = {}
    for l in r.stdout.splitlines():
        f = l.split(None, 1)
        if f and f[0].isdigit():
            out[int(f[0])] = f[1] if len(f) > 1 else ""
    return out


def leftovers(rundir, tmp, check=None, ps=ps_lines, me=None):
    """F2/F3 (fix round 4): the run's processes outside its group (setsid). First, by Seatbelt itself: a sandboxed
    process (sandbox_check) whose profile lets it read this run's policy.json, which only this run's does. Then, as a
    fallback, the run's TMPDIR in a process's environment only (in `ps -E` and not in its arguments), never argv."""
    me = os.getpid() if me is None else me
    pol = os.path.join(rundir, "policy.json")
    found = set()
    if check is not None:
        for pid in ps(False):
            if pid != me and check(pid) == 1 and check(pid, "file-read-data", pol) == 0:
                found.add(pid)
    mark = f" TMPDIR={tmp}/"
    plain, withenv = ps(False), ps(True)
    for pid, line in withenv.items():
        if pid != me and mark in f" {line}" and mark not in f" {plain.get(pid, '')}":
            found.add(pid)
    return sorted(found)


def stop_run(pgid, rundir, tmp, check="auto", rounds=20):
    """What is left of the run: its process group, then its other processes (leftovers), TERM then KILL, until a pass
    finds none (or ROUNDS passes)."""
    if check == "auto":
        check = sandbox_checker()
    for n in range(rounds):
        try:
            os.killpg(pgid, signal.SIGTERM if n == 0 else signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
        left = leftovers(rundir, tmp, check)
        if not left and n > 0:
            return
        for pid in left:
            try:
                os.kill(pid, signal.SIGTERM if n == 0 else signal.SIGKILL)
            except (ProcessLookupError, PermissionError):
                pass
        time.sleep(0.2 if n == 0 else 0.05)


def wait_job(pid, fg):
    """Wait for the run (its own process group), passing job control on: when it stops (^Z), pot's group
    stops too, so the shell sees the job stopped; when the shell resumes pot, the run is resumed."""
    while True:
        _, st = os.waitpid(pid, os.WUNTRACED)
        if not os.WIFSTOPPED(st):
            return os.waitstatus_to_exitcode(st)
        if fg:
            os.tcsetpgrp(0, os.getpgrp())
        os.killpg(os.getpgrp(), signal.SIGSTOP)
        if fg:
            os.tcsetpgrp(0, pid)
        try:
            os.killpg(pid, signal.SIGCONT)
        except ProcessLookupError:
            pass


def run(cfg, rundir):
    policy, launch = load(rundir)
    rundir = real(rundir)
    with open(os.path.join(rundir, "sb.json")) as f:
        sb = json.load(f)
    with open(os.path.join(rundir, "profile.params")) as f:
        params = json.load(f)
    net = [i for i in B.eff(policy, "network") if i["class"] == "reach"]
    proxy = None
    env = run_env(cfg, policy, launch, sb)
    extra = Profile()
    extra.keys = {p[2:].split("=", 1)[1]: p[2:].split("=", 1)[0] for p in params}
    extra.params = list(params)
    rd = redirects()                       # the run's own descriptors (after --detach's redirect)
    extra.paths("allow file-read-data file-read-metadata", "literal", rd["read"])
    extra.paths("allow file-write-data file-read-metadata", "literal", rd["write"])
    if net:
        proxy, port, token = start_proxy(cfg, rundir, policy, launch, net)
        extra.note("the network: the proxy's port alone")
        extra.lines.append(f'(allow network-outbound (remote ip "localhost:{int(port)}"))')
        url = f"http://pot:{token}@127.0.0.1:{port}"
        for v in ("HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"):
            env[v] = url
    with open(os.path.join(rundir, "profile.sb")) as f:
        text = f.read()
    prof = os.path.join(rundir, "profile.run.sb")
    with open(prof, "w") as f:
        f.write(text.replace(RUN_RULES + "\n", "\n".join(extra.lines + [RUN_RULES]) + "\n", 1))
    params = extra.params
    helpers = B.start_bindings(cfg, rundir, policy, launch)
    env.update(B.credentials(policy))
    cmd = [SANDBOX_EXEC, "-f", prof] + params + [cfg["env"], "--default-signal=INT"] + run_argv(policy, launch)
    fg = False
    try:
        fg = os.isatty(0) and os.tcgetpgrp(0) == os.getpgrp()
    except OSError:
        pass
    old = {s: signal.signal(s, signal.SIG_IGN) for s in (signal.SIGINT, signal.SIGTTOU)}
    pgid = None
    try:
        # the run is its own process group, in the terminal's foreground when pot is (^C reaches it alone)
        p = subprocess.Popen(cmd, env=env, cwd=sb["cwd"], process_group=0, close_fds=True)
        pgid = p.pid
        if fg:
            os.tcsetpgrp(0, pgid)
            os.killpg(pgid, signal.SIGCONT)      # in case it read the terminal before it was in the foreground
        status = wait_job(p.pid, fg)
        p.returncode = status
    finally:
        if pgid:
            stop_run(pgid, rundir, sb["tmp"])
        if fg:
            try:
                os.tcsetpgrp(0, os.getpgrp())
            except OSError:
                pass
        for s, h in old.items():
            signal.signal(s, h)
        tools = launch.get("hostTools") or {}
        if B.binding(policy, "status.relay") and tools.get("workmux") and launch.get("pane"):
            subprocess.run([tools["workmux"], "set-window-status", "clear"], env={**os.environ, "TMUX_PANE": launch["pane"]},
                           stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for h in ([proxy] if proxy else []) + helpers:
            h.terminate()
            try:
                h.wait(2)
            except subprocess.TimeoutExpired:
                h.kill()
    if status < 0:
        status = 128 - status
    with open(os.path.join(rundir, "status"), "w") as f:
        f.write(str(status))
    return status


# ---------------------------------------------------------------- after: the clone's git, sandboxed

GIT_PROFILE = '''
; fetch-back's git: its closure, the clone read-only, a temporary HOME; no network, nothing else of the host
(allow file-read* (subpath (param "CLONE")))
(allow file-read* file-write* (subpath (param "TMP")))
(allow file-read-metadata (literal "/private") (literal "/private/var") (literal "/Users"))
'''


def git_denies():
    """F1 (fix round 4): what the run's profile denies last, for the clone's git too (it runs the clone's own config:
    an fsmonitor, a filter): no TIOCSTI, no F_MAKECOMPRESSED or F_TRANSFEREXTENTS"""
    import termios
    return ("; last: what no rule above may reopen (as the run's profile)\n"
            "(deny system-fcntl (fcntl-command 80 110))\n"
            f"(deny file-ioctl (ioctl-command {getattr(termios, 'TIOCSTI', 0x80017472)}))\n")


# a new session (no controlling terminal), and stdin from /dev/null unless the git step speaks on it (upload-pack)
DETACH = "import os, sys; os.setsid(); n = int(sys.argv[1]); fd = os.open(os.devnull, os.O_RDONLY)\nif n: os.dup2(fd, 0)\nos.execv(sys.argv[2], sys.argv[2:])"


def sandboxed_git(cfg, rundir, clone, args, cwd="/clone", timeout=600):
    """bwrap.sandboxed_git, for Seatbelt: git under a profile with git's closure and the clone read-only, at the
    clone's own path (/clone… in ARGS and CWD is the clone)."""
    rundir, clone = real(rundir), real(clone)
    tmp = os.path.join(rundir, "gittmp")
    os.makedirs(tmp, exist_ok=True)
    prof = os.path.join(rundir, "git.sb")
    if not os.path.exists(prof):
        with open(cfg["gitClosure"]) as f:
            closure = sorted({l.strip() for l in f if l.strip()})
        p = Profile()
        store_rules(p, closure)
        p.paths("allow file-read-metadata", "literal", ancestors([clone, tmp]))
        with open(prof, "w") as f:
            f.write(p.text() + GIT_PROFILE + git_denies())
        with open(prof + ".params", "w") as f:
            json.dump(p.params, f)
    with open(prof + ".params") as f:
        params = json.load(f)
    at = lambda a: clone + a[len("/clone"):] if a == "/clone" or a.startswith("/clone/") else a   # noqa: E731
    # cwd is the git directory's top: run git from there (sandbox-exec has no --chdir)
    quiet = "0" if args[:1] == ["upload-pack"] else "1"
    return [cfg["python"], "-I", "-S", "-c", DETACH, quiet, SANDBOX_EXEC, "-f", prof, *params, f"-DCLONE={clone}", f"-DTMP={tmp}", cfg["env"], "-i", f"HOME={tmp}",
            f"PATH={os.path.dirname(cfg['git'])}", "GIT_CONFIG_NOSYSTEM=1", "GIT_CONFIG_GLOBAL=/dev/null",
            cfg["git"], "-C", at(cwd), "-c", "safe.directory=*"] + [at(a) for a in args]


def after(cfg, rundir):
    B.sandboxed_git = sandboxed_git     # the clone's look (staging, status) under Seatbelt
    try:
        with open(os.path.join(rundir, "sb.json")) as f:
            sb = json.load(f)
    except (OSError, ValueError):
        sb = {}
    # the profile denies a new hard link to a protected file (file-link): this is a second check
    for f in sb.get("ro") or []:
        try:
            if os.lstat(f).st_nlink > 1:
                print(B.visible(f"pot: WARNING: {f} is protected and now has another hard link: the run may have written "
                                "it through that link; read it, then replace it with a copy of its own"), file=sys.stderr)
        except OSError:
            pass
    B.after(cfg, rundir)


def cleanup(rundir):
    B.cleanup(rundir)
    for n in ("home", "tmp", "gittmp", "fixed"):
        d = os.path.join(rundir, n)
        if os.path.isdir(d) and not os.path.islink(d):
            # the run can make what it holds unwritable: give it back first (never through a link)
            for dp, ds, fs in os.walk(d):
                for x in ds:
                    q = os.path.join(dp, x)
                    if not os.path.islink(q):
                        try:
                            os.chmod(q, 0o700)
                        except OSError:
                            pass
            shutil.rmtree(d, ignore_errors=True)
    # git.sb stays: the clone's git profile, read again by a later look (pot review) and by pot_seatbelt's F1 check
    for n in ("profile.sb", "profile.run.sb", "profile.params", "empty"):
        try:
            os.unlink(os.path.join(rundir, n))
        except OSError:
            pass


def main(argv):
    if len(argv) < 4 or argv[0] != "--config":
        print("usage: backend --config CONFIG probe|prepare|run|after|cleanup RUNDIR [--print]", file=sys.stderr)
        return 2
    with open(argv[1]) as f:
        cfg = json.load(f)
    verb, rundir = argv[2], argv[3]
    try:
        if verb == "probe":
            probe(cfg)
        elif verb == "prepare":
            try:
                prepare(cfg, rundir, print_cmd="--print" in argv[4:])
            finally:
                if B.PLACED:
                    with open(os.path.join(rundir, "placeholders.json"), "w") as f:
                        json.dump(B.PLACED, f)
        elif verb == "run":
            return run(cfg, rundir)
        elif verb == "after":
            after(cfg, rundir)
        elif verb == "cleanup":
            cleanup(rundir)
        else:
            print(f"backend: unknown verb {verb}", file=sys.stderr)
            return 2
    except Stop as e:
        policy = {}
        try:
            policy, _ = B.load(rundir)
        except Exception:
            pass
        print(B.visible(f"pot ({policy.get('context', 'run')}): {e}"), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
