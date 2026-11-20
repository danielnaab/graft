"""The children table (design §10; siblings since v2.20.0), its narrowing proof and the sibling admission, the
refusal, --disable-userns in every run, the hook variables a hook command gets, and the sibling client's checks."""
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
        # v2.20.0: no dispatch rows (decision 19)
        self.assertEqual(sorted(rows), [f"{NK}:chk", f"{NK}:pc", f"{NK}:wm", "root:agent:ag"])
        self.assertEqual(rows["root:agent:ag"]["how"], "exec")
        self.assertEqual(rows[f"{NK}:chk"]["how"], "sibling")
        self.assertEqual(rows[f"{NK}:chk"]["slot"], "scions.nk")
        self.assertEqual(rows[f"{NK}:pc"]["hook"], "pre-commit")
        self.assertNotIn("children", rows[f"{NK}:pc"])        # no sub-tables: a sibling's table is composed at its start

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
        # v2.20.0: siblings need no nest guarantee (no backend proves it any more)
        _, rows, _ = table(probe=[g for g in NEST if g != "nest"])
        self.assertEqual(rows[f"{NK}:chk"]["how"], "sibling")

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
        # v2.20.0: on macOS too a narrower row is a sibling (decision 13); an equal row execs in place
        probe = ["closure-store", "no-tiocsti", "in-place", "private-proxy"]
        r = root(kits={"agent": {"commands": {"ag": {"program": "/nix/store/x-ag/bin/ag",
                                                      "requests": {"network": ["api.x"], "env": ["TOK"]}}}}})
        g = granted(("root:agent", ["env TOK", "network api.x:443"]), (NK, ["env WM_TOKEN", "network wm.example:443"]))
        i = inp(r=r, argv=["ag"], workspace=[ws(kit={}, scions=[scion(CMDS)])], grants=g, probe=probe)
        i["backendName"] = "seatbelt"
        p, rep = C.compose(i)
        rows, _ = C.children_table(i, p, rep)
        self.assertEqual(rows["root:agent:ag"]["how"], "exec")
        self.assertEqual(rows[f"{NK}:chk"]["how"], "sibling")
        self.assertEqual(rows[f"{NK}:wm"]["refused"], "needs wm.example, WM_TOKEN")
        self.assertEqual(C.children_lines({}, seatbelt=True), ["children   none"])

    def test_explain_rows(self):
        _, rows, _ = table()
        lines = C.children_lines(rows)
        self.assertEqual(lines[0], "children   chk pc (nk): sibling · ag (agent): exec")
        self.assertIn("  wm: refused (needs wm.example, WM_TOKEN)", lines)
        self.assertEqual(C.children_lines({}), ["children   none"])

    def test_no_standing_reach(self):
        # decision 15: a row never gets the context's include (standing reach), and its kit's digest is the same
        r = root(kits={"agent": {"commands": {"ag": {"program": "/nix/store/x-ag/bin/ag",
                                                      "requests": {"network": ["api.x"], "env": ["TOK"]}}}},
                       "daniel": {"requests": {"network": ["code.x"], "env": ["STAND"]}}},
                 contexts={"work": {"include": ["daniel"]}})
        g = granted(("root:agent", ["env TOK", "network api.x:443"]), ("root:daniel", ["env STAND", "network code.x:443"]),
                    (NK, ["env WM_TOKEN", "network wm.example:443"]))
        i = inp(r=r, argv=["ag"], workspace=[ws(kit={}, scions=[scion(CMDS)])], grants=g, probe=NEST)
        p, rep = C.compose(i)
        self.assertIn("code.x:443", [x["value"] for x in C.eff_reach(p)])
        rows, pols = C.children_table(i, p, rep)
        cp = pols[rows[f"{NK}:chk"]["policy"]]
        self.assertNotIn("code.x:443", [x["value"] for x in C.eff_reach(cp)])
        self.assertNotIn("STAND", [x["value"] for x in C.eff_reach(cp)])
        i2 = dict(i, argv=["chk"], flags={"commandKit": NK, "noInclude": True})
        _, r2 = C.compose(i2)
        _, r1 = C.compose(dict(i, argv=["chk"], flags={"commandKit": NK}))
        self.assertEqual({k["kit"]: k["digest"] for k in r2["kits"] if k["kit"] != "root:daniel"},
                         {k["kit"]: k["digest"] for k in r1["kits"] if k["kit"] != "root:daniel"})


class Admit(unittest.TestCase):
    def policy(self, access="write", protect=(), mode="in-place", tops=("/w",), items=()):
        return {"mode": mode, "access": access, "workspace": [{"path": t} for t in tops], "bindings": [],
                "mounts": {"state": []},
                "items": [dict({"class": "reach", "effective": True}, **i) for i in items]
                + [{"kind": "protect", "value": v, "class": "limits", "effective": True} for v in protect]}

    def test_protect(self):
        caller = self.policy(protect=["**/.claude", "**/flake.nix"])
        self.assertEqual(C.admit(self.policy(protect=["**/flake.nix"]), caller), ["protect **/.claude"])
        self.assertEqual(C.admit(self.policy(protect=["**/flake.nix", "**/.claude", "x"]), caller), [])
        # a read sibling writes nothing, so the caller's protect needs no copy
        self.assertEqual(C.admit(self.policy(access="read"), caller), [])

    def test_mode_and_workspace(self):
        caller = self.policy(tops=("/w", "/v"))
        self.assertEqual(C.admit(self.policy(mode="clone"), self.policy()), ["clone mode"])
        self.assertEqual(C.admit(self.policy(tops=("/v",)), caller), [])
        self.assertEqual(C.admit(self.policy(tops=("/u",)), caller), ["the workspace /u"])

    def test_narrower_first(self):
        self.assertEqual(C.admit(self.policy(items=[{"kind": "env", "value": "T"}]), self.policy()), ["T"])


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

    def test_bindings_covered(self):
        b = {"capability": "git.fetch", "selector": "ssh://h/*", "request": "capability git.fetch ssh://h/a"}
        child = pol([{"kind": "capability", "value": "git.fetch", "request": b["request"]}], bindings=[b])
        self.assertEqual(C.narrower(child, pol([], bindings=[b])), [])
        other = dict(b, request="capability git.fetch ssh://h/*")
        self.assertEqual(C.narrower(child, pol([], bindings=[other])), ["git.fetch"])


class Userns(unittest.TestCase):
    """v2.20.0: --disable-userns in every run; a run with a sibling row (and a host pot) binds its broker's socket."""

    def args(self, children, proved=("nest",)):
        d = tempfile.mkdtemp()
        policy = {"version": 1, "run": "abcd", "context": "work", "mode": "in-place", "access": "read",
                  "backend": {"name": "bubblewrap", "requires": [], "proved": list(proved)},
                  "workspace": [], "items": [], "mounts": {"state": [], "run": "/run/pot"}, "bindings": [],
                  "children": children, "grants": []}
        launch = {"argv": ["true"], "cwd": "/", "home": d + "/home", "user": "u", "uid": 1, "gid": 1, "group": "u",
                  "closure": [], "git": {}, "potBin": "/nix/store/p-pot/bin", "hostPot": ["/bin/python3", "-I"]}
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

    def test_sibling_row(self):
        a, d = self.args({"k:c": {"how": "sibling"}})
        self.assertIn(b"--disable-userns", a)
        i = a.index(b"/run/pot/children.sock")
        self.assertEqual(a[i - 2:i], [b"--bind", (d + "/children.sock").encode()])
        self.assertNotIn(b"/run/pot/table", a)
        self.assertNotIn(b"/run/pot/children", a)

    def test_nest_row_gone(self):
        # an older table's nest row nests nothing: user namespaces stay off
        a, _ = self.args({"k:c": {"how": "nest"}})
        self.assertIn(b"--disable-userns", a)
        self.assertNotIn(b"/run/pot/children.sock", a)

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


class Sibling(unittest.TestCase):
    """The checks around a sibling start: which fds cross, the caller's cwd and hook variables (on the host)."""

    def test_crossable(self):
        d = tempfile.mkdtemp()
        r, w = os.pipe()
        f = os.open(os.path.join(d, "f"), os.O_RDWR | os.O_CREAT)
        dfd = os.open(d, os.O_RDONLY)
        n = os.open(os.devnull, os.O_RDWR)
        self.assertTrue(pot.crossable(r))
        self.assertTrue(pot.crossable(n))
        self.assertFalse(pot.crossable(f))        # a regular file: the client pipes it
        self.assertFalse(pot.crossable(dfd))      # a directory: it would carry the caller's view (the spike)
        if hasattr(os, "O_PATH"):
            pfd = os.open(d, os.O_PATH)
            self.assertFalse(pot.crossable(pfd))
        os.close(w)
        self.assertIsNone(pot.crossable(w))       # closed

    def test_cwd(self):
        d = os.path.realpath(tempfile.mkdtemp())
        os.makedirs(d + "/w/sub")
        os.symlink("/etc", d + "/w/out")
        pol = {"backend": {"proved": ["same-path-clone"]}, "workspace": [{"path": d + "/w", "common": d + "/w/.git"}]}
        self.assertEqual(pot.sibling_cwd(pol, d + "/w/sub"), (d + "/w/sub", d + "/w/sub"))
        self.assertEqual(pot.sibling_cwd(pol, d + "/w/out"), (d + "/w", d + "/w"))      # a link out: the top
        self.assertEqual(pot.sibling_cwd(pol, "/etc"), (d + "/w", d + "/w"))
        self.assertEqual(pot.sibling_cwd(pol, d + "/w/../x"), (d + "/w", d + "/w"))
        self.assertEqual(pot.sibling_cwd(pol, 7), (d + "/w", d + "/w"))
        # a clone: at the original path in the run, the clone's on the host
        os.makedirs(d + "/clone/sub")
        cl = {"backend": {"proved": ["same-path-clone"]}, "workspace": [{"path": d + "/w", "clone": d + "/clone"}]}
        self.assertEqual(pot.sibling_cwd(cl, d + "/w/sub"), (d + "/w/sub", d + "/clone/sub"))

    def test_hook(self):
        pol = {"backend": {"proved": []}, "workspace": [{"path": "/w", "common": "/w/.git"}]}
        self.assertEqual(pot.sibling_hook(pol, {"GIT_INDEX_FILE": "/w/.git/index.lock", "GIT_PREFIX": "a/"}),
                         {"GIT_INDEX_FILE": "/w/.git/index.lock", "GIT_PREFIX": "a/"})
        for v in ("/w/.git/../../etc/index", "/tmp/index", "/w/.git/config", "w/.git/index", 3):
            self.assertEqual(pot.sibling_hook(pol, {"GIT_INDEX_FILE": v}), {}, v)
        self.assertEqual(pot.sibling_hook(pol, {"GIT_PREFIX": "../x", "GIT_EXEC_PATH": "/x"}), {})
        cl = {"backend": {"proved": ["same-path-clone"]}, "workspace": [{"path": "/w", "clone": "/s/c", "common": "/s/c/.git"}]}
        self.assertEqual(pot.sibling_hook(cl, {"GIT_INDEX_FILE": "/w/.git/index"}), {"GIT_INDEX_FILE": "/w/.git/index"})


class NoRow(unittest.TestCase):
    """Decision 16: a wrapper whose command has no row in the run's table is refused (126), never run under the run's
    own policy; a refused row says why; a grant row exits 125."""

    def run_inner(self, rows):
        saved = pot.read_manifest
        pot.read_manifest = lambda path, name: ({"kit": "scions.nk", "command": name, "version": 1},
                                                {"program": "/nix/store/y-chk/bin/chk"})
        try:
            with self.assertRaises(C.Refusal) as e:
                pot.inner_run({}, {"context": "work", "children": rows}, {"command": "chk", "manifest": "/m"}, [])
            return e.exception
        finally:
            pot.read_manifest = saved

    def test_no_row(self):
        e = self.run_inner({})
        self.assertEqual(e.status, 126)
        self.assertEqual(e.message, "pot (work): chk: not in this run's children table (a wrapper from another "
                                    "workspace?): run it on the host")

    def test_other_context_row(self):
        e = self.run_inner({"k:chk": {"slot": "scions.nk", "command": "chk", "context": "dispatch", "how": "exec"}})
        self.assertEqual(e.status, 126)

    def test_refused_and_grant(self):
        e = self.run_inner({"k:chk": {"slot": "scions.nk", "command": "chk", "context": "work", "refused": "needs X",
                                      "text": "pot (work): chk needs X, which this run (ag) lacks: run it on the host (chk ARGS)"}})
        self.assertEqual((e.status, e.message), (1, "pot (work): chk needs X, which this run (ag) lacks: run it on the host (chk ARGS)"))
        e = self.run_inner({"k:chk": {"slot": "scions.nk", "command": "chk", "context": "work", "grant": True,
                                      "refused": "not granted: pot grant nk (on the host)"}})
        self.assertEqual(e.status, C.EXIT_GRANT)

    def test_sibling_flag_inside(self):
        with self.assertRaises(C.Refusal) as e:
            pot.inner_run({}, {"context": "work"}, {"sibling": "3"}, [])
        self.assertEqual(e.exception.status, 2)


class Entry(unittest.TestCase):
    """The host entry (pot run --sibling FD): the caller's policy checked against the digest its start recorded, the
    row, and the depth cap (decision 18), all before anything is composed."""

    def entry(self, depth=0, tamper=False, how="sibling"):
        import signal
        d = tempfile.mkdtemp()
        pol = {"context": "work", "run": "abcd", "workspace": [{"path": d}],
               "children": {"k:chk": {"how": how, "command": "chk", "kit": "k", "slot": "self"}}}
        with open(os.path.join(d, "policy.json"), "w") as f:
            json.dump(pol, f)
        dig = pot.file_digest(os.path.join(d, "policy.json"))
        if tamper:
            with open(os.path.join(d, "policy.json"), "a") as f:
                f.write(" ")
        with open(os.path.join(d, "launch.json"), "w") as f:
            json.dump({"policyDigest": dig, "depth": depth}, f)
        r, w = os.pipe()
        os.write(w, json.dumps({"parent": d, "key": "k:chk", "args": [], "cwd": d, "hook": {}}).encode())
        os.close(w)
        saved = {x: signal.getsignal(x) for x in (signal.SIGINT, signal.SIGTERM)}
        try:
            with self.assertRaises(C.Refusal) as e:
                pot.sibling_entry(None, {}, {"sibling": str(r)})
            return e.exception
        finally:
            for x, h in saved.items():
                signal.signal(x, h)

    def test_depth_cap(self):
        e = self.entry(depth=pot.SIBLING_DEPTH)
        self.assertEqual(e.message, "pot (work): chk: this run is 8 starts deep (pot's cap): run it on the host")

    def test_tampered_policy(self):
        e = self.entry(tamper=True)
        self.assertEqual(e.status, 2)
        self.assertIn("not a broker's request (the caller's policy.json changed)", e.message)

    def test_exec_row_is_no_sibling(self):
        e = self.entry(how="exec")
        self.assertIn("not a sibling row in the caller's children table", e.message)


class Manifest(unittest.TestCase):
    def test_not_a_manifest(self):
        with self.assertRaises(C.Refusal) as e:
            pot.read_manifest("/tmp/x.json", "chk")
        self.assertEqual(e.exception.status, 2)
        self.assertIn("is not a wrapper's manifest (<bundle>/commands/chk.json)", e.exception.message)


if __name__ == "__main__":
    unittest.main()
