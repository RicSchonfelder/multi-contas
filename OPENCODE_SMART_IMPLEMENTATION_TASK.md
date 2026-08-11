Implemente a Fase 1 do plano `.planning/phase-smart-orchestrator/PLAN.md` no repositório `/home/schon/Programas/MULTI CONTAS` usando DeepSeek.

OBJETIVO: transformar o MULTI CONTAS em um orquestrador seguro com limite máximo de 3 perfis Chrome ativos e resource-aware scheduling. O usuário exige testes reais, tentativa de login/navegação sem expor credenciais e commit GitHub somente de arquivos da mudança, sem segredos.

ESCOPO DESTA FASE (implemente de verdade, não só documente):
1. Criar `apps/desktop/src-tauri/src/orchestrator_config.rs` com configuração local segura e defaults:
   max_active_profiles=3, max_xfce_sessions=2, min_free_ram_mb=512, max_load_pct=80, max_chrome_procs=30, queue_enabled=true. Persistência em diretório da aplicação, sem segredos.
2. Criar `apps/desktop/src-tauri/src/resource.rs` com leitura cross-platform usando sysinfo apenas se já disponível; se não houver dependência, use APIs std/procfs no Linux e fallback conservador. Expor `ResourceSnapshot`, `check_thresholds`, sem panics.
3. Criar `apps/desktop/src-tauri/src/orchestrator.rs` com contador/limite de 3, reason codes, fila de abertura por prioridade/FIFO, circuit breaker simples e jobs serializados por perfil. Evitar bloquear o servidor HTTP.
4. Integrar em `lib.rs`, `hermes.rs`, `registry.rs` e structs necessárias, preservando APIs existentes. `open-all` não pode mais abrir todos cegamente: deve rejeitar/encaminhar ao batch controlado, retornando 429/503 sanitizado quando necessário. Adicionar `/api/v1/profiles/open-batch` e `/api/v1/orchestrator/status` se compatível com a arquitetura atual.
5. DTOs/contratos TypeScript para status e batch; atualizar script `apps/desktop/scripts/orchestrate.mjs` para usar batch e nunca solicitar open-all sem lista explícita.
6. Testes: limite 3, fila/prioridade, resource threshold, circuit breaker, batch, backward compatibility e ausência de password em respostas. Não usar contas reais, tokens, cookies ou publicar.
7. Adicionar um script seguro `scripts/validate-orchestrator.mjs` ou equivalente que execute health/status em servidor local mock/dev e faça smoke test de abertura simulada. Não tentar login real automaticamente. Para teste GUI real, apenas verificar que perfis abrem em ambiente de teste sem inserir credenciais.
8. Atualizar README com configuração e limites.

REGRAS:
- Não ler/imprimir `contas_google.csv`, credenciais, tokens ou cookies.
- Não alterar/deletar `output`, perfis de usuário, arquivos de runtime ou segredos.
- Não usar `git add -A`, reset, stash amplo ou sobrescrever alterações preexistentes.
- Trabalhar somente nos arquivos diretamente necessários; antes de editar, inspecione diffs e preserve alterações existentes.
- Não commitar nesta etapa. Criar `.opencode-smart-orchestrator-status.md` com arquivos, testes e bloqueios.
- Node disponível em `/home/schon/.local/bin/node`; npm em `/home/schon/.local/bin/npm`. Rust/cargo pode estar ausente; registre isso como bloqueio e ainda rode testes TypeScript/Node.
- Se uma mudança Rust não puder ser compilada sem cargo, crie testes unitários/arquivos corretos e registre o bloqueio; não fabrique resultado.

Ao final: execute os testes possíveis, faça `git diff --check` apenas nos arquivos da fase e relate resultados reais. Não faça push.