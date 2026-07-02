# API_SPEC — API pública /api/v1

> Fase-alvo: F3 (interna) e F5 (pública). OpenAPI será **gerado do código** (NestJS + zod/openapi nos contratos em `/packages/contracts`); este documento define as convenções e o mapa de recursos.

## Convenções

| Tema | Regra |
|------|-------|
| Estilo | REST, JSON UTF-8, recursos no plural, kebab-case em rotas, camelCase em payloads |
| Versionamento | Prefixo `/api/v1`; mudanças breaking → `/v2`; depreciação com header `Deprecation` + `Sunset` e prazo mínimo de 6 meses |
| Autenticação | `Authorization: Bearer` — access token OAuth 2.1 (usuários) ou API key com prefixo identificável `bwk_` (integrações); tokens revogáveis; escopos granulares (ex.: `profiles:read`, `profiles:open`, `audit:read`) espelhando as permissões RBAC |
| Autorização | SEMPRE validada no backend por organização + papel + permissão; nunca confiar em filtro do cliente |
| Erros | RFC 7807 (`application/problem+json`): `type`, `title`, `status`, `detail`, `instance`, `correlationId` |
| Paginação | Cursor-based: `?cursor=...&limit=...` (limit ≤ 100); resposta com `data[]` + `nextCursor` |
| Filtros/ordenação | `?filter[status]=active&sort=-createdAt` |
| Idempotência | Header `Idempotency-Key` obrigatório em POSTs com efeito financeiro/criação; janela de 24h |
| Rate limit | Por token e por org (EntitlementService); headers `RateLimit-*`; 429 com `Retry-After` |
| Webhooks | Assinados com HMAC-SHA256 (`X-Webhook-Signature` + timestamp, tolerância 5 min contra replay); retries com backoff; ver `webhook_deliveries` |
| Correlação | `X-Correlation-Id` aceito/propagado; sempre retornado |

**Nunca expostos pela API:** cookies, senhas, tokens de sessão de navegador, conteúdo de dados de perfil em claro, segredos de proxy (write-only: aceita gravação, retorna sempre mascarado).

## Mapa de recursos (v1)

| Recurso | Rotas principais | Permissão exigida (exemplos) |
|---------|------------------|------------------------------|
| Auth | `POST /auth/login`, `/auth/mfa/verify`, `/auth/refresh`, `/auth/logout`, `POST /auth/password-reset` | pública/sessão |
| Organizações | `GET/PATCH /organizations/{id}`, `GET /organizations` | `org:read`, `org:manage` |
| Usuários e convites | `GET/POST /users`, `POST /invitations`, `DELETE /users/{id}` | `users:manage` |
| Equipes | CRUD `/teams`, `/teams/{id}/members` | `users:manage` |
| Perfis | CRUD `/profiles`; `POST /profiles/{id}/open-intent`, `/close`, `/archive`, `/restore`, `/duplicate` (só config), `POST /profiles/batch` | `profiles:read/create/edit/open/delete` |
| Leases | `POST /profiles/{id}/lease`, `PUT /leases/{id}/heartbeat`, `DELETE /leases/{id}`, `POST /leases/{id}/force-release` (admin) | `profiles:open`, `admin:force-close` |
| Sync | `POST /profiles/{id}/manifests`, `GET /profiles/{id}/snapshots`, `POST /snapshots/{id}/restore`, URLs pré-assinadas p/ chunks | `profiles:open` |
| Etiquetas / Pastas | CRUD `/tags`, `/folders` | `profiles:edit` |
| Redes | CRUD `/network-configs`; `POST /network-configs/{id}/test` | `network:manage`; segredo write-only |
| Extensões | CRUD `/extension-catalog`; vínculos `/profiles/{id}/extensions` | `extensions:manage` |
| Favoritos | CRUD `/bookmark-sets` | `profiles:edit` |
| Dispositivos | `GET /devices`, `DELETE /devices/{id}` (revogação) | `devices:manage` |
| Sessões | `GET /sessions`, `DELETE /sessions/{id}` | próprio usuário / `users:manage` |
| Auditoria | `GET /audit-events` (filtros, cursor; somente leitura, append-only) | `audit:read` |
| Automações | CRUD `/automation-flows`; `POST /automation-flows/{id}/runs`, `POST /runs/{id}/cancel`; allowlist de domínios por org | `automation:manage/run` |
| Webhooks | CRUD `/webhooks`; `GET /webhooks/{id}/deliveries` | `webhooks:manage` |
| Assinatura/uso | `GET /subscription`, `GET /usage`, `GET /entitlements` | `billing:read` |
| API keys | CRUD `/api-keys` (segredo exibido uma única vez) | `api:manage` |

## SDK e documentação (F5)

- SDK TypeScript gerado dos contratos (`/packages/contracts` compartilhado com desktop e web-admin).
- Portal de docs com exemplos por recurso, guia de webhooks, política de depreciação e changelog de API.
- Testes de contrato no CI validam que a API implementada corresponde ao OpenAPI publicado (ver TEST_STRATEGY).
