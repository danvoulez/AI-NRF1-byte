#!/usr/bin/env bash
# ==========================================================================
# LAB 512 — Setup Script
#
# Builds the BASE, computes binary attestation hash, and starts via PM2.
#
# Prerequisites:
#   - Rust toolchain (rustup)
#   - PostgreSQL running locally
#   - PM2 installed (npm install -g pm2)
#   - Cloudflare tunnel configured (cloudflared)
#
# Usage:
#   chmod +x deploy/setup.sh
#   ./deploy/setup.sh                                    # all modules (default)
#   ./deploy/setup.sh --no-modules                       # BASE only
#   ./deploy/setup.sh --features module-receipt-gateway   # specific modules
# ==========================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# --- Parse arguments ---
CARGO_FEATURES=""
for arg in "$@"; do
    case "$arg" in
        --no-modules)
            CARGO_FEATURES="--no-default-features"
            ;;
        --features)
            # next arg will be the features list
            ;;
        --features=*)
            CARGO_FEATURES="--no-default-features --features ${arg#--features=}"
            ;;
        *)
            # If previous arg was --features, this is the features list
            if [ "${prev_arg:-}" = "--features" ]; then
                CARGO_FEATURES="--no-default-features --features $arg"
            fi
            ;;
    esac
    prev_arg="$arg"
done

echo "=========================================="
echo "  LAB 512 — BASE Deployment"
echo "=========================================="
if [ -n "$CARGO_FEATURES" ]; then
    echo "  Build flags: $CARGO_FEATURES"
else
    echo "  Build flags: default (all modules)"
fi
echo ""

# --- Step 1: Build release binary ---
echo "[1/5] Building release binary..."
cd "$PROJECT_DIR"
cargo build --release -p registry $CARGO_FEATURES
echo "  ✓ Binary: target/release/registry"

# --- Step 2: Compute binary SHA-256 (Article VIII — Runtime Attestation) ---
BINARY_SHA=$(shasum -a 256 "$PROJECT_DIR/target/release/registry" | cut -d' ' -f1)
echo "[2/5] Binary SHA-256: $BINARY_SHA"

# --- Step 3: Generate signing key if not exists ---
ENV_FILE="$SCRIPT_DIR/.env.lab512"
if [ ! -f "$ENV_FILE" ]; then
    echo "[3/5] Generating signing key..."
    SIGNING_KEY=$(openssl rand -hex 32)
    cat > "$ENV_FILE" <<EOF
# LAB 512 Environment — generated $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# DO NOT COMMIT THIS FILE

DATABASE_URL=postgres://localhost:5432/ubl_registry
ISSUER_DID=did:ubl:lab512
CDN_BASE=https://passports.ubl.agency
SIGNING_KEY_HEX=$SIGNING_KEY
BINARY_SHA256=$BINARY_SHA
RUST_LOG=registry=info,axum=info
PORT=8080
EOF
    echo "  ✓ Created $ENV_FILE"
    echo "  ⚠ KEEP THIS FILE SECRET — it contains your signing key"
else
    echo "[3/5] Updating binary hash in existing .env.lab512..."
    # Update just the BINARY_SHA256 line
    if grep -q "BINARY_SHA256" "$ENV_FILE"; then
        sed -i '' "s/BINARY_SHA256=.*/BINARY_SHA256=$BINARY_SHA/" "$ENV_FILE"
    else
        echo "BINARY_SHA256=$BINARY_SHA" >> "$ENV_FILE"
    fi
    echo "  ✓ Updated BINARY_SHA256"
fi

# --- Step 4: Create database if not exists ---
echo "[4/5] Checking database..."
if psql -lqt 2>/dev/null | cut -d \| -f 1 | grep -qw ubl_registry; then
    echo "  ✓ Database ubl_registry exists"
else
    echo "  Creating database ubl_registry..."
    createdb ubl_registry 2>/dev/null || echo "  ⚠ Could not create database — create it manually"
fi

# --- Step 5: Start with PM2 ---
echo "[5/5] Starting with PM2..."

# Source env vars for PM2
set -a
source "$ENV_FILE"
set +a

# Export for PM2 ecosystem
export DATABASE_URL ISSUER_DID CDN_BASE SIGNING_KEY_HEX BINARY_SHA256 RUST_LOG PORT

pm2 start "$SCRIPT_DIR/ecosystem.config.js" --update-env 2>/dev/null || \
    pm2 restart base-registry --update-env 2>/dev/null || \
    echo "  ⚠ PM2 start failed — run manually: pm2 start deploy/ecosystem.config.js"

echo ""
echo "=========================================="
echo "  LAB 512 — BASE is live"
echo "=========================================="
echo ""
echo "  Binary:    target/release/registry"
echo "  SHA-256:   $BINARY_SHA"
echo "  DID:       did:ubl:lab512"
echo "  Port:      8080"
echo "  DB:        ubl_registry"
echo ""
echo "  Health:    curl http://localhost:8080/health"
echo "  Receipt:   POST http://localhost:8080/v1/:app/:tenant/receipts"
echo ""
echo "  PM2:       pm2 status"
echo "  Logs:      pm2 logs base-registry"
echo "  Stop:      pm2 stop base-registry"
echo ""
echo "  Tunnel:    cloudflared tunnel run ubl"
echo "=========================================="
