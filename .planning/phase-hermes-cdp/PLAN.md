# PLAN — Hermes: Orquestração Multi-Conta via CDP

## 1. Objetivo da Fase

Transformar o modelo **1 Chrome = 1 perfil = porta CDP fixa 29222** em um sistema onde **Hermes** (servidor HTTP local) orquestra **N perfis Chrome simultâneos**, cada um com porta CDP dinâmica, expondo uma API REST com token para automação multi-conta.

**Status atual:** `profile.rs` usa porta fixa 29222 para CDP (apenas quando `creds.is_some()`). `cdp.rs` opera sempre na 29222. `ProfileManager.open()` abre 1 perfil por vez — não há suporte a N simultâneos nem API remota.

## 2. Arquitetura Proposta

```
┌─────────────────────────────────────────────────┐
│  HermesServer (novo módulo hermes.rs)           │
│  ┌─────────────────────────────────────────────┐│
│  │ HTTP listener 127.0.0.1:{porto}            ││
│  │ Bearer token auth                           ││
│  │ Rotas: start, stop, status, exec, list      ││
│  └─────────────────────────────────────────────┘│
├─────────────────────────────────────────────────┤
│  ProfileManager + ProfileRegistry (modificado)  │
│  ┌─────────────────────────────────────────────┐│
│  │ Mapa: profile_id → { cdp_port, child_pid,  ││
│  │   ws_url, status, started_at }              ││
│  └─────────────────────────────────────────────┘│
├─────────────────────────────────────────────────┤
│  Chrome Launcher + Dynamic Port (modificado)    │
│  ┌─────────────────────────────────────────────┐│
│  │ --remote-debugging-port=0 → ler porta real  ││
│  │ do /json/version via HTTP GET               ││
│  └─────────────────────────────────────────────┘│
└─────────────────────────────────────────────────┘
```

## 3. Tasks

### T1: ProfileRegistry — rastreamento de perfis ativos

**Onde:** `profile.rs`

**O quê:** Substituir o lock file manual e `ACTIVE_JOBS` por um `ProfileRegistry` centralizado que mantém estado vivo de todos os perfis abertos:

```rust
struct ActiveProfile {
    profile_id: String,
    cdp_port: u16,
    child_pid: u32,
    ws_url: Option<String>,
    started_at: String,
}

struct ProfileRegistry {
    active: Mutex<HashMap<String, ActiveProfile>>,
}
```

- Mover `ACTIVE_JOBS` (Windows Job Objects) para dentro do registry
- `register(profile_id) -> ActiveProfile`
- `unregister(profile_id)`
- `get(profile_id) -> Option<ActiveProfile>`
- `list_active() -> Vec<ActiveProfile>`
- `is_active(profile_id) -> bool`
- Thread-safe via `Mutex` (já é o padrão do projeto)

**Arquivos afetados:** `profile.rs`

---

### T2: Porta CDP dinâmica por perfil

**Onde:** `profile.rs` (método `open`) + `cdp.rs`

**O quê:** Em vez de `cdp_port = if needs_cdp { 29222 } else { 0 }`:

1. Passar `--remote-debugging-port=0` para o Chrome (SO atribui porta livre)
2. Após spawn, consultar `http://127.0.0.1:{port}/json/version` em loop com retry (até ~5s) para descobrir a porta real
3. Extrair `webSocketDebuggerUrl` do `/json` e armazenar no registry
4. Passar `cdp_port` como parâmetro para todas as funções em `cdp.rs` (já aceitam `port: u16`, mas atualmente chamadas com 29222 hardcoded)

**Problema:** Com `--remote-debugging-port=0`, o Chrome pode alocar uma porta diferente da solicitada. Estratégia:
- Tentar bind em uma porta aleatória (ex: `29223..29999`) antes de passar ao Chrome, guardando o número
- Se falhar, tentar próxima
- Fallback: parsear saída do stderr do Chrome (regex `DevTools listening on ws://127.0.0.1:(\d+)`)

**Mudanças em `ProfileManager.open()`:**
- Mover `needs_cdp` decision logic para fora do `open`, usar parâmetro
- Alocar porta dinâmica
- Armazenar port + ws_url no registry após confirmação
- Retornar o `ActiveProfile` com a porta alocada

**Arquivos afetados:** `profile.rs`, `cdp.rs`

---

### T3: Multi-profile — permitir N perfis abertos simultaneamente

**Onde:** `profile.rs`

**O quê:** Remover a trava implícita de 1-perfil-por-vez:

- `open()` atual verifica `status == "in_use"` e `is_active()`, bloqueando reabertura. Manter essa proteção, mas permitir que perfis **diferentes** sejam abertos concorrentemente.
- Cada perfil ganha seu próprio diretório `--user-data-dir` (já é por UUID, OK)
- Cada perfil ganha seu próprio `--remote-debugging-port` (T2 resolve)
- `close()` precisa encerrar o processo do perfil correto pelo PID (não apenas pelo Job Object no Windows)
- `export_cookies()` e `import_cookies()` — buscar a porta do profile no registry em vez de usar 29222 fixo

**Mudanças no `close()`:**
- No Windows: encerrar o Job Object específico do perfil
- No Mac/Linux: enviar SIGTERM ao `child_pid` armazenado no registry
- Aguardar `child.wait()` com timeout

**Arquivos afetados:** `profile.rs`

---

### T4: Módulo `hermes.rs` — servidor HTTP local

**Novo arquivo:** `src/hermes.rs`

**O quê:** Servidor HTTP síncrono (bloqueante, em thread dedicada) escutando em `127.0.0.1` com:

| Método | Rota | Descrição |
|--------|------|-----------|
| `GET` | `/api/v1/health` | Health check (retorna versão, uptime) |
| `GET` | `/api/v1/profiles` | Lista todos os profiles com status + CDP port |
| `GET` | `/api/v1/profiles/:id` | Status de 1 profile (porta, wsUrl, conexão ativa) |
| `POST` | `/api/v1/profiles/:id/start` | Abre perfil, retorna `{ cdpPort, wsUrl }` |
| `POST` | `/api/v1/profiles/:id/stop` | Fecha perfil |
| `POST` | `/api/v1/profiles/:id/command` | Executa CDP command JSON arbitrário (proxy para WS) |

**Auth:** Header `Authorization: Bearer {token}` em todas as rotas exceto `/health`. Token configurável (padrão: gerado aleatoriamente na inicialização, logado no console).

**Implementação:** Usar `std::net::TcpListener` + parsing HTTP mínimo (ou adicionar `tiny_http` como dependência leve). Considerar:
- `tiny_http` ~ 1ms de compile extra, API robusta
- Manual: 0 deps, mais código e risco
- **Recomendação:** `tiny_http` pela segurança de parsing HTTP

**Threading:** `std::thread::spawn` para o listener, compartilhar `Arc<ProfileRegistry>` via `Arc`.

**CORS:** Não necessário (apenas localhost). Headers mínimos: `Content-Type: application/json`.

**Arquivos afetados:** `Cargo.toml` (+ dep), `src/hermes.rs` (novo), `src/lib.rs` (registrar módulo, iniciar server)

---

### T5: Integração Tauri — iniciar Hermes com o app

**Onde:** `lib.rs` (`run()`)

**O quê:**
1. Inicializar `ProfileRegistry` antes de `ProfileManager`
2. Gerar token aleatório (ou ler de env var `HERMES_TOKEN` / config file)
3. Escolher porta do Hermes (env var `HERMES_PORT` ou 29222 — liberar a porta fixa que antes era do CDP)
4. Spawn `HermesServer` em thread separada antes do `tauri::Builder::default().run()`
5. Logar no console: `HERMES listening on 127.0.0.1:{port} | token: {masked}`
6. Passar `ProfileManager` + `ProfileRegistry` como estado do Tauri (além do Hermes)

**Arquivos afetados:** `lib.rs`

---

### T6: Adaptar `cdp.rs` para multi-perfil

**Onde:** `cdp.rs`

**O quê:** Nenhuma função em `cdp.rs` atualmente recebe `profile_id` — apenas `port`. Como T2 já resolve a porta dinâmica, as funções `auto_login`, `export_cookies`, `import_cookies` e `apply_fingerprint` já são compatíveis desde que chamadas com a porta correta.

**Mudanças concretas:**
- `export_cookies(port, profile_id)` — já recebe `profile_id`, mas usa caminho fixo do `LOCALAPPDATA`. Manter.
- Remover hardcoded `29222` de qualquer lugar. Toda função `cdp.rs` já recebe `port: u16` por parâmetro.
- `auto_login` — está OK, só recebe port + creds

**Arquivos afetados:** `cdp.rs` (nenhuma mudança estrutural, apenas garantir chamadas corretas)

---

### T7: Cross-platform — Windows, Mac, Linux

**Onde:** `profile.rs`

**O quê:**
- `resolve_chrome()` — hoje só busca no Windows (`LOCALAPPDATA`, `Program Files`). **Adicionar**:
  - macOS: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`
  - Linux: `which google-chrome` ou `/usr/bin/google-chrome` ou `google-chrome-stable`
  - `CHROME_PATH` env var (já existe, manter)
- `close()` — implementar kill por sinal Unix (SIGTERM) via `nix::sys::signal`
- `is_active()` — no Linux/Mac, usar `kill(pid, 0)` para verificar se processo existe
- `recover()` — genérico, funciona cross-platform
- Adicionar `nix` crate para signals no Unix (feature-gated)

**Dependência nova:** `nix = { version = "0.29", features = ["signal", "process"], optional = true }` no Cargo.toml, ativada para non-Windows.

**Arquivos afetados:** `profile.rs`, `Cargo.toml`

---

### T8: Endpoint CDP proxy no Hermes

**Onde:** `hermes.rs`

**O quê:** Rota `POST /api/v1/profiles/:id/command` que:
1. Busca `wsUrl` do profile no registry
2. Abre WebSocket para o Chrome via `tungstenite`
3. Envia comando JSON (recebido no body da request)
4. Aguarda resposta (match por `id`)
5. Retorna resultado como JSON

**Formato do body:**
```json
{
  "method": "Runtime.evaluate",
  "params": { "expression": "document.title" },
  "id": 1
}
```

**Resposta:**
```json
{
  "result": { ... }
}
```

Isso permite que o Hermes faça **qualquer comando CDP** — navegação, extração de dados, screenshot, execução de JS — sem precisar de endpoints específicos.

**Arquivos afetados:** `hermes.rs`

---

## 4. Dependências

| Crate | Versão | Onde | Motivo |
|-------|--------|------|--------|
| `tiny_http` | 0.12 | hermes.rs | Servidor HTTP leve (evita async runtime pesado) |
| `nix` (opcional) | 0.29 | profile.rs | Signals Unix para kill graceful (Mac/Linux) |
| `rand` | 0.8 | hermes.rs | Gerar token aleatório (ou usar uuid + hash) |

Alternativa à `tiny_http`: `std::net::TcpListener` + parsing manual (0 deps, mais código). Decidir na implementação.

## 5. Critérios de Saída (Definition of Done)

1. [ ] 3 perfis diferentes abrem simultaneamente, cada um com porta CDP diferente
2. [ ] `netstat -an | findstr 127.0.0.1:29` mostra portas distintas por perfil
3. [ ] `curl -H "Authorization: Bearer {token}" 127.0.0.1:{hermes_port}/api/v1/profiles` lista todos os perfis com status + porta
4. [ ] `POST /api/v1/profiles/:id/command` com `Runtime.evaluate` retorna resultado correto
5. [ ] Fechar perfil via API efetivamente mata o processo Chrome (verificado por `tasklist` / `ps aux`)
6. [ ] Após fechar todos, registry fica vazio
7. [ ] `resolve_chrome()` retorna caminho válido no macOS e Linux (não só Windows)
8. [ ] Token inválido retorna 401 em todas as rotas protegidas
9. [ ] `cargo build` compila sem warnings em Windows (não requer `nix`)
10. [ ] Teste de regressão: `auto_login` continua funcionando com porta dinâmica

## 6. Arquivos a Criar / Modificar

| Arquivo | Ação |
|---------|------|
| `src/hermes.rs` | **CRIAR** — módulo HermesServer |
| `src/profile.rs` | **MODIFICAR** — ProfileRegistry, dynamic port, multi-profile, close cross-platform |
| `src/cdp.rs` | **MODIFICAR** — limpar hardcoded 29222, garantir compatibilidade com porta dinâmica |
| `src/lib.rs` | **MODIFICAR** — init HermesServer, registrar módulo, passar registry |
| `Cargo.toml` | **MODIFICAR** — adicionar `tiny_http`, `nix` (opt), `rand` |

## 7. Não Escopo (para evitar scope creep)

- **Não** implementar fila de comandos CDP
- **Não** implementar rate limiting no Hermes
- **Não** implementar dashboard web (apenas API)
- **Não** implementar persistência de estado do Hermes (volátil, recria se app reiniciar)
- **Não** substituir as Tauri commands existentes — o Hermes é complementar
- **Não** refatorar o módulo de fingerprint (já usa stealth extension, não mexe)
- **Não** adicionar suporte a Edge ou Chromium não-Google
- **Não** adicionar autenticação multifator ou RBAC no Hermes (token único basta)

## 8. Riscos

| Risco | Prob. | Impacto | Mitigação |
|-------|-------|---------|-----------|
| Chrome não expõe /json rápido o suficiente | Média | Médio | Loop com retry (até 5s, 200ms interval) |
| Porta 0 do Chrome difere da solicitada no stderr | Média | Alto | Parsing de stderr + fallback /json/version |
| `tiny_http` deprecated ou sem manutenção | Baixa | Baixo | Alternativa: `std::net::TcpListener` manual |
| Concorrência no SharedRegistry (data race) | Baixa | Alto | Usar `Mutex` (já é padrão no projeto) |
| macOS/Linux: Chrome não encontrado | Média | Alto | `CHROME_PATH` env var + which/where |

## 9. Ordem de Execução Sugerida

```
T1 (ProfileRegistry) ──► T2 (Dynamic Port) ──► T3 (Multi-profile) ──► T4 (Hermes HTTP)
                                                                          │
T7 (Cross-platform) ◄─────────────────────────────────────────────────────┘
  │
  └──► T6 (cdp.rs cleanup) ──► T8 (CDP proxy) ──► T5 (Tauri integration)
```

**Justificativa:** O registry é base para tudo. Porta dinâmica é o maior risco técnico — validar cedo. Multi-profile valida que T1+T2 funcionam. Hermes HTTP só faz sentido depois que multi-profile está operacional. Cross-platform pode ser feito em paralelo com T4-T5. CDP proxy fecha o ciclo de utilidade.
