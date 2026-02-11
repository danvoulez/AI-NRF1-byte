#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../traffic/lib.sh"
ME="${SENTINEL_NAME:-SENTINEL}"

run_once() {
  echo "🛡️ $ME: checando Blueprint ao vivo..."
  ok=true

  # Testes & coverage customizáveis
  if [ -n "${SENTINEL_TEST_CMD:-}" ]; then
    if ! bash -lc "$SENTINEL_TEST_CMD"; then ok=false; fi
  elif [ -f scripts/run_tests.sh ]; then
    if ! bash scripts/run_tests.sh; then ok=false; fi
  fi

  # Semgrep customizável
  if [ -n "${SENTINEL_SEMGREP_CONFIG:-}" ]; then
    if ! semgrep --config "$SENTINEL_SEMGREP_CONFIG" ${SENTINEL_SEMGREP_FLAGS:-}; then ok=false; fi
  elif command -v semgrep >/dev/null 2>&1 && [ -f rules/semgrep/rules.yaml ]; then
    if ! semgrep --config rules/semgrep/rules.yaml; then ok=false; fi
  fi

  # OPA/Rego customizável
  if [ -n "${SENTINEL_OPA_QUERY:-}" ] && [ -n "${SENTINEL_OPA_INPUT:-}" ]; then
    if ! opa eval -i "$SENTINEL_OPA_INPUT" -d ${SENTINEL_OPA_DATABROKER:-policies/opa} "$SENTINEL_OPA_QUERY"; then ok=false; fi
  elif command -v opa >/dev/null 2>&1 && [ -f policies/opa/policy.rego ]; then
    opa eval -i blueprint.yaml -d policies/opa 'data.blueprint.allow' || ok=false
  fi

  $ok && echo "🛡️ $ME: tudo certo." || echo "🛡️ $ME: falhas detectadas."
  $ok
}

while true; do
  who="$(current)"; print_light "$who" "$ME"
  if [[ "$who" == "$ME" ]]; then
    if run_once; then
      acquire_lock
        nxt="$(next_of "$ME")"
        echo "$nxt" > "$STATE_FILE"
      release_lock
      echo "➡️  Passei o turno para $nxt"
    else
      echo "❌ $ME falhou — mantenho o turno em $ME para nova rodada."
    fi
  fi
  sleep "$SLEEP_INTERVAL"
done
