"""pot: composition, a pure program (design §4, §7, §11).

compose(inputs) reads the root's manifest, the kits of a run, the grants file, the flags and host
facts, and returns policy.json v1 (Appendix A) with a report: refusals, notes, and each kit's grant
status. Nothing here touches the file system, the network or the environment: pot.py gathers the
host facts, and a Nix derivation can call compose() at build time with a store grants file.

Vocabulary is the design's: run, root, context, kit, request, capability, grant, binding, backend;
see, limits, reach.
"""
import base64
import fnmatch
import hashlib
import json
import os
import posixpath
import re
from urllib.parse import urlsplit

POLICY_VERSION = 1
GRANTS_VERSION = 1
CANONICAL = 1
EXIT_GRANT = 125

VERBS = ("explain", "why", "grant", "review", "init", "run", "serve", "runs", "doctor")
CONTEXT_FIELDS = {"extends", "mode", "access", "backend", "requires", "trust", "include", "commands",
                  "network", "nest"}
MODES = ("in-place", "clone", "none")
ACCESS = ("write", "read", "none")
ACCESS_RANK = {"none": 0, "read": 1, "write": 2}

# The contexts pot defines; a root adds its own and may change these (but not their names' meaning).
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

# M2: names pot owns, refused from kits' env.set
OWNED_ENV = {"BASH_ENV", "PATH", "HOME", "NIX_REMOTE", "SSL_CERT_FILE", "TZ", "HTTP_PROXY", "HTTPS_PROXY",
             "ALL_PROXY", "NO_PROXY", "http_proxy", "https_proxy", "all_proxy", "no_proxy"}

def owned(name):
    """A variable name pot owns: refused from kits' env.set and --env. POT* is pot's; GRAFT_SANDBOX* was
    v2.14.0's sandbox's, and is refused too, so that no kit can fake one."""
    return name in OWNED_ENV or name.startswith("POT") or name.startswith("GRAFT_SANDBOX")


# pot's default hidden list (Appendix B); the root adds its own. Globs are expanded at start.
DEFAULT_HIDDEN = [
    "~/.ssh", "~/.gnupg", "~/.aws", "~/.azure", "~/.config/gcloud", "~/.kube", "~/.docker/config.json",
    "~/.netrc", "~/.git-credentials", "~/.config/gh", "~/.config/sops",
    "~/.local/share/keyrings", "~/.mozilla", "~/.config/google-chrome", "~/.config/chromium",
    "~/.config/BraveSoftware", "~/.config/vivaldi", "~/snap/firefox",
    "/etc/ssh/*_key",
]

# §9: the base protect list, in the workspace (the flake directory is added when pot evaluates one, and
# its pot-kits bundle adds the producer's own: graft's mkFlake, every graft.nix and its scions directory)
# (agents' own folders, credential stores and markers are the root's floor: root.nix's baseHidden,
# baseProtect and agentMarkers, which reach pot in the root's manifest)
BASE_PROTECT = ["**/flake.nix", "**/flake.lock", "**/.gitmodules", "**/.envrc", "**/justfile",
                "**/.pre-commit-config.yaml", ".githooks", "**/.mcp.json", "**/.vscode"]

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
# kit/1 (missing version: 1.0). 1.1 adds version, agent and a command's dirs, and drops the command
# fields dirs replaces. tools is packages' new name in both (packages stays as its alias).
# 1.2 (v2.17.0 reads it; producers write it from v2.18.0) adds a command's hook and recipes as attrs.
KIT_VERSIONS = ("1.0", "1.1", "1.2")
KIT_FIELDS = {"1.0": {"packages", "tools", "env", "files", "protect", "requests", "commands", "recipes", "file", "kind",
                      "version"},
              "1.1": {"packages", "tools", "env", "files", "protect", "requests", "commands", "recipes", "file", "kind",
                      "version", "agent"},
              "1.2": {"packages", "tools", "env", "files", "protect", "requests", "commands", "recipes", "file", "kind",
                      "version", "agent"}}
COMMAND_FIELDS = {"1.0": {"program", "access", "requests", "args", "fixedDirs", "skillDirs", "realDirs", "memory",
                          "settings", "description"},
                  "1.1": {"program", "access", "requests", "args", "dirs", "settings", "description"},
                  "1.2": {"program", "access", "requests", "args", "dirs", "settings", "description", "hook"}}
HOOKS = ("pre-commit", "commit-msg", "prepare-commit-msg", "pre-push", "post-checkout", "post-merge")
RECIPE_FIELDS = {"run", "description"}
RECIPE_NAME = re.compile(r"[A-Za-z][A-Za-z0-9_-]*")
RECIPE_WORD = re.compile(r"[A-Za-z0-9._/=:,@+%-]+")
DIR_FIELDS = {"from", "mode", "real", "copy", "label"}
AGENT_FIELDS = {"markers", "select", "resume", "hooks"}
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


def kit_version(kit, where):
    """A kit's version ("1.0" when missing), refused when this pot does not read it."""
    v = kit.get("version", "1.0")
    if v in KIT_VERSIONS:
        return v
    if isinstance(v, (int, float)) and not isinstance(v, bool):
        raise KitError(f"{where}: version is a string: \"{v}\"")
    m = re.fullmatch(r"1\.([0-9]+)", v) if isinstance(v, str) else None
    if m and int(m.group(1)) > 1:
        raise KitError(f"{where}: kit version {v} is newer than this pot reads ({', '.join(KIT_VERSIONS)}): take a newer pot")
    raise KitError(f"{where}: kit version {v!r}: this pot reads kit {', '.join(KIT_VERSIONS[:-1])} and {KIT_VERSIONS[-1]}")


def validate_kit(kit, where, home, root=True):
    """A kit manifest, checked: unknown fields are errors. Returns its requests (kit and commands).
    root: a root's kit, the only one that may declare `agent`."""
    if not isinstance(kit, dict):
        raise KitError(f"{where}: a kit is an attribute set")
    ver = kit_version(kit, where)
    for k in kit:
        if k not in KIT_FIELDS[ver]:
            raise KitError(f"{where}: {k} is not a kit field ({', '.join(sorted(KIT_FIELDS[ver] - {'file', 'kind'}))})")
    if "tools" in kit and "packages" in kit:
        raise KitError(f"{where}: tools and packages are one field (packages is its old name): give tools")
    _list(kit.get("tools", kit.get("packages")), f"{where}: tools")
    if "agent" in kit:
        if not root:
            raise KitError(f"{where}: agent: only a root's kit may declare it (an agent kit is the root's)")
        check_agent(kit["agent"], where)
    env = kit.get("env") or {}
    if not isinstance(env, dict) or not set(env) <= {"set"}:
        raise KitError(f"{where}: env takes only env.set (a requested variable is requests.env)")
    for n, v in (env.get("set") or {}).items():
        if not ENV_RE.fullmatch(n) or not isinstance(v, str):
            raise KitError(f"{where}: env.set.{n}: a variable name with a string value")
        if owned(n):
            raise KitError(f"{where}: env.set.{n}: that name is pot's own")
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
            if k not in COMMAND_FIELDS[ver]:
                raise KitError(f"{where}: commands.{name}.{k} is not a command field"
                               + (" (kit 1.1: dirs)" if k in COMMAND_FIELDS["1.0"] else ""))
        fd = c.get("fixedDirs") or {}
        if not isinstance(fd, dict) or not all(isinstance(v, (str, type(None))) for v in fd.values()):
            raise KitError(f"{where}: commands.{name}.fixedDirs: {{ \"~/run/path\" = \"~/host/dir\" or null; }}")
        for at in fd:
            if not norm_path(at, f"{where}: commands.{name}.fixedDirs").startswith("~/"):
                raise KitError(f"{where}: commands.{name}.fixedDirs: {at} must be in the run's HOME")
        check_dirs(c.get("dirs"), f"{where}: commands.{name}.dirs")
        if c.get("access", "write") not in ("read", "write"):
            raise KitError(f"{where}: commands.{name}.access: read or write")
        for r in parse_requests(c.get("requests"), f"{where}: commands.{name}", home):
            r.command = name
            reqs.append(r)
    for name, c in (kit.get("commands") or {}).items():
        if isinstance(c, dict) and "hook" in c and c["hook"] not in HOOKS:
            raise KitError(f"{where}: commands.{name}.hook: one of {', '.join(HOOKS)}")
    for n, body in (kit.get("recipes") or {}).items():
        # every version (F1, v2.18.0): graft writes kits.just from these, and just runs it on the host. A recipe's name
        # is a name; run is one of this kit's own commands, then literal words of a safe set (no shell syntax)
        if not RECIPE_NAME.fullmatch(str(n)):
            raise KitError(f"{where}: recipes.{n}: a recipe's name is letters, digits, _ and - (starting with a letter)")
        if isinstance(body, dict):
            if ver != "1.2" or not set(body) <= RECIPE_FIELDS or not isinstance(body.get("run"), str) \
                    or not isinstance(body.get("description", ""), str) or "\n" in body.get("description", ""):
                raise KitError(f"{where}: recipes.{n}: {{ run; description; }} (kit 1.2), or one line")
            body = body["run"]
        if not isinstance(body, str) or "\n" in body:
            raise KitError(f"{where}: recipes.{n}: {{ run; description; }} (kit 1.2), or one line")
        words = body.split(" ")
        if any(not RECIPE_WORD.fullmatch(w) for w in words):
            raise KitError(f"{where}: recipes.{n}: literal words only (letters, digits and . _ / = : , @ + % -, one space apart)")
        cmds = sorted(kit.get("commands") or {})
        if words[0] not in cmds:
            raise KitError(f"{where}: recipes.{n}: runs {words[0]}, which is not one of this kit's "
                           f"commands ({', '.join(cmds) or 'none'})")
    return reqs


def check_dirs(dirs, where):
    """kit 1.1's dirs: { "~/run/path" = { from; mode = links|mirror; real; copy; label; }; }."""
    if dirs is None:
        return
    if not isinstance(dirs, dict):
        raise KitError(f"{where}: {{ \"~/run/path\" = {{ from; mode; … }}; }}")
    for at, d in dirs.items():
        if not norm_path(at, where).startswith("~/"):
            raise KitError(f"{where}: {at} must be in the run's HOME")
        if not isinstance(d, dict) or not set(d) <= DIR_FIELDS:
            raise KitError(f"{where}.\"{at}\": {{ {'; '.join(sorted(DIR_FIELDS))}; }}")
        mode = d.get("mode", "links")
        if mode not in ("links", "mirror"):
            raise KitError(f"{where}.\"{at}\".mode: links or mirror")
        if d.get("from") is not None:
            norm_path(d["from"].replace("{workspace}", "w"), f"{where}.\"{at}\".from")
        if "{workspace}" in at + (d.get("from") or "") and mode != "mirror":
            raise KitError(f"{where}.\"{at}\": {{workspace}} is a mirror's")
        real = d.get("real", [])
        if mode == "links" and not (real is True or (isinstance(real, list) and all(isinstance(x, str) for x in real))):
            raise KitError(f"{where}.\"{at}\".real: a list of names, or true")
        if mode == "mirror":
            if "real" in d:
                raise KitError(f"{where}.\"{at}\".real: a links dir's")
            _list(d.get("copy"), f"{where}.\"{at}\".copy")
            if not re.fullmatch(r"[a-z][a-z0-9-]*", d.get("label", "mirror")):
                raise KitError(f"{where}.\"{at}\".label: one lowercase word")
        elif "copy" in d or "label" in d:
            raise KitError(f"{where}.\"{at}\": copy and label are a mirror's")


def check_agent(a, where):
    """kit 1.1's agent: { markers; select.env; resume = { transcripts; args; render; }; hooks; }."""
    if not isinstance(a, dict) or not set(a) <= AGENT_FIELDS:
        raise KitError(f"{where}: agent: {{ {'; '.join(sorted(AGENT_FIELDS))}; }}")
    for m in _list(a.get("markers"), f"{where}: agent.markers"):
        if not ENV_RE.fullmatch(m):
            raise KitError(f"{where}: agent.markers: {m!r} is not a variable name")
    sel = a.get("select") or {}
    if not isinstance(sel, dict) or not set(sel) <= {"env"} or not isinstance(sel.get("env") or {}, dict) \
            or not all(ENV_RE.fullmatch(k) and isinstance(v, str) for k, v in (sel.get("env") or {}).items()):
        raise KitError(f"{where}: agent.select: {{ env.NAME = \"value\"; }}")
    res = a.get("resume") or {}
    if not isinstance(res, dict) or not set(res) <= {"transcripts", "args", "render"}:
        raise KitError(f"{where}: agent.resume: {{ transcripts; args; render; }}")
    if "transcripts" in res and not norm_path(res["transcripts"].replace("**/", "").replace("*", "x"),
                                             f"{where}: agent.resume.transcripts").startswith("~/"):
        raise KitError(f"{where}: agent.resume.transcripts must be in the run's HOME")
    _list(res.get("args"), f"{where}: agent.resume.args")
    _list(a.get("hooks"), f"{where}: agent.hooks")


def normalise_kit(kit):
    """A checked kit in pot's one internal form: `tools` (not packages), and each command's `dirs` (a 1.0
    command's fixedDirs, skillDirs, realDirs and memory as dirs). Requests are untouched."""
    k = dict(kit)
    if "packages" in k:
        k["tools"] = k.pop("packages")
    cmds = {}
    for n, c in (k.get("commands") or {}).items():
        c = dict(c)
        fd, sd, rd, mem = (c.pop("fixedDirs", None) or {}), c.pop("skillDirs", None) or [], \
            set(c.pop("realDirs", None) or []), c.pop("memory", None)
        dirs = dict(c.get("dirs") or {})
        for at, hostdir in fd.items():
            dirs[at] = {"from": hostdir, "mode": "links", "real": True if at in rd else list(sd)}
        if mem:
            # v2.15.0's memory: a project memory folder per workspace, MEMORY.md a writable copy
            dirs[f"{mem['at']}/{{workspace}}/memory"] = {"from": f"{mem['host']}/{{workspace}}/memory", "mode": "mirror",
                                                         "copy": ["MEMORY.md"], "label": "memory"}
        if dirs:
            c["dirs"] = dirs
        cmds[n] = c
    if cmds:
        k["commands"] = cmds
    return k


def kit_tools(m):
    return m.get("tools", m.get("packages")) or []


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
                short = ident.split(":", 1)[-1].rstrip("/").rsplit("/", 1)[-1]
                raise Refusal(f"the grants file's entry {ident}: its digest does not match its requests (an edit by hand?): "
                              f"every run stops until it is fixed: pot grant --revoke {short}, then pot grant {short}")
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


def grant_status(ident, kind, canons, powerful, source, grants, machine, verified=True, accepts=()):
    """match | none (nothing requested) | never | grown | source | unverified, and the diff. ACCEPTS: stored
    sources that the host found equivalent to SOURCE (a repo kit's tree whose defining files and built kit are
    unchanged: the narrowed comparison, v2.17.0)."""
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
    if powerful and kind != "root" and source is not None and entry.get("source") != source \
            and entry.get("source") not in list(accepts):
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
    """Identities whose entries differ between two parsed grants files (for "changed since this machine last read it")."""
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
        raise Refusal(f"pot: no context {name!r} in this root (contexts: {', '.join(sorted(ctxs))})", 2)
    seen, chain, c = set(), [], ctxs[name]
    out = {}
    cur = name
    while True:
        if cur in seen:
            raise Refusal(f"pot: context {name}: extends forms a loop", 2)
        seen.add(cur)
        chain.append(ctxs[cur])
        nxt = ctxs[cur].get("extends")
        if not nxt:
            break
        if nxt not in ctxs:
            raise Refusal(f"pot: context {cur} extends {nxt!r}, which this root does not define", 2)
        cur = nxt
    for c in reversed(chain):
        out.update({k: v for k, v in c.items() if k != "extends"})
    for k in out:
        if k not in CONTEXT_FIELDS:
            raise Refusal(f"pot: context {name}: {k} is not a context field", 2)
    res = {**CONTEXT_DEFAULTS, **out, "name": name}
    if res["mode"] not in MODES or res["access"] not in ACCESS or res["network"] not in ("granted", "none"):
        raise Refusal(f"pot: context {name}: mode, access or network has an unknown value", 2)
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
                return hits, name   # instances of one kit: the caller chooses
            raise Refusal(f"pot: {' and '.join(kit_display(k) for k in hits)} both ship {name}: pot KIT:{name}", 2)
        if hits:
            return hits[0], name
    if kitname is not None:
        raise Refusal(f"pot: no kit {kitname!r} ships a command {name!r}", 2)
    return None, name


def instance_form(cmd, instance, form):
    """The instance as the person typed it: CMD@NAME, --instance NAME or --account NAME."""
    return f"{cmd}@{instance}" if form == "@" else f"{form or '--instance'} {instance}"


def choose_instance(hits, instance, defaults, env_match=None, form=None):
    """§4.4, the instance of a command whose kit the root applies under several names: @NAME or
    --instance NAME; else the caller's variables (env_match: [(name, number of variables)] of the
    instances whose agent.select.env all match); else the kit's default."""
    by = {k["instance"]: k for k in hits}
    base, cmd = hits[0].get("base"), hits[0].get("cmd")
    names = ", ".join(f"{base}@{n}" for n in sorted(by))
    if instance:
        if instance not in by:
            raise Refusal(f"pot: {instance_form(cmd, instance, form)}: this root declares {names}", 2)
        return by[instance]
    default = (defaults or {}).get(base)
    match = [(n, c) for n, c in env_match or [] if n in by]
    if match:
        top = max(c for _, c in match)
        best = sorted(n for n, c in match if c == top)
        if len(best) == 1:
            return by[best[0]]
        if default in best:
            return by[default]
        raise Refusal(f"pot: {cmd}: the caller's variables match {' and '.join(f'{base}@{n}' for n in best)} equally: "
                      f"choose one with {cmd}@NAME or --instance NAME", 2)
    if default in by:
        return by[default]
    raise Refusal(f"pot: {base} has the instances {', '.join(sorted(by))}: choose one with {cmd}@NAME or --instance NAME", 2)


def select_matches(root, cmd, env, same_path):
    """The instances of CMD's root kit whose agent.select.env every variable equals the caller's:
    [(instance, number of variables)]. A value starting ~ or / is a path (same_path compares)."""
    out = []
    for n, k in sorted((root.get("kits") or {}).items()):
        base, _, inst = n.partition("@")
        sel = ((k.get("agent") or {}).get("select") or {}).get("env") or {}
        if not inst or cmd not in (k.get("commands") or {}) or not sel:
            continue
        ok = True
        for var, want in sel.items():
            have = env.get(var)
            if have is None or not (same_path(want, have) if want.startswith(("~", "/")) else have == want):
                ok = False
                break
        if ok:
            out.append((inst, len(sel)))
    return out


def agent_markers(root):
    """The variables that mean an agent is the caller: every root kit's agent.markers, the root's
    agentMarkers, and POT (a run)."""
    ms = []
    for k in (root.get("kits") or {}).values():
        ms += (k.get("agent") or {}).get("markers") or []
    return list(dict.fromkeys(list(root.get("baseAgentMarkers") or []) + ms + list(root.get("agentMarkers") or [])
                              + ["POT"]))


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
              "unsetEnv": [], "oneRun": [], "noBinding": [], "bindingFiles": []}
    machine = inp["machine"]
    grants = inp["grants"]

    clone_mode = (ctx["mode"] == "clone" and flags.get("mode") != "in-place") or flags.get("mode") == "clone"
    if ctx["mode"] == "none":
        raise Refusal("pot: host runs no sandbox and composes no policy")
    trust = ctx.get("trust", "workspace")
    if trust == "root":
        raise Refusal(f"pot ({ctx['name']}): trust = \"root\" comes in a later release")

    # ---- the kits of the run (§4.1)
    kits = []

    def add_kit(ident, kind, manifest, file, name=None, **extra):
        reqs = validate_kit(manifest, f"{file} ({ident})", home, root=kind == "root")
        manifest = normalise_kit(manifest)
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
            manifests.append((rk["manifest"], rk.get("file") or f"{w.get('flakeDir') or '.'} (its pot-kits)"))
        for m, f in w.get("rootRepoKits") or []:
            manifests.append((m, f))
        for m, f in manifests:
            validate_kit(m, f"{f} ({w['id']})", home, root=False)
        merged = merge_manifests([(normalise_kit(m), f) for m, f in manifests])
        w["kit"] = add_kit(w["id"], "repo", merged, " + ".join(f for _, f in manifests) or "(none)",
                           name=repo_short_name(w), source={"tree": w["source"]} if w.get("source") else None,
                           sourceAccepts=w.get("sourceAccepts") or [],
                           workspace=w)
        ws_kits.append(w["kit"])
        for s in w.get("scions") or []:
            if s.get("manifest") is None:
                continue
            k = add_kit(s["identity"], "scion", s["manifest"], s.get("file") or s["path"], s["name"],
                        verified=s.get("verified", False),
                        source={"commit": s["commit"], "tag": s["tag"]} if s.get("tag") else None, scion=s)
            ws_kits.append(k)

    # workspace kits may not ship names of root kits' commands, standard tools or pot (§4.2)
    root_cmds = {c for k in root_kits for c in (k["manifest"].get("commands") or {})}
    standard = set(root.get("standardNames") or []) | {"pot"}
    for k in ws_kits:
        for c in k["manifest"].get("commands") or {}:
            if c in root_cmds or c in standard:
                raise Refusal(f"pot ({ctx['name']}): {kit_display(k)} ships a command named {c}, "
                              f"which is {'pot' if c == 'pot' else 'a root kit command' if c in root_cmds else 'a standard tool'}: a workspace kit may not")

    # ---- the command kit
    argv = list(inp.get("argv") or [])
    cmdkit, cmdname = None, argv[0] if argv else None
    if argv and (flags.get("commandKit") or flags.get("commandSlot")):
        # a children-table row, or a wrapper's start (v2.18.0): the kit is the host's, by identity or by its slot in
        # this workspace's bundle (a workspace kit), never by a name the caller gives
        cmdkit = next((k for k in kits if argv[0] in (k["manifest"].get("commands") or {})
                       and (k["id"] == flags.get("commandKit")
                            or (flags.get("commandSlot") and k["kind"] != "root" and slot_of(k) == flags["commandSlot"]))), None)
        if cmdkit is None and flags.get("commandKit"):
            raise Refusal(f"pot: no kit {flags['commandKit']} ships {argv[0]}", 2)
        if cmdkit is None:
            raise Refusal(f"pot: {argv[0]}: this workspace's kits have no command {argv[0]} (the wrapper comes from "
                          "another workspace's tools): run it in its repository")
    elif argv and "/" not in argv[0]:
        hit, cmdname = find_command(root_kits, ws_kits, argv[0])
        if isinstance(hit, list):
            for k in hit:
                k["cmd"] = cmdname
            hit = choose_instance(hit, flags.get("instance"), root.get("defaultInstance"), flags.get("envMatch"),
                                  flags.get("instanceForm"))
        elif hit is not None and flags.get("instance") and hit.get("instance") and hit["instance"] != flags["instance"]:
            raise Refusal(f"pot: {instance_form(cmdname, flags['instance'], flags.get('instanceForm'))}: "
                          f"this root declares {kit_display(hit)}", 2)
        cmdkit = hit
    typed = instance_form(cmdname, flags.get("instance"), flags.get("instanceForm")) if flags.get("instance") else ""
    if cmdkit is not None and flags.get("instance") and cmdkit.get("instance") is None:
        raise Refusal(f"pot: {typed} names an instance, and {kit_display(cmdkit)} has none", 2)
    if cmdkit is None and flags.get("instance"):
        raise Refusal(f"pot: {typed} chooses an instance of an agent kit; this command has no kit", 2)
    report["command"] = {"kit": cmdkit["id"] if cmdkit else None, "name": cmdname,
                         "instance": (cmdkit or {}).get("instance"),
                         "defaultInstance": (root.get("defaultInstance") or {}).get((cmdkit or {}).get("base"))}
    report["workspaces"] = [{"consumer": w.get("repoKit") is not None, "key": w.get("originKey") or w["id"][len("repo:"):],
                             "file": (w.get("repoKit") or {}).get("file"),
                             "flakeDir": w.get("flakeDir")} for w in inp["workspace"]]
    if cmdkit and cmdkit.get("instance") and cmdkit["instance"] != (root.get("defaultInstance") or {}).get(cmdkit.get("base")):
        report["notes"].append(f"instance {cmdkit['instance']}")
    # an unverified scion kit's command runs with its see items, without its reach (architecture §4: one start
    # note, no 125; v2.16 refused it)

    included = []
    for n in ctx.get("include") or []:
        k = next((k for k in root_kits if k["name"] == n), None)
        if k is None:
            raise Refusal(f"pot: context {ctx['name']} includes {n!r}, which is not a root kit", 2)
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
        if k["kind"] == "scion" and not k.get("verified", True) and k["requests"]:
            s_ = k.get("scion") or {}
            report["notes"].append(f"{kit_display(k)}: its URL did not answer, so its kit is unverified and its grant does not apply"
                                   if s_.get("unreachable") else
                                   f"{kit_display(k)}: gitlink is not a release tag at its URL; its kit is unverified and its grant does not apply")
        canons = [r.canonical for r in k["requests"]]
        powerful = any(r.cls == "powerful" for r in k["requests"])
        st = grant_status(k["id"], k["kind"], canons, powerful, k.get("source"), grants, machine,
                          accepts=k.get("sourceAccepts") or (),
                          verified=k.get("verified", True) if k["kind"] == "scion" else True)
        st["kit"] = k["id"]
        st["name"] = kit_display(k)
        st["kind"] = k["kind"]
        st["applies"] = k["id"] in applying_ids
        st["digest"] = digest(canons) if canons else None
        st["requests"] = sorted(set(canons))
        st["source"] = k.get("source")
        st["file"] = k["file"]
        st["powerful"] = sorted({r.canonical for r in k["requests"] if r.cls == "powerful"})
        status[k["id"]] = st
        report["kits"].append(st)
        if st["status"] in ("grown", "source", "never") and not st["applies"]:
            if ctx.get("view") and k["kind"] == "repo":
                report["notes"].append(f"{st['name']} {STATUS_WORDS[st['status']]} (its reach does not apply in inspect)")
            elif st["status"] != "never":
                report["notes"].append(f"{st['name']} {STATUS_WORDS[st['status']]}: pot grant")
    for k, _ in applies:
        st = status[k["id"]]
        if st["status"] in ("grown", "source", "never"):
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
        for p in kit_tools(k["manifest"]):
            item("package", p, "see", [ksrc(k, "see")])
        for n, v in ((k["manifest"].get("env") or {}).get("set") or {}).items():
            if n in env_set and env_set[n][0] != v:
                raise Refusal(f"pot ({ctx['name']}): env.set {n} has two values, from {env_set[n][1]} and {k['id']}")
            env_set[n] = (v, k["id"])
            item("env.set", f"{n}={v}", "see", [ksrc(k, "see")])
        for path, f in (k["manifest"].get("files") or {}).items():
            pth = norm_path(path, k["file"])
            if pth in files and files[pth][0] != f["source"]:
                raise Refusal(f"pot ({ctx['name']}): two different files at {pth}, from {files[pth][1]} and {k['id']}")
            if pth.startswith("/") and "root-files" not in (inp.get("probe") or []):
                raise Refusal(f"pot ({ctx['name']}): {k['id']} places a file at {pth}, which needs the root-files guarantee")
            files[pth] = (f["source"], k["id"])
            item("file", pth, "see", [ksrc(k, "see")], source=f["source"], mode=f.get("mode", "fixed"),
                 **({"keys": f["keys"]} if f.get("keys") else {}))
    # repo kits' packages reach every run; their env.set and files only their own commands (§4.2)
    for k in ws_kits:
        if k["kind"] == "repo" and k is not cmdkit:
            for p in kit_tools(k["manifest"]):
                item("package", p, "see", [ksrc(k, "see")])
    for w in inp["workspace"]:
        if w.get("tools"):
            item("package", w["tools"], "see", [ksrc(w["kit"], "tools")], tools=True)
    attrs = flags.get("packageAttrs") or {}
    for p in flags.get("package") or []:
        item("package", p, "see", [{"flag": "--package"}], **({"attr": attrs[p]} if p in attrs else {}))
        if attrs.get(p) not in [v for f, v in report["oneRun"] if f == "--package"]:
            report["oneRun"].append(("--package", attrs.get(p, p)))
    if inp["host"].get("tz"):
        item("env.set", f"TZ={inp['host']['tz']}", "see", [{"root": "base"}])
    if root.get("caBundle"):
        item("file", "/etc/ssl/certs/ca-certificates.crt", "see", [{"root": "caBundle"}], source=root["caBundle"],
             mode="fixed")

    # limits
    access = ctx["access"]
    if flags.get("access"):
        if ACCESS_RANK[flags["access"]] > ACCESS_RANK[access]:
            raise Refusal(f"pot ({ctx['name']}): --access {flags['access']} would widen this context's access ({access})")
        access = flags["access"]
        report["oneRun"].append(("--access", flags["access"]))
    if cmdkit:
        ca = (cmdkit["manifest"].get("commands") or {}).get(cmdname, {}).get("access")
        if ca and ACCESS_RANK[ca] < ACCESS_RANK[access]:
            access = ca
    item("access", access, "limits", [{"context": ctx["name"]}])
    hidden = []
    for h in DEFAULT_HIDDEN + list(root.get("baseHidden") or []):
        hidden.append((h, {"root": "default"}))
    for h in root.get("hidden") or []:
        hidden.append((norm_path(h, "the root's hidden"), {"root": "hidden"}))
    for h in (root.get("grantsFile"), root.get("stateDir")):
        if h:
            hidden.append((canon_path(expand(h, home), home), {"root": "pot"}))
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
    for p in BASE_PROTECT + list(root.get("baseProtect") or []):
        protect_src.append((p, {"root": "default"}))
    pkits = [k for k in kits if k["kind"] in ("repo", "scion")] + ([cmdkit] if cmdkit and cmdkit["kind"] == "root" else []) \
        + [k for k in included if k is not cmdkit]
    for k in pkits:
        for p in k["manifest"].get("protect") or []:
            protect_src.append((p, ksrc(k, "limit")))
    for w in inp["workspace"]:
        if w.get("flakeDir"):
            protect_src.append((w["flakeDir"], {"root": "default", "why": "the flake directory pot evaluates"}))
        for p in w.get("bundleProtect") or []:
            protect_src.append((p, {"root": "bundle", "why": "the repository's pot-kits"}))
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
                why = STATUS_WORDS[st["status"]]
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
                # v2.17.1: a credential or MCP key this run's kits read from a file, granted or not yet (pot checks the
                # file before the grant prompts and the clone's use)
                f = ((b[1].get("from") if b and isinstance(b[1], dict) else None) or {})
                if (applies_here and r.value in ("credential", "mcp") and isinstance(f, dict) and f.get("file")
                        and not (view and r.value == "mcp")):
                    report["bindingFiles"].append({"capability": r.value, "selector": b[0], "kit": k["id"], "file": f["file"],
                                                   **({"refresh": b[1]["refresh"]} if b[1].get("refresh") else {})})
                if eff and b is None:
                    eff, why = False, "no binding"
                    report["noBinding"].append((k["id"], r))
                elif eff and CAPABILITIES[r.value]["phase"] != "1a":
                    eff, why = False, "comes in a later release"
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
                        raise Refusal(f"pot ({ctx['name']}): {k['id']} requests {r.kind} {r.value}, {bad}")
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
                raise Refusal(f"pot ({ctx['name']}): --{fl} {p}: {bad}")
            eff = not (view and kind == "write")
            item(kind, n, "reach", [{"flag": f"--{fl}"}], eff, path=real, **({} if eff else {"why": "inspect changes nothing"}))
            report["oneRun"].append((f"--{fl}", n))
    for h in flags.get("net") or []:
        v = parse_network(h, "--net")
        item("network", v, "reach", [{"flag": "--net"}], network_ok,
             **({} if network_ok else {"why": f"context {ctx['name']} has no network"}))
        report["oneRun"].append(("--net", v))
    for e in flags.get("env") or []:
        if not ENV_RE.fullmatch(e) or owned(e):
            raise Refusal(f"pot: --env {e}: a variable name that is not pot's own", 2)
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
                raise Refusal(f"pot ({ctx['name']}): {cmdname}'s program {hp} resolves to {canon_path(real, home)}, "
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
    if clone_mode and "same-path-clone" not in requires and "same-path-clone" in proved:
        # a clone is mounted at the original path (§9.3)
        requires.append("same-path-clone")
    elif clone_mode and "same-path-clone" not in proved and "same-path-clone" not in requires:
        # §11: Seatbelt has no mounts, so its clone runs at its own path, and the startup line says so (a context
        # that requires same-path-clone still stops, 125)
        report["notes"].append("clone at its own path")
    missing = [g for g in requires if g not in proved]
    if missing:
        hint = " (resource-limits: a user systemd scope, for runs without a terminal: pot doctor)" if "resource-limits" in missing else ""
        raise Refusal(f"pot ({ctx['name']}): requires {' '.join(missing)}; {inp.get('backendName', 'the backend')} proves "
                      f"{' '.join(proved) or 'nothing'}{hint}", EXIT_GRANT)

    workspace = [{"id": w["id"], "path": w["top"],
                  "common": os.path.join(w["top"], ".git") if w.get("clone") else w["common"],
                  "protected": protect if access == "write" else [],
                  "clone": w.get("clone")} for w in inp["workspace"]]
    policy = {
        "version": POLICY_VERSION,
        "run": inp["run"],
        "context": ctx["name"],
        "mode": "clone" if clone_mode else "in-place",
        "access": access,
        "backend": {"name": inp.get("backendName", "bubblewrap"), "requires": requires, "proved": proved},
        "workspace": workspace,
        "items": items,
        # launcher: the in-run pointer the host binds (v2.18.0); a wrapper that finds it named here, and not in the
        # run's home, refuses rather than run its kit code under this run's policy
        "mounts": {"state": state_mounts, "run": "/run/pot", "launcher": "/run/pot/launcher"},
        "bindings": pbind,
        "children": {},
        "grants": [{"id": st["kit"], "digest": st["digest"], "match": st["status"] in ("match", "none")}
                   for st in report["kits"]],
    }
    report["kitObjs"] = [{"id": k["id"], "kind": k["kind"], "name": k.get("name"), "slot": slot_of(k), "file": k.get("file"),
                          "display": kit_display(k),
                          "commands": sorted((k["manifest"].get("commands") or {})),
                          "hooks": {n: c["hook"] for n, c in (k["manifest"].get("commands") or {}).items() if c.get("hook")},
                          "recipes": {n: (r if isinstance(r, str) else r.get("run", "")).split(" ")[0]
                                      for n, r in (k["manifest"].get("recipes") or {}).items()}}
                         for k in kits]
    return policy, report


# ---------------------------------------------------------------- the children table (§10, v2.18.0)

def slot_of(k):
    """Where a kit sits in its bundle, as a wrapper's manifest names it: self, scions.<name>; root for a root kit."""
    return {"root": "root", "repo": "self"}.get(k["kind"], f"scions.{k.get('name')}")


def eff_reach(p):
    return [i for i in p["items"] if i["class"] == "reach" and i["effective"]]


def binding_keys(p):
    return {(b["capability"], b["selector"], b["request"]) for b in p.get("bindings") or []}


def boundary(p):
    """What decides a run's boundary: equal boundaries exec, a narrower one nests."""
    return (p["mode"], p["access"],
            frozenset((i["kind"], i["value"]) for i in eff_reach(p) if i["kind"] != "state"),
            frozenset(i["value"] for i in p["items"] if i["kind"] == "protect" and i["effective"]),
            frozenset(binding_keys(p)),
            frozenset((s["name"], s["at"], json.dumps(s["key"])) for s in p["mounts"]["state"]))


def lack_word(i):
    """A missing item as the refusal names it: a variable, a capability, a host (port 443 left out), a path."""
    if i["kind"] == "capability":
        return i["value"]
    if i["kind"] == "network":
        return i["value"][:-4] if i["value"].endswith(":443") else i["value"]
    if i["kind"] == "unsandboxed":
        return "unsandboxed"
    return i["value"]


def narrower(child, parent):
    """§10: the child's effective reach items that the parent's do not cover (empty: the child is narrower or
    equal). State is never a lack: the host binds a child's state into its parent (a parent holds everything its
    children get, residual 7)."""
    pr = eff_reach(parent)
    pnet = [i["value"] for i in pr if i["kind"] == "network"]
    pbind = binding_keys(parent)
    cbind = {b["request"]: b for b in child.get("bindings") or []}
    tops = [w["path"] for w in parent["workspace"]]

    def paths(kinds):
        return [i.get("path") or i["value"] for i in pr if i["kind"] in kinds] + \
            [i["value"] for i in pr if i["kind"] in kinds and i.get("path")]
    lacks = []
    if ACCESS_RANK.get(child.get("access"), 0) > ACCESS_RANK.get(parent.get("access"), 0):
        lacks.append(f"{child['access']} access")     # (conformance 1) a write child never nests under a read run
    for i in eff_reach(child):
        k, v = i["kind"], i["value"]
        if k == "network":
            ok = v in pnet or (not any(c in v for c in "*?[") and any(fnmatch.fnmatchcase(v.lower(), p.lower()) for p in pnet))
        elif k == "read":
            p = i.get("path") or v
            ok = p.startswith("/nix/store/") or any(under(p, t) for t in tops) \
                or any(under(p, q) or under(v, q) for q in paths(("read", "write", "socket")))
        elif k in ("write", "socket"):
            p = i.get("path") or v
            ok = (parent["access"] == "write" and any(under(p, t) for t in tops)) \
                or any(under(p, q) or under(v, q) for q in paths(("write", "socket") if k == "write" else ("socket",)))
        elif k == "env":
            ok = any(j["kind"] == "env" and j["value"] == v for j in pr)
        elif k == "state":
            ok = True
        elif k == "capability":
            b = cbind.get(i.get("request"))
            ok = b is not None and (b["capability"], b["selector"], b["request"]) in pbind
        elif k == "unsandboxed":
            ok = False
        else:
            ok = any(j["kind"] == k and j["value"] == v for j in pr)
        if not ok:
            lacks.append(lack_word(i))
    return list(dict.fromkeys(lacks))


def child_rows(report):
    """The commands a run may start (§10): (context, kit, command, dispatch) for each workspace kit's command, in
    place, and for the root kits that the context's commands list or that ship this run's command, in place and in
    dispatch (a clone of its own)."""
    ctx = report["context"]
    named = set(ctx.get("commands") or [])
    mine = (report.get("command") or {}).get("kit")
    rows = []
    for k in report.get("kitObjs") or []:
        if k["kind"] in ("repo", "scion"):
            rows += [(ctx["name"], k, c, False) for c in k["commands"]]
        elif k["kind"] == "root" and (k["id"] == mine or k["name"] in named or k["name"].partition("@")[0] in named):
            for c in k["commands"]:
                rows.append((ctx["name"], k, c, False))
                rows.append(("dispatch", k, c, True))
    return rows


def policy_digest(p):
    q = {k: v for k, v in p.items() if k not in ("run", "children")}
    return "sha256-" + hashlib.sha256(json.dumps(q, sort_keys=True).encode()).hexdigest()


def children_table(inp, policy, report):
    """policy.json's children: each row composed by the host with the child kit's own grant (compose(), as
    `pot CONTEXT KIT:CMD` would be here), then exec (an equal boundary), nest (narrower) or refused (it needs reach
    this run lacks, its grant does not match, or this run can't nest). Returns (rows, {digest: child policy})."""
    ctx = report["context"]
    can_nest = ctx.get("nest", True) is not False and ctx.get("trust") != "untrusted"
    proves = "nest" in (inp.get("probe") or [])
    parent_cmd = (report.get("command") or {}).get("name") or "the shell"
    recipes = {}
    for k in report.get("kitObjs") or []:
        for n, first in sorted(k["recipes"].items()):
            recipes.setdefault((k["id"], first), n)
    rows, policies, built = {}, {}, {}
    for cname, k, c, dispatch in child_rows(report):
        # a row in the run's own context runs in place, in the run's workspace (a clone parent's clone); a dispatch
        # row makes a clone of its own
        key = f"dispatch {k['id']}:{c}" if dispatch else f"{k['id']}:{c}"
        row = {"slot": k["slot"], "command": c, "kit": k["id"], "kitName": k["display"], "context": cname,
               "mode": "clone" if dispatch else "in-place", "file": k.get("file")}
        if k["hooks"].get(c):
            row["hook"] = k["hooks"][c]
        here = f"pot dispatch {c}" if dispatch else f"just {recipes[(k['id'], c)]}" if (k["id"], c) in recipes else f"{c} ARGS"
        sub = dict(inp, context=cname, argv=[c], flags={"commandKit": k["id"], "mode": "clone" if dispatch else "in-place"},
                   run="", workspace=[dict(w) for w in inp["workspace"]])
        try:
            cp, cr = compose(sub)
            if not dispatch:
                for w in cp["workspace"]:
                    if w.get("clone"):
                        # in place in a clone parent: its clone, at the original path, as the parent sees it
                        w["clone"], w["common"] = None, os.path.join(w["path"], ".git")
        except Refusal as e:
            row["refused"] = e.message.split(": ", 1)[-1] if e.message.startswith("pot") else e.message
            rows[key] = row
            continue
        stale = [s for s in cr["stale"]]
        if stale:
            row["refused"] = "not granted: pot grant " + " ".join(dict.fromkeys(s["name"] for s in stale)) + " (on the host)"
            row["grant"] = True
            rows[key] = row
            continue
        lacks = narrower(cp, policy)
        if lacks:
            row["refused"] = f"needs {', '.join(lacks)}"
            row["text"] = (f"pot ({ctx['name']}): {c} needs {', '.join(lacks)}, which this run ({parent_cmd}) lacks: "
                           f"run it on the host ({here})")
            rows[key] = row
            continue
        # a dispatch row always nests: its clone is a boundary of its own
        how = "exec" if boundary(cp) == boundary(policy) and not dispatch else "nest"
        if how == "nest" and not can_nest:
            row["refused"] = "this run's context is untrusted (nest = false)" if ctx.get("trust") == "untrusted" \
                else f"context {ctx['name']} has nest = false"
            rows[key] = row
            continue
        if how == "nest" and not proves:
            if inp.get("backendName") == "seatbelt":
                # design §10: Seatbelt cannot nest, so a narrower child is refused, never flattened into its parent
                row["refused"] = "Seatbelt cannot nest"
                row["text"] = (f"pot ({ctx['name']}): {'dispatch ' if dispatch else ''}{c} runs under its own, narrower "
                               f"policy, and Seatbelt cannot nest (a run on macOS starts no runs inside it): run it on the "
                               f"host ({here})")
            else:
                row["refused"] = f"{inp.get('backendName', 'this backend')} can't nest here"
            rows[key] = row
            continue
        d = policy_digest(cp)
        row.update(how=how, policy=d, argv=cr["argv"],
                   env={i["value"].partition("=")[0]: i["value"].partition("=")[2] for i in cp["items"]
                        if i["kind"] == "env.set" and i["effective"]})
        rows[key] = row
        policies.setdefault(d, cp)
        built[key] = cp
    # each nest row's own table: the rows narrower than it (one more level; a grandchild has none)
    for key, row in rows.items():
        if row.get("how") != "nest":
            continue
        mine = built[key]
        subt = {}
        for k2, r2 in rows.items():
            if k2 == key or "policy" not in r2 or r2["context"] != mine["context"] or r2.get("mode") == "clone":
                continue
            if narrower(built[k2], mine):
                continue
            how = "exec" if boundary(built[k2]) == boundary(mine) else "nest"
            subt[k2] = {**{x: y for x, y in r2.items() if x != "children"}, "how": how}
        row["children"] = subt
    return rows, policies


def children_lines(table, width=100, seatbelt=False):
    """explain's children rows: the commands this run may start, by how (§10)."""
    if not table:
        return ["children   none (Seatbelt cannot nest: a run on macOS starts no runs inside it)" if seatbelt
                else "children   none: this run keeps --disable-userns"]
    groups, refused = {}, []
    for key, r in sorted(table.items()):
        name = r["command"] if r.get("context") in (None,) or " " not in key else f"{r['context']} {r['command']}"
        if r.get("refused"):
            refused.append(f"  {name}: refused ({r['refused']})")
            continue
        kit = r.get("kitName") or r["kit"]
        groups.setdefault((kit, r["how"]), []).append(name)
    parts = [f"{' '.join(v)} ({k}): {how}" for (k, how), v in groups.items()]
    out = wrap_words("children   ", [p + " ·" for p in parts[:-1]] + parts[-1:], width, 11) if parts else ["children"]
    return out + refused


# the words people see for a grant's status (never the internal keys)
STATUS_WORDS = {"match": "granted", "none": "requests nothing", "never": "not granted", "grown": "asks for more",
                "source": "changed source", "unverified": "unverified"}

BINDING_SOCKETS = {"status.relay": "relay.sock", "git.fetch": "git.sock", "nix": "nix.sock"}


def cap_key(r):
    spec = CAPABILITIES[r.value]
    return r.data.get(spec["key"]) if spec["key"] else None


def find_binding(bindings, r, kit_id):
    """bindings.<capability>."<selector>" (§7.2): the selector is the capability's key (a host, a URL
    pattern, a server name) or "*", optionally narrowed to one kit: "HOST kit=KIT@NAME".
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
        e["socket"] = f"/run/pot/{BINDING_SOCKETS[r.value]}"
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
            raise Refusal(f"pot ({ctx['name']}): bindings.{r.value}.\"{sel}\".via = {via!r}: v2.17.1 carries "
                          "env and file; proxy comes in a later release")
        e.update(via=via, env=env, **{"from": cfg.get("from") or {}}, holds="the run holds it",
                 **({"refresh": cfg["refresh"]} if cfg.get("refresh") else {}))
        if r.value == "mcp":
            e.update(server=r.data.get("server"), url=cfg.get("url"), header=cfg.get("header", "Authorization"),
                     scheme=cfg.get("scheme", "Bearer"), config="/run/pot/mcp.json")
            if not cfg.get("url"):
                raise Refusal(f"pot ({ctx['name']}): bindings.mcp.\"{sel}\" needs a url")
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
            if k in ("file", "version"):
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
                                raise KitError(f"{f}: {k}.{kk}.{kkk} has two values")
                            cur[kk][kkk] = vvv
                    else:
                        if kk in cur and cur[kk] != vv:
                            raise KitError(f"{f}: {k}.{kk} has two values")
                        cur[kk] = vv
            else:
                out[k] = v
    return out


def path_refusal(canon, real, hidden_paths, inp):
    """M3/M4: why a reach path is refused, or None."""
    sd = inp.get("stateDir")
    if sd and under(real, sd):
        return "which is pot's own state"
    if sd and under(sd, real):
        # S7: a folder that holds pot's state, named as such
        return f"which holds pot's own state ({canon_path(sd, inp['host']['home'])})"
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
            out.append((m, f"programs.pot.repoKits.\"{pat}\""))
    return out


# ---------------------------------------------------------------- people-facing text

def describe_request(c):
    return c


def stale_message(ctxname, st):
    lines = []
    if st["status"] == "unverified":
        lines.append(f"pot ({ctxname}): {st['name']} ({st['kit']}) is not a verified release, so its grant does not apply")
        return "\n".join(lines)
    if st["status"] == "never":
        lines.append(f"pot ({ctxname}): {st['name']} ({st['kit']}) requests reach that is not granted:")
    elif st["status"] == "grown":
        lines.append(f"pot ({ctxname}): {st['name']} ({st['kit']}) asks for more than its grant:")
    else:
        lines.append(f"pot ({ctxname}): {st['name']} ({st['kit']}) changed source since its grant:")
        lines.append(f"  source: {fmt_source(st.get('was'))} -> {fmt_source(st.get('now'))}")
        for d in st.get("diffstat") or []:
            lines.append(f"    {d}")
    for a in st["added"]:
        lines.append(f"  + {a}")
    lines.append(f"  grant it: pot grant {st['name']}")
    return "\n".join(lines)


def fmt_source(s):
    if not s:
        return "(none)"
    if "tag" in s:
        return f"{s['tag']} ({s['commit'][:12]})"
    if "tree" in s:
        return f"tree {s['tree'][:12]}"
    return json.dumps(s, sort_keys=True)


def root_paths(root):
    """How a fix to the root is written, and what applies it: home-manager's programs.pot or mkRoot's."""
    if root.get("name") == "home-manager":
        return "programs.pot.", "home-manager switch"
    return "", "rebuild the root"


def kit_home(file, flake_dir):
    """Where a kit is written, as people find it: the bundle's file under the flake directory pot
    evaluated, its note after a comma ("flake.nix (perSystem)" in nix/agent-tools →
    "nix/agent-tools/flake.nix, perSystem")."""
    m = re.fullmatch(r"(.*?) \((.*)\)", file or "")
    path, note = (m.group(1), m.group(2)) if m else (file, None)
    if flake_dir and path and not path.startswith(flake_dir + "/"):
        path = f"{flake_dir}/{path}"
    return f"{path}, {note}" if note else path


def repo_fix(report, root, kind):
    """The repo kit's KIND: in the file its bundle names, under the flake directory, else the root's repoKits."""
    pre, apply = root_paths(root)
    ws = (report.get("workspaces") or [{}])[0]
    if ws.get("consumer"):
        if not ws.get("file"):
            return f"{kind} in a repo kit for {ws.get('flakeDir') or 'this repository'} (the flake pot evaluated)"
        return f"{kind} in the repo kit ({kit_home(ws['file'], ws.get('flakeDir'))})"
    return f"{pre}repoKits.\"{ws.get('key') or '<origin>'}\".{kind} ({apply})"


def permanent_fix(flag, value, report, root):
    """§12.2: the permanent fixes for a one-run flag, one per line ("\n" between them; fix_block lays
    them out): the command's kit when it is a workspace kit; otherwise the repo kit, with the context's
    included kit as the every-repository alternative."""
    kind = {"--net": "requests.network", "--read": "requests.read", "--write": "requests.write",
            "--env": "requests.env", "--package": "tools", "--access": "access"}[flag]
    pre, apply = root_paths(root)
    cmd = report.get("command") or {}
    inc = [n for n in (report["context"].get("include") or [])]
    if flag == "--access":
        return f"a context with access = \"{value}\" ({pre}contexts; {apply})"
    if flag == "--package":
        return f"{pre}tools ({apply})\nor {repo_fix(report, root, 'tools')}"
    if cmd.get("kit") and cmd["kit"].startswith("git:"):
        f = next((k.get("file") for k in report.get("kits") or [] if k["kit"] == cmd["kit"]), None)
        ws = (report.get("workspaces") or [{}])[0]
        home = kit_home(f, ws.get("flakeDir")) if f else None
        return f"{kind} in {kit_short(cmd['kit'], report)}'s kit ({home + ': ' if home else ''}a kit release)"
    alt = f"\nor {pre}kits.{inc[0]}.{kind}, for every repository ({apply})" if inc else ""
    return repo_fix(report, root, kind) + alt


def one_run(ctxname, rest):
    """D1: a hint's one-run form, in the run's own context: pot --net HOST … for work, pot dispatch --net HOST …"""
    return "pot " + ("" if ctxname in (None, "work") else f"{ctxname} ") + rest


def fix_block(once, fix, indent="  "):
    """The two lines of a fix, under the line that says what is missing:
         one run:   pot --net HOST …
         permanent: FIX
                    or OTHER FIX"""
    lines = [f"{indent}one run:   {once}"] if once else []
    alts = [a for a in (fix or "").split("\n") if a]
    if alts:
        lines.append(f"{indent}permanent: {alts[0]}")
        lines += [f"{indent}           {a}" for a in alts[1:]]
    return lines


def one_line(fix):
    """A fix in running text (a startup or exit line, wrapped by the caller)."""
    return " ".join((fix or "").split("\n"))


def wrap_words(head, words, width=100, indent=None, sep=" "):
    """HEAD then WORDS, filled to WIDTH; continuation lines start at INDENT (default: under HEAD's text)."""
    ind = " " * (indent if indent is not None else len(head) + 1)
    lines, cur = [], head
    for w in words:
        if cur.strip() and len(cur) + len(sep) + len(w) > width:
            lines.append(cur)
            cur = ind + w
        elif not cur.strip() or cur == ind:
            cur = cur + w
        else:
            cur += (sep if not cur.endswith(" ") else "") + w
    lines.append(cur)
    return lines


def wrap_line(line, width=100, indent=4):
    """A long startup or exit line, wrapped at spaces, continuations indented."""
    if len(line) <= width:
        return [line]
    return wrap_words(line.split(" ")[0], line.split(" ")[1:], width, indent)


def kit_short(ident, report):
    return next((k["name"] for k in report.get("kits") or [] if k["kit"] == ident), ident)


def denial_text(ctxname, hostport, fix, count=1):
    """The one phrasing of a network refusal (exit line, denials file, pot why inside a run): what was
    refused, then fix_block's two lines."""
    host, _, port = hostport.rpartition(":") if ":" in hostport else (hostport, "", "443")
    flag = host if port == "443" else f"{host}:{port}"
    times = f", {count}x" if count > 1 else ""
    return "\n".join([f"pot ({ctxname}): the run tried {hostport} (not in this run{times})."]
                     + fix_block(one_run(ctxname, f"--net {flag} …"), fix))


def not_found_text(ctxname, name, fix, on_host=True):
    """A program that is not in the run (exit 127): with how to get it, when the host has it; else first
    that the host lacks it too."""
    if on_host:
        return "\n".join([f"pot ({ctxname}): {name} is not in this run."] + fix_block(one_run(ctxname, "--package PKG …"), fix))
    return "\n".join([f"pot ({ctxname}): {name} is not in this run, nor on the host's PATH.",
                      "  install it (for a repository's own tools, its package manager), then:"]
                     + fix_block(one_run(ctxname, "--package PKG …"), fix))


def grant_count(n, what="request"):
    return f"{n} {what}{'' if n == 1 else 's'}"


def next_action(st):
    """What to run for a kit that is not granted as it is, and why, short (explain's next row, grant --diff)."""
    n = len(st.get("added") or [])
    if st["status"] == "never":
        return f"pot grant {st['name']} ({grant_count(n)})"
    if st["status"] == "grown":
        return f"pot grant {st['name']} ({n} more)"
    if st["status"] == "source":
        return f"pot grant {st['name']} (changed source)"
    if st["status"] == "unverified":
        return f"{st['name']}: gitlink is not a release tag at its URL; its kit is unverified and its grant does not apply"
    return None


def utf8_output(stream=None, env=None):
    """Whether marks like ✓ can be written: a terminal whose locale says UTF-8."""
    import os as _os
    import sys as _sys
    stream = stream or _sys.stdout
    env = env if env is not None else _os.environ
    loc = env.get("LC_ALL") or env.get("LC_CTYPE") or env.get("LANG") or ""
    return bool(getattr(stream, "isatty", lambda: False)()) and ("UTF-8" in loc.upper() or "UTF8" in loc.upper())


GRANT_MARKS = {True: {"ok": "✓", "no": "✗", "chg": "~"}, False: {"ok": "ok", "no": "no", "chg": "~"}}
SUMMARY_ORDER = ("match", "none", "never", "grown", "source", "unverified")


def implied_identity(st):
    """Whether a kit's identity is what its short name implies (root:NAME; a URL or origin ending in NAME)."""
    ident, name = st["kit"], st["name"]
    if ident == f"root:{name}":
        return True
    last = ident.rstrip("/").removesuffix("/.git").rsplit("/", 1)[-1].removesuffix(".git")
    return last == name


def grant_diff_lines(sts, utf8=False, verbose=False):
    """pot grant --diff: a one-line summary, then one line per kit (mark, name, status, the command to
    run), and the items of each kit whose grant changed (asks for more, changed source); a never-granted
    kit's items with verbose only."""
    marks = GRANT_MARKS[utf8]
    count = {}
    for st in sts:
        count[st["status"]] = count.get(st["status"], 0) + 1
    keys = [k for k in SUMMARY_ORDER if count.get(k)]
    parts = [f"{count[k]} {STATUS_WORDS[k]}" for k in keys]
    if parts:
        parts[0] = f"{count[keys[0]]} kit{'' if count[keys[0]] == 1 else 's'} {STATUS_WORDS[keys[0]]}"
    lines = [" · ".join(parts) if parts else "no kits here"]
    w = max([len(st["name"]) for st in sts] + [0])
    mw = max(len(m) for m in marks.values())
    for st in sts:
        mark = marks["ok"] if st["status"] in ("match", "none") else marks["no"] if st["status"] in ("never", "unverified") else marks["chg"]
        words = STATUS_WORDS[st["status"]]
        if st["status"] == "never":
            words += f" ({grant_count(len(st.get('added') or []))})"
        elif st["status"] == "grown":
            words += f" ({len(st.get('added') or [])} more)"
        tail = f" · pot grant {st['name']}" if st["status"] in ("never", "grown", "source") else ""
        ident = "" if implied_identity(st) and not verbose else f" ({st['kit']})"
        lines.append(f"{mark:<{mw}} {st['name']:<{w}}  {words}{tail}{ident}".rstrip())
        if st["status"] == "source":
            lines.append(f"{'':<{mw}}   source: {fmt_source(st.get('was'))} -> {fmt_source(st.get('now'))}")
            lines += [f"{'':<{mw}}     {d}" for d in st.get("diffstat") or []]
        if st["status"] == "grown" or (verbose and st["status"] in ("never", "unverified")):
            lines += [f"{'':<{mw}}   + {a}" for a in st.get("added") or []]
    return lines





def binding_desc(cap, cfg, root_name=None):
    """A binding as people read it, in explain and the grant prompt (Appendix C)."""
    cfg = cfg or {}
    src = cfg.get("from") or {}
    if cap == "status.relay":
        return "workmux relay"
    if cap == "git.fetch":
        return "ssh broker (fetch only)" + (f", key {cfg['key']['file']}" if (cfg.get("key") or {}).get("file") else ", the host's ssh agent")
    if cap == "nix":
        return "untrusted daemon relay: exposes the whole store read-only, drops closure-store"
    if cap in ("credential", "mcp"):
        where = src.get("file") or src.get("envFile") or (f"host ${src['env']}" if isinstance(src.get("env"), str) else "the host's environment")
        how = f"{cfg.get('via', 'env')} {where}" if cap == "credential" else f"env {cfg.get('env', '')} from {where}"
        return how + " (the run holds it)"
    return "bound"


def label(x, report=None):
    """A source as Appendix C labels it: (pot), (root), (repo), (<kit>), (one run), (<context>)."""
    if "kit" in x:
        k = x["kit"]
        if k.startswith("root:"):
            return k[5:]
        if k.startswith("repo:"):
            return "repo"
        return kit_short(k, report or {})
    if "flag" in x:
        return "one run"
    if "root" in x:
        return {"default": "pot", "bundle": "repo"}.get(x["root"], "root")
    if "context" in x:
        return x["context"]
    if "binding" in x:
        return "binding"
    return ""


def explain_text(policy, report, root, width=100):
    """pot explain: every item with the file or grant it came from (Appendix C's shape), in sections
    separated by a blank line: the run (a summary: what runs, where, and what to do next), see, limits,
    reach, not in run, backend. WIDTH is the terminal's (100 when it is not one)."""
    out = []
    ctx = report["context"]
    cmd = report.get("command") or {}
    eff = [i for i in policy["items"] if i["effective"]]
    status_of = {k["kit"]: k for k in report.get("kits") or []}

    def src(i):
        return ", ".join(dict.fromkeys(l for l in (label(x, report) for x in i["sources"]) if l))

    def by(i):
        """The granted-by column: the kit's own name."""
        return ", ".join(dict.fromkeys(kit_short(x["kit"], report) if "kit" in x else label(x, report)
                                       for x in i["sources"] if "kit" in x or "flag" in x or "context" in x or "binding" in x))

    # ---- the run: a summary first
    out.append(f"run        {cmd.get('name') or '(shell)'}" + (f" -> {cmd['kit']}" if cmd.get("kit") else " (no kit ships it)"))
    inc = ctx.get("include") or []
    out.append(f"context    {ctx['name']}: {policy['mode']}, access {policy['access']}, {policy['backend']['name']}"
               + (f"; includes {', '.join('root:' + n for n in inc)}" if inc else ""))
    for w in policy["workspace"]:
        out.append(f"workspace  {w['id']} ({policy['access']}) {w['path']}")
    scions = [k for k in report["kits"] if k["kind"] == "scion"]
    if scions:
        out += wrap_words("           scions:", [f"{k['name']} {(k.get('source') or {}).get('tag', '')}".rstrip()
                                                 + (" (unverified)" if k["status"] == "unverified" else "") + ","
                                                 for k in scions], width, 19)
        out[-1] = out[-1].rstrip(",")
    todo = [next_action(k) for k in report["kits"] if k.get("applies") and k["status"] in ("never", "grown", "source", "unverified")]
    out += wrap_words("next       ", [t + ";" for t in todo[:-1]] + todo[-1:], width, 11) if todo else ["next       ready"]
    if report.get("argv") and report["argv"][:1] != [cmd.get("name")]:
        out += wrap_words("argv       ", report["argv"], width, 11)
    hooks = report.get("agentHooks") or []
    if hooks:
        out.append(f"hooks      {hooks[0]}")
        out += [f"           {h}" for h in hooks[1:]]

    # ---- see
    out.append("")
    pk = [i for i in eff if i["kind"] == "package"]
    std = [i for i in pk if any(x.get("root") == "standard" for x in i["sources"])]
    rest = [i for i in pk if i not in std]

    def pname(i):
        v = i.get("attr") or i["value"]
        return posixpath.basename(v)[33:] if v.startswith("/nix/store/") else v
    see = [f"standard tools ({len(std)})"] + [f"{pname(i)} ({src(i)})" for i in rest]
    out += wrap_words("see        ", [x + " ·" for x in see[:-1]] + see[-1:], width, 11)
    # env.set, file and dir rows, grouped by the kit (or source) that gives them
    groups = {}
    for i in eff:
        if i["kind"] in ("env.set", "file", "dir") and not i.get("mirror"):
            groups.setdefault(src(i), []).append(i)
    for g, items in groups.items():
        out.append(f"           {g}:")
        for i in items:
            if i["kind"] == "dir":
                n = len(i.get("links") or [])
                out.append(f"             dir {i['value']} (read-only): {n} {'entry' if n == 1 else 'entries'}")
                if i.get("skipped"):
                    kid = next((x["kit"] for x in i["sources"] if "kit" in x), None)
                    st = status_of.get(kid) or {}
                    head = f"               left out until {st['name']} is granted:" if st.get("status") in ("never", "grown") \
                        else "               left out:"
                    out += wrap_words(head, i["skipped"], width, 17)
            elif i["kind"] == "file":
                mode = {"fixed": "read-only", "seed": "copied in once", "reassert": "keys copied in at each start"}.get(i.get("mode"), "")
                out.append(f"             file {i['value']}" + (f", {mode}" if mode else ""))
            else:
                out.append(f"             env.set {i['value']}")
    for lab in dict.fromkeys(i["mirror"] for i in eff if i.get("mirror")):
        n = len([i for i in eff if i.get("mirror") == lab])
        out.append(f"           host {lab}: {n} files, read-only (what the run writes stays in its state)")

    # ---- limits: each protected path once, with every source's label
    out.append("")
    prot = [i for i in eff if i["kind"] == "protect"]
    if policy["access"] == "write":
        labels = {}
        for i in prot:
            for l in src(i).split(", "):
                if l and l not in labels.setdefault(i["value"], []):
                    labels[i["value"]].append(l)
        groups = {}
        for v, ls in labels.items():
            groups.setdefault(", ".join(ls), []).append(v)
        first = True
        for k, v in groups.items():
            out += wrap_words("limits     protect:" if first else " " * 20, v[:-1] + [f"{v[-1]} ({k})"], width, 20)
            first = False
    else:
        out.append(f"limits     the workspace is read-only (access {policy['access']})")
    hid = [i for i in eff if i["kind"] == "hidden"]
    out.append(f"           hidden: {len(hid)} paths (pot explain --json lists them)")
    if any(i["kind"] == "network" and i["value"] == "none" for i in policy["items"]):
        out.append(f"           network: none (context {ctx['name']})")

    # ---- reach
    out.append("")
    rows = []
    for v in [i for i in eff if i["kind"] == "view"]:
        rows.append(("view", f"inspect: the machine read-only, minus secrets; in ~: {' '.join(v['home'])}", ctx["name"]))
    for kind in ("network", "read", "write", "socket", "env", "state", "capability", "unsandboxed"):
        for i in [i for i in policy["items"] if i["kind"] == kind and i["class"] == "reach"]:
            if not i["effective"] and i.get("why") == "not in this run":
                continue      # listed once, under "not in run"
            v = i["value"] if kind != "capability" else i.get("request", i["value"]).replace("capability ", "", 1)
            if kind == "state":
                v = f"{i['value']} -> {i.get('at')}" + (" (shared)" if i.get("shared") else "")
            if kind == "env" and i.get("secret"):
                v += " (the run holds it)"
            if i.get("via") == "mcp":
                v += " (mcp)"
            if kind == "capability" and i["effective"]:
                b = next((b for b in policy.get("bindings") or [] if b["request"] == i.get("request") and b["kit"] in str(i["sources"])), None)
                if b:
                    cfg = ((root.get("bindings") or {}).get(b["capability"]) or {}).get(b["selector"])
                    v += f": {binding_desc(b['capability'], cfg)}"
            status = by(i) if i["effective"] else f"{by(i)}: {i.get('why', 'not in this run')}"
            rows.append((kind, v, status))
    w = min(max([len(v) for _, v, _ in rows] + [40]), 60) + 2
    out.append(f"reach{' ' * (w + 6)}granted by")
    for kind, v, status in rows:
        if len(v) > w:
            out += wrap_words(f"  {kind:<10} ", v.split(" "), width, 13)
            out.append(f"  {'':<10} {'':<{w}} {status}")
        else:
            out.append(f"  {kind:<10} {v:<{w}} {status}")
    kind = ((root.get("backends") or {}).get(policy["backend"]["name"]) or {}).get("kind")
    out += children_lines(policy.get("children") or {}, width, seatbelt=kind == "seatbelt")

    # ---- not in run: each kit's items under its row, filled to the width
    nir = [k for k in report["kits"] if k["status"] in ("never", "grown", "source", "unverified")]
    if nir:
        out.append("")
    for k in nir:
        out.append(f"not in run {k['name']} ({k['kit']}): {STATUS_WORDS[k['status']]}" + (":" if k["added"] else ""))
        if k["added"]:
            out += wrap_words(" " * 13, [a + "," for a in k["added"][:-1]] + k["added"][-1:], width, 13)

    # ---- backend
    out.append("")
    b = policy["backend"]
    lacks = [g for g in GUARANTEES if g not in b["proved"]]
    out += wrap_words("backend    ", f"{b['name']} proves {' '.join(b['proved']) or 'nothing'}; lacks {' '.join(lacks)}".split(" "),
                      width, 11)
    # §11: resource-limits is proved per run, for a run in a user systemd scope
    sc = report.get("scope") or {}
    lim = root.get("limits") or {}
    if sc.get("limits"):
        out.append(f"resources  this run: a user systemd scope, {sc['limits']}")
    elif sc.get("note"):
        out.append(f"resources  {sc['note']}")
    elif kind == "seatbelt":
        out.append("resources  none (Seatbelt has no resource limits)")
    else:
        out.append(f"resources  none at a terminal; a run without one gets a user systemd scope, "
                   f"MemoryMax={lim.get('memoryMax', '50%')} TasksMax={lim.get('tasksMax', 2048)}")
    out.append("note       a kit command started in this run (its wrapper) runs under its own policy, from the children table")
    return "\n".join(out) + "\n"


BUNDLE_VERSION = 1
BUNDLE_FIELDS = ("protect", "scions", "self", "version")


def validate_bundle(b, where="pot-kits"):
    """A pot-kits bundle's kit.json (contract pot-kits/1), checked: its version, its fields, each kit
    (unknown fields are errors, named by the kit's own `file`), and its protect list (paths relative to
    the flake directory, or "**/NAME")."""
    if not isinstance(b, dict):
        raise KitError(f"{where}: kit.json is not an object")
    if b.get("version") != BUNDLE_VERSION:
        raise KitError(f"{where}: kit.json is version {b.get('version')!r}, and this pot reads version {BUNDLE_VERSION}")
    for k in b:
        if k not in BUNDLE_FIELDS:
            raise KitError(f"{where}: kit.json: {k} is not a pot-kits field ({', '.join(BUNDLE_FIELDS)})")
    kits = ([("self", b["self"])] if b.get("self") is not None else []) + sorted((b.get("scions") or {}).items())
    if not isinstance(b.get("scions") or {}, dict):
        raise KitError(f"{where}: kit.json: scions is an object of kits")
    for n, k in kits:
        if not isinstance(k, dict) or not isinstance(k.get("file"), str):
            raise KitError(f"{where}: kit.json: {n} is not a kit with its file")
        validate_kit(k, k["file"], "/home/user", root=False)
    for p in _list(b.get("protect"), f"{where}: protect"):
        check_protect(p, f"{where}: protect")


def validate_root(root):
    """The root's manifest, checked when it builds: its kits, repo kits and contexts."""
    for n, k in (root.get("kits") or {}).items():
        if not NAME_RE.fullmatch(n) or n.count("@") > 1:
            raise KitError(f"the root's kits: {n!r} is not a kit name (NAME or NAME@INSTANCE)")
        validate_kit(k, k.get("file", f"programs.pot.kits.{n}"), "/home/user")
    for n, k in (root.get("repoKits") or {}).items():
        validate_kit(k, k.get("file", f"programs.pot.repoKits.{n}"), "/home/user", root=False)
    for n in (root.get("contexts") or {}):
        if n in VERBS:
            raise KitError(f"the root's contexts: {n} is a pot verb")
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
            # pot-kits (pot.lib.kitsFor, as it builds): the repo kit, each scion's, and the protect list
            validate_bundle(data)
        elif sys.argv[1] == "validate-root":
            validate_root(data)
        else:
            raise SystemExit(f"usage: compose.py validate|validate-root FILE")
    except Refusal as e:
        print(f"pot: {e.message}", file=sys.stderr)
        sys.exit(1)
