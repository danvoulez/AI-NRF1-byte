#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../traffic/load_env.sh" || true

echo "## Reviewer (Rust)"
# 1) rustfmt check
if command -v cargo >/dev/null 2>&1; then
  if ! cargo fmt --all -- --check; then
    echo "- ❌ `cargo fmt --all -- --check` falhou (formatação necessária)"
  else
    echo "- ✅ rustfmt OK"
  fi
  # 2) clippy
  if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "- ❌ `cargo clippy -D warnings` encontrou warnings/erros"
  else
    echo "- ✅ clippy OK (sem warnings)"
  fi
else
  echo "- ⚠️ cargo não encontrado — pulando rustfmt/clippy"
fi

# 3) Heurístico opcional no diff (se existir Python)
if command -v python3 >/dev/null 2>&1 && [ -f scripts/review_diff.py ]; then
  echo ""
  python3 scripts/review_diff.py || true
fi
