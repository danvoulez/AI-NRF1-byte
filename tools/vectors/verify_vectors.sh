#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

echo "[vectors] building ubl cli"
cargo build -p ubl-cli

UBL="$ROOT/target/debug/ubl"

if [ ! -f tests/vectors/capsule/alice.pk ]; then
  echo "[vectors] missing tests/vectors/capsule/alice.pk; run: make vectors-keys"
  exit 1
fi

echo "[vectors] verifying signed vectors"
for name in capsule_ack capsule_ask capsule_nack; do
  "$UBL" cap verify "tests/vectors/capsule/${name}.signed.json" --pk tests/vectors/capsule/alice.pk --allowed-skew-ns 0
done

echo "[vectors] OK"

