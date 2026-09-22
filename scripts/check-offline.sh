#!/bin/sh
# VorTeX never touches the network. Fails if that could have changed:
#   1. an HTTP client crate is in the app's dependency tree
#   2. the frontend references a remote URL (CDN script, font, image, fetch)
#   3. the capability file grants a plugin that isn't on the allowlist
set -eu
cd "$(dirname "$0")/.."
status=0

banned='^(reqwest|hyper|hyper-util|ureq|isahc|curl|curl-sys|attohttpc|surf|h2|tungstenite|tokio-tungstenite)$'
found=$(cargo tree -p vortex-app -e normal --prefix none --locked 2>/dev/null | awk '{print $1}' | sort -u | grep -E "$banned" || true)
if [ -n "$found" ]; then
  echo "error: network crates in the vortex-app dependency tree:"
  echo "$found" | sed 's/^/  /'
  echo "  (see why with: cargo tree -p vortex-app -e normal -i <crate>)"
  status=1
fi

# Remote URLs in quoted strings or CSS url()/@import. Comments and generated docs are fine.
urls=$(grep -rnE "[\"'\`]((https?|wss?):)?//[a-zA-Z0-9]|url\(\s*[\"']?(https?:)?//|@import\s+[\"']?(https?:)?//" \
  ui/src ui/index.html 2>/dev/null || true)
if [ -n "$urls" ]; then
  echo "error: remote URLs in the frontend (bundle the resource instead):"
  echo "$urls" | sed 's/^/  /'
  status=1
fi

# Only these permission prefixes may be granted (plus the app's own allow-* commands).
bad_perms=$(grep -oE '"[a-z-]+:[a-z-]+"' src-tauri/capabilities/*.json | grep -vE '"(core:event|core:window|dialog):' || true)
if [ -n "$bad_perms" ]; then
  echo "error: capability grants outside the allowlist (core:event, core:window, dialog):"
  echo "$bad_perms" | sed 's/^/  /'
  status=1
fi

[ "$status" -eq 0 ] && echo "offline check passed"
exit "$status"
