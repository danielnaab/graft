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


class FixRound4(unittest.TestCase):
    def test_git_profile_denies(self):
        # F1: the clone's git gets the run profile's last denies
        d = tempfile.mkdtemp()
        self.addCleanup(lambda: __import__("shutil").rmtree(d, ignore_errors=True))
        clos = os.path.join(d, "closure")
        open(clos, "w").write("/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-git\n")
        os.makedirs(os.path.join(d, "clone"))
        argv = S.sandboxed_git({"gitClosure": clos, "env": "/usr/bin/env", "git": "/nix/store/x-git/bin/git",
                                "python": "/nix/store/x-py/bin/python3"}, d, os.path.join(d, "clone"), ["status"])
        prof = open(os.path.join(d, "git.sb")).read()
        self.assertIn("(deny system-fcntl (fcntl-command 80 110))", prof)
        self.assertRegex(prof, r"\(deny file-ioctl \(ioctl-command [0-9]+\)\)")
        self.assertLess(prof.index("(allow file-read* (subpath (param \"CLONE\")))"), prof.index("(deny file-ioctl"))
        # a new session; stdin from /dev/null, except for upload-pack, which speaks on it
        self.assertEqual(argv[:3], ["/nix/store/x-py/bin/python3", "-I", "-S"])
        self.assertIn("os.setsid()", argv[4])
        self.assertEqual(argv[5], "1")
        up = S.sandboxed_git({"gitClosure": clos, "env": "/usr/bin/env", "git": "/g", "python": "/p"}, d,
                             os.path.join(d, "clone"), ["upload-pack", "--strict", "/clone/.git"])
        self.assertEqual(up[5], "0")

    def test_leftovers(self):
        # F2/F3: Seatbelt says which processes are the run's; TMPDIR only from the environment, never argv
        rd, tmp = "/st/runs/ab12", "/st/runs/ab12/tmp"
        plain = {10: "sleep 600", 11: "sleep 600", 12: f"grep TMPDIR={tmp}/ x", 13: "vim", 99: "pot"}
        withenv = {10: "sleep 600", 11: f"sleep 600 TMPDIR={tmp}/ HOME=/x", 12: f"grep TMPDIR={tmp}/ x PATH=/bin",
                   13: "vim HOME=/u", 99: f"pot TMPDIR={tmp}/"}
        ps = lambda env: withenv if env else plain    # noqa: E731

        def check(pid, op=None, path=None):
            sandboxed = {10: 1, 11: 0, 12: 0, 13: 1, 99: 1}[pid]
            if op is None:
                return sandboxed
            self.assertEqual(op, "file-read-data")
            self.assertIn(path, (rd + "/policy.json", rd + "/launch.json"))
            if path.endswith("launch.json"):
                return 1                              # no run's profile reads launch.json
            return 0 if pid in (10, 99) else 1        # 13: another run's (or another sandbox's) profile
        self.assertEqual(S.leftovers(rd, tmp, check, ps, me=99), [10, 11])
        self.assertEqual(S.leftovers(rd, tmp, None, ps, me=99), [11])      # no sandbox_check: the fallback alone

    def test_leftovers_v2202_a_broad_profile_is_not_the_run(self):
        # v2.20.2: a stock macOS profile that may read the user's folders reads policy.json AND launch.json (neagent,
        # rapportd, secd …); v2.20.0-v2.20.1 counted it as the run's and signalled it at every run's end
        rd, tmp = "/st/runs/ab12", "/st/runs/ab12/tmp"
        plain = {10: "sleep 600", 20: "neagent", 99: "pot"}
        ps = lambda env: plain    # noqa: E731

        def check(pid, op=None, path=None):
            if op is None:
                return 1                              # all sandboxed
            if pid == 20:
                return 0                              # broad: reads both
            return 1 if path.endswith("launch.json") else 0
        self.assertEqual(S.leftovers(rd, tmp, check, ps, me=99), [10])

    def test_leftovers_v2202_never_older_than_the_run(self):
        # v2.20.2: with since (the run's start), a process that started before it is never the run's, whatever matched
        rd, tmp = "/st/runs/ab12", "/st/runs/ab12/tmp"
        plain = {10: "sleep 600", 11: "sleep 600", 12: "sleep 600", 99: "pot"}
        withenv = {10: "sleep 600", 11: f"sleep 600 TMPDIR={tmp}/", 12: "sleep 600", 99: "pot"}
        ps = lambda env: withenv if env else plain    # noqa: E731

        def check(pid, op=None, path=None):
            if op is None:
                return 1 if pid in (10, 12) else 0
            return 1 if path.endswith("launch.json") else 0
        starts = lambda: {10: 1000.0, 11: 999.0, 12: 1000.0}    # noqa: E731  (12: the run's first second)
        self.assertEqual(S.leftovers(rd, tmp, check, ps, me=99), [10, 11, 12])
        self.assertEqual(S.leftovers(rd, tmp, check, ps, me=99, since=1000.4, starts=starts), [10, 12])
        self.assertEqual(S.leftovers(rd, tmp, check, ps, me=99, since=1001.0, starts=starts), [])
        # a process with no start (gone between the two ps) is not signalled
        self.assertEqual(S.leftovers(rd, tmp, check, ps, me=99, since=0, starts=lambda: {10: 5.0}), [10])

    @unittest.skipUnless(os.uname().sysname == "Darwin", "macOS's /bin/ps")
    def test_ps_starts_v2202(self):
        import time
        try:
            st = S.ps_starts()
        except OSError:
            self.skipTest("no /bin/ps to run (nix's build sandbox)")
        self.assertLessEqual(st[os.getpid()], time.time())
        self.assertGreater(st[os.getpid()], time.time() - 3600)

    @unittest.skipUnless(os.uname().sysname == "Darwin", "sandbox_check(2) is macOS's")
    def test_sandbox_check_answers_for_a_path(self):
        # v2.19.2: the real call, which test_leftovers stubs. Its path is a variadic argument; passed as a fixed one
        # (on arm64, in a register) every path check came back -1 (EINVAL), and F2 found nothing
        # (sandboxed or not: nix's builder on macOS is itself under Seatbelt)
        check = S.sandbox_checker()
        self.assertIn(check(os.getpid()), (0, 1))
        self.assertEqual(check(os.getpid(), "file-read-data", os.path.realpath(__file__)), 0)   # it reads itself

    def test_link_folders_are_kept(self):
        # F4: the folders made on the way to a link are kept with it
        d = tempfile.mkdtemp()
        self.addCleanup(lambda: __import__("shutil").rmtree(d, ignore_errors=True))
        os.makedirs(os.path.join(d, "a"))
        made = []
        S.link(d, "a/b/c/x", "/nix/store/y", made)
        self.assertEqual(made, [os.path.join(d, "a/b"), os.path.join(d, "a/b/c"), os.path.join(d, "a/b/c/x")])


if __name__ == "__main__":
    unittest.main()
