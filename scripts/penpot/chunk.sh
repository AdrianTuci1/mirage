#!/bin/sh
# Print chunks of spotlight-build.js, ready to paste into penpot__execute_code.
# Chunk names: bootstrap palette lib specs tokens data layout
#   ./chunk.sh bootstrap lib > /tmp/send.js   (the file is then sent as one call)
# `bootstrap` must come first in any payload: it re-creates storage.mirageLib.
dir=$(dirname "$0")
for name in "$@"; do
  awk -v want="$name" '
    /^\/\/ =+ CHUNK / { on = ($NF == want) }
    /^\/\* Build commands/ { on = 0 }
    on { print }
  ' "$dir/spotlight-build.js"
done
