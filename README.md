# Multi Contas 🖥️

**Gerenciador de Perfis Chromium** — Crie, gerencie e abra múltiplos perfis isolados do Google Chrome, cada um com suas próprias configurações, cookies, sessões e fingerprint.

> Inspirado em ferramentas como Dolphin Anty e GoLogin, mas Open Source e gratuito. App desktop local (sem backend, sem nuvem).

![Tauri](https://img.shields.io/badge/Tauri-2.x-6366F1?logo=tauri)
![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)
![Rust](https://img.shields.io/badge/Rust-1.85+-DEA584?logo=rust)
![License](https://img.shields.io/badge/license-MIT-green)

---

## ✨ Funcionalidades

| Feature | Descrição |
|---------|-----------|
| **Perfis Isolados** | Cada perfil com `--user-data-dir` próprio — cookies, cache e sessões completamente separados |
| **Auto-Login** | Preenche email e senha automaticamente via Chrome DevTools Protocol |
| **Proxy por Perfil** | Suporte a HTTP, HTTPS, SOCKS5 e PAC — configure um proxy diferente para cada perfil |
| **Fingerprint Spoofing** | Mascara WebGL, Canvas, Áudio, Resolução de Tela, Timezone, Geolocalização, CPU, RAM e User-Agent (via extensão stealth) |
| **Cookies** | Importe e exporte cookies no formato Netscape — logue sem precisar de senha |
| **Extensões** | Gerencie extensões Chrome por perfil — ative/desative via toggle |
| **Grupos** | Organize perfis em pastas coloridas para melhor organização |
| **Interface Moderna** | UI escura e responsiva, grid de perfis com busca e filtros |

## 🚀 Como Usar

### Pré-requisitos

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.85+
- [Google Chrome](https://www.google.com/chrome/) instalado
- [pnpm](https://pnpm.io/) (`npm i -g pnpm`)

### Instalação

```bash
# Clone o repositório
git clone https://github.com/RicSchonfelder/multi-contas.git
cd multi-contas

# Instale as dependências
pnpm install

# Inicie o app (modo dev)
cd apps/desktop
pnpm tauri dev
```

Ou execute o `start.bat` na raiz do projeto.

### Desenvolvimento (somente frontend)

Para testar apenas a interface web:

```bash
cd apps/desktop
pnpm dev
```

Acesse `http://localhost:1420` no navegador.

## 🔌 Orquestração via Hermes (API local)

O app expõe um servidor HTTP local para o **Hermes Agent** orquestrar múltiplos perfis Chrome via CDP (Chrome DevTools Protocol), sem precisar abrir a interface.

- **Endereço:** `http://127.0.0.1:29222`
- **Autenticação:** token Bearer (configurado localmente)
- Cada perfil recebe uma **porta CDP dinâmica** (alocada com um listener reservado para evitar race condition ao subir o Chrome).
- **Limite:** máximo de **3 perfis Chrome ativos** simultaneamente (configurável via `orchestrator.json`).
- **Resource-aware:** monitora RAM livre (min 512MB), load average (max 80%) e processos Chrome (max 30) antes de abrir novos perfis.

Rotas principais:

| Método | Rota | Descrição |
|--------|------|-----------|
| `GET` | `/api/v1/health` | Status do servidor (inclui info do orquestrador) |
| `GET` | `/api/v1/profiles` | Lista os perfis (com porta CDP e prioridade) |
| `POST` | `/api/v1/profiles/open-batch` | Abre múltiplos perfis respeitando limite de 3. Aceita `{"profile_ids": [...], "navigate_to": "url", "priority": 5}`. Retorna `opened`, `queued`, `rejected` |
| `GET` | `/api/v1/orchestrator/status` | Estado do orquestrador: ativos, fila, recursos, circuit breakers |
| `POST` | `/api/v1/profiles/:id/open` | Abre um perfil específico (retorna **429** se limite atingido, **503** se recursos insuficientes) |
| `POST` | `/api/v1/profiles/:id/close` | Encerra o perfil |
| `POST` | `/api/v1/profiles/:id/navigate` | Navega o perfil para `{"url": "..."}` |
| `POST` | `/api/v1/profiles/:id/comment` | Comenta `{"text": "..."}` (YouTube live-chat) |

Rotas **DEPRECATED** (mantidas com header `Deprecation: true`):

| Método | Rota | Substituída por |
|--------|------|-----------------|
| `POST` | `/api/v1/profiles/open-all` | `POST /api/v1/profiles/open-batch` |
| `POST` | `/api/v1/profiles/open-all-and-navigate` | `POST /api/v1/profiles/open-batch` com `navigate_to` |

Respostas de erro incluem `reason` codes: `limit_exceeded` (429), `resource_exhausted` (503), `circuit_breaker_open` (503), `profile_cooldown` (503).

### Configuração do Orquestrador

Crie `orchestrator.json` no diretório da aplicação (`%LOCALAPPDATA%/MultiContas/`) para customizar limites:

```json
{
  "max_active_profiles": 3,
  "max_xfce_sessions": 2,
  "min_free_ram_mb": 512,
  "max_load_pct": 80,
  "max_chrome_procs": 30,
  "queue_enabled": true
}
```

Autenticação: header `X-Control-Token` (ou `Authorization: Bearer <token>`), com o token lido de `%LOCALAPPDATA%/MultiContas/control_token`. O servidor escuta **somente em `127.0.0.1`**.

### 🎬 Caso de uso: mesma live em várias contas + comentar de cada uma

Cada perfil aberto recebe uma **porta CDP única e estável** (alocada com um listener
reservado — sem race condition, a janela do Chrome sempre abre). O script de
orquestração `apps/desktop/scripts/orchestrate.mjs` faz o fluxo completo:

```bash
# Abre TODAS as contas e manda cada uma pra live (URL padrão ou a informada)
node apps/desktop/scripts/orchestrate.mjs "https://www.youtube.com/live/g7Qhgx48odw?si=-1M-yjXDW-XtLAku"

# Comenta o mesmo texto em TODAS as contas abertas
node apps/desktop/scripts/orchestrate.mjs comment "Fala galera, live insana!"

# Comenta em UMA conta específica (pelo id)
node apps/desktop/scripts/orchestrate.mjs comment "Oi" <profileId>

# Fecha todas as contas
node apps/desktop/scripts/orchestrate.mjs close-all
```

> ⚠️ **Comentário no YouTube:** o comando `comment` tenta primeiro o live-chat
> (`yt-live-chat-text-input-field-renderer`) e cai na caixa de comentários normal
> se não achar. Só funciona se a **live já tiver iniciado** e a caixa de chat
> estiver visível naquela conta. Requer que cada conta já esteja logada (credenciais
> salvas no perfil).

> Tudo roda localmente na sua máquina — nenhum dado ou comando sai do seu computador.

## 🏗️ Estrutura

```
multi-contas/
├── apps/
│   └── desktop/           # Aplicação Tauri (Rust + React)
│       ├── src/           # Frontend React
│       └── src-tauri/     # Backend Rust
│           ├── src/
│           │   ├── main.rs        # Entrypoint
│           │   ├── lib.rs         # Comandos Tauri (registra a API)
│           │   ├── profile.rs     # Gerenciamento de perfis + abertura do Chrome
│           │   ├── cdp.rs         # Chrome DevTools Protocol (cookies, navegação)
│           │   ├── auth.rs        # Credenciais de login por perfil
│           │   ├── bookmark.rs    # Favoritos por perfil
│           │   ├── extension.rs   # Extensões por perfil
│           │   ├── folder.rs     # Grupos/pastas de perfis
│           │   ├── proxy.rs      # Configuração de proxy por perfil
│           │   └── session.rs    # Sessões ativas
│           └── Cargo.toml
├── packages/
│   ├── contracts/         # Tipos TypeScript compartilhados (contratos)
│   ├── validation/        # Schemas Zod
│   └── logging/           # Logger estruturado
├── .gitignore
├── package.json
├── pnpm-workspace.yaml
└── start.bat
```

## 🔒 Segurança

- ✅ **Nenhuma credencial é enviada para servidores externos** — tudo fica local na sua máquina
- ✅ **Cookies e dados de sessão** armazenados apenas no seu computador
- ✅ **Código aberto** — auditável por qualquer pessoa
- ⚠️ As credenciais salvas nos perfis são armazenadas em texto plano no `profiles.json` local. Em produção, recomenda-se usar o gerenciador de credenciais do sistema.

## 📝 Licença

MIT
