#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../traffic/load_env.sh" || true
if [ ! -f blueprint.yaml ]; then
  echo "blueprint.yaml não encontrado"
  exit 1
fi
echo "Blueprint presente ✅"
