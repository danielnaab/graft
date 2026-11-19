"""v2.19.2: protect patterns' names and .git compare folded (NFD(casefold(NFD(name)))), as macOS's default filesystem
compares names: on APFS .Envrc is the .envrc host direnv opens, and flaKe.nix (a Kelvin sign) the flake.nix nix opens.
Before, an existing .Envrc was neither pinned read-only nor fingerprinted, and a run could create one with no warning.
Run with the other unit tests (pot-unit); on any system: folding is applied everywhere."""
import os
import shutil
import tempfile
import unittest

import bwrap as B
import compose as C

KELVIN, LONG_S, FL = "K", "ſ", "ﬂ"


class Fold(unittest.TestCase):
    def test_what_apfs_takes_for_the_same_name(self):
        for f in (B.fold, C.fold):
            for a, b in [(".Envrc", ".envrc"), ("FLAKE.NIX", "flake.nix"), (f"fla{KELVIN}e.nix", "flake.nix"),
                         (f".gitmodule{LONG_S}", ".gitmodules"), (f"{FL}ake.lock", "flake.lock"), (".GIT", ".git"),
                         ("é", "é")]:
                self.assertEqual(f(a), f(b), (a, b))
            self.assertNotEqual(f("agentś.md"), f("agents.md"))     # ś: another name, on APFS too


class Workspace(unittest.TestCase):
    def setUp(self):
        self.top = tempfile.mkdtemp()
        self.addCleanup(lambda: shutil.rmtree(self.top, ignore_errors=True))

    def make(self, *paths):
        # on a case-insensitive filesystem two spellings are one file: each test makes names that differ by more
        for p in paths:
            os.makedirs(os.path.dirname(os.path.join(self.top, p)), exist_ok=True)
            open(os.path.join(self.top, p), "w").close()

    def test_scan_finds_any_spelling(self):
        self.make(".Envrc", f"sub/FLA{KELVIN}E.NIX", "sub/.GIT/config", "sub/.GIT/.envrc", "Justfile", "other")
        os.makedirs(os.path.join(self.top, ".git"))
        p = B.Pins(self.top, os.path.join(self.top, ".git"), ["**/.envrc", "**/flake.nix", "**/justfile"], {})
        p.scan()
        rel = lambda l: sorted(x[len(self.top) + 1:] for x in l)    # noqa: E731
        self.assertEqual(rel(p.dotgits), [".git", "sub/.GIT"])
        self.assertEqual(rel(p.named), [".Envrc", "Justfile", f"sub/FLA{KELVIN}E.NIX"])    # never below a .git

    def test_hostrun_hashes_see_a_created_spelling(self):
        was = B.hostrun_hashes(self.top, ["**/.envrc"])
        self.make("a/.ENVRC")
        self.assertEqual(sorted(set(B.hostrun_hashes(self.top, ["**/.envrc"])) - set(was)), ["a/.ENVRC"])

    def test_sealed_dirs_any_spelling(self):
        os.makedirs(os.path.join(self.top, "x", ".PI"))
        self.make("y/.Pi")                                          # a file: never sealed
        self.assertEqual(B.sealed_dirs({}, self.top, ["**/.pi"]), [os.path.join(self.top, "x", ".PI")])


class Matches(unittest.TestCase):
    def test_folded(self):
        for path, pat in [("a/.Envrc", "**/.envrc"), (f"FLA{KELVIN}E.NIX", "**/flake.nix"),
                          (".GitHub/Workflows/x.yml", ".github/workflows"), ("Docs/X", "docs/x/")]:
            self.assertTrue(B.matches(path, [pat]), (path, pat))
            self.assertTrue(C.protected_by(path, pat), (path, pat))
        for path, pat in [("a/envrc", "**/.envrc"), (".githubx/workflows", ".github/workflows"), ("docsx", "docs")]:
            self.assertFalse(B.matches(path, [pat]), (path, pat))
            self.assertFalse(C.protected_by(path, pat), (path, pat))


class RunOrder(unittest.TestCase):
    def test_same_second_runs_list_in_start_order(self):
        import pot
        d = tempfile.mkdtemp()
        self.addCleanup(lambda: shutil.rmtree(d, ignore_errors=True))
        ents = []
        for n, ns in (("a", 2_000), ("b", 1_000), ("c", 3_000)):
            os.mkdir(os.path.join(d, n))
            open(os.path.join(d, n, "policy.json"), "w").close()
            os.utime(os.path.join(d, n, "policy.json"), ns=(ns, ns))
            ents.append(({"id": n, "started": "2026-11-19T12:31:34"}, [], os.path.join(d, n)))
        ents.append(({"id": "old", "started": "2026-11-19T12:31:33"}, [], os.path.join(d, "gone")))
        self.assertEqual([e[0]["id"] for e in sorted(ents, key=pot.by_start, reverse=True)], ["c", "a", "b", "old"])


if __name__ == "__main__":
    unittest.main()
