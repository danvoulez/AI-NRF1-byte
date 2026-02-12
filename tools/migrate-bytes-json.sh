#!/usr/bin/env bash
# migrate-bytes-json.sh — Canon 4 migration helper
#
# Converts legacy byte JSON forms to the canonical {"$bytes":"<lowercase hex>"}.
#
# What it fixes:
#   1. Uppercase hex in $bytes values → lowercase
#   2. (Manual) "b3:<hex>" and "b64:<base64>" strings that were bytes
#      must be converted externally before feeding into the pipeline.
#
# Usage:
#   ./tools/migrate-bytes-json.sh input.json > output.json
#   cat *.ndjson | ./tools/migrate-bytes-json.sh - > migrated.ndjson
#
# Requires: jq 1.6+

set -euo pipefail

INPUT="${1:--}"

jq 'walk(
  if type == "object" and has("$bytes") and (.["$bytes"] | type) == "string"
  then .["$bytes"] |= ascii_downcase
  else .
  end
)' "$INPUT"
