#!/usr/bin/env node
/**
 * Orquestração Multi Contas via Hermes (servidor local tiny_http).
 *
 * Fluxo para "abrir a mesma live em todas as contas e comentar de cada uma":
 *
 *   1. Usa POST /api/v1/profiles/open-batch (respeita limite de 3 ativos).
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
 *   node orchestrate.mjs status                # mostra status do orquestrador
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
  if (!res.ok) {
    console.error(`[${res.status}] ${path}`, json);
    if (res.status === 429) {
      const retryMs = json?.retryAfterMs || 5000;
      console.error(`Limite de perfis atingido. Aguardando ${retryMs}ms...`);
      await sleep(retryMs);
      return null; // caller should re-check
    }
    if (res.status === 503) {
      console.error('Recursos insuficientes — tente novamente mais tarde.');
    }
    return null;
  }
  if (res.headers.get('deprecation') === 'true') {
    console.warn('[DEPRECATED]', path, '→ use open-batch');
  }
  return json;
}

const sleep = (ms) => new Promise(r => setTimeout(r, ms));

async function openBatch(profileIds, url) {
  console.log(`→ Abrindo ${profileIds.length} conta(s) via open-batch`);
  if (url) console.log('  Navegando para:', url);

  let result = await api('POST', '/api/v1/profiles/open-batch', {
    profile_ids: profileIds,
    navigate_to: url || undefined,
  });

  // Retry on 429 with delay
  for (let attempt = 0; attempt < 3 && !result; attempt++) {
    await sleep(6000);
    result = await api('POST', '/api/v1/profiles/open-batch', {
      profile_ids: profileIds,
      navigate_to: url || undefined,
    });
  }

  if (!result) {
    console.error('Falha ao abrir perfis após tentativas.');
    process.exit(1);
  }

  console.log(`  Abertos: ${result.opened?.length || 0}`);
  for (const p of result.opened || []) {
    console.log(`    ✓ ${p.id} (porta ${p.cdpPort})`);
  }
  console.log(`  Na fila: ${result.queued?.length || 0}`);
  for (const q of result.queued || []) {
    console.log(`    ⏳ ${q.id} (posição ${q.position}, ETA ${q.etaMs}ms)`);
  }
  console.log(`  Rejeitados: ${result.rejected?.length || 0}`);
  for (const r of result.rejected || []) {
    console.log(`    ✗ ${r.id}: ${r.reason} — ${r.error}`);
  }

  await sleep(8000);

  const profiles = await api('GET', '/api/v1/profiles');
  if (profiles) {
    console.log('\nContas abertas:');
    for (const p of profiles.profiles) {
      if (p.cdpPort) {
        console.log(`  • ${p.name}  (porta ${p.cdpPort})  status=${p.status}`);
      }
    }
  }

  return profiles;
}

async function getStatus() {
  const status = await api('GET', '/api/v1/orchestrator/status');
  if (!status) return;
  console.log('Orquestrador:');
  console.log(`  Ativos:  ${status.activeProfiles}/${status.maxActiveProfiles}`);
  console.log(`  Fila:    ${status.queueDepth}`);
  console.log('  Recursos:');
  console.log(`    RAM livre: ${status.resources?.freeRamMb}MB`);
  console.log(`    Load 1m:   ${status.resources?.load1m}`);
  console.log(`    Chrome:    ${status.resources?.chromeProcesses} procs`);
  console.log(`    Healthy:   ${status.resources?.healthy}`);
  if (status.circuitBreakers?.length) {
    console.log('  Circuit Breakers:');
    for (const cb of status.circuitBreakers) {
      console.log(`    ${cb.profileId}: ${cb.state}${cb.cooldownUntil ? ` até ${cb.cooldownUntil}` : ''}`);
    }
  }
  return status;
}

async function openAllAndNavigate(url) {
  // Fetch all available profile IDs first
  const list = await api('GET', '/api/v1/profiles');
  if (!list) {
    console.error('Não foi possível listar perfis.');
    process.exit(1);
  }

  const availableIds = (list.profiles || [])
    .filter(p => p.status === 'available' || !p.cdpPort)
    .map(p => p.id);

  if (!availableIds.length) {
    console.log('Nenhum perfil disponível para abrir.');
    return list;
  }

  return openBatch(availableIds, url);
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
    await sleep(1500);
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
  if (cmd === 'status') return getStatus();
  // default: abre tudo e navega
  const url = cmd && cmd.startsWith('http') ? cmd : DEFAULT_LIVE;
  return openAllAndNavigate(url);
}

main().catch(e => { console.error(e); process.exit(1); });
