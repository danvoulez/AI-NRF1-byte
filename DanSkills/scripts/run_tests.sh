#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../traffic/load_env.sh" || true

# 1) Se TEST_CMD foi definido, usa ele.
if [ -n "${TEST_CMD:-}" ]; then
  bash -lc "$TEST_CMD"
  exit $?
fi

# 2) Rust first: llvm-cov se existir; senão cargo test simples.
if command -v cargo >/dev/null 2>&1; then
  if command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "▶️  Rodando cargo llvm-cov (se disponível)"
    cargo llvm-cov --summary-only || { echo "❌ llvm-cov falhou"; exit 1; }
    exit 0
  else
    echo "▶️  Rodando cargo test"
    cargo test --all --all-features --quiet
    exit $?
  fi
fi

# 3) Python fallback (pytest) se cargo não existir
if command -v pytest >/dev/null 2>&1; then
  pytest --maxfail=1 -q --disable-warnings --cov=. --cov-report=term || { echo "Testes falharam ❌"; exit 1; }
  exit 0
fi

echo "⚠️ Nenhum runner de testes encontrado (configure TEST_CMD ou instale cargo/pytest)."
exit 0
