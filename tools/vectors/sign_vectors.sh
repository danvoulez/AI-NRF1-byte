#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

mkdir -p tests/keys
mkdir -p tests/vectors/capsule

echo "[vectors] building ubl cli"
cargo build -p ubl-cli

UBL="$ROOT/target/debug/ubl"

if [ ! -f tests/keys/alice.sk ]; then
  echo "[vectors] missing tests/keys/alice.sk; run: make vectors-keys"
  exit 1
fi

echo "[vectors] signing capsule json"
for name in capsule_ack capsule_ask capsule_nack; do
  "$UBL" cap sign "tests/vectors/capsule/${name}.json" --sk tests/keys/alice.sk -o "tests/vectors/capsule/${name}.signed.json"
  "$UBL" cap to-nrf "tests/vectors/capsule/${name}.signed.json" -o "tests/vectors/capsule/${name}.signed.nrf"
done

echo "[vectors] signed"

