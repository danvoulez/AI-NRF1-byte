module.exports = {
  apps: [
    {
      name: "ai-nrf1-registry",
      namespace: "ai-nrf1",
      cwd: process.cwd(),
      script: "ops/pm2/run-registry.sh",
      interpreter: "bash",
      autorestart: true,
      max_restarts: 20,
      restart_delay: 2000,
      kill_timeout: 10000,
      time: true,
    },
  ],
};
