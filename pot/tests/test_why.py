"""pot why --json's answer (contract run-env/1): what a host, host:port or git URL reaches, from a
policy. Run with the other unit tests (pot-unit)."""
import unittest

import pot

POLICY = {"context": "work", "items": [
    {"kind": "network", "class": "reach", "effective": True, "value": "github.com:443"},
    {"kind": "network", "class": "reach", "effective": False, "value": "pypi.org:443"},
    {"kind": "network", "class": "reach", "effective": True, "value": "*.example.org:443"}],
    "bindings": [{"capability": "git.fetch", "remotes": ["ssh://forgejo@platform-vm:2222/*"]}]}


class Reach(unittest.TestCase):
    def test_targets(self):
        for t, want in [
            ("github.com", ("network", "github.com:443", True)),
            ("GitHub.com:443", ("network", "github.com:443", True)),
            ("github.com:22", ("network", "github.com:22", False)),
            ("pypi.org", ("network", "pypi.org:443", False)),          # requested, not effective
            ("https://github.com/o/r.git", ("network", "github.com:443", True)),
            ("https://u@a.example.org/x", ("network", "a.example.org:443", True)),
            ("http://github.com/o/r", ("network", "github.com:80", False)),
            ("ssh://forgejo@platform-vm:2222/daniel/graft.git", ("git.fetch", "platform-vm:2222/daniel/graft", True)),
            ("git@github.com:o/r.git", ("git.fetch", "github.com/o/r", False)),
            ("file:///", ("file", "/", None)),     # a path: why_json answers it from the policy
        ]:
            self.assertEqual(pot.reach_of(POLICY, t), want, t)

    def test_not_a_target(self):
        for t in ("bash", "HOME", "./x"):
            self.assertIsNone(pot.reach_of(POLICY, t))


class Classify(unittest.TestCase):
    """F10: one classification for pot why's text and --json; a git URL is never a path."""

    def test_kinds(self):
        for t, k in [("ssh://git@h/x.git", "url"), ("git@github.com:o/r.git", "url"), ("https://h/x", "url"),
                     ("file:///tmp", "url"), ("/etc/passwd", "path"), ("~/.ssh", "path"), ("a/b", "path"),
                     ("HOME", "var"), ("pypi.org", "host"), ("h:22", "host"), ("bash", "program")]:
            self.assertEqual(pot.classify(t), k, t)

    def test_command_first(self):
        # a root kit command is a command for text and --json alike, even when a path of that name exists (R5)
        self.assertEqual(pot.classify("nix", {"nix": ["k"]}), "command")
        self.assertEqual(pot.classify("compose.py"), "path")    # an existing name with a dot is a path, not a host

    def test_requested_not_in_run(self):
        pol = dict(POLICY, workspace=[], access="write", backend={"proved": []},
                   items=[{"kind": "read", "value": "~/src/s", "class": "reach", "effective": False, "path": "/home/u/src/s",
                           "why": "not in this run", "sources": [{"kit": "root:cc@flexion", "as": "request"}]}])
        t, reached = pot.path_answer(pol, "/home/u/src/s", "~/src/s", "pot (work):", "/home/u", {}, lambda i: "cc@flexion")
        self.assertEqual((t, reached), ("pot (work): ~/src/s is requested (read) by an instance of cc, not in this run", False))

    def test_text_and_json_agree(self):
        import io, contextlib, json
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            rc = pot.why_json(POLICY, "ssh://git@fake.example/x.git", lambda x: pot.answer_why(POLICY, x, "/home/u", {}), "/home/u")
        j = json.loads(out.getvalue())
        self.assertEqual((rc, j["kind"], j["reached"]), (1, "git.fetch", False))
        self.assertEqual(j["text"], pot.answer_why(POLICY, "ssh://git@fake.example/x.git", "/home/u", {}))
        self.assertNotIn("not in this run: pot --read", j["text"])

    def test_path_from_policy(self):
        import io, contextlib, json
        pol = dict(POLICY, items=POLICY["items"] + [{"kind": "hidden", "value": "~/.ssh", "class": "limits", "effective": True,
                                                       "sources": [{"root": "default"}], "paths": ["/home/u/.ssh"]}],
                   workspace=[], access="write", backend={"proved": []})
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            rc = pot.why_json(pol, "/home/u/.ssh/id", lambda x: pot.answer_why(pol, x, "/home/u", {}), "/home/u")
        j = json.loads(out.getvalue())
        self.assertEqual((rc, j["kind"], j["reach"], j["reached"]), (1, "file", "/home/u/.ssh/id", False))   # reach: the path as given (N-a)
        self.assertIn("is hidden by your root", j["text"])


class MissingPath(unittest.TestCase):
    def test_not_reached(self):
        import io, contextlib, json
        for t in ("nope/x.nix", "./flake.nix-not-here"):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                rc = pot.why_json(dict(POLICY, workspace=[], access="write", backend={"proved": []}), t,
                                  lambda x: pot.answer_why(dict(POLICY, workspace=[], access="write", backend={"proved": []}), x, "/home/u", {}),
                                  "/home/u", {})
            j = json.loads(out.getvalue())
            self.assertEqual((rc, j["kind"], j["reached"]), (1, "file", False), t)
            self.assertIn("no such path here", j["text"], t)


class Agree(unittest.TestCase):
    """R3-1: pot why's text and its --json come from one classification: an existing bare name (flake.nix, a
    folder), a root kit command, a host and an ssh URL, on the host and in a run."""

    def test_text_and_json_agree(self):
        import io, contextlib, json, os, tempfile
        d = tempfile.mkdtemp()
        cwd = os.getcwd()
        try:
            os.chdir(d)
            open("flake.nix", "w").close()
            os.mkdir("nix")
            pol = dict(POLICY, workspace=[{"id": "repo:x", "path": os.path.realpath(d)}], access="write",
                       backend={"proved": []}, items=[dict(i, sources=[{"kit": "root:daniel", "grant": "g"}]) for i in POLICY["items"]])
            facts = {"commands": {"claude": ["claude-code@personal"]}}
            for on_host in (True, False):
                text = lambda x: pot.answer_why(pol, x, "/home/u", facts, on_host=on_host)
                for target, kind in [("flake.nix", "file"), ("nix", "file"), ("github.com", "network"),
                                     ("ssh://forgejo@platform-vm:2222/daniel/graft.git", "git.fetch")]:
                    out = io.StringIO()
                    with contextlib.redirect_stdout(out):
                        pot.why_json(pol, target, text, "/home/u", facts)
                    j = json.loads(out.getvalue())
                    self.assertEqual(j["kind"], kind, target)
                    self.assertEqual(j["text"], text(target), target)
                    self.assertNotIn("one run: pot --package", j["text"], target)
                    if kind == "file":
                        self.assertIn("is in the workspace", j["text"], target)
                # a root kit command: a command in the text, and not a --json target
                self.assertTrue(text("claude").startswith("claude is claude-code@personal's command"))
                with self.assertRaises(pot.Usage):
                    pot.why_json(pol, "claude", text, "/home/u", facts)
        finally:
            os.chdir(cwd)


if __name__ == "__main__":
    unittest.main()
