#!/usr/bin/env sh
set -eu

NODE_BIN="${NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  if [ -x /usr/local/bin/node ]; then NODE_BIN=/usr/local/bin/node; else NODE_BIN=node; fi
fi

"$NODE_BIN" scripts/phase012_archive_validator.mjs "${1:-$(pwd)}"
