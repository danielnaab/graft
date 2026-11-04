# A stand-in for Forgejo's pulls API, for tests/run.sh: GET lists no PRs, POST "opens" one.
# Writes its port to argv[1] and the Authorization header of each request to argv[2].
import http.server, json, sys

class H(http.server.BaseHTTPRequestHandler):
    def reply(self, code, body):
        with open(sys.argv[2], "a") as f:
            f.write(self.command + " " + (self.headers.get("Authorization") or "-") + "\n")
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self): self.reply(200, [])
    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length") or 0))
        self.reply(201, {"html_url": "http://stub/pulls/1"})
    def log_message(self, *a): pass

s = http.server.HTTPServer(("127.0.0.1", 0), H)
with open(sys.argv[1], "w") as f: f.write(str(s.server_port))
s.serve_forever()
