# ADR-002 — Stack do Backend (Cloud API + Worker)

## Status

**Aceito** — 2026-07-01

## Contexto

O backend do Browser Workspace (apps `/apps/api` e `/apps/worker`) precisa de:

- API REST `/api/v1` com OpenAPI gerado do código, WebSocket (eventos de perfis/leases/sync), OIDC/OAuth2.1, RFC 7807, idempotency keys, rate limiting, webhooks assinados.
- PostgreSQL (multi-tenancy + RLS adicional), Redis (leases, cache, rate limit, pub/sub), S3-compatible (MinIO em dev) para objetos de sync, fila de jobs (sync, snapshots, webhooks, e-mails, retenção).
- Lógica sensível: lease distribuído com heartbeat, protocolo de sync com manifests/chunks, EntitlementService, auditoria imutável, envelope encryption (KEK/DEK).
- OpenTelemetry ponta a ponta.
- Contexto do monorepo: desktop (React/TS) e web-admin (React/TS) consomem contratos compartilhados em `/packages/contracts`.

## Opções

1. **Node.js + NestJS** (TypeScript, framework opinativo, DI, módulos)
2. **Node.js + Fastify** (TypeScript, minimalista, plugins)
3. **Go** (chi/echo + bibliotecas; fila River sobre PostgreSQL)

## Comparativo

| Critério | NestJS (Node/TS) | Fastify (Node/TS) | Go |
|---|---|---|---|
| **Compartilhamento de contratos com desktop/web (TS)** | Nativo: Zod/TypeBox + `@nestjs/swagger` ou ts-rest no `/packages/contracts`, tipados fim a fim | Nativo: TypeBox/Zod → JSON Schema → OpenAPI; mesma vantagem | Requer geração cruzada (OpenAPI → tipos TS) — funciona, mas contratos deixam de ser source-of-truth em TS e viram artefato gerado |
| **OpenAPI gerado do código** | Excelente (decorators + CLI plugin) | Excelente (schemas são nativos do framework) | Bom (oapi-codegen/ogen — geralmente *code from spec*, invertendo o fluxo) |
| **Performance/latência** | Boa (Fastify como adapter) | Muito boa | Excelente (2–5× em CPU-bound; melhor p99, sem GC pauses relevantes) |
| **Concorrência (heartbeats, WebSocket em massa, sync)** | Event loop: ótimo para I/O-bound, cuidado com CPU-bound (checksums!) → delegar a workers | Idem | Goroutines: superior para dezenas de milhares de conexões e trabalho misto CPU/I/O |
| **Fila** | BullMQ (Redis) — maduro, dashboards prontos | BullMQ | River (PostgreSQL) — transacional com o banco, um sistema a menos de consistência |
| **OIDC/OAuth2.1, Argon2id, TOTP** | Ecossistema maduro (openid-client, node-argon2, otplib) | Idem | Maduro (coreos/go-oidc, alexedwards, pquerna/otp) |
| **OpenTelemetry** | Auto-instrumentação ampla | Idem | Instrumentação mais manual, porém estável |
| **DI / arquitetura em módulos multi-time** | Forte e imposta pelo framework | Por convenção (mais disciplina manual) | Por convenção (interfaces + wire/fx) |
| **Velocidade de entrega no nosso monorepo** | Alta | Alta (um pouco mais de setup) | Média (mais boilerplate, segunda linguagem no time full-stack) |
| **Uso de memória/custo infra** | Médio | Médio | Baixo |
| **Risco de talento/contratação** | Baixo (TS em todo o produto) | Baixo | Médio (Go só no backend) |

## Decisão

**Node.js + NestJS (com adapter Fastify), TypeScript estrito, em `/apps/api` e `/apps/worker`.**

Stack completa:

| Preocupação | Escolha |
|---|---|
| Framework HTTP | NestJS 10+ sobre **adapter Fastify** |
| Validação/contratos | **Zod** em `/packages/contracts` (source of truth) + `nestjs-zod`; OpenAPI 3.1 gerado do código |
| Banco | PostgreSQL 16 + **Drizzle ORM** (migrations SQL versionadas, reversíveis) em `/packages/database`; RLS como camada adicional |
| Cache/lease/rate limit | Redis 7 (leases com `SET NX PX` + scripts Lua para renovação atômica; fonte de verdade do lease no PG — ver ARCHITECTURE.md) |
| Objetos | S3-compatible (MinIO dev, S3/R2 prod) via `@aws-sdk/client-s3` |
| Fila | **BullMQ** (Redis) — jobs de sync, webhooks, e-mails, retenção, snapshots |
| WebSocket | Gateway WS do Nest (adapter `ws`) com autenticação por token curto + canais por org |
| AuthN | OIDC/OAuth 2.1 próprio: Argon2id, TOTP, refresh rotation com detecção de reuso; futuro SSO via openid-client |
| Observabilidade | OpenTelemetry SDK (traces/métricas/logs), pino → OTLP |
| CPU-bound (checksums, compressão) | Nunca no event loop da API: no worker, com `worker_threads`/bindings nativos (blake3, zstd via N-API) |

### Justificativa

1. **Contratos compartilhados são o fator dominante.** Desktop (React/TS no Tauri — ADR-001), web-admin (React/TS) e SDK público TS consomem os mesmos schemas Zod de `/packages/contracts`. Com Node, o contrato é código-fonte único, tipado, validado em runtime nas duas pontas. Com Go, vira pipeline de geração e drift possível — atrito diário no monorepo.
2. **O backend é I/O-bound.** CRUD multi-tenant, orquestração de sync (o peso real fica no S3 e no cliente), leases em Redis, webhooks. O perfil de carga favorece o event loop; os pontos CPU-bound (verificação de checksums, assinaturas HMAC em massa) vão para o worker com threads/bindings nativos.
3. **Velocidade de entrega com guard-rails**: NestJS impõe módulos, DI e testabilidade — útil quando F3–F6 adicionarem sync, orgs, API pública e billing em sequência. Fastify puro seria ~equivalente em performance, mas exigiria reconstruir convenções que o Nest já dá (guards de permissão, interceptors de auditoria, pipes de validação) — e o Nest roda *sobre* Fastify, então não há perda.
4. **Avaliação honesta do Go**: é a melhor engenharia bruta da lista — p99 melhor, memória menor, goroutines ideais para heartbeat/WS em massa, River elimina uma classe de inconsistência fila↔banco. Se este backend fosse um produto isolado ou o time fosse Go-first, seria a escolha. Perde aqui porque o custo sistêmico (segunda linguagem, contratos gerados em vez de compartilhados, dois ecossistemas de tooling/lint/test no monorepo) supera os ganhos numa escala que o MVP e F3–F6 não alcançam.

### Trade-offs assumidos

- **Performance-teto menor que Go**: aceito; escala-se horizontalmente e o gargalo real do sync é storage/rede, não CPU da API.
- **BullMQ acopla jobs ao Redis** (menos transacional que River/PG): mitigado com outbox pattern para eventos críticos (auditoria, webhooks) — job só é enfileirado após commit no PG.
- **Disciplina anti-CPU-no-event-loop** precisa ser regra de revisão de código.

## Consequências

**Positivas**
- Um único stack TS do desktop ao backend; refactors de contrato quebram build em todas as pontas (segurança de tipo cross-app).
- OpenAPI e SDK TS público gerados dos mesmos schemas Zod — zero drift.
- Time full-stack único no MVP.

**Negativas**
- Node exige atenção a memória/GC em workers de sync com muitos chunks (streaming obrigatório, nunca buffers inteiros).
- Se automação (F5) ou fan-out de WS crescerem além do previsto, um serviço satélite em Go/Rust pode ser extraído — a fronteira por fila/eventos já permite isso sem redesign.

## Alternativas rejeitadas

- **Fastify puro**: rejeitado por pouco — mesma base de runtime; abriríamos mão da estrutura (DI, guards, módulos) que reduz risco com múltiplos domínios (auth, sync, billing, audit) crescendo em paralelo. Fica registrado que o adapter Fastify nos dá a performance dele dentro do Nest.
- **Go**: rejeitado para o núcleo pelo custo de contratos e de segunda linguagem no monorepo TS (ver justificativa 4). Reavaliar se surgir componente com perfil de carga extremo (gateway de WS, motor de automação) — candidato natural a serviço satélite em Go.
