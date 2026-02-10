#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

mkdir -p tests/keys
mkdir -p tests/vectors/capsule

echo "[vectors] building ubl cli"
cargo build -p ubl-cli

UBL="$ROOT/target/debug/ubl"

if [ ! -f tests/keys/alice.sk ] || [ ! -f tests/keys/alice.pk ]; then
  echo "[vectors] generating keypair tests/keys/alice.(sk|pk)"
  "$UBL" keygen -o tests/keys/alice
fi

cp -f tests/keys/alice.pk tests/vectors/capsule/alice.pk
echo "[vectors] wrote tests/vectors/capsule/alice.pk"

