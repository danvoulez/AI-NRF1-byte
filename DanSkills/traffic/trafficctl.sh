#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"

cmd="${1:-}"
case "$cmd" in
  init)
    set_state "${TRAFFIC_INITIAL:-BUILDER}"
    echo "Semáforo iniciado em $(current) 🚦 (ordem: ${AGENT_ORDER:-BUILDER,SENTINEL,REVIEWER})"
    ;;
  show)
    who="$(current)"; echo "Agora: $who"
    ;;
  set)
    target="${2:-${TRAFFIC_INITIAL:-BUILDER}}"
    set_state "$target"
    echo "Forçado para: $target"
    ;;
  *)
    echo "Uso: $0 {init|show|set <AGENTE>}"
    echo "Agentes na ordem: ${AGENT_ORDER:-BUILDER,SENTINEL,REVIEWER}"
    exit 1
    ;;
esac
