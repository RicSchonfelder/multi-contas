No repositório `/home/schon/Programas/MULTI CONTAS`, corrija somente `apps/desktop/src-tauri/tests/isolation_test.rs` para ficar compatível com as APIs atuais de `ProfileManager`, `Profile`, `open`, `close`, `create` e registry. O cargo test real mostrou 25 erros de assinaturas antigas.

Regras:
- Não alterar runtime/src/profile.rs/orquestrador nem arquivos de credenciais.
- Preservar a intenção de cada teste: isolamento/persistência, lock de concorrência, recuperação/órfãos quando a API atual oferecer equivalente. Se uma função antiga não existir, testar a API atual equivalente; não apagar testes silenciosamente.
- Usar `multi_contas_lib` como crate correto.
- Executar somente formatação/sintaxe lógica; cargo pode não estar disponível no host, mas o arquivo deve compilar no container.
- Atualizar `.opencode-smart-orchestrator-status.md` com o bloqueio e a correção proposta, sem commit.
- Não expor credenciais.