"""v2.18.0: the children table (design §10), its narrowing proof, the refusal, --disable-userns, and the hook
variables a hook command gets."""
import json
import os
import tempfile
import unittest

import bwrap
import compose as C
import pot
from test_compose import granted, inp, root, ws

NEST = ["closure-store", "own-netns", "own-pidns", "no-tiocsti", "protected-as-mounts", "in-place", "root-files",
        "same-path-clone", "nest"]
NK = "git:example.com/o/nk"


def scion(commands):
    return {"name": "nk", "path": "s/nk", "identity": NK, "url": "https://example.com/o/nk", "commit": "c",
            "tag": "v1.0.0", "verified": True, "file": "s/nk/graft.nix",
            "manifest": {"version": "1.2", "commands": commands, "recipes": {"check": "chk", "meet": "wm --now"}, "file": "s/nk/graft.nix"}}


CMDS = {"chk": {"program": "/nix/store/y-chk/bin/chk", "access": "read"},
        "pc": {"program": "/nix/store/y-pc/bin/pc", "access": "read", "hook": "pre-commit"},
        "wm": {"program": "/nix/store/z-wm/bin/wm", "requests": {"env": ["WM_TOKEN"], "network": ["wm.example"]}}}


def table(r=None, probe=NEST, cmds=CMDS, extra_grants=(), argv=("ag",), context="work"):
    r = r or root(kits={"agent": {"commands": {"ag": {"program": "/nix/store/x-ag/bin/ag",
                                                       "requests": {"network": ["api.x"], "env": ["TOK"]}}}}})
    g = granted(("root:agent", ["env TOK", "network api.x:443"]),
                (NK, ["env WM_TOKEN", "network wm.example:443"]), *extra_grants)
    i = inp(r=r, argv=argv, context=context, workspace=[ws(kit={}, scions=[scion(cmds)])], grants=g, probe=probe)
    p, rep = C.compose(i)
    rows, pols = C.children_table(i, p, rep)
    p["children"] = rows
    return p, rows, pols


class Table(unittest.TestCase):
    def test_rows_and_keys(self):
        _, rows, _ = table()
        self.assertEqual(sorted(rows), ["dispatch root:agent:ag", f"{NK}:chk", f"{NK}:pc", f"{NK}:wm", "root:agent:ag"])
        self.assertEqual(rows["root:agent:ag"]["how"], "exec")
        self.assertEqual(rows[f"{NK}:chk"]["how"], "nest")
        self.assertEqual(rows[f"{NK}:chk"]["slot"], "scions.nk")
        self.assertEqual(rows[f"{NK}:pc"]["hook"], "pre-commit")
        self.assertEqual(rows["dispatch root:agent:ag"]["how"], "nest")
        self.assertEqual(rows["dispatch root:agent:ag"]["context"], "dispatch")

    def test_refusal_text(self):
        _, rows, _ = table()
        r = rows[f"{NK}:wm"]
        self.assertEqual(r["refused"], "needs wm.example, WM_TOKEN")
        self.assertEqual(r["text"], "pot (work): wm needs wm.example, WM_TOKEN, which this run (ag) lacks: "
                                    "run it on the host (just meet)")
        self.assertNotIn("policy", r)

    def test_shared_digest(self):
        _, rows, pols = table()
        self.assertEqual(rows[f"{NK}:chk"]["policy"], rows[f"{NK}:pc"]["policy"])
        self.assertIn(rows[f"{NK}:chk"]["policy"], pols)

    def test_untrusted(self):
        r = root(kits={"agent": {"commands": {"ag": {"program": "/nix/store/x-ag/bin/ag",
                                                      "requests": {"network": ["api.x"], "env": ["TOK"]}}}}},
                 contexts={"work": {"trust": "untrusted"}})
        _, rows, _ = table(r=r)
        self.assertEqual(rows[f"{NK}:chk"]["refused"], "this run's context is untrusted (nest = false)")
        self.assertEqual(rows["root:agent:ag"]["how"], "exec")

    def test_nest_false(self):
        r = root(kits={"agent": {"commands": {"ag": {"program": "/nix/store/x-ag/bin/ag",
                                                      "requests": {"network": ["api.x"]}}}}},
                 contexts={"work": {"nest": False}})
        _, rows, _ = table(r=r)
        self.assertEqual(rows[f"{NK}:chk"]["refused"], "context work has nest = false")

    def test_no_nest_probe(self):
        _, rows, _ = table(probe=[g for g in NEST if g != "nest"])
        self.assertEqual(rows[f"{NK}:chk"]["refused"], "bubblewrap can't nest here")

    def test_not_granted(self):
        r = root(kits={"agent": {"commands": {"ag": {"program": "/nix/store/x-ag/bin/ag",
                                                      "requests": {"network": ["api.x", "wm.example"], "env": ["TOK", "WM_TOKEN"]}}}}})
        i = inp(r=r, argv=["ag"], workspace=[ws(kit={}, scions=[scion(CMDS)])], probe=NEST,
                grants=granted(("root:agent", ["env TOK", "env WM_TOKEN", "network api.x:443", "network wm.example:443"])))
        p, rep = C.compose(i)
        rows, _ = C.children_table(i, p, rep)
        self.assertTrue(rows[f"{NK}:wm"]["grant"])
        self.assertEqual(rows[f"{NK}:wm"]["refused"], "not granted: pot grant nk (on the host)")

    def test_unsandboxed_child(self):
        cmds = dict(CMDS, un={"program": "/nix/store/u/bin/un", "requests": {"unsandboxed": True}})
        _, rows, _ = table(cmds=cmds, extra_grants=())
        # its kit's grant grew (unsandboxed is not in it): not granted first
        self.assertIn("refused", rows[f"{NK}:un"])

    def test_seatbelt(self):
        # design §10 on macOS: Seatbelt cannot nest. A narrower row is refused, naming why and the host fix; an equal
        # row still execs in place; a dispatch row is refused too
        probe = ["closure-store", "no-tiocsti", "in-place", "private-proxy"]
        r = root(kits={"agent": {"commands": {"ag": {"program": "/nix/store/x-ag/bin/ag",
                                                      "requests": {"network": ["api.x"], "env": ["TOK"]}}}}})
        g = granted(("root:agent", ["env TOK", "network api.x:443"]), (NK, ["env WM_TOKEN", "network wm.example:443"]))
        i = inp(r=r, argv=["ag"], workspace=[ws(kit={}, scions=[scion(CMDS)])], grants=g, probe=probe)
        i["backendName"] = "seatbelt"
        p, rep = C.compose(i)
        rows, _ = C.children_table(i, p, rep)
        self.assertEqual(rows["root:agent:ag"]["how"], "exec")
        chk = rows[f"{NK}:chk"]
        self.assertEqual(chk["refused"], "Seatbelt cannot nest")
        self.assertEqual(chk["text"], "pot (work): chk runs under its own, narrower policy, and Seatbelt cannot nest (a run "
                                      "on macOS starts no runs inside it): run it on the host (just check)")
        d = rows["dispatch root:agent:ag"]
        self.assertEqual(d["refused"], "Seatbelt cannot nest")
        self.assertTrue(d["text"].endswith("run it on the host (pot dispatch ag)"), d["text"])
        self.assertEqual(rows[f"{NK}:wm"]["refused"], "needs wm.example, WM_TOKEN")
        self.assertEqual(C.children_lines({}, seatbelt=True),
                         ["children   none (Seatbelt cannot nest: a run on macOS starts no runs inside it)"])
        self.assertIn("  chk: refused (Seatbelt cannot nest)", C.children_lines(rows, seatbelt=True))

    def test_sub_table(self):
        _, rows, _ = table()
        sub = rows[f"{NK}:pc"]["children"]
        self.assertEqual(sub[f"{NK}:chk"]["how"], "exec")
        self.assertNotIn(f"{NK}:pc", sub)
        self.assertNotIn("root:agent:ag", sub)      # wider than the hook command
        self.assertNotIn("children", sub[f"{NK}:chk"])

    def test_explain_rows(self):
        _, rows, _ = table()
        lines = C.children_lines(rows)
        self.assertEqual(lines[0], "children   dispatch ag (agent): nest · chk pc (nk): nest · ag (agent): exec")
        self.assertIn("  wm: refused (needs wm.example, WM_TOKEN)", lines)
        self.assertEqual(C.children_lines({}), ["children   none: this run keeps --disable-userns"])


def pol(items, access="write", bindings=(), top="/w"):
    return {"mode": "in-place", "access": access, "workspace": [{"path": top}], "bindings": list(bindings),
            "mounts": {"state": []},
            "items": [dict({"class": "reach", "effective": True}, **i) for i in items]}


class Narrower(unittest.TestCase):
    def test_network(self):
        self.assertEqual(C.narrower(pol([{"kind": "network", "value": "a.x:443"}]),
                                    pol([{"kind": "network", "value": "*.x:443"}])), [])
        self.assertEqual(C.narrower(pol([{"kind": "network", "value": "*.x:443"}]),
                                    pol([{"kind": "network", "value": "a.x:443"}])), ["*.x"])
        self.assertEqual(C.narrower(pol([{"kind": "network", "value": "a.x:8443"}]),
                                    pol([{"kind": "network", "value": "a.x:443"}])), ["a.x:8443"])

    def test_paths(self):
        self.assertEqual(C.narrower(pol([{"kind": "read", "value": "/w/sub", "path": "/w/sub"}], access="read"), pol([], access="read")), [])
        self.assertEqual(C.narrower(pol([{"kind": "write", "value": "/w/sub", "path": "/w/sub"}], access="read"), pol([], access="read")),
                         ["/w/sub"])

    def test_access(self):
        # conformance 1: a write child is never narrower than a read run
        self.assertEqual(C.narrower(pol([], access="write"), pol([], access="read")), ["write access"])
        self.assertEqual(C.narrower(pol([], access="read"), pol([], access="write")), [])
        self.assertEqual(C.narrower(pol([{"kind": "read", "value": "~/x/y", "path": "/h/x/y"}]),
                                    pol([{"kind": "write", "value": "~/x", "path": "/h/x"}])), [])
        self.assertEqual(C.narrower(pol([{"kind": "read", "value": "~/z", "path": "/h/z"}]),
                                    pol([{"kind": "read", "value": "~/x", "path": "/h/x"}])), ["~/z"])

    def test_env_state_unsandboxed(self):
        self.assertEqual(C.narrower(pol([{"kind": "env", "value": "A"}]), pol([{"kind": "env", "value": "A"}])), [])
        self.assertEqual(C.narrower(pol([{"kind": "env", "value": "B"}]), pol([])), ["B"])
        self.assertEqual(C.narrower(pol([{"kind": "state", "value": "s"}]), pol([])), [])
        self.assertEqual(C.narrower(pol([{"kind": "unsandboxed", "value": "true"}]), pol([])), ["unsandboxed"])

    def test_bindings_must_be_equal(self):
        b = {"capability": "git.fetch", "selector": "ssh://h/*", "request": "capability git.fetch ssh://h/a"}
        child = pol([{"kind": "capability", "value": "git.fetch", "request": b["request"]}], bindings=[b])
        self.assertEqual(C.narrower(child, pol([], bindings=[b])), [])
        other = dict(b, request="capability git.fetch ssh://h/*")
        self.assertEqual(C.narrower(child, pol([], bindings=[other])), ["git.fetch"])


class Userns(unittest.TestCase):
    """--disable-userns unless the children table has a row that nests (and the backend proved nest)."""

    def args(self, children, proved=("nest",)):
        d = tempfile.mkdtemp()
        policy = {"version": 1, "run": "abcd", "context": "work", "mode": "in-place", "access": "read",
                  "backend": {"name": "bubblewrap", "requires": [], "proved": list(proved)},
                  "workspace": [], "items": [], "mounts": {"state": [], "run": "/run/pot"}, "bindings": [],
                  "children": children, "grants": []}
        launch = {"argv": ["true"], "cwd": "/", "home": d + "/home", "user": "u", "uid": 1, "gid": 1, "group": "u",
                  "closure": [], "git": {}, "potBin": "/nix/store/p-pot/bin"}
        json.dump(policy, open(os.path.join(d, "policy.json"), "w"))
        json.dump(launch, open(os.path.join(d, "launch.json"), "w"))
        cfg = {"baseClosure": os.devnull, "netClosure": os.devnull, "sh": "/bin/sh", "env": "/usr/bin/env",
               "cacert": "/etc/hosts", "fd": "true", "find": "true", "git": "git", "shell": "/bin/sh", "python": "python3",
               "shim": "/x", "bwrap": "bwrap"}
        os.makedirs(d + "/home")
        bwrap.prepare(cfg, d)
        return open(os.path.join(d, "bwrap.args"), "rb").read().split(b"\0"), d

    def test_empty_table(self):
        a, _ = self.args({})
        self.assertIn(b"--disable-userns", a)

    def test_exec_only(self):
        a, _ = self.args({"k:c": {"how": "exec"}})
        self.assertIn(b"--disable-userns", a)

    def test_nest_row(self):
        a, d = self.args({"k:c": {"how": "nest"}})
        self.assertNotIn(b"--disable-userns", a)
        self.assertIn(b"/run/pot/children", a)
        self.assertIn(b"/run/pot/table", a)

    def test_nest_unproved(self):
        a, _ = self.args({"k:c": {"how": "nest"}}, proved=())
        self.assertIn(b"--disable-userns", a)

    def test_launcher_read_only(self):
        a, d = self.args({})
        i = a.index((d + "/home/.config/pot").encode())
        self.assertEqual(a[i - 2], b"--ro-bind")
        # conformance 3: the fixed path, a read-only mount point the run can't rename or replace
        i = a.index(b"/run/pot/launcher")
        self.assertEqual(a[i - 2:i], [b"--ro-bind", b"/nix/store/p-pot/bin/pot"])
        self.assertEqual(os.readlink(os.path.join(d, "potcfg", "launcher")), "/run/pot/launcher")


class HookEnv(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        os.makedirs(self.d + "/r/.git")
        self.policy = {"workspace": [{"path": self.d + "/r", "common": self.d + "/r/.git"}]}

    def test_plain_commit(self):
        e = pot.hook_env(self.policy, self.d + "/r", {"GIT_INDEX_FILE": ".git/index", "GIT_PREFIX": ""})
        self.assertEqual(e, {"GIT_INDEX_FILE": os.path.realpath(self.d + "/r/.git/index"), "GIT_PREFIX": ""})

    def test_commit_a_and_path(self):
        for v in (self.d + "/r/.git/index.lock", self.d + "/r/.git/next-index-1601703.lock"):
            self.assertEqual(pot.hook_env(self.policy, self.d + "/r", {"GIT_INDEX_FILE": v})["GIT_INDEX_FILE"],
                             os.path.realpath(v))

    def test_dropped(self):
        for v in ("/tmp/index", self.d + "/r/.git/config", self.d + "/r/index", "../../etc/index"):
            self.assertNotIn("GIT_INDEX_FILE", pot.hook_env(self.policy, self.d + "/r", {"GIT_INDEX_FILE": v}), v)
        self.assertEqual(pot.hook_env(self.policy, self.d + "/r", {"GIT_EXEC_PATH": "/x", "GIT_CONFIG_PARAMETERS": "'a'='b'",
                                                                  "GIT_PREFIX": "../x"}), {})


class DispatchRow(unittest.TestCase):
    def test_instance(self):
        rows = {"root:a@x:claude": {"mode": "in-place", "command": "claude", "kit": "root:a@x"},
                "dispatch root:a@x:claude": {"mode": "clone", "command": "claude", "kit": "root:a@x"},
                "dispatch root:a@y:claude": {"mode": "clone", "command": "claude", "kit": "root:a@y"}}
        self.assertEqual(pot.dispatch_row(rows, "claude")["kit"], "root:a@x")
        self.assertEqual(pot.dispatch_row(rows, "claude@y")["kit"], "root:a@y")
        self.assertIsNone(pot.dispatch_row(rows, "claude@z"))
        self.assertIsNone(pot.dispatch_row(rows, "other"))


class Manifest(unittest.TestCase):
    def test_not_a_manifest(self):
        with self.assertRaises(C.Refusal) as e:
            pot.read_manifest("/tmp/x.json", "chk")
        self.assertEqual(e.exception.status, 2)
        self.assertIn("is not a wrapper's manifest (<bundle>/commands/chk.json)", e.exception.message)


if __name__ == "__main__":
    unittest.main()
