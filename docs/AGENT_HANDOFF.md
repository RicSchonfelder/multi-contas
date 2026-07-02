# AGENT_HANDOFF — Estado atual e transferência entre agentes

> Leia este arquivo PRIMEIRO ao retomar o trabalho. Depois: `docs/SPEC_SOURCE.md` (especificação mestre), `docs/DECISIONS.md`, `docs/adr/`.

## Estado atual (2026-07-01)

- **Fase:** F0 — Descoberta e fundação (em andamento).
- **Branch:** `feat/foundation` (main vazia; nunca trabalhar direto na main).
- **Repositório:** monorepo scaffolded (apps/, packages/, infrastructure/, docs/). Nenhum código de produto ainda — apenas documentação, por exigência da F0.

## O que já foi feito

| Item | Responsável | Status |
|------|-------------|--------|
| Auditoria do diretório (estava vazio) | Orquestrador | ✅ |
| git init + branch feat/foundation + estrutura do monorepo | Orquestrador | ✅ |
| docs/SPEC_SOURCE.md (spec mestre condensada) | Orquestrador | ✅ |
| PRODUCT_VISION, REQUIREMENTS, ROADMAP, EXECUTION_PLAN, BILLING_RULES | Agente Produto | ver tabela de handoffs |
| ADR-001 (desktop), ADR-002 (backend), ARCHITECTURE, DESKTOP_ARCHITECTURE, SYNC_PROTOCOL, API_SPEC | Agente Arquitetura | ver tabela de handoffs |
| SECURITY_MODEL, THREAT_MODEL, LGPD | Agente Segurança | ver tabela de handoffs |
| DATA_MODEL (ERD + dicionário) | Agente Dados | ver tabela de handoffs |
| TEST_STRATEGY, DEPLOYMENT | Agente QA/DevOps | ver tabela de handoffs |

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
