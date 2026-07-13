# Multi Contas 🖥️

**Gerenciador de Perfis Chromium** — Crie, gerencie e abra múltiplos perfis isolados do Google Chrome, cada um com suas próprias configurações, cookies, sessões e fingerprint.

> Inspirado em ferramentas como Dolphin Anty e GoLogin, mas Open Source e gratuito.

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
| **Fingerprint Spoofing** | Mascara WebGL, Canvas, Áudio, Resolução de Tela, Timezone, Geolocalização, CPU, RAM e User-Agent |
| **Cookies** | Importe e exporte cookies no formato Netscape — logue sem precisar de senha |
| **Extensões** | Gerencie extensões Chrome por perfil — ative/desative via toggle |
| **Grupos** | Organize perfis em pastas coloridas para melhor organização |
| **Interface Moderna** | UI escura e responsiva, grid de perfis com busca e filtros |

## 🚀 Como Usar

### Pré-requisitos

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://www.rust-lang.org/) 1.85+
- [Google Chrome](https://www.google.com/chrome/) instalado

### Instalação

```bash
# Clone o repositório
git clone https://github.com/RicSchonfelder/multi-contas.git
cd multi-contas

# Instale as dependências
npm exec -- pnpm install

# Inicie o app
cd apps/desktop
npm exec -- pnpm tauri dev
```

Ou execute o `start.bat` na raiz do projeto.

### Desenvolvimento (sem Tauri)

Para testar apenas a interface web:

```bash
cd apps/desktop
npm exec -- pnpm dev
```

Acesse `http://localhost:1420` no navegador.

## 🏗️ Estrutura

```
multi-contas/
├── apps/
│   └── desktop/           # Aplicação Tauri (Rust + React)
│       ├── src/           # Frontend React
│       └── src-tauri/     # Backend Rust
│           ├── src/
│           │   ├── main.rs      # Entrypoint
│           │   ├── lib.rs       # Comandos Tauri
│           │   ├── profile.rs   # Gerenciamento de perfis
│           │   └── cdp.rs       # Chrome DevTools Protocol
│           └── Cargo.toml
├── packages/
│   ├── contracts/         # Tipos TypeScript compartilhados
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
