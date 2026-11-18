"""Fix round 2: R8 (visible() escapes Unicode Cf, and pot's and the backend's agree), F7 (a command's newline in a
table), F10 (the requires resource-limits hint), R9 (prune skips a run folder with no start; record_source's
temporary file; a malformed net event; .gitmodules paths with ..). Run with the other unit tests (pot-unit)."""
import json
import os
import tempfile
import unittest

import bwrap
import clones as CL
import compose as C
import pot

RLO = chr(0x202E)
LRI, PDI = chr(0x2066), chr(0x2069)
ZWSP, ZWJ, BOM = chr(0x200B), chr(0x200D), chr(0xFEFF)


class Visible(unittest.TestCase):
    CASES = ["plain", "é ü 中", "a" + RLO + "b", LRI + "x" + PDI, "zero" + ZWSP + "width", ZWJ,
             "\x1b[2J", "\x9b", "tab\tx", BOM + "x"]

    def test_cf_escaped(self):
        self.assertEqual(pot.visible("a" + RLO + "b"), "a\\u202eb")
        self.assertEqual(bwrap.visible("x" + LRI + "y"), "x\\u2066y")
        self.assertEqual(pot.visible(ZWSP), "\\u200b")
        self.assertEqual(pot.visible("é 中"), "é 中")      # letters stay

    def test_agree(self):
        # pot's and the backend's agree on everything but the newline, which pot keeps (its messages have lines)
        for c in self.CASES:
            self.assertEqual(pot.visible(c), bwrap.visible(c), repr(c))
        self.assertEqual(pot.visible("a\nb"), "a\nb")
        self.assertEqual(bwrap.visible("a\nb"), "a\\x0ab")
        self.assertEqual(pot.visible("a\nb", lines=False), "a\\x0ab")


class Tables(unittest.TestCase):
    def test_newline_in_a_command(self):
        rec = {"command": {"typed": ["bash", "-c", "echo a\necho b"]}, "argv": []}
        t = pot.cmd_text(rec, 200)
        self.assertNotIn("\n", t)
        self.assertIn("\\x0a", t)


class ResourceLimitsHint(unittest.TestCase):
    def test_hint(self):
        from test_compose import inp
        i = inp(probe=["closure-store", "own-netns"])
        i["root"]["contexts"] = {"work": {"requires": ["resource-limits"]}}
        with self.assertRaises(C.Refusal) as e:
            C.compose(i)
        self.assertIn("(resource-limits: a user systemd scope, for runs without a terminal: pot doctor)", e.exception.message)


class Prune(unittest.TestCase):
    def test_skips_a_folder_with_no_start(self):
        # R9: a clone being prepared has no start yet; prune never takes it for an old one
        with tempfile.TemporaryDirectory() as st:
            rd = os.path.join(st, "runs", "ab12")
            os.makedirs(os.path.join(rd, "clone"))
            CL.prune(st, {"keepDays": 0, "maxSize": "1"}, lambda *a: None, lambda s: False)
            self.assertTrue(os.path.isdir(os.path.join(rd, "clone")))


class Sources(unittest.TestCase):
    def test_record_source_tmp(self):
        # R9: no fixed temporary name: a stray <file>.tmp folder does not stop the record
        with tempfile.TemporaryDirectory() as st:
            class H:
                state = st
            f = pot.sources_file(H, "repo:x")
            os.makedirs(f + ".tmp")
            pot.record_source(H, "repo:x", "t1", "sha256-a")
            with open(f) as fh:
                self.assertEqual(json.load(fh), {"t1": "sha256-a"})


class NetEvents(unittest.TestCase):
    def test_malformed(self):
        ev = pot.net_events({"denials": [{"host": "(malformed)", "count": 2}, {"host": "x.org:443", "count": 1}]})
        self.assertEqual(ev[0], {"ev": "net", "by": "host", "host": None, "port": None, "verdict": "malformed", "count": 2})
        self.assertEqual(ev[1]["host"], "x.org")
        self.assertEqual(ev[1]["port"], 443)


class Gitmodules(unittest.TestCase):
    def test_dotdot(self):
        self.assertTrue(CL.safe_rel("scions/a"))
        for bad in ("../x", "a/../../b", "/abs", "a/./b/..", "", "."):
            self.assertFalse(CL.safe_rel(bad), bad)


if __name__ == "__main__":
    unittest.main()
