# Conformance Vectors

This folder contains **golden hex** ai-nrf1 streams (include magic `6e726631`).

- `valid/*.hex` — must **round-trip** byte-identically.
- `invalid/*.hex` — must be **rejected** by the decoder.

Source of truth: `specs/ai-nrf1-core.md §8 Test Vectors`.

Naming: **ai-nrf1** = canonical binary format; **ai-json-nrf1** = JSON view.
