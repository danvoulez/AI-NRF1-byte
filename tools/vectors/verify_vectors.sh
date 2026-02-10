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

echo "[vectors] verifying chain2 vector"
"$UBL" cap verify tests/vectors/capsule/capsule_ack.chain2.signed.json \
  --pk tests/vectors/capsule/alice.pk \
  --verify-chain \
  --keyring tests/vectors/capsule/keyring.json \
  --allowed-skew-ns 0

echo "[vectors] verifying ASK chain2 vector"
"$UBL" cap verify tests/vectors/capsule/capsule_ask.chain2.signed.json \
  --pk tests/vectors/capsule/alice.pk \
  --verify-chain \
  --keyring tests/vectors/capsule/keyring.json \
  --allowed-skew-ns 0

echo "[vectors] verifying expected failures"
set +e
"$UBL" cap verify tests/vectors/capsule/capsule_expired.signed.json --pk tests/vectors/capsule/alice.pk --allowed-skew-ns 0 2>tests/vectors/capsule/_expired.err
RC_EXPIRED=$?
"$UBL" cap verify tests/vectors/capsule/capsule_expired_skew.signed.json --pk tests/vectors/capsule/alice.pk --allowed-skew-ns 0 2>tests/vectors/capsule/_expired_skew.err
RC_EXPIRED_SKEW_FAIL=$?
"$UBL" cap verify tests/vectors/capsule/capsule_expired_skew.signed.json --pk tests/vectors/capsule/alice.pk --allowed-skew-ns 9223372036854775807 >/dev/null 2>tests/vectors/capsule/_expired_skew_ok.err
RC_EXPIRED_SKEW_OK=$?
"$UBL" cap verify tests/vectors/capsule/capsule_ack.tampered.signed.json --pk tests/vectors/capsule/alice.pk --allowed-skew-ns 0 2>tests/vectors/capsule/_tampered.err
RC_TAMPERED=$?
set -e

grep -q "Err.Hdr.Expired" tests/vectors/capsule/_expired.err || { echo "[vectors] expired vector did not fail as expected"; cat tests/vectors/capsule/_expired.err; exit 1; }
grep -q "Err.Hdr.Expired" tests/vectors/capsule/_expired_skew.err || { echo "[vectors] expired_skew vector did not fail as expected"; cat tests/vectors/capsule/_expired_skew.err; exit 1; }
grep -q "Err.Seal.IdMismatch" tests/vectors/capsule/_tampered.err || { echo "[vectors] tampered vector did not fail as expected"; cat tests/vectors/capsule/_tampered.err; exit 1; }

[ "$RC_EXPIRED" -ne 0 ] || { echo "[vectors] expired vector unexpectedly verified"; exit 1; }
[ "$RC_EXPIRED_SKEW_FAIL" -ne 0 ] || { echo "[vectors] expired_skew vector unexpectedly verified (no skew)"; exit 1; }
[ "$RC_EXPIRED_SKEW_OK" -eq 0 ] || { echo "[vectors] expired_skew vector failed even with skew allowance"; cat tests/vectors/capsule/_expired_skew_ok.err; exit 1; }
[ "$RC_TAMPERED" -ne 0 ] || { echo "[vectors] tampered vector unexpectedly verified"; exit 1; }

rm -f tests/vectors/capsule/_expired.err tests/vectors/capsule/_expired_skew.err tests/vectors/capsule/_expired_skew_ok.err tests/vectors/capsule/_tampered.err

echo "[vectors] OK"
