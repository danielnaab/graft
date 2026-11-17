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
            ("file:///", ("file", "/", True)),
        ]:
            self.assertEqual(pot.reach_of(POLICY, t), want, t)

    def test_not_a_target(self):
        for t in ("bash", "HOME", "./x"):
            self.assertIsNone(pot.reach_of(POLICY, t))


if __name__ == "__main__":
    unittest.main()
