#!/usr/bin/env bash
set -euo pipefail

# ============================================================================
# install_from_release.sh — Download, verify, install AI-NRF1 release binaries
#
# Usage:
#   ./deploy/install_from_release.sh              # installs latest (v1.0.0)
#   ./deploy/install_from_release.sh v1.0.1       # installs specific tag
#
# What it does:
#   1. Downloads the platform package from GitHub Releases
#   2. Verifies SHA-256 checksum
#   3. Verifies cosign signature (if cosign is installed)
#   4. Backs up previous binaries (rollback-friendly)
#   5. Installs ai-nrf1, ubl, registry to /usr/local/bin
#   6. Removes macOS quarantine if needed
#   7. Restarts PM2 (if running)
# ============================================================================

REPO="danvoulez/AI-NRF1-byte"
TAG="${1:-v1.0.0}"
INSTALL_DIR="/usr/local/bin"
PM2_HOME="${PM2_HOME:-$HOME/.pm2-ai-nrf1}"
BINARIES=(ai-nrf1 ubl registry)

# --- Detect platform ---
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  darwin) OS_LABEL="macos" ;;
  linux)  OS_LABEL="linux" ;;
  *)      echo "❌ Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  arm64|aarch64) ARCH_LABEL="arm64" ;;
  x86_64)        ARCH_LABEL="x64"   ;;
  *)             echo "❌ Unsupported arch: $ARCH"; exit 1 ;;
esac

ASSET="base-${TAG}-${OS_LABEL}-${ARCH_LABEL}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"
WORK_DIR="$(mktemp -d)"

cleanup() { rm -rf "$WORK_DIR"; }
trap cleanup EXIT

echo "🔧 Installing AI-NRF1 ${TAG} (${OS_LABEL}/${ARCH_LABEL})"
echo "   Work dir: ${WORK_DIR}"
echo ""

# --- 1. Download ---
echo "⬇️  Downloading ${ASSET}..."
curl -fSL -o "${WORK_DIR}/${ASSET}" "${BASE_URL}/${ASSET}"
curl -fSL -o "${WORK_DIR}/CHECKSUMS.sha256" "${BASE_URL}/CHECKSUMS.sha256"

# --- 2. Verify checksum ---
echo "🔍 Verifying checksum..."
cd "$WORK_DIR"
EXPECTED=$(grep "$ASSET" CHECKSUMS.sha256 | awk '{print $1}')
ACTUAL=$(shasum -a 256 "$ASSET" | awk '{print $1}')

if [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "❌ Checksum mismatch!"
  echo "   Expected: $EXPECTED"
  echo "   Got:      $ACTUAL"
  exit 1
fi
echo "   ✅ SHA-256 OK"

# --- 3. Verify cosign signature (optional) ---
if command -v cosign &>/dev/null; then
  echo "🔏 Verifying cosign signature..."
  curl -fSL -o "${ASSET}.sig" "${BASE_URL}/${ASSET}.sig" 2>/dev/null || true
  if [ -f "${ASSET}.sig" ]; then
    cosign verify-blob --yes --signature "${ASSET}.sig" "${ASSET}" 2>/dev/null \
      && echo "   ✅ Signature OK" \
      || echo "   ⚠️  Signature verification failed (non-fatal)"
  fi
else
  echo "   ℹ️  cosign not found, skipping signature check"
fi

# --- 4. Extract ---
echo "📦 Extracting..."
tar -xzf "$ASSET"

# --- 5. Backup previous binaries ---
for bin in "${BINARIES[@]}"; do
  if [ -f "${INSTALL_DIR}/${bin}" ]; then
    PREV_VER=$("${INSTALL_DIR}/${bin}" --version 2>/dev/null | head -1 || echo "unknown")
    echo "   📋 Backing up ${bin} (${PREV_VER}) → ${bin}.prev"
    sudo cp -f "${INSTALL_DIR}/${bin}" "${INSTALL_DIR}/${bin}.prev"
  fi
done

# --- 6. Install ---
echo "🚀 Installing to ${INSTALL_DIR}..."
for bin in "${BINARIES[@]}"; do
  if [ -f "$bin" ]; then
    sudo install -m 0755 "$bin" "${INSTALL_DIR}/${bin}"
    echo "   ✅ ${bin}"
  else
    echo "   ⚠️  ${bin} not found in archive"
  fi
done

# --- 7. macOS quarantine ---
if [ "$OS" = "darwin" ]; then
  for bin in "${BINARIES[@]}"; do
    xattr -dr com.apple.quarantine "${INSTALL_DIR}/${bin}" 2>/dev/null || true
  done
  echo "   🍎 Quarantine cleared"
fi

# --- 8. Verify installed versions ---
echo ""
echo "📋 Installed versions:"
for bin in "${BINARIES[@]}"; do
  VER=$("${INSTALL_DIR}/${bin}" --version 2>/dev/null | head -1 || echo "(no --version)")
  echo "   ${bin}: ${VER}"
done

# --- 9. Restart PM2 (if running) ---
if command -v pm2 &>/dev/null && [ -d "$PM2_HOME" ]; then
  echo ""
  echo "♻️  Restarting PM2..."
  PM2_HOME="$PM2_HOME" pm2 restart all --update-env 2>/dev/null || true
  sleep 2
  PM2_HOME="$PM2_HOME" pm2 list
else
  echo ""
  echo "   ℹ️  PM2 not running, skipping restart"
fi

# --- 10. Health check ---
echo ""
echo "🏥 Health check..."
sleep 1
if curl -fsS http://127.0.0.1:8791/health 2>/dev/null; then
  echo ""
  echo "   ✅ Local registry healthy"
else
  echo "   ⚠️  Local registry not responding on :8791 (may need manual start)"
fi

if curl -fsS https://registry.ubl.agency/health 2>/dev/null; then
  echo ""
  echo "   ✅ Tunnel healthy"
else
  echo "   ⚠️  Tunnel not responding (may need cloudflared restart)"
fi

echo ""
echo "🎉 AI-NRF1 ${TAG} installed successfully!"
echo ""
echo "Rollback: sudo cp ${INSTALL_DIR}/{ai-nrf1,ubl,registry}.prev ${INSTALL_DIR}/ && PM2_HOME=${PM2_HOME} pm2 restart all"
