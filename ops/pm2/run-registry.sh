#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Load local env (not committed). Keep it simple: KEY=VALUE lines.
ENV_FILE="$ROOT/ops/pm2/local.env"
if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
fi

exec "$ROOT/target/release/registry"

