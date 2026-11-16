"""graft sb: composition, a pure program (design §4, §7, §11).

compose(inputs) reads the root's manifest, the kits of a run, the grants file, the flags and host
facts, and returns policy.json v1 (Appendix A) with a report: refusals, notes, and each kit's grant
status. Nothing here touches the file system, the network or the environment: sb.py gathers the
host facts, and a Nix derivation can call compose() at build time with a store grants file.

Vocabulary is the design's: run, root, context, kit, request, capability, grant, binding, backend;
see, limits, reach.
"""
import base64
import fnmatch
import hashlib
import json
import posixpath
import re
from urllib.parse import urlsplit

POLICY_VERSION = 1
GRANTS_VERSION = 1
CANONICAL = 1
EXIT_GRANT = 125

VERBS = ("explain", "why", "grant", "review", "init", "run", "serve")
CONTEXT_FIELDS = {"extends", "mode", "access", "backend", "requires", "trust", "include", "commands",
                  "network", "nest"}
MODES = ("in-place", "clone", "none")
ACCESS = ("write", "read", "none")
ACCESS_RANK = {"none": 0, "read": 1, "write": 2}

# The contexts graft defines; a root adds its own and may change these (but not their names' meaning).
BUILTIN_CONTEXTS = {
    "host": {"mode": "none", "access": "none", "network": "granted"},
    "inspect": {"mode": "in-place", "access": "read", "trust": "workspace", "network": "granted"},
    "work": {"mode": "in-place", "access": "write", "trust": "workspace", "network": "granted"},
    "dispatch": {"mode": "clone", "access": "write", "trust": "workspace", "network": "granted"},
}
CONTEXT_DEFAULTS = {"mode": "in-place", "access": "write", "backend": "auto", "requires": [],
                    "trust": "workspace", "include": [], "commands": [], "network": "granted", "nest": True}

# Guarantee names (§11), an open set: these are the ones 1a names.
GUARANTEES = ("closure-store", "own-netns", "own-pidns", "no-tiocsti", "protected-as-mounts", "local-listen",
              "resource-limits", "separate-kernel", "in-place", "same-path-clone", "nest", "private-proxy",
              "root-files")

# M2: names sb owns, refused from kits' env.set
OWNED_ENV = {"BASH_ENV", "PATH", "HOME", "NIX_REMOTE", "SSL_CERT_FILE", "TZ", "HTTP_PROXY", "HTTPS_PROXY",
             "ALL_PROXY", "NO_PROXY", "http_proxy", "https_proxy", "all_proxy", "no_proxy"}

# graft's default hidden list (Appendix B); the root adds its own. Globs are expanded at start.
DEFAULT_HIDDEN = [
    "~/.ssh", "~/.gnupg", "~/.aws", "~/.azure", "~/.config/gcloud", "~/.kube", "~/.docker/config.json",
    "~/.netrc", "~/.git-credentials", "~/.config/gh", "~/.config/sops",
    "~/.claude", "~/.claude.json", "~/.pi/agent/auth.json", "~/.codex/auth.json",
    "~/.local/share/opencode/auth.json",
    "~/.local/share/keyrings", "~/.mozilla", "~/.config/google-chrome", "~/.config/chromium",
    "~/.config/BraveSoftware", "~/.config/vivaldi", "~/snap/firefox",
    "/etc/ssh/*_key",
]

# §9: the base protect list, in the workspace (the flake directory is added when sb evaluates one)
BASE_PROTECT = ["**/flake.nix", "**/flake.lock", "**/graft.nix", "**/.gitmodules", "**/.envrc", "**/justfile",
                "**/.pre-commit-config.yaml", ".githooks", "**/.claude", "**/.mcp.json", "**/.pi", "**/.codex",
                "**/.vscode"]

# §5: inspect's secret patterns (revision 4), as file-name patterns; *.pem and *.key only outside
# package payload directories
SECRET_NAMES = [".env", ".env.*", "*.tfstate*", "id_rsa*", "id_dsa*", "id_ecdsa*", "id_ed25519*",
                ".git-credentials", ".netrc"]
SECRET_NOT = ["*.pub"]
SECRET_PAYLOAD_NAMES = ["*.pem", "*.key"]
PAYLOAD_DIRS = [".venv", "node_modules", "site-packages"]

# §7.1: capabilities, reserved now; fields beyond these are an error
CAPABILITIES = {
    "status.relay": {"fields": {}, "key": None, "class": "ordinary", "phase": "1a"},
    "git.fetch": {"fields": {"remote": str}, "key": "remote", "class": "ordinary", "phase": "1a"},
    "credential": {"fields": {"host": str, "env": (str, list)}, "key": "host", "class": "powerful", "phase": "1a"},
    "mcp": {"fields": {"server": str}, "key": "server", "class": "powerful", "phase": "1a"},
    "nix": {"fields": {"build": (bool, str)}, "key": None, "class": "powerful", "phase": "1a"},
    "git.push": {"fields": {"remote": str, "refs": list}, "key": "remote", "class": "powerful", "phase": "2"},
    "git.sign": {"fields": {}, "key": None, "class": "powerful", "phase": "2"},
    "ops.read": {"fields": {}, "key": None, "class": "powerful", "phase": "2"},
    "delegate": {"fields": {"contexts": list, "commands": list, "workspaces": (str, list), "widen": dict},
                 "key": None, "class": "powerful", "phase": "3"},
}
REQUEST_KINDS = ("read", "write", "socket", "network", "env", "state", "capabilities", "unsandboxed")
KIT_FIELDS = {"packages", "env", "files", "protect", "requests", "commands", "recipes", "file", "kind"}
COMMAND_FIELDS = {"program", "access", "requests", "args", "fixedDirs", "skillDirs", "realDirs", "memory", "settings",
                  "description"}
FILE_MODES = ("fixed", "seed", "reassert")

CASE_INSENSITIVE_FORGES = {"github.com", "gitlab.com", "codeberg.org", "bitbucket.org"}
DEFAULT_PORTS = {"https": 443, "http": 80, "ssh": 22, "git": 9418}

HOST_RE = re.compile(r"[A-Za-z0-9*?_][A-Za-z0-9*?._-]*")
ENV_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
NAME_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._@-]*")


class Refusal(Exception):
    """A run that cannot start. status 125: a grant or a guarantee; 1 otherwise."""

    def __init__(self, message, status=1):
        super().__init__(message)
        self.message = message
        self.status = status


class KitError(Refusal):
    pass


# ---------------------------------------------------------------- paths

def norm_path(p, where):
    """A request or hidden path, normalised: "~/x" or "/x", no "..", no "." parts, no trailing "/"."""
    if not isinstance(p, str) or not (p == "~" or p.startswith("~/") or p.startswith("/")):
        raise KitError(f"{where}: {p!r} is not a path starting with \"~/\" or \"/\"")
    parts = p.split("/")
    if ".." in parts:
        raise KitError(f"{where}: {p!r} has \"..\", which is refused")
    head, rest = (parts[0], parts[1:])
    rest = [x for x in rest if x not in ("", ".")]
    if head == "~":
        return "~" + ("/" + "/".join(rest) if rest else "")
    return "/" + "/".join(rest)


def canon_path(p, home):
    """A path under the user's home recorded as ~/… (the canonical form, §7.4)."""
    if p == home:
        return "~"
    if p.startswith(home.rstrip("/") + "/"):
        return "~/" + p[len(home.rstrip("/")) + 1:]
    return p


def expand(p, home):
    if p == "~":
        return home
    if p.startswith("~/"):
        return home.rstrip("/") + "/" + p[2:]
    return p


def under(p, prefix):
    """p is prefix or below it (both absolute, normalised)."""
    return p == prefix or p.startswith(prefix.rstrip("/") + "/")


# ---------------------------------------------------------------- identity (§4.4)

def normalise_origin(url):
    """A remote URL as a repo kit's identity key, host[:port]/path; None for a local path.

    scp form becomes ssh; scheme, user and any token are dropped; the host is lowercased; a port is
    dropped only when it is the scheme's default; "//" collapses; a trailing "/" and ".git" are
    stripped; the path is lowercased only on case-insensitive forges. ssh config aliases are not
    resolved."""
    u = url.strip()
    if "://" not in u:
        m = re.match(r"^(?:[^@/]+@)?([^:/]+):(?!//)(.*)$", u)
        if not m:
            return None
        u = f"ssh://{m.group(1)}/{m.group(2)}"
    p = urlsplit(u)
    scheme = p.scheme.lower()
    if scheme == "file":
        return None
    host = (p.hostname or "").lower()
    if not host:
        return None
    try:
        port = p.port
    except ValueError:
        port = None
    if port is not None and port == DEFAULT_PORTS.get(scheme):
        port = None
    path = re.sub(r"/+", "/", p.path).strip("/")
    path = re.sub(r"\.git$", "", path).rstrip("/")
    if host in CASE_INSENSITIVE_FORGES:
        path = path.lower()
    return f"{host}{':%d' % port if port else ''}/{path}"


def repo_identity(origin, machine, common):
    key = normalise_origin(origin) if origin else None
    return f"repo:{key}" if key else f"repo:{machine}:{common}"


def scion_identity(url):
    """git:<url>, from .gitmodules, with any userinfo (a token) dropped."""
    u = url.strip()
    if "://" in u:
        p = urlsplit(u)
        if p.username or p.password:
            netloc = p.hostname + (f":{p.port}" if p.port else "")
            u = p._replace(netloc=netloc).geturl()
    return f"git:{u}"


def is_local_identity(ident):
    """A repo kit with no remote: its grant goes in its machine's section."""
    return ident.startswith("repo:") and ":/" in ident[5:] and not re.match(r"repo:[^/]*:\d+/", ident)


# ---------------------------------------------------------------- kits and requests

def _list(v, where, kind=str):
    if v is None:
        return []
    if not isinstance(v, list) or not all(isinstance(x, kind) for x in v):
        raise KitError(f"{where}: must be a list of {kind.__name__}s")
    return v


def parse_network(v, where):
    if not isinstance(v, str):
        raise KitError(f"{where}: {v!r}: the forms host-loopback:PORT and {{ host; via = \"forward\"; }} are reserved for a later phase")
    if v.startswith("host-loopback:"):
        raise KitError(f"{where}: {v}: host-loopback is reserved for a later phase")
    m = re.fullmatch(r"(.+?)(?::([0-9]{1,5}))?", v)
    host, port = m.group(1), m.group(2)
    if not HOST_RE.fullmatch(host) or (port is not None and not 1 <= int(port) <= 65535):
        raise KitError(f"{where}: {v!r}: give \"host\" or \"host:port\" (the host may use * and ?)")
    return f"{host.lower()}:{int(port) if port else 443}"


class Request:
    __slots__ = ("kind", "value", "canonical", "cls", "data", "kit", "file", "command")

    def __init__(self, kind, value, canonical, cls, data=None):
        self.kind, self.value, self.canonical, self.cls, self.data = kind, value, canonical, cls, data or {}
        self.kit = self.file = self.command = None

    def __repr__(self):
        return f"Request({self.canonical!r})"


def _cap_canonical(c):
    spec = CAPABILITIES[c["name"]]
    parts = ["capability", c["name"]]
    if spec["key"] and spec["key"] in c:
        parts.append(_cap_value(c[spec["key"]]))
    for k in sorted(c):
        if k in ("name", spec["key"]):
            continue
        parts.append(f"{k}={_cap_value(c[k])}")
    return " ".join(parts)


def _cap_value(v):
    if isinstance(v, list):
        return ",".join(_cap_value(x) for x in v)
    if isinstance(v, dict):
        return json.dumps(v, sort_keys=True, separators=(",", ":"))
    if isinstance(v, bool):
        return "true" if v else "false"
    return str(v)


def parse_requests(reqs, where, home):
    """A kit's (or command's) `requests` as Request objects. Unknown kinds and fields are errors."""
    if reqs is None:
        return []
    if not isinstance(reqs, dict):
        raise KitError(f"{where}: requests must be an attribute set")
    out = []
    for kind in reqs:
        if kind not in REQUEST_KINDS:
            raise KitError(f"{where}: requests.{kind} is not a request kind ({', '.join(REQUEST_KINDS)})")
    for kind in ("read", "write", "socket"):
        for p in _list(reqs.get(kind), f"{where}: requests.{kind}"):
            n = canon_path(expand(norm_path(p, f"{where}: requests.{kind}"), home), home)
            out.append(Request(kind, n, f"{kind} {n}", "ordinary" if kind == "read" else "powerful"))
    for h in reqs.get("network") or []:
        v = parse_network(h, f"{where}: requests.network")
        wild = any(ch in v for ch in "*?")
        out.append(Request("network", v, f"network {v}", "powerful" if wild else "ordinary"))
    for e in reqs.get("env") or []:
        if isinstance(e, str):
            name, secret = e, False
        elif isinstance(e, dict) and set(e) <= {"name", "secret"} and isinstance(e.get("name"), str):
            name, secret = e["name"], bool(e.get("secret", False))
        else:
            raise KitError(f"{where}: requests.env: {e!r} is not a name or {{ name; secret; }}")
        if not ENV_RE.fullmatch(name):
            raise KitError(f"{where}: requests.env: {name!r} is not a variable name")
        out.append(Request("env", name, f"env {name}", "ordinary", {"secret": secret}))
    for s in reqs.get("state") or []:
        if not isinstance(s, dict) or not set(s) <= {"name", "at", "shared"} or "name" not in s:
            raise KitError(f"{where}: requests.state: {s!r} is not {{ name; at; shared; }}")
        if not NAME_RE.fullmatch(s["name"]) or "@" in s["name"]:
            raise KitError(f"{where}: requests.state: {s['name']!r} is not a state name")
        at = norm_path(s.get("at", f"~/.local/state/{s['name']}"), f"{where}: requests.state.{s['name']}.at")
        if not at.startswith("~/"):
            raise KitError(f"{where}: requests.state.{s['name']}.at must be in the run's HOME (\"~/…\")")
        shared = bool(s.get("shared", False))
        out.append(Request("state", s["name"], f"state {s['name']}" + (" shared" if shared else ""),
                           "powerful" if shared else "ordinary", {"at": at, "shared": shared}))
    for c in reqs.get("capabilities") or []:
        if not isinstance(c, dict) or not isinstance(c.get("name"), str):
            raise KitError(f"{where}: requests.capabilities: {c!r} is not {{ name = \"…\"; … }}")
        spec = CAPABILITIES.get(c["name"])
        if spec is None:
            raise KitError(f"{where}: requests.capabilities: {c['name']!r} is not a capability ({', '.join(CAPABILITIES)})")
        for k, v in c.items():
            if k == "name":
                continue
            if k not in spec["fields"]:
                raise KitError(f"{where}: capability {c['name']}: field {k!r} is unknown to this version")
            if not isinstance(v, spec["fields"][k]):
                raise KitError(f"{where}: capability {c['name']}: field {k} has the wrong type")
        cc = _cap_canonical(c)
        out.append(Request("capability", c["name"], cc, spec["class"], dict(c)))
    u = reqs.get("unsandboxed")
    if u not in (None, False):
        if u is not True:
            raise KitError(f"{where}: requests.unsandboxed must be true or false")
        out.append(Request("unsandboxed", "", "unsandboxed", "powerful"))
    return out


def validate_kit(kit, where, home):
    """A kit manifest, checked: unknown fields are errors. Returns its requests (kit and commands)."""
    if not isinstance(kit, dict):
        raise KitError(f"{where}: a kit is an attribute set")
    for k in kit:
        if k not in KIT_FIELDS:
            raise KitError(f"{where}: {k} is not a kit field ({', '.join(sorted(KIT_FIELDS - {'file', 'kind'}))})")
    _list(kit.get("packages"), f"{where}: packages")
    env = kit.get("env") or {}
    if not isinstance(env, dict) or not set(env) <= {"set"}:
        raise KitError(f"{where}: env takes only env.set (a requested variable is requests.env)")
    for n, v in (env.get("set") or {}).items():
        if not ENV_RE.fullmatch(n) or not isinstance(v, str):
            raise KitError(f"{where}: env.set.{n}: a variable name with a string value")
        if n in OWNED_ENV or n.startswith("GRAFT_SANDBOX"):
            raise KitError(f"{where}: env.set.{n}: that name is sb's own")
    for path, f in (kit.get("files") or {}).items():
        norm_path(path, f"{where}: files")
        if not isinstance(f, dict) or "source" not in f or not set(f) <= {"source", "mode", "keys"}:
            raise KitError(f"{where}: files.\"{path}\": {{ source; mode; keys; }}")
        if f.get("mode", "fixed") not in FILE_MODES:
            raise KitError(f"{where}: files.\"{path}\".mode: one of {', '.join(FILE_MODES)}")
    for p in _list(kit.get("protect"), f"{where}: protect"):
        check_protect(p, f"{where}: protect")
    reqs = parse_requests(kit.get("requests"), where, home)
    for r in reqs:
        r.command = None
    cmds = kit.get("commands") or {}
    if not isinstance(cmds, dict):
        raise KitError(f"{where}: commands is an attribute set of commands")
    for name, c in cmds.items():
        if not NAME_RE.fullmatch(name) or "@" in name or ":" in name:
            raise KitError(f"{where}: commands.{name}: not a command name")
        if not isinstance(c, dict):
            raise KitError(f"{where}: commands.{name} is an attribute set")
        for k in c:
            if k not in COMMAND_FIELDS:
                raise KitError(f"{where}: commands.{name}.{k} is not a command field")
        fd = c.get("fixedDirs") or {}
        if not isinstance(fd, dict) or not all(isinstance(v, (str, type(None))) for v in fd.values()):
            raise KitError(f"{where}: commands.{name}.fixedDirs: {{ \"~/run/path\" = \"~/host/dir\" or null; }}")
        for at in fd:
            if not norm_path(at, f"{where}: commands.{name}.fixedDirs").startswith("~/"):
                raise KitError(f"{where}: commands.{name}.fixedDirs: {at} must be in the run's HOME")
        if c.get("access", "write") not in ("read", "write"):
            raise KitError(f"{where}: commands.{name}.access: read or write")
        for r in parse_requests(c.get("requests"), f"{where}: commands.{name}", home):
            r.command = name
            reqs.append(r)
    for n, body in (kit.get("recipes") or {}).items():
        if not isinstance(body, str) or "\n" in body:
            raise KitError(f"{where}: recipes.{n}: one line")
    return reqs


def check_protect(p, where):
    ok = isinstance(p, str) and p != "" and not p.startswith("/") and not p.startswith("~") \
        and ".." not in p.split("/") and (re.fullmatch(r"[^*?\[]*", p) or re.fullmatch(r"\*\*/[^/*?\[]+", p))
    if not ok:
        raise KitError(f"{where}: {p!r}: a path in the repository, relative to its top, or \"**/NAME\"")


def digest(canons):
    s = "\n".join(sorted(set(canons))) + "\n"
    return "sha256-" + base64.b64encode(hashlib.sha256(s.encode()).digest()).decode()


# ---------------------------------------------------------------- the grants file (§7.4)

def parse_grants(text):
    """The grants file, checked. A file that does not parse stops every run (Refusal)."""
    if text is None:
        return {"version": GRANTS_VERSION, "canonical": CANONICAL, "grants": {}, "machines": {}}
    try:
        g = json.loads(text)
    except ValueError as e:
        raise Refusal(f"the grants file does not parse ({e}): every run stops until it is fixed")
    if not isinstance(g, dict) or g.get("version") != GRANTS_VERSION or g.get("canonical") != CANONICAL:
        raise Refusal("the grants file is not version 1, canonical 1: every run stops until it is fixed")
    for k in g:
        if k not in ("version", "canonical", "grants", "machines"):
            raise Refusal(f"the grants file has an unknown field {k!r}: every run stops until it is fixed")
    sections = [("grants", g.get("grants", {}))] + [(f"machines.{m}", s) for m, s in g.get("machines", {}).items()]
    for name, sec in sections:
        if not isinstance(sec, dict):
            raise Refusal(f"the grants file's {name} is not an object: every run stops until it is fixed")
        for ident, e in sec.items():
            if not isinstance(e, dict) or not isinstance(e.get("requests"), list) \
                    or not set(e) <= {"digest", "requests", "source", "granted", "delegate"}:
                raise Refusal(f"the grants file's entry {ident} is malformed: every run stops until it is fixed")
            if e.get("digest") != digest(e["requests"]):
                raise Refusal(f"the grants file's entry {ident}: its digest does not match its requests: every run stops until it is fixed")
    g.setdefault("grants", {})
    g.setdefault("machines", {})
    return g


def grant_entry(grants, ident, machine):
    m = grants.get("machines", {}).get(machine, {})
    if ident in m:
        return m[ident], f"machines.{machine}"
    if ident in grants.get("grants", {}):
        return grants["grants"][ident], "grants"
    return None, None


def grant_status(ident, kind, canons, powerful, source, grants, machine, verified=True):
    """match | none (nothing requested) | never | grown | source | unverified, and the diff."""
    canons = sorted(set(canons))
    if not canons:
        return {"status": "none", "added": [], "entry": None}
    if not verified:
        return {"status": "unverified", "added": canons, "entry": None}
    entry, where = grant_entry(grants, ident, machine)
    if entry is None:
        return {"status": "never", "added": canons, "entry": None}
    added = [c for c in canons if c not in entry["requests"]]
    if added:
        return {"status": "grown", "added": added, "entry": entry, "section": where}
    if powerful and kind != "root" and source is not None and entry.get("source") != source:
        return {"status": "source", "added": [], "entry": entry, "section": where, "was": entry.get("source"),
                "now": source}
    return {"status": "match", "added": [], "entry": entry, "section": where}


def write_grant(grants, ident, canons, source, machine, when, local):
    """Record a grant: the shared section when there is no shared entry or the set is a superset of it;
    this machine's section only when the sets diverge (each holds items the other lacks), and always for
    a machine-bound identity (no remote). A machine entry overrides the shared one for its machine."""
    canons = sorted(set(canons))
    e = {"digest": digest(canons), "requests": canons}
    if source:
        e["source"] = source
    e["granted"] = when
    mach = grants.setdefault("machines", {})
    shared = grants.setdefault("grants", {})
    if local:
        mach.setdefault(machine, {})[ident] = e
        return f"machines.{machine}"
    old = shared.get(ident)
    if old is None or set(canons) >= set(old["requests"]):
        # a superset is safe in the shared entry: a run only gets what its own machine's kit asks for
        shared[ident] = e
        mine = mach.get(machine, {})
        if ident in mine:
            del mine[ident]
            if not mine:
                del mach[machine]
        return "grants"
    if set(canons) <= set(old["requests"]) and old.get("source") == e.get("source"):
        return "grants"      # asking for less needs no approval: the shared entry covers it
    mach.setdefault(machine, {})[ident] = e
    return f"machines.{machine}"


def revoke_grant(grants, ident, machine):
    m = grants.get("machines", {}).get(machine, {})
    if ident in m:
        del m[ident]
        if not m:
            del grants["machines"][machine]
        return f"machines.{machine}"
    if ident in grants.get("grants", {}):
        del grants["grants"][ident]
        return "grants"
    return None


def grants_dump(grants):
    return json.dumps(grants, indent=2, sort_keys=True) + "\n"


def arrived(old, new):
    """Identities whose entries differ between two parsed grants files (for "arrived by git")."""
    def flat(g):
        out = {f"grants/{k}": v for k, v in g.get("grants", {}).items()}
        for m, s in g.get("machines", {}).items():
            out.update({f"{m}/{k}": v for k, v in s.items()})
        return out
    a, b = flat(old), flat(new)
    return sorted({k.split("/", 1)[1] for k in set(a) | set(b) if a.get(k, {}).get("digest") != b.get(k, {}).get("digest")
                   or a.get(k, {}).get("source") != b.get(k, {}).get("source")})


# ---------------------------------------------------------------- contexts

def resolve_context(root, name):
    ctxs = dict(BUILTIN_CONTEXTS)
    for n, c in (root.get("contexts") or {}).items():
        ctxs[n] = {**ctxs.get(n, {}), **c}
    if name not in ctxs:
        raise Refusal(f"sb: no context {name!r} in this root (contexts: {', '.join(sorted(ctxs))})", 2)
    seen, chain, c = set(), [], ctxs[name]
    out = {}
    cur = name
    while True:
        if cur in seen:
            raise Refusal(f"sb: context {name}: extends forms a loop", 2)
        seen.add(cur)
        chain.append(ctxs[cur])
        nxt = ctxs[cur].get("extends")
        if not nxt:
            break
        if nxt not in ctxs:
            raise Refusal(f"sb: context {cur} extends {nxt!r}, which this root does not define", 2)
        cur = nxt
    for c in reversed(chain):
        out.update({k: v for k, v in c.items() if k != "extends"})
    for k in out:
        if k not in CONTEXT_FIELDS:
            raise Refusal(f"sb: context {name}: {k} is not a context field", 2)
    res = {**CONTEXT_DEFAULTS, **out, "name": name}
    if res["mode"] not in MODES or res["access"] not in ACCESS or res["network"] not in ("granted", "none"):
        raise Refusal(f"sb: context {name}: mode, access or network has an unknown value", 2)
    # inspect's view is the one context that carries reach (§5): only `inspect` itself, or one extending it
    res["view"] = "inspect" if (name == "inspect" or "inspect" in seen) else None
    return res


def context_names(root):
    return sorted(set(BUILTIN_CONTEXTS) | set(root.get("contexts") or {}))


# ---------------------------------------------------------------- composition

def kit_display(k):
    if k["kind"] == "root":
        return k["id"][len("root:"):]
    return k.get("name") or k["id"]


def find_command(root_kits, ws_kits, name):
    """§4.2: the command kit for NAME (or KIT:NAME). Root kits first, then workspace kits. Returns
    (kit, command name) or (None, name)."""
    kitname = None
    if ":" in name and not name.startswith("/"):
        kitname, name = name.split(":", 1)
    for rank in (root_kits, ws_kits):
        hits = [k for k in rank if name in (k["manifest"].get("commands") or {})
                and (kitname is None or kitname in (kit_display(k), k["id"], k.get("base")))]
        if len(hits) > 1:
            bases = {k.get("base") or k["id"] for k in hits}
            if len(bases) == 1 and all(k.get("instance") for k in hits):
                return hits, name   # account instances: the caller chooses
            raise Refusal(f"sb: {' and '.join(kit_display(k) for k in hits)} both ship {name}: sb KIT:{name}", 2)
        if hits:
            return hits[0], name
    if kitname is not None:
        raise Refusal(f"sb: no kit {kitname!r} ships a command {name!r}", 2)
    return None, name


def choose_instance(hits, account, default_account):
    by = {k["instance"]: k for k in hits}
    if account:
        if account not in by:
            raise Refusal(f"sb: --account {account}: this root declares {', '.join(sorted(by))}", 2)
        return by[account]
    if default_account in by:
        return by[default_account]
    raise Refusal(f"sb: {hits[0].get('base')} has the accounts {', '.join(sorted(by))}: choose one with --account", 2)


def compose(inp):
    """inp: {root, context, argv, flags, workspace, grants, machine, host, run, probe, backendName}.

    root: the root's manifest (kits keyed by name, contexts, hidden, rootRepos, repoKits, inspect,
      standard, tools, …).
    workspace: [{id, path, common, top, origin, repoKit: {manifest, file} | None, rootRepoKits: [...],
      scions: [{name, path, identity, url, commit, tag, verified, manifest}], tree, flakeDir, tools}]
    host: {home, user, env: {NAME: bool}, resolve: {path: realpath|None}, tz, terminal, agent}
    Returns (policy, report)."""
    root, home = inp["root"], inp["host"]["home"]
    ctx = resolve_context(root, inp["context"])
    flags = inp.get("flags") or {}
    report = {"notes": [], "stale": [], "kits": [], "refusals": [], "context": ctx, "command": None,
              "unsetEnv": [], "oneRun": [], "noBinding": []}
    machine = inp["machine"]
    grants = inp["grants"]

    if ctx["mode"] == "clone" or flags.get("mode") == "clone":
        raise Refusal(f"graft sandbox ({ctx['name']}): clone mode (dispatch) lands in 1b; in 1a, sb works in place")
    if ctx["mode"] == "none":
        raise Refusal("sb: host runs no sandbox and composes no policy")
    trust = ctx.get("trust", "workspace")
    if trust == "root":
        raise Refusal(f"graft sandbox ({ctx['name']}): trust = \"root\" lands in phase 2")

    # ---- the kits of the run (§4.1)
    kits = []

    def add_kit(ident, kind, manifest, file, name=None, **extra):
        reqs = validate_kit(manifest, f"{file} ({ident})", home)
        for r in reqs:
            r.kit, r.file = ident, file
        k = {"id": ident, "kind": kind, "manifest": manifest, "file": file, "requests": reqs, "name": name, **extra}
        kits.append(k)
        return k

    root_kits = []
    for n, m in sorted((root.get("kits") or {}).items()):
        base, _, inst = n.partition("@")
        root_kits.append(add_kit(f"root:{n}", "root", m, m.get("file") or "the root", n, base=base,
                                 instance=inst or None))
    ws_kits = []
    for w in inp["workspace"]:
        rk = w.get("repoKit") or {}
        manifests = []
        if rk.get("manifest") is not None:
            manifests.append((rk["manifest"], rk.get("file", "flake.nix (perSystem)")))
        for m, f in w.get("rootRepoKits") or []:
            manifests.append((m, f))
        merged = merge_manifests(manifests)
        w["kit"] = add_kit(w["id"], "repo", merged, " + ".join(f for _, f in manifests) or "(none)",
                           name=repo_short_name(w), source={"tree": w["source"]} if w.get("source") else None,
                           workspace=w)
        ws_kits.append(w["kit"])
        for s in w.get("scions") or []:
            if s.get("manifest") is None:
                continue
            k = add_kit(s["identity"], "scion", s["manifest"], s.get("file", f"{s['path']}/graft.nix"), s["name"],
                        verified=s.get("verified", False),
                        source={"commit": s["commit"], "tag": s["tag"]} if s.get("tag") else None, scion=s)
            ws_kits.append(k)

    # workspace kits may not ship names of root kits' commands, standard tools or sb (§4.2)
    root_cmds = {c for k in root_kits for c in (k["manifest"].get("commands") or {})}
    standard = set(root.get("standardNames") or []) | {"sb"}
    for k in ws_kits:
        for c in k["manifest"].get("commands") or {}:
            if c in root_cmds or c in standard:
                raise Refusal(f"graft sandbox ({ctx['name']}): {kit_display(k)} ships a command named {c}, "
                              f"which is {'sb' if c == 'sb' else 'a root kit command' if c in root_cmds else 'a standard tool'}: a workspace kit may not")

    # ---- the command kit
    argv = list(inp.get("argv") or [])
    cmdkit, cmdname = None, argv[0] if argv else None
    if argv and "/" not in argv[0]:
        hit, cmdname = find_command(root_kits, ws_kits, argv[0])
        if isinstance(hit, list):
            hit = choose_instance(hit, flags.get("account"), root.get("defaultAccount"))
        cmdkit = hit
    if cmdkit is not None and flags.get("account") and cmdkit.get("instance") is None:
        raise Refusal(f"sb: --account names an account instance, and {kit_display(cmdkit)} has none", 2)
    if cmdkit is None and flags.get("account"):
        raise Refusal("sb: --account chooses an agent kit's account; this command has no kit", 2)
    report["command"] = {"kit": cmdkit["id"] if cmdkit else None, "name": cmdname}
    if cmdkit and cmdkit.get("instance") and cmdkit["instance"] != root.get("defaultAccount"):
        report["notes"].append(f"account {cmdkit['instance']}")
    if cmdkit and cmdkit["kind"] == "scion" and not cmdkit.get("verified"):
        raise Refusal(f"graft sandbox ({ctx['name']}): {kit_display(cmdkit)} is not a verified release "
                      f"({cmdkit['id']}), so its command runs with no grant: sb host {cmdname}, or graft upgrade")

    included = []
    for n in ctx.get("include") or []:
        k = next((k for k in root_kits if k["name"] == n), None)
        if k is None:
            raise Refusal(f"sb: context {ctx['name']} includes {n!r}, which is not a root kit", 2)
        included.append(k)

    # which kits' requests apply to this run, and which kit's see items it gets
    applies = []      # (kit, requests that apply)
    if cmdkit:
        applies.append((cmdkit, [r for r in cmdkit["requests"] if r.command in (None, cmdname)]))
    for k in ws_kits:
        if k["kind"] == "repo" and k is not cmdkit and not ctx.get("view"):
            applies.append((k, [r for r in k["requests"] if r.command is None]))
    for k in included:
        if k is not cmdkit:
            applies.append((k, [r for r in k["requests"] if r.command is None]))
    applying_ids = {k["id"] for k, _ in applies}

    # ---- grants
    status = {}
    for k in kits:
        canons = [r.canonical for r in k["requests"]]
        powerful = any(r.cls == "powerful" for r in k["requests"])
        st = grant_status(k["id"], k["kind"], canons, powerful, k.get("source"), grants, machine,
                          verified=k.get("verified", True) if k["kind"] == "scion" else True)
        st["kit"] = k["id"]
        st["name"] = kit_display(k)
        st["kind"] = k["kind"]
        st["applies"] = k["id"] in applying_ids
        st["digest"] = digest(canons) if canons else None
        st["requests"] = sorted(set(canons))
        status[k["id"]] = st
        report["kits"].append(st)
        if st["status"] in ("grown", "source", "never") and not st["applies"]:
            if ctx.get("view") and k["kind"] == "repo":
                report["notes"].append(f"{st['name']} {STATUS_WORDS[st['status']]} (its reach does not apply in inspect)")
            elif st["status"] != "never":
                report["notes"].append(f"{st['name']} {STATUS_WORDS[st['status']]}: sb grant")
    for k, _ in applies:
        st = status[k["id"]]
        if st["status"] in ("grown", "source", "never"):
            report["stale"].append(st)
        if st["status"] == "unverified":
            report["stale"].append(st)

    # ---- items
    items = []

    def item(kind, value, cls, sources, effective=True, **extra):
        it = {"kind": kind, "value": value, "class": cls, "sources": sources, "effective": effective, **extra}
        items.append(it)
        return it

    def ksrc(k, as_):
        return {"kit": k["id"], "file": k["file"], "as": as_}

    # see: the root's standard set, tools, TZ, CA bundle
    for p in root.get("standard") or []:
        item("package", p, "see", [{"root": "standard"}])
    for p in root.get("tools") or []:
        item("package", p, "see", [{"root": "tools"}])
    if resolve_context(root, inp["context"]).get("view"):
        for p in root.get("inspectTools") or []:
            item("package", p, "see", [{"root": "inspect"}])
    see_kits = ([cmdkit] if cmdkit else []) + [k for k in included if k is not cmdkit]
    env_set, files = {}, {}
    for k in see_kits:
        for p in k["manifest"].get("packages") or []:
            item("package", p, "see", [ksrc(k, "see")])
        for n, v in ((k["manifest"].get("env") or {}).get("set") or {}).items():
            if n in env_set and env_set[n][0] != v:
                raise Refusal(f"graft sandbox ({ctx['name']}): env.set {n} has two values, from {env_set[n][1]} and {k['id']} (M2)")
            env_set[n] = (v, k["id"])
            item("env.set", f"{n}={v}", "see", [ksrc(k, "see")])
        for path, f in (k["manifest"].get("files") or {}).items():
            pth = norm_path(path, k["file"])
            if pth in files and files[pth][0] != f["source"]:
                raise Refusal(f"graft sandbox ({ctx['name']}): two different files at {pth}, from {files[pth][1]} and {k['id']} (M2)")
            if pth.startswith("/") and "root-files" not in (inp.get("probe") or []):
                raise Refusal(f"graft sandbox ({ctx['name']}): {k['id']} places a file at {pth}, which needs the root-files guarantee")
            files[pth] = (f["source"], k["id"])
            item("file", pth, "see", [ksrc(k, "see")], source=f["source"], mode=f.get("mode", "fixed"),
                 **({"keys": f["keys"]} if f.get("keys") else {}))
    # repo kits' packages reach every run; their env.set and files only their own commands (§4.2)
    for k in ws_kits:
        if k["kind"] == "repo" and k is not cmdkit:
            for p in k["manifest"].get("packages") or []:
                item("package", p, "see", [ksrc(k, "see")])
    for w in inp["workspace"]:
        if w.get("tools"):
            item("package", w["tools"], "see", [ksrc(w["kit"], "tools")], tools=True)
    for p in flags.get("package") or []:
        item("package", p, "see", [{"flag": "--package"}])
        report["oneRun"].append(("--package", p))
    if inp["host"].get("tz"):
        item("env.set", f"TZ={inp['host']['tz']}", "see", [{"root": "base"}])
    if root.get("caBundle"):
        item("file", "/etc/ssl/certs/ca-certificates.crt", "see", [{"root": "caBundle"}], source=root["caBundle"],
             mode="fixed")

    # limits
    access = ctx["access"]
    if flags.get("access"):
        if ACCESS_RANK[flags["access"]] > ACCESS_RANK[access]:
            raise Refusal(f"graft sandbox ({ctx['name']}): --access {flags['access']} would widen this context's access ({access})")
        access = flags["access"]
        report["oneRun"].append(("--access", flags["access"]))
    if cmdkit:
        ca = (cmdkit["manifest"].get("commands") or {}).get(cmdname, {}).get("access")
        if ca and ACCESS_RANK[ca] < ACCESS_RANK[access]:
            access = ca
    item("access", access, "limits", [{"context": ctx["name"]}])
    hidden = []
    for h in DEFAULT_HIDDEN:
        hidden.append((h, {"root": "graft"}))
    for h in root.get("hidden") or []:
        hidden.append((norm_path(h, "the root's hidden"), {"root": "hidden"}))
    for h in (root.get("grantsFile"), root.get("stateDir")):
        if h:
            hidden.append((canon_path(expand(h, home), home), {"root": "sb"}))
    for h in binding_files(root):
        hidden.append((canon_path(expand(norm_path(h, "a binding's from.file"), home), home), {"root": "binding"}))
    resolve = inp["host"].get("resolve") or {}
    resolve_hidden = inp["host"].get("hidden") or {}
    hidden_paths = []
    for h, src in hidden:
        reals = resolve_hidden.get(h)
        if reals is None:
            reals = [] if any(ch in h for ch in "*?[") else [expand(h, home)]
        hidden_paths.extend(reals)
        item("hidden", h, "limits", [src], paths=reals)
    protect_src = []
    for p in BASE_PROTECT:
        protect_src.append((p, {"root": "graft"}))
    pkits = [k for k in kits if k["kind"] in ("repo", "scion")] + ([cmdkit] if cmdkit and cmdkit["kind"] == "root" else []) \
        + [k for k in included if k is not cmdkit]
    for k in pkits:
        for p in k["manifest"].get("protect") or []:
            protect_src.append((p, ksrc(k, "limit")))
    for w in inp["workspace"]:
        if w.get("flakeDir"):
            protect_src.append((w["flakeDir"], {"root": "graft", "why": "the flake directory sb evaluates"}))
        if w.get("scionsDir"):
            protect_src.append((w["scionsDir"], {"root": "graft", "why": "the scions directory"}))
    protect = []
    for p, src in protect_src:
        if p not in protect:
            protect.append(p)
        item("protect", p, "limits", [src])
    network_ok = ctx["network"] == "granted"
    if not network_ok:
        item("network", "none", "limits", [{"context": ctx["name"]}])

    # reach
    for k in included:
        item("include", k["name"], "reach", [{"context": ctx["name"]}])
    view = ctx.get("view")
    if view:
        item("view", "inspect", "reach", [{"context": ctx["name"]}],
             home=[canon_path(expand(norm_path(h, "inspect.home"), home), home)
                   for h in ((root.get("inspect") or {}).get("home") or ["~/src"])],
             homeManager=(root.get("inspect") or {}).get("homeManagerFiles", True))
    state_mounts = []
    bound = []
    bindings = root.get("bindings") or {}
    for k, reqs in applies + [(k, []) for k in kits if k["id"] not in applying_ids]:
        st = status[k["id"]]
        granted = st["status"] == "match"
        applies_here = k["id"] in applying_ids
        for r in reqs if applies_here else k["requests"]:
            src = [ksrc(k, "request")]
            if granted:
                src.append({"grant": inp.get("grantsName", "grants.json"), "digest": st["entry"]["digest"]})
            eff, why = granted and applies_here, None
            if not applies_here:
                why = "not in this run"
            elif not granted:
                why = {"never": "not granted", "grown": "not granted", "source": "source changed",
                       "unverified": "unverified"}[st["status"]]
            extra = {}
            if r.kind == "network" and not network_ok:
                eff, why = False, f"context {ctx['name']} has no network"
            if view and r.kind in ("write", "socket"):
                eff, why = False, "inspect changes nothing"
            if view and r.kind == "network" and k is not cmdkit:
                eff, why = False, "inspect reaches only the command kit's model API"
            if view and r.kind == "capability" and r.value == "mcp":
                eff, why = False, "inspect has no MCP"
            if r.kind == "capability":
                extra["data"] = r.data
                b = find_binding(bindings, r, k["id"])
                if eff and b is None:
                    eff, why = False, "no binding"
                    report["noBinding"].append((k["id"], r))
                elif eff and CAPABILITIES[r.value]["phase"] != "1a":
                    eff, why = False, f"phase {CAPABILITIES[r.value]['phase']}"
                elif eff:
                    extra["binding"] = b[0]
                    bound.append((k, r, b))
            if r.kind == "unsandboxed" and eff:
                report["unsandboxed"] = k["id"]
            if r.kind in ("read", "write", "socket"):
                real = resolve.get(r.value)
                extra["path"] = real if isinstance(real, str) else expand(r.value, home)
                if eff:
                    bad = path_refusal(r.value, extra["path"], hidden_paths, inp)
                    if bad:
                        raise Refusal(f"graft sandbox ({ctx['name']}): {k['id']} requests {r.kind} {r.value}, {bad}")
            if r.kind == "env":
                extra["secret"] = r.data.get("secret", False)
                if eff and not inp["host"].get("env", {}).get(r.value):
                    report["unsetEnv"].append(r.value)
            if r.kind == "state":
                extra.update(at=r.data["at"], shared=r.data["shared"])
                if eff:
                    ws = inp["workspace"][0]["id"] if inp["workspace"] else "none"
                    key = [k["id"], "shared"] if r.data["shared"] else [k["id"], ws, ctx["name"]]
                    h = hashlib.sha256(json.dumps(key + [r.value]).encode()).hexdigest()[:20]
                    state_mounts.append({"name": r.value, "at": r.data["at"], "key": key,
                                         "dir": f"{inp['stateDir']}/state/{h}/{r.value}"})
            item(r.kind, r.value, "reach", src, eff, request=r.canonical, requestClass=r.cls,
                 **({"why": why} if why else {}), **extra)

    # one-run flags (§4.2: the person, for this run)
    for fl, kind in (("read", "read"), ("write", "write")):
        for p in flags.get(fl) or []:
            n = canon_path(expand(norm_path(p, f"--{fl}"), home), home)
            real = resolve.get(n)
            real = real if isinstance(real, str) else expand(n, home)
            bad = path_refusal(n, real, hidden_paths, inp)
            if bad:
                raise Refusal(f"graft sandbox ({ctx['name']}): --{fl} {p}: {bad}")
            eff = not (view and kind == "write")
            item(kind, n, "reach", [{"flag": f"--{fl}"}], eff, path=real, **({} if eff else {"why": "inspect changes nothing"}))
            report["oneRun"].append((f"--{fl}", n))
    for h in flags.get("net") or []:
        v = parse_network(h, "--net")
        item("network", v, "reach", [{"flag": "--net"}], network_ok,
             **({} if network_ok else {"why": f"context {ctx['name']} has no network"}))
        report["oneRun"].append(("--net", v))
    for e in flags.get("env") or []:
        if not ENV_RE.fullmatch(e) or e in OWNED_ENV or e.startswith("GRAFT_SANDBOX"):
            raise Refusal(f"sb: --env {e}: a variable name that is not sb's own", 2)
        item("env", e, "reach", [{"flag": "--env"}], True)
        if not inp["host"].get("env", {}).get(e):
            report["unsetEnv"].append(e)
        report["oneRun"].append(("--env", e))

    # ---- bindings (§7.2): how each effective capability is carried out
    pbind = [binding_entry(k, r, b, inp, ctx) for k, r, b in bound]
    report["bindings"] = pbind
    for b in pbind:
        if b["capability"] == "mcp" and network_ok:
            u = urlsplit(b["url"])
            hp = f"{(u.hostname or '').lower()}:{u.port or (80 if u.scheme == 'http' else 443)}"
            item("network", hp, "reach", [{"kit": b["kit"], "file": "the root", "as": "request"},
                                          {"binding": f"mcp.{b['selector']}"}], True, via="mcp")

    # ---- the command's argv: its program (a host path resolved at start, under a read request) and args
    report["argv"] = argv
    if cmdkit:
        c = (cmdkit["manifest"].get("commands") or {}).get(cmdname, {})
        prog = c.get("program")
        if isinstance(prog, dict):
            h = prog.get("host", "")
            hp = h if "/" not in h else canon_path(expand(norm_path(h, f"{cmdkit['file']}: commands.{cmdname}.program.host"), home), home)
            real = resolve.get(hp) or (expand(hp, home) if "/" in hp else hp)
            reads = [i["path"] for i in items if i["kind"] == "read" and i["effective"] and i.get("path")]
            # checked only once the kit's grant matches: a first run reaches the grant prompt (or 125) first
            if status[cmdkit["id"]]["status"] == "match" and not real.startswith("/nix/store/") \
                    and not any(under(real, r) for r in reads):
                raise Refusal(f"graft sandbox ({ctx['name']}): {cmdname}'s program {hp} resolves to {canon_path(real, home)}, "
                              f"outside {kit_display(cmdkit)}'s granted read requests")
            prog = real
        if prog or c.get("args"):
            report["argv"] = [prog or cmdname] + [str(a) for a in c.get("args") or []] + argv[1:]

    # ---- backend and guarantees
    proved = list(inp.get("probe") or [])
    if view or any(b["capability"] == "nix" for b in pbind):
        # inspect sees the whole store (§5); so does a run with the nix capability (§8)
        proved = [g for g in proved if g != "closure-store"]
    requires = list(ctx.get("requires") or [])
    missing = [g for g in requires if g not in proved]
    if missing:
        raise Refusal(f"graft sandbox ({ctx['name']}): requires {' '.join(missing)}; {inp.get('backendName', 'the backend')} proves {' '.join(proved) or 'nothing'}", EXIT_GRANT)

    workspace = [{"id": w["id"], "path": w["top"], "common": w["common"], "protected": protect if access == "write" else [],
                  "clone": None} for w in inp["workspace"]]
    policy = {
        "version": POLICY_VERSION,
        "run": inp["run"],
        "context": ctx["name"],
        "mode": "in-place",
        "access": access,
        "backend": {"name": inp.get("backendName", "bubblewrap"), "requires": requires, "proved": proved},
        "workspace": workspace,
        "items": items,
        "mounts": {"state": state_mounts, "run": "/run/graft-sandbox"},
        "bindings": pbind,
        "children": {},
        "grants": [{"id": st["kit"], "digest": st["digest"], "match": st["status"] in ("match", "none")}
                   for st in report["kits"]],
    }
    return policy, report


# the words people see for a grant's status (never the internal keys)
STATUS_WORDS = {"match": "granted", "none": "requests nothing", "never": "not granted", "grown": "asks for more",
                "source": "changed source", "unverified": "unverified"}

BINDING_SOCKETS = {"status.relay": "relay.sock", "git.fetch": "git.sock", "nix": "nix.sock"}


def cap_key(r):
    spec = CAPABILITIES[r.value]
    return r.data.get(spec["key"]) if spec["key"] else None


def find_binding(bindings, r, kit_id):
    """bindings.<capability>."<selector>" (§7.2): the selector is the capability's key (a host, a URL
    pattern, a server name) or "*", optionally narrowed to one kit: "api.anthropic.com kit=claude-code@x".
    The narrowest match wins: kit-narrowed, then exact, then a pattern, then "*". (selector, config) or None."""
    key = cap_key(r)
    best = None
    for sel, cfg in (bindings.get(r.value) or {}).items():
        base, _, kitpart = sel.partition(" kit=")
        if kitpart and kitpart not in (kit_id, kit_id.split(":", 1)[-1]):
            continue
        if base == "*":
            rank = 3
        elif key is None or base == key:
            rank = 1
        elif isinstance(key, str) and fnmatch.fnmatchcase(key, base):
            rank = 2
        else:
            continue
        rank -= 0.5 if kitpart else 0
        if best is None or rank < best[0]:
            best = (rank, sel, cfg)
    return None if best is None else (best[1], best[2])


def binding_files(root):
    """Files a binding names (from.file, key.file): hidden automatically (§7.2)."""
    out = []
    for cap in (root.get("bindings") or {}).values():
        for cfg in cap.values():
            for k in ("from", "key"):
                f = (cfg.get(k) or {}).get("file") if isinstance(cfg.get(k), dict) else None
                if f:
                    out.append(f)
            if cfg.get("from", {}).get("envFile") if isinstance(cfg.get("from"), dict) else None:
                out.append(cfg["from"]["envFile"])
    return out


def binding_entry(k, r, b, inp, ctx):
    """policy.json's bindings entry: what the backend sets up. Never a secret value: references only."""
    sel, cfg = b
    e = {"capability": r.value, "selector": sel, "kit": k["id"], "request": r.canonical}
    if r.value in BINDING_SOCKETS:
        e["socket"] = f"/run/graft-sandbox/{BINDING_SOCKETS[r.value]}"
    if r.value == "status.relay":
        e.update(see=["workmux shim"], allow=["set-window-status", "register-agent"])
    elif r.value == "git.fetch":
        remote = r.data.get("remote", "*")
        e.update(see=["ssh shim (GIT_SSH_COMMAND)"], remotes=[remote], key=cfg.get("key") or {"agent": True})
    elif r.value == "nix":
        e.update(see=["nix", "NIX_REMOTE"], via=cfg.get("via", "daemon-untrusted"),
                 notes=["the whole store is visible (closure-store dropped)",
                        "fixed-output derivations reach the network outside the proxy"])
    elif r.value in ("credential", "mcp"):
        env = r.data.get("env") if r.value == "credential" else cfg.get("env")
        env = [env] if isinstance(env, str) else list(env or [])
        via = cfg.get("via", "env")
        if via not in ("env", "file"):
            raise Refusal(f"graft sandbox ({ctx['name']}): bindings.{r.value}.\"{sel}\".via = {via!r}: 1a carries "
                          "env and file (proxy lands in phase 2)")
        e.update(via=via, env=env, **{"from": cfg.get("from") or {}}, holds="the run holds it",
                 **({"refresh": cfg["refresh"]} if cfg.get("refresh") else {}))
        if r.value == "mcp":
            e.update(server=r.data.get("server"), url=cfg.get("url"), header=cfg.get("header", "Authorization"),
                     scheme=cfg.get("scheme", "Bearer"), config="/run/graft-sandbox/mcp.json")
            if not cfg.get("url"):
                raise Refusal(f"graft sandbox ({ctx['name']}): bindings.mcp.\"{sel}\" needs a url")
    return e


def repo_short_name(w):
    """A repo kit's short name: its normalised origin's last path part, else the checkout's folder."""
    return w["originKey"].rstrip("/").rsplit("/", 1)[-1] if w.get("originKey") else posixpath.basename(w["top"])


def merge_manifests(ms):
    """A repo kit from its perSystem `kit` and the root's repoKits matches: union of lists, M2 for
    values."""
    if len(ms) == 1:
        return ms[0][0]
    out = {}
    for m, f in ms:
        for k, v in m.items():
            if k == "file":
                continue
            if isinstance(v, list):
                out[k] = out.get(k, []) + [x for x in v if x not in out.get(k, [])]
            elif isinstance(v, dict):
                cur = out.setdefault(k, {})
                for kk, vv in v.items():
                    if isinstance(vv, list):
                        cur[kk] = cur.get(kk, []) + [x for x in vv if x not in cur.get(kk, [])]
                    elif isinstance(vv, dict):
                        for kkk, vvv in vv.items():
                            if kkk in cur.setdefault(kk, {}) and cur[kk][kkk] != vvv:
                                raise KitError(f"{f}: {k}.{kk}.{kkk} has two values (M2)")
                            cur[kk][kkk] = vvv
                    else:
                        if kk in cur and cur[kk] != vv:
                            raise KitError(f"{f}: {k}.{kk} has two values (M2)")
                        cur[kk] = vv
            else:
                out[k] = v
    return out


def path_refusal(canon, real, hidden_paths, inp):
    """M3/M4: why a reach path is refused, or None."""
    sd = inp.get("stateDir")
    if sd and (under(real, sd) or under(sd, real)):
        return "which is sb's own state (launcher mounts are sb's alone)"
    for h in hidden_paths:
        if under(real, h):
            return f"which resolves into a hidden path ({canon_path(h, inp['host']['home'])})"
    # a hidden path below a granted one stays hidden (masked at every depth)
    return None


def pattern_key(p):
    """A repository pattern as an identity key: a URL is normalised, a key (host[:port]/path) kept."""
    return (normalise_origin(p) or p) if ("://" in p or "@" in p) else p


def root_repo_match(top, origin_key, root_repos, home):
    """§6.4: is this repository one of the root's source repositories?"""
    for r in root_repos or []:
        if r.startswith("~") or r.startswith("/"):
            if top == expand(r, home).rstrip("/"):
                return r
        else:
            k = pattern_key(r)
            if origin_key and fnmatch.fnmatchcase(origin_key, k):
                return r
    return None


def repo_kits_for(origin_key, ident, repo_kits):
    out = []
    for pat, m in sorted((repo_kits or {}).items()):
        k = pattern_key(pat)
        if (origin_key and fnmatch.fnmatchcase(origin_key, k)) or ident == pat or ident == f"repo:{k}":
            out.append((m, f"programs.sb.repoKits.\"{pat}\""))
    return out


# ---------------------------------------------------------------- people-facing text

def describe_request(c):
    return c


def stale_message(ctxname, st):
    lines = []
    if st["status"] == "unverified":
        lines.append(f"graft sandbox ({ctxname}): {st['name']} ({st['kit']}) is not a verified release, so its grant does not apply")
        return "\n".join(lines)
    if st["status"] == "never":
        lines.append(f"graft sandbox ({ctxname}): {st['name']} ({st['kit']}) requests reach that is not granted:")
    elif st["status"] == "grown":
        lines.append(f"graft sandbox ({ctxname}): {st['name']} ({st['kit']}) asks for more than its grant:")
    else:
        lines.append(f"graft sandbox ({ctxname}): {st['name']} ({st['kit']}) changed source since its grant:")
        lines.append(f"  source: {fmt_source(st.get('was'))} -> {fmt_source(st.get('now'))}")
        for d in st.get("diffstat") or []:
            lines.append(f"    {d}")
    for a in st["added"]:
        lines.append(f"  + {a}")
    lines.append(f"  grant it: sb grant {st['name']}")
    return "\n".join(lines)


def fmt_source(s):
    if not s:
        return "(none)"
    if "tag" in s:
        return f"{s['tag']} ({s['commit'][:12]})"
    if "tree" in s:
        return f"tree {s['tree'][:12]}"
    return json.dumps(s, sort_keys=True)


def permanent_fix(flag, value, report, root):
    """§12.2: the one permanent fix for a one-run flag."""
    kind = {"--net": "requests.network", "--read": "requests.read", "--write": "requests.write",
            "--env": "requests.env", "--package": "packages", "--access": "access"}[flag]
    cmd = report.get("command") or {}
    inc = [n for n in (report["context"].get("include") or [])]
    if flag == "--access":
        return f"a context with access = \"{value}\" (programs.sb.contexts)"
    if flag == "--package":
        return "programs.sb.tools (home-manager switch), or the repo kit's packages"
    if cmd.get("kit") and not cmd["kit"].startswith("root:") and not cmd["kit"].startswith("repo:"):
        return f"{cmd['kit']}'s {kind} (a kit change)"
    alt = f", or programs.sb.kits.{inc[0]}.{kind} for every repository (home-manager switch)" if inc else ""
    return f"the repo kit's {kind} (perSystem.kit){alt}"


def explain_text(policy, report, root):
    """sb explain: every item with the file or grant it came from (Appendix C's shape)."""
    out = []
    ctx = report["context"]
    cmd = report.get("command") or {}
    eff = [i for i in policy["items"] if i["effective"]]

    def src(i):
        s = []
        for x in i["sources"]:
            if "kit" in x:
                s.append(x["kit"].replace("root:", "", 1))
            elif "flag" in x:
                s.append(f"{x['flag']} (one run)")
            elif "root" in x:
                s.append(f"root {x['root']}")
            elif "context" in x:
                s.append(f"context {x['context']}")
        return ", ".join(dict.fromkeys(s))

    out.append(f"run        {cmd.get('name') or '(shell)'}" + (f" -> {cmd['kit']}" if cmd.get("kit") else " (no kit ships it)"))
    if report.get("argv") and report["argv"][:1] != [cmd.get("name")]:
        out.append("argv       " + " ".join(report["argv"]))
    inc = ctx.get("include") or []
    out.append(f"context    {ctx['name']}: {policy['mode']}, access {policy['access']}, {policy['backend']['name']}"
               + (f"; includes {', '.join('root:' + n for n in inc)}" if inc else ""))
    for w in policy["workspace"]:
        out.append(f"workspace  {w['id']} ({policy['access']}) {w['path']}")
    scions = [k for k in report["kits"] if k["kind"] == "scion"]
    if scions:
        out.append("           scions: " + ", ".join(f"{k['name']} ({k['kit']})" + ("" if k["status"] != "unverified" else " unverified") for k in scions))
    pk = [i for i in eff if i["kind"] == "package"]
    out.append(f"see        {len(pk)} packages: " + " · ".join(f"{posixpath.basename(i['value'])[33:] or i['value']} ({src(i)})" for i in pk))
    for i in eff:
        if i["kind"] == "dir":
            out.append(f"           dir {i['value']} (read-only): {len(i.get('links') or [])} entries"
                       + (f", {len(i['skipped'])} left out (outside the read requests)" if i.get("skipped") else ""))
        if i["kind"] in ("env.set", "file"):
            out.append(f"           {i['kind']} {i['value']}" + (f" ({i.get('mode')})" if i["kind"] == "file" else "") + f"  [{src(i)}]")
    prot = [i for i in eff if i["kind"] == "protect"]
    if policy["access"] == "write":
        out.append("limits     protect: " + " · ".join(f"{i['value']} ({src(i)})" for i in prot))
    else:
        out.append(f"limits     the workspace is read-only (access {policy['access']})")
    hid = [i for i in eff if i["kind"] == "hidden"]
    out.append(f"           hidden: {len(hid)} paths (sb explain --json lists them)")
    if any(i["kind"] == "network" and i["value"] == "none" for i in policy["items"]):
        out.append(f"           network: none (context {ctx['name']})")
    out.append(f"reach{' ' * 60}granted")
    view = [i for i in eff if i["kind"] == "view"]
    for v in view:
        out.append(f"  view     inspect: the machine read-only, minus secrets; in ~: {' '.join(v['home'])}")
    for kind in ("network", "read", "write", "socket", "env", "state", "capability", "unsandboxed"):
        rows = [i for i in policy["items"] if i["kind"] == kind and i["class"] == "reach"]
        for i in rows:
            v = i["value"] if kind != "capability" else i.get("request", i["value"]).replace("capability ", "", 1)
            if kind == "state":
                v = f"{i['value']} -> {i.get('at')}" + (" (shared)" if i.get("shared") else "")
            if kind == "env" and i.get("secret"):
                v += " (the run holds it)"
            status = src(i) if i["effective"] else f"{src(i)}: {i.get('why', 'not in run')}"
            if kind == "capability" and i["effective"]:
                b = next((b for b in policy.get("bindings") or [] if b["request"] == i.get("request") and b["kit"] in str(i["sources"])), None)
                if b:
                    how = b.get("via") or ", ".join(b.get("see") or []) or "bound"
                    if b.get("holds"):
                        how += " (the run holds it)"
                    v += f": {how}" + (f"; {'; '.join(b['notes'])}" if b.get("notes") else "")
            out.append(f"  {kind[:8]:<8} {v:<55} {status}")
    out.append("children   none (nesting lands in 1b)")
    stale = [k for k in report["kits"] if k["status"] in ("never", "grown", "source", "unverified")]
    for k in stale:
        out.append(f"not in run {k['name']} ({k['kit']}): {k['status']}" + (": " + ", ".join(k["added"][:4]) + (" …" if len(k["added"]) > 4 else "") if k["added"] else ""))
    b = policy["backend"]
    lacks = [g for g in GUARANTEES if g not in b["proved"]]
    out.append(f"backend    {b['name']} proves {' '.join(b['proved']) or 'nothing'}; lacks {' '.join(lacks)}")
    out.append("note       phase 1a: kit tools run as the run's own programs, not as sandboxed commands (D4 lands in 1b)")
    return "\n".join(out) + "\n"


def validate_root(root):
    """The root's manifest, checked when it builds: its kits, repo kits and contexts."""
    for n, k in (root.get("kits") or {}).items():
        if not NAME_RE.fullmatch(n) or n.count("@") > 1:
            raise KitError(f"the root's kits: {n!r} is not a kit name (NAME or NAME@ACCOUNT)")
        validate_kit(k, k.get("file", f"programs.sb.kits.{n}"), "/home/user")
    for n, k in (root.get("repoKits") or {}).items():
        validate_kit(k, k.get("file", f"programs.sb.repoKits.{n}"), "/home/user")
    for n in (root.get("contexts") or {}):
        if n in VERBS:
            raise KitError(f"the root's contexts: {n} is an sb verb")
        resolve_context(root, n)
    for h in root.get("hidden") or []:
        norm_path(h, "the root's hidden")
    for h in (root.get("inspect") or {}).get("home") or []:
        norm_path(h, "the root's inspect.home")
    for b in root.get("bindings") or {}:
        if b not in CAPABILITIES:
            raise KitError(f"the root's bindings: {b} is not a capability ({', '.join(CAPABILITIES)})")


if __name__ == "__main__":
    import sys
    try:
        with open(sys.argv[2]) as f:
            data = json.load(f)
        if sys.argv[1] == "validate":
            # graft-kits: the repo kit and each provider's
            if data.get("self") is not None:
                validate_kit(data["self"], data.get("file", "perSystem.kit"), "/home/user")
            for n, k in (data.get("scions") or {}).items():
                validate_kit(k, k.get("file", n), "/home/user")
        elif sys.argv[1] == "validate-root":
            validate_root(data)
        else:
            raise SystemExit(f"usage: compose.py validate|validate-root FILE")
    except Refusal as e:
        print(f"graft: {e.message}", file=sys.stderr)
        sys.exit(1)
