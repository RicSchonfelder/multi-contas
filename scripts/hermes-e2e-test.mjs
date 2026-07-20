#!/usr/bin/env node
/**
 * Teste E2E da feature "Hermes multi-conta via CDP".
 *
 * PRECISA: o app Multi Contas rodando (janela visivel) — o servidor Hermes
 * sobe dentro do run() do Tauri. O token e gerado na inicializacao e logado
 * no console do app: "[HERMES] listening on 127.0.0.1:29222 | token: xxxx..."
 *
 * Uso:
 *   HERMES_TOKEN=xxxx node scripts/hermes-e2e-test.mjs
 *   (ou ajuste a porta com HERMES_PORT)
 */

const PORT = process.env.HERMES_PORT || 29222;
const TOKEN = process.env.HERMES_TOKEN;
const BASE = `http://127.0.0.1:${PORT}`;

if (!TOKEN) {
  console.error("ERRO: defina HERMES_TOKEN (veja o console do app Multi Contas)");
  process.exit(1);
}

function auth() {
  return { Authorization: `Bearer ${TOKEN}` };
}

async function test(name, fn) {
  try {
    await fn();
    console.log(`  ✓ ${name}`);
  } catch (e) {
    console.error(`  ✗ ${name}: ${e.message}`);
    process.exitCode = 1;
  }
}

async function main() {
  console.log(`\n=== E2E Hermes @ ${BASE} ===\n`);

  await test("health SEM token -> 401", async () => {
    const r = await fetch(`${BASE}/api/v1/health`);
    if (r.status !== 401) throw new Error(`status ${r.status}`);
  });

  await test("health COM token -> 200", async () => {
    const r = await fetch(`${BASE}/api/v1/health`, { headers: auth() });
    if (r.status !== 200) throw new Error(`status ${r.status}`);
    const j = await r.json();
    if (!j.ok) throw new Error("ok != true");
  });

  await test("GET /profiles lista", async () => {
    const r = await fetch(`${BASE}/api/v1/profiles`, { headers: auth() });
    if (r.status !== 200) throw new Error(`status ${r.status}`);
    const j = await r.json();
    if (!Array.isArray(j)) throw new Error("nao e array");
    console.log(`    (${j.length} perfis)`);
  });

  // Pega o primeiro perfil disponivel para abrir
  const list = await (await fetch(`${BASE}/api/v1/profiles`, { headers: auth() })).json();
  const target = list.find((p) => p.status === "available") || list[0];
  if (!target) {
    console.log("  (nenhum perfil para testar open/close — pulando)");
    return;
  }

  await test(`POST /profiles/${target.id}/start -> abre + porta CDP`, async () => {
    const r = await fetch(`${BASE}/api/v1/profiles/${target.id}/start`, { method: "POST", headers: auth() });
    if (r.status !== 200) throw new Error(`status ${r.status}: ${await r.text()}`);
    const j = await r.json();
    if (!j.cdpPort) throw new Error("cdpPort ausente");
    console.log(`    cdpPort=${j.cdpPort}`);
    // Valida que o Chrome CDP responde na porta
    const v = await (await fetch(`http://127.0.0.1:${j.cdpPort}/json/version`)).json();
    if (!v.webSocketDebuggerUrl) throw new Error("CDP nao respondeu");
    console.log(`    CDP ok: ${v.webSocketDebuggerUrl.slice(0, 40)}...`);
  });

  await test(`POST /profiles/${target.id}/command -> proxy CDP`, async () => {
    const r = await fetch(`${BASE}/api/v1/profiles/${target.id}/command`, {
      method: "POST",
      headers: { ...auth(), "Content-Type": "application/json" },
      body: JSON.stringify({ id: 1, method: "Browser.getVersion", params: {} }),
    });
    if (r.status !== 200) throw new Error(`status ${r.status}: ${await r.text()}`);
    const j = await r.json();
    if (!j.result) throw new Error("result ausente");
    console.log(`    Chrome ${j.result.product || "?"}`);
  });

  await test(`POST /profiles/${target.id}/stop -> fecha`, async () => {
    const r = await fetch(`${BASE}/api/v1/profiles/${target.id}/stop`, { method: "POST", headers: auth() });
    if (r.status !== 200) throw new Error(`status ${r.status}: ${await r.text()}`);
  });

  console.log("\n=== E2E concluido ===\n");
}

main();
