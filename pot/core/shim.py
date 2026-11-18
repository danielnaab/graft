"""pot's shims inside a run: `shim.py workmux ARGS` (status.relay) and `shim.py ssh ARGS`
(git.fetch's GIT_SSH_COMMAND). Each talks to the host broker over its socket in the run's folder (/run/pot);
the broker decides."""
import json
import os
import socket
import sys
import threading

mode, argv = sys.argv[1], sys.argv[2:]
sock = socket.socket(socket.AF_UNIX)
# the run's folder: /run/pot (bubblewrap), or the folder POT_POLICY names (Seatbelt, which has no mounts)
run = os.path.dirname(os.environ.get("POT_POLICY") or "/run/pot/policy.json")
path = os.path.join(run, {"workmux": "relay.sock", "ssh": "git.sock"}[mode])
try:
    sock.connect(path)
except OSError:
    print(f"pot ({os.environ.get('POT', 'run')}): {mode}: the host relay is not running", file=sys.stderr)
    sys.exit(0 if mode == "workmux" else 128)
f = sock.makefile("rb")
if mode == "workmux":
    sock.sendall(json.dumps(argv).encode() + b"\n")
    f.readline()
    sys.exit(0)
sock.sendall(json.dumps({"argv": argv, "protocol": os.environ.get("GIT_PROTOCOL", "")}).encode() + b"\n")
line = f.readline().decode(errors="replace").rstrip("\n")
if line != "OK":
    print(f"pot ({os.environ.get('POT', 'run')}): {line[4:] or 'the git broker refused'}", file=sys.stderr)
    sys.exit(128)


def up():
    while data := os.read(0, 65536):
        sock.sendall(data)
    sock.shutdown(socket.SHUT_WR)


threading.Thread(target=up, daemon=True).start()
while data := f.read1(65536):
    os.write(1, data)
