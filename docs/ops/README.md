# OPS — Operations & Deployment

> How to run, deploy, and operate the system.

## Documents

| Doc | Topic |
|-----|-------|
| [cli.md](cli.md) | CLI reference (`ubl` commands) |
| [RBAC_MIDDLEWARE.md](RBAC_MIDDLEWARE.md) | Registry RBAC middleware |
| [DEPLOY_S3_MINIO.md](DEPLOY_S3_MINIO.md) | S3/MinIO storage deployment |

## Quick Reference

```bash
# Registry service
LEDGER_DIR=./data cargo run -p registry --features modules

# CLI pipelines
ubl tdln policy --var data=hello
ubl llm engine --var prompt=hello

# Permit management
ubl permit approve --tenant T --ticket ID --role R
ubl permit list --tenant T

# Verify a capsule
ubl verify capsule.json --pk key.pub

# PM2 production deployment
pm2 start ecosystem.config.js
```
