#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ok=0
for i in 1 2 3; do
  if "$DIR/check.sh"; then ok=1; break; fi
  sleep 10
done

if [[ $ok -eq 0 ]]; then
  ALERT_WEBHOOK="${ALERT_WEBHOOK:-}" "$DIR/alert.sh" \
    "[ai-nrf1] 3x health failures (LAB512). Check PM2 logs and tunnel."
fi
