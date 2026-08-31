#!/usr/bin/env python3
"""Fetch a few labelled photographs from Wikimedia Commons for retrieval tests.

Each subject is looked up through the Commons search API so the file names do not
have to be guessed, then downloaded as a 640px thumbnail named <label>.jpg.
"""

import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

API = "https://commons.wikimedia.org/w/api.php"
UA = {"User-Agent": "MirageIndexTests/0.1 (local retrieval test)"}

SUBJECTS = [
    ("cat", "cat sitting"),
    ("beach", "sandy beach sea"),
    ("city", "city skyline night"),
    ("snow", "snow covered mountain"),
    ("car", "red car"),
    ("bridge", "golden gate bridge"),
]


def get(url, attempts=5):
    delay = 5
    for attempt in range(attempts):
        try:
            req = urllib.request.Request(url, headers=UA)
            with urllib.request.urlopen(req, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code not in (429, 503) or attempt == attempts - 1:
                raise
            print(f"  {error.code} from commons, waiting {delay}s", file=sys.stderr)
            time.sleep(delay)
            delay *= 2
    raise RuntimeError("unreachable")


def find_file(query):
    params = {
        "action": "query",
        "list": "search",
        "srsearch": query,
        "srnamespace": "6",
        "srlimit": "8",
        "format": "json",
    }
    data = json.loads(get(API + "?" + urllib.parse.urlencode(params)))
    titles = [hit["title"] for hit in data["query"]["search"]]
    if not titles:
        return None
    params = {
        "action": "query",
        "titles": "|".join(titles),
        "prop": "imageinfo",
        "iiprop": "url|mime",
        "iiurlwidth": "640",
        "format": "json",
    }
    data = json.loads(get(API + "?" + urllib.parse.urlencode(params)))
    for page in data["query"]["pages"].values():
        info = page.get("imageinfo", [{}])[0]
        mime = info.get("mime", "")
        if mime not in ("image/jpeg", "image/png"):
            continue
        return page["title"], info.get("thumburl") or info.get("url")
    return None


def main(dest):
    os.makedirs(dest, exist_ok=True)
    failures = []
    for label, query in SUBJECTS:
        time.sleep(2)  # the Commons API rate-limits bursts
        found = find_file(query)
        if not found:
            failures.append(f"{label}: no Commons result for {query!r}")
            continue
        title, url = found
        path = os.path.join(dest, f"{label}.jpg")
        if not os.path.exists(path) or os.path.getsize(path) < 1024:
            with open(path, "wb") as handle:
                handle.write(get(url))
        size = os.path.getsize(path)
        print(f"{label}\t{size}\t{title}\t{url}")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "test-images"))
