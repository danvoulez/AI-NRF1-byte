#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# ubl product init — the product spitter
#
# Copies the VANILLA ui-template (not the mother), writes a product.json
# manifest, and points it at the platform registry. The output is a
# standalone Next.js repo that talks to LAB 512 services via HTTP.
# No BASE or MODULES code is copied.
#
# The mother (services/tdln-ui) stays untouched — it is a working product,
# not a template. This script uses services/ui-template/ as the source.
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEMPLATE_UI="$REPO_ROOT/services/ui-template"

usage() {
  cat <<EOF
Usage: $(basename "$0") --name <name> --registry <url> --out <dir>

Options:
  --name       Product display name (e.g. "Acme Verify")
  --slug       Product slug (optional, derived from name)
  --registry   Registry URL (e.g. https://lab512.example.com)
  --out        Output directory for the new product repo
  --tenant     Default tenant ID (optional, default: "default")
  --locale     Locale (optional, default: "pt-BR")
  --primary    Primary theme color (optional, default: "#0f1117")
  --accent     Accent theme color (optional, default: "#10b981")

Example:
  $(basename "$0") --name "Acme Verify" --registry https://lab512.example.com --out ~/repos/acme-verify
EOF
  exit 1
}

# Parse args
PRODUCT_NAME=""
PRODUCT_SLUG=""
REGISTRY_URL=""
OUT_DIR=""
TENANT="default"
LOCALE="pt-BR"
PRIMARY="#0f1117"
ACCENT="#10b981"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --name)     PRODUCT_NAME="$2"; shift 2 ;;
    --slug)     PRODUCT_SLUG="$2"; shift 2 ;;
    --registry) REGISTRY_URL="$2"; shift 2 ;;
    --out)      OUT_DIR="$2"; shift 2 ;;
    --tenant)   TENANT="$2"; shift 2 ;;
    --locale)   LOCALE="$2"; shift 2 ;;
    --primary)  PRIMARY="$2"; shift 2 ;;
    --accent)   ACCENT="$2"; shift 2 ;;
    -h|--help)  usage ;;
    *) echo "Unknown option: $1"; usage ;;
  esac
done

if [[ -z "$PRODUCT_NAME" || -z "$REGISTRY_URL" || -z "$OUT_DIR" ]]; then
  echo "Error: --name, --registry, and --out are required."
  usage
fi

# Derive slug from name if not provided
if [[ -z "$PRODUCT_SLUG" ]]; then
  PRODUCT_SLUG=$(echo "$PRODUCT_NAME" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9-')
fi

echo "=== Product Spitter ==="
echo "  Name:     $PRODUCT_NAME"
echo "  Slug:     $PRODUCT_SLUG"
echo "  Registry: $REGISTRY_URL"
echo "  Tenant:   $TENANT"
echo "  Locale:   $LOCALE"
echo "  Theme:    $PRIMARY / $ACCENT"
echo "  Output:   $OUT_DIR"
echo ""

# ---------------------------------------------------------------------------
# 1. Copy vanilla template (excluding node_modules, .next)
# ---------------------------------------------------------------------------
if [[ ! -d "$TEMPLATE_UI" ]]; then
  echo "Error: Template not found at $TEMPLATE_UI"
  exit 1
fi

if [[ -d "$OUT_DIR" ]]; then
  echo "Error: $OUT_DIR already exists. Remove it first or choose a different path."
  exit 1
fi

echo "[1/6] Copying ui-template..."
mkdir -p "$OUT_DIR"
rsync -a \
  --exclude='node_modules' \
  --exclude='.next' \
  --exclude='.env.local' \
  --exclude='.env*.local' \
  "$TEMPLATE_UI/" "$OUT_DIR/"

# ---------------------------------------------------------------------------
# 2. Write product.json (the manifest that drives the UI)
# ---------------------------------------------------------------------------
echo "[2/6] Writing product.json..."
cat > "$OUT_DIR/product.json" <<MANIFEST
{
  "name": "$PRODUCT_NAME",
  "slug": "$PRODUCT_SLUG",
  "registry": "$REGISTRY_URL",
  "tenant": "$TENANT",
  "theme": {
    "primary": "$PRIMARY",
    "accent": "$ACCENT",
    "radius": "0.5rem"
  },
  "logo": "/placeholder-logo.svg",
  "title": "$PRODUCT_NAME - Prova criptografica",
  "description": "Verificacao criptografica verificavel, offline e sem custodia.",
  "locale": "$LOCALE",
  "pages": [
    "dashboard",
    "executions",
    "receipts",
    "audits",
    "evidence",
    "policies",
    "integrations",
    "billing",
    "team",
    "settings",
    "help"
  ],
  "features": {
    "run_pipeline": true,
    "team_management": true,
    "billing": true,
    "marketing_pages": true
  }
}
MANIFEST

# ---------------------------------------------------------------------------
# 3. Customize package.json
# ---------------------------------------------------------------------------
echo "[3/6] Customizing package.json..."
if command -v python3 &>/dev/null; then
  python3 -c "
import json
with open('$OUT_DIR/package.json') as f:
    pkg = json.load(f)
pkg['name'] = '$PRODUCT_SLUG'
pkg['version'] = '1.0.0'
pkg['private'] = True
with open('$OUT_DIR/package.json', 'w') as f:
    json.dump(pkg, f, indent=2)
    f.write('\n')
"
else
  sed -i.bak "s/\"name\": \"my-project\"/\"name\": \"$PRODUCT_SLUG\"/" "$OUT_DIR/package.json"
  rm -f "$OUT_DIR/package.json.bak"
fi

# ---------------------------------------------------------------------------
# 4. Write .env.local
# ---------------------------------------------------------------------------
echo "[4/6] Writing .env.local..."
cat > "$OUT_DIR/.env.local" <<EOF
NEXT_PUBLIC_REGISTRY_URL=$REGISTRY_URL
NEXT_PUBLIC_TENANT=$TENANT
NEXT_PUBLIC_PRODUCT=$PRODUCT_SLUG
EOF

# ---------------------------------------------------------------------------
# 5. Create README
# ---------------------------------------------------------------------------
echo "[5/6] Creating README..."
cat > "$OUT_DIR/README.md" <<EOF
# $PRODUCT_NAME

A product built on the UBL cryptographic proof platform.

## Quick Start

\`\`\`bash
pnpm install
pnpm dev
\`\`\`

Open [http://localhost:3000](http://localhost:3000).

## Configuration

Edit \`product.json\` to customize:
- **name/slug** — product identity
- **theme** — colors and radius
- **pages** — which console pages are enabled
- **features** — toggle run_pipeline, team_management, billing, etc.

Edit \`.env.local\` for runtime config:

| Variable | Description |
|---|---|
| \`NEXT_PUBLIC_REGISTRY_URL\` | Platform registry URL |
| \`NEXT_PUBLIC_TENANT\` | Your tenant ID |
| \`NEXT_PUBLIC_PRODUCT\` | Product slug |

## Architecture

This product is a **thin UI layer** that connects to the platform
via HTTP. All policy evaluation, receipt generation, and cryptographic
proofs are handled by the platform services.

- \`POST /modules/run\` — Execute a pipeline
- \`GET /api/executions\` — List past executions
- \`GET /api/receipts/:cid\` — Receipt details (SIRP timeline, proofs)
- \`GET /api/metrics\` — Dashboard statistics

## Deploy

\`\`\`bash
pnpm build
# Deploy to Vercel, Docker, or any static host
\`\`\`
EOF

# ---------------------------------------------------------------------------
# 6. Validate: no platform code leaked
# ---------------------------------------------------------------------------
echo "[6/6] Validating output..."
LEAKED=0
for pattern in "*.rs" "Cargo.toml" "cap-intake" "cap-policy" "module-runner"; do
  if find "$OUT_DIR" -name "$pattern" -not -path "*/node_modules/*" 2>/dev/null | grep -q .; then
    echo "  WARNING: Found '$pattern' in output — possible leak!"
    LEAKED=1
  fi
done
if [[ $LEAKED -eq 0 ]]; then
  echo "  No platform code detected in output."
fi

# ---------------------------------------------------------------------------
# 7. Init git repo
# ---------------------------------------------------------------------------
cd "$OUT_DIR"
git init -q
git add -A
git commit -q -m "init: $PRODUCT_NAME (generated by product-spitter)"

echo ""
echo "=== Done! ==="
echo ""
echo "  cd $OUT_DIR"
echo "  pnpm install"
echo "  pnpm dev"
echo ""
echo "Your product is connected to: $REGISTRY_URL"
echo "Customize product.json to change branding, pages, and features."
echo "No platform code was copied — only the UI template."
