# LGPD — Governança de dados pessoais

> Complementa [SECURITY_MODEL.md](SECURITY_MODEL.md) e [THREAT_MODEL.md](THREAT_MODEL.md). Revisão jurídica humana obrigatória antes do lançamento comercial (F7).

## Papéis (art. 5º)

| Dados | Papel da plataforma | Controlador |
|-------|--------------------|-------------|
| Dados de conta (nome, e-mail, autenticação, billing) | **Controladora** | — |
| Dados dentro dos perfis de navegador dos clientes (cookies, sessões, histórico sincronizado) | **Operadora** — trata por conta e ordem do cliente | Organização cliente |
| Auditoria e telemetria | Controladora | — |

O contrato (DPA anexo aos termos) deve refletir essa divisão e as instruções documentadas do controlador.

## Inventário de dados pessoais

| Categoria | Exemplos | Base legal | Retenção |
|-----------|----------|-----------|----------|
| Cadastro | nome, e-mail, senha (hash Argon2id) | Execução de contrato (art. 7º, V) | Vigência + 5 anos (obrigações legais) |
| Billing | CNPJ/CPF, endereço de faturamento, faturas | Obrigação legal (art. 7º, II) | Prazos fiscais |
| Segurança | IP em eventos de segurança, dispositivo, sessões | Legítimo interesse (art. 7º, IX) — prevenção a fraude/segurança, com teste de balanceamento documentado | Conforme plano (7–365 dias, ver BILLING_RULES) |
| Conteúdo de perfis (operadora) | dados de navegação cifrados | Instrução do controlador | Definida pelo controlador + retenção de snapshots do plano |
| Suporte | tickets, comunicações | Execução de contrato | 2 anos |

**Minimização:** não coletamos conteúdo de navegação para publicidade (compromisso de produto); telemetria de uso é agregada/pseudonimizada; IP registrado apenas em eventos de segurança onde juridicamente adequado.

## Direitos do titular (art. 18) → funcionalidade concreta

| Direito | Implementação |
|---------|---------------|
| Acesso / portabilidade | Exportação de dados da conta e metadados de perfis (JSON) — `GET /users/me/export` (F4) |
| Correção | Edição de cadastro no app |
| Exclusão | Exclusão de conta com confirmação, período de recuperação (soft delete 30 dias) e expurgo definitivo inclusive de backups no ciclo de rotação | 
| Revogação de consentimento | Central de privacidade (telemetria opcional desligável) |
| Informação sobre compartilhamento | Lista pública de suboperadores |
| Canal do titular | e-mail do Encarregado (DPO) publicado na política de privacidade; SLA de resposta 15 dias |

Titulares dos dados **dentro** dos perfis exercem direitos junto ao controlador (cliente); a plataforma fornece ferramentas (exclusão de perfil, snapshots, exportação) para o cliente cumprir.

## Consentimento e termos

- Termos de uso, política de privacidade e AUP **versionados**; aceite registrado (usuário, versão, timestamp, IP) em `consent_records`/auditoria.
- Mudanças materiais → novo aceite obrigatório no login.

## Suboperadores e transferência internacional

- Lista pública mantida: cloud provider, e-mail transacional, gateway de pagamento, observabilidade.
- Se houver transferência internacional: cláusulas contratuais + adequação ao art. 33; preferência por região de dados no Brasil quando disponível.
- Troca de suboperador com aviso prévio aos clientes.

## Incidentes de segurança (art. 48)

1. Detecção → contenção → avaliação de risco a titulares (registro em `security_events`).
2. Comunicação à **ANPD e aos titulares em prazo razoável** (meta interna: avaliação em 24h, notificação em até 3 dias úteis quando houver risco relevante), conforme Resolução CD/ANPD nº 15/2024.
3. Post-mortem documentado; trilha completa preservada (logs imutáveis — SECURITY_MODEL).

## Registro de operações (art. 37) e segurança (art. 46)

- Este documento + DATA_MODEL + SECURITY_MODEL constituem a base do registro de operações de tratamento (ROPA); manter planilha ROPA viva a partir da F4.
- Medidas técnicas: criptografia em trânsito/repouso, envelope encryption, RBAC backend, MFA, auditoria imutável, segregação multi-tenant, minimização de logs (nunca senhas/cookies/tokens).

## Anonimização e telemetria

- Métricas de produto agregadas por org/versão, sem conteúdo de navegação.
- Dados usados em analytics internos são pseudonimizados; nenhum dado pessoal para publicidade de terceiros.
