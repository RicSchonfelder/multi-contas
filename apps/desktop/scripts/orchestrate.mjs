#!/usr/bin/env node
/**
 * Orquestração Multi Contas via Hermes (servidor local tiny_http).
 *
 * Fluxo para "abrir a mesma live em todas as contas e comentar de cada uma":
 *
 *   1. Abre TODAS as contas (perfis disponíveis) — cada uma numa porta CDP única.
 *   2. Manda cada conta navegar para a URL (ex.: a live do YouTube).
 *   3. Você manda um comentário e ele vai PARA CADA CONTA (ou para uma específica).
 *
 * Requer: o app Multi Contas rodando (janela aberta) — ele sobe o servidor
 * de controle em 127.0.0.1:29222 e gera o token em %LOCALAPPDATA%/MultiContas/control_token.
 *
 * Uso:
 *   node orchestrate.mjs                       # abre tudo + navega (URL abaixo)
 *   node orchestrate.mjs "<url-da-live>"      # abre tudo + navega pra URL dada
 *   node orchestrate.mjs comment "Fala galera!"           # comenta em TODAS as contas
 *   node orchestrate.mjs comment "Oi" <profileId>         # comenta em UMA conta
 *   node orchestrate.mjs close-all             # fecha todas as contas
 */
import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const BASE = 'http://127.0.0.1:29222';
const DEFAULT_LIVE = 'https://www.youtube.com/live/g7Qhgx48odw?si=-1M-yjXDW-XtLAku';
const TOKEN_PATH = join(homedir(), 'AppData', 'Local', 'MultiContas', 'control_token');

function getToken() {
  try { return readFileSync(TOKEN_PATH, 'utf8').trim(); }
  catch { console.error('Token não encontrado em', TOKEN_PATH, '\nAbra o app Multi Contas primeiro.'); process.exit(1); }
}

async function api(method, path, body) {
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json', 'X-Control-Token': getToken() },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let json; try { json = JSON.parse(text); } catch { json = text; }
  if (!res.ok) { console.error(`[${res.status}] ${path}`, json); process.exit(1); }
  return json;
}

const sleep = (ms) => new Promise(r => setTimeout(r, ms));

async function openAllAndNavigate(url) {
  console.log('→ Abrindo TODAS as contas e navegando para:', url);
  const r = await api('POST', '/api/v1/profiles/open-all-and-navigate', { url });
  console.log(JSON.stringify(r, null, 2));
  // dá tempo das janelas subirem e o CDP registrar o ws_url
  await sleep(8000);
  const profiles = await api('GET', '/api/v1/profiles');
  console.log('\nContas abertas:');
  for (const p of profiles.profiles) {
    console.log(`  • ${p.name}  (porta ${p.cdpPort})  status=${p.status}`);
  }
  return profiles;
}

async function commentAll(text) {
  const profiles = await api('GET', '/api/v1/profiles');
  const abertas = profiles.profiles.filter(p => p.cdpPort);
  if (!abertas.length) { console.error('Nenhuma conta aberta. Rode primeiro: node orchestrate.mjs "<url>"'); process.exit(1); }
  console.log(`→ Comentando em ${abertas.length} conta(s): "${text}"`);
  for (const p of abertas) {
    try {
      const r = await api('POST', `/api/v1/profiles/${p.id}/comment`, { text });
      console.log(`  ✓ ${p.name}: ${JSON.stringify(r)}`);
    } catch (e) {
      console.log(`  ✗ ${p.name}: falhou (verifique se a live já iniciou e a caixa de chat está visível)`);
    }
    await sleep(1500); // espaça para não parecer spam
  }
}

async function commentOne(text, id) {
  const r = await api('POST', `/api/v1/profiles/${id}/comment`, { text });
  console.log(`✓ ${id}:`, JSON.stringify(r));
}

async function closeAll() {
  const profiles = await api('GET', '/api/v1/profiles');
  for (const p of profiles.profiles.filter(p => p.cdpPort)) {
    try { await api('POST', `/api/v1/profiles/${p.id}/close`); console.log(`  ✓ fechou ${p.name}`); }
    catch { /* ignore */ }
  }
}

async function main() {
  const [cmd, arg1, arg2] = process.argv.slice(2);
  if (cmd === 'comment') {
    if (arg2) return commentOne(arg1, arg2);
    return commentAll(arg1);
  }
  if (cmd === 'close-all') return closeAll();
  // default: abre tudo e navega
  const url = cmd && cmd.startsWith('http') ? cmd : DEFAULT_LIVE;
  return openAllAndNavigate(url);
}

main().catch(e => { console.error(e); process.exit(1); });
