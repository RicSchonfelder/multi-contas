# Plano — Adaptador MCP do MULTI CONTAS + teste XFCE/YouTube

**Status: planejamento; não implementar sem aprovação.**

## Contexto verificado
- Repositório: `/home/schon/Programas/MULTI CONTAS`.
- Branch atual: `feat/hermes-cdp-comments`.
- A árvore de trabalho já está amplamente modificada e contém arquivos/runtime sensíveis; não fazer reset, stash, commit ou push.
- O app já expõe API HTTP local em `127.0.0.1:29222`, autenticada por `control_token`.
- O limite documentado é de 3 perfis Chrome ativos.
- XFCE/Xvfb é a camada de desktop; não deve ser confundida com o gerenciador de perfis.

## Objetivo
Adicionar um servidor MCP local, preferencialmente em processo separado, que use a API HTTP existente como backend. O MCP não deve substituir nem quebrar as rotas HTTP atuais.

## Decisão proposta
- Transporte: MCP `stdio` em um adaptador Node.js separado, com stdout reservado exclusivamente para JSON-RPC.
- Backend: chamadas HTTP para `http://127.0.0.1:29222` usando token lido localmente; nunca imprimir token, senha, cookies ou conteúdo de arquivos sensíveis.
- Registro: `hermes mcp add multi-contas --command node --args /caminho/absoluto/mcp/index.mjs`, seguido de `hermes mcp test multi-contas` e verificação em `hermes mcp list`.
- Logs, diagnóstico e erros sanitizados somente em stderr.
- Preservar compatibilidade com a API HTTP atual e com `apps/desktop/scripts/orchestrate.mjs`.

## Ferramentas MCP propostas
1. `multi_contas_health` — saúde do backend, recursos e limites.
2. `multi_contas_list_profiles` — lista perfis com ID, nome, status e sessão CDP; sem credenciais.
3. `multi_contas_open_profiles` — recebe lista explícita de IDs, URL opcional e prioridade; nunca abre todos implicitamente; respeita limite 3 e retorna opened/queued/rejected.
4. `multi_contas_close_profile` — fecha um perfil específico; sem `close_all` implícito.
5. `multi_contas_navigate_profile` — navega um perfil já aberto para URL `http/https`, com validação básica.
6. `multi_contas_session_status` — retorna estado de uma sessão/perfil e motivos de falha, sem dados secretos.

Qualquer ferramenta de comentários/publicação fica fora deste primeiro escopo de teste.

## Implementação prevista
- Criar `mcp/` no repositório com `package.json`, `src/index.mjs` ou equivalente e testes.
- Criar cliente HTTP pequeno com timeout, tratamento de 401/403/429/503 e mensagens sanitizadas.
- Validar esquemas de entrada/saída e rejeitar URLs não HTTP(S).
- Atualizar `.gitignore` para excluir tokens, perfis, cookies, logs de runtime e credenciais.
- Adicionar documentação de instalação, configuração e remoção do MCP.
- Não alterar a lógica de alocação de portas CDP sem teste específico; manter o listener reservado durante o spawn.

## Teste seguro no XFCE
Pré-condições:
- Sessão XFCE/X11 disponível e Chrome/Chromium ou Firefox disponível.
- App MULTI CONTAS iniciado pelo usuário; nenhuma credencial será digitada.
- Usar somente vídeos públicos, sem login, comentários, likes, inscrições, upload ou publicação.

URLs públicas encontradas e verificadas via oEmbed:
- `https://www.youtube.com/watch?v=w0rQ2qNAT_E` — “De Assustador a Viral: Como Ana Rinald Fez Sucesso no TikTok Falando de ETs!” — PodCastSorocaba.
- `https://www.youtube.com/watch?v=MGRWD6k0Vyo` — “O Dia em Que Um Cara Brigou de Soco com o Motorista da Nave Alienígena!” — PodCastSorocaba.

E2E:
1. Criar/selecionar dois perfis de teste sem inserir credenciais.
2. Abrir explicitamente os dois perfis pelo MCP.
3. Navegar cada perfil para uma das URLs acima.
4. Capturar estado visual/URL/título de cada janela no XFCE.
5. Confirmar que ambos carregam vídeo público; registrar evidência e fechar apenas os perfis de teste.
6. Se o app Tauri não iniciar no ambiente atual, marcar como bloqueio de GUI, não como falha do adaptador; executar ao menos os testes de compilação e stdio.

## Verificações obrigatórias
- `cargo check`/`cargo test` sem introduzir regressões.
- Testes do adaptador: `initialize`, `tools/list`, chamadas válidas e erros.
- Smoke test JSON-RPC por stdin: stdout contém apenas respostas JSON-RPC; logs ficam em stderr.
- `hermes mcp add` + `hermes mcp test` + `hermes mcp list`.
- Teste de limite: enviar 4 IDs e confirmar rejeição/fila, sem abrir quarto perfil.
- Teste de segurança: nenhum segredo aparece em stdout, stderr, diffs ou mensagens de erro.
- Verificação final de `git diff --check`, `git status` e arquivos sensíveis não rastreados.

## Critérios de aceite
- Hermes lista e invoca as ferramentas MCP.
- Duas sessões/perfis conseguem abrir os dois vídeos públicos no XFCE.
- HTTP existente continua funcionando.
- Nenhum quarto perfil é aberto quando o limite 3 é atingido.
- Processo MCP não contamina stdout.
- Credenciais, cookies, tokens e arquivos runtime não entram no Git nem no chat.

## Riscos e limites
- O app Tauri exige sessão gráfica; Xvfb sem XFCE funcional não comprova o E2E visual.
- YouTube pode alterar UI, exigir consentimento de cookies ou limitar reprodução; isso será reportado como evidência/limitação, sem clicar em login ou permissões.
- A árvore suja pode conter alterações de terceiros; as mudanças desta fase devem ficar isoladas e claramente identificadas.
