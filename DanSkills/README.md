# Kit: 3 agentes com semáforo — **100% configurável por ENV**

Este pacote implementa três agentes (BUILDER, SENTINEL, REVIEWER) coordenados por um **semáforo** de arquivo.
Tudo é configurável via **variáveis de ambiente** e/ou arquivo `.env` (já incluído como `.env.example`).

## Como começar
1. **Copie** o conteúdo deste kit para a raiz do seu repo.
2. **Duplique** `.env.example` como `.env` e ajuste valores.
3. Dê permissão:
   ```bash
   chmod +x traffic/trafficctl.sh traffic/lib.sh agents/*.sh scripts/*.sh
   ```
4. Inicie o semáforo e rode os agentes (três terminais):
   ```bash
   ./traffic/trafficctl.sh init
   ./agents/builder.sh
   ./agents/sentinel.sh
   ./agents/reviewer.sh
   ```

## Principais variáveis (ver `.env.example`)
- **Fluxo e turnos**
  - `AGENT_ORDER` (ex.: `BUILDER,SENTINEL,REVIEWER`)
  - `TRAFFIC_INITIAL` (quem começa)
  - `TRAFFIC_STATE_FILE`, `SEM_LOCK_DIR`, `AGENT_SLEEP_INTERVAL`
- **Builder**
  - `BUILDER_PRE_CMD`, `BUILDER_CMD`, `BUILDER_POST_CMD`
  - `BUILDER_GIT_ADD`, `BUILDER_GIT_COMMIT_MSG`, `BUILDER_GIT_PUSH`, `BUILDER_GIT_BRANCH`
- **Sentinel**
  - `TEST_CMD` ou `SENTINEL_TEST_CMD`
  - `MIN_COVERAGE`
  - `SENTINEL_SEMGREP_CONFIG`, `SENTINEL_SEMGREP_FLAGS`
  - `SENTINEL_OPA_QUERY`, `SENTINEL_OPA_INPUT`, `SENTINEL_OPA_DATABROKER`
- **Reviewer**
  - `REVIEWER_CMD` (se quiser acoplar Claude/Copilot/Cursor/Script próprio)
  - `REQUIRED_FN_PREFIX` (ex.: `bill_`)

## Dicas
- Você pode montar `REVIEWER_CMD` chamando seu agente favorito (Claude/Copilot) e salvando o relatório em `review_report.md`.
- Para multi-repo, rode o semáforo em um runner por repo **ou** crie um script que itera repos em canários.

## CI (GitHub Actions)
- Sentinel e Reviewer puxam configurações de **Repository Variables** (em `Settings > Variables > Actions`).
- Você pode sobrepor com `.env` local para desenvolvimento.


## Turbo (opcional)
- `scripts/bootstrap_tools.sh` instala **pytest/pytest-cov**, **Semgrep** e **OPA** (se possível) para dev local ou runners self-hosted.
- Os workflows já usam **concurrency** para evitar jobs duplicados em um mesmo PR.
- Para comentários no PR, o Reviewer usa `gh pr comment -F review_report.md --create-if-none`.

## Mapping ENV → GitHub Variables
- No CI, você pode definir `MIN_COVERAGE`, `TEST_CMD`, `SENTINEL_SEMGREP_*`, `SENTINEL_OPA_*`, `REVIEWER_CMD`, `REQUIRED_FN_PREFIX` em **Settings → Variables → Actions** do repositório/organização.
- Localmente, use `.env` para os mesmos nomes.

