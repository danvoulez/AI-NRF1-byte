# @ubl/ai-nrf1-wasm

WASM bindings for the `ai-nrf1` canonical encoder/decoder/hash.

**Canon lives in Rust. This is the ONLY way to touch canon from JS/TS.**
Never reimplement encode/decode/hash in JavaScript.

## API

| Function | Input | Output | Description |
|----------|-------|--------|-------------|
| `encode(value)` | JS object | `Uint8Array` | JSON → ρ-normalize → NRF bytes |
| `decode(bytes)` | `Uint8Array` | JS object | NRF bytes → JSON view |
| `hashBytes(data)` | `Uint8Array` | `Uint8Array` (32B) | BLAKE3 hash of raw bytes |
| `hashValue(value)` | JS object | `Uint8Array` (32B) | encode + BLAKE3 |
| `canonicalCid(value)` | JS object | `string` | `"b3:<hex>"` CID |
| `verify(bytes)` | `Uint8Array` | `boolean` | Decode + re-encode roundtrip check |
| `normalize(value)` | JS object | JS object | ρ-normalize without encoding |
| `encodeHex(bytes)` | `Uint8Array` | `string` | Lowercase hex |
| `parseHex(hex)` | `string` | `Uint8Array` | Parse lowercase hex |
| `version()` | — | `string` | Package version |

## Build

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build for bundler (webpack/vite)
wasm-pack build --target bundler --release

# Build for Node.js
wasm-pack build --target nodejs --release

# Build for browser (no bundler)
wasm-pack build --target web --release
```

## Usage (Node.js)

```js
const nrf = require('@ubl/ai-nrf1-wasm');

// Encode a JSON value to canonical NRF bytes
const bytes = nrf.encode({ name: "test", value: 42 });

// Decode back
const obj = nrf.decode(bytes);

// Get canonical CID
const cid = nrf.canonicalCid({ name: "test", value: 42 });
// → "b3:..."

// Verify bytes are canonical
const ok = nrf.verify(bytes); // true

// Hash raw bytes
const hash = nrf.hashBytes(bytes); // Uint8Array(32)
```

## Usage (TypeScript / ESM)

```ts
import init, { encode, decode, canonicalCid, verify } from '@ubl/ai-nrf1-wasm';

await init();

const bytes = encode({ name: "test", value: 42 });
const cid = canonicalCid({ name: "test", value: 42 });
const valid = verify(bytes);
```

## Rules

- **Floats are forbidden.** Use decimal strings (`"3.14"`) or Int64.
- **Bytes in JSON** use `"b3:<hex>"` or `"0x<hex>"` prefix convention.
- **Strings are NFC-normalized** and BOM is rejected.
- **Map keys are sorted** by UTF-8 bytes. Null values are stripped by ρ.
