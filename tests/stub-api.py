# A stand-in for Forgejo's pulls API, for tests/run.sh: GET lists the PRs in argv[4] (none if
# that file is missing), POST "opens" one, PATCH "edits" one. Writes its port to argv[1], the
# method and Authorization header of each request to argv[2], and each request's method, path and
# JSON body to argv[3], one JSON object a line. argv[5], if given, is a JSON object of HTTP
# statuses to answer instead, by method: {"POST": 404} (a failure's body is {"message": "stub"}).
import http.server, json, os, sys

class H(http.server.BaseHTTPRequestHandler):
    def reply(self, code, body):
        with open(sys.argv[2], "a") as f:
            f.write(self.command + " " + (self.headers.get("Authorization") or "-") + "\n")
        raw = self.rfile.read(int(self.headers.get("Content-Length") or 0))
        with open(sys.argv[3], "a") as f:
            f.write(json.dumps({"method": self.command, "path": self.path,
                                "body": json.loads(raw) if raw else None}) + "\n")
        if len(sys.argv) > 5:
            code = json.loads(sys.argv[5]).get(self.command, code)
            if code >= 300: body = {"message": "stub"}
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self):
        listing = []
        if len(sys.argv) > 4 and os.path.exists(sys.argv[4]):
            with open(sys.argv[4]) as f: listing = json.load(f)
        self.reply(200, listing)
    def do_POST(self): self.reply(201, {"html_url": "http://stub/pulls/1"})
    def do_PATCH(self): self.reply(201, {"html_url": "http://stub/pulls/7"})
    def log_message(self, *a): pass

s = http.server.HTTPServer(("127.0.0.1", 0), H)
with open(sys.argv[1], "w") as f: f.write(str(s.server_port))
s.serve_forever()
