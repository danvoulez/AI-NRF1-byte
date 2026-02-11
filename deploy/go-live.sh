#!/usr/bin/env bash
set -euo pipefail

# ==========================================================================
# go-live.sh — Build, configure, and start the AI-NRF1 production stack.
#
# ONE binary (registry --features modules) + Cloudflare Tunnel via PM2.
#
# Usage:
#   bash deploy/go-live.sh          # full build + start
#   bash deploy/go-live.sh --skip-build   # just (re)start PM2
# ==========================================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
STATE_DIR="${STATE_DIR:-$HOME/.ai-nrf1/state}"
PM2_HOME="${PM2_HOME:-$HOME/.pm2-ai-nrf1}"
export PM2_HOME

SKIP_BUILD=false
if [[ "${1:-}" == "--skip-build" ]]; then
  SKIP_BUILD=true
fi

echo "╔══════════════════════════════════════════════════════╗"
echo "║  AI-NRF1 / UBL — Go Live                           ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""
echo "  ROOT:      $ROOT"
echo "  STATE_DIR: $STATE_DIR"
echo "  PM2_HOME:  $PM2_HOME"
echo ""

# ------------------------------------------------------------------
# 1. Preflight checks
# ------------------------------------------------------------------
echo "▸ Preflight checks..."

command -v cargo >/dev/null 2>&1 || { echo "✗ cargo not found"; exit 1; }
command -v pm2 >/dev/null 2>&1 || { echo "✗ pm2 not found (npm install -g pm2)"; exit 1; }

if ! command -v cloudflared >/dev/null 2>&1; then
  echo "⚠ cloudflared not found — tunnel will not start"
  echo "  Install: brew install cloudflared"
fi

# ------------------------------------------------------------------
# 2. Create state directories
# ------------------------------------------------------------------
echo "▸ Creating state directories..."
mkdir -p "$STATE_DIR"/{idem,permit-tickets,llm-cache,resume}
echo "  ✓ $STATE_DIR"

# ------------------------------------------------------------------
# 3. .env file
# ------------------------------------------------------------------
ENV_FILE="$ROOT/.env"
if [[ ! -f "$ENV_FILE" ]]; then
  echo "▸ No .env found — copying template..."
  cp "$ROOT/deploy/.env.production" "$ENV_FILE"
  echo "  ✓ Created $ENV_FILE"
  echo "  ⚠ EDIT .env with your real secrets before proceeding!"
  echo "    Required: SIGNING_KEY_HEX"
  echo "    Optional: OPENAI_API_KEY, WH_SEC"
  echo ""
  read -rp "  Press Enter after editing .env (or Ctrl+C to abort)..."
fi

# ------------------------------------------------------------------
# 4. Build release binary
# ------------------------------------------------------------------
if [[ "$SKIP_BUILD" == "false" ]]; then
  echo "▸ Building release binary (registry --features modules)..."
  cd "$ROOT"
  cargo build --release -p registry --features modules
  BINARY="$ROOT/target/release/registry"
  echo "  ✓ $BINARY"

  # Compute binary hash for runtime attestation
  HASH=$(shasum -a 256 "$BINARY" | awk '{print $1}')
  echo "  Binary SHA256: $HASH"

  # Inject into .env if not already set
  if ! grep -q "^BINARY_SHA256=" "$ENV_FILE" 2>/dev/null; then
    echo "BINARY_SHA256=$HASH" >> "$ENV_FILE"
    echo "  ✓ Appended BINARY_SHA256 to .env"
  fi
else
  echo "▸ Skipping build (--skip-build)"
fi

# ------------------------------------------------------------------
# 5. Run tests
# ------------------------------------------------------------------
if [[ "$SKIP_BUILD" == "false" ]]; then
  echo "▸ Running tests..."
  cd "$ROOT"
  cargo test -p module-runner --quiet
  echo "  ✓ module-runner tests passed"
fi

# ------------------------------------------------------------------
# 6. Stop existing PM2 processes
# ------------------------------------------------------------------
echo "▸ Stopping existing PM2 processes..."
pm2 delete all 2>/dev/null || true

# ------------------------------------------------------------------
# 7. Start PM2
# ------------------------------------------------------------------
echo "▸ Starting PM2..."
cd "$ROOT"
pm2 start deploy/ecosystem.config.js
pm2 save

echo ""
echo "▸ PM2 status:"
pm2 list

# ------------------------------------------------------------------
# 8. Health check
# ------------------------------------------------------------------
echo ""
echo "▸ Waiting for health check..."
sleep 2

PORT="${PORT:-8791}"
if curl -sf "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
  HEALTH=$(curl -s "http://127.0.0.1:$PORT/health")
  echo "  ✓ http://127.0.0.1:$PORT/health → $HEALTH"
else
  echo "  ⚠ Health check failed — check logs: pm2 logs ai-nrf1"
fi

# ------------------------------------------------------------------
# 9. Done
# ------------------------------------------------------------------
echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║  ✓ AI-NRF1 is live!                                ║"
echo "║                                                    ║"
echo "║  Local:  http://127.0.0.1:$PORT                      ║"
echo "║  Tunnel: https://registry.ubl.agency (if configured)║"
echo "║                                                    ║"
echo "║  Commands:                                         ║"
echo "║    pm2 logs ai-nrf1     — view logs                ║"
echo "║    pm2 restart ai-nrf1  — restart                  ║"
echo "║    pm2 monit            — dashboard                ║"
echo "║    ubl permit list --tenant <t>  — list tickets    ║"
echo "╚══════════════════════════════════════════════════════╝"
