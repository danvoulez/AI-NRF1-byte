#!/usr/bin/env python3
"""
Extract code and doc artifacts from MODULOS-INICIO.md into EXTRACTED/ staging folder.

Each file gets a clear name and a MANIFEST.md explains what it is and where it belongs.

Usage:
    python3 scripts/extract_to_staging.py
"""

import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "MODULOS-INICIO.md")
OUT = os.path.join(ROOT, "EXTRACTED")


def read_source():
    with open(SRC, "r") as f:
        return f.readlines()


def extract_code_block(lines, start, end, lang):
    """Extract the LAST fenced code block matching lang in the 1-indexed line range."""
    region = lines[start - 1: end]
    blocks = []
    in_block = False
    current = []
    for line in region:
        stripped = line.rstrip()
        if not in_block and stripped.startswith("```") and lang in stripped:
            in_block = True
            current = []
        elif in_block and stripped == "```":
            in_block = False
            blocks.append("\n".join(current))
        elif in_block:
            current.append(line.rstrip("\n"))
    if not blocks:
        return None
    return blocks[-1] + "\n"


def extract_design_doc(lines):
    """Collect architecture/design sections (non-code) into a clean markdown doc."""
    out = []
    out.append("# Module Architecture — Design Document\n\n")
    out.append("> Extracted from MODULOS-INICIO.md.\n")
    out.append("> Architectural rationale and design decisions for the MODULE phase.\n")
    out.append("> Code lives in separate extracted files.\n\n---\n\n")

    doc_ranges = [
        (1, 57, "Original Vision: 5 Canonical Modules"),
        (59, 231, "Refined Architecture: Capabilities + Orchestrator"),
        (242, 475, "Module Model v0: Traits, Ctx, Contracts"),
        (492, 767, "Refined Model: Effects, Idempotency, Taxonomy"),
    ]
    skip = ["## Prompt:", "## Response:", "Thought for"]

    for start, end, title in doc_ranges:
        out.append(f"## {title}\n\n")
        for line in lines[start - 1: end]:
            if any(p in line for p in skip):
                continue
            out.append(line)
        out.append("\n---\n\n")
    return "".join(out)


def write(rel_path, content, description):
    abs_path = os.path.join(OUT, rel_path)
    os.makedirs(os.path.dirname(abs_path), exist_ok=True)
    with open(abs_path, "w") as f:
        f.write(content)
    size = len(content)
    print(f"  {rel_path:50s} ({size:>5} bytes)  — {description}")
    return (rel_path, size, description)


def main():
    lines = read_source()
    print(f"Source: {len(lines)} lines\n")
    print("=== Extracting to EXTRACTED/ ===\n")

    manifest_entries = []

    # ---------------------------------------------------------------
    # CODE: modules-core (v3 — the Capability contract)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2284, 2364, "rust")
    if c:
        manifest_entries.append(write(
            "code/modules-core--lib.rs", c,
            "Capability trait, CapInput/CapOutput, Verdict, Effect, AssetResolver"))

    # ---------------------------------------------------------------
    # CODE: cap-intake v4 (declarative mapping DSL)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2957, 3077, "rust")
    if c:
        manifest_entries.append(write(
            "code/cap-intake--lib.rs", c,
            "IntakeModule: normalize via dot-path mapping + defaults"))

    # ---------------------------------------------------------------
    # CODE: cap-policy v4 (rule DSL: EXIST/THRESHOLD/RANGE/ALLOWLIST/NOT)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 3397, 3512, "rust")
    if c:
        manifest_entries.append(write(
            "code/cap-policy--lib.rs", c,
            "PolicyModule: DSL with EXIST, THRESHOLD, THRESHOLD_RANGE, ALLOWLIST, NOT"))

    # ---------------------------------------------------------------
    # CODE: cap-enrich v1 (status-page + webhook drivers)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 3642, 3755, "rust")
    if c:
        manifest_entries.append(write(
            "code/cap-enrich--lib.rs", c,
            "EnrichModule: status-page HTML + webhook effect drivers"))

    # ---------------------------------------------------------------
    # CODE: runner (pipeline orchestrator)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2519, 2603, "rust")
    if c:
        manifest_entries.append(write(
            "code/runner--runner.rs", c,
            "Runner: iterates manifest pipeline, calls cap.execute(), collects hop receipts"))

    # ---------------------------------------------------------------
    # CODE: manifest types
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2408, 2436, "rust")
    if c:
        manifest_entries.append(write(
            "code/runner--manifest.rs", c,
            "Manifest + Step: product.json deserialization types"))

    # ---------------------------------------------------------------
    # CODE: cap_registry
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2437, 2468, "rust")
    if c:
        manifest_entries.append(write(
            "code/runner--cap_registry.rs", c,
            "CapRegistry: register capabilities by kind+version, lookup with semver match"))

    # ---------------------------------------------------------------
    # CODE: effects (EffectExecutor trait)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2470, 2486, "rust")
    if c:
        manifest_entries.append(write(
            "code/runner--effects.rs", c,
            "EffectExecutor trait + NoopExecutor"))

    # ---------------------------------------------------------------
    # CODE: assets (MemoryResolver)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2488, 2517, "rust")
    if c:
        manifest_entries.append(write(
            "code/runner--assets.rs", c,
            "MemoryResolver: in-memory AssetResolver for tests"))

    # ---------------------------------------------------------------
    # CODE: finalize (capsule sealing + receipt chaining)
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2605, 2643, "rust")
    if c:
        manifest_entries.append(write(
            "code/runner--finalize.rs", c,
            "finalize_capsule: compute id, seal.sig, chain hop receipts (ASPIRATIONAL — needs real ubl_capsule types)"))

    # ---------------------------------------------------------------
    # JSON: product manifest schema
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 2828, 2857, "json")
    if c:
        manifest_entries.append(write(
            "schemas/product.v1.json", c,
            "JSON Schema for product.json manifests"))

    # ---------------------------------------------------------------
    # JSON: example product manifest
    # ---------------------------------------------------------------
    c = extract_code_block(lines, 3859, 3910, "json")
    if c:
        manifest_entries.append(write(
            "examples/product--api-receipt-gateway.json", c,
            "Example product manifest: intake -> policy -> enrich pipeline"))

    # ---------------------------------------------------------------
    # DOC: design document
    # ---------------------------------------------------------------
    doc = extract_design_doc(lines)
    manifest_entries.append(write(
        "docs/MODULES-DESIGN.md", doc,
        "Architecture rationale: module model, effects, taxonomy, anti-chaos rules"))

    # ---------------------------------------------------------------
    # Write MANIFEST.md
    # ---------------------------------------------------------------
    manifest_lines = ["# EXTRACTED — Staging Manifest\n\n"]
    manifest_lines.append("Files extracted from `MODULOS-INICIO.md` for review before placement.\n\n")
    manifest_lines.append("| File | Bytes | Target Location | Status |\n")
    manifest_lines.append("|------|-------|-----------------|--------|\n")

    placement = {
        "code/modules-core--lib.rs":
            "`crates/modules-core/src/lib.rs` — NEW crate (module contracts)",
        "code/cap-intake--lib.rs":
            "`modules/cap-intake/src/lib.rs` — NEW crate (capability module)",
        "code/cap-policy--lib.rs":
            "`modules/cap-policy/src/lib.rs` — NEW crate (capability module)",
        "code/cap-enrich--lib.rs":
            "`modules/cap-enrich/src/lib.rs` — NEW crate (capability module)",
        "code/runner--runner.rs":
            "`crates/module-runner/src/runner.rs` — NEW crate (NOT crates/runtime which is BASE)",
        "code/runner--manifest.rs":
            "`crates/module-runner/src/manifest.rs` — NEW crate",
        "code/runner--cap_registry.rs":
            "`crates/module-runner/src/cap_registry.rs` — NEW crate",
        "code/runner--effects.rs":
            "`crates/module-runner/src/effects.rs` — NEW crate",
        "code/runner--assets.rs":
            "`crates/module-runner/src/assets.rs` — NEW crate",
        "code/runner--finalize.rs":
            "`crates/module-runner/src/finalize.rs` — NEW crate (ASPIRATIONAL: needs ubl_capsule API)",
        "schemas/product.v1.json":
            "`schemas/product.v1.json` — already exists, UPDATE",
        "examples/product--api-receipt-gateway.json":
            "`products/api-receipt-gateway/product.json` — NEW",
        "docs/MODULES-DESIGN.md":
            "`docs/MODULES-DESIGN.md` — NEW",
    }

    status_map = {
        "code/runner--finalize.rs": "ASPIRATIONAL",
    }

    for rel, size, desc in manifest_entries:
        target = placement.get(rel, "TBD")
        status = status_map.get(rel, "NEEDS POLISH")
        manifest_lines.append(f"| `{rel}` | {size} | {target} | {status} |\n")

    manifest_lines.append("\n## Notes\n\n")
    manifest_lines.append("- **NEEDS POLISH**: Code extracted verbatim from chat. Needs: import fixes, ")
    manifest_lines.append("trait adjustments (associated consts → methods for dyn dispatch), ")
    manifest_lines.append("removal of `#[async_trait]` on non-async traits, Default impls, etc.\n")
    manifest_lines.append("- **ASPIRATIONAL**: References types/APIs that don't exist yet in the codebase ")
    manifest_lines.append("(e.g. `ubl_capsule::Capsule` struct). Keep as reference, don't compile yet.\n")
    manifest_lines.append("- **crates/runtime** (BASE) is UNTOUCHED. The new runner lives in `crates/module-runner`.\n")
    manifest_lines.append("- **crates/modules-core** is a NEW crate — the contract layer between runner and modules.\n")

    write("MANIFEST.md", "".join(manifest_lines), "Staging manifest with placement guide")

    print(f"\n=== Done: {len(manifest_entries) + 1} files in EXTRACTED/ ===")
    print("Next: review EXTRACTED/MANIFEST.md, polish each file, then place.")


if __name__ == "__main__":
    main()
