# Verify Checklist (Release Gate)

- [ ] Specs present and referenced:
  - [ ] `specs/ai-nrf1-core.md`
  - [ ] `specs/ai-nrf1-mapping-json.md`
  - [ ] `specs/security-considerations.md`
  - [ ] `specs/ai-nrf1-llm-guide.md`
- [ ] README has tagline + differentiation section
- [ ] Conformance vectors wired in tests (valid/invalid)
- [ ] CLI `ainrf1 verify` enforces:
  - [ ] Hash over NRF bytes only (magic + value)
  - [ ] NFC/BOM/string validations
  - [ ] Minimal varint32, duplicate keys, sort order
- [ ] Bundle verifier runs offline and fails on policy/runtime mismatch
- [ ] SBOM generated and attached to release
