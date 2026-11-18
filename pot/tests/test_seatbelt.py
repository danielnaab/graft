"""The Seatbelt backend's pure parts (pot/core/seatbelt.py): its links never go through a planted link (P3),
where a ~/ entry lands, the run's folder in the command's arguments, and the profile's parameters. Runs on any
system: sandbox-exec itself is tested by tests/run.sh on macOS (pot_seatbelt)."""
import os
import tempfile
import unittest

import seatbelt as S


class Links(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.addCleanup(lambda: __import__("shutil").rmtree(self.d, ignore_errors=True))

    def test_link_is_made_and_remade(self):
        made = []
        S.link(self.d, "a/b", "/nix/store/x", made)
        self.assertEqual(os.readlink(os.path.join(self.d, "a/b")), "/nix/store/x")
        S.link(self.d, "a/b", "/nix/store/y", made)          # a link from an earlier run is replaced
        self.assertEqual(os.readlink(os.path.join(self.d, "a/b")), "/nix/store/y")

    def test_never_through_a_planted_link(self):
        outside = tempfile.mkdtemp()
        self.addCleanup(lambda: os.rmdir(outside))
        os.symlink(outside, os.path.join(self.d, "projects"))     # what an earlier run could leave in its state
        with self.assertRaisesRegex(S.Stop, "is a symbolic link in the run's state"):
            S.link(self.d, "projects/m/MEMORY.md", "/x", [])
        self.assertEqual(os.listdir(outside), [])

    def test_a_real_entry_is_not_removed(self):
        os.mkdir(os.path.join(self.d, "skills"))
        with self.assertRaisesRegex(S.Stop, "is a folder of the run's state"):
            S.link(self.d, "skills", "/x", [])


class Place(unittest.TestCase):
    POLICY = {"mounts": {"state": [{"at": "~/.claude", "dir": "/s/claude"}, {"at": "~/.claude/x", "dir": "/s/x"}]}}

    def test_in_a_state_directory(self):
        self.assertEqual(S.place(self.POLICY, "/home/u", "/r/home", "~/.claude/settings.json"), (S.real("/s/claude"), "settings.json"))

    def test_a_nested_state_is_in_the_outer_one(self):
        self.assertEqual(S.place(self.POLICY, "/home/u", "/r/home", "~/.claude/x"), (S.real("/s/claude"), "x"))

    def test_else_in_home(self):
        self.assertEqual(S.place(self.POLICY, "/home/u", "/r/home", "~/.gitconfig"), ("/r/home", ".gitconfig"))


class Argv(unittest.TestCase):
    def test_run_folder(self):
        pol = {"mounts": {"run": "/st/runs/ab12"}}
        self.assertEqual(S.run_argv(pol, {"argv": ["claude", "--mcp-config", "/run/pot/mcp.json", "/run/potx", "-x"]}),
                         ["claude", "--mcp-config", "/st/runs/ab12/mcp.json", "/run/potx", "-x"])


class Profile(unittest.TestCase):
    def test_paths_are_parameters(self):
        p = S.Profile()
        p.paths("allow file-read*", "subpath", ['/a "b") (allow default', "/c"])
        t = p.text()
        self.assertNotIn("allow default", t.split(S.BASE, 1)[1])     # a name never adds a rule
        self.assertEqual(p.params, ['-DP0=/a "b") (allow default', "-DP1=/c"])
        self.assertIn('(subpath (param "P0"))', t)

    def test_store_paths_are_checked(self):
        with self.assertRaisesRegex(S.Stop, "not a store path"):
            S.store_rules(S.Profile(), ['/nix/store/x") (allow default'])


if __name__ == "__main__":
    unittest.main()
