"""The golden grant test (check pot-golden): the canonical grant items, and their digests, of the
kits in KITS.json (golden.nix's root and graft's default root, as pot reads their manifests) must
equal FIXTURE.json exactly. A difference re-asks a grant on every root that holds the kit.

  python3 -I golden.py POT_SRC KITS.json FIXTURE.json      check (exit 1 on any difference)
  python3 -I golden.py POT_SRC KITS.json --print           print what the kits give now
"""
import json
import sys

HOME = "/home/u"     # every golden path is ~/…; an absolute path under HOME would be recorded as ~/…


def items(compose, kits):
    out = {}
    for root, r in sorted(kits.items()):
        found = {f"root:{n}": m for n, m in (r.get("kits") or {}).items()}
        found.update({f"repo:{p}": m for p, m in (r.get("repoKits") or {}).items()})
        out[root] = {}
        for ident, m in sorted(found.items()):
            canons = sorted({x.canonical for x in compose.validate_kit(m, ident, HOME)})
            out[root][ident] = {"digest": compose.digest(canons), "requests": canons}
    return out


def main(argv):
    sys.path.insert(0, argv[1])
    import compose
    with open(argv[2]) as f:
        now = items(compose, json.load(f))
    if argv[3] == "--print":
        print(json.dumps(now, indent=2, sort_keys=True))
        return 0
    with open(argv[3]) as f:
        want = json.load(f)
    bad = 0
    for root in sorted(set(want) | set(now)):
        w, n = want.get(root, {}), now.get(root, {})
        for ident in sorted(set(w) | set(n)):
            a, b = w.get(ident), n.get(ident)
            if a == b:
                print(f"ok    {root} {ident} {b['digest']}")
                continue
            bad += 1
            if a is None or b is None:
                print(f"FAIL  {root} {ident}: {'new, not in the fixture' if a is None else 'gone'}")
                continue
            print(f"FAIL  {root} {ident}: {a['digest']} -> {b['digest']}")
            for c in sorted(set(a["requests"]) - set(b["requests"])):
                print(f"        - {c}")
            for c in sorted(set(b["requests"]) - set(a["requests"])):
                print(f"        + {c}")
    if bad:
        print(f"pot-golden: {bad} kit(s) changed their canonical grant items: every grant of them would be asked again")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
