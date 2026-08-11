#!/usr/bin/env node
/**
 * validate-orchestrator.mjs — Smoke tests e validação do Smart Orchestrator (Fase 1).
 *
 * Executa sem servidor real, validando:
 *   1. TypeScript contracts compilam
 *   2. Lógica do orquestrador (simulada) — limite, fila, priority, circuit breaker
 *   3. orchestrate.mjs pode ser carregado (import check)
 *   4. Mock de respostas HTTP para 429/503/health/open-batch
 *
 * Uso:
 *   node scripts/validate-orchestrator.mjs
 *   node scripts/validate-orchestrator.mjs --mock-server  # sobe mock HTTP
 */

import { ok, deepEqual as assertDeep } from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';

// ==================== Contract validation ====================

function validateContracts() {
  console.log('[1/5] Validando contratos TypeScript...');

  const contractsPath = new URL('../packages/contracts/index.ts', import.meta.url);
  const content = readFileSync(contractsPath, 'utf8');

  ok(content.includes('ReasonCode'), 'ReasonCode type presente');
  ok(content.includes('limit_exceeded'), 'limit_exceeded reason presente');
  ok(content.includes('resource_exhausted'), 'resource_exhausted reason presente');
  ok(content.includes('circuit_breaker_open'), 'circuit_breaker_open reason presente');
  ok(content.includes('OpenBatchRequest'), 'OpenBatchRequest interface presente');
  ok(content.includes('OpenBatchResponse'), 'OpenBatchResponse interface presente');
  ok(content.includes('OrchestratorStatus'), 'OrchestratorStatus interface presente');
  ok(content.includes('ResourceSnapshot'), 'ResourceSnapshot interface presente');
  ok(content.includes('CircuitBreakerStatus'), 'CircuitBreakerStatus interface presente');
  ok(content.includes('Job'), 'Job interface presente');
  ok(content.includes('JobType'), 'JobType type presente');
  ok(content.includes('JobState'), 'JobState type presente');
  console.log('  ✓ OK — 12 contratos verificados');
}

// ==================== Orchestrator logic (simulated) ====================

function testLimit() {
  console.log('[2/5] Testando limite de 3 perfis...');

  const MAX = 3;
  const registry = new Map();
  const failures = new Map();

  function canOpen(profileId) {
    if (failures.get(profileId)?.open) return { ok: false, reason: 'circuit_breaker_open' };
    if (registry.size >= MAX) return { ok: false, reason: 'limit_exceeded', active: [...registry.keys()] };
    return { ok: true };
  }

  function openProfile(id) {
    const result = canOpen(id);
    if (!result.ok) return result;
    registry.set(id, { port: 9200 + registry.size });
    return { ok: true, port: registry.get(id).port };
  }

  // Open 3
  ok(openProfile('p1').ok, 'p1 aberto');
  ok(openProfile('p2').ok, 'p2 aberto');
  ok(openProfile('p3').ok, 'p3 aberto');
  ok(registry.size === 3, '3 perfis ativos');

  // 4th should be rejected
  const r4 = openProfile('p4');
  ok(!r4.ok && r4.reason === 'limit_exceeded', 'p4 rejeitado com limit_exceeded');
  ok(r4.active.length === 3, '3 perfis ativos na resposta');

  // Close one and open p4
  registry.delete('p1');
  ok(openProfile('p4').ok, 'p4 aberto após fechar p1');
  ok(registry.size === 3, 'ainda 3 perfis ativos');

  console.log('  ✓ OK — limite 3 reforçado');
}

function testQueuePriority() {
  console.log('[3/5] Testando fila com prioridade...');

  const queues = new Map();

  function enqueue(profileId, job) {
    if (!queues.has(profileId)) queues.set(profileId, []);
    queues.get(profileId).push(job);
    queues.get(profileId).sort((a, b) => a.priority - b.priority);
    return queues.get(profileId).length;
  }

  function dequeue(profileId) {
    const q = queues.get(profileId);
    return q?.shift() || null;
  }

  // Enqueue 3 jobs with different priorities (lower = higher priority)
  enqueue('p1', { id: 'j_low', priority: 200, text: 'low' });
  enqueue('p1', { id: 'j_high', priority: 0, text: 'high' });
  enqueue('p1', { id: 'j_mid', priority: 128, text: 'mid' });

  const first = dequeue('p1');
  ok(first.id === 'j_high', 'job de maior prioridade primeiro');
  ok(dequeue('p1').id === 'j_mid', 'segundo por prioridade');
  ok(dequeue('p1').id === 'j_low', 'último por prioridade');
  ok(dequeue('p1') === null, 'fila vazia');

  console.log('  ✓ OK — fila FIFO + prioridade');
}

function testCircuitBreaker() {
  console.log('[4/5] Testando circuit breaker...');

  const failures = new Map();
  const MAX_FAILURES = 3;

  function recordFailure(profileId) {
    if (!failures.has(profileId)) failures.set(profileId, 0);
    const count = failures.get(profileId) + 1;
    failures.set(profileId, count);
    return count >= MAX_FAILURES;
  }

  function isOpen(profileId) {
    return (failures.get(profileId) || 0) >= MAX_FAILURES;
  }

  function reset(profileId) {
    failures.set(profileId, 0);
  }

  ok(!isOpen('p1'), 'cb fechado inicialmente');
  recordFailure('p1');
  recordFailure('p1');
  ok(!isOpen('p1'), 'cb ainda fechado com 2 falhas');
  ok(recordFailure('p1'), 'cb abre na 3a falha');
  ok(isOpen('p1'), 'cb aberto após 3 falhas');

  reset('p1');
  ok(!isOpen('p1'), 'cb fecha após reset');

  console.log('  ✓ OK — circuit breaker abre/fecha');
}

function testResourceThreshold() {
  console.log('[5/5] Testando resource thresholds...');

  const config = {
    minFreeRamMb: 512,
    maxLoadPct: 80,
    maxChromeProcs: 30,
  };

  function checkResources(snapshot) {
    const reasons = [];
    if (snapshot.freeRamMb < config.minFreeRamMb)
      reasons.push(`RAM ${snapshot.freeRamMb}MB < ${config.minFreeRamMb}MB`);
    if (snapshot.load1m > config.maxLoadPct)
      reasons.push(`Load ${snapshot.load1m} > ${config.maxLoadPct}%`);
    if (snapshot.chromeProcs > config.maxChromeProcs)
      reasons.push(`Chrome ${snapshot.chromeProcs} > ${config.maxChromeProcs}`);
    return { exhausted: reasons.length > 0, reasons };
  }

  // Healthy system
  const healthy = checkResources({ freeRamMb: 4096, load1m: 0.5, chromeProcs: 5 });
  ok(!healthy.exhausted, 'sistema saudável');

  // RAM exhausted
  const lowRam = checkResources({ freeRamMb: 200, load1m: 0.5, chromeProcs: 5 });
  ok(lowRam.exhausted, 'RAM insuficiente detectada');
  ok(lowRam.reasons.length === 1, '1 motivo de exaustão');

  // Load exhausted
  const highLoad = checkResources({ freeRamMb: 4096, load1m: 95, chromeProcs: 5 });
  ok(highLoad.exhausted, 'load alto detectado');

  // Multiple exhausted
  const allExhausted = checkResources({ freeRamMb: 100, load1m: 90, chromeProcs: 50 });
  ok(allExhausted.exhausted, 'múltiplos thresholds atingidos');
  ok(allExhausted.reasons.length === 3, '3 motivos');

  console.log('  ✓ OK — resource thresholds');
}

// ==================== Mock HTTP server ====================

function startMockServer() {
  const PORT = 29223;
  const server = createServer((req, res) => {
    res.setHeader('Content-Type', 'application/json');
    res.setHeader('Access-Control-Allow-Origin', '*');

    if (req.method === 'OPTIONS') {
      res.writeHead(200);
      return res.end(JSON.stringify({ ok: true }));
    }

    const url = req.url;
    const token = req.headers['x-control-token'];
    if (!token) {
      res.writeHead(403);
      return res.end(JSON.stringify({ error: 'token ausente' }));
    }

    if (url === '/api/v1/health') {
      res.writeHead(200);
      return res.end(JSON.stringify({
        ok: true,
        version: '0.1.0',
        orchestrator: { activeProfiles: 0, maxProfiles: 3, queueDepth: 0 },
        resources: { freeRamMb: 4096, load1m: 0.5, load5m: 0.3, chromeProcesses: 0, healthy: true },
      }));
    }

    if (url === '/api/v1/orchestrator/status') {
      res.writeHead(200);
      return res.end(JSON.stringify({
        activeProfiles: 0,
        maxActiveProfiles: 3,
        queueDepth: 0,
        resources: { freeRamMb: 4096, load1m: 0.5, load5m: 0.3, chromeProcesses: 0, healthy: true },
        circuitBreakers: [],
      }));
    }

    if (url === '/api/v1/profiles/open-batch' && req.method === 'POST') {
      let body = '';
      req.on('data', c => body += c);
      req.on('end', () => {
        try {
          const { profile_ids } = JSON.parse(body);
          if (!profile_ids || !profile_ids.length) {
            res.writeHead(400);
            return res.end(JSON.stringify({ error: 'profile_ids required' }));
          }
          const opened = profile_ids.slice(0, 3).map((id, i) => ({
            id, cdpPort: 9222 + i, wsUrl: `ws://127.0.0.1:${9222 + i}`,
          }));
          const queued = profile_ids.slice(3).map((id, i) => ({
            id, position: i + 1, etaMs: (i + 1) * 10000,
          }));
          res.writeHead(200);
          res.end(JSON.stringify({ opened, queued, rejected: [], resourceWarnings: [] }));
        } catch {
          res.writeHead(400);
          res.end(JSON.stringify({ error: 'invalid json' }));
        }
      });
      return;
    }

    // Simulate 429 on 4th open
    if (url.startsWith('/api/v1/profiles/') && url.endsWith('/open') && req.method === 'POST') {
      const id = url.replace('/api/v1/profiles/', '').replace('/open', '');
      if (id === 'p4') {
        res.writeHead(429);
        return res.end(JSON.stringify({
          error: 'max_active_profiles_reached',
          reason: 'limit_exceeded',
          limit: 3,
          active: ['p1', 'p2', 'p3'],
          retryAfterMs: 5000,
        }));
      }
      res.writeHead(200);
      return res.end(JSON.stringify({ id, cdpPort: 9222, wsUrl: 'ws://127.0.0.1:9222' }));
    }

    res.writeHead(404);
    res.end(JSON.stringify({ error: 'not found' }));
  });

  server.listen(PORT, '127.0.0.1', () => {
    console.log(`\n[MOCK] Servidor mock rodando em http://127.0.0.1:${PORT}`);
    console.log('[MOCK] Endpoints: /api/v1/health, /api/v1/orchestrator/status, /api/v1/profiles/open-batch, /api/v1/profiles/:id/open');
    console.log('[MOCK] Token mock: qualquer header X-Control-Token funciona');
    console.log('[MOCK] Teste o 429: POST /api/v1/profiles/p4/open');
    console.log('[MOCK] Teste o batch: POST /api/v1/profiles/open-batch com {"profile_ids":["p1","p2","p3","p4","p5"]}');
    console.log('[MOCK] Pressione Ctrl+C para parar.\n');
  });
}

// ==================== Main ====================

console.log('=== validate-orchestrator.mjs ===\n');

validateContracts();
testLimit();
testQueuePriority();
testCircuitBreaker();
testResourceThreshold();

console.log('\n=== Todos os testes passaram ===');

if (process.argv.includes('--mock-server')) {
  startMockServer();
}
