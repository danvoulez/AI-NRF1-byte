// ==========================================================================
// LAB 512 — PM2 Ecosystem Configuration
//
// ONE binary, TWO layers:
//   cargo build --release -p registry --features modules
//
// Architecture:
//   ┌─────────────────────────────────────────────┐
//   │  LAB 512 (this machine)                     │
//   │                                             │
//   │  ┌─────────────────────────────────┐        │
//   │  │  registry (--features modules)  │        │
//   │  │  Port 8791                      │        │
//   │  │                                 │        │
//   │  │  BASE routes:                   │        │
//   │  │    /health, /v1/receipts, ...   │        │
//   │  │                                 │        │
//   │  │  MODULES routes (feature flag): │        │
//   │  │    /permit/:t/:id/approve       │        │
//   │  │    /permit/:t/:id/deny          │        │
//   │  │    /modules/run                 │        │
//   │  └──────────┬──────────────────────┘        │
//   │             │ 127.0.0.1:8791                │
//   │  ┌──────────┴──────────────────────┐        │
//   │  │  cloudflared tunnel             │        │
//   │  │  → registry.ubl.agency          │        │
//   │  └─────────────────────────────────┘        │
//   └─────────────────────────────────────────────┘
//
// Usage:
//   # First time:
//   bash deploy/go-live.sh
//
//   # Manual:
//   pm2 start deploy/ecosystem.config.js
//   pm2 save
//   pm2 startup
//
// ==========================================================================

const path = require("path");
const ROOT = path.resolve(__dirname, "..");

module.exports = {
  apps: [
    // -----------------------------------------------------------------
    // The ONE binary: BASE + MODULES compiled together.
    //
    // Parent Law (Constitution of Modules, Article VI):
    //   The Base is always the parent. A module is always the child.
    //   Modules are feature flags of the same binary, not separate
    //   processes.
    // -----------------------------------------------------------------
    {
      name: "ai-nrf1",
      script: "./target/release/registry",
      cwd: ROOT,
      interpreter: "none",
      autorestart: true,
      max_restarts: 10,
      restart_delay: 3000,
      watch: false,
      kill_timeout: 10000,
      env_file: path.join(ROOT, ".env"),
      env: {
        PORT: "8791",
        RUST_LOG: "registry=info,axum=info,tower_http=info,module_runner=info",
        ISSUER_DID: "did:ubl:lab512",
        CDN_BASE: "https://registry.ubl.agency",
        STATE_DIR: process.env.HOME + "/.ai-nrf1/state",
      },
    },

    // -----------------------------------------------------------------
    // Cloudflare Tunnel — exposes 127.0.0.1:8791 as registry.ubl.agency
    //
    // Prerequisites:
    //   brew install cloudflared
    //   cloudflared tunnel login
    //   cloudflared tunnel create ai-nrf1
    //   cloudflared tunnel route dns ai-nrf1 registry.ubl.agency
    //   cp ops/cloudflare/cloudflared.config.example.yml ~/.cloudflared/config.yml
    //   # Edit credentials-file path in config.yml
    // -----------------------------------------------------------------
    {
      name: "cloudflared",
      script: "cloudflared",
      args: "tunnel --config " + path.join(ROOT, "ops/cloudflare/cloudflared.config.yml") + " run ai-nrf1",
      interpreter: "none",
      autorestart: true,
      max_restarts: 5,
      restart_delay: 5000,
      watch: false,
    },
  ],
};
