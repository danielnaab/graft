"""graft sb: the bubblewrap backend (design §11).

    backend probe|prepare|run|after|cleanup <run dir>

The backend reads <run dir>/policy.json (policy.json v1, the contract) and the launcher's facts in
<run dir>/launch.json (the argv, the working directory, the closure, the caller's identity), and
carries the policy out with bubblewrap. --config names the store paths this backend was built with.

Guarantees it proves (probe): closure-store own-netns own-pidns no-tiocsti protected-as-mounts
local-listen in-place same-path-clone private-proxy root-files (nest: not in 1a, whose children
tables are empty, so every run gets --disable-userns).

What it does with the workspace (in place): the repository bound writable, each git directory, its
config and hooks, the files that say where a repository is, and the protect list bound onto
themselves (mount points: not renamed, removed or replaced; read-only where protected). A link on a
protected path, or a protected file with a second hard link, stops the run.
"""
import json
import os
import re
import signal
import stat
import subprocess
import sys
import time

GUARANTEES = ["closure-store", "own-netns", "own-pidns", "no-tiocsti", "protected-as-mounts", "local-listen",
              "in-place", "same-path-clone", "private-proxy", "root-files"]
GITFILES = ["config", "config.worktree", "hooks", "info", "commondir", "gitdir"]
# §9: changed host-run files the after event reports, beyond the protect list
HOST_RUN_EXTRA = [".gitmodules", ".lfsconfig", ".gitattributes", ".github/workflows", ".forgejo/workflows",
                  ".gitlab-ci.yml"]
SECRET_RE = re.compile(r"^(\.env|\.env\..*|.*\.tfstate.*|id_(rsa|dsa|ecdsa|ed25519).*|\.git-credentials|\.netrc"
                       r"|.*\.pem|.*\.key)$")
PAYLOAD_DIRS = {".venv", "node_modules", "site-packages"}


class Stop(Exception):
    pass


def visible(s):
    """Run-chosen text, safe for the host's terminal: control bytes as \\xNN."""
    return "".join(c if (0x20 <= ord(c) < 0x7f or ord(c) > 0x9f) else f"\\x{ord(c):02x}" for c in s)


def load(rundir):
    with open(os.path.join(rundir, "policy.json")) as f:
        policy = json.load(f)
    with open(os.path.join(rundir, "launch.json")) as f:
        launch = json.load(f)
    return policy, launch


def eff(policy, kind):
    return [i for i in policy["items"] if i["kind"] == kind and i["effective"]]


def tilde(p, home):
    return "~" + p[len(home):] if p == home or p.startswith(home + "/") else p


# ---------------------------------------------------------------- probe

def probe(cfg):
    r = subprocess.run([cfg["bwrap"], "--unshare-all", "--unshare-user", "--disable-userns", "--ro-bind", "/", "/", "true"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(json.dumps({"guarantees": [], "error": r.stderr.strip()}))
        return
    print(json.dumps({"guarantees": GUARANTEES}))


# ---------------------------------------------------------------- the workspace's pins (in place)

class Pins:
    def __init__(self, top, common, protect, cfg):
        self.top, self.common, self.protect, self.cfg = top, common, protect, cfg
        self.dirs, self.ro, self.seen = [], [], set()

    def rel(self, p):
        return p[len(self.top) + 1:] if p.startswith(self.top + "/") else p

    def nolink(self, p, root):
        d = p
        while len(d) > len(root):
            if os.path.islink(d):
                what = "make it read-only" if d == p else f"pin {self.rel(p)}, below it"
                raise Stop(f"{self.rel(d)} is a symbolic link, so the sandbox can't {what} (a run must not write "
                           "what decides the sandbox or what the host runs): replace the link with what it points "
                           "to, or remove it, then run again")
            d = os.path.dirname(d)

    def pin(self, how, p):
        root = self.top if p.startswith(self.top + "/") else self.common
        self.nolink(p, root)
        up, d = [], p
        while True:
            d = os.path.dirname(d)
            if len(d) <= len(root):
                break
            up.insert(0, d)
        if how == "rw":
            up.append(p)
        for d in up:
            if d not in self.seen:
                self.seen.add(d)
                self.dirs.append(d)
        if how == "ro" and ("ro:" + p) not in self.seen:
            self.seen.add("ro:" + p)
            self.ro.append(p)

    def gitdirs(self, g):
        out = [g]
        wt = os.path.join(g, "worktrees")
        if os.path.isdir(wt):
            for d in sorted(os.listdir(wt)):
                if os.path.isfile(os.path.join(wt, d, "HEAD")):
                    out += self.gitdirs(os.path.join(wt, d))
        mods = os.path.join(g, "modules")
        if os.path.isdir(mods):
            out += self.below(mods)
        return out

    def below(self, d):
        out = []
        for e in sorted(os.listdir(d)):
            p = os.path.join(d, e)
            if os.path.isdir(p) and not os.path.islink(p):
                out += self.gitdirs(p) if os.path.isfile(os.path.join(p, "HEAD")) else self.below(p)
        return out

    def scan(self):
        names = [p[3:] for p in self.protect if p.startswith("**/")]
        expr = ["-name", ".git", "-prune", "-print0"]
        if names:
            ors = []
            for n in names:
                ors += (["-o"] if ors else []) + ["-name", n]
            expr += ["-o", "("] + ors + [")", "-print0"]
        out = subprocess.run([self.cfg["find"], self.top] + expr, capture_output=True, check=False).stdout
        self.dotgits, self.named = [], []
        for e in out.split(b"\0"):
            if not e:
                continue
            e = os.fsdecode(e)
            (self.dotgits if os.path.basename(e) == ".git" else self.named).append(e)

    def allgitdirs(self):
        out = []
        for e in self.dotgits:
            if os.path.isdir(e) and not os.path.islink(e):
                out += self.gitdirs(e)
        if self.common != os.path.join(self.top, ".git"):
            out += self.gitdirs(self.common)
        return out

    def compute(self, hooks):
        self.scan()
        for g in self.allgitdirs():
            self.pin("rw", g)
            if os.path.isfile(os.path.join(g, "config")) and not os.path.isdir(os.path.join(g, "hooks")):
                os.mkdir(os.path.join(g, "hooks"))
            for f in GITFILES:
                if os.path.lexists(os.path.join(g, f)):
                    self.pin("ro", os.path.join(g, f))
        for e in self.dotgits:
            if not (os.path.isdir(e) and not os.path.islink(e)):
                self.pin("ro", e)
        for e in self.named:
            self.pin("ro", e)
        for p in self.protect:
            if p.startswith("**/"):
                continue
            ap = os.path.join(self.top, p.rstrip("/"))
            self.nolink(ap, self.top)
            if os.path.lexists(ap):
                self.pin("ro", ap)
        if hooks and (hooks.startswith(self.top + "/") or hooks.startswith(self.common + "/")):
            os.makedirs(hooks, exist_ok=True)
            self.pin("ro", hooks)
        for f in self.ro:
            st = os.lstat(f)
            if stat.S_ISREG(st.st_mode) and st.st_nlink > 1:
                raise Stop(f"{self.rel(f)} has another hard link, so a run could write it through that link (a run "
                           "must not write what decides the sandbox or what the host runs): replace it with a copy of "
                           "its own (cp -p F F.copy && mv F.copy F), then run again")


def git(cfg, top, *args):
    """Host git reading a run-written repository: no hooks, no fsmonitor, no config it could run."""
    return subprocess.run([cfg["git"], "-c", "core.fsmonitor=false", "-c", "core.hooksPath=/dev/null",
                           "-c", "safe.directory=*", "-C", top] + list(args),
                          capture_output=True, text=True, check=False)


def fingerprint(cfg, pins):
    """What host git, a shell prompt, an editor or direnv would read and could run code from."""
    lines = set(pins.dotgits) | set(pins.named)
    for g in pins.allgitdirs():
        lines.add(g)
        for f in GITFILES:
            if os.path.lexists(os.path.join(g, f)):
                lines.add(os.path.join(g, f))
    for l in git(cfg, pins.top, "ls-files", "-s").stdout.splitlines():
        if l.startswith("160000 "):
            lines.add("gitlink " + l.split("\t", 1)[1])
    return sorted(lines)


def hostrun_hashes(top, protect):
    """Contents of host-run files (the protect list, plus §9's extras): path -> hash."""
    import hashlib
    out = {}
    pats = [p for p in protect if not p.startswith("**/")] + HOST_RUN_EXTRA
    names = {p[3:] for p in protect if p.startswith("**/")}

    def h(p):
        if os.path.isdir(p) and not os.path.islink(p):
            for d, ds, fs in os.walk(p):
                ds[:] = [x for x in ds if x != ".git"]
                for f in fs:
                    h(os.path.join(d, f))
        elif os.path.lexists(p):
            try:
                with open(p, "rb") as f:
                    out[p[len(top) + 1:]] = hashlib.sha256(f.read()).hexdigest()
            except OSError:
                out[p[len(top) + 1:]] = "unreadable"
    for p in pats:
        h(os.path.join(top, p))
    for d, ds, fs in os.walk(top):
        ds[:] = [x for x in ds if x != ".git" and x != "node_modules"]
        for f in fs + ds:
            if f in names:
                h(os.path.join(d, f))
    return out


def refs(cfg, top):
    r = git(cfg, top, "for-each-ref", "--format=%(objectname) %(refname)")
    head = git(cfg, top, "rev-parse", "HEAD").stdout.strip()
    return {"refs": r.stdout.splitlines(), "head": head}


# ---------------------------------------------------------------- prepare

EMPTY = None   # <run dir>/empty, set by prepare: what a hidden file, a secret or a socket shows


def masked(p):
    """bwrap arguments that hide P: a directory as an empty read-only tmpfs, anything else as an
    empty read-only file."""
    if os.path.isdir(p) and not os.path.islink(p):
        return [["--tmpfs", p], ["--remount-ro", p]]
    return [["--ro-bind", EMPTY, p]]


def depth(p):
    return p.rstrip("/").count("/")


def secret_walk(cfg, dirs):
    """§5: secret files in readable trees (gitignore-unaware: secrets are gitignored by design), and sockets."""
    dirs = [d for d in dirs if os.path.isdir(d)]
    if not dirs:
        return [], []
    r = subprocess.run([cfg["fd"], "-HI", "-u", "--type", "f", "--type", "l", "--absolute-path",
                        "-0", SECRET_RE.pattern, *dirs], capture_output=True, check=False)
    secrets = []
    for e in r.stdout.split(b"\0"):
        if not e:
            continue
        p = os.fsdecode(e)
        n = os.path.basename(p)
        if n.startswith("id_") and n.endswith(".pub"):
            continue
        if (n.endswith(".pem") or n.endswith(".key")) and PAYLOAD_DIRS & set(p.split("/")):
            continue
        if n.startswith("id_") and not re.match(r"id_(rsa|dsa|ecdsa|ed25519)", n):
            continue
        secrets.append(p)
    s = subprocess.run([cfg["fd"], "-HI", "-u", "--type", "s", "--absolute-path", "-0", ".", *dirs],
                       capture_output=True, check=False)
    socks = [os.fsdecode(e) for e in s.stdout.split(b"\0") if e]
    return secrets, socks


def sockets_in(cfg, dirs):
    out = []
    for d in dirs:
        if os.path.isdir(d) and not os.path.islink(d):
            r = subprocess.run([cfg["find"], d, "-xdev", "-type", "s", "-print0"], capture_output=True, check=False)
            out += [os.fsdecode(e) for e in r.stdout.split(b"\0") if e]
    return out


def hm_links(home):
    """home-manager's generated files: (link in ~, its target), from the current generation."""
    for c in (".local/state/nix/profiles/home-manager/home-files", ".local/state/home-manager/gcroots/current-home/home-files"):
        hf = os.path.join(home, c)
        if os.path.isdir(hf):
            break
    else:
        return []
    hf = os.path.realpath(hf)
    out = []
    for d, ds, fs in os.walk(hf):
        for n in list(ds) + fs:
            p = os.path.join(d, n)
            if not os.path.islink(p):
                continue
            rel = p[len(hf) + 1:]
            target = os.path.join(home, rel)
            if os.path.islink(target) and os.readlink(target).startswith("/nix/store/"):
                out.append((target, os.readlink(target)))
            if n in ds:
                ds.remove(n)
    return out


def readme(policy, launch):
    ctx = policy["context"]
    return f"""# graft sandbox: this run ({ctx})

You are inside a graft sandbox run (`sb`), context `{ctx}`, access `{policy['access']}`.
The policy is /run/graft-sandbox/policy.json; `sb why PATH|HOST|CMD|VAR` explains one item.

- HOME is empty but for what the policy places there. ~/.graft-sandbox-hidden lists the paths
  hidden from every run: they are not mounted here, and cannot be written.
- The network goes only through the run's proxy, to the hosts the policy grants. When a client
  prints `CONNECT tunnel failed, response 403`, `tunnel error`, a 403, or `Could not resolve host`,
  the sandbox refused it: read /run/graft-sandbox/denials, or run `sb why HOST`.
- A program that is not here is not in this run: `sb why NAME` says how to add it.
- Protected files (what decides the sandbox, what the host runs, agent configuration) are
  read-only, and their folders cannot be renamed or removed. Edit them on the host.
- Commits made here are unsigned; the host signs them afterwards.
- Every denial message starts `graft sandbox ({ctx}):`.
"""


def prepare(cfg, rundir, print_cmd=False):
    global EMPTY
    policy, launch = load(rundir)
    EMPTY = os.path.join(rundir, "empty")
    open(EMPTY, "w").close()
    os.chmod(EMPTY, 0o444)
    home, ctx = launch["home"], policy["context"]
    view = next((i for i in eff(policy, "view")), None)
    args = [["--unshare-all"], ["--unshare-user"], ["--disable-userns"], ["--die-with-parent"], ["--hostname", "sb"]]
    mounts = []          # (dest, priority, args): sorted by depth, then priority (write 0, read 1, hidden 2)

    def mount(dest, prio, a):
        mounts.append((dest, prio, a))

    hidden = [p for i in eff(policy, "hidden") for p in i.get("paths", [])]
    run_mnt = policy["mounts"]["run"]
    net = [i for i in eff(policy, "network") if i["class"] == "reach"]
    bound_dirs = []

    if view:
        args += [["--ro-bind", "/", "/"], ["--proc", "/proc"], ["--dev", "/dev"], ["--tmpfs", "/tmp"], ["--tmpfs", "/run"]]
        if os.path.exists("/run/current-system"):
            args.append(["--ro-bind", "/run/current-system", "/run/current-system"])
        xdg = launch.get("xdgRuntime")
        if xdg and not xdg.startswith("/run/"):
            args.append(["--tmpfs", xdg])
        args.append(["--tmpfs", home])
        readable = []
        for h in view["home"]:
            p = os.path.join(home, h[2:]) if h.startswith("~/") else h
            if os.path.exists(p):
                mount(p, 1, ["--ro-bind", p, p])
                readable.append(p)
        if view.get("homeManager", True):
            for link, target in hm_links(home):
                if any(link == h or link.startswith(h + "/") for h in hidden):
                    continue
                if not any(link == r or link.startswith(r + "/") for r in readable):
                    mount(link, 1, ["--symlink", target, link])
        for w in policy["workspace"]:
            # the workspace, read-only, wherever it is (in ~ outside inspect.home, or in /tmp)
            if not any(w["path"].startswith(r + "/") or w["path"] == r for r in readable):
                mount(w["path"], 1, ["--ro-bind", w["path"], w["path"]])
                readable.append(w["path"])
        t = time.time()
        secrets, socks = secret_walk(cfg, readable)
        for s in secrets:
            mount(s, 2, ["--ro-bind", EMPTY, s])
        others = [d for d in ("/etc", "/var", "/srv", "/opt", "/root", "/mnt", "/media") if os.path.isdir(d)]
        if os.path.isdir("/home"):
            others += [os.path.join("/home", h) for h in os.listdir("/home") if os.path.join("/home", h) != home]
        socks += [s for s in sockets_in(cfg, others) if not s.startswith(home + "/")]
        for s in socks:
            mount(s, 2, ["--ro-bind", EMPTY, s])
        launch["walk"] = {"secrets": len(secrets), "sockets": len(socks), "seconds": round(time.time() - t, 2)}
        for h in hidden:
            gone = any(h == d or h.startswith(d + "/") for d in ("/tmp", "/run", launch.get("xdgRuntime") or "/run"))
            if gone and not any(h.startswith(r + "/") for r in readable):
                continue
            if os.path.lexists(h) and (not h.startswith(home + "/") or any(h.startswith(r + "/") or h == r for r in readable)):
                for a in masked(h):
                    mount(h, 2, a)
        for i in eff(policy, "read"):
            p = i["path"]
            if os.path.exists(p):
                mount(p, 1, ["--ro-bind", p, p])
    else:
        closure = launch.get("closure") or []
        with open(cfg["baseClosure"]) as f:
            closure = sorted(set(closure) | {l.strip() for l in f if l.strip()})
        if net:
            with open(cfg["netClosure"]) as f:
                closure = sorted(set(closure) | {l.strip() for l in f if l.strip()})
        launch["closureCount"] = len(closure)
        with open(os.path.join(rundir, "closure.args"), "wb") as f:
            for p in closure:
                f.write(b"--ro-bind\0" + p.encode() + b"\0" + p.encode() + b"\0")
        args += [["--proc", "/proc"], ["--dev", "/dev"], ["--tmpfs", "/tmp"], ["--tmpfs", "/run"],
                 ["--ro-bind", os.path.join(rundir, "passwd"), "/etc/passwd"],
                 ["--ro-bind", os.path.join(rundir, "group"), "/etc/group"],
                 ["--ro-bind", os.path.join(rundir, "hosts"), "/etc/hosts"],
                 ["--ro-bind", cfg["sh"], "/bin/sh"], ["--ro-bind", cfg["env"], "/usr/bin/env"],
                 ["--tmpfs", home]]
        if not any(i["value"] == "/etc/ssl/certs/ca-certificates.crt" for i in eff(policy, "file")):
            args.append(["--ro-bind", cfg["cacert"], "/etc/ssl/certs/ca-certificates.crt"])
        if os.path.exists("/run/current-system"):
            args.append(["--ro-bind", "/run/current-system", "/run/current-system"])
        for i in eff(policy, "read"):
            if os.path.exists(i["path"]):
                mount(i["path"], 1, ["--ro-bind", i["path"], i["path"]])
                bound_dirs.append(i["path"])
        for i in eff(policy, "write"):
            if os.path.exists(i["path"]):
                mount(i["path"], 0, ["--bind", i["path"], i["path"]])
                bound_dirs.append(i["path"])
        for h in hidden:
            if os.path.lexists(h) and any(h.startswith(b + "/") for b in bound_dirs):
                for a in masked(h):
                    mount(h, 2, a)

    # sockets (M5): every socket under a granted directory is masked unless a socket request names it
    named_socks = {i["path"] for i in eff(policy, "socket")}
    for i in eff(policy, "socket"):
        if os.path.exists(i["path"]):
            mount(i["path"], 0, ["--bind", i["path"], i["path"]])
    for s in sockets_in(cfg, bound_dirs):
        if s not in named_socks:
            mount(s, 2, ["--ro-bind", EMPTY, s])

    # the workspace
    pinsets = []
    for w in policy["workspace"]:
        top, common = w["path"], w["common"]
        if view or policy["access"] != "write":
            if not view:
                mount(top, 1, ["--ro-bind", top, top])
                if not common.startswith(top + "/"):
                    mount(common, 1, ["--ro-bind", common, common])
            continue
        pins = Pins(top, common, w["protected"], cfg)
        hooks = git(cfg, top, "rev-parse", "--path-format=absolute", "--git-path", "hooks").stdout.strip()
        pins.compute(hooks)
        pinsets.append(pins)
        mount(top, 0, ["--bind", top, top])
        if not common.startswith(top + "/"):
            mount(common, 0, ["--bind", common, common])
        for d in pins.dirs:
            mount(d, 0, ["--bind", d, d])
        for p in pins.ro:
            mount(p, 1, ["--ro-bind", p, p])
        for s in sockets_in(cfg, [top]):
            mount(s, 2, ["--ro-bind", EMPTY, s])
    # files, state, the run's own
    for i in eff(policy, "file"):
        dest = i["value"] if not i["value"].startswith("~") else home + i["value"][1:]
        mount(dest, 1, ["--ro-bind", i["source"], dest])
    for s in policy["mounts"]["state"]:
        os.makedirs(s["dir"], exist_ok=True)
        dest = home + s["at"][1:]
        mount(dest, 0, ["--bind", s["dir"], dest])
    with open(os.path.join(rundir, "hidden.txt"), "w") as f:
        f.write("".join(i["value"] + "\n" for i in eff(policy, "hidden")))
    mount(home + "/.graft-sandbox-hidden", 1, ["--ro-bind", os.path.join(rundir, "hidden.txt"), home + "/.graft-sandbox-hidden"])
    if not view:
        mount(home + "/.gitconfig", 1, ["--ro-bind", os.path.join(rundir, "gitconfig"), home + "/.gitconfig"])
    for n in ("policy.json", "README.md", "denials"):
        mount(f"{run_mnt}/{n}", 1, ["--ro-bind", os.path.join(rundir, n), f"{run_mnt}/{n}"])
    if net:
        mount(f"{run_mnt}/proxy.sock", 1, ["--ro-bind", os.path.join(rundir, "proxy.sock"), f"{run_mnt}/proxy.sock"])

    mounts.sort(key=lambda m: (depth(m[0]), m[1]))
    for _, _, a in mounts:
        args.append(a)
    args += [["--chdir", launch["cwd"]], ["--clearenv"]]

    # files the run sees
    with open(os.path.join(rundir, "passwd"), "w") as f:
        f.write(f"{launch['user']}:x:{launch['uid']}:{launch['gid']}::{home}:{cfg['shell']}\n")
    with open(os.path.join(rundir, "group"), "w") as f:
        f.write(f"{launch['group']}:x:{launch['gid']}:\n")
    with open(os.path.join(rundir, "hosts"), "w") as f:
        f.write("127.0.0.1 localhost sb\n::1 localhost sb\n")
    g = launch.get("git") or {}
    with open(os.path.join(rundir, "gitconfig"), "w") as f:
        f.write("# generated by sb: user.* from the host, no signing, no includes or helpers\n[user]\n")
        for k in ("name", "email"):
            if g.get(k):
                f.write(f"\t{k} = {json.dumps(g[k])}\n")
        f.write("[commit]\n\tgpgsign = false\n[tag]\n\tgpgsign = false\n[safe]\n\tdirectory = *\n")
    with open(os.path.join(rundir, "README.md"), "w") as f:
        f.write(readme(policy, launch))
    open(os.path.join(rundir, "denials"), "a").close()
    with open(os.path.join(rundir, "bwrap.args"), "wb") as f:
        for a in args:
            for x in a:
                f.write(x.encode() + b"\0")
    # before the run: what the after event compares
    before = {"pins": [], "hostrun": {}, "refs": {}}
    for pins in pinsets:
        before["pins"].append({"top": pins.top, "fingerprint": fingerprint(cfg, pins)})
        before["hostrun"][pins.top] = hostrun_hashes(pins.top, pins.protect)
        before["refs"][pins.top] = refs(cfg, pins.top)
    with open(os.path.join(rundir, "before.json"), "w") as f:
        json.dump(before, f)
    with open(os.path.join(rundir, "launch.json"), "w") as f:
        json.dump(launch, f)
    if print_cmd:
        print_command(cfg, rundir, policy, launch, args)


def run_env(cfg, policy, launch):
    """The run's environment, whole: what sb sets, and of the caller's only TERM, COLORTERM and the
    granted variables."""
    home, ctx = launch["home"], policy["context"]
    paths = []
    tools = None
    for i in eff(policy, "package"):
        b = i["value"] + "/bin"
        if os.path.isdir(b) and b not in paths:
            paths.append(b)
        if i.get("tools"):
            tools = b
    if launch.get("sbBin"):
        paths.append(launch["sbBin"])
    env = {"HOME": home, "USER": launch["user"], "LOGNAME": launch["user"], "SHELL": cfg["shell"],
           "PATH": ":".join(paths), "LANG": "C.UTF-8", "TERMINFO_DIRS": cfg["terminfo"],
           "SSL_CERT_FILE": "/etc/ssl/certs/ca-certificates.crt", "NIX_SSL_CERT_FILE": "/etc/ssl/certs/ca-certificates.crt",
           "TZDIR": cfg["tzdir"], "GRAFT_SANDBOX": ctx, "GRAFT_SANDBOX_POLICY": f"{policy['mounts']['run']}/policy.json",
           "PS1": f"(sb:{ctx}) \\w \\$ "}
    if tools:
        env["GRAFT_SANDBOX_TOOLS"] = tools
    for v in ("TERM", "COLORTERM"):
        if v in os.environ:
            env[v] = os.environ[v]
    for i in eff(policy, "env.set"):
        n, _, v = i["value"].partition("=")
        env[n] = v
    for i in eff(policy, "env"):
        if i["value"] in os.environ:
            env[i["value"]] = os.environ[i["value"]]
    if [i for i in eff(policy, "network") if i["class"] == "reach"]:
        for v in ("HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"):
            env[v] = "http://127.0.0.1:3128"
        env["NO_PROXY"] = env["no_proxy"] = "localhost,127.0.0.1,::1"
    return env


def print_command(cfg, rundir, policy, launch, args):
    import shlex
    env = run_env(cfg, policy, launch)
    print("# the bubblewrap command for this run (sb explain --backend-command)")
    print(f"env -i {cfg['bwrap']} --args 3 \\")
    for a in args:
        print("  " + " ".join(shlex.quote(x) for x in a) + " \\")
    for k in sorted(env):
        shown = env[k] if not any(k == i["value"] and i.get("secret") for i in eff(policy, "env")) else "(the run holds it)"
        print(f"  --setenv {k} {shlex.quote(shown)} \\")
    print("  -- " + " ".join(shlex.quote(x) for x in launch["argv"]))
    if "closureCount" in launch:
        print(f"# --args 3: --ro-bind for each of {launch['closureCount']} store paths (the closure)")
    print(f"# fd 4: the TIOCSTI seccomp filter ({cfg['bpf']})")


# ---------------------------------------------------------------- run

def run(cfg, rundir):
    policy, launch = load(rundir)
    net = [i for i in eff(policy, "network") if i["class"] == "reach"]
    proxy = None
    if net:
        allow = [i["value"] for i in net]
        log = open(os.path.join(rundir, "proxy.log"), "w")
        proxy = subprocess.Popen([cfg["python"], "-I", "-S", cfg["proxy"], "proxy.sock",
                                  "--denials", os.path.join(rundir, "denials"), "--context", policy["context"]] + allow,
                                 cwd=rundir, stdout=log, stderr=subprocess.STDOUT)
        for _ in range(300):
            if os.path.exists(os.path.join(rundir, "proxy.sock")) or proxy.poll() is not None:
                break
            time.sleep(0.01)
        if not os.path.exists(os.path.join(rundir, "proxy.sock")):
            log.close()
            with open(os.path.join(rundir, "proxy.log")) as f:
                raise Stop("the network proxy did not start: " + f.read())
    env = run_env(cfg, policy, launch)
    with open(os.path.join(rundir, "bwrap.args"), "rb") as f:
        argbytes = f.read()
    for k, v in env.items():
        argbytes += b"--setenv\0" + k.encode() + b"\0" + v.encode() + b"\0"
    # the arguments and the environment through a pipe: /proc/1/cmdline in the run names no host
    # path, ps no passed value, and no value is written to disk
    import threading
    r, w = os.pipe()

    def feed():
        with os.fdopen(w, "wb") as f:
            f.write(argbytes)
    threading.Thread(target=feed, daemon=True).start()
    cl = os.path.join(rundir, "closure.args")
    closure = os.open(cl if os.path.exists(cl) else "/dev/null", os.O_RDONLY)
    bpf = os.open(cfg["bpf"], os.O_RDONLY)
    cmd = [cfg["bwrap"], "--args", str(closure), "--args", str(r), "--seccomp", str(bpf), "--",
           cfg["env"], "--default-signal=INT"]
    if net:
        cmd.append(cfg["netwrap"])
    cmd += launch["argv"]
    # sb and the backend ignore INT while the run goes; the command's is the default again (env
    # --default-signal), so ^C reaches the command alone, and the after event survives it
    old = signal.signal(signal.SIGINT, signal.SIG_IGN)
    try:
        p = subprocess.Popen(cmd, env={}, pass_fds=(closure, r, bpf), close_fds=True)
        os.close(r)
        status = p.wait()
    finally:
        signal.signal(signal.SIGINT, old)
        if proxy:
            proxy.terminate()
            try:
                proxy.wait(2)
            except subprocess.TimeoutExpired:
                proxy.kill()
    if status < 0:
        status = 128 - status
    with open(os.path.join(rundir, "status"), "w") as f:
        f.write(str(status))
    return status


# ---------------------------------------------------------------- after

def after(cfg, rundir):
    policy, launch = load(rundir)
    try:
        with open(os.path.join(rundir, "before.json")) as f:
            before = json.load(f)
    except OSError:
        before = {"pins": [], "hostrun": {}, "refs": {}}
    ev = {"ev": "after", "by": "host", "commits": [], "unsigned": 0, "touches_host_run": [], "repo_kit_change": [],
          "root_repo_change": False, "new_git_metadata": [], "denials": []}
    for b in before["pins"]:
        top = b["top"]
        w = next(w for w in policy["workspace"] if w["path"] == top)
        pins = Pins(top, w["common"], w["protected"], cfg)
        pins.scan()
        new = sorted(set(fingerprint(cfg, pins)) - set(b["fingerprint"]))
        ev["new_git_metadata"] += [visible(n.replace(top + "/", "")) for n in new]
        now = hostrun_hashes(top, w["protected"])
        was = before["hostrun"].get(top, {})
        ev["touches_host_run"] += sorted(visible(p) for p in set(now) | set(was) if now.get(p) != was.get(p))
        old = set(before["refs"].get(top, {}).get("refs", []))
        cur = refs(cfg, top)
        newrefs = [l.split(" ", 1)[0] for l in cur["refs"] if l not in old]
        if newrefs:
            olds = [l.split(" ", 1)[0] for l in old]
            r = git(cfg, top, "rev-list", *newrefs, "--not", *olds) if olds else git(cfg, top, "rev-list", *newrefs)
            commits = [c for c in r.stdout.split() if c]
            ev["commits"] += [c[:12] for c in commits]
            if commits:
                sig = git(cfg, top, "log", "--no-walk", "--format=%G?", *commits).stdout.split()
                ev["unsigned"] += sum(1 for s in sig if s == "N")
                ev["base"] = before["refs"].get(top, {}).get("head", "")[:12]
        flake = [p for p in w["protected"] if not p.startswith("**/")]
        ev["repo_kit_change"] += [p for p in ev["touches_host_run"] if any(p == f or p.startswith(f + "/") for f in flake)]
    try:
        with open(os.path.join(rundir, "proxy.log")) as f:
            counts = {}
            for l in f:
                parts = l.split()
                if parts and parts[0] == "DENY":
                    k = " ".join(parts[1:])
                    counts[k] = counts.get(k, 0) + 1
            ev["denials"] = [{"host": visible(k), "count": n} for k, n in sorted(counts.items())]
    except OSError:
        pass
    print(json.dumps(ev))


def cleanup(rundir):
    for n in ("proxy.sock", "bwrap.args", "bwrap.args.all", "closure.args", "passwd", "group", "hosts"):
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
            prepare(cfg, rundir, print_cmd="--print" in argv[4:])
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
            policy, _ = load(rundir)
        except Exception:
            pass
        print(visible(f"graft sandbox ({policy.get('context', 'run')}): {e}"), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
