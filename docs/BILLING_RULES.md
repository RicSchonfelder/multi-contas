# BILLING_RULES — Planos, limites e entitlement (PRELIMINAR — Fase 6)

> Tudo neste documento é preliminar e será validado comercialmente antes da F6. A arquitetura, porém, vale desde a F3: **nenhuma verificação de plano espalhada pelo código**.

## Regra central: EntitlementService

- Serviço único (`/apps/api`, consumido também pelo desktop via API) que responde: `can(orgId, feature)` e `limit(orgId, resource)`.
- Toda funcionalidade premium/limitada consulta o EntitlementService — UI apenas reflete, backend decide.
- Entitlements materializados na tabela `entitlements` (ver DATA_MODEL) a partir do plano + overrides por org (ex.: cortesia, enterprise custom).
- Cache com invalidação por evento de assinatura; fail-closed para features premium, fail-open apenas para leitura de dados já existentes (nunca trancar o usuário fora dos próprios dados).

## Planos preliminares

| Limite | Trial (14d) | Starter | Team | Business |
|---|---|---|---|---|
| Perfis | 3 | 10 | 50 | 200+ (custom) |
| Usuários | 1 | 1 | 5 | 25+ |
| Dispositivos por usuário | 1 | 2 | 3 | 5 |
| Armazenamento de sync | 1 GB | 10 GB | 50 GB | 250 GB+ |
| Snapshots retidos por perfil | 2 | 5 | 15 | 30 |
| Redes cadastradas | 2 | 10 | 50 | ilimitado* |
| Extensões no catálogo | 3 | 10 | 50 | ilimitado* |
| Retenção de auditoria | 7 dias | 30 dias | 90 dias | 365 dias |
| Chamadas de API/mês | — | — | 50k | 500k+ |
| Automações / minutos/mês | — | — | 10 fluxos / 500 min | 100 fluxos / 5.000 min |
| Suporte | comunidade | e-mail | e-mail prioritário | SLA dedicado |

\* "ilimitado" = sem limite comercial; limites técnicos anti-abuso permanecem.

## Ciclo de vida da assinatura

- **Upgrade:** imediato, pró-rata; entitlements atualizados na hora.
- **Downgrade:** aplicado na virada do ciclo; se o uso atual excede o novo limite, recursos excedentes ficam **somente leitura/arquivados** (nunca excluídos automaticamente) até adequação.
- **Inadimplência:** D+3 aviso → D+7 bloqueio de criação (read-only) → D+30 suspensão de sync → D+90 exclusão conforme política de retenção LGPD, com avisos e exportação disponível até o fim.
- **Cancelamento:** acesso até fim do ciclo pago; exportação de dados disponível; exclusão conforme LGPD.md.

## Medição de uso

- `usage_records` (ver DATA_MODEL) alimentada por eventos (perfil criado, GB armazenado, chamada de API, minuto de automação); agregação por worker; exposta em `GET /usage` e no painel.
- Medição idempotente (chave de evento) — nunca cobrar duplicado.

## Gateway de pagamento

- Abstração `PaymentGatewayPort` em `/packages/contracts`: criar assinatura, atualizar, cancelar, webhook de status, fatura.
- Regras de negócio (planos, limites, dunning) vivem no nosso domínio; o gateway (Stripe ou similar — decidir em ADR na F6) só processa pagamento. Sem dados de cartão em nossos servidores (tokenização no gateway).
- Eventos de billing entram na auditoria (`subscription.changed`, `invoice.paid`…), sem dados financeiros sensíveis.
