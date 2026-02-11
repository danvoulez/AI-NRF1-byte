#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../traffic/lib.sh"
ME="${BUILDER_NAME:-BUILDER}"

run_once() {
  echo "👷 $ME: executando tarefa..."
  # Pré-cmd opcional
  if [ -n "${BUILDER_PRE_CMD:-}" ]; then bash -lc "$BUILDER_PRE_CMD" || true; fi

  if [ -n "${BUILDER_CMD:-}" ]; then
    bash -lc "$BUILDER_CMD" || true
  else
    # Exemplo padrão
    if [ -f scripts/gen_from_blueprint.py ]; then
      python3 scripts/gen_from_blueprint.py || true
    fi
  fi

  # Git commit/push configuráveis
  GIT_ADD="${BUILDER_GIT_ADD:-true}"
  GIT_COMMIT_MSG="${BUILDER_GIT_COMMIT_MSG:-feat: alterações do Builder (automático)}"
  GIT_PUSH="${BUILDER_GIT_PUSH:-true}"
  GIT_BRANCH="${BUILDER_GIT_BRANCH:-}"

  if [ "${GIT_ADD}" = "true" ]; then git add -A; fi

  if ! git diff --cached --quiet; then
    if [ -n "$GIT_BRANCH" ]; then
      git checkout -B "$GIT_BRANCH" || git checkout "$GIT_BRANCH" || true
    fi
    git commit -m "$GIT_COMMIT_MSG" || true
    if [ "${GIT_PUSH}" = "true" ]; then git push -u origin "$(git branch --show-current)" || true; fi
  else
    echo "Sem mudanças a commitar."
  fi

  # Pós-cmd opcional
  if [ -n "${BUILDER_POST_CMD:-}" ]; then bash -lc "$BUILDER_POST_CMD" || true; fi

  echo "👷 $ME: pronto."
}

while true; do
  who="$(current)"; print_light "$who" "$ME"
  if [[ "$who" == "$ME" ]]; then
    run_once
    acquire_lock
      nxt="$(next_of "$ME")"
      echo "$nxt" > "$STATE_FILE"
    release_lock
    echo "➡️  Passei o turno para $nxt"
  fi
  sleep "$SLEEP_INTERVAL"
done
