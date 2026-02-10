module.exports = {
  apps: [
    {
      name: "ai-nrf1-tunnel",
      namespace: "ai-nrf1",
      cwd: process.cwd(),
      script: "cloudflared",
      interpreter: "none",
      args: ["tunnel", "--config", "ops/cloudflare/cloudflared.config.yml", "run"],
      autorestart: true,
      max_restarts: 50,
      restart_delay: 2000,
      kill_timeout: 10000,
      time: true,
    },
  ],
};

