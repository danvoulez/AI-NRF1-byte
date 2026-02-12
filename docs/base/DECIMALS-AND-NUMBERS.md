# Numbers Without Floats — Developer Guide

ai-nrf1 **forbids IEEE 754 floating point**. This is not a limitation —
it is the foundation of determinism. Floats produce different bit patterns
on different architectures, rounding modes, and compilers. One bit of drift
breaks the entire hash chain.

This guide explains how to represent every kind of numeric data.

## The Rule

ai-nrf1 has exactly **one** numeric type: **Int64** (signed, 64-bit,
two's complement, big-endian). Everything else is a **String** in
ρ-canonical decimal form.

## Quick Reference

| Domain | Representation | Example |
|--------|---------------|---------|
| Counters, IDs, epochs | `Int64` | `42`, `-1`, `1738000000` |
| Currency (USD) | String, cents | `"1999"` (= $19.99) |
| Currency (BTC) | String, satoshis | `"100000000"` (= 1 BTC) |
| Currency (arbitrary) | String, decimal | `"19.99"` |
| Percentages | String, decimal | `"0.15"` (= 15%) |
| Scientific measurements | String, decimal | `"3.14159"` |
| Coordinates (lat/lon) | String, decimal | `"-23.5505"` |
| Ratios / scores | String, decimal | `"0.95"` |
| Timestamps | `Int64` (epoch nanos) | `1738000000000000000` |
| Durations | `Int64` (nanos) | `5000000000` (= 5s) |

## ρ-Canonical Decimal Rules

Decimal strings MUST match: `^-?(0|[1-9][0-9]*)(\.[0-9]+)?$`

- **No exponent notation**: `1e2` → rejected, use `"100"`
- **No leading zeros**: `01.5` → rejected, use `"1.5"`
- **No trailing zeros**: `"1.50"` → normalized to `"1.5"`
- **No superfluous `.0`**: `"1.0"` → normalized to `"1"`
- **Negative zero forbidden**: `"-0"` → normalized to `"0"`
- **Negative zero with fraction**: `"-0.0"` → normalized to `"0"`

ρ normalization handles these automatically when you use
`rho::normalize_decimal()`. If you pass already-canonical strings,
they pass through unchanged.

## Patterns by Domain

### Money / Currency

**Best practice:** use the smallest unit as an integer string.

```json
{
  "amount": "1999",
  "currency": "USD",
  "unit": "cents"
}
```

Or use decimal strings with explicit precision:

```json
{
  "amount": "19.99",
  "currency": "USD"
}
```

Both are valid. The key insight: `"19.99"` is a **String** in ai-nrf1,
not a number. It will always hash identically on every platform.

### Scientific / Measurement Data

Use decimal strings. Precision is preserved exactly as written
(after ρ strips trailing zeros).

```json
{
  "temperature": "36.6",
  "unit": "celsius",
  "precision": "0.1"
}
```

### Scores / Probabilities / Ratios

```json
{
  "confidence": "0.95",
  "threshold": "0.8"
}
```

### Coordinates

```json
{
  "lat": "-23.5505",
  "lon": "-46.6333"
}
```

### Large Numbers

Int64 covers ±9.2×10¹⁸. For larger values, use decimal strings:

```json
{
  "total_supply": "21000000000000000"
}
```

## What Happens If You Send a Float?

| Language | Behavior |
|----------|----------|
| Rust | `Error::Float` at encode time |
| Python | `Float` exception at encode time |
| JS/WASM | `JsError("Float: ai-nrf1 forbids floating point...")` |
| JSON input | `serde_json::Number::is_f64()` → rejected |

The error is immediate and clear. No silent rounding.

## Converting From External Systems

When ingesting data from systems that use floats:

```rust
// Rust: convert f64 to canonical decimal string
fn float_to_decimal(f: f64) -> String {
    // Format with enough precision, then normalize
    let s = format!("{f}");
    nrf_core::rho::normalize_decimal(&s)
        .expect("valid decimal")
}
```

```python
# Python: convert float to canonical decimal string
from decimal import Decimal
def float_to_decimal(f: float) -> str:
    # Use Decimal to avoid repr artifacts
    d = Decimal(str(f)).normalize()
    return str(d)
```

```typescript
// TypeScript: convert number to decimal string
function floatToDecimal(n: number): string {
  // Use toPrecision to avoid floating point artifacts
  return parseFloat(n.toPrecision(15)).toString();
}
```

## FAQ

**Q: Why not fixed-point integers everywhere?**
A: Fixed-point works great for single-currency systems. But ai-nrf1 is
domain-agnostic — different domains need different precisions. Decimal
strings preserve arbitrary precision without forcing a universal scale.

**Q: Can I store `NaN` or `Infinity`?**
A: No. These are IEEE 754 concepts. If you need to represent "no value",
use `Null`. If you need to represent "unbounded", use a sentinel string
like `"inf"` with domain-specific semantics.

**Q: What about arithmetic?**
A: ai-nrf1 is a **serialization and proof format**, not a computation
engine. Do your math in your language's native types, then convert the
result to a canonical string before encoding.

**Q: Performance impact?**
A: String comparison is slower than integer comparison, but ai-nrf1's
bottleneck is never arithmetic — it's hashing and I/O. The determinism
guarantee is worth orders of magnitude more than nanoseconds of comparison.
