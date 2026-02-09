#!/usr/bin/env bash
# ==========================================================================
# LAB 512 — Smoke Test
#
# Run this after deploy to verify the live system works end-to-end.
#
# Usage:
#   chmod +x deploy/smoke-test.sh
#   ./deploy/smoke-test.sh                          # test localhost:8080
#   ./deploy/smoke-test.sh https://passports.ubl.agency  # test production
# ==========================================================================

set -euo pipefail

BASE="${1:-http://localhost:8080}"
USER_ID="00000000-0000-0000-0000-000000000099"
PASS=0
FAIL=0

green() { printf "\033[32m✓ %s\033[0m\n" "$1"; }
red()   { printf "\033[31m✗ %s\033[0m\n" "$1"; }

check() {
    local name="$1"
    local expected_status="$2"
    local actual_status="$3"
    if [ "$actual_status" = "$expected_status" ]; then
        green "$name (HTTP $actual_status)"
        PASS=$((PASS + 1))
    else
        red "$name (expected $expected_status, got $actual_status)"
        FAIL=$((FAIL + 1))
    fi
}

echo "=========================================="
echo "  LAB 512 — Smoke Test"
echo "  Target: $BASE"
echo "=========================================="
echo ""

# --- 1. Health check ---
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/health")
check "Health endpoint" "200" "$STATUS"

# --- 2. Health response body ---
HEALTH=$(curl -s "$BASE/health")
if echo "$HEALTH" | grep -q '"ok"'; then
    green "Health body contains 'ok'"
    PASS=$((PASS + 1))
else
    red "Health body missing 'ok': $HEALTH"
    FAIL=$((FAIL + 1))
fi

# --- 3. POST without auth → 401 ---
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE/v1/lab512/dev/receipts" \
    -H "Content-Type: application/json" \
    -d '{"body":{"test":true},"act":"ATTEST","subject":"b3:0000000000000000000000000000000000000000000000000000000000000000"}')
check "POST without auth → 401" "401" "$STATUS"

# --- 4. POST with unknown user → 403 ---
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE/v1/lab512/dev/receipts" \
    -H "Content-Type: application/json" \
    -H "x-user-id: 00000000-0000-0000-0000-000000000077" \
    -d '{"body":{"test":true},"act":"ATTEST","subject":"b3:0000000000000000000000000000000000000000000000000000000000000000"}')
check "POST unknown user → 403" "403" "$STATUS"

# --- 5. POST to unknown app → 404 ---
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE/v1/nonexistent/dev/receipts" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $USER_ID" \
    -d '{"body":{"test":true},"act":"ATTEST","subject":"b3:0000000000000000000000000000000000000000000000000000000000000000"}')
check "POST unknown app → 404" "404" "$STATUS"

# --- 6. Full pipeline: create receipt ---
RECEIPT_RESP=$(curl -s -w "\n%{http_code}" \
    -X POST "$BASE/v1/lab512/dev/receipts" \
    -H "Content-Type: application/json" \
    -H "x-user-id: $USER_ID" \
    -d "{\"body\":{\"smoke_test\":true,\"ts\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"},\"act\":\"ATTEST\",\"subject\":\"b3:0000000000000000000000000000000000000000000000000000000000000000\"}")

RECEIPT_STATUS=$(echo "$RECEIPT_RESP" | tail -1)
RECEIPT_BODY=$(echo "$RECEIPT_RESP" | sed '$d')

check "POST create receipt → 200" "200" "$RECEIPT_STATUS"

if [ "$RECEIPT_STATUS" = "200" ]; then
    # --- 7. Verify receipt has CID ---
    RECEIPT_CID=$(echo "$RECEIPT_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('receipt_cid',''))" 2>/dev/null || echo "")
    if echo "$RECEIPT_CID" | grep -q "^b3:"; then
        green "Receipt CID is valid: ${RECEIPT_CID:0:20}..."
        PASS=$((PASS + 1))
    else
        red "Receipt CID missing or invalid: $RECEIPT_CID"
        FAIL=$((FAIL + 1))
    fi

    # --- 8. Verify ghost CID ---
    GHOST_CID=$(echo "$RECEIPT_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('ghost_cid',''))" 2>/dev/null || echo "")
    if echo "$GHOST_CID" | grep -q "^b3:"; then
        green "Ghost CID is valid: ${GHOST_CID:0:20}..."
        PASS=$((PASS + 1))
    else
        red "Ghost CID missing or invalid: $GHOST_CID"
        FAIL=$((FAIL + 1))
    fi

    # --- 9. Verify verifying key ---
    VK=$(echo "$RECEIPT_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('verifying_key_hex',''))" 2>/dev/null || echo "")
    if [ ${#VK} -eq 64 ]; then
        green "Verifying key present (64 hex chars)"
        PASS=$((PASS + 1))
    else
        red "Verifying key missing or wrong length: ${#VK} chars"
        FAIL=$((FAIL + 1))
    fi

    # --- 10. Read back by ID ---
    RECEIPT_ID=$(echo "$RECEIPT_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
    if [ -n "$RECEIPT_ID" ]; then
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/v1/lab512/dev/receipts/$RECEIPT_ID")
        check "GET receipt by ID → 200" "200" "$STATUS"
    fi

    # --- 11. Read back by CID ---
    if [ -n "$RECEIPT_CID" ]; then
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/v1/lab512/dev/receipts/by-cid/$RECEIPT_CID")
        check "GET receipt by CID → 200" "200" "$STATUS"
    fi
else
    red "Skipping receipt verification (create failed)"
    FAIL=$((FAIL + 5))
fi

# --- Summary ---
echo ""
echo "=========================================="
TOTAL=$((PASS + FAIL))
echo "  Results: $PASS/$TOTAL passed"
if [ "$FAIL" -gt 0 ]; then
    echo "  Status:  FAIL ($FAIL failures)"
    echo "=========================================="
    exit 1
else
    echo "  Status:  ALL PASS"
    echo "=========================================="
    exit 0
fi
