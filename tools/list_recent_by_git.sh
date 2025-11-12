#!/usr/bin/env bash
set -euo pipefail
# Lists tracked files sorted by their LAST commit time (newest first).
# Usage: ./tools/list_recent_by_git.sh [limit]
limit="${1:-50}"
tmp="$(mktemp)"
# Collect last commit timestamp per file
git ls-files | while read -r f; do
  ts="$(git log -1 --format='%ct' -- "$f" || echo 0)"
  printf '%s\t%s\n' "$ts" "$f"
done > "$tmp"
# Sort by timestamp desc and format
sort -nr "$tmp" | head -n "$limit" | awk '{
  cmd="date -d @"$1" +\"%Y-%m-%d %H:%M:%S\" 2>/dev/null"; cmd | getline h; close(cmd);
  if (h=="") { h=strftime("%Y-%m-%d %H:%M:%S", $1); }
  $1=""; sub(/^\t/, "", $0); printf "%s  %s\n", h, $0
}'
rm -f "$tmp"
