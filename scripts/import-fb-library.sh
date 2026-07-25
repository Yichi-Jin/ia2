#!/usr/bin/env bash
#
# import-fb-library.sh — thin wrapper kept for muscle memory; the real
# tool is the CLI (one implementation, not two):
#
#   cs library import process-control [--blocks fb_pid.st,fb_alarm_hl.st]
#   cs ls library          # registry + imported state
#   cs rm library/<name>   # remove from the project
#
# Usage: scripts/import-fb-library.sh <project-name-or-dir> [fb_name ...]
# Server URL defaults to http://127.0.0.1:3001 (IA2_SERVER_URL overrides).

set -euo pipefail

SERVER="${IA2_SERVER_URL:-http://127.0.0.1:3001}"
PROJ="${1:-}"
[ -n "$PROJ" ] || { echo "usage: $0 <project-name-or-dir> [fb_name ...]" >&2; exit 2; }
shift || true

CS="${CS:-cs}"
command -v "$CS" >/dev/null 2>&1 || CS="$(dirname "$0")/../target/debug/cs"
[ -x "$CS" ] || { echo "ERROR: cs not found — install it or set CS=/path/to/cs" >&2; exit 2; }

# A directory argument means "open that project first" (idempotent).
if [ -d "$PROJ" ]; then
  ABS="$(cd "$PROJ" && pwd)"
  "$CS" --server "$SERVER" project open "$ABS" >/dev/null
  PROJ="$(basename "$ABS")"
fi

if [ "$#" -gt 0 ]; then
  BLOCKS="$(printf '%s\n' "$@" | sed -E 's/(\.st)?$/.st/' | paste -sd, -)"
  exec "$CS" --server "$SERVER" --project "$PROJ" library import process-control --blocks "$BLOCKS"
else
  exec "$CS" --server "$SERVER" --project "$PROJ" library import process-control
fi
