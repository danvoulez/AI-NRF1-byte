// PM2 ecosystem config for AI-NRF1-byte registry
// Usage:
//   cargo build --release -p registry --features modules
//   pm2 start ecosystem.config.js
//   pm2 logs registry
//   pm2 restart registry
//   pm2 stop registry

module.exports = {
  apps: [
    {
      name: "registry",
      script: "./target/release/registry",
      cwd: __dirname,
      interpreter: "none", // standalone binary, not Node
      env: {
        RUST_LOG: "info,registry=debug,tower_http=debug",
        PORT: "4000",
        STATE_DIR: "~/.ai-nrf1/state",
        LEDGER_DIR: "~/.ai-nrf1/ledger",
        // API_KEYS: "tdln:sk_your_key_here", // uncomment for prod
        RATE_LIMIT_RPM: "120",
      },
      env_production: {
        RUST_LOG: "info,registry=info",
        PORT: "4000",
        STATE_DIR: "/var/lib/ai-nrf1/state",
        LEDGER_DIR: "/var/lib/ai-nrf1/ledger",
        // API_KEYS: "tdln:sk_prod_key,acme:sk_acme_key",
        RATE_LIMIT_RPM: "300",
      },
      instances: 1,
      autorestart: true,
      watch: false,
      max_memory_restart: "512M",
      error_file: "~/.pm2/logs/registry-error.log",
      out_file: "~/.pm2/logs/registry-out.log",
      log_date_format: "YYYY-MM-DD HH:mm:ss Z",
      // Weekly ledger compression cron (Sundays at 03:00)
      cron_restart: "0 3 * * 0",
    },
    {
      name: "registry-compress",
      script: "./tools/compress-ledger.sh",
      cwd: __dirname,
      interpreter: "/bin/bash",
      cron_restart: "0 3 * * 0", // Sundays at 03:00
      autorestart: false,
      watch: false,
      env: {
        LEDGER_DIR: "~/.ai-nrf1/ledger",
      },
      env_production: {
        LEDGER_DIR: "/var/lib/ai-nrf1/ledger",
      },
    },
  ],
};
