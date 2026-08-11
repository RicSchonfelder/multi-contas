# Smart Orchestrator — Technical Plan

> **Não implementar ainda.** Documento de planejamento para a fase Smart Orchestrator.
> Base: `OPENCODE_SMART_ORCHESTRATOR_PLAN.md` (19 linhas, 10 seções obrigatórias).
> Projeto: Rust/Tauri + React — gerenciador de perfis Chromium com servidor HTTP local (Hermes).

---

## 1. Limite duro de 3 perfis Chrome ativos + política de rejeição/queue

### Invariantes
- `MAX_ACTIVE_PROFILES = 3` (padrão, configurável via `orchestrator.json`).
- O `ProfileRegistry` atual (em memória) já rastreia ativos; deve ser estendido para **contar e rejeitar**.
- A contagem usa `registry.len()` que é `O(1)` com `HashMap::len()`.

### Política de rejeição
- Quando `registry.len() >= MAX_ACTIVE_PROFILES` e chega uma nova requisição `POST /api/v1/profiles/:id/open`:
  - Resposta `HTTP 429 Too Many Requests` com corpo:
    ```json
    { "error": "max_active_profiles_reached", "limit": 3, "active": ["id1", "id2", "id3"], "retryAfterMs": 5000 }
    ```
  - O cliente (Hermes Agent) deve reenviar após `retryAfterMs`.
- **Sem fila interna por enquanto** — o cliente faz polling/retry. A fila é adicionada no item 2.

### Arquivos afetados
- `apps/desktop/src-tauri/src/orchestrator.rs` (novo) — `OrchestratorConfig`, `check_limit()`.
- `apps/desktop/src-tauri/src/hermes.rs` — chamar `check_limit` antes de `manager.open()`.
- `apps/desktop/src-tauri/src/registry.rs` — adicionar `len()` e `active_ids()`.

---

## 2. Orquestração inteligente: prioridades, fila FIFO/prioridade, jobs

### Modelo de jobs
Cada perfil pode ter jobs enfileirados. Um job é uma operação sobre um perfil Chrome já aberto via CDP:

```rust
struct Job {
    id: String,               // UUID
    profile_id: String,
    job_type: JobType,        // Navigate | Comment | Close | CustomScript
    payload: Value,           // { "url": "...", "text": "...", "script": "..." }
    priority: u8,             // 0 = highest, 255 = lowest
    state: JobState,          // Pending | Running | Done | Failed
    created_at: String,
    started_at: Option<String>,
    error: Option<String>,
}
```

### Fila de jobs
- **FIFO por perfil** (ordem de chegada), mas com **prioridade intra-perfil**.
- Jobs são executados sequencialmente por perfil (um perfil = um Chrome = uma thread CDP).
- Jobs entre perfis diferentes podem rodar em paralelo (cada perfil tem sua própria porta CDP).
- Estrutura: `HashMap<profile_id, VecDeque<Job>>` protegido por `Mutex`.

### Política "nunca abrir todos cegamente"
- Remover comportamento atual de `open-all` e `open-all-and-navigate` que itera sobre **todos** os perfis disponíveis.
- Substituir por `POST /api/v1/profiles/open-batch` com corpo:
  ```json
  { "profile_ids": ["id1", "id2"], "navigate_to": "https://...", "priority": 5 }
  ```
- O endpoint respeita o limite de 3 ativos e retorna os que foram abertos + os que entraram em fila.

### Prioridades
- Configurável por perfil (campo novo `priority: u8` no `Profile`, default 128).
- Na fila global de **abertura** (quando limite atinge 3), perfis de menor prioridade numérica abrem primeiro.

### Arquivos afetados
- `apps/desktop/src-tauri/src/orchestrator.rs` — `Job`, `JobQueue`, `dispatch_job()`.
- `apps/desktop/src-tauri/src/hermes.rs` — novas rotas `POST /api/v1/jobs` (criar job), `GET /api/v1/jobs/:id` (status).
- `apps/desktop/src-tauri/src/profile.rs` — campo `priority` no `Profile`.
- `apps/desktop/src-tauri/src/lib.rs` — `AppState` ganha `Arc<Orchestrator>`.
- `packages/contracts/index.ts` — tipos `JobType`, `JobState`, `Job`, `JobBatchRequest`.

---

## 3. Resource-aware scheduling

### Métricas monitoradas
Usar crate `sysinfo` (já popular, sem nova dependência exótica):

```toml
# Cargo.toml
sysinfo = "0.32"
```

Coletar a cada 5s em thread dedicada:
- **RAM disponível** (bytes livres, não cache/buffer).
- **Load average** (1 min, via `/proc/loadavg` no Linux; via `sysinfo` cross-platform).
- **Número de processos Chrome** (`ps aux | grep chrome | wc -l` via `sysinfo::System::processes()`).

### Limiares (defaults seguros, configuráveis em `orchestrator.json`)
| Métrica | Limiar | Ação |
|---------|--------|------|
| RAM livre | < 512 MB | Não abre novo perfil, retorna 503 `resource_exhausted` |
| Load 1min / cores | > 80% | Não abre novo perfil, retorna 503 `resource_exhausted` |
| Processos Chrome | > 30 | Não abre novo perfil, retorna 503 `resource_exhausted` |

### Circuit breaker
- Se 3 tentativas consecutivas de abrir perfil falharem (CDP timeout, crash do Chrome), o perfil entra em estado `error` com `cooldown_until` (2 min).
- Durante o cooldown, qualquer job para esse perfil retorna `503 profile_cooldown`.
- Após cooldown, reseta para `available`.

### Backoff exponencial
- Para retry de jobs CDP que falham (navegação, comentário): 1s, 2s, 4s, 8s (max 3 retries).
- Para retry de abertura de perfil negada por limite: cliente faz polling com backoff (não implementar retry server-side).

### Liberação segura
- `close` deve: (a) matar o processo Chrome (Windows: `CloseHandle` no Job Object; Linux: `kill -TERM` no PID), (b) remover do registry, (c) limpar lock file, (d) marcar status `available`.
- Timeout de 10s para shutdown; se não responder, `kill -9`.

### Arquivos afetados
- `apps/desktop/src-tauri/src/resource.rs` (novo) — `ResourceMonitor`, `HealthStatus`, `check_thresholds()`.
- `apps/desktop/src-tauri/src/orchestrator.rs` — `CircuitBreaker`, `Backoff`.
- `apps/desktop/src-tauri/src/lib.rs` — iniciar `ResourceMonitor` em thread no `run()`.
- `apps/desktop/src-tauri/src/hermes.rs` — verificar thresholds antes de abrir.

---

## 4. Separação perfis Chrome × XFCE/Xvfb

### Decisão de design
O sistema atual é **Windows-first** (`LOCALAPPDATA`, `windows-sys`). XFCE/Xvfb é relevante apenas para deploys Linux headless. O plano:

1. **Não criar XFCE/Xvfb por perfil por padrão.**
2. Criar abstração opcional `SessionProvider` que pode ser `Headless` (Xvfb) ou `Desktop` (Windows/macOS nativo).
3. Limite de XFCE = 2 (configurável), limite de perfis = 3 (configurável). Ambos no `orchestrator.json`.

### Configuração
```json
// %LOCALAPPDATA%/MultiContas/orchestrator.json
{
  "max_active_profiles": 3,
  "max_xfce_sessions": 2,
  "session_provider": "desktop",
  "xfce_display_base": 100,
  "resource_limits": {
    "min_free_ram_mb": 512,
    "max_load_pct": 80,
    "max_chrome_procs": 30
  }
}
```

### Implementação
- `session_provider` enum: `Desktop` | `Xfce(display_number)`.
- `SessionProvider::launch(profile_id) -> Result<ChildProcess>`.
- `Desktop` usa `Command::new(&chrome)` como hoje.
- `Xfce` wrappa com `Xvfb :{display} -screen 0 1920x1080x24 & DISPLAY=:{display} chrome ...`.
- Pool de displays Xvfb: alocar display 100-199, reutilizar quando liberado.

### Arquivos afetados
- `apps/desktop/src-tauri/src/session.rs` — refatorar para `SessionProvider` trait.
- `apps/desktop/src-tauri/src/orchestrator_config.rs` (novo) — load/save de `orchestrator.json`.
- `apps/desktop/src-tauri/src/profile.rs` — delegar `open()` para `SessionProvider`.
- `apps/desktop/src-tauri/src/lib.rs` — carregar config no startup.

---

## 5. API local: compatibilidade e novas rotas

### Rotas existentes (mantidas, sem breaking changes)
| Método | Rota | Compatibilidade |
|--------|------|-----------------|
| `GET` | `/api/v1/health` | Mantida; adicionar `orchestrator_version`, `active_count`, `resource_status` |
| `GET` | `/api/v1/profiles` | Mantida |
| `POST` | `/api/v1/profiles/:id/open` | Mantida; agora verifica limite, retorna 429 se cheio |
| `POST` | `/api/v1/profiles/:id/close` | Mantida |
| `POST` | `/api/v1/profiles/:id/navigate` | Mantida; agora via job queue |
| `POST` | `/api/v1/profiles/:id/comment` | Mantida; agora via job queue |

### Rotas DEPRECATED (mantidas com warning log, remoção na v0.3)
| Método | Rota | Substituída por |
|--------|------|-----------------|
| `POST` | `/api/v1/profiles/open-all` | `POST /api/v1/profiles/open-batch` |
| `POST` | `/api/v1/profiles/open-all-and-navigate` | `POST /api/v1/profiles/open-batch` com `navigate_to` |

### Novas rotas
| Método | Rota | Descrição |
|--------|------|-----------|
| `POST` | `/api/v1/profiles/open-batch` | Abre N perfis (respeita limite), opcionalmente navega |
| `GET` | `/api/v1/orchestrator/status` | Estado do orquestrador: active, queue, resource, circuit_breakers |
| `POST` | `/api/v1/jobs` | Cria job (navigate/comment/script) para um perfil aberto |
| `GET` | `/api/v1/jobs/:id` | Status de um job |
| `DELETE` | `/api/v1/jobs/:id` | Cancela job pendente |
| `GET` | `/api/v1/jobs?profile_id=:id` | Lista jobs de um perfil |
| `POST` | `/api/v1/profiles/:id/close` | Adicionar `?force=true` para kill -9 |

### Autenticação
- Mantida: `X-Control-Token` ou `Authorization: Bearer <token>`.
- Sem bind externo: servidor escuta **somente** `127.0.0.1` (já implementado).
- Adicionar rate limiting básico: max 60 req/min por token (usando `std::time::Instant` tracking em `HashMap`).

### Contratos JSON (novos)
```json
// POST /api/v1/profiles/open-batch request
{
  "profile_ids": ["uuid1", "uuid2"],
  "navigate_to": "https://youtube.com/live/...",  // opcional
  "priority": 5                                     // opcional, default 128
}

// POST /api/v1/profiles/open-batch response
{
  "opened": [
    { "id": "uuid1", "cdpPort": 9223, "wsUrl": "ws://..." }
  ],
  "queued": [
    { "id": "uuid2", "position": 1, "etaMs": 30000 }
  ],
  "rejected": [],
  "resource_warnings": []
}

// POST /api/v1/jobs request
{
  "profile_id": "uuid1",
  "type": "comment",
  "payload": { "text": "Fala galera!" },
  "priority": 5
}

// GET /api/v1/orchestrator/status response
{
  "active_profiles": 2,
  "max_active_profiles": 3,
  "queue_depth": 1,
  "resources": {
    "free_ram_mb": 3421,
    "load_1m": 1.2,
    "chrome_processes": 5
  },
  "circuit_breakers": [
    { "profile_id": "uuid3", "state": "open", "cooldown_until": "2026-08-09T..." }
  ],
  "xfce_sessions": 0
}
```

### Arquivos afetados
- `apps/desktop/src-tauri/src/hermes.rs` — adicionar handlers para novas rotas, deprecar `open-all`/`open-all-and-navigate`.
- `apps/desktop/src-tauri/src/orchestrator.rs` — lógica dos novos endpoints.
- `apps/desktop/scripts/orchestrate.mjs` — atualizar para usar `open-batch` e job API.
- `packages/contracts/index.ts` — novos tipos.

---

## 6. Isolamento de credenciais/cookies e prevenção de auto-login perigoso

### Situação atual
- Cada perfil tem `--user-data-dir` próprio → cookies/sessões são naturalmente isolados.
- `auto_login()` em `cdp.rs` injeta email/senha via CDP — credenciais em texto plano no `profiles.json`.

### Melhorias planejadas
1. **Sanitização de logs**: nunca logar email/senha. O `hermes.rs` atual já não loga payload; manter.
2. **Sanitização de respostas**: endpoints que retornam perfil nunca incluem `credentials.password`. Usar `#[serde(skip_serializing)]` ou um DTO de resposta sem o campo.
3. **Flag `auto_login_enabled` por perfil**: permite desabilitar auto-login mesmo que credenciais estejam salvas. Default `false` para novos perfis.
4. **Token de controle rotativo**: o `control_token` deve ser regenerável via UI (botão "Regenerar token de controle").
5. **Auditoria de acesso**: logar (em arquivo separado, sanitizado) toda requisição à API de controle: timestamp, perfil acessado, operação, IP de origem (sempre 127.0.0.1).

### Arquivos afetados
- `apps/desktop/src-tauri/src/profile.rs` — campo `auto_login_enabled` no `Profile`.
- `apps/desktop/src-tauri/src/hermes.rs` — DTO de resposta sem password; audit log.
- `apps/desktop/src-tauri/src/cdp.rs` — verificar `auto_login_enabled` antes de `auto_login()`.
- `apps/desktop/src-tauri/src/audit.rs` (novo) — `AuditLogger`.
- `apps/desktop/src-tauri/src/lib.rs` — comando Tauri `regenerate_control_token`.

---

## 7. Estado persistente, recuperação, lock, shutdown graceful, órfãos

### Persistência do estado do orquestrador
- Arquivo `%LOCALAPPDATA%/MultiContas/orchestrator_state.json`:
  ```json
  {
    "updated_at": "2026-08-09T...",
    "active_sessions": {
      "profile_id_1": { "port": 9223, "pid": 12345, "opened_at": "..." }
    },
    "queued_profiles": ["profile_id_2", "profile_id_3"],
    "pending_jobs": [
      { "id": "job1", "profile_id": "profile_id_1", "type": "comment", "state": "running", "payload": {...} }
    ]
  }
  ```

### Recuperação após crash
- Na inicialização (`ProfileManager::recover()` existente + novo `Orchestrator::recover()`):
  1. Carregar `orchestrator_state.json`.
  2. Para cada `active_session`: verificar se o PID ainda existe (já feito em `is_active()`).
  3. Se PID não existe: remover do registry, marcar perfil como `available`.
  4. Para cada `pending_job` com estado `running`: marcar como `failed` com `error: "orchestrator_restart"`.
  5. Para `queued_profiles`: reenfileirar no startup.

### Lock por perfil
- Lock file já existe em `get_profiles_dir().join(id).join("lock")`.
- Adicionar timestamp dentro do lock para detectar locks stale (> 5 min sem heartbeat).
- Heartbeat: thread que a cada 30s atualiza o timestamp do lock file para perfis ativos.
- Se lock está stale, força liberação (mata PID, remove lock).

### Shutdown graceful
- Handler de `SIGTERM`/`SIGINT` (Unix) e `CTRL_CLOSE_EVENT` (Windows):
  1. Para cada perfil ativo: enviar `Browser.close` via CDP (shutdown limpo do Chrome).
  2. Timeout de 5s; se não fechar, `kill -TERM` no PID.
  3. Timeout de 3s; se não morrer, `kill -9`.
  4. Salvar `orchestrator_state.json`.
  5. Remover lock files.
- Registrar handler no `main.rs` com `ctrlc` crate ou `signal_hook`.

### Limpeza de processos órfãos
- Startup scan: listar todos os processos Chrome; para cada um, verificar se o PID está no registry.
- Se não estiver e o `--user-data-dir` aponta para `%LOCALAPPDATA%/MultiContas/profiles/<id>/chromium`: matar.
- Windows: o Job Object já garante que ao fechar o handle, o processo morre. Mas se o app crashar sem `CloseHandle`, o processo fica órfão. Startup scan resolve.

### Arquivos afetados
- `apps/desktop/src-tauri/src/orchestrator.rs` — `recover()`, `save_state()`, heartbeat.
- `apps/desktop/src-tauri/src/profile.rs` — `is_active()` já existe, melhorar com stale detection.
- `apps/desktop/src-tauri/src/main.rs` — signal handler.
- `apps/desktop/src-tauri/src/orphan.rs` (novo) — `cleanup_orphans()`.
- `apps/desktop/src-tauri/Cargo.toml` — `ctrlc = "3"` ou `signal-hook`.

---

## 8. Observabilidade: health, métricas, rejeição, logs sanitizados

### Endpoint de health estendido
`GET /api/v1/health` já existe. Adicionar:
```json
{
  "ok": true,
  "version": "0.1.0",
  "orchestrator": {
    "active_profiles": 2,
    "max_profiles": 3,
    "queue_depth": 0,
    "uptime_seconds": 3600
  },
  "resources": {
    "free_ram_mb": 3421,
    "load_1m": 1.2,
    "load_5m": 0.9,
    "chrome_processes": 5,
    "healthy": true
  }
}
```

### Métricas por perfil (novo endpoint)
`GET /api/v1/profiles/:id/metrics`:
```json
{
  "profile_id": "uuid1",
  "status": "in_use",
  "cdp_port": 9223,
  "pid": 12345,
  "uptime_seconds": 1200,
  "jobs_completed": 5,
  "jobs_failed": 1,
  "circuit_breaker_state": "closed",
  "last_error": null
}
```

### Motivo de rejeição
- Toda resposta de erro agora inclui `reason` (enum) além de `error` (mensagem humana):
  ```json
  { "error": "max_active_profiles_reached", "reason": "limit_exceeded", "limit": 3 }
  ```
- Reasons: `limit_exceeded`, `resource_exhausted`, `circuit_breaker_open`, `profile_not_found`, `profile_already_open`, `auth_failed`, `invalid_request`.

### Logs sanitizados
- Log estruturado em arquivo `%LOCALAPPDATA%/MultiContas/logs/orchestrator.log`:
  ```
  [2026-08-09T10:00:00Z] INFO  profile_open  profile_id=uuid1  port=9223  pid=12345
  [2026-08-09T10:00:05Z] INFO  job_start     job_id=job1  profile_id=uuid1  type=comment
  [2026-08-09T10:00:07Z] WARN  resource_warn free_ram_mb=480  threshold=512
  [2026-08-09T10:00:10Z] ERROR profile_open_rejected  reason=resource_exhausted  free_ram_mb=450
  ```
- **Nunca logar**: email, senha, token de controle, cookies, URLs completas com query params sensíveis.
- Rotacionar a cada 10MB, manter últimos 5 arquivos.

### Arquivos afetados
- `apps/desktop/src-tauri/src/hermes.rs` — estender `/health`, adicionar `/profiles/:id/metrics`.
- `apps/desktop/src-tauri/src/metrics.rs` (novo) — `ProfileMetrics`, coleta.
- `apps/desktop/src-tauri/src/logging.rs` (novo) — `StructuredLogger` com sanitização.
- `apps/desktop/src-tauri/src/lib.rs` — iniciar logger.

---

## 9. Testes

### Rust unit tests (`#[cfg(test)] mod tests` nos próprios arquivos)
| Arquivo | Testes |
|---------|--------|
| `orchestrator.rs` | `test_limit_enforced()`, `test_limit_configurable()`, `test_queue_fifo()`, `test_queue_priority()`, `test_job_lifecycle()`, `test_circuit_breaker_opens()`, `test_circuit_breaker_cooldown()` |
| `resource.rs` | `test_threshold_ram()`, `test_threshold_load()`, `test_threshold_procs()`, `test_healthy_when_all_ok()` |
| `registry.rs` | `test_insert_remove_len()`, `test_active_ids()` |
| `hermes.rs` | `test_auth_required()`, `test_rate_limit()`, `test_open_batch_limit()`, `test_health_endpoint()` |
| `cdp.rs` | `test_port_allocation_no_race()` (já parcialmente coberto pelo `allocate_free_port` com listener vivo) |

### Testes de integração Rust (em `tests/` separado)
| Teste | Descrição |
|-------|-----------|
| `test_three_profile_limit` | Abre 4 perfis sequencialmente, verifica que o 4º recebe 429 |
| `test_queue_and_drain` | Abre 3, enfileira 2, fecha 1, verifica que o 1º da fila abre |
| `test_circuit_breaker_recovery` | Simula 3 falhas CDP, verifica cooldown, espera, verifica reset |
| `test_crash_recovery` | Abre perfil, mata processo Chrome externamente, reinicia orquestrador, verifica recuperação |
| `test_cdp_port_no_race` | Abre 3 perfis simultaneamente, verifica que cada um tem porta CDP única e funcional |

### Frontend tests (Vitest + React Testing Library, já deve existir no projeto)
| Teste | Descrição |
|-------|-----------|
| `OrchestratorStatus` | Renderiza painel com active/max/queue/resources |
| `ProfileLimitWarning` | Mostra alerta quando 3/3 perfis ativos |
| `JobProgress` | Mostra spinner enquanto job está pending/running |

### Testes manuais / scripts de validação
- Script `apps/desktop/scripts/test-orchestrator.mjs`:
  1. `POST /open-batch` com 4 IDs → verifica 3 opened + 1 queued.
  2. `GET /orchestrator/status` → verifica `active_profiles=3`.
  3. `POST /close` em 1 perfil → verifica queue drena.
  4. `POST /jobs` comment → verifica `state: "done"`.
  5. Simula resource exhaustion (seta threshold 99999 MB RAM) → verifica 503.

### Arquivos afetados
- `apps/desktop/src-tauri/src/orchestrator.rs` — `#[cfg(test)] mod tests`.
- `apps/desktop/src-tauri/src/resource.rs` — `#[cfg(test)] mod tests`.
- `apps/desktop/src-tauri/tests/` (novo diretório, se ainda não existir) — testes de integração.
- `apps/desktop/src/__tests__/` — testes de frontend.
- `apps/desktop/scripts/test-orchestrator.mjs` — script de validação manual.

---

## 10. Migração backward-compatible e estratégia de rollout

### Compatibilidade
- **Todas as rotas existentes continuam funcionando** com os mesmos contratos.
- `open-all` e `open-all-and-navigate` são **deprecated**, não removidos. Retornam header `Deprecation: true` e aviso no response body.
- `profiles.json` ganha campos novos com `#[serde(default)]` → backward-compatible.
- `orchestrator.json` é novo; se ausente, usa defaults internos (3 perfis, sem XFCE).
- `control_token` continua no mesmo local.

### Estratégia de rollout
1. **Fase 0** (este plano): planejamento — **não alterar código**.
2. **Fase 1**: Infraestrutura base
   - Criar `orchestrator.rs`, `orchestrator_config.rs`, `resource.rs` com defaults.
   - Limite duro de 3 ativos (item 1).
   - Resource monitoring básico (item 3, sem circuit breaker ainda).
   - Health endpoint estendido (item 8 parcial).
   - **Sem breaking changes.**
3. **Fase 2**: Jobs + fila
   - Job queue, prioridades (item 2).
   - `open-batch` endpoint.
   - Circuit breaker + backoff (itens 3 restantes).
   - Deprecar `open-all`/`open-all-and-navigate`.
   - **Breaking changes: nenhum. Rotas antigas ainda funcionam.**
4. **Fase 3**: Robustez
   - Persistência de estado, crash recovery, shutdown graceful, órfãos (item 7).
   - Session provider + XFCE opcional (item 4).
   - Logs sanitizados, métricas completas (item 8 restante).
   - Isolamento de credenciais (item 6).
   - **Breaking changes: nenhum.**
5. **Fase 4**: Testes + docs
   - Testes Rust unit/integration (item 9).
   - Script de validação.
   - Atualizar README.
   - **Breaking changes: remover `open-all` e `open-all-and-navigate` (se todos os clientes migraram).**

### Git strategy
- Branch: `feature/smart-orchestrator`.
- Cada fase = um PR revisável.
- Commits atômicos por arquivo/módulo.
- Nunca commitar `orchestrator.json` ou `control_token` — estão no `.gitignore`.

---

## Resumo de arquivos novos e alterados

### Novos arquivos Rust (`apps/desktop/src-tauri/src/`)
| Arquivo | Responsabilidade |
|---------|-----------------|
| `orchestrator.rs` | Config, limite, job queue, circuit breaker, dispatch, recover |
| `orchestrator_config.rs` | Load/save `orchestrator.json` |
| `resource.rs` | `ResourceMonitor`, thresholds, health check |
| `metrics.rs` | `ProfileMetrics`, coleta, exposição via API |
| `logging.rs` | `StructuredLogger` com sanitização |
| `audit.rs` | `AuditLogger` para API de controle |
| `session_provider.rs` | Trait + Desktop/Xfce implementations |
| `orphan.rs` | `cleanup_orphans()` |
| `signal.rs` | Signal handlers (SIGTERM/SIGINT) |

### Alterados
| Arquivo | Mudanças |
|---------|----------|
| `hermes.rs` | Novas rotas, deprecation, rate limit, reason codes, DTO seguro |
| `profile.rs` | Campos `priority`, `auto_login_enabled` no Profile |
| `registry.rs` | `len()`, `active_ids()` |
| `cdp.rs` | Verificar `auto_login_enabled` antes de auto-login |
| `lib.rs` | `AppState` com `Orchestrator`; iniciar monitor, logger, recover |
| `main.rs` | Signal handler setup |
| `Cargo.toml` | Dependências: `sysinfo`, `ctrlc`/`signal-hook` |

### Alterados não-Rust
| Arquivo | Mudanças |
|---------|----------|
| `packages/contracts/index.ts` | Tipos `Job`, `JobType`, `JobState`, `OpenBatchRequest`, `OrchestratorStatus`, `ReasonCode` |
| `apps/desktop/scripts/orchestrate.mjs` | Usar `open-batch` e job API |
| `apps/desktop/src/` (React) | Componente `OrchestratorStatus`, indicador de limite, job progress |
| `README.md` | Documentar novas rotas, limites, resource awareness |

---

## Riscos

| Risco | Probabilidade | Impacto | Mitigação |
|-------|---------------|---------|-----------|
| `sysinfo` não compilar cross-platform (Windows + Linux) | Média | Alto | Crate maduro; testar em CI Windows e Linux desde a Fase 1 |
| Chrome não responder a `Browser.close` via CDP | Alta | Baixo | Timeout + fallback para `kill -TERM` → `kill -9` |
| Race condition no registry entre thread Tauri e thread Hermes | Baixa | Alto | `Mutex<HashMap>` já protege; manter single-writer |
| Limite 3 ser muito restritivo para alguns usuários | Média | Baixo | Configurável via `orchestrator.json`; documentar como aumentar |
| Job queue crescer indefinidamente | Baixa | Médio | Limite de 100 jobs pendentes; recusar novos com 429 |
| Vazamento de credenciais em logs de erro | Média | Alto | Sanitização no `StructuredLogger`; code review obrigatório em paths de erro |
| XFCE/Xvfb não disponível em todos os Linux | Alta | Baixo | `SessionProvider` é opcional; default é Desktop; XFCE só ativa se configurado |

---

## Critérios de aceite

1. **Limite**: 4º `POST /open` retorna 429 enquanto 3 perfis ativos.
2. **Fila**: Ao fechar 1 perfil, o próximo da fila abre automaticamente em ≤ 5s.
3. **Resource**: Com < 512MB RAM livre, `POST /open` retorna 503.
4. **Circuit breaker**: Após 3 falhas CDP consecutivas, perfil entra em cooldown de 2 min.
5. **Recovery**: Após `kill -9` no processo do app e restart, perfis órfãos são limpos e estado é recuperado.
6. **Backward compat**: Script `orchestrate.mjs` existente funciona sem alterações (rotas deprecated ainda respondem).
7. **Sanitização**: Nenhum log contém `password`, `email`, `token` ou cookie value.
8. **Isolamento**: Dois perfis abertos simultaneamente não compartilham cookies/sessão.
9. **Shutdown graceful**: Ao receber SIGTERM, todos os Chromes fecham limpo em ≤ 10s.
10. **Porta CDP sem race**: 3 perfis abertos simultaneamente cada um com porta CDP única e funcional.

---

## Sequência de implementação

```
Fase 1 (infra):
  orchestrator_config.rs → orchestrator.rs (limite) → resource.rs
  → hermes.rs (429/503 + health) → registry.rs (len)

Fase 2 (jobs):
  orchestrator.rs (Job/Queue/CircuitBreaker/Backoff)
  → hermes.rs (open-batch, jobs CRUD) → deprecate open-all

Fase 3 (robustez):
  orchestrator.rs (recover, save_state) → orphan.rs
  → signal.rs → session_provider.rs → logging.rs → audit.rs → metrics.rs

Fase 4 (testes + docs):
  tests/ → scripts/test-orchestrator.mjs → frontend components → README
```
