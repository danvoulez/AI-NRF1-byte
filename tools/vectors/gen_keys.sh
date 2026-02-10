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

if [ ! -f tests/keys/relay.sk ] || [ ! -f tests/keys/relay.pk ]; then
  echo "[vectors] generating keypair tests/keys/relay.(sk|pk)"
  "$UBL" keygen -o tests/keys/relay
fi

if [ ! -f tests/keys/exec.sk ] || [ ! -f tests/keys/exec.pk ]; then
  echo "[vectors] generating keypair tests/keys/exec.(sk|pk)"
  "$UBL" keygen -o tests/keys/exec
fi

cp -f tests/keys/alice.pk tests/vectors/capsule/alice.pk
echo "[vectors] wrote tests/vectors/capsule/alice.pk"

python3 - <<'PY'
import json
from pathlib import Path

base = Path("tests/keys")
out = {
    "did:ubl:test:relay#key-1": base.joinpath("relay.pk").read_text().strip(),
    "did:ubl:test:exec#key-1": base.joinpath("exec.pk").read_text().strip(),
}
Path("tests/vectors/capsule/keyring.json").write_text(json.dumps(out, indent=2) + "\n")
print("[vectors] wrote tests/vectors/capsule/keyring.json")
PY
