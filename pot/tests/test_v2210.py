"""v2.21.0 (R0.5, attribution for host-level agents): `pot trailer`, git's prepare-commit-msg hook that writes
`Pot-Run: ID` from POT_RUN; pot host's after look (the commits a host run left on HEAD, with their signatures); the
run's gitconfig, which wires the hook; and runs/1's `commits` for a pot host run. Run with the other unit tests
(pot-unit); on any system."""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import types
import unittest

import bwrap
import pot
import runs as R

GIT = shutil.which("git")


def git_version():
    try:
        v = subprocess.run([GIT, "--version"], capture_output=True, text=True).stdout.split()[2]
        return tuple(int(x) for x in v.split(".")[:2])
    except Exception:
        return (0, 0)


CONFIG_HOOKS = GIT and git_version() >= (2, 54)     # git's config-based hooks (hook.<name>.event)
ENV = {"GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
       "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t"}


def git(cwd, *args):
    return subprocess.run([GIT, "-C", cwd, *args], check=True, capture_output=True, text=True,
                          env={**os.environ, **ENV}).stdout.strip()


class Env:
    """POT_RUN (and git's isolation) for one test, restored after."""
    def __init__(self, tc, **kv):
        self.saved = {k: os.environ.get(k) for k in list(kv) + list(ENV)}
        os.environ.update(ENV)
        for k, v in kv.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v
        tc.addCleanup(self.restore)

    def restore(self):
        for k, v in self.saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v


@unittest.skipUnless(GIT, "needs git")
class Trailer(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.addCleanup(lambda: shutil.rmtree(self.d, ignore_errors=True))
        self.msg = os.path.join(self.d, "COMMIT_EDITMSG")

    def write(self, text):
        with open(self.msg, "w") as f:
            f.write(text)

    def read(self):
        with open(self.msg) as f:
            return f.read()

    def test_adds_the_trailer_once(self):
        Env(self, POT_RUN="ab12")
        self.write("subject\n\nbody\n")
        self.assertEqual(pot.trailer([self.msg, "message"]), 0)
        self.assertEqual(pot.trailer([self.msg, "message"]), 0)
        self.assertEqual(self.read().count("Pot-Run: ab12"), 1)
        self.assertTrue(self.read().startswith("subject\n\nbody\n"))

    def test_keeps_an_existing_run(self):
        # an amend keeps the run that made the commit first (--if-exists doNothing)
        Env(self, POT_RUN="cd34")
        self.write("subject\n\nPot-Run: ab12\n")
        pot.trailer([self.msg, "commit", "HEAD"])
        self.assertNotIn("cd34", self.read())

    def test_merge_and_squash(self):
        Env(self, POT_RUN="ab12")
        for source in ("merge", "squash"):
            self.write("Merge branch 'x'\n")
            pot.trailer([self.msg, source])
            self.assertIn("Pot-Run: ab12", self.read(), source)

    def test_leaves_an_editor_message_alone(self):
        # no SOURCE, or a template: the person writes the message; a trailer would keep an emptied one from aborting
        Env(self, POT_RUN="ab12")
        for args in ([self.msg], [self.msg, "template"]):
            self.write("\n# Please enter the commit message\n")
            pot.trailer(args)
            self.assertNotIn("Pot-Run", self.read())

    def test_leaves_an_empty_message_alone(self):
        Env(self, POT_RUN="ab12")
        self.write("# only comments\n\n")
        pot.trailer([self.msg, "message"])
        self.assertNotIn("Pot-Run", self.read())

    def test_needs_a_run_id(self):
        for rid in (None, "", "ab1", "AB12", "ab12\nX: y", "../x"):
            Env(self, POT_RUN=rid)
            self.write("subject\n")
            self.assertEqual(pot.trailer([self.msg, "message"]), 0)
            self.assertEqual(self.read(), "subject\n", rid)

    def test_no_repository_configuration(self):
        # a repository's trailer.<key>.cmd never runs (interpret-trailers runs outside it, with no configuration)
        Env(self, POT_RUN="ab12")
        repo = os.path.join(self.d, "r")
        git(self.d, "init", "-q", repo)
        mark = os.path.join(self.d, "ran")
        git(repo, "config", "trailer.pot-run.cmd", f"touch {mark}; echo x")
        git(repo, "config", "trailer.pot-run.key", "Pot-Run")
        msg = os.path.join(repo, ".git", "COMMIT_EDITMSG")
        with open(msg, "w") as f:
            f.write("subject\n")
        here = os.getcwd()
        os.chdir(repo)
        self.addCleanup(os.chdir, here)
        pot.trailer([".git/COMMIT_EDITMSG", "message"])
        self.assertFalse(os.path.exists(mark))
        with open(msg) as f:
            self.assertIn("Pot-Run: ab12", f.read())

    def test_never_fails(self):
        Env(self, POT_RUN="ab12")
        self.assertEqual(pot.trailer([os.path.join(self.d, "missing"), "message"]), 0)
        self.assertEqual(pot.trailer([]), 0)

    @unittest.skipUnless(CONFIG_HOOKS, "needs git 2.54 or later (config-based hooks)")
    def test_as_git_runs_it(self):
        # git's config-based hook appends its arguments: MSGFILE SOURCE [SHA]
        Env(self, POT_RUN="ab12")
        repo = os.path.join(self.d, "r")
        git(self.d, "init", "-q", repo)
        hook = os.path.join(self.d, "hook")
        core = os.path.dirname(os.path.abspath(pot.__file__))
        with open(hook, "w") as f:
            f.write(f"#!{sys.executable} -I\nimport sys\nsys.path.insert(0, {core!r})\nimport pot\n"
                    "sys.exit(pot.trailer(sys.argv[1:]))\n")
        os.chmod(hook, 0o755)
        git(repo, "-c", "hook.pt.event=prepare-commit-msg", "-c", f"hook.pt.command={hook}", "-c", "commit.gpgsign=false",
            "commit", "-q", "--allow-empty", "-m", "one")
        self.assertEqual(git(repo, "log", "-1", "--format=%(trailers:key=Pot-Run,valueonly)"), "ab12")


@unittest.skipUnless(GIT, "needs git")
class HostLook(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.addCleanup(lambda: shutil.rmtree(self.d, ignore_errors=True))
        Env(self)
        self.repo = os.path.join(self.d, "r")
        git(self.d, "init", "-q", "-b", "main", self.repo)
        git(self.repo, "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", "base")
        self.host = types.SimpleNamespace(git=GIT)

    def commit(self, msg):
        git(self.repo, "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", msg)
        return git(self.repo, "rev-parse", "HEAD")

    def test_outside_a_repository(self):
        self.assertIsNone(pot.host_look_before(self.host, self.d))

    def test_no_commits(self):
        b = pot.host_look_before(self.host, self.repo)
        ev = pot.host_look_after(self.host, b)
        self.assertEqual(ev["commits"], [])
        self.assertEqual(pot.host_after_lines("ab12", ev), [])

    def test_the_commits_on_head(self):
        b = pot.host_look_before(self.host, os.path.join(self.repo))
        c1 = self.commit("one")
        c2 = self.commit("two")
        ev = pot.host_look_after(self.host, b)
        self.assertEqual(ev["ev"], "after")
        self.assertEqual(ev["commits"], [c2[:12], c1[:12]])
        self.assertEqual(ev["unsigned"], 2)
        self.assertEqual([s["status"] for s in ev["signatures"]], ["N", "N"])
        self.assertEqual(ev["signatures"][0]["subject"], "two")
        self.assertEqual(ev["base"], b["head"][:12])
        lines = pot.host_after_lines("ab12", ev)
        self.assertEqual(lines[0], "pot: host run ab12 · 2 commits on HEAD")
        self.assertIn("two", lines[1])

    def test_not_another_branch_or_what_was_there(self):
        # a branch that existed at the start, merged in, is not the run's; a branch left off HEAD is not looked at
        git(self.repo, "branch", "old")
        git(self.repo, "switch", "-q", "old")
        old = self.commit("old work")
        git(self.repo, "switch", "-q", "main")
        b = pot.host_look_before(self.host, self.repo)
        git(self.repo, "switch", "-q", "-c", "side")
        self.commit("side")
        git(self.repo, "switch", "-q", "main")
        git(self.repo, "-c", "commit.gpgsign=false", "merge", "-q", "--no-ff", "-m", "merge old", "old")
        mine = git(self.repo, "rev-parse", "HEAD")
        ev = pot.host_look_after(self.host, b)
        self.assertEqual(ev["commits"], [mine[:12]])
        self.assertNotIn(old[:12], ev["commits"])

    def test_a_cap(self):
        b = pot.host_look_before(self.host, self.repo)
        old = pot.HOST_LOOK_MAX
        pot.HOST_LOOK_MAX = 2
        self.addCleanup(lambda: setattr(pot, "HOST_LOOK_MAX", old))
        for i in range(3):
            self.commit(str(i))
        ev = pot.host_look_after(self.host, b)
        self.assertEqual(len(ev["commits"]), 2)
        self.assertEqual(ev["more"], 1)
        self.assertEqual(pot.host_after_lines("ab12", ev)[0], "pot: host run ab12 · 3 commits on HEAD")

    def test_a_subject_is_escaped(self):
        b = pot.host_look_before(self.host, self.repo)
        self.commit("evil \x1b[31m red")
        ev = pot.host_look_after(self.host, b)
        self.assertNotIn("\x1b", json.dumps(ev["signatures"][0]["subject"], ensure_ascii=False))


class GitConfig(unittest.TestCase):
    def test_without_a_pot_no_hook(self):
        t = bwrap.gitconfig_text({"git": {"name": "D", "email": "d@x"}})
        self.assertIn("gpgsign = false", t)
        self.assertNotIn("pot-trailer", t)

    def test_a_pot_outside_the_store_no_hook(self):
        d = tempfile.mkdtemp()
        self.addCleanup(lambda: shutil.rmtree(d, ignore_errors=True))
        p = os.path.join(d, "pot")
        open(p, "w").close()
        os.chmod(p, 0o755)
        self.assertNotIn("pot-trailer", bwrap.gitconfig_text({"potBin": d}))

    def test_the_hook(self):
        real = os.access
        bwrap.os.access = lambda p, m: True if p.startswith("/nix/store/") else real(p, m)
        self.addCleanup(lambda: setattr(bwrap.os, "access", real))
        t = bwrap.gitconfig_text({"potBin": "/nix/store/x-pot/bin"})
        self.assertIn('[hook "pot-trailer"]\n\tevent = prepare-commit-msg\n\tcommand = "/nix/store/x-pot/bin/pot trailer"\n', t)


class Record(unittest.TestCase):
    def test_a_host_runs_commits(self):
        sig = {"commit": "a1b2c3d4e5f6", "status": "G", "signer": "agent@h", "key": "SHA256:x", "subject": "s"}
        ev = [{"ev": "start", "run": "ab12", "mode": "none", "context": "host", "argv": ["claude"]},
              {"ev": "exit", "status": 0},
              {"ev": "after", "by": "host", "commits": ["a1b2c3d4e5f6"], "unsigned": 0, "signs": True, "signatures": [sig]}]
        rec = R.record("ab12", ev, lambda s: False)
        self.assertEqual(rec["mode"], "none")
        self.assertEqual(rec["commits"], [sig])
        self.assertIsNone(rec["clone"])

    def test_others_have_none(self):
        ev = [{"ev": "start", "run": "ab12", "mode": "in-place"}, {"ev": "exit", "status": 0},
              {"ev": "after", "commits": ["a1b2c3d4e5f6"], "unsigned": 0}]
        self.assertNotIn("commits", R.record("ab12", ev, lambda s: False))


if __name__ == "__main__":
    unittest.main()
