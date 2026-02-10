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

if [ ! -f tests/keys/relay.sk ] || [ ! -f tests/keys/exec.sk ]; then
  echo "[vectors] missing tests/keys/(relay|exec).sk; run: make vectors-keys"
  exit 1
fi

echo "[vectors] signing capsule json"
for name in capsule_ack capsule_ask capsule_nack capsule_expired capsule_expired_skew; do
  "$UBL" cap sign "tests/vectors/capsule/${name}.json" --sk tests/keys/alice.sk -o "tests/vectors/capsule/${name}.signed.json"
  "$UBL" cap to-nrf "tests/vectors/capsule/${name}.signed.json" -o "tests/vectors/capsule/${name}.signed.nrf"
done

echo "[vectors] building chain2 vector (2 hops)"
"$UBL" cap sign tests/vectors/capsule/capsule_ack.json --sk tests/keys/alice.sk -o tests/vectors/capsule/capsule_ack.chain2.signed.json
"$UBL" cap receipt add tests/vectors/capsule/capsule_ack.chain2.signed.json --kind relay --node did:ubl:test:relay#key-1 --sk tests/keys/relay.sk --ts 1700000000100 -o tests/vectors/capsule/capsule_ack.chain2.signed.json
"$UBL" cap receipt add tests/vectors/capsule/capsule_ack.chain2.signed.json --kind exec  --node did:ubl:test:exec#key-1  --sk tests/keys/exec.sk  --ts 1700000000200 -o tests/vectors/capsule/capsule_ack.chain2.signed.json
"$UBL" cap to-nrf tests/vectors/capsule/capsule_ack.chain2.signed.json -o tests/vectors/capsule/capsule_ack.chain2.signed.nrf

echo "[vectors] building ASK chain2 vector (2 hops)"
"$UBL" cap sign tests/vectors/capsule/capsule_ask.json --sk tests/keys/alice.sk -o tests/vectors/capsule/capsule_ask.chain2.signed.json
"$UBL" cap receipt add tests/vectors/capsule/capsule_ask.chain2.signed.json --kind relay --node did:ubl:test:relay#key-1 --sk tests/keys/relay.sk --ts 1700000001100 -o tests/vectors/capsule/capsule_ask.chain2.signed.json
"$UBL" cap receipt add tests/vectors/capsule/capsule_ask.chain2.signed.json --kind exec  --node did:ubl:test:exec#key-1  --sk tests/keys/exec.sk  --ts 1700000001200 -o tests/vectors/capsule/capsule_ask.chain2.signed.json
"$UBL" cap to-nrf tests/vectors/capsule/capsule_ask.chain2.signed.json -o tests/vectors/capsule/capsule_ask.chain2.signed.nrf

echo "[vectors] building tamper vector (signed but mutated)"
python3 - <<'PY'
import json
from pathlib import Path

src = Path("tests/vectors/capsule/capsule_ack.signed.json")
dst = Path("tests/vectors/capsule/capsule_ack.tampered.signed.json")

cap = json.loads(src.read_text())
cap.setdefault("env", {}).setdefault("body", {}).setdefault("decision", {})["reason"] = "tampered"
dst.write_text(json.dumps(cap, indent=2) + "\n")
print("[vectors] wrote", dst)
PY
"$UBL" cap to-nrf tests/vectors/capsule/capsule_ack.tampered.signed.json -o tests/vectors/capsule/capsule_ack.tampered.signed.nrf

echo "[vectors] signed"
