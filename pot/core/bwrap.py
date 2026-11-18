"""pot: the bubblewrap backend (design §11).

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
import shlex
import signal
import stat
import subprocess
import sys
import time

GUARANTEES = ["closure-store", "own-netns", "own-pidns", "no-tiocsti", "protected-as-mounts", "local-listen",
              "in-place", "same-path-clone", "private-proxy", "root-files"]
GITFILES = ["config", "config.worktree", "hooks", "info", "commondir", "gitdir"]
# S1: names host git reads in a git directory that a run could create when they are absent (commondir would point
# host git at a run-written config). pot puts a neutral placeholder at each absent one, read-only in the run, and
# removes it after: commondir "." (the directory itself), the others empty (host git verified to work with each).
PLACEHOLDERS = {"commondir": ".\n", "config.worktree": "", "gitdir": ""}
PLACED = []   # every placeholder this backend made, written to <run dir>/placeholders.json even when prepare stops
# §9: changed host-run files the after event reports, beyond the protect list
HOST_RUN_EXTRA = [".gitmodules", ".lfsconfig", ".gitattributes", ".github/workflows", ".forgejo/workflows",
                  ".gitlab-ci.yml"]
SECRET_RE = re.compile(r"^(\.env|\.env\..*|.*\.tfstate.*|id_(rsa|dsa|ecdsa|ed25519).*|\.git-credentials|\.netrc"
                       r"|.*\.pem|.*\.key)$")
PAYLOAD_DIRS = {".venv", "node_modules", "site-packages"}


class Stop(Exception):
    pass


def visible(s, lines=False):
    """Run-chosen text, safe for the host's terminal: C0 and C1 controls as \\xNN, Unicode format characters (Cf:
    bidi overrides and isolates, zero-width ones) as \\uNNNN (R8). LINES keeps a newline (a message's own lines).
    pot.py and bwrap.py hold the same function (bwrap's never keeps lines)."""
    import unicodedata
    out = []
    for c in s:
        o = ord(c)
        if c == "\n" and lines:
            out.append(c)
        elif o < 0x20 or 0x7f <= o <= 0x9f:
            out.append(f"\\x{o:02x}")
        elif o > 0x9f and unicodedata.category(c) == "Cf":
            out.append(f"\\u{o:04x}" if o <= 0xffff else f"\\U{o:08x}")
        else:
            out.append(c)
    return "".join(out)


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
        self.placeholders = []

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
            for name, body in PLACEHOLDERS.items():
                f = os.path.join(g, name)
                if not os.path.lexists(f) and os.path.isfile(os.path.join(g, "HEAD")):
                    with open(f, "x") as fh:
                        fh.write(body)
                    self.placeholders.append([f, body])
                    PLACED.append([f, body])
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
        # a protected folder is read-only throughout: a mount point pinned below it (a scion's folder,
        # the parent of its .git file) is bound read-only too, or its deeper bind would make it writable
        rod = [p for p in self.ro if os.path.isdir(p) and not os.path.islink(p)]
        for d in [d for d in self.dirs if any(d.startswith(r + "/") for r in rod)]:
            self.dirs.remove(d)
            if ("ro:" + d) not in self.seen:
                self.seen.add("ro:" + d)
                self.ro.append(d)
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


def binding(policy, cap):
    return [b for b in policy.get("bindings") or [] if b["capability"] == cap]


def safe_dest(root, rel):
    """ROOT/REL, refusing a link anywhere below ROOT: the run writes ROOT (its state), and the host
    must never write through a link the run planted (P3)."""
    p = root
    for part in rel.split("/")[:-1]:
        p = os.path.join(p, part)
        if os.path.islink(p):
            raise Stop(f"{p} is a symbolic link in the run's state: pot won't write through it")
        os.makedirs(p, exist_ok=True)
    return os.path.join(p, rel.split("/")[-1])


def set_key(d, path, value, workspaces):
    """d[path] = value, with "*" in path standing for each workspace path (projects.*.x)."""
    head, rest = path[0], path[1:]
    for k in (workspaces if head == "*" else [head]):
        if not rest:
            d[k] = value
        else:
            if not isinstance(d.get(k), dict):
                d[k] = {}
            sub = value
            set_key(d[k], rest, sub, workspaces)


def get_key(d, path):
    for k in path:
        if not isinstance(d, dict) or k not in d:
            return None
        d = d[k]
    return d


def into_state(policy, launch, i):
    """files in mode seed (copied into state on first use) or reassert (listed keys rewritten from
    the source at every start), written by the host into the run's state directory."""
    home = launch["home"]
    dest = home + i["value"][1:]
    st = next((s for s in policy["mounts"]["state"] if dest.startswith(home + s["at"][1:] + "/")), None)
    if st is None:
        raise Stop(f"{i['value']} ({i['mode']}) is not inside a state request's directory")
    rel = dest[len(home + st["at"][1:]) + 1:]
    target = safe_dest(st["dir"], rel)
    if os.path.islink(target):
        os.unlink(target)
    if i["mode"] == "seed":
        if i.get("refresh") or not os.path.lexists(target):
            with open(i["source"], "rb") as f, open(target + ".pot-tmp", "wb") as g:
                g.write(f.read())
            os.replace(target + ".pot-tmp", target)
        return
    with open(i["source"]) as f:
        src = json.load(f)
    try:
        with open(target) as f:
            cur = json.load(f)
        if not isinstance(cur, dict):
            cur = {}
    except (OSError, ValueError):
        cur = {}
    ws = [w["path"] for w in policy["workspace"]] or [launch["cwd"]]
    for k in i.get("keys") or []:
        path = k.split(".")
        v = get_key(src, [p for p in path])
        if v is None and "*" in path:
            v = get_key(src, path)
        set_key(cur, path, v, ws)
    with open(target + ".pot-tmp", "w") as f:
        json.dump(cur, f, indent=2)
    os.replace(target + ".pot-tmp", target)


# the not-found shims, last on PATH: advice for what a run often looks for (§8)
NOTFOUND = {
    "nix": "nix needs the nix capability: the repo kit's requests.capabilities = [ { name = \"nix\"; } ], then pot grant",
    "nix-build": "nix-build needs the nix capability", "nix-shell": "nix-shell needs the nix capability",
    "ssh": "ssh is not in this run: git fetch over ssh is the git.fetch capability; anything else, on the host",
    "scp": "scp is not in this run: run it on the host",
    "sudo": "sudo is host-only: pot host sudo …, or run it on the host",
    "systemctl": "systemctl is host-only (pot inspect diagnoses; pot host repairs)",
    "docker": "docker is host-only: its socket is root-equivalent",
    "home-manager": "home-manager is host-only: run it on the host",
    "darwin-rebuild": "darwin-rebuild is host-only: run it on the host",
    "nixos-rebuild": "nixos-rebuild is host-only: run it on the host",
}
# Only programs a run must not have get a shim: a shim for an ordinary name (node, say) would be found
# by programs that look for one on PATH and fall back without it (bun's node). Every other name gets
# bash's command_not_found_handle (BASH_ENV), and the startup check of the command itself.


def not_found_body(ctx, launch):
    """The not-found handler's body: the startup check's words and permanent fix."""
    alts = [a for a in (launch.get("pkgFix") or "programs.pot.tools\nor the repo kit's tools").split("\n") if a]
    body = "  one run:   pot --package PKG …\\n  permanent: " + alts[0] + "\\n" + "".join(f"             {a}\\n" for a in alts[1:])
    body = body.replace("%", "%%").replace("'", "'\\''")
    return ("{ printf 'pot (%s): %s is not in this run.\\n" + body + "' "
            f"'{ctx}' \"$1\" >&2; return 127; }}")


def write_own(cfg, rundir, policy, launch):
    ctx = policy["context"]
    b = os.path.join(rundir, "bin")
    nf = os.path.join(rundir, "notfound")
    os.makedirs(b, exist_ok=True)
    os.makedirs(nf, exist_ok=True)
    shim = f"#!/bin/sh\nexec {cfg['python']} -I -S {cfg['shim']} %s \"$@\"\n"
    own = {}
    if binding(policy, "status.relay"):
        own["workmux"] = shim % "workmux"
    if binding(policy, "git.fetch"):
        own["pot-ssh"] = shim % "ssh"
    for n, text in own.items():
        with open(os.path.join(b, n), "w") as f:
            f.write(text)
        os.chmod(os.path.join(b, n), 0o555)
    for n, msg in NOTFOUND.items():
        if n == "nix" and binding(policy, "nix"):
            continue
        with open(os.path.join(nf, n), "w") as f:
            f.write(f"#!/bin/sh\necho {shlex.quote(f'pot ({ctx}): ' + msg)} >&2\nexit 127\n")
        os.chmod(os.path.join(nf, n), 0o555)
    with open(os.path.join(rundir, "bash_env"), "w") as f:
        f.write("# pot: what a run's bash says for a program that is not in the run\n"
                f"command_not_found_handle() {not_found_body(ctx, launch)}\n")
    servers = {}
    for m in binding(policy, "mcp"):
        env = (m.get("env") or [None])[0]
        srv = {"type": "http", "url": m["url"]}
        if env:
            scheme = m.get("scheme", "Bearer")
            srv["headers"] = {m.get("header") or "Authorization": f"{scheme} ${{{env}}}" if scheme else f"${{{env}}}"}
        servers[m["server"]] = srv
    with open(os.path.join(rundir, "mcp.json"), "w") as f:
        json.dump({"mcpServers": servers}, f, indent=2)


def key_file(path):
    """A binding's key file: NAME=VALUE lines (`export NAME=VALUE` too; `#` comments and blank lines
    skipped; quotes stripped), or None when it holds no such line."""
    vals = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("export "):
                line = line[len("export "):].lstrip()
            k, eq, v = line.partition("=")
            if eq and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", k.strip()):
                v = v.strip()
                if len(v) >= 2 and v[0] == v[-1] and v[0] in "\"'":
                    v = v[1:-1]
                vals[k.strip()] = v
    return vals or None


def credentials(policy):
    """The values credential and mcp bindings pass by env (or file): read on the host at run time,
    never written to policy.json, launch.json or a log. {NAME: value}.

    from.file: NAME=VALUE lines naming the variables; a credential's file with no such line is one
    value, the whole file (a token file). An mcp key file must name its variable.
    from.env (or nothing): the host's variables of the same names."""
    out = {}
    for b in binding(policy, "credential") + binding(policy, "mcp"):
        src = b.get("from") or {}
        names = b.get("env") or []
        f = src.get("file") or src.get("envFile")
        if f:
            path = os.path.expanduser(f)
            try:
                vals = key_file(path)
                if vals is None:
                    if b["capability"] == "mcp" or len(names) != 1 or src.get("envFile"):
                        raise Stop(f"the {b['capability']} binding {b['selector']} reads {f}, which holds no "
                                   f"NAME=VALUE line for {', '.join(names) or 'its variable'}: write {names[0] if names else 'NAME'}=… in it")
                    with open(path) as fh:
                        vals = {names[0]: fh.read().strip()}
            except OSError as e:
                raise Stop(f"the {b['capability']} binding {b['selector']} reads {f}: {e.strerror}")
            missing = [n for n in names if n not in vals and n != "AWS_CREDENTIAL_EXPIRATION"]
            if missing:
                raise Stop(f"the {b['capability']} binding {b['selector']} reads {f}, which has no {', '.join(missing)}")
            out.update({n: vals[n] for n in names if n in vals})
        else:
            for n in names:
                v = os.environ.get(src["env"]) if isinstance(src.get("env"), str) and len(names) == 1 else os.environ.get(n)
                if v is not None:
                    out[n] = v
    return out


def mirrors_text(policy):
    """The README's lines for each mirror dir (kit 1.1's dirs mode mirror), by its label."""
    out = ""
    for lab in dict.fromkeys(i["mirror"] for i in policy["items"] if i.get("mirror")):
        cps = sorted({i["value"].rsplit("/", 1)[-1] for i in policy["items"] if i.get("mirror") == lab and i.get("refresh")})
        cp = " and ".join(cps)
        out += (f"- The host's {lab} for this workspace is read-only here" + (f"; {cp} {'is a writable copy' if len(cps) == 1 else 'are writable copies'}" if cps else "")
                + f". New {lab} files" + (f", and a changed {cp}," if cps else "") + " are copied out for the person to review when the run ends; "
                "nothing is written back to the host.\n")
    return out


def readme(policy, launch):
    ctx = policy["context"]
    return f"""# pot: this run ({ctx})

You are inside a pot run (pot, graft's sandbox), context `{ctx}`, access `{policy['access']}`.
The policy is /run/pot/policy.json; `pot why PATH|HOST|CMD|VAR` explains one item.

- HOME is empty but for what the policy places there. ~/.pot-hidden lists the paths
  hidden from every run: they are not mounted here, and cannot be written.
- The network goes only through the run's proxy, to the hosts the policy grants. When a client
  prints `CONNECT tunnel failed, response 403`, `tunnel error`, a 403, or `Could not resolve host`,
  the sandbox refused it: read /run/pot/denials, or run `pot why HOST`.
- A program that is not here is not in this run: `pot why NAME` says how to add it.
- Protected files (what decides the sandbox, what the host runs, agent configuration) are
  read-only, and their folders cannot be renamed or removed. Edit them on the host.
- Commits made here are unsigned; the host signs them afterwards.
{mirrors_text(policy)}- Every denial message starts `pot ({ctx}):`.
"""


def prepare(cfg, rundir, print_cmd=False):
    global EMPTY
    policy, launch = load(rundir)
    EMPTY = os.path.join(rundir, "empty")
    open(EMPTY, "w").close()
    os.chmod(EMPTY, 0o444)
    home, ctx = launch["home"], policy["context"]
    view = next((i for i in eff(policy, "view")), None)
    args = [["--unshare-all"], ["--unshare-user"], ["--disable-userns"], ["--die-with-parent"], ["--hostname", "pot"]]
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
        if binding(policy, "nix"):
            # the nix capability: the whole store, read-only (new build outputs appear at once; §8)
            closure = []
            args.append(["--ro-bind", "/nix/store", "/nix/store"])
        with open(cfg["baseClosure"]) as f:
            closure = sorted(set(closure) | {l.strip() for l in f if l.strip()})
        if net:
            with open(cfg["netClosure"]) as f:
                closure = sorted(set(closure) | {l.strip() for l in f if l.strip()})
        launch["closureCount"] = "the whole store (nix)" if binding(policy, "nix") else len(closure)
        with open(os.path.join(rundir, "closure.args"), "wb") as f:
            for p in closure:
                if not binding(policy, "nix"):
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
    clones = []
    for w in policy["workspace"]:
        top, common = w["path"], w["common"]
        if w.get("clone"):
            # clone mode (§9): the clone, bound at the original path (same-path-clone); its git directory, config
            # and hooks pinned as in place, so the run can't change what a later host git would run
            src = w["clone"]

            def at(p, src=src, top=top):
                return top + p[len(src):] if p == src or p.startswith(src + "/") else p
            pins = Pins(src, os.path.join(src, ".git"), w["protected"], cfg)
            pins.compute(os.path.join(src, ".git", "hooks"))
            clones.append((w, pins))
            mount(top, 0, ["--bind", src, top])
            for d in pins.dirs:
                mount(at(d), 0, ["--bind", d, at(d)])
            for p in pins.ro:
                mount(at(p), 1, ["--ro-bind", p, at(p)])
            continue
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
    for s in policy["mounts"]["state"]:
        os.makedirs(s["dir"], exist_ok=True)
        dest = home + s["at"][1:]
        mount(dest, 0, ["--bind", s["dir"], dest])
    for i in eff(policy, "file"):
        dest = i["value"] if not i["value"].startswith("~") else home + i["value"][1:]
        if i.get("mode", "fixed") == "fixed":
            mount(dest, 1, ["--ro-bind", i["source"], dest])
        else:
            into_state(policy, launch, i)
    for n, i in enumerate(eff(policy, "dir")):
        # a fixed directory (skills, agents, …): links to targets the run sees at their own paths
        d = os.path.join(rundir, "fixed", str(n))
        os.makedirs(d, exist_ok=True)
        dest = home + i["value"][1:]
        for entry in i.get("links") or []:
            name, target = entry[0], entry[1]
            if entry[2:] == ["bind"]:
                # a real entry: a placeholder in the view, the host entry bound onto it (its host path may lie
                # under this very mount point, which bwrap could not create a mount point in)
                if os.path.isdir(target):
                    os.mkdir(os.path.join(d, name))
                else:
                    open(os.path.join(d, name), "w").close()
                mount(f"{dest}/{name}", 1, ["--ro-bind", target, f"{dest}/{name}"])
                continue
            os.symlink(target, os.path.join(d, name))
            if not target.startswith("/nix/store/"):
                mount(target, 1, ["--ro-bind", target, target])
        mount(dest, 1, ["--ro-bind", d, dest])
    # pot's own: shims, the not-found handler, the MCP configuration, the bindings' sockets
    write_own(cfg, rundir, policy, launch)
    for n in ("explain.txt", "why.json"):
        if not os.path.exists(os.path.join(rundir, n)):
            with open(os.path.join(rundir, n), "w") as f:
                f.write("{}" if n.endswith(".json") else "")
    for n in ("bin", "notfound", "bash_env", "mcp.json", "explain.txt", "why.json"):
        mount(f"{run_mnt}/{n}", 1, ["--ro-bind", os.path.join(rundir, n), f"{run_mnt}/{n}"])
    for b in {b["socket"]: b for b in policy.get("bindings") or [] if b.get("socket")}.values():
        if b.get("socket"):
            sock = os.path.basename(b["socket"])
            mount(b["socket"], 1, ["--bind", os.path.join(rundir, sock), b["socket"]])
    with open(os.path.join(rundir, "hidden.txt"), "w") as f:
        f.write("".join(i["value"] + "\n" for i in eff(policy, "hidden")))
    mount(home + "/.pot-hidden", 1, ["--ro-bind", os.path.join(rundir, "hidden.txt"), home + "/.pot-hidden"])
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
        f.write("127.0.0.1 localhost pot\n::1 localhost pot\n")
    g = launch.get("git") or {}
    with open(os.path.join(rundir, "gitconfig"), "w") as f:
        f.write("# generated by pot: user.* from the host, no signing, no includes or helpers\n[user]\n")
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
    for w, pins in clones:
        before.setdefault("clones", []).append({"top": w["path"], "clone": w["clone"], "gits": sorted(pins.dotgits)})
    with open(os.path.join(rundir, "before.json"), "w") as f:
        json.dump(before, f)
    with open(os.path.join(rundir, "launch.json"), "w") as f:
        json.dump(launch, f)
    if print_cmd:
        print_command(cfg, rundir, policy, launch, args)


def run_env(cfg, policy, launch):
    """The run's environment, whole: what pot sets, and of the caller's only TERM, COLORTERM and the
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
    if launch.get("potBin"):
        paths.append(launch["potBin"])
    run_mnt = policy["mounts"]["run"]
    if binding(policy, "nix") and (launch.get("hostTools") or {}).get("nix"):
        paths.append(os.path.dirname(launch["hostTools"]["nix"]))
    paths = [f"{run_mnt}/bin"] + paths + [f"{run_mnt}/notfound"]
    env = {"HOME": home, "USER": launch["user"], "LOGNAME": launch["user"], "SHELL": cfg["shell"],
           "PATH": ":".join(paths), "LANG": "C.UTF-8", "TERMINFO_DIRS": cfg["terminfo"],
           "SSL_CERT_FILE": "/etc/ssl/certs/ca-certificates.crt", "NIX_SSL_CERT_FILE": "/etc/ssl/certs/ca-certificates.crt",
           "TZDIR": cfg["tzdir"], "POT": ctx, "POT_POLICY": f"{policy['mounts']['run']}/policy.json",
           "PS1": f"(pot:{ctx}) \\w \\$ "}
    if tools:
        env["POT_TOOLS"] = tools
    if policy.get("mode") == "clone":
        env["POT_CLONE"] = "1"
    for v in ("TERM", "COLORTERM"):
        if v in os.environ:
            env[v] = os.environ[v]
    for i in eff(policy, "env.set"):
        n, _, v = i["value"].partition("=")
        env[n] = home + v[1:] if v == "~" or v.startswith("~/") else v
    env["BASH_ENV"] = f"{run_mnt}/bash_env"
    # and exported, a backstop for a bash that skips BASH_ENV
    env["BASH_FUNC_command_not_found_handle%%"] = "() " + not_found_body(ctx, launch)
    if binding(policy, "git.fetch"):
        env["GIT_SSH_COMMAND"] = f"{run_mnt}/bin/pot-ssh"
        env["GIT_SSH_VARIANT"] = "ssh"
    if binding(policy, "nix"):
        env["NIX_REMOTE"] = f"unix://{run_mnt}/nix.sock"
        env["NIX_CONFIG"] = "experimental-features = nix-command flakes"
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
    for b in binding(policy, "credential") + binding(policy, "mcp"):
        for n in b.get("env") or []:
            env[n] = "(the run holds it)"
    print("# the bubblewrap command for this run (pot explain --backend-command)")
    print(f"env -i {cfg['bwrap']} --args 3 \\")
    for a in args:
        print("  " + " ".join(shlex.quote(x) for x in a) + " \\")
    for k in sorted(env):
        shown = env[k] if not any(k == i["value"] and i.get("secret") for i in eff(policy, "env")) else "(the run holds it)"
        print(f"  --setenv {k} {shlex.quote(shown)} \\")
    print("  -- " + " ".join(shlex.quote(x) for x in launch["argv"]))
    if "closureCount" in launch:
        print(f"# --args 3: --ro-bind for each of {launch['closureCount']} store paths (the closure)"
              if isinstance(launch["closureCount"], int) else f"# the store: {launch['closureCount']}, read-only")
    print(f"# fd 4: the TIOCSTI seccomp filter ({cfg['bpf']})")


# ---------------------------------------------------------------- run

def wait_for(paths, procs, what):
    for _ in range(500):
        if all(os.path.exists(p) for p in paths) or any(p.poll() is not None for p in procs):
            break
        time.sleep(0.01)
    if not all(os.path.exists(p) for p in paths):
        raise Stop(f"{what} did not start")


def start_bindings(cfg, rundir, policy, launch):
    """The host side of the run's bindings: the broker (status.relay, git.fetch) and the nix relay.
    Sockets are relative to the run directory (AF_UNIX allows 108 bytes; socat truncates silently)."""
    tools = launch.get("hostTools") or {}
    procs, socks = [], []
    log = open(os.path.join(rundir, "bindings.log"), "w")
    cmd = [cfg["python"], "-I", "-S", cfg["broker"], "--context", policy["context"],
           "--denials", os.path.join(rundir, "denials")]
    if binding(policy, "status.relay") and tools.get("workmux"):
        cmd += ["--status", "relay.sock", "--workmux", tools["workmux"]] + (["--pane", launch["pane"]] if launch.get("pane") else [])
        socks.append("relay.sock")
    elif binding(policy, "status.relay"):
        cmd += ["--status", "relay.sock", "--workmux", "/bin/false"]
        socks.append("relay.sock")
    gf = binding(policy, "git.fetch")
    if gf:
        if not tools.get("ssh"):
            raise Stop("git.fetch's broker needs ssh on the host's PATH")
        cmd += ["--git", "git.sock", "--ssh", tools["ssh"]]
        for b in gf:
            cmd += [x for r in b.get("remotes") or [] for x in ("--remote", r)]
            kf = (b.get("key") or {}).get("file")
            if kf:
                cmd += ["--key-file", os.path.expanduser(kf)]
        socks.append("git.sock")
    if socks:
        procs.append(subprocess.Popen(cmd, cwd=rundir, stdout=subprocess.DEVNULL, stderr=log))
    if binding(policy, "nix"):
        nix = tools.get("nix")
        if not nix:
            raise Stop("the nix capability's relay needs nix on the host's PATH")
        relay = os.path.join(rundir, "nix-relay")
        with open(relay, "w") as f:
            # verified on Nix 2.34.6: both features, every time (first checks, item 10)
            f.write(f"#!/bin/sh\nexec {nix} daemon --stdio --force-untrusted --process-ops "
                    "--extra-experimental-features 'daemon-trust-override mounted-ssh-store'\n")
        os.chmod(relay, 0o500)
        procs.append(subprocess.Popen([cfg["socat"], "-lf", "/dev/null", "UNIX-LISTEN:nix.sock,fork,unlink-early,mode=600",
                                       "EXEC:./nix-relay"], cwd=rundir, stdout=subprocess.DEVNULL, stderr=log))
        socks.append("nix.sock")
    wait_for([os.path.join(rundir, s) for s in socks], procs, "the run's bindings (" + ", ".join(socks) + ")")
    return procs


def run(cfg, rundir):
    policy, launch = load(rundir)
    net = [i for i in eff(policy, "network") if i["class"] == "reach"]
    proxy = None
    if net:
        allow = [i["value"] for i in net]
        log = open(os.path.join(rundir, "proxy.log"), "w")
        tmux = (launch.get("hostTools") or {}).get("tmux")
        live = ["--tmux", tmux, "--pane", launch["pane"]] if tmux and launch.get("pane") else []
        live += ["--fix", launch.get("netFix") or "the repo kit's requests.network"]
        proxy = subprocess.Popen([cfg["python"], "-I", "-S", cfg["proxy"], "proxy.sock",
                                  "--denials", os.path.join(rundir, "denials"), "--context", policy["context"]] + live + allow,
                                 cwd=rundir, stdout=log, stderr=subprocess.STDOUT)
        for _ in range(300):
            if os.path.exists(os.path.join(rundir, "proxy.sock")) or proxy.poll() is not None:
                break
            time.sleep(0.01)
        if not os.path.exists(os.path.join(rundir, "proxy.sock")):
            log.close()
            with open(os.path.join(rundir, "proxy.log")) as f:
                raise Stop("the network proxy did not start: " + f.read())
    helpers = start_bindings(cfg, rundir, policy, launch)
    env = run_env(cfg, policy, launch)
    env.update(credentials(policy))
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
    # pot and the backend ignore INT while the run goes; the command's is the default again (env
    # --default-signal), so ^C reaches the command alone, and the after event survives it
    old = signal.signal(signal.SIGINT, signal.SIG_IGN)
    try:
        p = subprocess.Popen(cmd, env={}, pass_fds=(closure, r, bpf), close_fds=True)
        os.close(r)
        status = p.wait()
    finally:
        signal.signal(signal.SIGINT, old)
        tools = launch.get("hostTools") or {}
        if binding(policy, "status.relay") and tools.get("workmux") and launch.get("pane"):
            # the agent is gone: clear its pane (workmux has no command that unregisters an agent)
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


# ---------------------------------------------------------------- after

def after(cfg, rundir):
    policy, launch = load(rundir)
    try:
        with open(os.path.join(rundir, "before.json")) as f:
            before = json.load(f)
    except OSError:
        before = {"pins": [], "hostrun": {}, "refs": {}}
    ev = {"ev": "after", "by": "host", "commits": [], "unsigned": 0, "touches_host_run": [], "repo_kit_change": [],
          "root_repo_change": False, "new_git_metadata": [], "denials": [], "signs": bool(launch.get("hostSigns"))}
    for b in before.get("clones") or []:
        clone_after(cfg, rundir, policy, launch, b, ev)
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
        # commits new on local branches and HEAD only: fetched ones (refs/remotes) are not the run's
        local = [l for l in cur["refs"] if l.split(" ", 1)[1].startswith("refs/heads/")]
        newrefs = [l.split(" ", 1)[0] for l in local if l not in old]
        if cur["head"] and cur["head"] != before["refs"].get(top, {}).get("head"):
            newrefs.append(cur["head"])
        if newrefs:
            olds = [l.split(" ", 1)[0] for l in old] + ([before["refs"][top]["head"]] if before["refs"].get(top, {}).get("head") else [])
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


# ---------------------------------------------------------------- clone mode: staging and its after event

def sandboxed_git(cfg, rundir, clone, args, cwd="/clone", timeout=600):
    """git inside bubblewrap: the clone read-only at /clone, git's closure, no network, no home. What the
    clone's config, hooks or attributes make it run, runs there (P4)."""
    with open(cfg["gitClosure"]) as f:
        closure = [l.strip() for l in f if l.strip()]
    cmd = [cfg["bwrap"], "--unshare-all", "--unshare-user", "--disable-userns", "--die-with-parent", "--new-session",
           "--clearenv", "--setenv", "HOME", "/tmp", "--setenv", "PATH", os.path.dirname(cfg["git"]),
           "--setenv", "GIT_CONFIG_NOSYSTEM", "1", "--setenv", "GIT_CONFIG_GLOBAL", "/dev/null"]
    for p in closure:
        cmd += ["--ro-bind", p, p]
    cmd += ["--ro-bind", clone, "/clone", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp", "--chdir", cwd,
            cfg["git"], "-c", "safe.directory=*"] + args
    return cmd


def upload_pack(cfg, rundir, clone, gitdir, name):
    """The --upload-pack program a host fetch runs: upload-pack, sandboxed, for one git directory of the clone."""
    path = os.path.join(rundir, f"upload-pack-{name}")
    inner = "/clone" + gitdir[len(clone):]
    with open(path, "w") as f:
        f.write("#!/bin/sh\n# pot: fetch-back's upload-pack, in bubblewrap (the clone read-only, nothing else of the host)\nexec "
                + " ".join(shlex.quote(a) for a in sandboxed_git(cfg, rundir, clone, ["upload-pack", "--strict", inner])) + "\n")
    os.chmod(path, 0o700)
    return path


def host_git(cfg, gitdir, *args, timeout=None):
    """Host git on a repository pot created (staging): none of the user's config, no hooks."""
    env = {**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1"}
    for k in [k for k in env if k.startswith("GIT_CONFIG_KEY_") or k.startswith("GIT_CONFIG_VALUE_") or k == "GIT_CONFIG_COUNT"]:
        del env[k]
    return subprocess.run([cfg["git"], "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false",
                           "--git-dir", gitdir] + list(args), capture_output=True, text=True, env=env, timeout=timeout)


def stage(cfg, rundir, clone, gitdir, name, alternates):
    """Fetch refs/heads/* of one git directory of the clone into <run dir>/staging/<name>.git (host-owned,
    bare), through the sandboxed upload-pack, with fsck. Returns the staging directory, or None."""
    sd = os.path.join(rundir, "staging", f"{name}.git")
    if not os.path.isdir(sd):
        os.makedirs(os.path.dirname(sd), exist_ok=True)
        subprocess.run([cfg["git"], "init", "-q", "--bare", sd], capture_output=True,
                       env={**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1"})
        if alternates and os.path.isdir(alternates):
            with open(os.path.join(sd, "objects", "info", "alternates"), "w") as f:
                f.write(alternates + "\n")
    up = upload_pack(cfg, rundir, clone, gitdir, name)
    opts = ["-c", "transfer.fsckObjects=true", "-c", "fetch.fsckObjects=true", "-c", "protocol.allow=never",
            "-c", "protocol.file.allow=always"]
    url = "file://" + gitdir
    upq = f"--upload-pack={shlex.quote(up)}"     # git runs it through a shell (R9)

    def failed(what, r):
        lines = [l for l in (r.stderr if r else "").strip().splitlines() if l.strip()] if r else []
        for l in lines[-6:]:     # F8: one line per line, never \x0a
            print(visible(f"pot: staging {name}: {l}"), file=sys.stderr)
        FETCH_FAILED.append((f"{name} " if name != "top" else "") + what + ": "
                            + (visible(lines[-1])[:200] if lines else (f"timed out" if r is None else f"git exit {r.returncode}")))

    try:
        ls = host_git(cfg, sd, *opts, "ls-remote", upq, url, timeout=600)
    except subprocess.TimeoutExpired:
        failed("listing its refs", None)
        return None
    if ls.returncode != 0:
        failed("listing its refs", ls)
        return None
    refs = []
    for l in ls.stdout.splitlines():
        sha, _, ref = l.partition("\t")
        if ref == "HEAD" or ref.startswith("refs/heads/"):
            refs.append(ref)
    # F3/R2: branch by branch, so one bad object does not hide the good branches
    good = 0
    for ref in refs:
        spec = "+HEAD:refs/pot/head" if ref == "HEAD" else f"+{ref}:{ref}"
        try:
            r = host_git(cfg, sd, *opts, "fetch", "-q", "--no-tags", upq, url, spec, timeout=600)
        except subprocess.TimeoutExpired:
            failed(ref, None)
            continue
        if r.returncode != 0:
            failed(ref, r)
            continue
        good += 1
    return sd if good else None


FETCH_FAILED = []   # S2: each failed staging fetch, an attention item (the run stays in review)


def tree_size(p):
    n = 0
    for d, _, fs in os.walk(p):
        for f in fs:
            try:
                n += os.lstat(os.path.join(d, f)).st_blocks * 512
            except OSError:
                pass
    return n


def matches(path, pats):
    for p in pats:
        if p.startswith("**/"):
            if p[3:] in path.split("/"):
                return True
        elif path == p.rstrip("/") or path.startswith(p.rstrip("/") + "/"):
            return True
    return False


def clone_after(cfg, rundir, policy, launch, b, ev):
    """The after event for a clone (§9), from the staging repositories, never from the clone."""
    clone, top = b["clone"], b["top"]
    w = next(w for w in policy["workspace"] if w["path"] == top)
    cl = launch.get("clone") or {}
    base = cl.get("base")
    sd = stage(cfg, rundir, clone, os.path.join(clone, ".git"), "top", cl.get("objects"))
    ev["base"] = base[:12] if base else ""
    ev["refs"], ev["scions"], ev["gitlinks"], ev["root_attention"] = {}, {}, [], []
    if sd:
        heads = host_git(cfg, sd, "for-each-ref", "--format=%(refname:strip=2) %(objectname)", "refs/heads").stdout.split("\n")
        moved = {}
        for l in heads:
            if " " in l:
                n, o = l.split(" ", 1)
                if o != base:
                    moved[n] = o
        # a detached HEAD that moved (a commit on no branch) comes back as pot-<id>/detached
        hd = host_git(cfg, sd, "rev-parse", "--verify", "--quiet", "refs/pot/head").stdout.strip()
        tips = {o for l in heads if " " in l for o in [l.split(" ", 1)[1]]}
        if hd and hd != base and hd not in tips:
            moved["detached"] = hd
        ev["refs"] = {n: o[:12] for n, o in moved.items()}
        # R7: each moved ref's own commits since the base, for the exit line
        ev["refCommits"] = {n: int(host_git(cfg, sd, "rev-list", "--count", o, "--not", base).stdout.strip() or 0)
                            for n, o in moved.items()}
        if moved:
            commits = [c for c in host_git(cfg, sd, "rev-list", *moved.values(), "--not", base).stdout.split() if c]
            ev["commits"] += [c[:12] for c in commits]
            for c in commits:
                body = host_git(cfg, sd, "cat-file", "commit", c).stdout
                head = body.split("\n\n", 1)[0]
                if "\ngpgsig " not in head and not head.startswith("gpgsig "):
                    ev["unsigned"] += 1
            changed = set()
            for o in moved.values():
                # R3: -z, so a path is never C-quoted (dossier-é/justfile stays itself); shown through visible()
                f = host_git(cfg, sd, "diff-tree", "-r", "-z", "--no-renames", "--raw", base, o).stdout.split("\0")
                for meta, path in zip(f[0::2], f[1::2]):
                    if not meta.startswith(":"):
                        continue
                    changed.add(path)
                    if "160000" in meta.split()[:2]:
                        ev["gitlinks"].append(visible(path))
            prot = w["protected"] + HOST_RUN_EXTRA
            ev["touches_host_run"] += sorted(visible(p) for p in changed if matches(p, prot))
            flake = [p for p in w["protected"] if not p.startswith("**/")]
            ev["repo_kit_change"] += [p for p in ev["touches_host_run"] if any(p == f or p.startswith(f + "/") for f in flake)]
            if launch.get("rootRepo"):
                ev["root_repo_change"] = True
                g = launch.get("grantsRel")
                att = {visible(p) for p in changed if g and p == g}
                # a changed file that sets programs.pot, at the run's tip or at the base (git grep reads objects)
                for o in moved.values():
                    for rev in (o, base):
                        out = host_git(cfg, sd, "grep", "-l", "--null", "-F", "programs.pot", rev, "--", *sorted(changed)).stdout
                        att |= {visible(e.split(":", 1)[1]) + " (programs.pot)" for e in out.split("\0") if ":" in e}
                ev["root_attention"] = sorted(att)
    # the scions: each submodule git directory the clone holds, staged the same way
    for m in cl.get("scions") or []:
        gd = os.path.join(clone, ".git", "modules", m["name"])
        if not os.path.isdir(gd):
            continue
        key = m["path"].replace("/", "-")
        ssd = stage(cfg, rundir, clone, gd, key, m.get("objects"))
        if not ssd:
            continue
        tips = [l.split(" ", 1)[1] for l in host_git(cfg, ssd, "for-each-ref", "--format=%(refname) %(objectname)").stdout.splitlines() if " " in l]
        tips = [t for t in tips if t != m["commit"]]
        cs = [c[:12] for c in host_git(cfg, ssd, "rev-list", *tips, "--not", m["commit"]).stdout.split()] if tips else []
        if cs:
            ev["scions"][m["path"]] = {"commits": cs}
    # what stays in the clone: uncommitted changes (read by the sandboxed git) and new git metadata
    r = subprocess.run(sandboxed_git(cfg, rundir, clone, ["--no-optional-locks", "status", "--porcelain", "--ignore-submodules=dirty"]),
                       capture_output=True, text=True, timeout=600)
    ev["uncommitted"] = len([l for l in r.stdout.splitlines() if l.strip()]) if r.returncode == 0 else None
    now = subprocess.run([cfg["find"], clone, "-name", ".git", "-prune", "-print0"], capture_output=True).stdout
    gits = {os.fsdecode(e) for e in now.split(b"\0") if e}
    ev["new_git_metadata"] += sorted(visible(top + g[len(clone):]).replace(top + "/", "") for g in gits - set(b.get("gits") or [])
                                     if g != os.path.join(clone, ".git"))
    ev["size"] = tree_size(clone) + tree_size(os.path.join(rundir, "staging"))
    if FETCH_FAILED:
        ev["fetch_failed"] = "; ".join(FETCH_FAILED)


def cleanup(rundir):
    # S1's placeholders: removed when they still hold what pot wrote (the run saw them read-only)
    try:
        with open(os.path.join(rundir, "placeholders.json")) as f:
            for path, body in json.load(f):
                try:
                    st = os.lstat(path)
                    if stat.S_ISREG(st.st_mode):
                        with open(path) as fh:
                            same = fh.read() == body
                        if same:
                            os.unlink(path)
                except OSError:
                    pass
    except (OSError, ValueError):
        pass
    for n in ("proxy.sock", "relay.sock", "git.sock", "nix.sock", "nix-relay", "bwrap.args", "bwrap.args.all",
              "closure.args", "passwd", "group", "hosts") + tuple(x for x in os.listdir(rundir) if x.startswith("upload-pack-")):
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
                if PLACED:
                    with open(os.path.join(rundir, "placeholders.json"), "w") as f:
                        json.dump(PLACED, f)
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
        print(visible(f"pot ({policy.get('context', 'run')}): {e}"), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
