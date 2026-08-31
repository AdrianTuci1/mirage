#!/usr/bin/env python3
"""Drive a running Mirage daemon over its JSON-RPC socket.

Used to prove the product path end to end: ask for an indexing pass, watch the
progress the way the Settings window does, then search like Spotlight does.

    python3 scripts/daemon-probe.py /path/mirage.sock index
    python3 scripts/daemon-probe.py /path/mirage.sock search "a photo of a cat"
"""

import json
import socket
import sys
import time

_counter = [0]


def call(sock, method, params=None, timeout=30):
    _counter[0] += 1
    request = {"jsonrpc": "2.0", "id": _counter[0], "method": method}
    if params is not None:
        request["params"] = params
    payload = (json.dumps(request) + "\n").encode()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.settimeout(timeout)
        s.connect(sock)
        s.sendall(payload)
        data = b""
        while not data.endswith(b"\n"):
            chunk = s.recv(65536)
            if not chunk:
                break
            data += chunk
    response = json.loads(data.decode())
    if "error" in response:
        raise SystemExit(f"{method} failed: {response['error']}")
    return response["result"]


def main():
    sock = sys.argv[1]
    action = sys.argv[2] if len(sys.argv) > 2 else "status"

    print("ping:", call(sock, "ping"))
    status = call(sock, "status")
    print("status:", json.dumps(status, indent=2))

    if action == "index":
        print("index_files:", call(sock, "index_files"))
        last = None
        deadline = time.time() + 240
        while time.time() < deadline:
            progress = call(sock, "index_status")
            if progress != last:
                print("  progress:", json.dumps(progress))
                last = progress
            if not progress.get("running"):
                break
            time.sleep(0.5)
        print("final:", json.dumps(call(sock, "index_status")))
        print("status:", json.dumps(call(sock, "status")["index"]))

    elif action == "search":
        query = sys.argv[3]
        started = time.time()
        results = call(sock, "search", {"query": query, "top_k": 8})
        print(f'search {query!r} in {(time.time() - started) * 1000:.0f}ms')
        for item in results:
            print(f"  {item['category']:9} {item['score']:.3f}  {item['relativePath'] if 'relativePath' in item else item['relative_path']}")

    elif action == "settings":
        print("get_indexing_settings:", call(sock, "get_indexing_settings"))

    else:
        print("index_status:", call(sock, "index_status"))


if __name__ == "__main__":
    main()
