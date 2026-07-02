# Browser Workspace (nome provisório)

Plataforma corporativa desktop + cloud para gerenciamento de ambientes de navegador (perfis Chromium) isolados, seguros, auditáveis e colaborativos.

> ⚠️ Este produto NÃO implementa evasão de antifraude, manipulação de fingerprint ou qualquer recurso de contorno de plataformas. Ver [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md) e limites em [docs/SPEC_SOURCE.md](docs/SPEC_SOURCE.md).

## Estrutura

```
/apps          desktop, web-admin, api, worker
/packages      ui, contracts, database, crypto, logging, config, validation,
               browser-core, sync-engine, automation-sdk
/infrastructure docker, terraform, monitoring
/docs          documentação viva do projeto (começar por SPEC_SOURCE.md e EXECUTION_PLAN.md)
```

## Para agentes/desenvolvedores continuando o trabalho

1. Leia `docs/SPEC_SOURCE.md` (especificação mestre condensada).
2. Leia `docs/AGENT_HANDOFF.md` (estado atual e próximos passos).
3. Leia `docs/DECISIONS.md` e `docs/adr/` (decisões tomadas).
4. Trabalhe em branch `feat/*`, nunca na main. Commits pequenos. Testes antes de commit.

## Status

**Fase 0 — Descoberta e fundação** (em andamento). Nenhuma funcionalidade implementada ainda; apenas documentação, ADRs e scaffolding.
