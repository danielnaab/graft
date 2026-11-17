"""Golden output tests (v2.16.1): pot explain, pot grant --diff, pot why and the 127 line, for a root
shaped like a real one (three claude-code instances, a standing kit `daniel`, a repo kit), against the
texts in output/. A change of wording or layout shows here as a diff; never rewrite a golden file to
make the test pass without reading what changed (POT_GOLDEN_WRITE=1 writes them, for a reviewed change).

Run from `nix flake check` (pot-unit), or: cd pot/core && python3 -m unittest discover -s ../tests -t ../tests"""
import io
import contextlib
import os
import unittest

import compose as C
import pot

HOME = "/home/u"
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "output")


def claude(api="anthropic", cfg=".claude", mcp=True):
    host = "bedrock-runtime.us-east-2.amazonaws.com" if api == "bedrock" else "api.anthropic.com"
    net = [host] + (["claude.ai"] if api == "anthropic" else ["bedrock.us-east-2.amazonaws.com", "sts.us-east-2.amazonaws.com"])
    caps = [{"name": "status.relay"},
            {"name": "credential", "host": host,
             "env": ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN", "AWS_CREDENTIAL_EXPIRATION"]
             if api == "bedrock" else "CLAUDE_CODE_OAUTH_TOKEN"}] + ([{"name": "mcp", "server": "sessions"}] if mcp else [])
    return {"version": "1.1", "file": "the root (programs.pot.kits)",
            "tools": ["/nix/store/pqyiidz5mi73wv5f91d48s9plcqyj5bz-glibc-2.44-57"],
            "env": {"set": {"CLAUDE_CONFIG_DIR": "~/.claude", "DISABLE_AUTOUPDATER": "1",
                            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1"}},
            "files": {"~/.claude/.claude.json": {"source": "/nix/store/x-onboarding.json", "mode": "reassert", "keys": ["hasCompletedOnboarding"]},
                      "~/.claude/CLAUDE.md": {"source": "/nix/store/x-CLAUDE.md", "mode": "fixed"}},
            "protect": ["**/.claude", "**/.mcp.json"],
            "agent": {"markers": ["CLAUDECODE"], "select": {"env": {"CLAUDE_CONFIG_DIR": "~/" + cfg}},
                      "hooks": ["PreToolUse, PostToolUse, PostToolUseFailure (Bash|WebFetch); SessionStart",
                                "(pot's --settings overlay; /hooks does not list them)"]},
            "commands": {"claude": {"program": {"host": "~/.local/bin/claude"}, "args": ["--settings", "/nix/store/x-overlay.json"],
                                    "requests": {"read": ["~/.local/share/claude", "~/src/agent-skills", "~/src/agent-skills-private"],
                                                 "network": net, "env": ["CLAUDE_CODE_USE_BEDROCK", "ANTHROPIC_MODEL", "AWS_REGION"] if api == "bedrock" else [],
                                                 "state": [{"name": "claude", "at": "~/.claude"}], "capabilities": caps}}}}


ROOT = {"version": 1, "name": "home-manager", "grantsFile": "/state/grants.json", "stateDir": "/state/pot",
        "kits": {"claude-code@personal": claude(), "claude-code@flexion": claude(cfg=".claude-accounts/flexion"),
                 "claude-code@flexion-bedrock": claude("bedrock", ".claude-accounts/flexion"),
                 "daniel": {"requests": {"network": ["pypi.org", "files.pythonhosted.org", "registry.npmjs.org"],
                                         "capabilities": [{"name": "git.fetch", "remote": "ssh://forgejo@platform-vm:2222/*"}]}}},
        "defaultInstance": {"claude-code": "personal"}, "contexts": {"work": {"include": ["daniel"]}},
        "hidden": [], "standard": ["/nix/store/aaa-bash"] * 1, "standardNames": ["bash"], "tools": [], "rootRepos": [],
        "repoKits": {}, "bindings": {"status.relay": {"*": {"workmux": True}}}, "defaultContext": "work",
        "baseHidden": ["~/.claude", "~/.claude.json"], "baseProtect": ["**/.claude", "**/.pi", "**/.codex"],
        "baseAgentMarkers": ["CODEX_SANDBOX", "CLAUDECODE"], "agentMarkers": []}
REPO_KIT = {"file": "flake.nix (perSystem)", "protect": ["nix/agent-tools/ensure.sh", ".local", ".githooks", "AGENTS.md"],
            "requests": {"capabilities": [{"name": "nix"}]}}


def granted(*idents):
    g = C.parse_grants(None)
    for ident in idents:
        if ident.startswith("root:"):
            reqs = C.validate_kit(ROOT["kits"][ident[5:]], ident, HOME)
        else:
            reqs = C.validate_kit(REPO_KIT, ident, HOME)
        C.write_grant(g, ident, [r.canonical for r in reqs], {"tree": "t"} if ident.startswith("repo:") else None, "m1", "today", False)
    return g


def compose(instance="flexion-bedrock", grants=None):
    w = {"id": "repo:platform-vm:2222/daniel/flexion-assistant", "top": "/home/u/src/flexion-assistant",
         "path": "/home/u/src/flexion-assistant", "common": "/home/u/src/flexion-assistant/.git",
         "origin": None, "originKey": "platform-vm:2222/daniel/flexion-assistant",
         "repoKit": {"manifest": REPO_KIT, "file": "flake.nix (perSystem)"}, "rootRepoKits": [], "scions": [],
         "tree": "sha256-t", "source": "t", "flakeDir": "nix/agent-tools",
         "bundleProtect": ["**/graft.nix", "nix/agent-tools/.scions"]}
    inp = {"root": ROOT, "context": "work", "argv": ["claude"], "flags": {"instance": instance}, "workspace": [w],
           "grants": grants if grants is not None else granted("root:claude-code@flexion-bedrock", "root:daniel", w["id"]),
           "machine": "m1", "run": "abcd",
           "probe": ["own-netns", "own-pidns", "no-tiocsti", "protected-as-mounts", "in-place", "root-files"],
           "backendName": "bubblewrap", "stateDir": "/state/pot",
           "host": {"home": HOME, "user": "u", "env": {}, "hidden": {}, "tz": "America/Chicago",
                    "resolve": {"~/.local/bin/claude": "/home/u/.local/share/claude/versions/2.1.326"}}}
    policy, report = C.compose(inp)
    report["agentHooks"] = ROOT["kits"]["claude-code@" + instance]["agent"]["hooks"]
    src = [{"kit": "root:claude-code@" + instance, "file": "the root", "as": "see"}]
    for d, n, skipped in (("agents", 0, []), ("plugins", 7, []), ("skills", 15, ["address-pr-comments", "grill-me"])):
        policy["items"].append({"kind": "dir", "value": f"~/.claude/{d}", "class": "see", "effective": True, "mode": "fixed",
                                "sources": src, "links": [["x", "/nix/store/x"]] * n, **({"skipped": skipped} if skipped else {})})
    return policy, report


def golden(name, text):
    p = os.path.join(OUT, name)
    if os.environ.get("POT_GOLDEN_WRITE"):
        with open(p, "w") as f:
            f.write(text)
    with open(p) as f:
        return f.read()


class Explain(unittest.TestCase):
    def test_granted_instance(self):
        p, r = compose()
        t = C.explain_text(p, r, ROOT, width=100)
        self.assertEqual(t, golden("explain.txt", t))

    def test_not_granted_instance(self):
        p, r = compose("personal")
        t = C.explain_text(p, r, ROOT, width=100)
        self.assertEqual(t, golden("explain-personal.txt", t))
        self.assertIn("next       pot grant claude-code@personal (9 requests)\n", t)
        self.assertIn("left out until claude-code@personal is granted:", t)


class GrantDiff(unittest.TestCase):
    def test_lines(self):
        _, r = compose()
        sts = [k for k in r["kits"] if k["kind"] != "scion"]
        for utf8, name in ((True, "grant-diff.txt"), (False, "grant-diff-ascii.txt")):
            t = "\n".join(C.grant_diff_lines(sts, utf8=utf8)) + "\n"
            self.assertEqual(t, golden(name, t))
        t = "\n".join(C.grant_diff_lines(sts, utf8=False, verbose=True)) + "\n"
        self.assertEqual(t, golden("grant-diff-verbose.txt", t))


class Why(unittest.TestCase):
    def test_answers(self):
        p, r = compose()
        facts = pot.why_facts(p, r, ROOT)
        cwd = os.getcwd()
        os.chdir(HERE)
        try:
            lines = []
            for x in ("files.example.org", "pypi.org", "flake.nix", "nix/agent-tools/flake.nix", "/home/u/.ssh/id", "no-such-tool-x"):
                for on_host in (True, False):
                    lines.append(f"$ pot why {x}" + ("" if on_host else "   (in a run)"))
                    lines.append(pot.answer_why(p, x, HOME, facts, on_host=on_host))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                pot.why_json(p, "flake.nix", lambda x: pot.answer_why(p, x, HOME, facts, on_host=True), HOME, facts)
            lines.append("$ pot why flake.nix --json")
            lines.append(out.getvalue().rstrip("\n"))
        finally:
            os.chdir(cwd)
        t = "\n".join(lines) + "\n"
        self.assertEqual(t, golden("why.txt", t))

    def test_not_found(self):
        p, r = compose()
        fix = C.permanent_fix("--package", "", r, ROOT)
        t = C.not_found_text("work", "rg", fix) + "\n" + C.not_found_text("work", "tsc", fix, on_host=False) + "\n"
        self.assertEqual(t, golden("not-found.txt", t))


class Pieces(unittest.TestCase):
    def test_wrap(self):
        self.assertEqual(C.wrap_words("head", ["aaaa", "bbbb", "cccc"], 14, 5), ["head aaaa bbbb", "     cccc"])
        self.assertEqual(C.wrap_line("x " * 3, 100), ["x " * 3])

    def test_protect_dedupe(self):
        p, r = compose()
        t = C.explain_text(p, r, ROOT)
        self.assertEqual(t.count(".githooks"), 1)
        self.assertIn(".githooks (pot, repo)", t)

    def test_file_like(self):
        for x, k in (("flake.nix", "file-like"), ("notes.md", "file-like"), ("pypi.org", "host"), ("example.nix.dev", "host"),
                     ("a/missing", "path")):
            self.assertEqual(pot.classify(x), k, x)


if __name__ == "__main__":
    unittest.main()
