#!/usr/bin/env bash
set -euo pipefail
# Lists files by their FIRST commit time (newest first).
# Usage: ./tools/list_recent_created.sh [limit]
limit="${1:-50}"
git log --diff-filter=A --pretty=format:'%ct' --name-only \
| awk 'NF{a[$0]=$prev} {prev=$0} END{for(f in a) if (f !~ /^[0-9]+$/) print a[f]"\t"f}' \
| sort -nr | head -n "$limit" \
| awk '{ts=$1; $1=""; sub(/^\t/, "", $0);
        cmd="date -d @"ts" +\"%Y-%m-%d %H:%M:%S\" 2>/dev/null"; cmd | getline h; close(cmd);
        if (h=="") { h=strftime("%Y-%m-%d %H:%M:%S", ts); }
        printf "%s  %s\n", h, $0 }'
