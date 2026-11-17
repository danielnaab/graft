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
                     ("HOME", "var"), ("pypi.org", "host"), ("h:22", "host"), ("bash", "command")]:
            self.assertEqual(pot.classify(t), k, t)

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
        self.assertEqual((rc, j["kind"], j["reach"], j["reached"]), (1, "file", "~/.ssh/id", False))
        self.assertIn("is hidden by your root", j["text"])


if __name__ == "__main__":
    unittest.main()
