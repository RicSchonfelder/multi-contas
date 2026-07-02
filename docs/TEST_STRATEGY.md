# TEST_STRATEGY — Estratégia de Testes (Browser Workspace)

> Derivado de `docs/SPEC_SOURCE.md` (seções "Testes", "Fases", "MVP" e "Prioridade final").
> Regra inegociável: **nenhuma fase é declarada concluída sem evidência de execução dos testes** (ver §9).

---

## 1. Princípios

1. Prioridade da spec: segurança > isolamento > integridade > conformidade > recuperação de falhas > UX > performance. A cobertura de testes segue a mesma ordem.
2. Testes acompanham a fase que entrega a funcionalidade — não existe "fase de testes" separada.
3. Todo bug corrigido ganha teste de regressão antes do merge.
4. Testes determinísticos: sem `sleep` arbitrário, sem dependência de rede externa, relógio e aleatoriedade injetáveis.
5. Segredos nunca em fixtures versionadas; dados de teste sintéticos.

## 2. Pirâmide de testes

```
        ▲  E2E (Playwright)            — poucos, fluxos críticos completos
       ▲▲  Contrato (OpenAPI/schema)   — API pública e IPC desktop↔serviço
      ▲▲▲  Integração (Vitest + testcontainers) — API+PG+Redis+MinIO, sync, leases
    ▲▲▲▲▲  Unitário (Vitest)           — maioria absoluta; rápido, isolado
```

| Camada | Escopo | Ferramentas | Onde roda |
|---|---|---|---|
| Unitário | Funções puras, serviços com deps mockadas, validação (zod), crypto, máquina de estados de perfil/lease | **Vitest** (+ `@vitest/coverage-v8`, `fast-check` para property-based em crypto/sync) | Todo push, < 2 min |
| Integração | API real contra **PostgreSQL 16 + Redis 7 + MinIO** via **testcontainers** (fallback: `docker compose -f compose.test.yml`); migrations aplicadas do zero; worker + filas | Vitest + testcontainers, supertest/fetch nativo | Todo push, < 10 min |
| Contrato | Conformidade da API com `openapi.yaml` (request/response schema, códigos, paginação, erros); IPC desktop↔serviço local validado contra schema compartilhado em `packages/contracts` | Schemathesis ou `openapi-backend` validator + zod schemas de `packages/contracts` como fonte única | Todo push |
| E2E | Desktop (F1+): abertura real de Chromium gerenciado, isolamento, crash; Web-admin (F4+): fluxos de UI | **Playwright** (Electron/Tauri driver p/ desktop; browser p/ web-admin) em runner **Windows** | PR para main + nightly |
| Especiais | Concorrência, recuperação, segurança, migração, performance | ver §5–§8 | nightly + gate de fase |

Convenções: `*.test.ts` (unit), `*.int.test.ts` (integração), `*.contract.test.ts`, `apps/*/e2e/*.spec.ts`.

## 3. Cobertura mínima por pacote

Cobertura medida por linhas + branches (`@vitest/coverage-v8`); gate no CI por pacote (não média global — média esconde buraco em pacote crítico).

| Pacote | Linhas | Branches | Justificativa |
|---|---|---|---|
| `packages/crypto` | **95%** | **90%** | Envelope encryption (DEK/KEK), rotação, revogação — falha aqui é catastrófica |
| `packages/sync-engine` | **90%** | **85%** | Manifests, checksums, retomada, conflito, rollback |
| `packages/browser-core` | **90%** | **85%** | Isolamento de diretórios, lifecycle Chromium, kill de órfãos, detecção de crash |
| `packages/contracts`, `validation` | 90% | 85% | Fonte de verdade de schemas |
| `packages/database` | 85% | 80% | Repositórios, isolamento por org, soft delete, versionamento otimista |
| `apps/api`, `apps/worker` | 85% | 80% | Autorização SEMPRE no backend |
| `apps/desktop` (lógica/serviço local) | 80% | 75% | IPC, lease client, atualização |
| UI (`packages/ui`, telas React) | **60%** | 50% | Coberta principalmente por E2E; unit só p/ lógica de componente |

Regra: PR não pode reduzir cobertura de pacote crítico (crypto, sync-engine, browser-core).

## 4. Testes de multi-tenancy e permissões

- **Isolamento cross-org (integração)**: para cada recurso da API (`profiles`, `networks`, `extensions`, `audit`, ...), teste parametrizado: usuário da org A com token válido tenta `GET/PUT/DELETE` recurso da org B → `404` (não `403`, para não vazar existência). Complementado por testes de RLS direto no PG (RLS é camada ADICIONAL — testar ambas).
- **Matriz papel × ação (integração, gerada por tabela)**: teste data-driven que percorre a matriz completa; qualquer célula não declarada = negado por padrão.

| Ação \ Papel | Superadmin | Admin org | Gerente | Operador | Auditor | Financeiro | Suporte | Somente leitura |
|---|---|---|---|---|---|---|---|---|
| Ver perfil | ✔ | ✔ | ✔ | ✔ (atribuídos) | ✔ | ✖ | ✔ | ✔ |
| Criar/editar perfil | ✔ | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Abrir/encerrar perfil | ✔ | ✔ | ✔ | ✔ (atribuídos) | ✖ | ✖ | ✖ | ✖ |
| Excluir/restaurar perfil | ✔ | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Compartilhar perfil | ✔ | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Gerenciar rede/proxy | ✔ | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Ver segredos (proxy) | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Gerenciar extensões | ✔ | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Ver auditoria | ✔ | ✔ | ✖ | ✖ | ✔ | ✖ | ✔ | ✖ |
| Gerenciar usuários/convites | ✔ | ✔ | ✔ (equipe) | ✖ | ✖ | ✖ | ✖ | ✖ |
| Cobrança/assinatura | ✔ | ✔ | ✖ | ✖ | ✖ | ✔ | ✖ | ✖ |
| API keys / automação | ✔ | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ |
| Encerrar sessão de outro operador | ✔ | ✔ | ✔ | ✖ | ✖ | ✖ | ✖ | ✖ |

(MVP usa 2 papéis — Admin org e Operador — a matriz completa entra na F4; o teste data-driven é o mesmo arquivo, expandindo a tabela.)

## 5. Testes de concorrência (leases, double-open)

Ambiente: integração com PG+Redis reais (testcontainers); simulação de 2+ "dispositivos" como clientes concorrentes no mesmo processo de teste.

| Cenário | Técnica |
|---|---|
| Double-open: N clientes disputam lease do mesmo perfil simultaneamente | `Promise.all` de N aquisições → exatamente 1 sucesso; asserção no banco (1 linha ativa em `profile_leases`) |
| Race de aquisição sob carga | Loop de 100 iterações aquisição/liberação intercaladas; invariante: nunca 2 leases ativos |
| Heartbeat mantém lease; parada de heartbeat expira | Relógio fake/TTL curto; após expiração, segundo cliente adquire |
| Expiração indevida com processo vivo (TC-017) | Cliente detecta perda de lease no heartbeat → alerta + encerramento gracioso, sem corromper dados |
| Encerramento forçado por admin | Admin revoga → cliente recebe sinal, encerra Chromium, evento de auditoria gravado |
| Eventos conflitantes no backend (TC-018) | Dois comandos contraditórios (ex.: `close` + `force-terminate`) → versionamento otimista/idempotência resolve; estado final consistente |

## 6. Testes de recuperação (crash, corrupção)

| Cenário | Técnica |
|---|---|
| App fecha abruptamente | Matar processo do serviço local (`taskkill /F`) com perfil aberto; ao reiniciar: detectar crash, matar Chromium órfão, liberar/recuperar lease, status coerente |
| Chromium trava | Matar processo Chromium; serviço detecta saída anômala, marca perfil "com erro", registra evento, permite reabertura |
| Corrupção de arquivo | Corromper bytes de arquivo do diretório do perfil / do snapshot; checksum detecta antes de abrir/sincronizar; oferece restauração de backup pré-migração ou snapshot |
| Upload interrompido / perda de internet | Cortar conexão no meio do multipart (proxy de teste tipo toxiproxy); retomada continua do offset; falha de sync **não destrói perfil local** (critério MVP 14) |
| Disco cheio | Filesystem pequeno/quota simulada; operações falham com erro claro, sem estado meio-escrito (escrita atômica: temp + rename) |
| Restaurar snapshot | Roundtrip: snapshot → alterar → restaurar → checksums batem |

## 7. Teste de isolamento — 2 perfis não compartilham cookies (F1, automatizado)

Teste E2E Playwright + serviço local real, executado em Windows. **É o critério de sucesso da F1.**

**Pré-condições**: serviço local rodando; Chromium gerenciado instalado; nenhum perfil existente; servidor HTTP de teste local (fixture) que: (a) em `GET /set` grava cookie `probe=<valor>` e escreve `localStorage`/`IndexedDB` via página; (b) em `GET /read` devolve no corpo o cookie recebido e o conteúdo dos storages.

**Passos**:
1. Criar perfil **P1** e perfil **P2** via API do serviço local; asserção: diretórios distintos `profiles/{uuid1}` e `profiles/{uuid2}` criados com permissões corretas.
2. Abrir P1 → navegar para `http://127.0.0.1:<porta>/set?v=AAA` → página grava cookie `probe=AAA`, `localStorage.probe=AAA`, registro IndexedDB `AAA`. Fechar P1 (encerramento normal).
3. Abrir P2 → navegar para `/read`. **Asserções**: cookie `probe` ausente; `localStorage.probe` indefinido; IndexedDB vazio. Navegar para `/set?v=BBB`. Fechar P2.
4. Reabrir P1 → `/read`. **Asserções**: cookie `probe=AAA` presente (persistência — critério MVP 1); nenhum vestígio de `BBB` (critério MVP 2).
5. Reabrir P2 → `/read`. **Asserções**: `probe=BBB`, sem `AAA`.
6. Verificação de disco: cookies SQLite de P1 (`Cookies`) e de P2 são arquivos distintos; nenhum arquivo de sessão compartilhado; cache dirs distintos.
7. Verificação de processos: cada abertura usa `--user-data-dir` exclusivo; ao final, zero processos Chromium restantes.

**Resultado esperado**: todas as asserções passam; relatório Playwright anexado como evidência da F1.

## 8. Testes de segurança (contínuos)

- Segredos fora de logs: teste que percorre saída de logs estruturados após fluxos completos e falha se encontrar padrões (senha, token, cookie, `Authorization`) — critério MVP 6.
- Auth: rate limiting, lockout, rotação de refresh token e detecção de reuso (integração).
- IPC desktop↔serviço: origem inválida/token efêmero expirado → rejeitado.
- `npm audit`/`osv-scanner` + gitleaks no CI (ver DEPLOYMENT.md).

## 9. Evidência de execução (gate de fase)

Uma fase só é declarada concluída com **os dois artefatos**:

1. **Saída de CI**: link permanente para o run verde do GitHub Actions no commit da tag da fase, incluindo job de testes da fase (unit+int+contrato+e2e aplicáveis) e relatório de cobertura publicado como artifact.
2. **Resumo em docs**: arquivo `docs/evidence/F<N>-TEST-EVIDENCE.md` contendo: data, commit SHA, link do run, tabela TC-xxx → status (pass/fail/skip com justificativa), cobertura por pacote crítico, e observações. Sem esse arquivo, a fase permanece aberta.

## 10. Os 18 cenários obrigatórios → casos de teste

| ID | Cenário (spec) | Pré-condições | Passos | Resultado esperado | Camada | Fase |
|---|---|---|---|---|---|---|
| TC-001 | Dois usuários abrem o mesmo perfil | Perfil sincronizado; 2 dispositivos registrados | D1 adquire lease e abre; D2 tenta abrir o mesmo perfil | D2 recebe recusa clara ("em uso por D1"); nenhum segundo processo; auditoria registra tentativa | Integração + E2E | F3 |
| TC-002 | Perda de internet durante sync | Sync em andamento (upload multipart) | Cortar rede (toxiproxy) no meio; restaurar após N s | Sync retoma do offset; sem corrupção; perfil local intacto; status "sincronizando"→"disponível" | Integração | F3 |
| TC-003 | App fecha abruptamente | Perfil aberto, heartbeat ativo | `taskkill /F` no serviço local; reiniciar serviço | Detecção de crash; órfãos mortos; lease recuperado/liberado; perfil reabrível; evento gravado | E2E desktop | F1 (local) / F3 (lease) |
| TC-004 | Chromium trava | Perfil aberto | Matar processo Chromium externamente | Serviço detecta saída anômala; status "com erro"; dados preservados; reabertura funciona | E2E desktop | F1 |
| TC-005 | Arquivo corrompido | Perfil fechado com dados válidos e checksum | Corromper bytes do diretório/objeto de sync | Checksum falha ANTES de abrir/subir; opção de restaurar backup/snapshot; nunca sobrescrever remoto com dado corrompido | Integração | F1 (detecção) / F3 (sync) |
| TC-006 | Upload interrompido | Upload multipart > 1 parte em andamento | Abortar conexão; reiniciar cliente | Retomada multipart do último part confirmado; manifest consistente; sem objetos órfãos no MinIO | Integração | F3 |
| TC-007 | Token revogado | Sessão ativa; admin revoga token/dispositivo | Cliente faz próxima chamada autenticada | `401`; refresh rejeitado (detecção de reuso); cliente desloga; dados locais preservados | Integração | F3 |
| TC-008 | Perda de acesso com perfil aberto | Operador com perfil aberto; admin remove permissão | Propagação da revogação (próximo heartbeat/poll) | Cliente notificado; encerramento gracioso com sync final se permitido; auditoria | Integração + E2E | F4 |
| TC-009 | Versão incompatível | Perfil salvo com formato v(N+1); cliente v(N) | Tentar abrir/baixar perfil | Bloqueio com mensagem clara "atualize o app"; sem migração destrutiva; sem abertura parcial | Unit + Integração | F2 |
| TC-010 | Rede corporativa (proxy) indisponível | Perfil com proxy configurado; proxy fora do ar | Abrir perfil | Teste de conexão falha antes de iniciar Chromium OU abre com aviso conforme política; erro claro; sem vazar tráfego direto se política exigir proxy | Integração + E2E | F2 |
| TC-011 | Acesso cross-org | Usuário org A autenticado; recurso pertence à org B | Chamar todos endpoints com IDs da org B | `404` em todos; RLS bloqueia no PG; evento de segurança registrado em tentativas repetidas | Integração (parametrizado) | F3/F4 |
| TC-012 | Restaurar snapshot | Perfil com ≥2 snapshots | Restaurar snapshot antigo | Estado do perfil = estado do snapshot (checksums); snapshot atual preservado; auditoria | Integração | F3 |
| TC-013 | Atualização falha | Update do app/Chromium em andamento | Injetar falha (checksum errado / corte no download) | Rollback automático para versão anterior funcional; perfis intactos; evento registrado | E2E desktop + Integração | F7 (app) / F2 (Chromium) |
| TC-014 | Disco cheio | Quota/volume pequeno | Criar perfil / sync / snapshot com espaço insuficiente | Erro claro antes de escrita parcial; escrita atômica; sem corrupção; status coerente | Integração | F2 |
| TC-015 | Credencial de proxy alterada | Perfil aberto usando proxy; admin troca senha do proxy | Continuar navegação / reabrir perfil | Falha de auth detectada; UI mascara segredo; re-teste de conexão; senha nunca em log; auditoria da mudança | Integração | F2 |
| TC-016 | Processo órfão | Chromium rodando sem serviço (serviço morto e reiniciado) | Reiniciar serviço local | Varredura identifica Chromium órfão por perfil (pid file/args); kill seguro; lock liberado; temporários limpos | E2E desktop | F1 |
| TC-017 | Lease expira indevidamente | Perfil aberto e saudável; lease expirado no servidor (bug/clock skew) | Heartbeat seguinte falha com "lease perdido" | Cliente NÃO mata dados: encerra graciosamente, preserva estado local, alerta usuário; servidor loga anomalia | Integração | F3 |
| TC-018 | Eventos conflitantes no backend | Dois comandos concorrentes sobre o mesmo perfil (ex.: delete + open; close + force-terminate) | Disparar simultaneamente | Versionamento otimista/locks resolvem; exatamente um vence; outro recebe `409`; estado final consistente e auditado | Integração (concorrência) | F3 |

## 11. Mapeamento para critérios de aceitação do MVP

| Critério MVP | Coberto por |
|---|---|
| 1–2 (persistência/isolamento) | §7 (teste F1) |
| 3 (bloqueio concorrente) | TC-001 |
| 4–5 (encerramento/crash) | TC-003, TC-004, TC-016 |
| 6 (segredos fora dos logs) | §8 |
| 7 (sem permissão = sem abertura) | §4 + TC-008 |
| 8 (validação de rede) | TC-010, TC-015 |
| 9 (auditoria) | asserções de auditoria em todos os TCs |
| 10–11 (testes passando/docs) | §9 (evidência) |
| 12–13 (instalador/update assinado) | E2E de instalação em VM limpa (F7) + TC-013 |
| 14 (sync não destrói local) | TC-002, TC-005, TC-006 |
| 15 (exclusão com recuperação) | teste de soft delete + restauração (integração, F2/F3) |
