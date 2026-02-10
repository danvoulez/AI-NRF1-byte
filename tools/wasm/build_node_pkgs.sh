#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack not found. Install with: cargo install wasm-pack" >&2
  exit 2
fi

mkdir -p target/wasm-pkg

echo "[wasm] building ubl-capsule-wasm (nodejs)"
wasm-pack build bindings/wasm/ubl-capsule-wasm \
  --target nodejs \
  --out-dir ../../../target/wasm-pkg/ubl-capsule-wasm \
  --release

echo "[wasm] building ai-nrf1-wasm (nodejs)"
wasm-pack build bindings/wasm/ai-nrf1-wasm \
  --target nodejs \
  --out-dir ../../../target/wasm-pkg/ai-nrf1-wasm \
  --release

echo "[wasm] done"

