#!/usr/bin/env bash
# Carrega variáveis de ambiente de um arquivo .env (se existir).
# Procura .env na raiz do projeto, no diretório atual e no diretório do script.
set -o allexport
if [ -f ".env" ]; then source ".env"; fi
# tenta raiz do repo (um nível acima de traffic/agents)
if [ -f "$(dirname "$0")/../.env" ]; then source "$(dirname "$0")/../.env"; fi
# tenta raiz 2 níveis acima (caso scripts dentro de subpastas)
if [ -f "$(dirname "$0")/../../.env" ]; then source "$(dirname "$0")/../../.env"; fi
set +o allexport
