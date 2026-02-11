#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../traffic/lib.sh"
ME="${REVIEWER_NAME:-REVIEWER}"

run_once() {
  echo "🧐 $ME: analisando diffs e sugerindo patches..."
  if [ -n "${REVIEWER_CMD:-}" ]; then
    bash -lc "$REVIEWER_CMD" || true
  elif [ -f scripts/review_diff.py ]; then
    python3 scripts/review_diff.py > .review_report.txt || true
    cat .review_report.txt
  else
    echo "Sem REVIEWER_CMD e sem scripts/review_diff.py"
  fi
  echo "🧐 $ME: revisão concluída."
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
