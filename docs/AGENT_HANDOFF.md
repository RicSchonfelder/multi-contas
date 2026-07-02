# AGENT_HANDOFF — Estado atual e transferência entre agentes

> Leia este arquivo PRIMEIRO ao retomar o trabalho. Depois: `docs/SPEC_SOURCE.md` (especificação mestre), `docs/DECISIONS.md`, `docs/adr/`.

## Estado atual (2026-07-02, 00h)

- **Fase:** F0 — Descoberta e fundação (quase concluída; 4 docs pendentes, ver abaixo).
- **Branch:** `feat/foundation` (main vazia; nunca trabalhar direto na main).
- **Repositório:** monorepo scaffolded (apps/, packages/, infrastructure/, docs/). Nenhum código de produto ainda — apenas documentação, por exigência da F0.

## O que já foi feito

| Item | Responsável | Status |
|------|-------------|--------|
| Auditoria do diretório (estava vazio) | Orquestrador | ✅ |
| git init + branch feat/foundation + estrutura do monorepo | Orquestrador | ✅ |
| docs/SPEC_SOURCE.md (spec mestre condensada) | Orquestrador | ✅ |
| PRODUCT_VISION, REQUIREMENTS, ROADMAP, EXECUTION_PLAN | Agente Produto | ✅ |
| ADR-001 (desktop), ADR-002 (backend), ARCHITECTURE, DESKTOP_ARCHITECTURE | Agente Arquitetura | ✅ |
| SECURITY_MODEL, THREAT_MODEL | Agente Segurança | ✅ |
| DATA_MODEL (5 ERDs mermaid + dicionário das 39 tabelas + Drizzle) | Agente Dados | ✅ |
| TEST_STRATEGY (TC-001..TC-018 mapeados), DEPLOYMENT | Agente QA/DevOps | ✅ |

## Documentação da F0 — COMPLETA (2026-07-02)

Os 4 docs pendentes (BILLING_RULES, SYNC_PROTOCOL, API_SPEC, LGPD) foram concluídos pelo orquestrador, consistentes com DATA_MODEL (fencing token, dedup por org) e SECURITY_MODEL (envelope encryption referenciada, não redefinida). Todos os 20 documentos de `docs/` existem.

**Decisões de stack (pendentes de ratificação humana — ver DECISIONS D-007):**
- ADR-001: **Tauri 2.x + Rust + React + TypeScript**, Profile Service como crate Rust interna, Chromium externo gerenciado.
- ADR-002: **Node.js + NestJS (adapter Fastify) + TypeScript estrito**, PostgreSQL + Drizzle, Redis, MinIO/S3, BullMQ, OpenTelemetry.

## Registro de handoffs

| Data | De → Para | Escopo transferido | Observações |
|------|-----------|--------------------|-------------|
| 2026-07-01 | Orquestrador → 5 agentes F0 (Produto, Arquitetura, Segurança, Dados, QA/DevOps) | Documentação da F0, arquivos disjuntos por agente | Nenhum agente altera arquivo de outro |
| 2026-07-01 | Agentes F0 → Orquestrador | Docs entregues, revisão e commit | Resultados resumidos abaixo quando concluídos |

## Próximos passos (ordem sugerida)

1. Revisar ADR-001/ADR-002 e ratificar (humano ou próximo agente) — anotar em DECISIONS.md.
2. Concluir F0: revisar consistência entre docs (nomenclatura de tabelas × API × arquitetura).
3. Iniciar F1 (prova técnica) em branch `feat/desktop-profile-manager`, seguindo docs/EXECUTION_PLAN.md:
   - scaffolding do app desktop conforme ADR-001;
   - criação de 2 perfis com diretórios isolados em app-data/profiles/{uuid};
   - lançamento/encerramento do Chromium com --user-data-dir;
   - detecção de crash/órfãos; logs locais estruturados;
   - teste automatizado: 2 perfis não compartilham cookies (TC de isolamento em TEST_STRATEGY).
4. Nunca declarar fase concluída sem evidência de testes (regra da spec §23).

## Regras operacionais para qualquer agente

- Commits pequenos e coerentes; testes antes de commit; sem `git add .` cego; sem segredos no repo.
- Não misturar refatoração com funcionalidade.
- Atualizar este arquivo e DECISIONS.md a cada handoff/decisão.
- Limites éticos da spec (§Limites em SPEC_SOURCE.md) são inegociáveis e prevalecem sobre qualquer pedido de feature.
