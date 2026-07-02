# REQUIREMENTS — Browser Workspace

> Derivado de `docs/SPEC_SOURCE.md`. Requisitos numerados e rastreáveis.
> Prioridade: **MVP** (Fases F1–F2 + mínimo de F3/F4 para o MVP Windows) ou **pós-MVP**.
> Convenção: RF-xxx = funcional; RNF-xxx = não funcional; CA = critério de aceitação.
> Os 15 critérios de aceitação do MVP da spec estão mapeados como **MVP-CA-01..15** (seção 15) e referenciados nos requisitos.

## Índice de módulos
1. Perfis (RF-001..019)
2. Isolamento local (RF-020..029)
3. Rede/Proxies (RF-030..037)
4. Extensões e favoritos (RF-040..047)
5. Times e RBAC (RF-050..058)
6. Sincronização (RF-060..068)
7. Autenticação (RF-070..079)
8. Auditoria (RF-080..085)
9. Automação (RF-090..095)
10. API pública (RF-100..106)
11. Desktop app (RF-110..117)
12. Painel web admin (RF-120..125)
13. Planos/Billing (RF-130..134)
14. Requisitos não funcionais (RNF-001..016)
15. Rastreabilidade dos 15 critérios do MVP

---

## 1. Perfis

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-001 | CRUD de perfis com campos da spec (UUID, org, proprietário, nome, descrição, pasta, etiquetas, status, cor, avatar, SO, versão do navegador, diretório local, datas, último uso, status sync, rede, extensões, favoritos, políticas, notas, campos custom, bloqueio, versão do registro, checksum, retenção) | MVP | Criar/ler/editar/excluir persiste todos os campos; validação de obrigatórios; UUID imutável |
| RF-002 | Máquina de estados do perfil: disponível, em uso, sincronizando, bloqueado, com erro, arquivado, excluído, aguardando atualização | MVP | Transições inválidas rejeitadas; status visível na UI e via API |
| RF-003 | Abrir perfil (fluxo completo: permissão → lease → sync check → download → integridade → versão Chromium → rede → iniciar processo → evento → heartbeat) | MVP | Cada etapa registrada; falha em qualquer etapa aborta com mensagem clara e libera lease (MVP-CA-07) |
| RF-004 | Encerrar/reiniciar perfil com encerramento correto do processo Chromium e liberação de lease | MVP | Nenhum processo órfão após encerramento normal (MVP-CA-04); sync pós-encerramento disparado |
| RF-005 | Persistência de sessão: perfil mantém cookies/sessões após fechar e reabrir | MVP | MVP-CA-01 |
| RF-006 | Duplicar perfil copiando somente configurações (sem cookies/sessões/histórico) | MVP | Perfil duplicado abre "limpo"; config de rede/extensões/favoritos copiada |
| RF-007 | Arquivar e restaurar perfil | MVP | Perfil arquivado não abre; restauração retorna estado íntegro |
| RF-008 | Exclusão de perfil com confirmação explícita e janela de recuperação (soft delete) | MVP | MVP-CA-15: exclusão exige confirmação; recuperável dentro do período de retenção |
| RF-009 | Limpar cache/cookies de um perfil sob demanda | MVP | Ação auditada; demais dados preservados |
| RF-010 | Etiquetas (tags) e cores em perfis; filtro por etiqueta | MVP | CRUD de tags; múltiplas tags por perfil |
| RF-011 | Pastas de perfis (organização hierárquica) e mover perfil entre pastas | MVP | Perfil pertence a pastas; mover não afeta dados do perfil |
| RF-012 | Pesquisa, filtros, ordenação e paginação na lista de perfis | MVP | Busca por nome/tag/status/dono; resposta < 1 s para 500 perfis |
| RF-013 | Operações em lote (abrir múltiplos, etiquetar, arquivar, mover) | pós-MVP | Resultado por item; falhas parciais reportadas |
| RF-014 | Visualizações salvas (colunas, filtros, ordenação) | pós-MVP | Views por usuário; view padrão configurável |
| RF-015 | Histórico de alterações do perfil (changelog) | pós-MVP | Toda edição gera entrada com autor/data/campo |
| RF-016 | Lease distribuído: aquisição, heartbeat, expiração, recuperação pós-falha, proteção contra race | MVP | MVP-CA-03: perfil nunca aberto em 2 máquinas; cenários de teste 1, 8, 17 da spec passam |
| RF-017 | Encerramento forçado de lease por admin, auditado | MVP | Somente papel com permissão; operador afetado notificado; evento de auditoria |
| RF-018 | Detecção de crash do Chromium e recuperação segura do perfil | MVP | MVP-CA-05: após crash (cenários 3, 4), perfil reabre íntegro; lease recuperado |
| RF-019 | Versionamento otimista e checksum do registro do perfil | MVP | Edições concorrentes detectadas (cenário 18); checksum validado na abertura |

## 2. Isolamento local

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-020 | Diretório exclusivo por perfil em `/app-data/profiles/{uuid}/` contendo cookies, cache, LocalStorage, IndexedDB, sessões, histórico, downloads, permissões, preferências | MVP | MVP-CA-02: dois perfis nunca compartilham sessão/cache/storage (critério de sucesso da F1) |
| RF-021 | Validação de permissões do sistema de arquivos no diretório do perfil | MVP | Permissões incorretas bloqueiam abertura com erro explicativo |
| RF-022 | Detecção de corrupção de dados do perfil (cenário 5) | MVP | Arquivo corrompido detectado antes da abertura; oferta de restauração |
| RF-023 | Versão do formato do diretório + migração com backup prévio | MVP | Migração falha → rollback automático ao backup (cenário 13) |
| RF-024 | Bloqueio concorrente local (lock de diretório) | MVP | Segunda instância local não abre o mesmo diretório |
| RF-025 | Kill de processos Chromium órfãos e limpeza de temporários | MVP | Cenário 16: órfãos detectados e encerrados na inicialização |
| RF-026 | Segredos NUNCA no diretório do Chromium — usar Windows Credential Manager (MVP) / macOS Keychain / Secret Service | MVP | Varredura do diretório do perfil não encontra credenciais/tokens; MVP-CA-06 |
| RF-027 | Restauração segura pós-falha (app fechado abruptamente — cenário 3) | MVP | Estado consistente após restart; sem perda de dados confirmados |
| RF-028 | Tratamento de disco cheio (cenário 14) | MVP | Falha de escrita não corrompe perfil; usuário alertado |
| RF-029 | Backup pré-migração de formato/versão | MVP | Backup criado e verificado antes de qualquer migração destrutiva |

## 3. Rede/Proxies (uso corporativo/QA — não evasão)

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-030 | Configs de rede: direto, HTTP, HTTPS, SOCKS5, PAC, com campos da spec (nome, tipo, host, porta, user, senha, país declarado, org, dono, compartilhamento, validade, status, notas) | MVP | CRUD completo; vínculo perfil↔config |
| RF-031 | Teste de conexão, medição de latência e validação de credenciais de proxy | MVP | MVP-CA-08: config inválida detectada antes do uso; cenário 10 (rede indisponível) tratado |
| RF-032 | Credenciais de proxy criptografadas em repouso e mascaradas na UI | MVP | Senha nunca exibida em claro; nunca logada (MVP-CA-06) |
| RF-033 | Rotação manual de credenciais de proxy com propagação (cenário 15) | pós-MVP | Perfis usando a config recebem a atualização; sessões ativas notificadas |
| RF-034 | Auditoria de toda mudança em configs de rede | MVP | Evento com ator/antes-depois (sem segredos) |
| RF-035 | Compartilhamento de configs de rede por org/equipe conforme permissões | pós-MVP | Somente papéis autorizados veem/usam |
| RF-036 | Proibição de features de anonimização: sem alteração de identidade declarada, sem promessa de anonimato, sem venda/revenda de proxies | MVP (restrição permanente) | Revisão de produto: nenhuma UI/copy promete anonimato |
| RF-037 | Última verificação/latência/status visíveis por config | pós-MVP | Atualizados a cada teste |

## 4. Extensões e favoritos

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-040 | Catálogo corporativo de extensões aprovadas (cadastro, vínculo org/grupo, obrigatória/opcional, versionamento) | MVP | Somente extensões do catálogo instaláveis nos perfis |
| RF-041 | Ativar/desativar/remover extensão por perfil | MVP | Estado refletido na próxima abertura |
| RF-042 | Bloqueio de extensões não autorizadas + verificação de integridade | MVP | Extensão fora do catálogo não carrega; hash validado |
| RF-043 | Exibição de permissões solicitadas pela extensão antes da aprovação | MVP | Admin vê permissões no cadastro |
| RF-044 | Sem instalação remota de extensão sem aprovação explícita | MVP (restrição) | Fluxo exige aprovação; evento auditado |
| RF-045 | Favoritos por perfil: pastas, URLs, ícones, ordenação | MVP | CRUD; refletidos no Chromium do perfil |
| RF-046 | Conjuntos reutilizáveis de favoritos (globais/org/perfil) | MVP | Conjunto aplicável a N perfis; atualização propagada |
| RF-047 | Auditoria de mudanças em extensões e favoritos | MVP | Eventos com ator/recurso |

## 5. Times e RBAC (multi-tenancy)

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-050 | Entidades: plataforma, organização, workspace, equipe, usuário, convite, função, permissão, grupo, dispositivo, sessão, assinatura | MVP (1 org no MVP) | Modelo de dados conforme ERD; isolamento por org |
| RF-051 | Papéis: Superadmin, Admin da org, Gerente, Operador, Auditor, Financeiro, Suporte, Somente leitura | MVP: Admin + Operador; demais pós-MVP | Papéis atribuíveis; MVP tem 2 papéis funcionais |
| RF-052 | Permissões granulares (ver/criar/editar/abrir/encerrar/excluir/restaurar/compartilhar perfil, gerenciar rede, ver segredos, gerenciar extensões, ver auditoria, gerenciar usuários, cobrança, API, automação, encerrar sessão de outro operador) | MVP (subconjunto) | Matriz papel×permissão documentada e testada |
| RF-053 | Autorização SEMPRE validada no backend (nunca só na UI) | MVP | Testes de API sem permissão retornam 403; MVP-CA-07 |
| RF-054 | Convites de usuários com expiração e revogação | pós-MVP (F4) | Convite expira; aceite cria membership auditada |
| RF-055 | Compartilhamento de perfis com equipes/grupos | pós-MVP (F4) | Acesso segue permissões; revogação imediata |
| RF-056 | Anti cross-tenant: nenhum recurso acessível fora da org (cenário 11) | MVP | Testes de acesso cross-org falham com 404/403; RLS como camada adicional |
| RF-057 | Encerrar sessão/lease de outro operador (com permissão) | MVP | Auditado; operador notificado |
| RF-058 | Gestão de dispositivos por usuário (listar, revogar) | MVP (básico) | Dispositivo revogado perde acesso e sync (cenário 7) |

## 6. Sincronização criptografada

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-060 | Protocolo de sync: inventário, manifests, checksums, compactação, multipart, retomada de upload | MVP (experimental) | Upload interrompido retoma sem corromper (cenários 2, 6) |
| RF-061 | Separação de categorias: metadados / configurações / dados do navegador / downloads (não sync por padrão) / segredos / logs / snapshots | MVP | Downloads não sincronizados por padrão; segredos nunca no bucket de dados comuns |
| RF-062 | Criptografia por envelope: DEK por perfil, KEK por organização | MVP | Objetos ilegíveis sem KEK; documentação honesta sobre ausência de E2EE no MVP |
| RF-063 | Rotação de chaves, revogação de dispositivo, recuperação administrativa | pós-MVP | Dispositivo revogado não decripta novos dados |
| RF-064 | Detecção de conflito + bloqueio (lease) para evitar sync concorrente | MVP | Conflito detectado gera resolução explícita, nunca merge silencioso |
| RF-065 | Snapshots com versionamento e rollback (cenário 12) | pós-MVP (F3 completo) | Restaurar snapshot recupera estado íntegro |
| RF-066 | Falha de sync NUNCA destrói o perfil local | MVP | MVP-CA-14: dados locais intactos após qualquer falha de sync |
| RF-067 | Verificação de integridade (checksum) em download antes de abrir perfil | MVP | Download corrompido rejeitado; retry |
| RF-068 | Política de retenção de snapshots/versões | pós-MVP | Configurável por org/plano |

## 7. Autenticação

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-070 | Cadastro/login com e-mail + verificação; hash Argon2id | MVP | E-mail não verificado limita acesso; parâmetros Argon2id documentados |
| RF-071 | MFA TOTP + códigos de recuperação | MVP | Setup, uso e recuperação testados |
| RF-072 | Sessões revogáveis, expiração, rotação de refresh token, detecção de reuso | MVP | Reuso de refresh token revoga a família de tokens (cenário 7) |
| RF-073 | Rate limiting e proteção anti credential-stuffing | MVP | Bloqueio progressivo; auditado |
| RF-074 | Confirmação de ações críticas (exclusões, mudanças de segurança) | MVP | Re-auth ou confirmação explícita |
| RF-075 | Notificação de novo acesso/dispositivo + revogação remota | MVP (básico) | E-mail em novo login; revogação encerra sessão |
| RF-076 | Dispositivos confiáveis | pós-MVP | MFA reduzido em dispositivo confiável, revogável |
| RF-077 | Desktop↔serviço local: IPC seguro em loopback restrito, token efêmero, origem validada, sem portas públicas | MVP | Porta não acessível externamente; token expira; origem verificada |
| RF-078 | Passkeys, SSO, OIDC, SAML | pós-MVP (fora do MVP) | — |
| RF-079 | Auditoria de todos os eventos de auth (login, logout, falha, MFA, revogação) | MVP | Eventos presentes na trilha (MVP-CA-09) |

## 8. Auditoria

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-080 | Registro dos eventos da spec: auth, CRUD perfil, abertura/encerramento, rede, extensão, exportação, permissão, convite, remoção, acesso admin, assinatura, API, automação, revogação | MVP (subconjunto do MVP) | MVP-CA-09; cada evento com org, ator, ação, recurso, data, dispositivo, resultado, IP quando adequado, ID de correlação, metadados seguros |
| RF-081 | Proibição absoluta em logs: senhas, cookies, tokens, formulários, dados bancários, headers de auth, segredos de proxy | MVP | MVP-CA-06: varredura automatizada de logs sem segredos |
| RF-082 | Logs imutáveis / detecção de adulteração para eventos críticos | MVP (mínimo: hash encadeado) | Alteração detectável; verificação periódica |
| RF-083 | Consulta/filtro/exportação da trilha por papel autorizado (Auditor) | MVP (consulta básica) | Filtros por ator, recurso, período, ação |
| RF-084 | Retenção de logs configurável (limites por plano) | pós-MVP (F6) | Retenção aplicada e auditada |
| RF-085 | Logs locais do desktop (abertura, crash, sync) com mesma política de segredos | MVP | MVP-CA-06 aplicado a logs locais |

## 9. Automação (autorizada)

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-090 | Automação (Playwright) apenas em domínios de allowlist do próprio cliente/QA/APIs oficiais | pós-MVP (F5) | Execução fora da allowlist bloqueada e auditada |
| RF-091 | Identificação clara de automação (sem ocultação) | pós-MVP (F5, restrição permanente) | Sinal de automação preservado; revisão confirma ausência de flags de stealth |
| RF-092 | Limites de execução (rate, concorrência, minutos por plano) | pós-MVP | Limite excedido interrompe com erro claro |
| RF-093 | Botão de emergência (parar todas as automações da org) | pós-MVP | Interrupção < 5 s; auditada |
| RF-094 | Auditoria completa de runs (flow, ator, perfil, resultado) | pós-MVP | Cada run rastreável |
| RF-095 | Proibições permanentes: sem CAPTCHA solving, sem "aquecimento de conta", sem automação em domínios de terceiros não autorizados | permanente | Revisão de produto/código em cada release |

## 10. API pública `/api/v1`

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-100 | Recursos: auth, orgs, usuários, equipes, perfis, etiquetas, pastas, redes, extensões, favoritos, dispositivos, sessões, auditoria, automações, webhooks, assinatura, uso | pós-MVP (F5); API interna no MVP | OpenAPI publicado; exemplos por recurso |
| RF-101 | Tokens de API com escopos, revogáveis | pós-MVP | Escopo insuficiente → 403; revogação imediata |
| RF-102 | Idempotency keys, paginação, filtros, rate limits | pós-MVP | Retry idempotente não duplica; limites por plano |
| RF-103 | Webhooks assinados com entregas rastreáveis | pós-MVP | Assinatura verificável; retries com backoff |
| RF-104 | Versionamento e política de depreciação | pós-MVP | Mudanças breaking só em nova versão |
| RF-105 | API nunca expõe cookies, senhas ou dados de sessão de navegador | permanente | Testes de contrato garantem ausência |
| RF-106 | SDK TypeScript | pós-MVP | SDK cobre recursos publicados |

## 11. Desktop app (MVP: Windows)

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-110 | 21 telas da spec: Login, Recuperação, MFA, Seleção de org, Dashboard, Lista de perfis, Criação, Detalhes, Pastas, Etiquetas, Redes, Extensões, Favoritos, Equipe, Logs, Automações, Dispositivos, Configurações, Plano e uso, Diagnóstico, Atualizações | MVP (subconjunto; Automações/Plano podem ser placeholder) | Navegação completa; sem becos sem saída |
| RF-111 | Lista de perfis rica: colunas configuráveis, filtros, ordenação, paginação, multi-seleção, ações em lote, views salvas, atalhos de teclado, menu contextual | MVP (núcleo); views/lote pós-MVP | Estados de loading/empty/error implementados em todas as listas |
| RF-112 | Tela de Diagnóstico (saúde do serviço local, Chromium, disco, rede, sync) | MVP | Detecta e explica problemas comuns; exportável para suporte |
| RF-113 | Instalador Windows funcional em máquina limpa | MVP | MVP-CA-12: instala e roda sem dependências pré-existentes |
| RF-114 | Auto-update com binários assinados, canais stable/beta/internal, rollback | MVP (stable + assinatura) | MVP-CA-13: atualização assinada aplica com sucesso; assinatura inválida rejeitada |
| RF-115 | Downloads (app/Chromium) só via HTTPS + assinatura + checksum + origem verificada | MVP | Download adulterado rejeitado |
| RF-116 | Gestão de versão do Chromium com compatibilidade de perfis e bloqueio de versões inseguras (cenário 9) | MVP (básico) | Versão incompatível bloqueia abertura com orientação |
| RF-117 | Documentação de usuário atualizada a cada release | MVP | MVP-CA-11 |

## 12. Painel web admin

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-120 | Painel mínimo do MVP: visão geral, orgs, usuários, dispositivos, auditoria | MVP | Operações básicas de suporte possíveis |
| RF-121 | Painel completo: assinaturas, planos, cobranças, faturas, cupons, limites, storage, versões, incidentes, suporte, abuso, políticas, feature flags, filas, jobs, webhooks, métricas, saúde | pós-MVP | — |
| RF-122 | Impersonation restrita: justificativa obrigatória, autorização especial, banner visível, tempo limitado, sem acesso a segredos, auditoria, notificação ao usuário | pós-MVP | Todos os 7 controles verificados em teste |
| RF-123 | Gestão de feature flags | pós-MVP | Flags por org/percentual; auditadas |
| RF-124 | Painel de abuso/segurança (security_events) | pós-MVP | Eventos de abuso triáveis |
| RF-125 | Sessões admin com MFA obrigatório e expiração curta | MVP (para o painel mínimo) | Sem MFA → sem acesso admin |

## 13. Planos/Billing (preliminar — detalhes em `BILLING_RULES.md`)

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RF-130 | EntitlementService central: TODA verificação de limite/feature passa por ele | MVP (estrutura), F6 (completo) | Nenhum check de plano espalhado no código (verificado em review) |
| RF-131 | Limites por plano: perfis, usuários, orgs, storage, dispositivos, automações, minutos, API calls, retenção de logs, snapshots, redes, extensões, suporte | pós-MVP (F6) | Limite atingido → erro claro + upsell não bloqueante de dados |
| RF-132 | Gateway de pagamento abstraído (interface própria) | pós-MVP (F6) | Troca de gateway sem alterar domínio |
| RF-133 | Medição de uso (usage_records) | pós-MVP (F6) | Uso consultável por org |
| RF-134 | Regras de upgrade/downgrade/inadimplência sem destruição de dados | pós-MVP (F6) | Downgrade nunca apaga perfis; ver BILLING_RULES.md |

## 14. Requisitos não funcionais

| ID | Requisito | Prioridade | Critérios de aceitação |
|---|---|---|---|
| RNF-001 | **Segurança acima de tudo** (prioridade 1 da spec): threat model mantido; pentest antes de produção (F7) | MVP | Threat model em docs/; issues críticas bloqueiam release |
| RNF-002 | Isolamento verificável entre perfis e entre orgs | MVP | Testes automatizados de isolamento e multi-tenancy passam (MVP-CA-02, cenário 11) |
| RNF-003 | Integridade: checksums em registros, manifests e downloads; versionamento otimista | MVP | Corrupção sempre detectada antes do uso |
| RNF-004 | Conformidade LGPD: minimização, retenção, exclusão de conta, exportação/portabilidade, consentimento registrado, termos versionados; sem coleta de navegação para publicidade | MVP (básico), completo até F7 | Fluxos de exclusão e exportação funcionais |
| RNF-005 | Recuperação de falhas: os 18 cenários obrigatórios da spec têm testes automatizados | MVP (cenários 1–10, 14, 16, 17), demais até F3/F4 | MVP-CA-05, MVP-CA-10, MVP-CA-14 |
| RNF-006 | Observabilidade: logs estruturados, métricas, traces, correlation ID, OpenTelemetry, health/readiness/liveness | MVP (logs + health), completo F3+ | Toda request rastreável por correlation ID |
| RNF-007 | Testes: unitário, integração, contrato, e2e, segurança, concorrência, sync, recuperação, migração, isolamento, multi-tenancy, permissões, performance | MVP (pipeline com gates) | MVP-CA-10: suíte verde obrigatória; nenhuma fase concluída sem evidências |
| RNF-008 | Performance: abertura de perfil < 5 s (perfil local, hardware de referência); lista de 500 perfis < 1 s | MVP (alvo, não gate) | Medido em telemetria |
| RNF-009 | Banco: UUID, timestamps, soft delete, versionamento otimista, índices, FKs, constraints, migrations reversíveis, RLS adicional | MVP | ERD aprovado antes das migrations; migrations com down testado |
| RNF-010 | DevOps: Docker Compose local (API, PostgreSQL, Redis, MinIO, worker, observabilidade, mail dev); pipelines com lint, typecheck, testes, build, audit de deps, SBOM, secret scanning, assinatura | MVP | Pipeline falha bloqueia merge |
| RNF-011 | Ambientes dev/staging/prod separados; credenciais nunca compartilhadas | MVP (dev/staging), prod em F7 | Secret scanning ativo |
| RNF-012 | Distribuição segura: binários assinados; rollout gradual; rollback | MVP (assinatura + rollback), gradual F7 | MVP-CA-13 |
| RNF-013 | Restrições éticas permanentes (seção "Limites obrigatórios" da spec) valem como requisito não funcional inegociável em TODAS as fases | permanente | Checklist ético em toda revisão de release |
| RNF-014 | Documentação honesta: declarar explicitamente ausência de E2EE no MVP; nunca usar linguagem de evasão em UI/docs/marketing | permanente | Revisão de copy por release |
| RNF-015 | ADRs obrigatórios: ADR-001 (desktop stack: Tauri vs Electron vs React+daemon Go) e ADR-002 (backend: Node vs Go) antes de código de produção | MVP (F0) | ADRs em docs/adr/ com comparação completa da spec |
| RNF-016 | Escopo MVP: Windows only; FORA: macOS, Linux, billing complexo, automação visual, marketplace, SSO, white-label, sync tempo real, E2EE, centenas de perfis simultâneos | MVP | Backlog marca itens fora de escopo |

## 15. Rastreabilidade — 15 critérios de aceitação do MVP

| MVP-CA | Critério (spec) | Requisitos que o cobrem |
|---|---|---|
| MVP-CA-01 | Perfil mantém sessão após fechar/abrir | RF-005, RF-020 |
| MVP-CA-02 | Perfis não compartilham sessão | RF-020, RNF-002 |
| MVP-CA-03 | Bloqueio de abertura concorrente | RF-016, RF-024 |
| MVP-CA-04 | Encerramento correto do processo | RF-004, RF-025 |
| MVP-CA-05 | Recuperação pós-crash | RF-018, RF-027, RNF-005 |
| MVP-CA-06 | Segredos fora dos logs | RF-026, RF-032, RF-081, RF-085 |
| MVP-CA-07 | Sem permissão = sem abertura | RF-003, RF-053 |
| MVP-CA-08 | Validação de config de rede | RF-031 |
| MVP-CA-09 | Auditoria registrada | RF-079, RF-080 |
| MVP-CA-10 | Testes automatizados passando | RNF-005, RNF-007 |
| MVP-CA-11 | Documentação atualizada | RF-117, RNF-014 |
| MVP-CA-12 | Instalador funciona em máquina limpa | RF-113 |
| MVP-CA-13 | Atualização assinada funciona | RF-114, RNF-012 |
| MVP-CA-14 | Falha de sync não destrói perfil local | RF-066, RNF-005 |
| MVP-CA-15 | Exclusão com confirmação e recuperação | RF-008 |
