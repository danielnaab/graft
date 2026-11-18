"""Fix round 3: N2 (a record's clone.fetchFailed), N1 and D4 (next after a failed staging, and for a stopped
dispatch with its kept clone), D1 (a hint's one-run form names the run's context), G2 (visible() shows a byte
that is not UTF-8 as \\xNN). Run with the other unit tests (pot-unit)."""
import unittest

import bwrap
import compose as C
import pot
import runs as R

START = {"ev": "start", "by": "host", "run": "9b0e", "context": "dispatch", "mode": "clone", "backend": "bubblewrap",
         "argv": ["claude"], "kits": [], "invoker": "person", "time": "2026-11-17T14:02:11", "pid": 4242,
         "pidStart": 77, "boot": "b-1", "workspace": [{"id": "repo:h/x", "path": "/home/u/x", "name": "x"}],
         "command": {"name": "claude"}, "pane": None, "detached": False, "parent": None, "oneRun": [],
         "clone": {"base": "c9996c5aaaa", "branch": "main", "path": "/s/runs/9b0e/clone"}}
EXIT = {"ev": "exit", "by": "host", "status": 0, "time": "2026-11-17T14:31:40"}
FAILED = {"ev": "after", "by": "host", "commits": [], "unsigned": 0, "refs": {}, "base": "c9996c5aaaa",
          "fetch_failed": "listing its refs: error: no such object", "signs": False}


def dead(_start):
    return False


class FetchFailed(unittest.TestCase):
    def test_record_carries_it(self):
        r = R.record("9b0e", [START, EXIT, FAILED], dead)
        self.assertEqual(r["clone"]["fetchFailed"], "listing its refs: error: no such object")
        ok = dict(FAILED, commits=["e91d2c7"], refs={"main": "e91d2c7"})
        del ok["fetch_failed"]
        self.assertIsNone(R.record("9b0e", [START, EXIT, ok], dead)["clone"]["fetchFailed"])

    def test_next_without_fetch(self):
        r = R.record("9b0e", [START, EXIT, FAILED], dead)
        nxt = [l for l in pot.record_text(r, []) if l.startswith("next")]
        self.assertEqual(nxt, ["next       pot review 9b0e · pot review 9b0e --discard (the staging fetch failed: nothing to fetch)"])
        nxt = [l for l in pot.record_text(r, [], review=True) if l.startswith("next")]
        self.assertEqual(nxt, ["next       pot review 9b0e --discard (the staging fetch failed: nothing to fetch)"])


class StoppedDispatch(unittest.TestCase):
    def test_next_names_the_clone(self):
        stop = {"ev": "stop", "by": "host", "reason": "grant", "kits": [], "fix": "pot grant needs", "time": "2026-11-17T14:02:12"}
        r = R.record("9b0e", [START, stop], dead)
        r["clone"]["keptUntil"] = "2026-12-01"
        nxt = [l for l in pot.record_text(r, []) if l.startswith("next")]
        self.assertEqual(nxt, ["next       pot grant needs · its clone is kept until 2026-12-01: pot review 9b0e --discard"])
        r["stop"]["grantedSince"] = True
        nxt = [l for l in pot.record_text(r, []) if l.startswith("next")]
        self.assertEqual(nxt, ["next       start it again · its clone is kept until 2026-12-01: pot review 9b0e --discard"])


class Hints(unittest.TestCase):
    def test_denial_names_the_context(self):
        self.assertIn("  one run:   pot dispatch --net example.com …", C.denial_text("dispatch", "example.com:443", "FIX"))
        self.assertIn("  one run:   pot inspect --net example.com:80 …", C.denial_text("inspect", "example.com:80", "FIX"))
        self.assertIn("  one run:   pot --net example.com …", C.denial_text("work", "example.com:443", "FIX"))

    def test_not_found_names_the_context(self):
        self.assertIn("  one run:   pot dispatch --package PKG …", C.not_found_text("dispatch", "jq", "FIX"))
        self.assertIn("  one run:   pot dispatch --package PKG …", C.not_found_text("dispatch", "jq", "FIX", on_host=False))
        self.assertIn("  one run:   pot --package PKG …", C.not_found_text("work", "jq", "FIX"))


class Bytes(unittest.TestCase):
    def test_surrogate_escapes(self):
        s = b"bad\xffdir\r/justfile".decode("utf-8", "surrogateescape")
        self.assertEqual(bwrap.visible(s), "bad\\xffdir\\x0d/justfile")
        self.assertEqual(pot.visible(s), "bad\\xffdir\\x0d/justfile")


if __name__ == "__main__":
    unittest.main()
