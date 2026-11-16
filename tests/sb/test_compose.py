"""Unit tests for graft sb's composition (lib/sb/compose.py): identity, normalisation, digests,
grants, merge rules, contexts, and policy.json v1's shape. Run from `nix flake check` (sb-unit), or
here: cd lib/sb && python3 -m unittest discover -s ../../tests/sb -t ."""
import copy
import json
import unittest

import compose as C

HOME = "/home/u"


def root(**kw):
    r = {"version": 1, "name": "t", "grantsFile": "/state/grants.json", "stateDir": "/state/sb", "kits": {},
         "contexts": {}, "hidden": [], "standard": ["/nix/store/aaa-bash"], "tools": [], "rootRepos": [],
         "repoKits": {}, "bindings": {}, "defaultContext": "work"}
    r.update(kw)
    return r


def ws(ident="repo:example.com/o/r", top="/home/u/src/r", kit=None, scions=(), tree="sha256-t"):
    return {"id": ident, "top": top, "common": top + "/.git", "origin": None, "originKey": None,
            "repoKit": {"manifest": kit, "file": "flake.nix (perSystem)"} if kit is not None else None,
            "rootRepoKits": [], "scions": list(scions), "tree": tree, "flakeDir": None}


def inp(r=None, context="work", argv=(), flags=None, workspace=None, grants=None, env=None, probe=None):
    return {"root": r or root(), "context": context, "argv": list(argv), "flags": flags or {},
            "workspace": [ws()] if workspace is None else workspace,
            "grants": grants or C.parse_grants(None), "machine": "m1", "run": "abcd",
            "probe": probe if probe is not None else ["closure-store", "own-netns", "own-pidns", "no-tiocsti",
                                                      "protected-as-mounts", "in-place", "root-files"],
            "backendName": "bubblewrap", "stateDir": "/state/sb",
            "host": {"home": HOME, "user": "u", "env": env or {}, "resolve": {}, "hidden": {}, "tz": "UTC"}}


def granted(*pairs):
    g = C.parse_grants(None)
    for ident, reqs in pairs:
        C.write_grant(g, ident, reqs, None, "m1", "today", False)
    return g


class Normalise(unittest.TestCase):
    # first checks, item 12
    CASES = [
        ("git@github.com:danielnaab/notebook-kit.git", "github.com/danielnaab/notebook-kit"),
        ("ssh://git@github.com/danielnaab/notebook-kit", "github.com/danielnaab/notebook-kit"),
        ("ssh://git@github.com:22/danielnaab/notebook-kit.git", "github.com/danielnaab/notebook-kit"),
        ("https://GitHub.com/DanielNaab/Notebook-Kit/", "github.com/danielnaab/notebook-kit"),
        ("https://github.com:443/danielnaab/notebook-kit.git/", "github.com/danielnaab/notebook-kit"),
        ("https://user:TOKEN@github.com/danielnaab/notebook-kit.git", "github.com/danielnaab/notebook-kit"),
        ("https://github.com/GSA-TTS/agentic-coding-quickstart.git", "github.com/gsa-tts/agentic-coding-quickstart"),
        ("ssh://forgejo@platform-vm:2222/daniel/graft.git", "platform-vm:2222/daniel/graft"),
        ("https://platform-vm:3000/daniel/graft.git", "platform-vm:3000/daniel/graft"),
        ("forgejo@platform-vm:daniel/graft.git", "platform-vm/daniel/graft"),
        ("ssh://forgejo@platform-vm:2222/daniel/Graft.git", "platform-vm:2222/daniel/Graft"),
        ("https://example.org//a//b.git?x=1#f", "example.org/a/b"),
    ]

    def test_table(self):
        for url, key in self.CASES:
            self.assertEqual(C.normalise_origin(url), key, url)

    def test_local(self):
        self.assertIsNone(C.normalise_origin("/home/u/src/garden"))
        self.assertIsNone(C.normalise_origin("file:///srv/x.git"))

    def test_no_token_anywhere(self):
        self.assertNotIn("TOKEN", C.repo_identity("https://u:TOKEN@x.org/a.git", "m", "/c"))
        self.assertNotIn("TOKEN", C.scion_identity("https://u:TOKEN@x.org/a"))

    def test_identity_forms(self):
        self.assertEqual(C.repo_identity("ssh://forgejo@platform-vm:2222/daniel/flexion-assistant.git", "m", "/c"),
                         "repo:platform-vm:2222/daniel/flexion-assistant")
        self.assertEqual(C.repo_identity(None, "platform-vm", "/home/u/scratch/.git"),
                         "repo:platform-vm:/home/u/scratch/.git")
        self.assertTrue(C.is_local_identity("repo:platform-vm:/home/u/scratch/.git"))
        self.assertFalse(C.is_local_identity("repo:platform-vm:2222/daniel/x"))
        self.assertEqual(C.scion_identity("https://github.com/danielnaab/meeting-kit"),
                         "git:https://github.com/danielnaab/meeting-kit")


class Requests(unittest.TestCase):
    def reqs(self, r):
        return C.parse_requests(r, "k", HOME)

    def test_canonical_paths(self):
        a = self.reqs({"read": ["/home/u/src/agent-skills/"], "write": ["~/x//y/./z"]})
        self.assertEqual([x.canonical for x in a], ["read ~/src/agent-skills", "write ~/x/y/z"])

    def test_dotdot_refused(self):
        with self.assertRaisesRegex(C.KitError, r"\.\."):
            self.reqs({"read": ["~/a/../b"]})

    def test_relative_refused(self):
        with self.assertRaises(C.KitError):
            self.reqs({"read": ["src"]})

    def test_network(self):
        a = self.reqs({"network": ["api.anthropic.com", "x.org:8443", "*.npmjs.org"]})
        self.assertEqual([(x.canonical, x.cls) for x in a], [
            ("network api.anthropic.com:443", "ordinary"), ("network x.org:8443", "ordinary"),
            ("network *.npmjs.org:443", "powerful")])
        for bad in ("https://x", "x:0", "x:70000", "host-loopback:80", {"host": "x", "via": "forward"}):
            with self.assertRaises(C.KitError, msg=bad):
                self.reqs({"network": [bad]})

    def test_classes(self):
        a = self.reqs({"read": ["~/r"], "write": ["~/w"], "socket": ["~/s"], "env": ["A", {"name": "B", "secret": True}],
                       "state": [{"name": "c", "at": "~/.c"}, {"name": "d", "shared": True}], "unsandboxed": True,
                       "capabilities": [{"name": "status.relay"}, {"name": "git.fetch", "remote": "ssh://h/*"},
                                        {"name": "credential", "host": "api.anthropic.com", "env": "T"},
                                        {"name": "mcp", "server": "metamcp"}, {"name": "nix"}]})
        got = {x.canonical: x.cls for x in a}
        self.assertEqual(got, {
            "read ~/r": "ordinary", "write ~/w": "powerful", "socket ~/s": "powerful", "env A": "ordinary",
            "env B": "ordinary", "state c": "ordinary", "state d shared": "powerful", "unsandboxed": "powerful",
            "capability status.relay": "ordinary", "capability git.fetch ssh://h/*": "ordinary",
            "capability credential api.anthropic.com env=T": "powerful", "capability mcp metamcp": "powerful",
            "capability nix": "powerful"})

    def test_unknown_kind_and_field(self):
        with self.assertRaisesRegex(C.KitError, "not a request kind"):
            self.reqs({"paths": []})
        with self.assertRaisesRegex(C.KitError, "unknown to this version"):
            self.reqs({"capabilities": [{"name": "nix", "store": "x"}]})
        with self.assertRaisesRegex(C.KitError, "not a capability"):
            self.reqs({"capabilities": [{"name": "git.pull"}]})

    def test_reserved_capabilities_parse(self):
        for c in ("git.push", "git.sign", "ops.read", "delegate"):
            self.assertEqual(self.reqs({"capabilities": [{"name": c}]})[0].cls, "powerful")


class Kits(unittest.TestCase):
    def test_unknown_field(self):
        with self.assertRaisesRegex(C.KitError, "not a kit field"):
            C.validate_kit({"sandbox": {}}, "f", HOME)

    def test_owned_env(self):
        for n in ("PATH", "HOME", "TZ", "GRAFT_SANDBOX_X", "HTTPS_PROXY", "NIX_REMOTE", "BASH_ENV", "SSL_CERT_FILE"):
            with self.assertRaisesRegex(C.KitError, "sb's own", msg=n):
                C.validate_kit({"env": {"set": {n: "x"}}}, "f", HOME)

    def test_protect_forms(self):
        C.validate_kit({"protect": ["a/b", "**/x", ".githooks"]}, "f", HOME)
        for bad in ("/abs", "~/x", "a/../b", "a*", "**/a/b"):
            with self.assertRaises(C.KitError, msg=bad):
                C.validate_kit({"protect": [bad]}, "f", HOME)

    def test_command_requests_counted(self):
        r = C.validate_kit({"requests": {"read": ["~/a"]}, "commands": {"c": {"requests": {"network": ["x.org"]}}}}, "f", HOME)
        self.assertEqual([(x.canonical, x.command) for x in r], [("read ~/a", None), ("network x.org:443", "c")])


class Digests(unittest.TestCase):
    def test_order_and_duplicates(self):
        self.assertEqual(C.digest(["b", "a", "a"]), C.digest(["a", "b"]))
        self.assertTrue(C.digest(["a"]).startswith("sha256-"))
        self.assertNotEqual(C.digest(["a"]), C.digest(["a", "b"]))

    def test_machine_independent(self):
        a = C.parse_requests({"read": ["/home/u/src/x"]}, "k", "/home/u")
        b = C.parse_requests({"read": ["/Users/u/src/x"]}, "k", "/Users/u")
        self.assertEqual(C.digest([x.canonical for x in a]), C.digest([x.canonical for x in b]))


class Grants(unittest.TestCase):
    def test_parse_errors(self):
        for bad in ("{", '{"version": 2, "canonical": 1}', '{"version": 1, "canonical": 1, "x": 1}',
                    '{"version":1,"canonical":1,"grants":{"root:a":{"digest":"sha256-x","requests":["a"]}}}'):
            with self.assertRaisesRegex(C.Refusal, "every run stops"):
                C.parse_grants(bad)

    def test_round_trip(self):
        g = granted(("root:a", ["network x:443"]))
        self.assertEqual(C.parse_grants(C.grants_dump(g)), g)

    def test_status(self):
        g = granted(("root:a", ["network x:443", "read ~/r"]))
        st = lambda reqs, **kw: C.grant_status("root:a", "root", reqs, False, None, g, "m1", **kw)["status"]
        self.assertEqual(st(["network x:443"]), "match")          # asking for less needs no approval
        self.assertEqual(st(["network y:443"]), "grown")
        self.assertEqual(st([]), "none")
        self.assertEqual(C.grant_status("root:b", "root", ["x"], False, None, g, "m1")["status"], "never")
        self.assertEqual(C.grant_status("git:u", "scion", ["x"], False, None, g, "m1", verified=False)["status"], "unverified")

    def test_source(self):
        g = C.parse_grants(None)
        C.write_grant(g, "git:u", ["capability nix"], {"commit": "c1", "tag": "v1.0.0"}, "m1", "t", False)
        s = lambda src, powerful=True: C.grant_status("git:u", "scion", ["capability nix"], powerful, src, g, "m1")["status"]
        self.assertEqual(s({"commit": "c1", "tag": "v1.0.0"}), "match")
        self.assertEqual(s({"commit": "c2", "tag": "v1.1.0"}), "source")
        self.assertEqual(s({"commit": "c2", "tag": "v1.1.0"}, powerful=False), "match")   # ordinary: digest alone
        # root kits carry no source (§7.4)
        C.write_grant(g, "root:k", ["capability nix"], None, "m1", "t", False)
        self.assertEqual(C.grant_status("root:k", "root", ["capability nix"], True, {"x": 1}, g, "m1")["status"], "match")

    def test_machine_sections(self):
        g = C.parse_grants(None)
        self.assertEqual(C.write_grant(g, "root:c", ["network api.anthropic.com:443"], None, "m1", "t", False), "grants")
        # a superset on m1: the shared entry (a run only gets what its own machine's kit asks for)
        self.assertEqual(C.write_grant(g, "root:c", ["network api.anthropic.com:443", "env A"], None, "m1", "t", False), "grants")
        self.assertEqual(g["grants"]["root:c"]["requests"], ["env A", "network api.anthropic.com:443"])
        # asking for less needs no approval: m2's smaller set matches the shared entry
        self.assertEqual(C.grant_status("root:c", "root", ["network api.anthropic.com:443"], False, None, g, "m2")["status"], "match")
        # diverging sets (Bedrock on m2): m2's section, the shared entry unchanged
        shared = copy.deepcopy(g["grants"]["root:c"])
        self.assertEqual(C.write_grant(g, "root:c", ["network bedrock:443"], None, "m2", "t", False), "machines.m2")
        self.assertEqual(g["grants"]["root:c"], shared)
        self.assertEqual(C.grant_status("root:c", "root", ["network bedrock:443"], False, None, g, "m2")["status"], "match")
        self.assertEqual(C.grant_status("root:c", "root", ["network bedrock:443"], False, None, g, "m1")["status"], "grown")
        # m2 later asks for a superset of the shared entry: back to the shared section, m2's entry gone
        self.assertEqual(C.write_grant(g, "root:c", ["network api.anthropic.com:443", "env A", "network bedrock:443"], None, "m2", "t", False), "grants")
        self.assertNotIn("m2", g["machines"])
        # no remote: its machine's section
        self.assertEqual(C.write_grant(g, "repo:m1:/s/.git", ["read ~/x"], None, "m1", "t", True), "machines.m1")
        self.assertEqual(C.revoke_grant(g, "root:c", "m2"), "grants")
        self.assertIsNone(C.revoke_grant(g, "root:c", "m2"))

    def test_arrived(self):
        a = granted(("root:a", ["x"]))
        b = copy.deepcopy(a)
        C.write_grant(b, "root:a", ["x", "y"], None, "m1", "t", False)
        C.write_grant(b, "root:b", ["z"], None, "m1", "t", False)
        self.assertEqual(C.arrived(a, b), ["root:a", "root:b"])
        self.assertEqual(C.arrived(a, a), [])


class Contexts(unittest.TestCase):
    def test_builtin_and_extends(self):
        r = root(contexts={"readonly": {"extends": "work", "access": "read"}})
        c = C.resolve_context(r, "readonly")
        self.assertEqual((c["mode"], c["access"], c["view"]), ("in-place", "read", None))
        self.assertEqual(C.resolve_context(r, "inspect")["view"], "inspect")

    def test_errors(self):
        with self.assertRaises(C.Refusal):
            C.resolve_context(root(), "nope")
        with self.assertRaises(C.Refusal):
            C.resolve_context(root(contexts={"a": {"extends": "b"}, "b": {"extends": "a"}}), "a")
        with self.assertRaises(C.Refusal):
            C.resolve_context(root(contexts={"a": {"grants": []}}), "a")
        with self.assertRaises(C.KitError):
            C.validate_root(root(contexts={"why": {}}))


class Compose(unittest.TestCase):
    def test_shape(self):
        p, r = C.compose(inp())
        self.assertEqual(set(p), {"version", "run", "context", "mode", "access", "backend", "workspace", "items",
                                  "mounts", "bindings", "children", "grants"})
        self.assertEqual(p["version"], 1)
        self.assertEqual(set(p["backend"]), {"name", "requires", "proved"})
        self.assertEqual(set(p["workspace"][0]), {"id", "path", "common", "protected", "clone"})
        self.assertEqual(set(p["mounts"]), {"state", "run"})
        self.assertEqual(p["mounts"]["run"], "/run/graft-sandbox")
        self.assertEqual(p["children"], {})
        for i in p["items"]:
            self.assertTrue({"kind", "value", "class", "sources", "effective"} <= set(i), i)
            self.assertIn(i["class"], ("see", "limits", "reach"))
        json.dumps(p)

    def test_base_protect_and_hidden(self):
        p, _ = C.compose(inp())
        prot = [i["value"] for i in p["items"] if i["kind"] == "protect"]
        for x in ("**/flake.nix", "**/graft.nix", "**/.gitmodules", "**/.envrc", "**/justfile", "**/.claude", ".githooks"):
            self.assertIn(x, prot)
        hid = [i["value"] for i in p["items"] if i["kind"] == "hidden"]
        for x in ("~/.ssh", "~/.claude", "/state/grants.json", "/state/sb"):
            self.assertIn(x, hid)

    def test_repo_kit_reach_needs_grant(self):
        kit = {"requests": {"network": ["pypi.org"]}, "packages": ["/nix/store/p-x"], "env": {"set": {"A": "1"}}}
        p, r = C.compose(inp(workspace=[ws(kit=kit)]))
        self.assertEqual([s["status"] for s in r["stale"]], ["never"])
        net = [i for i in p["items"] if i["kind"] == "network"]
        self.assertFalse(net[0]["effective"])
        # the repo kit's packages reach every run; its env.set only its own commands (§4.2)
        self.assertIn("/nix/store/p-x", [i["value"] for i in p["items"] if i["kind"] == "package"])
        self.assertNotIn("A=1", [i["value"] for i in p["items"] if i["kind"] == "env.set"])
        g = granted(("repo:example.com/o/r", ["network pypi.org:443"]))
        p, r = C.compose(inp(workspace=[ws(kit=kit)], grants=g))
        self.assertEqual(r["stale"], [])
        net = [i for i in p["items"] if i["kind"] == "network"][0]
        self.assertTrue(net["effective"])
        self.assertEqual(net["sources"][1]["digest"], C.digest(["network pypi.org:443"]))
        self.assertEqual(p["grants"], [{"id": "repo:example.com/o/r", "digest": C.digest(["network pypi.org:443"]), "match": True}])

    def test_grown_stops_only_its_runs(self):
        g = granted(("git:https://x/sk", ["network a.org:443"]))
        sc = {"name": "sk", "path": "scions/sk", "identity": "git:https://x/sk", "url": "https://x/sk", "commit": "c",
              "tag": "v1.0.0", "verified": True, "manifest": {"requests": {"network": ["a.org", "b.org"]},
                                                              "protect": ["sk.yaml"]}}
        p, r = C.compose(inp(workspace=[ws(scions=[sc])], grants=g))
        self.assertEqual(r["stale"], [])                       # a scion's reach applies only when its command runs
        self.assertIn("sk asks for more: sb grant", r["notes"])
        self.assertIn("sk.yaml", p["workspace"][0]["protected"])   # its limits apply

    def test_command_kit_and_include(self):
        r = root(kits={"cc": {"commands": {"claude": {"requests": {"network": ["api.anthropic.com"]}}},
                              "env": {"set": {"DISABLE_AUTOUPDATER": "1"}}},
                       "daniel": {"requests": {"network": ["pypi.org"]}}},
                 contexts={"work": {"include": ["daniel"]}})
        g = granted(("root:cc", ["network api.anthropic.com:443"]), ("root:daniel", ["network pypi.org:443"]))
        p, rep = C.compose(inp(r=r, argv=["claude"], grants=g))
        self.assertEqual(rep["command"], {"kit": "root:cc", "name": "claude"})
        eff = sorted(i["value"] for i in p["items"] if i["kind"] == "network" and i["effective"])
        self.assertEqual(eff, ["api.anthropic.com:443", "pypi.org:443"])
        self.assertIn("DISABLE_AUTOUPDATER=1", [i["value"] for i in p["items"] if i["kind"] == "env.set"])
        # another program: no command kit, so no claude reach, but the included kit's
        p, rep = C.compose(inp(r=r, argv=["bash"], grants=g))
        eff = sorted(i["value"] for i in p["items"] if i["kind"] == "network" and i["effective"])
        self.assertEqual(eff, ["pypi.org:443"])

    def test_accounts(self):
        r = root(kits={"cc@personal": {"commands": {"claude": {}}}, "cc@flexion": {"commands": {"claude": {}}}},
                 defaultAccount="personal")
        _, rep = C.compose(inp(r=r, argv=["claude"]))
        self.assertEqual(rep["command"]["kit"], "root:cc@personal")
        _, rep = C.compose(inp(r=r, argv=["claude"], flags={"account": "flexion"}))
        self.assertEqual(rep["command"]["kit"], "root:cc@flexion")
        self.assertIn("account flexion", rep["notes"])
        with self.assertRaises(C.Refusal):
            C.compose(inp(r=r, argv=["claude"], flags={"account": "nope"}))

    def test_two_kits_one_name(self):
        r = root(kits={"a": {"commands": {"x": {}}}, "b": {"commands": {"x": {}}}})
        with self.assertRaisesRegex(C.Refusal, "sb KIT:x"):
            C.compose(inp(r=r, argv=["x"]))
        _, rep = C.compose(inp(r=r, argv=["b:x"]))
        self.assertEqual(rep["command"]["kit"], "root:b")

    def test_workspace_may_not_shadow(self):
        r = root(kits={"a": {"commands": {"x": {}}}}, standardNames=["git"])
        for name in ("x", "git", "sb"):
            with self.assertRaisesRegex(C.Refusal, "a workspace kit may not"):
                C.compose(inp(r=r, workspace=[ws(kit={"commands": {name: {}}})]))

    def test_m2_conflicts(self):
        r = root(kits={"a": {"commands": {"x": {}}, "env": {"set": {"V": "1"}}}, "d": {"env": {"set": {"V": "2"}}}},
                 contexts={"work": {"include": ["d"]}})
        with self.assertRaisesRegex(C.Refusal, "two values"):
            C.compose(inp(r=r, argv=["x"]))
        r = root(kits={"a": {"commands": {"x": {}}, "files": {"~/f": {"source": "/nix/store/1-a"}}},
                       "d": {"files": {"~/f": {"source": "/nix/store/2-b"}}}}, contexts={"work": {"include": ["d"]}})
        with self.assertRaisesRegex(C.Refusal, "two different files"):
            C.compose(inp(r=r, argv=["x"]))

    def test_m3_hidden_reach_refused(self):
        r = root(kits={"d": {"requests": {"read": ["~/.ssh/config"]}}}, contexts={"work": {"include": ["d"]}})
        g = granted(("root:d", ["read ~/.ssh/config"]))
        i = inp(r=r, grants=g)
        i["host"]["hidden"] = {"~/.ssh": ["/home/u/.ssh"]}
        with self.assertRaisesRegex(C.Refusal, "hidden path"):
            C.compose(i)
        # a link that resolves into a hidden path
        r = root(kits={"d": {"requests": {"read": ["~/innocent"]}}}, contexts={"work": {"include": ["d"]}})
        g = granted(("root:d", ["read ~/innocent"]))
        i = inp(r=r, grants=g)
        i["host"]["hidden"] = {"~/.ssh": ["/home/u/.ssh"]}
        i["host"]["resolve"] = {"~/innocent": "/home/u/.ssh"}
        with self.assertRaisesRegex(C.Refusal, "hidden path"):
            C.compose(i)
        # the state directory is sb's alone (M4)
        with self.assertRaisesRegex(C.Refusal, "sb's own state"):
            C.compose(inp(flags={"read": ["/state/sb/runs"]}))

    def test_flags(self):
        p, rep = C.compose(inp(flags={"net": ["x.org"], "read": ["~/r"], "env": ["V"]}, env={"V": True}))
        f = {(i["kind"], i["value"]) for i in p["items"] if any("flag" in s for s in i["sources"])}
        self.assertEqual(f, {("network", "x.org:443"), ("read", "~/r"), ("env", "V")})
        self.assertEqual(len(rep["oneRun"]), 3)
        with self.assertRaises(C.Refusal):
            C.compose(inp(flags={"env": ["PATH"]}))
        with self.assertRaisesRegex(C.Refusal, "widen"):
            C.compose(inp(context="inspect", flags={"access": "write"}))
        p, _ = C.compose(inp(flags={"access": "read"}))
        self.assertEqual(p["access"], "read")
        self.assertEqual(p["workspace"][0]["protected"], [])

    def test_clone_lands_in_1b(self):
        with self.assertRaisesRegex(C.Refusal, "a later release"):
            C.compose(inp(context="dispatch"))
        with self.assertRaisesRegex(C.Refusal, "a later release"):
            C.compose(inp(flags={"mode": "clone"}))

    def test_inspect(self):
        r = root(kits={"cc": {"commands": {"claude": {"requests": {
            "network": ["api.anthropic.com"], "write": ["~/w"],
            "capabilities": [{"name": "mcp", "server": "m"}]}}}},
            "daniel": {"requests": {"network": ["pypi.org"]}}}, contexts={"inspect": {"include": ["daniel"]}},
            inspect={"home": ["~/src", "~/.workstation"]}, bindings={"mcp": {"m": {}}})
        g = granted(("root:cc", ["network api.anthropic.com:443", "write ~/w", "capability mcp m"]),
                    ("root:daniel", ["network pypi.org:443"]))
        p, _ = C.compose(inp(r=r, context="inspect", argv=["claude"], grants=g))
        eff = {(i["kind"], i["value"]) for i in p["items"] if i["effective"] and i["class"] == "reach"}
        self.assertIn(("network", "api.anthropic.com:443"), eff)
        self.assertNotIn(("network", "pypi.org:443"), eff)     # the command kit's model API only
        self.assertNotIn(("write", "~/w"), eff)
        self.assertNotIn(("capability", "mcp"), eff)
        view = [i for i in p["items"] if i["kind"] == "view"][0]
        self.assertEqual(view["home"], ["~/src", "~/.workstation"])
        self.assertNotIn("closure-store", p["backend"]["proved"])
        self.assertEqual(p["access"], "read")

    def test_requires(self):
        r = root(contexts={"strict": {"extends": "work", "requires": ["separate-kernel"]}})
        with self.assertRaises(C.Refusal) as e:
            C.compose(inp(r=r, context="strict"))
        self.assertEqual(e.exception.status, 125)
        self.assertIn("requires separate-kernel; bubblewrap proves", e.exception.message)

    def test_state_key(self):
        r = root(kits={"cc": {"commands": {"claude": {"requests": {"state": [{"name": "claude", "at": "~/.claude"}]}}}}})
        g = granted(("root:cc", ["state claude"]))
        p, _ = C.compose(inp(r=r, argv=["claude"], grants=g))
        s = p["mounts"]["state"][0]
        self.assertEqual(s["key"], ["root:cc", "repo:example.com/o/r", "work"])
        self.assertTrue(s["dir"].startswith("/state/sb/state/"))
        p2, _ = C.compose(inp(r=r, argv=["claude"], grants=g, context="inspect"))
        self.assertNotEqual(p2["mounts"]["state"][0]["dir"], s["dir"])
        p3, _ = C.compose(inp(r=r, argv=["claude"], grants=g, workspace=[]))
        self.assertEqual(p3["mounts"]["state"][0]["key"][1], "none")

    def test_capability_without_binding(self):
        r = root(kits={"d": {"requests": {"capabilities": [{"name": "nix"}]}}}, contexts={"work": {"include": ["d"]}})
        g = granted(("root:d", ["capability nix"]))
        p, rep = C.compose(inp(r=r, grants=g))
        c = [i for i in p["items"] if i["kind"] == "capability"][0]
        self.assertEqual((c["effective"], c["why"]), (False, "no binding"))
        self.assertEqual(len(rep["noBinding"]), 1)

    def test_unverified_scion_command(self):
        sc = {"name": "sk", "path": "s/sk", "identity": "git:u", "url": "u", "commit": "c", "tag": None,
              "verified": False, "manifest": {"commands": {"skc": {}}}}
        with self.assertRaisesRegex(C.Refusal, "not a verified release"):
            C.compose(inp(argv=["skc"], workspace=[ws(scions=[sc])]))

    def test_root_repos(self):
        self.assertEqual(C.root_repo_match("/home/u/.workstation", None, ["~/.workstation"], HOME), "~/.workstation")
        self.assertEqual(C.root_repo_match("/x", "platform-vm:2222/daniel/homelab",
                                           ["ssh://forgejo@platform-vm:2222/daniel/homelab"], HOME),
                         "ssh://forgejo@platform-vm:2222/daniel/homelab")
        self.assertIsNone(C.root_repo_match("/home/u/src/x", "github.com/a/b", ["~/.workstation"], HOME))

    def test_repo_kits_for(self):
        rk = {"platform-vm:2222/daniel/graft": {"requests": {"capabilities": [{"name": "nix"}]}}}
        self.assertEqual(len(C.repo_kits_for("platform-vm:2222/daniel/graft", "repo:platform-vm:2222/daniel/graft", rk)), 1)
        self.assertEqual(C.repo_kits_for("github.com/a/b", "repo:github.com/a/b", rk), [])

    def test_merge_manifests(self):
        m = C.merge_manifests([({"protect": ["a"], "requests": {"network": ["x"]}}, "f"),
                               ({"protect": ["b", "a"], "requests": {"network": ["y"]}}, "g")])
        self.assertEqual(m, {"protect": ["a", "b"], "requests": {"network": ["x", "y"]}})

    def test_explain_text(self):
        p, rep = C.compose(inp(flags={"net": ["x.org"]}))
        t = C.explain_text(p, rep, root())
        for s in ("context    work", "limits     protect:", "  network    x.org:443", "backend    bubblewrap proves",
                  "standard tools (1)", "comes in a later release"):
            self.assertIn(s, t)

    def test_permanent_fix(self):
        r = root(name="home-manager", kits={"daniel": {}}, contexts={"work": {"include": ["daniel"]}})
        _, rep = C.compose(inp(r=r, flags={"net": ["x.org"]}, workspace=[ws(kit={})]))
        self.assertEqual(C.permanent_fix("--net", "x.org:443", rep, r),
                         "the repo kit's requests.network (perSystem.kit), or programs.sb.kits.daniel.requests.network "
                         "for every repository (home-manager switch)")
        # an mkRoot root, a repository that is not a consumer, no included kit
        r = root()
        _, rep = C.compose(inp(r=r, flags={"net": ["x.org"]}))
        self.assertEqual(C.permanent_fix("--net", "x.org:443", rep, r),
                         'repoKits."example.com/o/r".requests.network (rebuild the root)')
        self.assertEqual(C.denial_text("work", "x.org:443", "F"),
                         "graft sandbox (work): the run tried x.org:443 (not in this run). one run: sb --net x.org …  permanent: F")

if __name__ == "__main__":
    unittest.main()
