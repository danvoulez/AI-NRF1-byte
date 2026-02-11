#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../traffic/load_env.sh" || true

echo "## Reviewer (UI forte)"
# 1) ESLint/TypeScript (se Node/monorepo web estiver disponível)
if command -v node >/dev/null 2>&1 && [ -f package.json ]; then
  if npx -y eslint . >/dev/null 2>&1; then
    echo "- ✅ ESLint passou"
  else
    echo "- ❌ ESLint apontou problemas"; npx -y eslint . || true
  fi
fi

# 2) Stylelint (opcional)
if command -v node >/dev/null 2>&1 && [ -f package.json ] && [ -f .stylelintrc* ]; then
  if npx -y stylelint "**/*.{css,scss}" >/dev/null 2>&1; then
    echo "- ✅ Stylelint passou"
  else
    echo "- ❌ Stylelint apontou problemas"; npx -y stylelint "**/*.{css,scss}" || true
  fi
fi

# 3) A11y (axe CLI) — requer URL (ex.: local/preview). Configure UI_A11Y_URL no .env.
if [ -n "${UI_A11Y_URL:-}" ]; then
  if command -v npx >/dev/null 2>&1; then
    echo "- ▶️ Rodando axe em ${UI_A11Y_URL}"
    npx -y @axe-core/cli "${UI_A11Y_URL}" || true
  else
    echo "- ⚠️ npx não encontrado; pulei axe"
  fi
else
  echo "- ℹ️ Defina UI_A11Y_URL para rodar axe (ex.: http://localhost:3000)"
fi

# 4) Lighthouse CI (opcional) — requer LHCI_URL; budgets e config podem vir do repo
if [ -n "${LHCI_URL:-}" ]; then
  if command -v npx >/dev/null 2>&1; then
    echo "- ▶️ Rodando Lighthouse CI em ${LHCI_URL}"
    npx -y @lhci/cli autorun --collect.url="${LHCI_URL}" || true
  else
    echo "- ⚠️ npx não encontrado; pulei Lighthouse"
  fi
else
  echo "- ℹ️ Defina LHCI_URL para rodar Lighthouse (ex.: http://localhost:3000)"
fi

echo ""
echo "### Observações do SKILL.md"
if [ -f skills/ui-strong/SKILL.md ]; then
  awk 'BEGIN{print "Checklist UI:\n- Fluxo\n- A11y\n- Performance\n- Responsivo\n- DS\n- Forms\n- Segurança\n- i18n"}' skills/ui-strong/SKILL.md >/dev/null
  echo "- Consulte skills/ui-strong/SKILL.md para itens qualitativos não cobertos por linters"
fi
