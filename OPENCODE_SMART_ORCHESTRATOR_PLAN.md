Tarefa de PLANEJAMENTO — não implemente ainda.

Projeto: `/home/schon/Programas/MULTI CONTAS` (Rust/Tauri + React). O usuário decidiu que o sistema deve permitir no máximo 3 perfis/sessões simultâneas e quer uma forma inteligente de orquestrar tudo.

Leia README, estrutura Rust/React e estado git sem expor segredos de `contas_google.csv` ou arquivos de runtime. Crie um plano técnico em `.planning/phase-smart-orchestrator/PLAN.md` e não altere código nesta fase.

O plano deve cobrir:
1. Limite duro de 3 perfis/Chrome ativos e política de rejeição/queue quando o limite é atingido.
2. Orquestração inteligente: prioridades, fila FIFO/prioridade, jobs por perfil, navegação, comentários e close; nunca abrir todos cegamente.
3. Resource-aware scheduling: medir RAM disponível, load/CPU, número de processos; circuit breaker; backoff; não iniciar novo perfil quando limiares forem excedidos; liberação segura.
4. Separar o conceito de perfis Chrome do XFCE/Xvfb: não criar XFCE por perfil por padrão; opcionalmente associar um display/sessão se necessário. Limite de XFCE = 2 e limite de perfis = 3 deve ser configurável, mas default seguro.
5. API local: compatibilidade com rotas atuais open/open-all/open-all-and-navigate/close, novas rotas/status se necessário, autenticação e sem bind externo.
6. Isolamento de credenciais/cookies e evitar auto-login perigoso; não imprimir segredos.
7. Estado persistente, recuperação após crash, lock por perfil, shutdown graceful, limpeza de processos órfãos.
8. Observabilidade: health, métricas por perfil, motivo de rejeição, logs sanitizados.
9. Testes: Rust unit/integration, frontend, teste do limite 3, fila, circuit breaker, crash recovery e teste de porta CDP sem race.
10. Migração backward-compatible e estratégia de rollout.

Incluir arquivos prováveis, contratos JSON, invariantes, critérios de aceite, riscos e sequência de implementação. Não fazer commit e não alterar código.