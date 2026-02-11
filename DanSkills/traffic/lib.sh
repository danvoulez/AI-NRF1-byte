#!/usr/bin/env bash
set -euo pipefail

# Carrega .env se existir
source "$(dirname "$0")/load_env.sh" || true

LOCK_DIR="${SEM_LOCK_DIR:-.sem_lock}"
STATE_FILE="${TRAFFIC_STATE_FILE:-traffic/light.state}"
SLEEP_INTERVAL="${AGENT_SLEEP_INTERVAL:-1}"

# Ordem dos agentes (customizável via env: "BUILDER,SENTINEL,REVIEWER")
AGENT_ORDER_CSV="${AGENT_ORDER:-BUILDER,SENTINEL,REVIEWER}"

IFS=',' read -r -a AGENT_ORDER_ARR <<< "$AGENT_ORDER_CSV"

# Helpers
acquire_lock() { while ! mkdir "$LOCK_DIR" 2>/dev/null; do sleep 0.1; done; }
release_lock() { rmdir "$LOCK_DIR" 2>/dev/null || true; }

current() { cat "$STATE_FILE" 2>/dev/null || echo "${TRAFFIC_INITIAL:-BUILDER}"; }
set_state() {
  acquire_lock
  echo "$1" > "$STATE_FILE"
  release_lock
}

next_of() {
  local me="$1"
  local idx=-1
  for i in "${!AGENT_ORDER_ARR[@]}"; do
    if [[ "${AGENT_ORDER_ARR[$i]}" == "$me" ]]; then idx=$i; break; fi
  done
  if [[ $idx -lt 0 ]]; then echo "${AGENT_ORDER_ARR[0]}"; return; fi
  local next=$(( (idx + 1) % ${#AGENT_ORDER_ARR[@]} ))
  echo "${AGENT_ORDER_ARR[$next]}"
}

print_light() {
  local who="$1" me="$2"
  if [[ "$who" == "$me" ]]; then
    echo "🟢 $me: SUA VEZ"
  else
    echo "🔴 $me: aguardando (turno de $who)"
  fi
}
