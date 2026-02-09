# ai-nrf1 Error Codes — Cross-Language Reference

Every implementation of ai-nrf1 (Rust, Python, JS/WASM, future) MUST use
these exact error code strings. This ensures that debugging across the stack
is possible — an LLM agent, a human, or a log parser can match errors
regardless of which language produced them.

## Canonical Error Codes

### Decoder Errors (NRF binary → Value)

| Code | Meaning |
|------|---------|
| `InvalidMagic` | First 4 bytes are not `nrf1` (0x6E726631) |
| `InvalidTypeTag(0xNN)` | Unknown type tag byte |
| `NonMinimalVarint` | varint32 uses more bytes than necessary |
| `UnexpectedEOF` | Input ends before expected payload |
| `InvalidUTF8` | String bytes are not valid UTF-8 |
| `NotNFC` | String is not in Unicode NFC normal form |
| `BOMPresent` | String contains U+FEFF (Byte Order Mark) |
| `NonStringKey` | Map key is not a String (tag ≠ 0x04) |
| `UnsortedKeys` | Map keys are not sorted by UTF-8 bytes |
| `DuplicateKey` | Map contains duplicate key |
| `TrailingData` | Extra bytes after the root value |
| `DepthExceeded` | Nesting exceeds implementation limit (≥256) |
| `SizeExceeded` | Total size exceeds implementation limit |

### Encoder Errors (Value → NRF binary)

| Code | Meaning |
|------|---------|
| `Float` | Floating point number encountered (forbidden) |
| `NotNFC` | String is not NFC (encoder should normalize, not reject) |
| `BOMPresent` | String contains BOM (always rejected) |
| `NonStringKey` | Map key is not a string |

### Hex Utilities

| Code | Meaning |
|------|---------|
| `HexOddLength` | Hex string has odd number of characters |
| `HexUppercase` | Hex string contains uppercase A-F |
| `HexInvalidChar` | Hex string contains non-hex character |

### Validation Utilities

| Code | Meaning |
|------|---------|
| `NotASCII` | String contains non-ASCII characters (DID/KID fields) |

### ρ (Rho) Normalization Errors

| Code | Meaning |
|------|---------|
| `Rho.InvalidUTF8` | String cannot be NFC-normalized |
| `Rho.InvalidDecimal(detail)` | Decimal string fails canonical form |
| `Rho.InvalidTimestamp(detail)` | Timestamp string fails RFC-3339 UTC Z |

## Rules

1. Error codes are **string constants**, not numeric.
2. Parameterized codes use parentheses: `InvalidTypeTag(0x08)`.
3. Every implementation MUST produce the same code string for the same error.
4. Error codes are stable — removing or renaming a code requires a major version bump.
5. Implementations MAY add language-specific context (stack trace, file path)
   but the **code string** must be extractable and identical.

## Implementation Status

| Language | Core Decoder | Hex Utils | ρ Errors | ASCII |
|----------|-------------|-----------|----------|-------|
| Rust (`nrf-core`) | ✅ All | ✅ All | ✅ All | ✅ |
| Python (`nrf_core_ref`) | ✅ All | ✅ All | ❌ None | ✅ |
| JS/WASM (`ai-nrf1-wasm`) | ✅ via Rust | ✅ via Rust | ✅ via Rust | ✅ via Rust |
