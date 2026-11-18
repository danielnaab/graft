"""v2.17.1: binding files are checked before the grant prompts and the clone's use. compose lists each
credential and mcp binding that a kit of the run reads from a file (granted or not yet), and
pot.binding_file_refusals names a missing or unreadable one with its fix. Run with the other unit tests
(pot-unit)."""
import copy
import os
import tempfile
import unittest
from unittest import mock

import compose as C
import pot
import test_output as T

SEL = "api.anthropic.com kit=claude-code@flexion"


def root_with(bindings):
    r = copy.deepcopy(T.ROOT)
    r["bindings"] = dict(r["bindings"], **bindings)
    return r


class Collected(unittest.TestCase):
    def report(self, instance, grants=None):
        r = root_with({"credential": {SEL: {"via": "file", "from": {"file": "~/.config/pot/claude-token-flexion"},
                                            "refresh": "claude setup-token"}}})
        with mock.patch.object(T, "ROOT", r):
            return T.compose(instance, grants)[1]

    def test_not_granted_kit_is_listed(self):
        # claude-code@flexion is not granted (the default grants hold @flexion-bedrock): its binding file is
        # still checked, so a missing token shows before the grant prompt
        rep = self.report("flexion")
        self.assertEqual([k["status"] for k in rep["kits"] if k["kit"] == "root:claude-code@flexion"], ["never"])
        self.assertEqual(rep["bindingFiles"], [{"capability": "credential", "selector": SEL, "kit": "root:claude-code@flexion",
                                                "file": "~/.config/pot/claude-token-flexion", "refresh": "claude setup-token"}])

    def test_granted_kit_is_listed(self):
        rep = self.report("flexion", T.granted("root:claude-code@flexion", "root:daniel",
                                               "repo:platform-vm:2222/daniel/flexion-assistant"))
        self.assertEqual([b["selector"] for b in rep["bindingFiles"]], [SEL])

    def test_other_instance_is_not(self):
        # @flexion-bedrock's credential is bound by env: nothing to check; @flexion's binding is not in this run
        self.assertEqual(self.report("flexion-bedrock")["bindingFiles"], [])


class Refusals(unittest.TestCase):
    def setUp(self):
        self.home = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.home, ".config/pot"))

    def entry(self, name, refresh=None, cap="credential", sel=SEL):
        return {"capability": cap, "selector": sel, "kit": "root:claude-code@flexion", "file": f"~/.config/pot/{name}",
                **({"refresh": refresh} if refresh else {})}

    def test_missing_names_refresh(self):
        got = pot.binding_file_refusals([self.entry("claude-token-flexion", "claude setup-token")], "work", self.home)
        self.assertEqual(got, ["pot (work): ~/.config/pot/claude-token-flexion is missing (the credential binding "
                               "api.anthropic.com kit=claude-code@flexion): claude setup-token, saved with mode 600, then start again"])

    def test_missing_without_refresh(self):
        got = pot.binding_file_refusals([self.entry("key.env", cap="mcp", sel="sessions")], "dispatch", self.home)
        self.assertEqual(got, ["pot (dispatch): ~/.config/pot/key.env is missing (the mcp binding sessions): "
                               "create it, with mode 600, then start again"])

    def test_present_is_fine(self):
        p = os.path.join(self.home, ".config/pot/tok")
        with open(p, "w") as f:
            f.write("x")
        os.chmod(p, 0o600)
        self.assertEqual(pot.binding_file_refusals([self.entry("tok")], "work", self.home), [])

    def test_a_folder_is_unreadable(self):
        os.makedirs(os.path.join(self.home, ".config/pot/dir"))
        got = pot.binding_file_refusals([self.entry("dir")], "work", self.home)
        self.assertEqual(got, ["pot (work): ~/.config/pot/dir is unreadable (the credential binding "
                               "api.anthropic.com kit=claude-code@flexion): create it, with mode 600, then start again"])

    @unittest.skipIf(os.geteuid() == 0, "root reads a mode-000 file")
    def test_mode_000_is_unreadable(self):
        p = os.path.join(self.home, ".config/pot/tok")
        with open(p, "w") as f:
            f.write("x")
        os.chmod(p, 0)
        got = pot.binding_file_refusals([self.entry("tok", "claude setup-token")], "work", self.home)
        self.assertEqual(got, ["pot (work): ~/.config/pot/tok is unreadable (the credential binding "
                               "api.anthropic.com kit=claude-code@flexion): claude setup-token, saved with mode 600, then start again"])

    def test_each_file_once(self):
        # two bindings reading the same file (two instances' tokens in one file): one line
        e = self.entry("claude-token-flexion")
        self.assertEqual(len(pot.binding_file_refusals([e, dict(e, selector="x")], "work", self.home)), 1)


if __name__ == "__main__":
    unittest.main()
