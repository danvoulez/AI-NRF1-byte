#!/usr/bin/env bash
set -euo pipefail

TARGETS=(
  "http://127.0.0.1:8791/health"
  "http://127.0.0.1:8791/version"
  "https://registry.ubl.agency/health"
  "https://registry.ubl.agency/version"
)

fail=0
for url in "${TARGETS[@]}"; do
  if ! curl -fsS --max-time 3 "$url" >/dev/null; then
    echo "[FAIL] $url"
    fail=1
  fi
done

exit $fail
