# THREAT_MODEL — Modelo de Ameaças STRIDE (Browser Workspace)

> Derivado de `docs/SPEC_SOURCE.md` e `docs/SECURITY_MODEL.md`. Metodologia: STRIDE por componente + ameaças específicas nomeadas na spec.
> Legenda de status: **MVP** = mitigação obrigatória no MVP Windows; **Planejado** = fase F3–F7; **Futuro** = pós-1.0.
> Impacto/Probabilidade: A (alto), M (médio), B (baixo). Risco = combinação qualitativa.

## 1. Escopo e componentes

```
[Desktop UI] ⇄ IPC ⇄ [Serviço Local] ⇄ [Processos Chromium] ⇄ [Diretórios de Perfil]
      │                     │
      └──────── HTTPS ──────┴──> [Sync Client] ⇄ [API Cloud] ⇄ [PostgreSQL/Redis] / [Object Storage]
                                                     │
                                        [Painel Admin] / [Webhooks] / [API pública]
```

Atores de ameaça considerados: atacante externo remoto; usuário malicioso da plataforma (abusador); operador malicioso interno à org do cliente; funcionário malicioso da plataforma; malware no host do usuário; atacante com acesso físico à máquina; concorrente/tenant vizinho; dependência/fornecedor comprometido.

---

## 2. STRIDE por componente

### 2.1 Desktop UI (Tauri/Electron + React)

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| UI-1 | Spoofing | App falso/repackaged distribuído fora do canal oficial captura credenciais | A | M | Assinatura Authenticode, download só do site oficial, verificação de assinatura no auto-update, comunicação clara de canais oficiais | MVP |
| UI-2 | Tampering | XSS/injeção na UI (renderer) leva a execução no contexto do app | A | M | CSP estrita, sem `eval`, contextIsolation/isolamento do renderer, sanitização de dados vindos da API, sem carregamento de conteúdo remoto no shell | MVP |
| UI-3 | Repudiation | Ação disparada da UI sem trilha | M | M | Toda mutação passa pela API e gera `audit_events` (UI nunca grava direto) | MVP |
| UI-4 | Info Disclosure | Tokens/segredos expostos em logs da UI, crash dumps ou devtools | A | M | Redaction central (§6 SECURITY_MODEL), devtools desabilitado em produção, access token só em memória, crash reports sem payloads | MVP |
| UI-5 | DoS | UI travada por payloads enormes (listas de milhares de perfis) | B | M | Paginação server-side, virtualização de tabela, limites de payload | MVP |
| UI-6 | EoP | UI "esconde" botão mas API aceita ação sem permissão | A | M | Autorização exclusivamente no backend (P3); testes de permissão por endpoint (cenário #7/#11) | MVP |

### 2.2 Serviço local (daemon/profile manager)

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| SL-1 | Spoofing | Processo local arbitrário conecta ao IPC e comanda perfis | A | M | Loopback/named pipe com ACL por SID, token efêmero por boot, validação de PID→binário→assinatura do peer | MVP |
| SL-2 | Tampering | Binário do serviço substituído em disco (persistence de malware) | A | M | Instalação em `Program Files` (ACL admin), assinatura verificada no update, integridade verificada pela UI no handshake | MVP |
| SL-3 | Repudiation | Operações locais (abrir perfil, limpar dados) sem registro | M | M | Log local estruturado + espelhamento de eventos ao backend quando online; fila offline com envio posterior | MVP |
| SL-4 | Info Disclosure | Token IPC vazado via arquivo temporário legível por outros usuários da máquina | A | B | Handshake in-band no pipe; se arquivo for usado, ACL restrita ao usuário + apagar após leitura | MVP |
| SL-5 | DoS | Flood de requisições IPC ou processos órfãos esgotam recursos | M | M | Rate limit local, kill de órfãos, watchdog de processos Chromium (cenário #16) | MVP |
| SL-6 | EoP | Serviço rodando com privilégio alto vira vetor de escalação local | A | M | Rodar como usuário logado (não SYSTEM); se serviço de sistema for necessário, superfície mínima e sem exec arbitrário | MVP |

### 2.3 Processos Chromium gerenciados

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| CH-1 | Spoofing | Chromium adulterado/trojanizado executado como se fosse o gerenciado | A | M | Verificação HTTPS+checksum SHA-256+assinatura no download; hash do binário validado antes de cada launch; versão+hash auditados | MVP |
| CH-2 | Tampering | Extensão não autorizada injetada no perfil rouba sessões | A | M | Catálogo corporativo aprovado, bloqueio de extensões fora do catálogo, verificação de integridade, exibição de permissões, auditoria | MVP |
| CH-3 | Tampering | Flags de linha de comando manipuladas (ex.: `--remote-debugging-port`) abrem porta de debug que expõe cookies | A | M | Serviço local monta a linha de comando (usuário não injeta flags); debugging port só em modo automação autorizada, loopback, com token, auditado | MVP |
| CH-4 | Info Disclosure | Chromium desatualizado com CVE explorado por site malicioso | A | A | Canal de atualização de Chromium, bloqueio de versões inseguras (lista assinada via API), telemetria de versão | MVP |
| CH-5 | DoS | Crash do Chromium corrompe perfil | M | A | Detecção de crash, restauração segura, backup pré-migração, checksums (cenários #4/#5) | MVP |
| CH-6 | EoP | Sandbox escape do Chromium compromete o host | A | B | Manter Chromium atualizado (CH-4); sandbox nunca desabilitada (`--no-sandbox` proibido) | MVP |

### 2.4 Diretórios de perfil locais

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| DP-1 | Info Disclosure | **Roubo do diretório de perfil** (cópia por outro usuário da máquina, backup exfiltrado, notebook furtado) → sessões roubadas | A | M | ACL NTFS restrita ao usuário; segredos da plataforma nunca no diretório; cookies protegidos por DPAPI (Chromium/Windows); documentar limite honesto: quem executa código como o usuário lê o que o usuário lê; futuro: cifrar diretório quando perfil fechado + BitLocker recomendado nos requisitos | MVP (ACL/DPAPI) / Futuro (cifra em repouso do diretório) |
| DP-2 | Tampering | Corrupção ou adulteração de arquivos do perfil | M | M | Checksums no manifest, detecção de corrupção, backup pré-migração, versão de formato, restauração de snapshot (cenário #12) | MVP |
| DP-3 | Tampering | Dois processos escrevem no mesmo diretório | A | M | Bloqueio concorrente local (lockfile + verificação de PID) além do lease distribuído | MVP |
| DP-4 | Info Disclosure | Temporários/downloads vazam dados após exclusão do perfil | M | M | Limpeza de temporários, exclusão do diretório com verificação, downloads fora do sync por padrão | MVP |
| DP-5 | Repudiation | Não saber qual usuário/dispositivo tocou o perfil | M | B | Campos "última abertura por usuário/dispositivo" + auditoria de abertura/encerramento | MVP |

### 2.5 Sync client

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| SY-1 | Spoofing/MITM | **MITM no sync** (rede hostil, proxy TLS corporativo malicioso) lê ou altera blobs | A | M | TLS 1.2+ com validação estrita de cadeia; blobs cifrados fim-a-fim do cliente ao storage (AES-256-GCM antes do upload) — MITM vê só ciphertext; AAD amarra blob a org/perfil/versão | Planejado (F3) |
| SY-2 | Tampering | Servidor/storage devolve blob antigo (rollback attack) ou de outro perfil | A | B | Versionamento no manifest assinado pelo servidor + AAD com versão; cliente rejeita versão menor que a conhecida | Planejado (F3) |
| SY-3 | Repudiation | Conflito de sync sem trilha de quem sobrescreveu | M | M | Manifests versionados, detecção de conflito com registro, snapshots com autor | Planejado (F3) |
| SY-4 | Info Disclosure | Metadados de sync vazam conteúdo (nomes de arquivos internos) | B | M | Minimizar metadados em claro; nomes de objetos = UUIDs | Planejado (F3) |
| SY-5 | DoS | Upload interrompido / perda de internet deixa perfil inconsistente | M | A | Multipart com retomada, transação de manifest (só publica versão completa), sync falho nunca destrói perfil local (critério MVP #14, cenários #2/#6) | Planejado (F3) |
| SY-6 | EoP | **Lease hijacking**: atacante força expiração/rouba token de lease e abre perfil em uso | A | M | Token de lease vinculado a device+sessão, aquisição atômica, heartbeat, recuperação pós-falha com reconciliação, auditoria de todas as transições (cenários #1/#17/#18) | Planejado (F3) |

### 2.6 API Cloud

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| AP-1 | Spoofing | Credential stuffing / brute force / tokens roubados | A | A | Argon2id, MFA TOTP, rate limiting em camadas, rotação de refresh com detecção de reuso (revoga família), notificação de novo acesso | MVP |
| AP-2 | Tampering | Injeção (SQLi, mass assignment), IDOR | A | M | Queries parametrizadas, validação por schema, DTOs explícitos (sem bind de payload direto em modelo), UUIDs, 404 uniforme | MVP |
| AP-3 | Repudiation | Ações via API key sem atribuição | M | M | API keys com dono/escopo, `audit_events` para todo uso mutável, tokens revogáveis | Planejado (F5) |
| AP-4 | Info Disclosure | **Cross-tenant access**: falha de filtro por org expõe dados de outro cliente | A | M | Middleware obrigatório de escopo por org + RLS no Postgres como segunda camada + testes automatizados de cross-org (cenário #11) em CI | MVP |
| AP-5 | Info Disclosure | API expõe cookies/senhas/sessões | A | B | Proibição de design (spec §API): nenhum endpoint serializa esses campos; testes de contrato verificam ausência | MVP |
| AP-6 | DoS | Abuso de endpoints caros (sync, export, busca) | M | M | Rate limits por token/plano, paginação obrigatória, filas para trabalhos pesados, limites do EntitlementService | Planejado (F3) |
| AP-7 | EoP | Elevação por manipulação de papel/convite (aceitar convite forjado, alterar próprio papel) | A | M | Convites com token de uso único e org fixa; mudança de papel exige `member.manage` + não permitir auto-elevação; auditoria | Planejado (F4) |

### 2.7 Banco de dados (PostgreSQL/Redis)

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| DB-1 | Spoofing | Credenciais de banco vazadas dão acesso direto | A | M | Segredos em vault, rotação, rede privada (sem exposição pública), TLS, credenciais distintas por ambiente | MVP |
| DB-2 | Tampering | UPDATE/DELETE em `audit_events` apaga rastros | A | B | Append-only (privilégios + trigger), hash chain com âncora WORM | Planejado (F4) |
| DB-3 | Info Disclosure | Dump do banco expõe segredos | A | B | TOTP secrets e credenciais de proxy cifrados em nível de aplicação (inúteis sem KMS); backups cifrados | MVP |
| DB-4 | DoS | Migração destrutiva ou disco cheio | M | M | Migrations reversíveis, revisão obrigatória, monitoramento de storage (cenário #14), backups testados | MVP |
| DB-5 | EoP | Aplicação conecta como owner e bypassa RLS | A | M | Role de aplicação sem `BYPASSRLS`/ownership; RLS ativa; revisão de grants em CI | Planejado (F3) |

### 2.8 Object storage (S3-compat)

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| OS-1 | Info Disclosure | Bucket público/URL adivinhável expõe blobs | A | M | Buckets privados, acesso só via URLs pré-assinadas curtas emitidas pela API após checagem de permissão; chaves de objeto UUID | Planejado (F3) |
| OS-2 | Info Disclosure | Operador da nuvem/insider lê blobs | A | B | Envelope encryption cliente-side: storage só vê ciphertext | Planejado (F3) |
| OS-3 | Tampering | Blob substituído no storage | A | B | GCM (integridade autenticada) + checksum no manifest; falha de auth de decrypt = rejeitar e alertar | Planejado (F3) |
| OS-4 | DoS | Exaustão de storage por upload abusivo | M | M | Quotas por org (EntitlementService), limites de tamanho por objeto/manifest | Planejado (F6) |

### 2.9 Painel admin (web)

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| AD-1 | Spoofing | Conta de admin da plataforma comprometida | A | M | MFA obrigatório para staff, sessões curtas, allowlist de rede/VPN para o painel, alertas de login | Planejado (F4) |
| AD-2 | EoP | **Admin da plataforma abusa de impersonation** para acessar dados de cliente | A | M | Justificativa + aprovação especial + TTL 1h + banner + bloqueio de rotas de segredo em sessão impersonada + auditoria + notificação à org afetada | Planejado (F4) |
| AD-3 | Tampering | CSRF/XSS no painel executa ações administrativas | A | M | CSP, SameSite, CSRF tokens, template escaping, revisão de dependências front | Planejado (F4) |
| AD-4 | Repudiation | Ações administrativas sem trilha | A | B | Sessões admin auditadas integralmente (spec: "sessões admin"), hash chain | Planejado (F4) |
| AD-5 | Info Disclosure | Suporte vê segredos/conteúdo de perfil em telas de diagnóstico | M | M | Papel Suporte sem `secret.view`; telas de diagnóstico com dados minimizados | Planejado (F4) |

### 2.10 Webhooks

| ID | STRIDE | Ameaça | Impacto | Prob. | Mitigação | Status |
|----|--------|--------|---------|-------|-----------|--------|
| WH-1 | Spoofing | Consumidor não consegue distinguir webhook forjado | M | M | Assinatura HMAC-SHA-256 por endpoint + timestamp anti-replay ±5 min; docs de verificação no SDK | Planejado (F5) |
| WH-2 | Info Disclosure | Payload de webhook contém dados sensíveis e vaza no destino | M | M | Payloads mínimos (IDs + tipo de evento; consumidor busca detalhes via API autenticada) | Planejado (F5) |
| WH-3 | SSRF | URL de webhook aponta para rede interna/metadata endpoint | A | M | Validação de destino (bloquear RFC1918, link-local, metadata), resolução DNS re-checada no envio | Planejado (F5) |
| WH-4 | DoS | Endpoint lento do cliente trava filas | B | M | Timeouts curtos, retries com backoff, circuit breaker por endpoint, fila isolada | Planejado (F5) |

---

## 3. Ameaças transversais específicas (exigidas pela spec)

| ID | Ameaça | Descrição | Impacto | Prob. | Mitigação | Status |
|----|--------|-----------|---------|-------|-----------|--------|
| T-1 | Roubo de diretório de perfil local | Cópia física/lógica de `/app-data/profiles/{uuid}/` dá as sessões salvas | A | M | Ver DP-1. Complementos: sessão de trabalho remoto encerra perfis ao logout; recomendação formal de disco cifrado (BitLocker) na documentação de requisitos corporativos; revogação de dispositivo dispara rotação de DEK | MVP parcial / Futuro |
| T-2 | Malware no host | Keylogger/infostealer no Windows do usuário lê tudo que o usuário lê | A | M | Limite honesto: **fora do modelo de proteção total**. Reduzir raio: segredos em Credential Manager (não em arquivos), tokens de vida curta, detecção de reuso de refresh, revogação remota de dispositivo, notificação de novo acesso, MFA. Documentar explicitamente aos clientes | MVP (mitigação parcial) |
| T-3 | Operador malicioso interno (org do cliente) | Operador legítimo exfiltra sessões/segredos da sua org | A | M | Menor privilégio (Operador não vê segredos), sem export aberto de cookies, auditoria completa de aberturas/exportações, revogação imediata ao desligar membro, encerramento forçado de sessões (`session.terminate_others`), alertas de comportamento anômalo | Planejado (F4) |
| T-4 | Admin da plataforma abusando de impersonation | Ver AD-2 | A | M | Controles AD-2 + revisão periódica dos logs de impersonation por segunda pessoa | Planejado (F4) |
| T-5 | Cross-tenant access | Ver AP-4/DB-5 | A | M | RBAC backend + escopo org obrigatório + RLS + testes cross-org em CI + UUIDs/404 uniforme | MVP |
| T-6 | Lease hijacking | Ver SY-6 | A | M | Lease atômico com token vinculado, heartbeat, reconciliação, auditoria | Planejado (F3) |
| T-7 | MITM no sync | Ver SY-1/SY-2 | A | M | TLS estrito + cifra cliente-side + anti-rollback | Planejado (F3) |
| T-8 | Dependência comprometida | Pacote npm/crate malicioso entra no build | A | M | Lockfiles congelados, audit bloqueante, `--ignore-scripts`+allowlist, SBOM, secret scanning, builds só em CI, revisão de bumps de deps críticas | MVP |
| T-9 | Binário Chromium adulterado | Ver CH-1 | A | M | HTTPS de origem fixa + SHA-256 + assinatura + hash validado a cada launch + lista de versões bloqueadas assinada | MVP |
| T-10 | **Abuso do produto para fraude** | Cliente usa a plataforma para fazendas de contas, evasão de antifraude, golpes | A (legal/reputacional) | A | Por design: sem manipulação de fingerprint, sem export de cookies, automação com identificação obrigatória. Controles: allowlist de automação aprovada por admin da org, auditoria imutável, AUP com enforcement gradativo, detecção de padrões de abuso (volume anômalo de perfis/proxies/automação) com fila de revisão humana, KYC de cobrança, canal de denúncia, botão de emergência de automação | MVP (design) / Planejado (detecção F5–F6) |
| T-11 | Update do app comprometido | Servidor de update invadido distribui versão maliciosa | A | B | Assinatura Authenticode verificada pelo updater (chave fora do servidor de update), anti-downgrade, rollout gradual com telemetria, rollback | Planejado (F7) |
| T-12 | Acesso físico à máquina desbloqueada | Pessoa usa app aberto de terceiro | M | M | Bloqueio da app por inatividade (re-auth), step-up para ações críticas, políticas corporativas de lock do SO | Futuro |

---

## 4. Tabela-resumo priorizada — riscos críticos

Ordenação por risco residual esperado no MVP (impacto × probabilidade × maturidade da mitigação).

| Pri | ID | Risco | Impacto | Prob. | Mitigação-chave | Status |
|-----|----|-------|---------|-------|-----------------|--------|
| 1 | T-5/AP-4 | Cross-tenant access via falha de autorização | A | M | RBAC backend + escopo org em middleware + RLS + testes cross-org em CI | MVP |
| 2 | T-10 | Abuso do produto para fraude (risco legal/reputacional existencial) | A | A | Limites por design + allowlist de automação + detecção de abuso + AUP + auditoria | MVP (design) |
| 3 | T-1/DP-1 | Roubo de diretório de perfil local (sequestro de sessões) | A | M | ACL+DPAPI no MVP; cifra de diretório em repouso e rotação de DEK na revogação (futuro) | MVP parcial |
| 4 | AP-1 | Comprometimento de contas (stuffing/tokens roubados) | A | A | Argon2id + MFA + rate limit + rotação de refresh com detecção de reuso | MVP |
| 5 | T-9/CH-1/CH-4 | Chromium adulterado ou vulnerável | A | M | Download verificado (HTTPS+checksum+assinatura) + bloqueio de versões inseguras | MVP |
| 6 | T-6/SY-6 | Lease hijacking / corrupção por concorrência | A | M | Lease atômico + heartbeat + reconciliação + auditoria | F3 |
| 7 | T-7/SY-1 | MITM/rollback no sync | A | M | TLS estrito + envelope cifrado cliente-side + anti-rollback no manifest | F3 |
| 8 | T-4/AD-2 | Abuso de impersonation por staff | A | M | Justificativa+aprovação+TTL+banner+sem segredos+notificação à org | F4 |
| 9 | T-8 | Supply chain (dependência/build comprometido) | A | M | Lockfiles, audit bloqueante, SBOM, assinatura, build só em CI | MVP |
| 10 | T-3 | Insider na org do cliente | A | M | Menor privilégio, sem export aberto, auditoria, offboarding com revogação | F4 |

## 5. Manutenção deste documento

- Revisar a cada fase (F0–F7) e a cada ADR aceito; nova superfície (ex.: SSO, passkeys, E2EE) exige nova passada STRIDE.
- Cada mitigação "Planejado" deve virar item de plano de fase com teste de evidência (spec: "nunca declarar fase concluída sem evidências").
- Os 18 cenários de teste obrigatórios da spec mapeiam para: SY-5 (#2, #6), CH-5 (#3, #4, #5), AP-1 (#7), T-3 (#8), CH-4 (#9), SL-5 (#16), SY-6/T-6 (#1, #17, #18), T-5 (#11), DP-2 (#12), T-11 (#13), DB-4 (#14), §9 SECURITY_MODEL (#15), DP-3 (#1).
