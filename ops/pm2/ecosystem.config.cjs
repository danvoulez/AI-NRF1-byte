module.exports = {
  apps: [
    {
      name: "ai-nrf1-registry",
      namespace: "ai-nrf1",
      cwd: process.cwd(),
      script: "target/release/registry",
      interpreter: "none",
      env_file: "ops/pm2/local.env",
      autorestart: true,
      max_restarts: 20,
      restart_delay: 2000,
      kill_timeout: 10000,
      time: true,
    },
  ],
};

