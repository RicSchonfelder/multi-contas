# ROADMAP — Browser Workspace

> Derivado de `docs/SPEC_SOURCE.md` (seção "Fases"). Estimativas por complexidade, não por horas:
> **P** = pequena (dias) · **M** = média (1–2 semanas) · **G** = grande (2–5 semanas) · **GG** = muito grande (> 5 semanas).
> Estimativas assumem 1–2 agentes/devs trabalhando em paralelo; são relativas, para priorização.
> Requisitos referenciados em `docs/REQUIREMENTS.md`. Plano detalhado de F0/F1 em `docs/EXECUTION_PLAN.md`.

## Visão geral

```
F0 ──► F1 ──► F2 ──► F3 ──► F4 ──► F5 ──► F6 ──► F7
Descoberta  Prova  MVP     Cloud   Times  API/   Monet.  Produção
            técn.  indiv.  +sync          Autom.
```

O **MVP comercial (Windows)** corresponde a F1+F2 completos + subconjuntos de F3 (sync experimental, backend básico) e F4 (1 org, 2 papéis, painel admin mínimo), validado pelos 15 critérios MVP-CA-01..15.

| Fase | Nome | Complexidade | Depende de |
|---|---|---|---|
| F0 | Descoberta | M | — |
| F1 | Prova técnica local | M | F0 (ADR-001) |
| F2 | MVP individual | GG | F1 |
| F3 | Cloud e sync | GG | F0 (ADR-002, ERD), F2 |
| F4 | Equipes e permissões | G | F3 |
| F5 | API pública e automação | G | F4 |
| F6 | Monetização | G | F4 (F5 desejável) |
| F7 | Produção | G | F2–F6 (tudo que for a produção) |

---

## F0 — Descoberta (M)

**Objetivo:** decidir antes de construir. Sem código além de scaffolding.

**Entregáveis**
- Inventário de requisitos (`REQUIREMENTS.md`) e visão (`PRODUCT_VISION.md`)
- `ROADMAP.md`, `EXECUTION_PLAN.md`, `BILLING_RULES.md` (preliminar)
- ADR-001 desktop stack (Tauri+Rust vs Electron vs React+daemon Go) — comparando memória, isolamento, atualização, assinatura, extensões, multiplataforma, controle de processos, debugging, segurança, distribuição, complexidade
- ADR-002 backend (NestJS/Fastify vs Go) + PostgreSQL, Redis, S3-compat, filas, WebSocket, OpenAPI, OIDC/OAuth2.1, OpenTelemetry
- ERD completo (tabelas mínimas da spec) — antes de qualquer migration
- Threat model inicial
- Registro de riscos; definição de MVP; estimativas
- Scaffolding do monorepo (/apps, /packages, /infrastructure) sem lógica de negócio

**Critérios de saída**
1. ADR-001 e ADR-002 escritos, com decisão registrada e trade-offs comparados conforme spec.
2. ERD revisado cobrindo todas as tabelas mínimas da spec (seção "Banco").
3. Threat model cobre: cross-tenant, segredos, sync, lease, atualização, IPC local.
4. Documentos de produto (visão, requisitos, roadmap, execução, billing preliminar) revisados.
5. Monorepo compila/linta vazio no CI.

---

## F1 — Prova técnica local (M)

**Objetivo:** provar o núcleo de risco — perfis Chromium isolados gerenciados por um app desktop.

**Entregáveis** (detalhados em `EXECUTION_PLAN.md`)
- App desktop mínimo (stack do ADR-001)
- 2 perfis com diretórios isolados (`/app-data/profiles/{uuid}/`)
- Abrir/encerrar Chromium por perfil; detecção de processo e de crash
- Persistência de sessão entre execuções; crash recovery; kill de órfãos
- Logs estruturados locais (sem segredos)

**Critérios de saída**
1. **Critério de sucesso da spec:** 2 perfis não compartilham cookies/cache/storage — verificado por teste automatizado (login em site A no perfil 1 não aparece no perfil 2).
2. Sessão persiste após fechar/reabrir app e Chromium (MVP-CA-01).
3. Kill do Chromium e kill do app não corrompem perfil; próxima abertura recupera (MVP-CA-05).
4. Nenhum processo órfão após encerramento normal (MVP-CA-04); órfãos de crash são detectados e limpos.
5. Logs auditáveis do ciclo abrir→monitorar→encerrar, sem qualquer segredo (MVP-CA-06).

**Dependências:** ADR-001 decidido (F0). Sem backend — tudo local.

---

## F2 — MVP individual (GG)

**Objetivo:** produto utilizável por um usuário single-tenant no Windows.

**Entregáveis**
- Auth local do app + storage local seguro (Credential Manager) — RF-026, RF-070..075 (parte local)
- CRUD completo de perfis, pastas, etiquetas (RF-001..012)
- Favoritos e catálogo de extensões autorizadas (RF-040..047)
- Config de proxy corporativo com teste/validação (RF-030..032, RF-034)
- Telas principais do desktop (subconjunto das 21; RF-110..112)
- Diagnóstico; instalador Windows; auto-update assinado (RF-112..115)
- Logs locais estruturados (RF-085)

**Critérios de saída**
1. MVP-CA-01, 02, 03 (local), 04, 05, 06, 08, 12, 13 verificados.
2. Suíte de testes verde no CI (cenários locais: 3, 4, 5, 9, 14, 16).
3. Instalação em máquina limpa validada e documentada.
4. Exclusão de perfil com confirmação e recuperação (MVP-CA-15).

**Dependências:** F1.

---

## F3 — Cloud e sync (GG)

**Objetivo:** backend, contas reais, sincronização criptografada e leases distribuídos.

**Entregáveis**
- API backend (stack do ADR-002), PostgreSQL com migrations do ERD, Redis, S3-compat
- Auth completo (Argon2id, MFA TOTP, sessões, refresh rotation) — RF-070..077, RF-079
- Sync: manifests, checksums, multipart, retomada, conflitos, criptografia por envelope (DEK/KEK) — RF-060..067
- Lease distribuído com heartbeat e recuperação — RF-016..017
- Snapshots e rollback — RF-065
- Gestão de dispositivos — RF-058
- Observabilidade (OpenTelemetry, health checks) — RNF-006
- Docker Compose local completo — RNF-010

**Critérios de saída**
1. Cenários 1, 2, 6, 7, 8, 12, 17, 18 com testes automatizados passando.
2. MVP-CA-03 (distribuído), 07, 09, 14 verificados.
3. Falha total de rede/cloud nunca corrompe perfil local (MVP-CA-14).
4. Documentação honesta do modelo criptográfico publicada (sem E2EE no MVP — RNF-014).

**Dependências:** ADR-002 + ERD (F0); F2 (cliente que consome).

---

## F4 — Equipes e permissões (G)

**Objetivo:** multi-tenancy real e colaboração.

**Entregáveis**
- Organizações, workspaces, equipes, convites — RF-050, RF-054
- 8 papéis + permissões granulares, validação backend — RF-051..053
- Compartilhamento de perfis/redes; encerramento forçado por admin — RF-055, RF-057, RF-017
- Auditoria completa (todos os eventos da spec) + consulta para Auditor — RF-080..083
- Anti cross-tenant com testes dedicados + RLS — RF-056, RNF-009
- Painel web admin mínimo→intermediário — RF-120, RF-125

**Critérios de saída**
1. Cenário 11 (cross-org) com testes negativos passando.
2. Matriz papel×permissão 100% coberta por testes de API.
3. Offboarding de operador (revogação total) funcional e auditado.
4. Trilha de auditoria íntegra com detecção de adulteração (RF-082).

**Dependências:** F3.

---

## F5 — API pública e automação autorizada (G)

**Objetivo:** extensibilidade governada.

**Entregáveis**
- `/api/v1` com OpenAPI, escopos, idempotência, rate limits, webhooks assinados, SDK TS — RF-100..106
- Automação Playwright com allowlist por org, identificação de automação, limites, auditoria de runs, botão de emergência — RF-090..095
- Telas de Automações e chaves de API no desktop/admin

**Critérios de saída**
1. Contrato OpenAPI publicado; testes de contrato verdes; API nunca expõe cookies/senhas/sessões (RF-105).
2. Execução fora de allowlist bloqueada + auditada; sinal de automação verificado presente (RF-091).
3. Botão de emergência interrompe todas as runs da org.

**Dependências:** F4 (escopos e permissões).

---

## F6 — Monetização (G)

**Objetivo:** planos, limites e cobrança. Regras em `BILLING_RULES.md`.

**Entregáveis**
- EntitlementService central (único ponto de verificação) — RF-130
- Planos, limites, medição de uso, assinaturas, faturas — RF-131, RF-133
- Gateway de pagamento abstraído (1 gateway implementado) — RF-132
- Regras de upgrade/downgrade/inadimplência sem perda de dados — RF-134
- Painel financeiro no web admin; tela "Plano e uso" no desktop — RF-121 (parte)

**Critérios de saída**
1. Nenhuma verificação de limite fora do EntitlementService (verificado em code review automatizado).
2. Downgrade/inadimplência jamais apagam dados; comportamento documentado e testado.
3. Ciclo completo: trial → assinar → usar → exceder limite → upgrade → cancelar, testado e auditado.

**Dependências:** F4 (orgs/papéis). F5 desejável (medição de API calls/minutos de automação).

---

## F7 — Produção (G)

**Objetivo:** operação comercial confiável.

**Entregáveis**
- Assinatura de binários em pipeline, instaladores finais, auto-update com canais e rollout gradual — RNF-012, RF-114..116
- Monitoramento/alertas completos, dashboards, SLOs — RNF-006
- Backups, disaster recovery testado, runbooks
- Pentest externo + correções; revisão LGPD final — RNF-001, RNF-004
- Documentação de usuário/admin/API; base de suporte — RF-117
- Ambientes prod isolados; SBOM, secret scanning, audit de deps no release — RNF-010..011

**Critérios de saída**
1. Pentest sem achados críticos/altos abertos.
2. Restore de backup e DR ensaiados com evidência.
3. Todos os 18 cenários de teste da spec verdes no CI.
4. 15 critérios MVP-CA re-verificados no build de produção.
5. Fluxos LGPD (exclusão, exportação, consentimento) operacionais.

**Dependências:** todas as fases que forem a produção (mínimo F2–F4; idealmente F5–F6).

---

## Regras transversais

- **Nunca declarar fase concluída sem evidências** (testes, logs, screenshots, relatórios) — regra explícita da spec.
- Os **limites éticos** (SPEC_SOURCE, "Limites obrigatórios") são gate de saída de TODAS as fases: checklist de revisão confirma que nenhuma feature/copy viola as proibições.
- Prioridade em conflitos de escopo: segurança > isolamento > integridade > conformidade > recuperação > UX > performance > quantidade de features.
