#!/usr/bin/env bash
set -euo pipefail
# Bootstrap opcional de deps de análise (para dev local ou runner self-hosted)

# Python tooling
if command -v python3 >/dev/null 2>&1; then
  python3 -m pip install --upgrade pip wheel setuptools || true
  pip3 install pytest pytest-cov || true
fi

# Semgrep (via pip)
if command -v python3 >/dev/null 2>&1; then
  pip3 install semgrep || true
fi

# OPA (binário)
if ! command -v opa >/dev/null 2>&1; then
  uname_s="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "$uname_s" in
    linux*)  url="https://openpolicyagent.org/downloads/latest/opa_linux_amd64" ;;
    darwin*) url="https://openpolicyagent.org/downloads/latest/opa_darwin_amd64" ;;
    *) url="";;
  esac
  if [ -n "$url" ]; then
    curl -L -o opa "$url" && chmod +x opa && sudo mv opa /usr/local/bin/opa || true
  fi
fi

echo "Bootstrap finalizado (alguns passos podem ter sido pulados conforme SO/permissões)."


# ---- Rust toolchain (opcional) ----
# Instala cargo-llvm-cov se cargo estiver disponível
if command -v cargo >/dev/null 2>&1; then
  echo "Instalando cargo-llvm-cov (se necessário)"
  cargo install cargo-llvm-cov || true
fi
