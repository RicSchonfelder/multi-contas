# SPEC_SOURCE — Especificação Mestre Condensada (Browser Workspace)

> Fonte de verdade derivada do prompt mestre do cliente (2026-07-01). Nome do projeto é PROVISÓRIO.
> Todos os agentes devem ler este arquivo antes de produzir documentação.

## Produto
Plataforma comercial desktop + cloud para empresas gerenciarem ambientes de navegador (perfis Chromium) isolados, seguros, auditáveis e colaborativos. Implementação 100% própria — sem copiar código, marca, layout ou textos de concorrentes. Nunca apresentar como "clone".

Cada perfil tem armazenamento isolado: cookies, cache, LocalStorage, IndexedDB, sessões, histórico, favoritos, extensões autorizadas, downloads, permissões, preferências, certificados, config de rede.

Casos de uso autorizados: agências com autorização de clientes, suporte, comercial, e-commerce, franquias, QA/testes, dev web, atendimento multi-empresa, labs de segurança autorizados, separação de contextos profissionais, trabalho remoto em equipe, navegação corporativa controlada.

## Limites obrigatórios (NÃO implementar)
- Evasão de antifraude, bloqueios, banimentos; fazendas de contas; criação massiva de contas; bypass de CAPTCHA/KYC; falsificação de identidade; roubo/reuso de sessões de terceiros; extração de credenciais; export de cookies em formato aberto inseguro; ocultação de automação; spoofing de dispositivos.
- NÃO alterar arbitrariamente: Canvas/WebGL/AudioContext fingerprint, fontes, hardwareConcurrency, deviceMemory, Client Hints, WebRTC para mascarar origem, parâmetros de hardware, identificadores de dispositivo.
- PERMITIDO (privacidade legítima, transparente): bloqueio de rastreadores, controle de cookies, limpeza de dados, bloqueio de WebRTC, permissões por domínio, políticas corporativas, proteção contra scripts invasivos. Nunca vender como mecanismo anti-antifraude.
- Automação: só domínios em allowlist do próprio cliente/QA/APIs oficiais. Sem "aquecimento de conta", sem CAPTCHA solving, com identificação clara de automação, limites, auditoria, botão de emergência.

## Arquitetura geral
Desktop Client → Local Profile Manager → Managed Chromium Processes → Encrypted Sync Client → Cloud API → PostgreSQL/Redis/Object Storage.

Monorepo: /apps (desktop, web-admin, api, worker), /packages (ui, contracts, database, crypto, logging, config, validation, browser-core, sync-engine, automation-sdk), /infrastructure (docker, terraform, monitoring).

ADRs obrigatórios comparando:
- Desktop: (A) Tauri+Rust+React+TS+Chromium externo+serviço local; (B) Electron+React+TS; (C) React UI + daemon Go + Chromium externo + IPC autenticado. Comparar memória, isolamento, atualização, assinatura, extensões, multiplataforma, controle de processos, debugging, segurança, distribuição, complexidade. → docs/adr/ADR-001-desktop-stack.md
- Backend: Node (NestJS/Fastify) vs Go; PostgreSQL, Redis, S3-compatible, filas, WebSocket, OpenAPI, OIDC/OAuth2.1, OpenTelemetry. → ADR-002.

## Perfis
Campos: UUID, organização, proprietário, nome, descrição, pasta, etiquetas, status, cor, avatar, SO real, versão do navegador, diretório local, datas, usuário/dispositivo da última abertura, status sync, config rede, extensões, favoritos, políticas, notas, campos custom, bloqueio, versão do registro, checksum, retenção.
Status: disponível, em uso, sincronizando, bloqueado, com erro, arquivado, excluído, aguardando atualização.
Operações: CRUD, duplicar só configurações (sem sessões), arquivar/restaurar, abrir/encerrar/reiniciar, limpar cache/cookies, mover pasta, etiquetas, extensões, favoritos, rede, histórico de alterações, lote, pesquisa/filtro/ordenar, visualizações salvas.
Lease distribuído: aquisição, heartbeat, expiração, recuperação pós-falha, encerramento forçado por admin, auditoria, proteção contra race. Um perfil nunca aberto em 2 máquinas simultaneamente.

## Isolamento local
Diretório exclusivo por perfil (`/app-data/profiles/{uuid}/`). Validar permissões do SO, detectar corrupção, backup pré-migração, versão do formato, bloqueio concorrente, detecção de crash, restauração segura, kill de processos órfãos, limpeza de temporários. Segredos NUNCA dentro do diretório do Chromium — usar Windows Credential Manager / macOS Keychain / Secret Service.

## Rede/Proxies (uso corporativo/QA, não evasão)
Direto, HTTP, HTTPS, SOCKS5, PAC. Campos: nome, tipo, host, porta, user, senha, país declarado, descrição, org, dono, compartilhamento, validade, última verificação, latência, status, notas. Testar conexão, medir latência, validar credenciais, nunca logar senha, criptografar credenciais, mascarar na UI, rotação manual, auditar mudanças. Não comprar proxies, não prometer anonimato, não alterar identidade declarada.

## Extensões e favoritos
Catálogo corporativo aprovado: cadastro, vínculo org/grupo, obrigatória/opcional, versionamento, ativar/desativar/remover, auditoria, bloqueio de não autorizadas, integridade, exibir permissões. Sem instalação remota sem aprovação. Favoritos: pastas, URLs, ícones, ordem, conjuntos reutilizáveis, globais/org/perfil.

## Times e permissões (multi-tenancy real)
Entidades: plataforma, organização, workspace, equipe, usuário, convite, função, permissão, grupo, perfil, dispositivo, sessão, assinatura.
Papéis: Superadmin, Admin da org, Gerente, Operador, Auditor, Financeiro, Suporte, Somente leitura.
Permissões granulares (ver/criar/editar/abrir/encerrar/excluir/restaurar/compartilhar perfil, gerenciar rede, ver segredos, gerenciar extensões, ver auditoria, gerenciar usuários, cobrança, API, automação, encerrar sessão de outro operador). Autorização SEMPRE validada no backend.

## Sincronização criptografada
Protocolo próprio: inventário, manifests, checksums, compactação, multipart, retomada, versionamento, detecção de conflito, bloqueio, snapshots, rollback, criptografia, integridade, retenção. Separar: metadados / configurações / dados do navegador / downloads (não sync por padrão) / segredos / logs / snapshots. Criptografia por envelope: DEK por perfil, KEK por organização, rotação, revogação de dispositivo, recuperação administrativa. Documentar honestamente se há ou não E2EE.

## Autenticação
E-mail + verificação, Argon2id, MFA TOTP, códigos de recuperação, sessões revogáveis, dispositivos confiáveis, expiração, rotação de refresh token, detecção de reuso, rate limiting, anti credential-stuffing, confirmação de ações críticas, notificação de novo acesso, revogação remota, auditoria. Futuro: passkeys, SSO, OIDC, SAML. Desktop↔serviço local: IPC seguro/loopback restrito, token efêmero, origem validada, sem portas públicas.

## Auditoria
Eventos: login/logout/falha, CRUD perfil, abertura/encerramento, rede, extensão, exportação, permissão, convite, remoção, acesso admin, assinatura, API, automação, revogação. Cada evento: org, ator, ação, recurso, data, dispositivo, resultado, IP quando adequado, correlação, metadados seguros. NUNCA armazenar: senhas, cookies, tokens, formulários, dados bancários, headers de auth, segredos de proxy. Logs imutáveis/detecção de adulteração para eventos críticos.

## API pública /api/v1
Recursos: auth, orgs, usuários, equipes, perfis, etiquetas, pastas, redes, extensões, favoritos, dispositivos, sessões, auditoria, automações, webhooks, assinatura, uso. OpenAPI, docs, exemplos, SDK TS, idempotency keys, paginação, filtros, rate limits, escopos, tokens revogáveis, webhooks assinados, versionamento, depreciação. Não expor cookies/senhas/sessões.

## Desktop — 21 telas
Login, Recuperação, MFA, Seleção de org, Dashboard, Lista de perfis (tabela rica: colunas configuráveis, filtros, ordenação, paginação, multi-seleção, lote, views salvas, atalhos, menu contextual, loading/empty/error states), Criação, Detalhes, Pastas, Etiquetas, Redes, Extensões, Favoritos, Equipe, Logs, Automações, Dispositivos, Configurações, Plano e uso, Diagnóstico, Atualizações.
Fluxo de abertura de perfil: permissão → lease → sync check → download → integridade → versão Chromium → rede → iniciar processo → evento → heartbeat → monitorar → sync pós-encerramento → liberar lease.

## Painel web admin
Visão geral, orgs, usuários, assinaturas, planos, cobranças, faturas, cupons, limites, storage, dispositivos, versões, incidentes, suporte, auditoria, abuso, políticas, feature flags, filas, jobs, webhooks, métricas, saúde, sessões admin. Impersonation só se estritamente necessário (justificativa, autorização especial, banner, tempo limitado, auditoria, sem segredos, notificação).

## Planos
EntitlementService central (não espalhar checks). Limites: perfis, usuários, orgs, storage, dispositivos, automações, minutos, API calls, retenção de logs, snapshots, redes, extensões, suporte. Gateway de pagamento abstraído.

## Banco (ERD antes das migrations)
Tabelas mínimas: users, organizations, organization_members, roles, permissions, role_permissions, workspaces, teams, team_members, browser_profiles, profile_tags, tags, folders, profile_folders, network_configs, profile_network_configs, extension_catalog, profile_extensions, bookmark_sets, profile_bookmark_sets, devices, device_sessions, profile_leases, sync_manifests, sync_snapshots, sync_objects, automation_flows, automation_runs, api_keys, webhooks, webhook_deliveries, audit_events, plans, subscriptions, entitlements, usage_records, invoices, feature_flags, support_cases, security_events.
UUID, timestamps, soft delete, versionamento otimista, índices, FKs, constraints, isolamento por org, anti cross-tenant, migrations reversíveis. RLS como camada ADICIONAL.

## Observabilidade
Logs estruturados, métricas, traces, correlation ID, dashboards, alertas, health/readiness/liveness, métricas de filas, falhas de sync, corrupção, crashes Chromium, CPU/memória/storage, latência, falhas de auth, erros por versão. OpenTelemetry.

## Atualizações
Chromium + app: assinatura de binários, canais stable/beta/internal, rollback, compatibilidade de perfis, migrações, bloqueio de versões inseguras, rollout gradual, feature flags, telemetria. Downloads só com HTTPS + assinatura + checksum + origem verificada.

## LGPD
Controlador/operador, finalidade, base legal, minimização, retenção, exclusão, portabilidade, exportação, consentimento, suboperadores, transferência internacional, resposta a incidente, canal do titular, anonimização, segurança, registro de operações. Implementar: exclusão de conta, exportação, revogação de dispositivos, retenção configurável, termos versionados, registro de consentimento. Não coletar navegação para publicidade.

## Testes
Unitário, integração, contrato, e2e, segurança, concorrência, sync, recuperação, atualização, migração, isolamento, multi-tenancy, permissões, performance, crash recovery.
18 cenários obrigatórios: (1) dois usuários abrem o mesmo perfil; (2) perda de internet durante sync; (3) app fecha abruptamente; (4) Chromium trava; (5) arquivo corrompido; (6) upload interrompido; (7) token revogado; (8) perda de acesso com perfil aberto; (9) versão incompatível; (10) rede corporativa indisponível; (11) acesso cross-org; (12) restaurar snapshot; (13) atualização falha; (14) disco cheio; (15) credencial de proxy alterada; (16) processo órfão; (17) lease expira indevidamente; (18) eventos conflitantes no backend.
Nunca declarar fase concluída sem evidências.

## DevOps
Docker Compose local: API, PostgreSQL, Redis, S3-compat (MinIO), worker, observabilidade, mail dev. Pipelines: lint, typecheck, testes, build, audit de deps, SBOM, secret scanning, assinatura, publicação, migrations, deploy, rollback. Ambientes dev/staging/prod separados, credenciais nunca compartilhadas.

## Fases
- F0 Descoberta: inventário, requisitos, riscos, arquitetura, ADRs, ERD, threat model, roadmap, estimativas, MVP. Sem código além de scaffolding.
- F1 Prova técnica local: desktop, 2 perfis, diretórios isolados, abrir/fechar Chromium, detecção de processo, persistência, logs, crash recovery. Sucesso: 2 perfis não compartilham cookies/cache/storage.
- F2 MVP individual: auth, CRUD, pastas, etiquetas, favoritos, extensões, rede, diagnóstico, atualização, storage local seguro.
- F3 Cloud e sync: backend, banco, storage, manifests, snapshots, sync, leases, recuperação, dispositivos.
- F4 Equipes: orgs, convites, papéis, permissões, compartilhamento, auditoria.
- F5 API e automação autorizada (Playwright, allowlist).
- F6 Monetização: planos, limites, assinatura, cobrança, painel financeiro.
- F7 Produção: assinatura de binários, instaladores, auto-update, monitoramento, backups, DR, pentest, docs, suporte.

## MVP (Windows only)
Login, desktop app, perfis Chromium persistentes, isolamento cookies/cache, CRUD perfis, pastas, etiquetas, favoritos, extensões autorizadas, proxy corporativo, abrir/encerrar, detecção de crash, logs locais, backend básico, 1 organização, sync experimental, 2 papéis, painel admin mínimo.
FORA: macOS, Linux, billing complexo, automação visual, marketplace, SSO, white-label, sync tempo real, E2EE, centenas de perfis simultâneos, gestão remota avançada.

### 15 critérios de aceitação do MVP
1. Perfil mantém sessão após fechar/abrir. 2. Perfis não compartilham sessão. 3. Bloqueio de abertura concorrente. 4. Encerramento correto do processo. 5. Recuperação pós-crash. 6. Segredos fora dos logs. 7. Sem permissão = sem abertura. 8. Validação de config de rede. 9. Auditoria registrada. 10. Testes automatizados passando. 11. Documentação atualizada. 12. Instalador funciona em máquina limpa. 13. Atualização assinada funciona. 14. Falha de sync não destrói perfil local. 15. Exclusão com confirmação e recuperação.

## Prioridade final
1. segurança; 2. isolamento; 3. integridade; 4. conformidade; 5. recuperação de falhas; 6. UX; 7. performance; 8. quantidade de features.
