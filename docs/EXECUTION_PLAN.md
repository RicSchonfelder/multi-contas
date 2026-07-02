# EXECUTION_PLAN — F0 e F1 (Browser Workspace)

> Plano incremental detalhado das fases F0 (Descoberta) e F1 (Prova técnica local).
> Derivado de `docs/SPEC_SOURCE.md` e `docs/ROADMAP.md`. Requisitos em `docs/REQUIREMENTS.md`.
> Tarefas pequenas, ordenadas, com definição de pronto (DoD) individual. Branch sugerida por grupo.
> Convenções: commits atômicos; PR por grupo de tarefas; nenhuma tarefa "pronta" sem evidência (teste, doc ou artefato).

## Sequência de grupos

```
F0: G0.1 docs ─► G0.2 ADRs ─► G0.3 ERD/threat ─► G0.4 scaffold ─► G0.5 CI
F1: G1.1 esqueleto app ─► G1.2 modelo de perfil ─► G1.3 launcher ─► G1.4 monitor/crash ─► G1.5 logs ─► G1.6 prova de isolamento
```

---

## F0 — Descoberta

### G0.1 — Documentação de produto · branch `docs/f0-product-docs`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-001 | Consolidar `SPEC_SOURCE.md` como fonte de verdade (já existe; validar completude contra prompt do cliente) | Revisão registrada no topo do arquivo (data + revisor) |
| T-002 | Escrever `PRODUCT_VISION.md` | Contém problema, personas (≥4), proposta de valor, "o que NÃO é", posicionamento |
| T-003 | Escrever `REQUIREMENTS.md` com RF/RNF numerados e rastreabilidade dos 15 critérios do MVP | Todos os módulos da spec cobertos; cada requisito com prioridade e CA |
| T-004 | Escrever `ROADMAP.md` (F0–F7) | Cada fase com entregáveis, critérios de saída, complexidade, dependências |
| T-005 | Escrever `BILLING_RULES.md` preliminar | Planos, limites, EntitlementService, regras de transição; marcado como preliminar |

### G0.2 — ADRs · branch `docs/f0-adrs`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-006 | Pesquisar e redigir `docs/adr/ADR-001-desktop-stack.md`: (A) Tauri+Rust+React+TS+Chromium externo+serviço local; (B) Electron+React+TS; (C) React UI + daemon Go + IPC autenticado | Tabela comparando os 11 eixos da spec (memória, isolamento, atualização, assinatura, extensões, multiplataforma, controle de processos, debugging, segurança, distribuição, complexidade); decisão + consequências registradas |
| T-007 | Redigir `docs/adr/ADR-002-backend-stack.md`: Node (NestJS/Fastify) vs Go; incluir PostgreSQL, Redis, S3-compat, filas, WebSocket, OpenAPI, OIDC/OAuth2.1, OpenTelemetry | Decisão + trade-offs; compatível com ADR-001 (ex.: reuso de linguagem no daemon) |
| T-008 | ADR-003: estratégia de gestão do Chromium (bundled vs download gerenciado; canal de versão; user-data-dir) | Decisão define como F1 obtém/aponta o Chromium; requisitos de verificação de origem anotados |

### G0.3 — Dados e segurança · branch `docs/f0-data-security`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-009 | ERD completo (`docs/ERD.md` + diagrama) com todas as tabelas mínimas da spec | Todas as ~40 tabelas presentes; FKs, soft delete, versionamento otimista, isolamento por org anotados; revisado |
| T-010 | Threat model inicial (`docs/THREAT_MODEL.md`): STRIDE sobre desktop, IPC local, sync, lease, segredos, atualização, multi-tenancy | Cada ameaça com mitigação mapeada a requisito ou fase; riscos aceitos explícitos |
| T-011 | Registro de riscos do projeto (`docs/RISKS.md`) | ≥10 riscos com probabilidade/impacto/mitigação/dono |

### G0.4 — Scaffolding do monorepo · branch `chore/f0-scaffold`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-012 | Criar estrutura do monorepo: `/apps/{desktop,web-admin,api,worker}`, `/packages/{ui,contracts,database,crypto,logging,config,validation,browser-core,sync-engine,automation-sdk}`, `/infrastructure/{docker,terraform,monitoring}` | Workspaces resolvem; cada pacote com README de 3 linhas (propósito/status) |
| T-013 | Tooling base: gerenciador de pacotes, TypeScript config compartilhado, lint, format, hooks de pre-commit | `lint` e `typecheck` rodam limpos na raiz |
| T-014 | `packages/logging`: logger estruturado com redação automática de segredos (lista de chaves proibidas: password, token, cookie, secret, authorization, proxy_pass) | Teste unitário prova que campos sensíveis saem como `[REDACTED]` (base do MVP-CA-06) |
| T-015 | `packages/config`: carregamento de config tipada + validação (sem segredos em arquivos versionados) | Config inválida falha rápido com mensagem clara; `.env.example` documentado |

### G0.5 — CI mínimo · branch `chore/f0-ci`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-016 | Pipeline CI: lint, typecheck, testes, build, audit de dependências, secret scanning | Pipeline verde no monorepo vazio; falha bloqueia merge |
| T-017 | Template de PR com checklist ético (limites da spec) e checklist de evidências | Template ativo; inclui item "não introduz evasão/fingerprint spoofing" |

**Saída da F0:** critérios de saída do `ROADMAP.md` F0 atendidos; revisão formal dos ADRs antes de iniciar F1.

---

## F1 — Prova técnica local

**Objetivo:** app desktop mínimo (stack do ADR-001) gerenciando **2 perfis Chromium com diretórios isolados**: criar, abrir, encerrar, detectar crash, persistir sessão, logar. Windows only.

**Fora de escopo da F1:** backend/cloud, auth de usuário, sync, proxies, extensões, favoritos, RBAC, UI rica. Interface pode ser mínima (lista + botões).

### G1.1 — Esqueleto do app desktop · branch `feat/f1-app-skeleton`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-101 | Bootstrap do app desktop em `/apps/desktop` conforme ADR-001 (janela, build dev, empacotamento dev) | App abre janela "hello" no Windows; build reproduzível documentado |
| T-102 | Integrar `packages/logging` no app (logs em `%APPDATA%/<app>/logs/`, rotação simples) | Logs estruturados (JSON) gravados; teste de redação passa no contexto do app |
| T-103 | Definir diretório de dados do app (`/app-data/`) com criação idempotente e validação de permissões do SO | Diretório criado com permissões restritas ao usuário; falha de permissão gera erro claro (RF-021) |

### G1.2 — Modelo e armazenamento de perfis · branch `feat/f1-profile-store`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-104 | Definir tipo `Profile` mínimo em `packages/contracts` (uuid, nome, cor, diretório, status, datas, versão do registro, checksum) — subconjunto do RF-001 compatível com o ERD | Tipo revisado contra ERD; validação com `packages/validation` |
| T-105 | Store local de perfis (arquivo JSON/SQLite local com escrita atômica: write-temp + rename) | Kill do processo durante escrita não corrompe store (teste simula); versão do formato gravada (RF-023) |
| T-106 | Criar perfil: gera UUID, cria `/app-data/profiles/{uuid}/` vazio, registra no store | Criar 2 perfis gera 2 diretórios distintos; operação idempotente em retry |
| T-107 | Máquina de estados mínima: disponível → em uso → disponível; com erro | Transições inválidas rejeitadas com erro tipado (RF-002 reduzido) |
| T-108 | UI mínima: listar perfis, criar, abrir, encerrar, indicador de status | 2 perfis visíveis e operáveis por botão |

### G1.3 — Launcher do Chromium · branch `feat/f1-chromium-launcher`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-109 | Resolver binário do Chromium conforme ADR-003 (caminho configurável em dev) | Chromium localizado ou erro claro com instrução |
| T-110 | Abrir Chromium com `--user-data-dir=/app-data/profiles/{uuid}` + flags de isolamento documentadas (sem flags de spoofing — checklist ético) | Processo abre com o diretório do perfil; flags revisadas e documentadas em `docs/adr/ADR-003` |
| T-111 | Lock local por perfil: impedir segunda abertura do mesmo perfil (lockfile + verificação de PID vivo) | Tentativa dupla retorna erro "perfil em uso" (RF-024; base do MVP-CA-03) |
| T-112 | Encerrar perfil: sinal de encerramento gracioso → timeout → kill; liberar lock; status volta a "disponível" | Após encerrar, nenhum processo filho do perfil vivo (MVP-CA-04); lock liberado |

### G1.4 — Monitoramento e crash recovery · branch `feat/f1-process-monitor`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-113 | Monitor de processo: acompanhar PID do Chromium (polling/evento), detectar saída normal vs crash (exit code) | Kill manual do Chromium é detectado < 5 s e classificado como crash (RF-018) |
| T-114 | Recuperação pós-crash: liberar lock, marcar status "com erro"→"disponível", registrar evento, validar integridade básica do diretório | Cenário 4 da spec: após crash, perfil reabre e sessão persiste (MVP-CA-05) |
| T-115 | Detecção e limpeza de processos órfãos na inicialização do app (PIDs registrados vs vivos) | Cenário 16: app morto com Chromium aberto → próximo start detecta, encerra órfão, corrige locks (RF-025) |
| T-116 | Recuperação de app fechado abruptamente (cenário 3): reconciliar store + locks + processos no boot | Estado consistente após kill do app em qualquer momento do ciclo |

### G1.5 — Logs e eventos locais · branch `feat/f1-local-audit`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-117 | Eventos locais do ciclo de vida: perfil criado/aberto/encerrado/crash/órfão-limpo, com timestamp, uuid do perfil, resultado, correlação | Todos os eventos do ciclo presentes nos logs de um fluxo completo (base do MVP-CA-09) |
| T-118 | Verificação automatizada de ausência de segredos nos logs (scan por padrões: cookie, token, senha, paths de dados sensíveis) | Teste no CI falha se log contiver padrão proibido (MVP-CA-06) |

### G1.6 — Prova de isolamento (gate da F1) · branch `test/f1-isolation-proof`

| ID | Tarefa | Definição de pronto |
|---|---|---|
| T-119 | Teste automatizado de isolamento: abrir perfil A, gravar cookie/LocalStorage em página de teste local, encerrar; abrir perfil B na mesma página → estado ausente; reabrir A → estado presente | Teste roda no CI (ou script reproduzível com evidência gravada); **critério de sucesso da F1 e MVP-CA-01/02** |
| T-120 | Teste de concorrência: tentar abrir o mesmo perfil 2× (mesma máquina) → segunda tentativa falha com mensagem clara | Automatizado; MVP-CA-03 local |
| T-121 | Suíte de crash: cenários 3, 4 e 16 automatizados (kill app, kill Chromium, órfãos) | 3 cenários verdes no CI |
| T-122 | Relatório de evidências da F1 (`docs/evidence/F1_REPORT.md`): resultados dos testes, logs de exemplo, limitações conhecidas | Relatório revisado; **F1 só é declarada concluída com este relatório** (regra da spec: nunca concluir fase sem evidências) |

### Critérios de saída da F1 (resumo)
1. T-119 verde: perfis não compartilham cookies/cache/storage.
2. T-120/T-121 verdes: bloqueio de dupla abertura + recuperação de crash.
3. T-118 verde: zero segredos em logs.
4. T-122 entregue: relatório de evidências aprovado.

### Riscos específicos da F1
- Flags/comportamento do Chromium variam por versão → fixar versão de referência (ADR-003) e testar upgrade cedo.
- Encerramento gracioso do Chromium no Windows é sutil (janelas múltiplas, processos filhos) → T-112 deve enumerar a árvore de processos, não só o PID raiz.
- Escrita atômica no Windows (rename sobre arquivo existente) tem semântica própria → validar T-105 com testes de kill agressivos.
