#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

mkdir -p tests/vectors/capsule

echo "[vectors] building ubl cli"
cargo build -p ubl-cli

UBL="$ROOT/target/debug/ubl"

echo "[vectors] encoding capsule json -> nrf"
for name in capsule_ack capsule_ask capsule_nack capsule_expired capsule_expired_skew; do
  "$UBL" cap to-nrf "tests/vectors/capsule/${name}.json" -o "tests/vectors/capsule/${name}.nrf"
done

echo "[vectors] done"
