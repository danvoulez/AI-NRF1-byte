#!/usr/bin/env bash
set -euo pipefail
# Generate SBOMs (CycloneDX JSON + SPDX tag-value) into dist/sbom/
# Requires syft. If syft not found, exits with message (non-fatal if called with --best-effort)

BEST_EFFORT=0
if [[ "${1:-}" == "--best-effort" ]]; then
  BEST_EFFORT=1
  shift || true
fi

if ! command -v syft >/dev/null 2>&1; then
  echo "syft not found. Install: https://github.com/anchore/syft"
  if [[ $BEST_EFFORT -eq 1 ]]; then
    echo "(best-effort) skipping SBOM generation"
    exit 0
  fi
  exit 1
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist/sbom"
mkdir -p "$DIST"

# Target 1: workspace source tree
syft dir:"$ROOT" -o cyclonedx-json > "$DIST/workspace.cdx.json"
syft dir:"$ROOT" -o spdx-tag-value > "$DIST/workspace.spdx"

# Target 2: CLI binaries if present
for bin in "$ROOT/target/release/ai-nrf1" "$ROOT/target/release/ubl" "$ROOT/target/release/nrf1"; do
  if [[ -f "$bin" ]]; then
    name="$(basename "$bin")"
    syft file:"$bin" -o cyclonedx-json > "$DIST/${name}.cdx.json" || true
    syft file:"$bin" -o spdx-tag-value > "$DIST/${name}.spdx" || true
  fi
done

echo "SBOMs written to $DIST"
