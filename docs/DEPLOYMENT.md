# DEPLOYMENT — DevOps, CI/CD e Releases (Browser Workspace)

> **Status: Implementado (F7 — Produção)**
> Derivado de `docs/SPEC_SOURCE.md` (seções "DevOps", "Atualizações", "Observabilidade", "Fases").
> Documentos operacionais em `docs/runbooks/`. Terraform em `infrastructure/terraform/`.
> Monitoring stack em `infrastructure/monitoring/`. Backups em `scripts/backup.ps1`.

---

## 1. Ambiente local (docker-compose)

Arquivo alvo: `infrastructure/docker/compose.yaml` (+ `compose.test.yml` para CI). Rede interna única; apenas portas listadas expostas no host. Credenciais locais via `.env` (nunca commitado; `.env.example` versionado).

| Serviço | Imagem | Porta(s) host | Função |
|---|---|---|---|
| `api` | build local (`apps/api`) | 3000 | API REST `/api/v1` + WebSocket; health em `/healthz`, `/readyz` |
| `worker` | build local (`apps/worker`) | — | Filas (sync, webhooks, e-mails, limpeza de leases expirados) |
| `postgres` | `postgres:16-alpine` | 5432 | Banco principal; volume nomeado; `POSTGRES_DB=bw_dev` |
| `redis` | `redis:7-alpine` | 6379 | Cache, filas (BullMQ), leases/locks distribuídos |
| `minio` | `minio/minio` | 9000 (S3) / 9001 (console) | Object storage S3-compat (sync objects, snapshots) |
| `mailpit` | `axllent/mailpit` | 8025 (UI) / 1025 (SMTP) | E-mails de dev (verificação, convites, alertas) |
| `grafana` | `grafana/grafana` | 3001 | Dashboards (LGTM) |
| `prometheus` | `prom/prometheus` | 9090 | Métricas (scrape de api/worker via `/metrics`) |
| `tempo` | `grafana/tempo` | 3200 | Traces (OpenTelemetry) |
| `loki` | `grafana/loki` | 3100 | Logs estruturados |
| `otel-collector` | `otel/opentelemetry-collector-contrib` | 4317 (gRPC) / 4318 (HTTP) | Ponto único de ingestão OTLP → Tempo/Loki/Prometheus |

Regras:
- API e worker enviam telemetria **somente** ao otel-collector (OTLP), nunca direto aos backends.
- `depends_on` com healthchecks (PG/Redis/MinIO prontos antes de api/worker).
- Migrations: comando explícito (`pnpm db:migrate`), nunca automático no boot do container em staging/prod (ver §3.4).
- Desktop em dev aponta para `http://localhost:3000` via config; serviço local roda fora do compose (é nativo Windows).

## 2. CI/CD — GitHub Actions

Workflows em `.github/workflows/`. Runners: `ubuntu-latest` (backend/web), `windows-latest` (desktop/E2E/assinatura).

### 2.1 `ci.yml` — todo push/PR

| Job | Conteúdo | Gate |
|---|---|---|
| `lint` | ESLint + Prettier check + commitlint | obrigatório |
| `typecheck` | `tsc --noEmit` por pacote (turbo) | obrigatório |
| `test-unit` | Vitest unit, cobertura por pacote (limiares do TEST_STRATEGY.md §3) | obrigatório |
| `test-integration` | Vitest + testcontainers (PG16/Redis7/MinIO) ou `compose.test.yml` | obrigatório |
| `test-contract` | Validação OpenAPI + schemas `packages/contracts` | obrigatório |
| `e2e-desktop` | Playwright em `windows-latest` (PRs para `main` + nightly) | obrigatório p/ main |
| `audit-deps` | `pnpm audit --prod` + `osv-scanner`; falha em CVE high/critical sem exceção documentada | obrigatório |
| `secret-scan` | **gitleaks** (histórico completo em PRs; push protection habilitado no repo) | obrigatório |
| `sbom` | **syft** → SBOM SPDX + CycloneDX publicados como artifact; anexados a releases | obrigatório em release |
| `build` | `turbo build` de todos os apps/pacotes; artifacts versionados | obrigatório |

### 2.2 `release-backend.yml` — deploy por tag

Disparo: tag `api-vX.Y.Z`. Sequência:
1. CI completo verde (reusa `ci.yml`).
2. Build de imagem Docker → push no registry com tag imutável (`sha` + `semver`); imagem assinada com **cosign** + SBOM anexado.
3. **Migrations gated**: job separado que roda `db:migrate` contra o ambiente-alvo **após aprovação manual** (GitHub Environments com required reviewers). Migrations devem ser reversíveis e compatíveis com a versão anterior (expand/contract) para permitir rollback sem down-migration em produção.
4. Deploy staging → smoke tests automáticos (health, auth, CRUD perfil) → aprovação manual → deploy prod.
5. **Rollback**: redeploy da tag de imagem anterior (imagens imutáveis mantidas ≥ 90 dias); como migrations são expand/contract, código N-1 funciona com schema N.

### 2.3 `release-desktop.yml` — instalador Windows

Disparo: tag `desktop-vX.Y.Z[-beta.N|-internal.N]`. Sequência:
1. CI verde + E2E desktop verde em `windows-latest`.
2. Build do instalador (MSIX/NSIS) em runner Windows.
3. **Assinatura de binários** (ver §5).
4. Teste de instalação em VM limpa (job E2E: instalar → abrir → criar perfil → desinstalar) — critério MVP 12.
5. Publicação do instalador + manifest de update no canal correspondente (bucket versionado por canal).
6. Release notes geradas de conventional commits; SBOM anexado ao GitHub Release.

### 2.4 Segurança do pipeline

- OIDC do GitHub → cloud (sem secrets de longa duração no repo).
- Secrets por **Environment** (dev/staging/prod), com required reviewers em staging/prod.
- Branch protection em `main`: PR + CI verde + 1 review; sem push direto.
- Actions pinadas por SHA; `permissions:` mínimos por job.

## 3. Ambientes

| | dev | staging | prod |
|---|---|---|---|
| Infra | docker-compose local | espelho reduzido da prod (Terraform, `infrastructure/terraform/envs/staging`) | Terraform `envs/prod` |
| Dados | sintéticos/seeds | sintéticos (NUNCA cópia de prod com dados pessoais — LGPD) | reais |
| Credenciais | `.env` local | secret manager, contas próprias | secret manager, contas próprias |
| Deploy | manual | automático em tag, pós-CI | promoção manual da MESMA imagem/instalador validado em staging |
| Acesso | dev team | dev team | restrito + auditado |

Regras: **separação total de credenciais** (nenhum secret compartilhado entre ambientes; bancos, buckets e chaves KEK distintos); promoção **por tag** — o artefato que vai a prod é byte-idêntico ao validado em staging; feature flags para divergência de comportamento, nunca builds diferentes.

## 4. Release do desktop — canais e rollout

| Canal | Público | Cadência | Origem |
|---|---|---|---|
| `internal` | equipe | contínua (toda tag `-internal`) | staging infra |
| `beta` | orgs opt-in | semanal | prod infra, feed beta |
| `stable` | todos | após ≥ 1 semana estável em beta | prod infra, feed stable |

- **Auto-update assinado**: cliente consulta manifest de update via HTTPS (pinning do host); manifest assinado (assinatura verificada antes de qualquer download); binário verificado (assinatura Authenticode + checksum SHA-256 do manifest) antes de aplicar. Falha em qualquer verificação = abort + evento de telemetria.
- **Rollout gradual**: manifest suporta `rolloutPercentage` (1% → 10% → 50% → 100%), decidido por hash estável do device ID; promoção automática só se métricas de crash/erro por versão (OTel) ficarem sob limiar; qualquer degradação → congela rollout.
- **Rollback**: manifest pode apontar versão anterior como "latest"; cliente aceita downgrade assinado apenas quando o manifest marca a versão atual como revogada (proteção contra downgrade attack); binários anteriores permanecem publicados. Atualização nunca migra formato de perfil sem backup pré-migração (spec §Isolamento local).
- Update em uso: nunca atualizar com perfil aberto; agenda para próximo start ou pede encerramento.

## 5. Assinatura de binários Windows

Processo (Azure Trusted Signing — preferido — ou certificado EV em HSM):

1. Certificado de **code signing EV/verified publisher** emitido para a entidade legal do produto; chave privada **nunca** sai do serviço (Azure Trusted Signing) ou do HSM (FIPS 140-2 L2+) — jamais em secrets do GitHub.
2. Job de assinatura em runner Windows autentica via **OIDC federado** (GitHub → Entra ID, sem client secret) com role restrita ao perfil de assinatura.
3. `signtool`/`Invoke-TrustedSigning` assina: executável principal, serviço local, DLLs próprias, instalador e desinstalador; timestamp RFC 3161 obrigatório (validade pós-expiração do cert).
4. Verificação pós-assinatura no pipeline: `signtool verify /pa /all` + execução do instalador em VM limpa sem warning SmartScreen bloqueante.
5. Auditoria: cada assinatura logada (quem/qual tag/qual hash); assinatura só em workflow de release a partir de tag protegida — nunca em PR.
6. Revogação: plano documentado para comprometimento de chave (revogar cert, re-assinar última versão boa, manifest de update força upgrade).

## 6. Atualização do Chromium gerenciado

- **Origem oficial única**: **Chrome for Testing** (`googlechromelabs.github.io/chrome-for-testing` / bucket oficial). Proibido: mirrors, builds de terceiros, download de fontes não verificadas.
- **Verificação em camadas**: (1) HTTPS com validação estrita de certificado; (2) checksum SHA-256 conferido contra manifest interno **assinado por nós** (o pipeline baixa da origem oficial, valida, calcula hash e publica manifest assinado no nosso feed — o cliente só confia no nosso manifest); (3) assinatura do nosso manifest verificada antes de qualquer instalação.
- **Matriz de compatibilidade perfil × versão** (mantida em `packages/browser-core`, validada por teste de migração no CI):

| Versão Chromium | Formato de perfil | Ação ao abrir perfil antigo | Downgrade permitido? |
|---|---|---|---|
| N (atual stable) | vK | abre direto | — |
| N+1 (nova) | vK ou vK+1 | backup pré-migração → migrar → validar | não (Chromium não lê perfil de versão maior) |
| < N mínima suportada | < vK | bloqueado: exigir atualização | n/a |

  Regra dura: **nunca** abrir perfil com Chromium mais antigo que o que o gravou (corrupção silenciosa); o metadado `browser_version` do perfil é verificado no fluxo de abertura (spec §Desktop).
- **Bloqueio de versões inseguras**: lista de versões revogadas (CVE crítico) no manifest assinado; cliente recusa iniciar Chromium revogado e força update; admin da org pode fixar versão apenas dentro da janela suportada.
- Cadência: acompanhar stable do Chrome; janela alvo ≤ 7 dias para CVE explorado ativamente; testar nova versão no canal `internal` → `beta` → `stable` com a mesma mecânica de rollout do §4.
- Cache local de binários por versão (`/app-data/chromium/<versao>/`), com verificação de integridade a cada inicialização (hash do binário vs manifest).

## 7. Observabilidade e operação (resumo)

- OpenTelemetry em api/worker/desktop-serviço; correlation ID ponta a ponta.
- Alertas mínimos: taxa de falha de sync, corrupção detectada, crashes Chromium por versão, latência p95 da API, falhas de auth anômalas, fila atrasada, disco/storage.
- Backups PG: contínuo (PITR) + snapshot diário; restore testado mensalmente em staging (evidência em docs). MinIO/S3: versionamento de bucket + replicação.
- DR: RPO ≤ 1h, RTO ≤ 4h (runbook em `docs/runbooks/DR-RUNBOOK.md`).

## 8. F7 — Produção (Implementado)

### Artefatos criados

| Artefato | Caminho | Descrição |
|----------|---------|-----------|
| CI workflow | `.github/workflows/ci.yml` | Lint + build + cargo check em todo push/PR |
| Deploy workflow | `.github/workflows/deploy.yml` | Deploy da API em push para main |
| Monitoring stack | `infrastructure/monitoring/docker-compose.monitoring.yml` | Grafana + Prometheus + Tempo + node-exporter |
| Prometheus config | `infrastructure/monitoring/prometheus.yml` | Scrape configs para API, Worker, Node, PG, Redis |
| Alerting rules | `infrastructure/monitoring/alerts.yml` | Alertas: downtime, latência, erros, conexões, fila |
| Terraform AWS | `infrastructure/terraform/main.tf` | ECS Fargate, RDS, ElastiCache, S3, ALB, IAM, SSM |
| Terraform vars | `infrastructure/terraform/variables.tf` | Todos os parâmetros configuráveis |
| Terraform outputs | `infrastructure/terraform/outputs.tf` | Endpoints pós-deploy |
| Backup script | `scripts/backup.ps1` | pg_dump + S3 sync + DR upload + retenção 30d |
| DR runbook | `docs/runbooks/DR-RUNBOOK.md` | RTO 4h / RPO 1h, failover + recovery passo a passo |
| Pentest checklist | `docs/runbooks/PENTEST-CHECKLIST.md` | OWASP ASVS L1 — auth, session, access, data, API |

### Pendências (pós-F7)

- [ ] Provisionar Terraform remote state (S3 + DynamoDB)
- [ ] Configurar OIDC GitHub → AWS para pipelines
- [ ] Automatizar failover (Lambda + Terraform)
- [ ] Configurar domínio e certificado ACM no ALB
- [ ] Teste mensal de restore em staging
