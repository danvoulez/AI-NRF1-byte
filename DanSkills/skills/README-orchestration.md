# Orchestration Skills
Playbooks de orquestração **de módulos** e **multi‑produtos** (agnósticos).

- `orchestration-modules/` — sweeps/coordenadas em módulos/serviços, com canário, rate‑limit e gates.
- `orchestration-multiproduct/` — release train multi‑produto com flags, compatibilidade e comunicação.

Use com o kit de 3 agentes (Builder/Sentinel/Reviewer). Aponte `REVIEWER_CMD` para ler o SKILL.md certo conforme o tipo de PR/release.
