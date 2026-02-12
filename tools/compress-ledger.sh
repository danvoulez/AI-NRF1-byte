#!/usr/bin/env bash
# Weekly NDJSON ledger compression
# Called by PM2 cron or manually: bash tools/compress-ledger.sh
#
# Walks LEDGER_DIR/{app}/{tenant}/*.ndjson and gzips non-empty files
# into {stream}.{ISO_WEEK}.ndjson.gz, then truncates the active file.

set -euo pipefail

LEDGER_DIR="${LEDGER_DIR:-$HOME/.ai-nrf1/ledger}"
ISO_WEEK=$(date +%G-W%V)

echo "[compress-ledger] $(date -u +%FT%TZ) — scanning $LEDGER_DIR"

if [ ! -d "$LEDGER_DIR" ]; then
  echo "[compress-ledger] ledger dir does not exist, nothing to do"
  exit 0
fi

compressed=0

find "$LEDGER_DIR" -name '*.ndjson' -type f | while read -r ndjson; do
  # Skip empty files
  if [ ! -s "$ndjson" ]; then
    continue
  fi

  dir=$(dirname "$ndjson")
  base=$(basename "$ndjson" .ndjson)
  archive="${dir}/${base}.${ISO_WEEK}.ndjson.gz"

  echo "[compress-ledger] compressing $ndjson → $archive"
  gzip -c "$ndjson" > "$archive"
  : > "$ndjson"  # truncate active file

  original=$(stat -f%z "$archive" 2>/dev/null || stat -c%s "$archive" 2>/dev/null || echo "?")
  echo "[compress-ledger]   done (archive: ${original} bytes)"
  compressed=$((compressed + 1))
done

echo "[compress-ledger] compressed $compressed file(s)"
