# DECISIONS — Registro de decisões do projeto

> Decisões críticas nunca ficam apenas no histórico do agente. Toda decisão relevante entra aqui (resumo) e, se arquitetural, vira ADR em `docs/adr/`.

| # | Data | Decisão | Motivo | Onde detalhado |
|---|------|---------|--------|----------------|
| D-001 | 2026-07-01 | Nome provisório `Browser Workspace`; nenhuma referência a marcas de concorrentes em código, docs ou UI | Exigência do cliente; risco jurídico/marca | README |
| D-002 | 2026-07-01 | Limites éticos são requisitos de produto: sem evasão de antifraude, sem manipulação de fingerprint, sem export aberto de cookies | Conformidade e política de uso aceitável | docs/SPEC_SOURCE.md §Limites, SECURITY_MODEL |
| D-003 | 2026-07-01 | Monorepo com /apps, /packages, /infrastructure | Contratos compartilhados, CI unificado | docs/ARCHITECTURE.md |
| D-004 | 2026-07-01 | Perfis usam Chromium como PROCESSO EXTERNO com `--user-data-dir` isolado por perfil; a UI do app não embute o navegador | Isolamento real de cookies/cache; independência da stack de UI | ADR-001, DESKTOP_ARCHITECTURE |
| D-005 | 2026-07-01 | MVP: Windows-only, sem E2EE (cifra em repouso + envelope), sync experimental, 1 org, 2 papéis | Escopo realista definido na spec | SPEC_SOURCE §MVP |
| D-006 | 2026-07-01 | Fase 0 executada por multiagentes com arquivos disjuntos; handoffs em AGENT_HANDOFF.md | Regra de coordenação da spec §25 | AGENT_HANDOFF |
| D-007 | 2026-07-01 | Stack desktop e backend: ver ADR-001 e ADR-002 (decisões dos agentes de arquitetura, pendentes de ratificação humana) | Comparação formal exigida pela spec §4 | docs/adr/ |
| D-008 | 2026-07-01 | Segredos do cliente no cofre nativo do SO (Windows Credential Manager no MVP); nunca dentro do diretório do Chromium | Spec §7 e §12 | SECURITY_MODEL |
| D-009 | 2026-07-02 | Desktop: Tauri 2.x + Rust + React/TS; Profile Service como crate Rust interna (extraível para Windows Service na F7) | Menor footprint, assinatura simples, isolamento; trade-off: velocidade de entrega vs Electron | ADR-001 |
| D-010 | 2026-07-02 | Backend: NestJS (adapter Fastify) + TS estrito; PostgreSQL + Drizzle; Redis; S3/MinIO; BullMQ; OTel | Contratos TS compartilhados no monorepo (desktop/web/SDK) | ADR-002 |
| D-011 | 2026-07-02 | Sync: lease + fencing token global como prevenção primária de conflito; dedup de chunks POR organização; MVP declarado NÃO-E2EE (KEK no KMS da plataforma) | Integridade > economia de storage; honestidade sobre E2EE exigida pela spec §11 | SYNC_PROTOCOL, DATA_MODEL |
| D-012 | 2026-07-02 | Segredos de proxy write-only na API (grava, nunca retorna em claro); dados de cartão só no gateway (tokenização) | Spec §8, §13, §15 | API_SPEC, BILLING_RULES |
| D-013 | 2026-07-02 | Chromium alvo na F1 local: usar a instalação do Google Chrome nativo (%ProgramFiles%/Google/Chrome/Application/chrome.exe) | Já presente na máquina local, evita download lento do runtime Chromium na F1 | docs/EXECUTION_PLAN.md |
| D-014 | 2026-07-02 | Gerenciador de pacotes: pnpm (instalado globalmente via npm) | Alinhado com a especificação e segurança, centralizando lockfiles | docs/SECURITY_MODEL.md |
| D-015 | 2026-07-02 | Dependências de compilação desktop: Rustup + Visual Studio Build Tools 2022 instalados via winget | Necessário para compilar o app Tauri 2.x com target MSVC no Windows | docs/EXECUTION_PLAN.md |

## Como registrar uma nova decisão
1. Adicione linha na tabela (D-XXX, data, decisão, motivo, referência).
2. Se arquitetural/estrutural: crie `docs/adr/ADR-XXX-titulo.md` (Status/Contexto/Decisão/Consequências).
3. Commit dedicado com mensagem `docs: registra D-XXX ...`.
