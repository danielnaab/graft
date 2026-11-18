"""runs/1: a run's record, computed from its log.jsonl alone (states, crash detection, review). Run with the
other unit tests (pot-unit)."""
import unittest

import runs as R

START = {"ev": "start", "by": "host", "run": "9b0e", "context": "work", "mode": "in-place", "backend": "bubblewrap",
         "argv": ["claude"], "kits": [], "invoker": "person", "time": "2026-11-17T14:02:11", "pid": 4242,
         "pidStart": 77, "boot": "b-1", "workspace": [{"id": "repo:h/x", "path": "/home/u/x"}],
         "command": {"name": "claude", "kit": "root:claude-code@personal", "instance": "personal"},
         "pane": None, "detached": False, "parent": None, "oneRun": [], "clone": None}
CLONE = dict(START, context="dispatch", mode="clone",
             clone={"base": "c9996c5aaaa", "branch": "main", "path": "/s/runs/9b0e/clone"})
EXIT = {"ev": "exit", "by": "host", "status": 0, "time": "2026-11-17T14:31:40"}
AFTER = {"ev": "after", "by": "host", "commits": ["e91d2c7"], "unsigned": 1, "touches_host_run": ["justfile"],
         "repo_kit_change": [], "root_repo_change": False, "new_git_metadata": [], "denials": [{"host": "x.org:443", "count": 2}],
         "base": "c9996c5aaaa", "refs": {"main": "e91d2c7"}, "size": 1024, "signs": True}


def alive(_start):
    return True


def dead(_start):
    return False


class States(unittest.TestCase):
    def test_running(self):
        self.assertEqual(R.record("9b0e", [START], alive)["state"], "running")

    def test_exited(self):
        r = R.record("9b0e", [START, EXIT, dict(AFTER, commits=[], refs={})], dead)
        self.assertEqual((r["state"], r["exit"], r["lost"]), ("exited", 0, False))

    def test_lost(self):
        # a start with no exit, and its process gone: pot was killed or crashed
        r = R.record("9b0e", [START], dead)
        self.assertEqual((r["state"], r["exit"], r["lost"]), ("exited", None, True))

    def test_stopped(self):
        stop = {"ev": "stop", "by": "host", "reason": "grant", "kits": [{"id": "repo:h/x", "status": "source"}],
                "fix": "pot grant x", "text": "…"}
        r = R.record("9b0e", [START, stop], dead)
        self.assertEqual(r["state"], "stopped")
        self.assertEqual(r["stop"], {"reason": "grant", "kits": [{"id": "repo:h/x", "status": "source"}], "fix": "pot grant x"})

    def test_review_then_fetched(self):
        r = R.record("9b0e", [CLONE, EXIT, AFTER], dead)
        self.assertEqual(r["state"], "review")
        self.assertEqual(r["clone"]["commits"], 1)
        self.assertEqual(r["clone"]["unsigned"], 1)
        fetch = {"ev": "fetch", "by": "host", "refs": ["refs/remotes/pot-9b0e/main"], "time": "2026-11-17T15:00:00"}
        r = R.record("9b0e", [CLONE, EXIT, AFTER, fetch], dead)
        self.assertEqual(r["state"], "exited")
        self.assertEqual(r["clone"]["fetched"], ["refs/remotes/pot-9b0e/main"])

    def test_discarded(self):
        d = {"ev": "discard", "by": "host", "why": "person", "time": "2026-11-17T15:00:00"}
        r = R.record("9b0e", [CLONE, EXIT, AFTER, d], dead)
        self.assertEqual((r["state"], r["clone"]["discarded"]), ("exited", "2026-11-17T15:00:00"))

    def test_scion_commits_need_review(self):
        a = dict(AFTER, commits=[], refs={}, scions={"scions/sbd": {"commits": ["abc"]}})
        r = R.record("9b0e", [CLONE, EXIT, a], dead)
        self.assertEqual((r["state"], r["clone"]["scionCommits"]), ("review", 1))

    def test_clone_without_commits(self):
        r = R.record("9b0e", [CLONE, EXIT, dict(AFTER, commits=[], refs={})], dead)
        self.assertEqual(r["state"], "exited")

    def test_running_wins(self):
        # a clone whose run still goes on is running, not review
        self.assertEqual(R.record("9b0e", [CLONE], alive)["state"], "running")

    def test_needs_staging(self):
        # pot died before after: the record says so, and pot runs completes the staging
        self.assertTrue(R.needs_after([CLONE], dead))
        self.assertFalse(R.needs_after([CLONE], alive))
        self.assertFalse(R.needs_after([CLONE, EXIT, AFTER], dead))
        self.assertFalse(R.needs_after([START], dead))

    def test_attention(self):
        # ATTENTION is the after event's findings; a commit count alone is not attention (lead's note 2)
        r = R.record("9b0e", [CLONE, EXIT, AFTER], dead)
        self.assertEqual(r["attention"], ["1 unsigned", "host-run: justfile", "2 denied: x.org:443"])
        plain = dict(AFTER, touches_host_run=[], denials=[], unsigned=0)
        self.assertEqual(R.record("9b0e", [CLONE, EXIT, plain], dead)["attention"], [])

    def test_unsigned_until_fetched(self):
        fetch = {"ev": "fetch", "by": "host", "refs": ["refs/remotes/pot-9b0e/main"], "time": "2026-11-17T15:00:00"}
        r = R.record("9b0e", [CLONE, EXIT, AFTER, fetch], dead)
        self.assertNotIn("1 unsigned", r["attention"])

    def test_lost_clone_with_commits_is_review(self):
        # pot was killed before its after; recovery logged the after: unfetched commits are review (ruling 4)
        r = R.record("9b0e", [CLONE, dict(AFTER, recovered=True)], dead)
        self.assertEqual((r["state"], r["lost"]), ("review", True))

    def test_discarded_lost_clone(self):
        d = {"ev": "discard", "by": "host", "why": "person", "time": "2026-11-17T15:00:00"}
        r = R.record("9b0e", [CLONE, dict(AFTER, recovered=True), d], dead)
        self.assertEqual((r["state"], r["clone"]["discarded"]), ("exited", "2026-11-17T15:00:00"))
        self.assertEqual(R.note(r), "discarded (pot ended without an exit: killed or crashed)")

    def test_notes(self):
        self.assertEqual(R.note(R.record("9b0e", [CLONE, EXIT, AFTER], dead)), "1 commit · pot review 9b0e")
        self.assertEqual(R.note(R.record("9b0e", [START], dead)), "lost: pot ended without an exit (killed or crashed)")
        self.assertEqual(R.note(R.record("9b0e", [START, EXIT], dead)), "exit 0")

    def test_workspace_name(self):
        # one rule: the name the start event records (the origin's last path part, else the folder); a log
        # without one shows its folder
        self.assertEqual(R.workspace_name({"workspace": [{"id": "repo:h/x", "path": "/s/fa", "name": "flexion-assistant"}]}),
                         "flexion-assistant")
        self.assertEqual(R.workspace_name({"workspace": [{"id": "repo:h/x", "path": "/s/fa"}]}), "fa")
        self.assertEqual(R.workspace_name({"workspace": []}), "-")

    def test_unsigned_only_when_the_host_signs(self):
        r = R.record("9b0e", [CLONE, EXIT, dict(AFTER, signs=False)], dead)
        self.assertNotIn("1 unsigned", r["attention"])

    def test_record_fields(self):
        r = R.record("9b0e", [CLONE, EXIT, AFTER], dead)
        self.assertEqual(sorted(r), sorted(["version", "id", "state", "context", "mode", "command", "argv", "workspace",
                                            "invoker", "detached", "pane", "parent", "started", "ended", "exit", "lost",
                                            "stop", "clone", "attention", "children"]))
        self.assertEqual(r["version"], 1)
        self.assertEqual(r["children"], [])

    def test_old_log(self):
        # a v2.16 log: start without the new keys, then exit and after
        old = {k: START[k] for k in ("ev", "by", "run", "context", "backend", "argv", "kits", "invoker", "time")}
        r = R.record("53ae", [old, EXIT, dict(AFTER, commits=[])], dead)
        self.assertEqual((r["state"], r["mode"], r["command"]), ("exited", "in-place", None))


class Since(unittest.TestCase):
    def test_forms(self):
        import datetime
        now = datetime.datetime(2026, 11, 17, 15, 0, 0)
        self.assertEqual(R.parse_time("2h", now), datetime.datetime(2026, 11, 17, 13, 0, 0))
        self.assertEqual(R.parse_time("3d", now), datetime.datetime(2026, 11, 14, 15, 0, 0))
        self.assertEqual(R.parse_time("30m", now), datetime.datetime(2026, 11, 17, 14, 30, 0))
        self.assertEqual(R.parse_time("2026-11-16", now), datetime.datetime(2026, 11, 16))
        self.assertEqual(R.parse_time("2026-11-16T09:30", now), datetime.datetime(2026, 11, 16, 9, 30))
        with self.assertRaises(ValueError):
            R.parse_time("yesterday", now)


if __name__ == "__main__":
    unittest.main()
