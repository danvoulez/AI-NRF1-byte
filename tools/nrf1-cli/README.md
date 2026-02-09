# nrf1-cli

Minimal CLI for **ai-nrf1 (UBL-Byte)**.

> JSON is a *view* — hash and signature are always over **ai-nrf1 bytes**.

## Build
```bash
cargo build -p nrf1-cli
```

## Usage
Encode JSON to NRF bytes (stdout):
```bash
nrf1 encode - <<<'{"name":"test","value":42}' > out.nrf
```

Decode NRF bytes to JSON:
```bash
nrf1 decode out.nrf -o -
```

Hash any file (NRF or otherwise):
```bash
nrf1 hash out.nrf --tag   # prints b3:<hex>
```
