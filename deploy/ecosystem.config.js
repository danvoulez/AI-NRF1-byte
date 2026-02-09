// ==========================================================================
// LAB 512 — PM2 Ecosystem Configuration
//
// The BASE is the kernel. It runs once. Modules are processes that drain
// energy and rules from the installed BASE.
//
// Architecture:
//   ┌─────────────────────────────────────────────┐
//   │  LAB 512 (your machine)                     │
//   │                                             │
//   │  ┌─────────────────────────────────┐        │
//   │  │  BASE (registry service)        │        │
//   │  │  Port 8080                      │        │
//   │  │  - ρ normalization              │        │
//   │  │  - PolicyEngine socket          │        │
//   │  │  - RuntimeAttestation           │        │
//   │  │  - Receipt/Ghost/Permit storage │        │
//   │  │  - Ed25519 signing              │        │
//   │  └──────────┬──────────────────────┘        │
//   │             │ localhost:8080                 │
//   │  ┌──────────┴──────────────────────┐        │
//   │  │  MODULES (future PM2 processes) │        │
//   │  │  - receipt-gateway (built-in)   │        │
//   │  │  - policy packs (future)        │        │
//   │  │  - intake adapters (future)     │        │
//   │  │  - enrichments (future)         │        │
//   │  └─────────────────────────────────┘        │
//   │             │                               │
//   │  ┌──────────┴──────────────────────┐        │
//   │  │  Cloudflare Tunnel              │        │
//   │  │  → passports.ubl.agency         │        │
//   │  └─────────────────────────────────┘        │
//   └─────────────────────────────────────────────┘
//
// Usage:
//   pm2 start deploy/ecosystem.config.js
//   pm2 save
//   pm2 startup
//
// ==========================================================================

module.exports = {
  apps: [
    // -----------------------------------------------------------------
    // BASE — The Registry Service (the kernel)
    //
    // This is the ONE process that owns the canonical pipeline.
    // All modules connect to it. It never goes down.
    // -----------------------------------------------------------------
    {
      name: "base-registry",
      script: "./target/release/registry",
      cwd: __dirname + "/..",
      interpreter: "none",              // it's a compiled binary
      autorestart: true,
      max_restarts: 10,
      restart_delay: 3000,
      watch: false,                     // binaries don't hot-reload
      env: {
        // --- Required ---
        DATABASE_URL: "postgres://localhost:5432/ubl_registry",
        RUST_LOG: "registry=info,axum=info",

        // --- Identity ---
        ISSUER_DID: "did:ubl:lab512",
        CDN_BASE: "https://passports.ubl.agency",

        // --- Signing (generate with: openssl rand -hex 32) ---
        // SIGNING_KEY_HEX: "<your-32-byte-hex-key>",
        // If not set, generates ephemeral key (dev mode)

        // --- Runtime attestation ---
        // BINARY_SHA256: "<sha256-of-registry-binary>",
        // If not set, uses "dev-build-no-hash"

        // --- Port ---
        PORT: "8080",
      },
    },

    // -----------------------------------------------------------------
    // MODULES ARE NOT SEPARATE PROCESSES.
    //
    // The Parent Law (Constitution of Modules, Article VI):
    //   The Base is always the parent. A module is always the child.
    //
    // Rust modules:  compiled into the base-registry binary.
    //                No separate process. No network boundary.
    //
    // Non-Rust modules (future): spawned as CHILD processes BY the Base.
    //                The Base starts them, feeds them, signs for them.
    //                They never hold a signing key, touch the DB, or
    //                open a port. They are guests.
    //
    // There is ONE PM2 process: base-registry. That's the whole cloud.
    // -----------------------------------------------------------------
  ],
};
