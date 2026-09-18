#!/usr/bin/env python3
"""Loopback-only static server for EDUS terminal test package."""
from __future__ import annotations
import argparse
import http.server
import json
import os
import socket
import sys
import urllib.error
import urllib.request
from functools import partial

PRODUCT = "edus-library-terminal-test"
HEALTH = "/__edus_terminal_test_health__"

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, version: str, instance: str | None = None, **kwargs):
        self.version = version
        self.instance = instance
        super().__init__(*args, **kwargs)
    def do_GET(self):
        if self.path.split("?", 1)[0] == HEALTH:
            body = json.dumps({"product": PRODUCT, "version": self.version, "instance": self.instance}).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        super().do_GET()
    def log_message(self, fmt, *args):
        sys.stdout.write("%s - %s\n" % (self.log_date_time_string(), fmt % args))

def check(port: int, version: str, instance: str | None = None) -> int:
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}{HEALTH}", timeout=1.5) as response:
            data = json.loads(response.read().decode("utf-8"))
            expected = data.get("product") == PRODUCT and data.get("version") == version
            if instance is not None:
                expected = expected and data.get("instance") == instance
            return 0 if expected else 11
    except (urllib.error.URLError, TimeoutError, ValueError, OSError):
        return 10

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--instance")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        return check(args.port, args.version, args.instance)
    root = os.path.abspath(args.root or "")
    if not os.path.isdir(root):
        print(f"Static app directory is missing: {root}", file=sys.stderr)
        return 2
    try:
        handler = partial(Handler, directory=root, version=args.version, instance=args.instance)
        with http.server.ThreadingHTTPServer(("127.0.0.1", args.port), handler) as server:
            print(f"{PRODUCT} {args.version} on http://127.0.0.1:{args.port}", flush=True)
            server.serve_forever()
    except OSError as error:
        print(f"Cannot use 127.0.0.1:{args.port}: {error}", file=sys.stderr)
        return 3
if __name__ == "__main__":
    raise SystemExit(main())
