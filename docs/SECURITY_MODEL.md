# SECURITY_MODEL — Modelo de Segurança (Browser Workspace)

> Derivado de `docs/SPEC_SOURCE.md` (2026-07-01). Documento normativo: os controles descritos aqui são requisitos de implementação, não sugestões.
> Prioridade do projeto: **1. segurança; 2. isolamento; 3. integridade; 4. conformidade** — este documento cobre as quatro.

## 0. Princípios

| # | Princípio | Consequência prática |
|---|-----------|----------------------|
| P1 | Defesa em profundidade | Nenhum controle é único ponto de falha (ex.: RBAC no backend **e** RLS no banco). |
| P2 | Menor privilégio | Papéis granulares; tokens com escopos mínimos; serviços com credenciais próprias por ambiente. |
| P3 | Backend é a fonte de verdade | UI nunca decide autorização; toda checagem é revalidada no servidor. |
| P4 | Segredos nunca em texto plano em repouso | Credential Manager/DPAPI no cliente, KMS/vault no servidor, envelope encryption nos dados. |
| P5 | Auditabilidade | Toda ação sensível gera evento imutável correlacionável. |
| P6 | Falha segura | Em dúvida (lease ambíguo, checksum inválido, token suspeito), negar e registrar. |
| P7 | Transparência ética | Nada de evasão de antifraude, manipulação de fingerprint ou ocultação de automação. Ver §12. |

---

## 1. Autenticação

### 1.1 Armazenamento de senhas — Argon2id

| Parâmetro | Valor recomendado | Observação |
|-----------|-------------------|------------|
| Algoritmo | Argon2id | Nunca bcrypt/scrypt/PBKDF2 para novos hashes. |
| Memória (m) | 64 MiB (65536 KiB) | Mínimo aceitável: 19 MiB (perfil OWASP low-memory) se o hardware do servidor exigir. |
| Iterações (t) | 3 | Ajustar para ~0,5s por hash no hardware de produção. |
| Paralelismo (p) | 4 | |
| Salt | 16 bytes, CSPRNG, único por senha | Gerado pela lib, nunca reutilizado. |
| Tag (hash) | 32 bytes | |
| Pepper (opcional) | HMAC-SHA-256 da senha com chave no KMS antes do Argon2id | Se adotado, documentar rotação. |

- Política de senha: mínimo 12 caracteres, sem regras de composição arbitrárias; checar contra lista de senhas vazadas (k-anonymity, ex.: API Pwned Passwords via prefixo de hash — nunca enviar a senha).
- Re-hash transparente no login quando parâmetros mudarem (armazenar parâmetros no encoded hash PHC string).
- Comparações sempre constant-time (a verificação Argon2id já é).

### 1.2 MFA TOTP

- RFC 6238: SHA-1, 6 dígitos, período 30s, janela de tolerância ±1 step.
- Segredo TOTP: 160 bits gerados por CSPRNG; armazenado **criptografado** (envelope, KEK de plataforma) — nunca em claro no banco.
- Enrolamento: exibir QR + segredo em texto uma única vez; exigir confirmação de um código válido antes de ativar.
- Anti-replay: rejeitar código já usado no mesmo step (guardar último timestep aceito por usuário).
- Rate limit: máx. 5 tentativas de TOTP por sessão de login; depois exigir novo login + backoff.
- Ações críticas (desativar MFA, trocar e-mail, revogar todos dispositivos, excluir org) exigem re-confirmação (step-up): senha + TOTP recentes (< 5 min).

### 1.3 Códigos de recuperação

- 10 códigos de uso único, 10 caracteres alfanuméricos (≥ 50 bits de entropia cada), gerados no enrolamento do MFA.
- Armazenados como hash (Argon2id com parâmetros reduzidos ou SHA-256 — são de alta entropia).
- Exibidos uma única vez; download em arquivo texto permitido; regeneração invalida todos os anteriores e gera evento de auditoria + notificação por e-mail.
- Uso de código de recuperação: notificar por e-mail, marcar sessão como "recuperação" e sugerir re-enrolamento do MFA.

### 1.4 Tokens de sessão — rotação de refresh token com detecção de reuso

- **Access token**: JWT assinado (ES256 ou EdDSA), TTL 10–15 min, claims mínimas: `sub`, `org`, `sid` (session id), `scope`, `exp`, `iat`, `jti`. Nunca colocar permissões completas nem dados pessoais no payload.
- **Refresh token**: opaco (256 bits CSPRNG), armazenado no servidor apenas como hash SHA-256, TTL absoluto 30 dias / sliding 7 dias, vinculado a `session_id` + `device_id`.
- **Rotação**: cada uso de refresh emite novo par e invalida o anterior, mantendo cadeia (`family_id`).
- **Detecção de reuso**: se um refresh token já rotacionado for apresentado → **revogar a família inteira** (todas as sessões derivadas), gerar `security_event`, notificar o usuário por e-mail, exigir novo login com MFA.
- Armazenamento no desktop: refresh token no Windows Credential Manager (DPAPI, escopo usuário); access token só em memória. **Nunca** em arquivo de config, registry em claro, ou dentro do diretório de perfil Chromium.

### 1.5 Sessões revogáveis e dispositivos confiáveis

- Tabela `device_sessions`: usuário, dispositivo, IP de criação, user-agent/versão do app, criada em, última atividade, status.
- Tela "Dispositivos" (desktop) e painel admin: listar e revogar sessões individualmente ou "revogar todas as outras".
- Revogação é efetiva em ≤ TTL do access token (10–15 min) via checagem de `sid` em denylist (Redis) para revogações imediatas de eventos críticos.
- **Dispositivo confiável**: registro do dispositivo (fingerprint próprio do app: hostname + ID de instalação gerado localmente — **não** fingerprint de hardware para rastreio) permite pular MFA por 30 dias, revogável. Novo dispositivo = e-mail de notificação obrigatório com IP, data e ação de "não fui eu" (revoga + força troca de senha).

### 1.6 Rate limiting e anti credential-stuffing

| Superfície | Limite | Ação ao exceder |
|-----------|--------|-----------------|
| Login por conta | 5 falhas / 15 min | Backoff exponencial + CAPTCHA próprio/Turnstile a partir da 3ª; lockout suave (só via e-mail) após 10. |
| Login por IP | 20 falhas / 15 min | Bloqueio temporário de IP (janela deslizante, Redis). |
| TOTP | 5 / sessão de login | Encerrar tentativa de login. |
| Reset de senha | 3 / hora / conta | Silencioso (resposta idêntica para conta inexistente). |
| API pública | Por plano (EntitlementService) + burst por token | 429 com `Retry-After`. |
| Criação de conta | Por IP + verificação de e-mail obrigatória | Fila de revisão anti-abuso. |

- Detecção de credential stuffing: alertar (observabilidade) sobre picos de falhas distribuídas, muitos usuários distintos de mesmo IP/ASN, taxa anormal de senhas corretas + MFA falho.
- Respostas de erro de login **sempre genéricas** ("credenciais inválidas") — sem distinguir e-mail inexistente de senha errada; timing uniforme.

### 1.7 Fluxos auxiliares

- Verificação de e-mail: token de uso único, TTL 24h, hash no banco.
- Reset de senha: token uso único TTL 1h; ao consumir, revogar todas as sessões; notificar.
- Futuro (fora do MVP): passkeys (WebAuthn), SSO OIDC/SAML — arquitetura de auth deve isolar o "método de credencial" para permitir plugá-los sem reescrever sessões.

---

## 2. Autorização (RBAC multi-tenant)

### 2.1 Regras estruturais

1. **Toda** decisão de autorização é tomada no backend, por request, com dados frescos. O desktop/web só esconde botões (UX), nunca autoriza.
2. Todo recurso carrega `organization_id`; toda query filtra por org do contexto autenticado (middleware obrigatório, não opcional por handler).
3. **RLS no PostgreSQL como camada adicional**: políticas por `organization_id` usando `SET LOCAL app.current_org`; a aplicação nunca conecta como superuser/owner das tabelas.
4. Nenhum ID sequencial exposto — UUIDs v4/v7; ausência de recurso e falta de permissão retornam o mesmo `404` para evitar enumeração cross-tenant.
5. Checks de plano/limite centralizados no `EntitlementService` — nunca `if` espalhado.

### 2.2 Papéis (conforme spec)

| Papel | Escopo | Resumo |
|-------|--------|--------|
| Superadmin | Plataforma | Operação da plataforma; sem acesso a segredos de clientes; impersonation controlada (§2.4). |
| Admin da org | Organização | Tudo na org, inclusive membros, cobrança, políticas. |
| Gerente | Workspace/equipe | Gerencia perfis, redes, extensões e operadores do seu escopo. |
| Operador | Perfis atribuídos | Abrir/usar/encerrar perfis; sem gestão de usuários ou segredos. |
| Auditor | Organização (leitura) | Auditoria e relatórios; nenhuma ação mutável. |
| Financeiro | Cobrança | Assinatura, faturas, uso; sem perfis. |
| Suporte | Limitado | Diagnóstico sem segredos e sem conteúdo de perfil. |
| Somente leitura | Escopo atribuído | Visualização. |

### 2.3 Permissões granulares (catálogo mínimo)

`profile.view / profile.create / profile.edit / profile.open / profile.close / profile.delete / profile.restore / profile.share`, `network.manage`, `secret.view`, `extension.manage`, `audit.view`, `member.manage`, `billing.manage`, `api.manage`, `automation.manage`, `session.terminate_others`.

- Papéis = conjuntos de permissões (`roles`, `permissions`, `role_permissions`); permissões avaliadas por recurso + escopo (org → workspace → equipe → perfil).
- `secret.view` (ver senha de proxy, por exemplo) é permissão separada de `network.manage` — gerenciar não implica ler segredo.
- `session.terminate_others` cobre encerramento forçado de perfil aberto por outro operador (quebra de lease por admin) — sempre auditado com justificativa.

### 2.4 Impersonation (painel admin da plataforma)

Controles obrigatórios (spec §Painel web admin):
1. Justificativa textual obrigatória + vínculo a ticket de suporte.
2. Autorização especial (segunda permissão/aprovação, não incluída no papel Superadmin por padrão).
3. Sessão de impersonation com TTL máximo de 1h, banner visível, **sem acesso a segredos** (proxy, DEKs, dados de perfil) — as rotas de segredo rejeitam sessões impersonadas.
4. Auditoria completa (início, cada ação, fim) + **notificação ao admin da org** afetada.

---

## 3. Gestão de segredos

### 3.1 Cliente desktop (Windows — MVP)

| Segredo | Onde | Como |
|---------|------|------|
| Refresh token | Windows Credential Manager | Entrada por app+usuário; DPAPI escopo usuário. |
| DEKs de perfil (cache local) | Credential Manager / arquivo cifrado com DPAPI | Nunca dentro de `/app-data/profiles/{uuid}/`. |
| Credenciais de proxy | Credential Manager | Referenciadas por ID; o config em disco guarda só a referência. |
| Token de IPC efêmero | Memória apenas | Ver §4. |

Regras: segredos **nunca** dentro do diretório de perfil do Chromium (que é sincronizado/copiável); nunca em logs locais; nunca em variáveis de ambiente persistidas; futuro macOS/Linux: Keychain / Secret Service (a abstração `packages/crypto` deve prever isso).

### 3.2 Servidor

- Segredos operacionais (DB, Redis, S3, chaves de assinatura JWT, chaves de webhook) em vault/secret manager (ex.: AWS Secrets Manager, HashiCorp Vault, SOPS+KMS para IaC) — nunca em repositório, nunca compartilhados entre dev/staging/prod.
- **KMS** para chaves raiz: KEKs nunca saem do KMS em claro (operações via API de encrypt/decrypt/wrap).

### 3.3 Envelope encryption dos dados de perfil

```
KMS root key (plataforma, HSM/KMS)
  └── KEK por organização (wrapped pela root; cache curto no serviço)
        └── DEK por perfil (AES-256-GCM; wrapped pela KEK; armazenada wrapped junto ao manifest)
              └── blobs do perfil (cookies, storage, etc.) cifrados com a DEK
```

- Algoritmo: AES-256-GCM, nonce de 96 bits único por objeto (contador + random, nunca reutilizado com a mesma chave), AAD = `org_id || profile_id || object_id || versão` (impede transplante de blob entre perfis/orgs).
- **Rotação de KEK**: gerar nova KEK, re-wrap de todas as DEKs da org (operação de metadados, não re-cifra blobs), manter `key_version`; agendada (anual) e sob demanda (incidente).
- **Rotação de DEK**: na próxima gravação completa/snapshot do perfil; forçável por admin.
- **Revogação de dispositivo**: dispositivo revogado perde acesso porque (a) sessão revogada bloqueia API de unwrap, e (b) na revogação por comprometimento, rotacionar DEKs dos perfis que o dispositivo acessou (lista via auditoria). Cache local de DEK no dispositivo tem TTL curto e é apagado no logout/revogação (best-effort — assumir no threat model que dispositivo comprometido pode reter o que já viu).
- **Recuperação administrativa**: possível porque as KEKs são da plataforma (não E2EE). **Documentar honestamente**: no MVP **não há E2EE** — a plataforma tem capacidade técnica de decifrar dados de perfil; o controle é organizacional (KMS com políticas, auditoria de unwrap, dupla aprovação para acesso administrativo a dados). E2EE opcional é item futuro explícito da spec (fora do MVP).

---

## 4. IPC local (desktop UI ↔ serviço local)

- Transporte: **loopback apenas** (`127.0.0.1`, e/ou named pipes do Windows — preferir named pipe com SDDL restringindo ao SID do usuário logado). Nenhuma porta em `0.0.0.0`. Firewall/binding validados no startup.
- **Token efêmero**: gerado por boot do serviço (256 bits CSPRNG), entregue à UI por canal fora de banda (handshake no named pipe ou arquivo com ACL restrita ao usuário, apagado após leitura). Toda mensagem IPC carrega o token; rotação a cada sessão do app.
- **Validação de origem**: para o processo conectado ao pipe/socket, verificar PID → caminho do executável → assinatura Authenticode do binário esperado. Rejeitar e auditar mismatch.
- Se houver superfície HTTP local (ex.: para OAuth redirect), validar header `Origin`/`Host`, responder apenas a origens esperadas, CSRF token, e nunca expor operações de segredo por essa superfície.
- Mensagens IPC validadas por schema (`packages/validation`); serviço local aplica as mesmas checagens de permissão em cache (com revalidação no backend para ações sensíveis).

---

## 5. Criptografia em trânsito e em repouso

### 5.1 Trânsito

- **TLS 1.2 mínimo, TLS 1.3 preferido** em toda comunicação app↔API, sync↔storage, worker↔serviços. Cipher suites modernas (AEAD apenas); sem renegociação insegura; HSTS no painel web (`max-age` ≥ 1 ano, `includeSubDomains`).
- Certificados: CA pública com rotação automatizada; **certificate pinning não** no MVP (complica rotação); em vez disso, validar cadeia + hostname estritamente e rejeitar proxies MITM corporativos silenciosamente apenas com aviso claro ao usuário (transparência).
- Conexões internas (API↔Postgres/Redis/MinIO) com TLS quando cruzarem host/rede; em Docker Compose local dev pode ser plaintext, **nunca** em staging/prod.
- WebSockets sob WSS com mesma política.

### 5.2 Repouso

| Dado | Mecanismo |
|------|-----------|
| Blobs de perfil (object storage) | Envelope AES-256-GCM (§3.3) — cifrado **antes** do upload, pelo cliente de sync. Server-side encryption do bucket como camada extra. |
| Banco (PostgreSQL) | Encryption at rest do volume/serviço gerenciado + colunas sensíveis (segredo TOTP, credenciais de proxy) cifradas em nível de aplicação com KEK de plataforma. |
| Backups | Cifrados com chave própria, testados (restore drill), retenção definida na LGPD.md. |
| Diretório local de perfil | Dados do Chromium ficam como o Chromium os grava (DPAPI para cookies no Windows); complementos: ACL NTFS restrita ao usuário, opção futura de cifrar o diretório em repouso quando perfil fechado. Segredos da plataforma nunca lá (§3.1). |

---

## 6. Política de logs

### 6.1 Lista NUNCA logar (proibição absoluta, testada em CI)

- Senhas (qualquer campo `password*`, `secret*`, `token*`).
- Cookies e qualquer conteúdo de sessão de navegador.
- Access/refresh tokens, tokens de API, tokens de IPC, códigos TOTP/recuperação.
- Segredos e senhas de proxy.
- Headers de autenticação (`Authorization`, `Cookie`, `Set-Cookie`, `Proxy-Authorization`).
- Conteúdo de formulários, dados bancários/cartão, corpo de páginas navegadas.
- DEKs/KEKs ou qualquer material de chave.

Implementação: redaction centralizada em `packages/logging` (denylist de chaves + padrões: JWT `eyJ...`, PANs por Luhn, strings tipo `Bearer `); serializadores seguros por default (opt-in de campos, não opt-out); teste automatizado que injeta segredos sintéticos nos fluxos e falha o build se aparecerem em qualquer sink de log. Critério de aceitação MVP #6 depende disso.

### 6.2 O que logar

- Logs estruturados (JSON), `correlation_id` propagado (OpenTelemetry), `org_id`, `actor_id`, `device_id`, ação, resultado, latência.
- IP apenas quando adequado (eventos de segurança/autenticação) — ver LGPD.md para retenção.
- Níveis: erro/aviso/info; debug desabilitado em produção por padrão e nunca com payloads completos.

---

## 7. Supply chain

| Controle | Implementação |
|----------|---------------|
| Lockfiles | `pnpm-lock.yaml`/`Cargo.lock`/`go.sum` commitados; CI falha se lockfile desatualizado; instalação com `--frozen-lockfile`. |
| Audit de dependências | `pnpm audit`/`cargo audit`/`govulncheck` no CI (bloqueante para severidade alta/crítica) + Dependabot/Renovate com revisão humana. |
| Pinning | Versões exatas para deps críticas (crypto, auth); proibir instalação de pacotes com scripts de pós-instalação sem revisão (`--ignore-scripts` + allowlist). |
| SBOM | CycloneDX gerado por build e publicado com cada release. |
| Secret scanning | gitleaks/trufflehog no CI + pre-commit. |
| Assinatura de binários | Authenticode (certificado EV recomendado) para instalador, executável e serviço local; auto-update **só aplica** pacote com assinatura válida + versão maior (anti-downgrade). |
| Chromium | Download exclusivamente por HTTPS de origem fixa verificada; validar **checksum (SHA-256) publicado por canal confiável + assinatura do binário**; recusar e alertar em mismatch; registrar versão+hash instalado por máquina (auditoria); bloqueio de versões inseguras via lista servida pela API (assinada). |
| Build | Pipelines com builds reproduzíveis quando viável; artefatos de release gerados apenas por CI (nunca de máquina de dev); credenciais de assinatura em HSM/KMS do CI. |
| Ambientes | dev/staging/prod com credenciais totalmente separadas (spec §DevOps). |

---

## 8. Auditoria imutável

- Tabela `audit_events` **append-only**: sem UPDATE/DELETE (revogar privilégios do papel da aplicação; trigger que rejeita).
- **Hash chain** para eventos críticos: cada evento guarda `prev_hash` e `hash = SHA-256(prev_hash || payload_canônico)`; âncora periódica (a cada N eventos/hora) gravada em meio WORM (ex.: bucket S3 com Object Lock em modo compliance). Verificador de cadeia roda como job e alerta em quebra.
- Eventos (spec §Auditoria): login/logout/falha, CRUD de perfil, abertura/encerramento, mudanças de rede, extensão, exportação, permissão, convite/remoção, acesso admin/impersonation, assinatura, uso de API, automação, revogações.
- Campos por evento: `org_id`, ator (usuário/API key/sistema), ação, recurso (tipo+id), timestamp, dispositivo, resultado, IP quando adequado, `correlation_id`, metadados **seguros** (passar pela redaction §6).
- Retenção conforme plano (EntitlementService) com piso legal definido em LGPD.md; exportação para o cliente (Auditor role) em formato estruturado.

---

## 9. Rede e proxies (segurança específica)

- Credenciais de proxy cifradas em nível de aplicação (§3.3/§5.2); mascaradas na UI (`••••`, revelar exige `secret.view` + auditoria).
- Teste de conexão/latência executado pelo serviço local sem logar senha (URL de proxy nunca serializada com credencial embutida em logs/erros).
- Mudança de config de rede de um perfil = evento de auditoria + invalidação do cache no cliente.
- Sem promessas de anonimato; país declarado é informativo, nunca verificado/forjado.

---

## 10. Lease de perfil (integridade de concorrência)

- Aquisição atômica no backend (constraint única em `profile_leases` ativo por perfil + comparação de versão otimista); TTL curto (ex.: 90s) renovado por heartbeat (ex.: 30s).
- Token de lease vinculado a `device_id` + `session_id`; operações de sync/encerramento exigem lease válido.
- Expiração sem heartbeat → perfil marcado "recuperação pendente": exigir reconciliação de estado antes de novo lease (evitar perda de dados — cenário de teste #17).
- Encerramento forçado por admin: permissão `session.terminate_others`, justificativa, auditoria, notificação ao operador.
- Todas as transições de lease auditadas; proteção contra race testada (cenário #1 e #18).

---

## 11. Hardening adicional do serviço/API

- Headers: `Content-Security-Policy` estrita no web-admin, `X-Content-Type-Options`, `Referrer-Policy`, cookies `HttpOnly; Secure; SameSite=Lax` (sessão web).
- Validação de entrada por schema em toda borda (`packages/validation`); parametrização total de SQL (sem concatenação); uploads com limite de tamanho, tipo verificado por conteúdo e armazenados fora do docroot.
- Idempotency keys em operações mutáveis da API pública (spec §API).
- Webhooks **assinados** (HMAC-SHA-256 com segredo por endpoint, timestamp anti-replay ±5 min); retries com backoff; URL de destino validada contra SSRF (bloquear IPs privados/metadata endpoints).
- SSRF geral: qualquer fetch server-side de URL fornecida por usuário passa por allowlist/deny de redes internas.

---

## 12. Política de uso aceitável (AUP) e anti-abuso

### 12.1 AUP resumida (a versão jurídica completa deriva daqui)

**Permitido**: gestão de perfis isolados para operação corporativa legítima — agências com autorização dos clientes, suporte, e-commerce multi-loja próprio, QA/dev, labs de segurança autorizados, separação de contextos profissionais.

**Proibido (causa suspensão)**: evasão de antifraude/bloqueios/banimentos; fazendas de contas e criação massiva; bypass de CAPTCHA/KYC; falsificação de identidade; uso de sessões/credenciais de terceiros sem autorização; fraude de qualquer natureza; automação fora da allowlist ou sem identificação.

**O produto, por design, NÃO**: manipula fingerprints (Canvas/WebGL/AudioContext, fontes, `hardwareConcurrency`, `deviceMemory`, Client Hints, identificadores de hardware), não oculta automação, não exporta cookies em formato aberto, não "aquece contas", não resolve CAPTCHA, não promete anonimato. Configurações de privacidade legítimas (bloqueio de rastreadores, controle de cookies, bloqueio de WebRTC, permissões por domínio) são oferecidas de forma transparente e nunca vendidas como anti-antifraude.

### 12.2 Mecanismos anti-abuso da plataforma

| Mecanismo | Descrição |
|-----------|-----------|
| Allowlist de automação | Automação (F5, Playwright) só executa contra domínios cadastrados e justificados pela própria org; mudanças auditadas; aprovação por Admin da org. |
| Identificação de automação | Sessões automatizadas se identificam (header/UA marker documentado) e respeitam limites de taxa. Botão de emergência (kill switch) por org e por plataforma. |
| Sem export aberto de cookies | Nenhuma API/tela exporta cookies/sessões em claro; exportação de perfil = blob cifrado restaurável apenas pela mesma org na plataforma. |
| Detecção de padrões de abuso | Sinais monitorados: volume anômalo de criação de perfis, muitos perfis + muitos proxies + automação intensa, padrões de domínio típicos de fraude, várias orgs do mesmo pagador criadas em rajada. Gera `security_events` → fila de revisão humana no painel admin (spec: seção "abuso"). |
| Limites por plano | EntitlementService impõe tetos (perfis, automações, minutos, API calls) que encarecem abuso em escala. |
| KYC de cobrança | Dados de pagamento verificados pelo gateway; orgs com chargeback/denúncia entram em revisão. |
| Canal de denúncia | E-mail/form público para relatos de abuso; SLA de triagem definido; cooperação com autoridades conforme LGPD.md/termos. |
| Enforcement gradativo | Aviso → suspensão de automação → suspensão da org → banimento; sempre auditado. |

---

## 13. Rastreabilidade para testes

Controles deste documento cobrem diretamente os critérios de aceitação do MVP: #3 (lease, §10), #5 (recuperação, §10), #6 (segredos fora de logs, §6), #7 (RBAC backend, §2), #8 (validação de rede, §9/§11), #9 (auditoria, §8), #13 (atualização assinada, §7), #14 (sync não destrói perfil, §10 + THREAT_MODEL), #15 (exclusão com recuperação, LGPD.md §retenção). Nenhuma fase se declara concluída sem evidência de teste (spec §Testes).
