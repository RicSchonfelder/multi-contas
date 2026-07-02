# PRODUCT_VISION — Browser Workspace

> Derivado de `docs/SPEC_SOURCE.md` (fonte de verdade). Nome do produto é PROVISÓRIO.
> Última atualização: 2026-07-01. Autor: Agente Produto.

## 1. Visão

Ser a plataforma corporativa de referência para **gerenciamento de ambientes de navegador isolados, auditáveis e colaborativos**, permitindo que empresas operem múltiplos contextos web (contas de clientes, ambientes de teste, atendimentos multi-empresa) com segurança, rastreabilidade e conformidade — sem improvisos como "vários Chromes abertos", planilhas de senhas ou perfis pessoais misturados a dados corporativos.

**Frase-síntese:** *"Cada contexto de trabalho no navegador, isolado, governado e auditável — para equipes, não para burlar sistemas."*

## 2. Problema

Empresas que operam contas web de terceiros (com autorização) ou múltiplos ambientes enfrentam:

| Problema | Consequência |
|---|---|
| Sessões misturadas entre contas/clientes no mesmo navegador | Vazamento de contexto, ações na conta errada, incidentes reputacionais |
| Compartilhamento informal de credenciais (planilha, chat) | Risco de segurança, impossibilidade de revogar acesso individual |
| Sem trilha de auditoria de "quem acessou o quê, quando" | Não conformidade (LGPD, contratos), impossibilidade de investigar incidentes |
| Perfis locais presos a uma máquina | Sem continuidade em troca de dispositivo, sem trabalho em equipe |
| Dois operadores abrindo a mesma sessão simultaneamente | Corrupção de dados de perfil, logout mútuo, comportamento imprevisível |
| Ferramentas existentes no mercado posicionadas como "anti-detecção" | Risco jurídico e reputacional para empresas legítimas que só precisam de isolamento |

## 3. Público-alvo

Empresas e equipes com uso **autorizado e legítimo** de múltiplos contextos de navegador (conforme casos de uso da spec):

- Agências de marketing/social media com autorização formal dos clientes
- Times de suporte, comercial e atendimento multi-empresa
- E-commerce e franquias (múltiplas lojas/painéis)
- QA, testes e desenvolvimento web
- Labs de segurança autorizados
- Trabalho remoto em equipe com navegação corporativa controlada

**Não é o público:** indivíduos buscando anonimato, evasão de bloqueios/banimentos, fazendas de contas, growth hacking em violação de ToS de plataformas.

## 4. Personas

### P1 — Renata, Admin de Agência (decisora + administradora)
- **Contexto:** sócia-operacional de agência com 12 pessoas e ~40 clientes; cada cliente autorizou formalmente o acesso às suas contas.
- **Objetivos:** dar/retirar acesso por cliente em segundos; garantir que ex-funcionário não leve acessos; provar aos clientes que o acesso é controlado.
- **Dores:** senhas em planilha; não sabe quem acessou o quê; onboarding/offboarding manual e arriscado.
- **O que precisa do produto:** organização multi-tenant, papéis (Admin/Gerente/Operador), convites, compartilhamento de perfis por equipe, revogação imediata, painel de plano/uso.
- **Métrica de sucesso:** offboarding completo de um funcionário em < 5 minutos, com evidência.

### P2 — Diego, Operador (usuário diário)
- **Contexto:** analista que atende 8 clientes por dia; alterna entre painéis, redes sociais autorizadas e ferramentas dos clientes.
- **Objetivos:** abrir o perfil certo rápido, com favoritos e extensões corretas já configurados; nunca agir na conta errada.
- **Dores:** confusão de abas/janelas; perder sessão ao trocar de máquina; conflito quando um colega abre o mesmo perfil.
- **O que precisa do produto:** lista de perfis com busca/filtros/etiquetas/cores, abertura em 1 clique, lease que impede abertura dupla (com mensagem clara de quem está usando), sync de sessão entre dispositivos, favoritos por perfil.
- **Métrica de sucesso:** encontrar e abrir o perfil correto em < 10 segundos; zero incidentes de "conta errada".

### P3 — Marina, Gerente de QA (usuária técnica)
- **Contexto:** lidera time de QA que testa aplicações web em múltiplos estados de sessão, contas de teste e configurações de rede/proxy corporativo.
- **Objetivos:** ambientes reproduzíveis e descartáveis; duplicar configuração de perfil sem sessões; automação **identificada** apenas nos domínios da própria empresa (allowlist).
- **Dores:** contaminação de estado entre testes; setup manual repetitivo; automação frágil e sem governança.
- **O que precisa do produto:** duplicação de perfis (config sem sessão), limpeza de cache/cookies, config de proxy por perfil com teste de conexão, automação com allowlist + auditoria + botão de emergência, API pública.
- **Métrica de sucesso:** provisionar 10 ambientes de teste idênticos em minutos, com estado garantidamente isolado.

### P4 — Fábio, Auditor / Compliance (fiscalizador)
- **Contexto:** responsável por conformidade (LGPD, contratos com clientes, políticas internas) em empresa de médio porte que usa a plataforma.
- **Objetivos:** responder "quem acessou o perfil X entre as datas Y e Z, de qual dispositivo, com que resultado"; evidenciar controles em auditorias externas.
- **Dores:** ferramentas sem trilha de auditoria; logs adulteráveis; segredos vazando em logs; ausência de papel somente-leitura.
- **O que precisa do produto:** papel Auditor (somente leitura), eventos de auditoria completos (ator, ação, recurso, dispositivo, resultado, correlação), logs imutáveis para eventos críticos, garantia de que senhas/cookies/tokens nunca aparecem em logs, exportação de trilhas, retenção configurável.
- **Métrica de sucesso:** responder a uma auditoria externa apenas com relatórios da plataforma, sem coleta manual.

## 5. Proposta de valor

Para **equipes com uso autorizado de múltiplos contextos web**, o Browser Workspace oferece **perfis Chromium totalmente isolados (cookies, cache, storage, extensões, favoritos, rede), governados por organização com papéis e permissões, sincronizados com criptografia e auditados de ponta a ponta** — diferente de soluções improvisadas (perfis nativos do Chrome, VMs) e de ferramentas "anti-detect", porque prioriza **segurança, integridade e conformidade**, não ocultação.

## 6. Diferenciais

Ordem alinhada à prioridade da spec (segurança > isolamento > integridade > conformidade > recuperação > UX > performance > features):

1. **Segurança como fundamento:** segredos em cofres do SO (Credential Manager/Keychain/Secret Service), nunca no diretório do Chromium nem em logs; Argon2id, MFA, rotação de refresh token, detecção de reuso.
2. **Isolamento real e verificável:** um diretório exclusivo por perfil; critério objetivo (F1): dois perfis jamais compartilham cookies/cache/storage.
3. **Integridade e recuperação:** lease distribuído (nunca 2 máquinas no mesmo perfil), detecção de crash, checksums, snapshots com rollback, falha de sync nunca destrói dados locais.
4. **Auditoria de primeira classe:** todo evento relevante registrado com ator/recurso/dispositivo/resultado; logs imutáveis para eventos críticos; papel Auditor dedicado.
5. **Conformidade nativa (LGPD):** minimização, retenção, exclusão, exportação, portabilidade, registro de consentimento — desenhados desde o início, não retrofit.
6. **Multi-tenancy corporativo real:** organizações, workspaces, equipes, 8 papéis, permissões granulares sempre validadas no backend.
7. **Automação governada:** somente em domínios allowlistados do próprio cliente, com identificação explícita de automação, limites, auditoria e botão de emergência.
8. **Transparência técnica:** documentação honesta (inclusive sobre presença/ausência de E2EE), binários assinados, atualizações verificadas.

**Diferencial que NÃO temos e não teremos:** qualquer capacidade de evasão de antifraude, fingerprint spoofing ou ocultação de automação. Isso é um diferencial *comercial positivo* para empresas com área jurídica/compliance: adotar a plataforma não cria passivo.

## 7. O que o produto explicitamente NÃO é

Conforme limites obrigatórios da spec (seção "Limites obrigatórios"):

- **Não é ferramenta anti-detecção/anti-fingerprint.** Não altera Canvas/WebGL/AudioContext, fontes, hardwareConcurrency, deviceMemory, Client Hints, WebRTC para mascarar origem, nem qualquer identificador de dispositivo.
- **Não é ferramenta de evasão** de antifraude, bloqueios ou banimentos.
- **Não é plataforma de fazendas de contas** nem criação massiva de contas.
- **Não faz bypass de CAPTCHA/KYC** nem falsificação de identidade.
- **Não rouba/reusa sessões de terceiros**, não extrai credenciais, não exporta cookies em formato aberto inseguro.
- **Não oculta automação** — automação é sempre identificada, limitada a allowlist e auditada. Sem "aquecimento de conta".
- **Não vende anonimato.** Suporte a proxies é para roteamento corporativo/QA declarado, sem promessa de ocultar identidade.
- **Não é clone** de nenhum concorrente — implementação, marca, layout e textos 100% próprios.

Privacidade legítima e transparente é permitida (bloqueio de rastreadores, controle de cookies, limpeza de dados, bloqueio de WebRTC como política de privacidade, permissões por domínio) — mas nunca comunicada como mecanismo anti-antifraude.

## 8. Posicionamento

**Categoria:** gestão corporativa de ambientes de navegador (browser environment management / enterprise browser workspace).

**Declaração de posicionamento:**
> Para empresas e equipes que operam múltiplos contextos web com autorização, o Browser Workspace é a plataforma de perfis de navegador isolados que oferece governança, auditoria e conformidade de nível corporativo. Diferente de ferramentas "anti-detect" (que criam risco jurídico) e de soluções improvisadas (que não escalam nem auditam), nós tratamos isolamento de contexto como um problema de **segurança e governança**, não de ocultação.

**Pilares de comunicação (nesta ordem):** 1) Segurança; 2) Isolamento; 3) Auditoria/Conformidade; 4) Colaboração em equipe. Nunca usar em marketing: "indetectável", "anti-detect", "múltiplas identidades", "burlar", "farm", "stealth".

**Âncoras de mercado:** mais próximo de um "enterprise browser" leve + gestão de acessos do que de ferramentas multi-conta de growth. Concorrência indireta: perfis nativos de navegador, VMs/VDI, gerenciadores de senha com compartilhamento. Não competimos com, nem nos comparamos a, ferramentas anti-detect.

**Modelo de entrega:** app desktop (MVP: Windows) + backend cloud + painel web admin; monetização por planos/assinatura (Fase 6; ver `BILLING_RULES.md`).

## 9. Critérios de sucesso do produto (indicadores iniciais)

| Indicador | Alvo inicial |
|---|---|
| Incidentes de vazamento entre perfis (cross-profile) | 0 (critério absoluto) |
| Tempo de offboarding de operador (revogação total) | < 5 min |
| Tempo para localizar e abrir perfil correto | < 10 s |
| Cobertura de auditoria dos eventos da spec | 100% dos eventos listados |
| Recuperação pós-crash sem perda de perfil | 100% dos 18 cenários de teste |
| Adoção: organizações ativas com ≥ 2 operadores | crescimento mês a mês (baseline pós-MVP) |
