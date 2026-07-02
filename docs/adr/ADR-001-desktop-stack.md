# ADR-001 — Stack do Desktop Client

## Status

**Aceito** — 2026-07-01

## Contexto

O Browser Workspace precisa de um cliente desktop (MVP: **Windows-only**) que:

1. Ofereça UI rica (21 telas, tabela de perfis com filtros/lote/views salvas) em React + TypeScript.
2. **Gerencie processos Chromium EXTERNOS** — o produto NÃO embute o navegador na UI. Cada perfil roda como `chrome.exe --user-data-dir=<diretório isolado>`, processo filho monitorado.
3. Mantenha um **serviço local de perfis** de longa duração: leases, heartbeat, detecção de crash, kill de órfãos, sync em segundo plano — funções que devem sobreviver/independer do ciclo de vida da janela da UI.
4. Proteja segredos via Windows Credential Manager (DPAPI), sem expô-los ao contexto web da UI.
5. Seja distribuído com instalador assinado (Authenticode) e auto-update assinado (critérios 12 e 13 do MVP).
6. Respeite a prioridade da spec: segurança > isolamento > integridade > conformidade > recuperação > UX > performance > features.

**Consideração-chave**: como os perfis usam Chromium externo, a principal "vantagem" do Electron (sessions/partitions embutidas) é **irrelevante** para o core do produto. Electron seria apenas um shell de UI carregando um Chromium inteiro só para renderizar telas de gestão. A comparação real é: qual runtime hospeda melhor (a) a UI React e (b) o serviço nativo que controla processos, arquivos e criptografia.

## Opções

- **(A) Tauri (Rust) + React/TS + Chromium externo + serviço local em Rust**
  UI renderizada pelo WebView2 do sistema (já presente no Windows 10/11). Backend Tauri em Rust no mesmo binário (comandos IPC nativos) + serviço local Rust (mesmo processo no MVP; extraível para Windows Service depois).
- **(B) Electron + React/TS**
  Shell Chromium embutido para a UI; lógica de processos em Node.js (main process). As sessions do Electron NÃO seriam usadas para perfis — perfis continuam em Chromium externo.
- **(C) React UI (shell leve) + daemon local em Go + Chromium externo + IPC autenticado**
  Daemon Go como Windows Service; UI React servida em WebView2 ou empacotada à parte; comunicação via loopback HTTP/gRPC com token efêmero.

## Tabela comparativa

| Critério | (A) Tauri + Rust | (B) Electron | (C) React UI + daemon Go |
|---|---|---|---|
| **Consumo de memória (shell UI)** | ~60–150 MB (WebView2 compartilhado com o SO) | ~250–500 MB (Chromium completo dedicado à UI, além dos Chromium dos perfis) | ~80–180 MB (WebView2 + daemon Go ~20–40 MB) |
| **Tamanho do instalador** | ~10–25 MB | ~120–200 MB | ~30–60 MB (2 artefatos) |
| **Isolamento de segredos da UI** | Forte: segredos ficam no core Rust; UI só recebe handles/resultados via comandos com allowlist | Fraco por padrão: `contextBridge` bem configurado mitiga, mas main process em JS manipula segredos em heap GC, histórico de CVEs do próprio Electron | Forte: segredos só no daemon; UI nem roda no mesmo processo |
| **Atualização do app** | `tauri-updater` com assinatura embutida (minisign) + validação obrigatória; artefatos pequenos = updates rápidos | `electron-updater` maduro (Squirrel/NSIS); artefatos grandes; precisa acompanhar releases do Electron (ritmo mensal de CVEs do Chromium embutido) | Dois componentes para atualizar (UI + daemon) com matriz de compatibilidade entre versões — mais complexo |
| **Assinatura de binários (Authenticode)** | 1 executável + instalador MSI/NSIS; superfície mínima | Muitos binários/DLLs no pacote (chrome_*.dll etc.); tudo precisa ser assinado; superfície maior | 2 executáveis + instalador; simples, mas o daemon como serviço exige assinatura e instalação privilegiada |
| **Compatibilidade com extensões Chrome (dos PERFIS)** | Total — perfis rodam Chrome/Chromium real externo | Total — idem (extensões nos perfis não dependem do Electron) | Total — idem |
| **Multiplataforma futura** | Boa (WebView2/WKWebView/WebKitGTK); variações de renderização entre WebViews exigem QA por SO | Excelente — renderização idêntica em todos os SOs | Boa; daemon Go compila trivialmente para mac/Linux; UI precisa de host por SO |
| **Controle de processos filhos (spawn, kill tree, órfãos)** | Excelente: Rust com acesso direto a Win32 (Job Objects para kill de árvore, handles, WMI) | Adequado: `child_process` + módulos nativos (node-ffi/koffi) para Job Objects; mais indireção e risco de bloqueio do event loop | Excelente: Go com `x/sys/windows`, goroutines ideais para monitorar N processos + heartbeats |
| **Debugging / DX** | Rust tem curva de aprendizado; devtools do WebView2 para a UI; boa história de logs/tracing (`tracing` crate) | Melhor DX do mercado: tudo TS/JS, Chrome DevTools em tudo, hot reload | Duas linguagens (Go + TS) e dois processos = debugging distribuído local; bom tooling (Delve), porém mais fricção |
| **Segurança (superfície de ataque)** | Pequena: sem Node no renderer, IPC com allowlist de comandos, CSP estrita; core em linguagem memory-safe | Maior: runtime Node + Chromium embutido versionado por nós; histórico de RCEs por misconfiguração; exige disciplina (sandbox, contextIsolation, no nodeIntegration) | Pequena na UI; daemon expõe porta loopback — exige autenticação de origem/token efêmero corretos (a spec já exige isso, mas é código nosso a errar) |
| **Distribuição** | MSI/NSIS pequeno; dependência do WebView2 Runtime (embutível no instalador; presente por padrão no Win 11) | Autossuficiente, funciona em qualquer Windows sem dependências | Instalador precisa registrar serviço Windows (privilégio admin) — atrito em ambientes corporativos travados, embora comum |
| **Complexidade operacional** | Média: 1 app, 2 linguagens (Rust + TS), 1 pipeline de release | Baixa: 1 linguagem, 1 app, ecossistema maduro | Alta: 2 artefatos, 2 linguagens, versionamento de protocolo IPC, ciclo de vida de serviço |
| **Velocidade de entrega do MVP** | Média (Rust é mais lento de escrever; ecossistema Tauri v2 maduro o suficiente) | Alta | Baixa-média |

## Decisão

**Opção A — Tauri 2.x + Rust + React + TypeScript, com serviço local de perfis em Rust e Chromium externo gerenciado.**

Arquitetura resultante:

- **Processo único no MVP**: app Tauri contém a UI (WebView2) e o *Profile Service* (crate Rust interna, futura extração para Windows Service em F7 se necessário — a fronteira já nasce como módulo com API própria).
- Perfis = processos `chrome.exe`/`chromium.exe` externos lançados com `--user-data-dir` por diretório isolado, agrupados em **Windows Job Objects** (kill de árvore garantido, detecção de término, prevenção de órfãos).
- IPC UI↔core via comandos Tauri (sem porta de rede local no MVP); quando o serviço for extraído, aplica-se o modelo da spec (loopback restrito + token efêmero) — ver `DESKTOP_ARCHITECTURE.md`.
- Segredos: crate `windows` → DPAPI/Credential Manager, nunca tocando o contexto web.

### Justificativa

1. **Prioridade nº 1 é segurança**: core em Rust (memory-safe), sem Node no renderer, IPC por allowlist, superfície de assinatura mínima. Electron exigiria disciplina contínua para não regredir; Tauri é seguro por construção.
2. **Footprint**: o produto já vai rodar N processos Chromium (os perfis). Gastar 300+ MB extras num Chromium só para a UI (Electron) contradiz a proposta de valor para quem opera dezenas de perfis.
3. **Controle de processos é o coração do produto**: Job Objects, handles, WMI, watchdogs — território onde Rust/Win32 é direto e Node é improvisado.
4. **Assinatura e distribuição**: binário único pequeno simplifica Authenticode, SBOM, checksum e auto-update assinado (critério 13 do MVP).
5. **Sobre C (Go daemon)**: tecnicamente excelente para o daemon, mas paga o custo de *dois* artefatos, protocolo IPC versionado e instalação de serviço já no MVP — complexidade que a Opção A adia sem fechar portas (a extração do serviço Rust é o mesmo movimento, feito quando houver necessidade real: perfis persistindo com UI fechada, multiusuário na mesma máquina).

### Trade-off assumido (registrado)

- **Velocidade de entrega**: Electron entregaria o MVP mais rápido (uma linguagem, DX superior, contratação fácil). Aceitamos entrega ~20–30% mais lenta em troca de segurança estrutural, footprint e simplicidade de assinatura — coerente com a ordem de prioridades da spec (UX/velocidade vêm depois de segurança/isolamento/integridade).
- **Duas linguagens** (Rust + TS) no time desktop.
- **WebView2 como dependência** — mitigado: presente por padrão no Windows 11 e embutível no instalador (bootstrapper) para Windows 10.

## Consequências

**Positivas**
- Shell de UI leve; memória sobra para os Chromium dos perfis.
- Segredos nunca atravessam contexto JavaScript.
- Pipeline de release simples: 1 exe + instalador MSI/NSIS assinados + update assinado.
- Fronteira UI/serviço já modular → extração futura para Windows Service sem redesign.

**Negativas / custos**
- Curva de aprendizado Rust; menos exemplos prontos que Electron.
- QA de renderização por versão do WebView2 (Evergreen — atualiza sozinho, raramente quebra, mas fora do nosso controle de versão).
- Módulos nativos do ecossistema Node (se algum fosse desejado) não se aplicam; tudo nativo é Rust.

## Alternativas rejeitadas

- **(B) Electron**: rejeitada. Sua única vantagem estrutural (Chromium embutido com sessions) não é usada — os perfis são Chromium externo por definição do produto. Restaria pagar 150–200 MB de instalador, memória dobrada na UI, superfície de assinatura grande e o passivo de segurança do runtime Node exposto — contra as prioridades 1, 2 e 7 da spec. Vantagem real (velocidade de entrega/DX) é insuficiente para inverter a decisão.
- **(C) React UI + daemon Go**: rejeitada *para o MVP*. Arquiteturalmente sólida e a melhor opção se o requisito "perfis rodam com UI fechada / máquina multiusuário / gestão via CLI" se tornar central. Custo imediato (dois artefatos, IPC autenticado versionado, serviço Windows com instalação privilegiada, duas linguagens e debugging distribuído) não se justifica no MVP Windows-only single-user. A Opção A preserva o caminho de migração: extrair o Profile Service Rust para serviço autônomo replica C com uma linguagem a menos.
