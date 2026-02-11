#!/usr/bin/env bash
set -euo pipefail
: "${ALERT_WEBHOOK:=}"

msg="${1:-"[ai-nrf1] health check failed on LAB512"}"
if [[ -n "$ALERT_WEBHOOK" ]]; then
  curl -fsS -X POST -H 'Content-type: application/json' \
    --data "$(jq -nc --arg t "$msg" '{text:$t}')" \
    "$ALERT_WEBHOOK" >/dev/null || true
else
  echo "$msg"
fi
