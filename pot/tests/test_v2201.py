"""v2.20.1: a host pot killed outside a sibling (SIGTERM, say) skips its backend's cleanup, and v2.20.0's next start
left two things behind: <run dir>/children.sock (tidy_lost looked only for the other sockets, and under Seatbelt, whose
proxy is TCP, a run with no git binding has no other) and <state>/trees/<id> (the tree's sibling slots). The next
start now cleans up such a run on both backends, once its backend is gone too. Run with the other unit tests
(pot-unit); on any system."""
import json
import os
import shutil
import socket
import stat
import sys
import tempfile
import textwrap
import types
import unittest

import bwrap
import pot
import runs as R

CORE = os.path.dirname(os.path.abspath(bwrap.__file__))


class TidyLost(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        self.addCleanup(lambda: shutil.rmtree(self.d, ignore_errors=True))
        self.state = os.path.join(self.d, "state")
        os.makedirs(os.path.join(self.state, "runs"))
        self.socks = []
        self.addCleanup(lambda: [s.close() for s in self.socks])

    def host(self, backend):
        # a backend program whose cleanup verb is that backend's own (seatbelt's calls bwrap's, then its own)
        prog = os.path.join(self.d, f"backend-{backend}")
        with open(prog, "w") as f:
            f.write(f"#!{sys.executable} -I\n" + textwrap.dedent(f"""\
                import sys
                sys.path.insert(0, {CORE!r})
                import {backend}
                assert sys.argv[1] == "cleanup"
                {backend}.cleanup(sys.argv[2])
                """))
        os.chmod(prog, 0o755)
        return types.SimpleNamespace(state=self.state, root={"backends": {backend: {"program": prog}}})

    def run_dir(self, rid, alive=False, listening=False, status=False, sibling_of=None, mode="in-place"):
        rd = os.path.join(self.state, "runs", rid)
        os.makedirs(rd)
        me = os.getpid()
        start = {"ev": "start", "mode": mode, "backend": None, "pid": me,
                 "pidStart": R.proc_start(me) if alive else -1}      # another start time: that process is gone
        with open(os.path.join(rd, "log.jsonl"), "w") as f:
            f.write(json.dumps(start) + "\n")
        with open(os.path.join(rd, "launch.json"), "w") as f:
            json.dump({"siblingOf": sibling_of} if sibling_of else {}, f)
        here = os.getcwd()
        os.chdir(rd)
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.bind("children.sock")
            if listening:
                s.listen(1)
                self.socks.append(s)
            else:
                s.close()        # the file stays, with no one listening: the broker died with the backend
        finally:
            os.chdir(here)
        if status:
            with open(os.path.join(rd, "status"), "w") as f:
                f.write("143")
        tree = os.path.join(self.state, "trees", rid)
        os.makedirs(tree)
        open(os.path.join(tree, "slot-0"), "w").close()
        return rd, tree

    def gone(self, rd, tree):
        return not os.path.lexists(os.path.join(rd, "children.sock")) and not os.path.exists(tree)

    def test_lost_run_with_only_children_sock(self):
        for backend in ("bwrap", "seatbelt"):
            with self.subTest(backend=backend):
                rd, tree = self.run_dir("a1b2" if backend == "bwrap" else "c3d4")
                self.assertTrue(stat.S_ISSOCK(os.lstat(os.path.join(rd, "children.sock")).st_mode))
                pot.tidy_lost(self.host(backend))
                self.assertTrue(self.gone(rd, tree))
                self.assertTrue(os.path.exists(os.path.join(rd, "log.jsonl")))     # the record stays

    def test_backend_wrote_status(self):
        rd, tree = self.run_dir("e5f6", status=True)
        pot.tidy_lost(self.host("bwrap"))
        self.assertTrue(self.gone(rd, tree))

    def test_pot_alive_is_kept(self):
        rd, tree = self.run_dir("0a0b", alive=True)
        pot.tidy_lost(self.host("bwrap"))
        self.assertFalse(self.gone(rd, tree))

    def test_backend_still_running_is_kept(self):
        # the pot was killed alone; its backend runs on (its broker still listens) until the command ends
        rd, tree = self.run_dir("1c1d", listening=True)
        pot.tidy_lost(self.host("bwrap"))
        self.assertTrue(os.path.lexists(os.path.join(rd, "children.sock")))
        self.assertTrue(os.path.exists(tree))

    def test_clone_run_awaiting_recovery_is_kept(self):
        rd, tree = self.run_dir("2e2f", mode="clone")
        pot.tidy_lost(self.host("bwrap"))
        self.assertFalse(self.gone(rd, tree))        # load_record's recovery: the after event, then cleanup

    def test_lost_sibling_keeps_the_top_runs_tree(self):
        rd, tree = self.run_dir("3a3b", sibling_of="4c4d")
        pot.tidy_lost(self.host("bwrap"))
        self.assertFalse(os.path.lexists(os.path.join(rd, "children.sock")))
        self.assertTrue(os.path.exists(tree))        # only a top run's cleanup removes its tree

    def test_helpers_alive_with_a_long_path(self):
        # sun_path holds about 104 bytes: helpers_alive connects from inside the folder
        deep = os.path.join(self.state, "runs", "x" * 60, "y" * 60)
        os.makedirs(deep)
        here = os.getcwd()
        os.chdir(deep)
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.bind("children.sock")
            s.listen(1)
            self.socks.append(s)
        finally:
            os.chdir(here)
        self.assertTrue(pot.helpers_alive(deep))
        self.assertEqual(os.getcwd(), here)


if __name__ == "__main__":
    unittest.main()
