interface Credentials { email: string; password: string; url: string; }
interface FingerprintConfig {
  enabled: boolean; userAgent: string; platform: string; resolution: string;
  timezone: string; geolocation: string; webglVendor: string; webglRenderer: string;
  hardwareConcurrency: number; deviceMemory: number; canvasNoise: boolean;
  audioNoise: boolean; fonts: string[]; language: string;
}

interface ProxyConfig {
  host: string; port: number; type: string;
  username?: string; password?: string; pacUrl?: string; bypassList: string[];
}

interface ProfileGroup {
  id: string; name: string; color: string; sortOrder: number; createdAt: string;
}

interface Extension {
  id: string; name: string; description: string; version: string; enabled: boolean; installUrl: string;
}

interface Profile {
  id: string; name: string; color: string; description: string;
  tags: string[]; status: string; proxy: ProxyConfig | null; credentials: Credentials | null; fingerprint: FingerprintConfig | null; extensions: Extension[];
  localDir: string; createdAt: string; updatedAt: string; lastOpenedAt: string | null;
  groupId: string | null;
}

let mockGroups: ProfileGroup[] = [
  { id: "g1", name: "Clientes", color: "#F59E0B", sortOrder: 1, createdAt: "2026-07-10T08:00:00Z" },
  { id: "g2", name: "Pessoal", color: "#10B981", sortOrder: 2, createdAt: "2026-07-10T08:00:00Z" },
  { id: "g3", name: "Trabalho", color: "#6366F1", sortOrder: 3, createdAt: "2026-07-10T08:00:00Z" },
];

let profiles: Profile[] = [
  { id: "1", name: "Grupo Soluções", color: "#6366F1", description: "Conta Google principal", tags: ["google","work"], status: "available", proxy: null, credentials: { email: "seu@email.com", password: "sua-senha", url: "https://accounts.google.com" }, fingerprint: null, extensions: [], localDir: "", createdAt: "2026-07-10T10:00:00Z", updatedAt: "2026-07-10T10:00:00Z", lastOpenedAt: null, groupId: "g3" },
  { id: "2", name: "Pessoal", color: "#10B981", description: "Redes sociais e pessoal", tags: ["social"], status: "in_use", proxy: null, credentials: null, fingerprint: null, extensions: [], localDir: "", createdAt: "2026-07-10T11:00:00Z", updatedAt: "2026-07-10T11:00:00Z", lastOpenedAt: "2026-07-10T12:00:00Z", groupId: "g2" },
  { id: "3", name: "Cliente A", color: "#F59E0B", description: "Gestão de tráfego", tags: ["cliente","ads"], status: "available", proxy: { host: "192.168.1.100", port: 3128, type: "http", bypassList: [] }, credentials: null, fingerprint: null, extensions: [], localDir: "", createdAt: "2026-07-10T09:00:00Z", updatedAt: "2026-07-10T09:00:00Z", lastOpenedAt: null, groupId: "g1" },
  { id: "4", name: "Dev", color: "#EF4444", description: "Testes e desenvolvimento", tags: ["dev","test"], status: "available", proxy: null, credentials: null, fingerprint: null, extensions: [], localDir: "", createdAt: "2026-07-09T08:00:00Z", updatedAt: "2026-07-09T08:00:00Z", lastOpenedAt: "2026-07-09T18:00:00Z", groupId: null },
];

let nextId = 5;
let nextGroupId = 4;

export function isTauri(): boolean {
  try { return !!(window as any).__TAURI_INTERNALS__; }
  catch { return false; }
}

export async function mockInvoke<T>(cmd: string, args?: any): Promise<T> {
  await new Promise(r => setTimeout(r, 200));

  switch (cmd) {
    case "list_profiles": return [...profiles] as any;
    case "create_profile": {
      const p: Profile = { id: String(nextId++), name: args.name, color: args.color || "#6366F1", description: args.description || "", tags: args.tags || [], status: "available", proxy: null, credentials: args.credentials || null, fingerprint: args.fingerprint || null, extensions: [], groupId: null, localDir: "", createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), lastOpenedAt: null };
      profiles.push(p); return p as any;
    }
    case "update_profile": {
      const p = profiles.find(x => x.id === args.id); if (!p) throw "not found";
      if (args.name !== undefined) p.name = args.name;
      if (args.color !== undefined) p.color = args.color;
      if (args.description !== undefined) p.description = args.description;
      if (args.tags !== undefined) p.tags = args.tags;
      if (args.credentials !== undefined) p.credentials = args.credentials;
      if (args.fingerprint !== undefined) p.fingerprint = args.fingerprint;
      p.updatedAt = new Date().toISOString();
      return p as any;
    }
    case "delete_profile": { profiles = profiles.filter(x => x.id !== args.id); return undefined as any; }
    case "set_profile_proxy": {
      const p2 = profiles.find(x => x.id === args.id); if (!p2) throw "not found";
      p2.proxy = args.proxy; p2.updatedAt = new Date().toISOString();
      return p2 as any;
    }
    case "open_profile": {
      const p3 = profiles.find(x => x.id === args.id); if (!p3) throw "not found";
      p3.status = "in_use";
      return undefined as any;
    }
    case "close_profile": {
      const p4 = profiles.find(x => x.id === args.id); if (!p4) throw "not found";
      p4.status = "available";
      return undefined as any;
    }
    case "get_chrome_path": return "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe" as any;
    case "set_profile_credentials": {
      const pc = profiles.find(x => x.id === args.id); if (!pc) throw "not found";
      pc.credentials = args.credentials; pc.updatedAt = new Date().toISOString();
      return pc as any;
    }
    case "set_profile_fingerprint": {
      const pf = profiles.find(x => x.id === args.id); if (!pf) throw "not found";
      pf.fingerprint = args.fingerprint; pf.updatedAt = new Date().toISOString();
      return pf as any;
    }
    case "export_profile_cookies": {
      const pc = profiles.find(x => x.id === args.id); if (!pc) throw "not found";
      if (pc.status !== "in_use") throw "Profile must be open to export cookies";
      return JSON.stringify([{name:"example",value:"mock",domain:".example.com",path:"/",secure:true,httpOnly:false,sameSite:"None",expires:Math.floor(Date.now()/1000)+86400}]) as any;
    }
    case "import_profile_cookies": {
      const pi = profiles.find(x => x.id === args.id); if (!pi) throw "not found";
      if (pi.status !== "in_use") throw "Profile must be open to import cookies";
      return undefined as any;
    }
    case "list_groups": return [...mockGroups] as any;
    case "create_group": {
      const g: ProfileGroup = { id: String(nextGroupId++), name: args.name, color: args.color || "#6366F1", sortOrder: mockGroups.length + 1, createdAt: new Date().toISOString() };
      mockGroups.push(g); return g as any;
    }
    case "update_group": {
      const g = mockGroups.find(x => x.id === args.id); if (!g) throw "group not found";
      if (args.name !== undefined) g.name = args.name;
      if (args.color !== undefined) g.color = args.color;
      return g as any;
    }
    case "delete_group": {
      mockGroups = mockGroups.filter(x => x.id !== args.id);
      profiles.forEach(p => { if (p.groupId === args.id) p.groupId = null; });
      return undefined as any;
    }
    case "set_profile_group": {
      const p = profiles.find(x => x.id === args.profileId); if (!p) throw "not found";
      p.groupId = args.groupId || null;
      return p as any;
    }
    case "add_profile_extension": {
      const pa = profiles.find(x => x.id === args.id); if (!pa) throw "not found";
      if (!pa.extensions) pa.extensions = [];
      pa.extensions.push({ id: args.extId, name: args.name, description: args.description || "", version: args.version || "1.0", enabled: true, installUrl: args.installUrl || "" });
      pa.updatedAt = new Date().toISOString();
      return pa as any;
    }
    case "remove_profile_extension": {
      const pr = profiles.find(x => x.id === args.id); if (!pr) throw "not found";
      pr.extensions = (pr.extensions || []).filter(e => e.id !== args.extId);
      pr.updatedAt = new Date().toISOString();
      return pr as any;
    }
    case "toggle_profile_extension": {
      const pt = profiles.find(x => x.id === args.id); if (!pt) throw "not found";
      const ext = (pt.extensions || []).find(e => e.id === args.extId);
      if (ext) ext.enabled = args.enabled;
      pt.updatedAt = new Date().toISOString();
      return pt as any;
    }
    default: throw `unknown command: ${cmd}`;
  }
}
