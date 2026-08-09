import { useState, useEffect, useCallback, useRef } from "react";
import { isTauri, mockInvoke as mock } from "./mock";
import "./App.css";

async function invoke<T = any>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return mock<T>(cmd, args);
  const { invoke: ti } = await import("@tauri-apps/api/core");
  return ti<T>(cmd, args);
}

interface Credentials { email: string; password: string; url: string; }
interface ProxyConfig { host: string; port: number; type: string; username?: string; password?: string; pacUrl?: string; bypassList: string[]; }
interface FingerprintConfig {
  enabled: boolean; userAgent: string; platform: string; resolution: string;
  timezone: string; geolocation: string; webglVendor: string; webglRenderer: string;
  hardwareConcurrency: number; deviceMemory: number; canvasNoise: boolean;
  audioNoise: boolean; fonts: string[]; language: string;
}
interface Extension { id: string; name: string; description: string; version: string; enabled: boolean; installUrl: string; }
interface ProfileGroup { id: string; name: string; color: string; sortOrder: number; createdAt: string; }
interface Profile { id: string; name: string; color: string; description: string; tags: string[]; status: string; proxy: ProxyConfig | null; credentials: Credentials | null; fingerprint: FingerprintConfig | null; extensions: Extension[]; localDir: string; createdAt: string; updatedAt: string; lastOpenedAt: string | null; groupId: string | null; }

const COLORS = ["#6366F1","#EC4899","#10B981","#F59E0B","#3B82F6","#EF4444","#8B5CF6","#14B8A6"];

function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [groups, setGroups] = useState<ProfileGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [color, setColor] = useState(COLORS[0]);
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState("");
  const [search, setSearch] = useState("");
  const [tab, setTab] = useState<"basico" | "login" | "fingerprint">("basico");
  const [activeGroup, setActiveGroup] = useState<string | null>(null);
  const [showGroupForm, setShowGroupForm] = useState(false);
  const [groupName, setGroupName] = useState("");
  const [groupColor, setGroupColor] = useState(COLORS[1]);
  const [editGroupId, setEditGroupId] = useState<string | null>(null);
  const [showGroupManager, setShowGroupManager] = useState(false);
  const [selectedGroupId, setSelectedGroupId] = useState<string>("");

  // Credentials fields (inside form)
  const [credEmail, setCredEmail] = useState("");
  const [credPass, setCredPass] = useState("");
  const [credUrl, setCredUrl] = useState("https://accounts.google.com");

  // Fingerprint
  const [fpEnabled, setFpEnabled] = useState(false);
  const [fpUa, setFpUa] = useState("");
  const [fpPlatform, setFpPlatform] = useState("Win32");
  const [fpResolution, setFpResolution] = useState("1920x1080");
  const [fpTz, setFpTz] = useState("America/Sao_Paulo");
  const [fpGeo, setFpGeo] = useState("-23.5505,-46.6333");
  const [fpLang, setFpLang] = useState("pt-BR");
  const [fpVendor, setFpVendor] = useState("");
  const [fpRenderer, setFpRenderer] = useState("");
  const [fpHc, setFpHc] = useState("4");
  const [fpDm, setFpDm] = useState("8");
  const [fpCanvas, setFpCanvas] = useState(true);
  const [fpAudio, setFpAudio] = useState(true);

  // Extensions modal
  const [extProfileId, setExtProfileId] = useState<string | null>(null);
  const [extensions, setExtensions] = useState<Extension[]>([]);
  const [showAddExt, setShowAddExt] = useState(false);
  const [extName, setExtName] = useState("");
  const [extId, setExtId] = useState("");
  const [extVersion, setExtVersion] = useState("1.0");
  const [extUrl, setExtUrl] = useState("");

  // Proxy modal
  const [proxyProfileId, setProxyProfileId] = useState<string | null>(null);
  const [proxyHost, setProxyHost] = useState("");
  const [proxyPort, setProxyPort] = useState("8080");
  const [proxyType, setProxyType] = useState("http");
  const [proxyUser, setProxyUser] = useState("");
  const [proxyPass, setProxyPass] = useState("");
  const [proxyPac, setProxyPac] = useState("");

  // Cookie modal
  const [cookieProfileId, setCookieProfileId] = useState<string | null>(null);
  const [cookieData, setCookieData] = useState("");
  const [cookieCount, setCookieCount] = useState(0);
  const [cookieImportText, setCookieImportText] = useState("");
  const [cookieError, setCookieError] = useState("");
  const [cookieSuccess, setCookieSuccess] = useState("");
  const [csvMsg, setCsvMsg] = useState("");

  const handleExportCsv = async () => {
    try {
      const csv = await invoke<string>("export_profiles_csv");
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a"); a.href = url; a.download = `perfis-${new Date().toISOString().slice(0,10)}.csv`; a.click();
      URL.revokeObjectURL(url);
      setCsvMsg("CSV exportado com sucesso!");
      setTimeout(() => setCsvMsg(""), 3000);
    } catch (e: any) { alert(e); }
  };

  const handleImportCsv = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".csv";
    input.onchange = async (e: any) => {
      const file = e.target?.files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const count = await invoke<number>("import_profiles_csv", { csvData: text });
        setCsvMsg(`${count} perfil(is) importado(s) com sucesso!`);
        setTimeout(() => setCsvMsg(""), 4000);
        load();
      } catch (err: any) { alert(err); }
    };
    input.click();
  };

  const profilesRef = useRef(profiles);
  profilesRef.current = profiles;
  const groupsRef = useRef(groups);
  groupsRef.current = groups;
  const load = useCallback(async () => {
    try {
      const next = (await invoke<Profile[]>("list_profiles")) || [];
      const nextGroups = (await invoke<ProfileGroup[]>("list_groups")) || [];
      if (JSON.stringify(next) !== JSON.stringify(profilesRef.current)) setProfiles(next);
      if (JSON.stringify(nextGroups) !== JSON.stringify(groupsRef.current)) setGroups(nextGroups);
    }
    catch (e) { console.error(e); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); invoke<string>("get_chrome_path").catch(()=>{}); const iv = setInterval(()=>{ if (document.visibilityState === "visible") load(); }, 10000); const onShow = ()=>{ if (document.visibilityState === "visible") load(); }; document.addEventListener("visibilitychange", onShow); return ()=>{ clearInterval(iv); document.removeEventListener("visibilitychange", onShow); }; }, [load]);

  const resetForm = () => {
    setEditId(null); setName(""); setColor(COLORS[0]); setDescription(""); setTags("");
    setCredEmail(""); setCredPass(""); setCredUrl("https://accounts.google.com");
    setFpEnabled(false); setFpUa(""); setFpPlatform("Win32"); setFpResolution("1920x1080");
    setFpTz("America/Sao_Paulo"); setFpGeo("-23.5505,-46.6333"); setFpLang("pt-BR");
    setFpVendor(""); setFpRenderer(""); setFpHc("4"); setFpDm("8"); setFpCanvas(true); setFpAudio(true);
    setSelectedGroupId("");
    setTab("basico");
  };
  const openNewForm = () => { resetForm(); setShowForm(true); };
  const openEditForm = (p: Profile) => {
    setEditId(p.id); setName(p.name); setColor(p.color); setDescription(p.description);
    setTags(p.tags?.join(", ")||"");
    setCredEmail(p.credentials?.email||""); setCredPass(""); setCredUrl(p.credentials?.url||"https://accounts.google.com");
    setFpEnabled(p.fingerprint?.enabled||false); setFpUa(p.fingerprint?.userAgent||"");
    setFpPlatform(p.fingerprint?.platform||"Win32"); setFpResolution(p.fingerprint?.resolution||"1920x1080");
    setFpTz(p.fingerprint?.timezone||"America/Sao_Paulo"); setFpGeo(p.fingerprint?.geolocation||"-23.5505,-46.6333");
    setFpLang(p.fingerprint?.language||"pt-BR"); setFpVendor(p.fingerprint?.webglVendor||"");
    setFpRenderer(p.fingerprint?.webglRenderer||""); setFpHc(String(p.fingerprint?.hardwareConcurrency||4));
    setFpDm(String(p.fingerprint?.deviceMemory||8)); setFpCanvas(p.fingerprint?.canvasNoise??true);
    setFpAudio(p.fingerprint?.audioNoise??true);
    setSelectedGroupId(p.groupId || "");
    setShowForm(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault(); if (!name.trim()) return;
    try {
      const creds = credEmail.trim() ? { email: credEmail.trim(), password: credPass, url: credUrl || "https://accounts.google.com" } : null;
      const fp: FingerprintConfig | null = fpEnabled ? {
        enabled: true, userAgent: fpUa, platform: fpPlatform, resolution: fpResolution,
        timezone: fpTz, geolocation: fpGeo, webglVendor: fpVendor, webglRenderer: fpRenderer,
        hardwareConcurrency: parseInt(fpHc)||4, deviceMemory: parseInt(fpDm)||8,
        canvasNoise: fpCanvas, audioNoise: fpAudio, fonts: [], language: fpLang,
      } : null;
      const args: any = { name: name.trim(), color, description: description.trim()||null, tags: tags?tags.split(",").map(t=>t.trim()).filter(Boolean):null, credentials: creds, fingerprint: fp };
      if (editId) {
        args.id = editId;
        const updated = await invoke<Profile>("update_profile", args);
        const newGroupId = selectedGroupId || null;
        if (updated.groupId !== newGroupId) {
          await invoke("set_profile_group", { profileId: editId, groupId: newGroupId });
        }
      } else {
        const created = await invoke<Profile>("create_profile", args);
        const newGroupId = selectedGroupId || null;
        if (newGroupId) {
          await invoke("set_profile_group", { profileId: created.id, groupId: newGroupId });
        }
      }
      setShowForm(false); load();
    } catch (e: any) { alert(e); }
  };

  const handleDelete = async (id: string) => { if (!confirm("Excluir este perfil?")) return; await invoke("delete_profile", { id }); load(); };
  const handleSaveGroup = async () => {
    if (!groupName.trim()) return;
    try {
      if (editGroupId) { await invoke("update_group", { id: editGroupId, name: groupName.trim(), color: groupColor }); }
      else { await invoke("create_group", { name: groupName.trim(), color: groupColor }); }
      setShowGroupForm(false); setEditGroupId(null); setGroupName(""); setGroupColor(COLORS[1]); load();
    } catch (e: any) { alert(e); }
  };
  const handleDeleteGroup = async (id: string) => {
    if (!confirm("Excluir grupo? Os perfis permanecem sem grupo.")) return;
    await invoke("delete_group", { id }); if (activeGroup === id) setActiveGroup(null); load();
  };
  const openEditGroup = (g: ProfileGroup) => { setEditGroupId(g.id); setGroupName(g.name); setGroupColor(g.color); setShowGroupForm(true); };
  const openNewGroup = () => { setEditGroupId(null); setGroupName(""); setGroupColor(COLORS[1]); setShowGroupForm(true); };
  const handleOpen = (id: string) => invoke("open_profile", { id }).catch(alert);
  const handleClose = (id: string) => invoke("close_profile", { id }).catch(alert);

  const handleSaveProxy = async () => {
    if (!proxyProfileId) return;
    try {
      const proxy = proxyHost ? { host: proxyHost, port: parseInt(proxyPort)||8080, type: proxyType, username: proxyUser||undefined, password: proxyPass||undefined, pacUrl: proxyPac||undefined, bypassList: [] } : null;
      await invoke("set_profile_proxy", { id: proxyProfileId, proxy }); setProxyProfileId(null); load();
    } catch (e: any) { alert(e); }
  };

  const openExtensionsModal = (p: Profile) => { setExtProfileId(p.id); setExtensions(p.extensions || []); setShowAddExt(false); };

  const handleAddExtension = async () => {
    if (!extProfileId || !extName.trim() || !extId.trim()) return;
    try {
      const updated = await invoke<Profile>("add_profile_extension", {
        id: extProfileId, extId: extId.trim(), name: extName.trim(), description: "",
        version: extVersion || "1.0",
        installUrl: extUrl.trim() || `https://clients2.google.com/service/update2/crx?response=redirect&prod=chrome&x=id%3D${extId.trim()}%26installsource%3Dondemand%26uc`,
      });
      setExtensions(updated.extensions || []); setShowAddExt(false);
      setExtName(""); setExtId(""); setExtVersion("1.0"); setExtUrl(""); load();
    } catch (e: any) { alert(e); }
  };

  const handleToggleExtension = async (extId: string, enabled: boolean) => {
    if (!extProfileId) return;
    try { const updated = await invoke<Profile>("toggle_profile_extension", { id: extProfileId, extId, enabled }); setExtensions(updated.extensions || []); load(); }
    catch (e: any) { alert(e); }
  };

  const handleRemoveExtension = async (extId: string) => {
    if (!extProfileId || !confirm("Remover esta extensão?")) return;
    try { const updated = await invoke<Profile>("remove_profile_extension", { id: extProfileId, extId }); setExtensions(updated.extensions || []); load(); }
    catch (e: any) { alert(e); }
  };

  const openProxyModal = (p: Profile) => { setProxyProfileId(p.id); setProxyHost(p.proxy?.host||""); setProxyPort(String(p.proxy?.port||8080)); setProxyType(p.proxy?.type||"http"); setProxyUser(p.proxy?.username||""); setProxyPass(""); setProxyPac(p.proxy?.pacUrl||""); };

  const filtered = profiles.filter(p => {
    if (search && !p.name.toLowerCase().includes(search.toLowerCase()) && !p.tags?.some(t=>t.includes(search))) return false;
    if (activeGroup && p.groupId !== activeGroup) return false;
    return true;
  });

  const getGroup = (id: string | null) => id ? groups.find(g => g.id === id) : null;

  return (
    <div className="app">
      <header>
        <div className="logo"><div className="logo-icon"/><h1>Multi Contas</h1></div>
        <div className="header-actions">
          <input className="search" placeholder="Buscar..." value={search} onChange={e=>setSearch(e.target.value)}/>
          <button className="btn small" onClick={handleExportCsv} title="Exportar perfis para CSV">📥 CSV</button>
          <button className="btn small" onClick={handleImportCsv} title="Importar perfis de um arquivo CSV">📤 CSV</button>
          <button className="btn primary" onClick={openNewForm}>+ Novo Perfil</button>
        </div>
      </header>
      {csvMsg && <div className="csv-msg">{csvMsg}</div>}

      {/* Form Modal — with credentials tab */}
      {showForm && (
        <div className="modal-overlay" onClick={()=>setShowForm(false)}>
          <div className="modal modal-wide" onClick={e=>e.stopPropagation()}>
            <h2>{editId?"Editar":"Novo"} Perfil</h2>
            <div className="form-tabs">
              <button className={`form-tab ${tab==="basico"?"active":""}`} onClick={()=>setTab("basico")}>Dados</button>
              <button className={`form-tab ${tab==="login"?"active":""}`} onClick={()=>setTab("login")}>Login</button>
              <button className={`form-tab ${tab==="fingerprint"?"active":""}`} onClick={()=>setTab("fingerprint")}>Anti-Fingerprint</button>
            </div>
            <form onSubmit={handleSave}>
              {tab === "basico" && (
                <>
                  <input placeholder="Nome do perfil" value={name} onChange={e=>setName(e.target.value)} required/>
                  <textarea placeholder="Descrição" value={description} onChange={e=>setDescription(e.target.value)} rows={2}/>
                  <input placeholder="Tags (separadas por vírgula)" value={tags} onChange={e=>setTags(e.target.value)}/>
                  <select value={selectedGroupId} onChange={e=>setSelectedGroupId(e.target.value)}>
                    <option value="">Sem grupo</option>
                    {[...groups].sort((a,b) => a.sortOrder - b.sortOrder).map(g => (
                      <option key={g.id} value={g.id}>{g.name}</option>
                    ))}
                  </select>
                  <div className="color-picker">{COLORS.map(c=><button key={c} type="button" className={`color-btn ${color===c?"sel":""}`} style={{background:c}} onClick={()=>setColor(c)}/>)}</div>
                </>
              )}
              {tab === "login" && (
                <>
                  <p className="modal-hint">Ao clicar em "Abrir", o Chrome vai navegar para esta URL e preencher os dados automaticamente.</p>
                  <input placeholder="URL de login" value={credUrl} onChange={e=>setCredUrl(e.target.value)}/>
                  <input placeholder="Email / Usuário" value={credEmail} onChange={e=>setCredEmail(e.target.value)}/>
                  <input placeholder="Senha" type="password" value={credPass} onChange={e=>setCredPass(e.target.value)}/>
                </>
              )}
              {tab === "fingerprint" && (
                <>
                  <p className="modal-hint">Spoofing de fingerprint faz cada perfil parecer um computador diferente.</p>
                  <label className="fp-label"><input type="checkbox" checked={fpEnabled} onChange={e=>setFpEnabled(e.target.checked)}/> Ativar spoofing</label>
                  {fpEnabled && <>
                    <div className="form-row">
                      <input placeholder="User-Agent" value={fpUa} onChange={e=>setFpUa(e.target.value)} title="User-Agent"/>
                      <input placeholder="Platform" value={fpPlatform} onChange={e=>setFpPlatform(e.target.value)} title="Platform"/>
                    </div>
                    <div className="form-row">
                      <input placeholder="Resolução (ex: 1920x1080)" value={fpResolution} onChange={e=>setFpResolution(e.target.value)}/>
                      <input placeholder="Idioma (pt-BR)" value={fpLang} onChange={e=>setFpLang(e.target.value)}/>
                    </div>
                    <div className="form-row">
                      <input placeholder="Fuso (America/Sao_Paulo)" value={fpTz} onChange={e=>setFpTz(e.target.value)}/>
                      <input placeholder="Geo (lat,lon)" value={fpGeo} onChange={e=>setFpGeo(e.target.value)}/>
                    </div>
                    <div className="form-row">
                      <input placeholder="WebGL Vendor" value={fpVendor} onChange={e=>setFpVendor(e.target.value)}/>
                      <input placeholder="WebGL Renderer" value={fpRenderer} onChange={e=>setFpRenderer(e.target.value)}/>
                    </div>
                    <div className="form-row">
                      <input placeholder="CPU cores" type="number" value={fpHc} onChange={e=>setFpHc(e.target.value)} style={{flex:1}}/>
                      <input placeholder="RAM (GB)" type="number" value={fpDm} onChange={e=>setFpDm(e.target.value)} style={{flex:1}}/>
                    </div>
                    <label className="fp-label"><input type="checkbox" checked={fpCanvas} onChange={e=>setFpCanvas(e.target.checked)}/> Ruído Canvas</label>
                    <label className="fp-label"><input type="checkbox" checked={fpAudio} onChange={e=>setFpAudio(e.target.checked)}/> Ruído Áudio</label>
                  </>}
                </>
              )}
              <div className="modal-actions">
                <button type="button" className="btn" onClick={()=>setShowForm(false)}>Cancelar</button>
                <button type="submit" className="btn primary">{editId?"Salvar":"Criar Perfil"}</button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Proxy Modal */}
      {proxyProfileId && (
        <div className="modal-overlay" onClick={()=>setProxyProfileId(null)}>
          <div className="modal" onClick={e=>e.stopPropagation()}>
            <h2>Proxy</h2>
            <div className="form-row">
              <select value={proxyType} onChange={e=>setProxyType(e.target.value)}>
                <option value="http">HTTP</option><option value="https">HTTPS</option><option value="socks5">SOCKS5</option>
              </select>
              <input placeholder="Host" value={proxyHost} onChange={e=>setProxyHost(e.target.value)}/>
              <input placeholder="Porta" type="number" value={proxyPort} onChange={e=>setProxyPort(e.target.value)}/>
            </div>
            <input placeholder="PAC URL (opcional)" value={proxyPac} onChange={e=>setProxyPac(e.target.value)}/>
            <div className="form-row">
              <input placeholder="Usuário" value={proxyUser} onChange={e=>setProxyUser(e.target.value)}/>
              <input placeholder="Senha" type="password" value={proxyPass} onChange={e=>setProxyPass(e.target.value)}/>
            </div>
            <div className="modal-actions">
              <button className="btn" onClick={()=>{setProxyProfileId(null);invoke("set_profile_proxy",{id:proxyProfileId,proxy:null}).then(load);}}>Remover</button>
              <button className="btn primary" onClick={handleSaveProxy}>Salvar</button>
            </div>
          </div>
        </div>
      )}

      {/* Cookie Modal */}
      {cookieProfileId && (
        <div className="modal-overlay" onClick={()=>setCookieProfileId(null)}>
          <div className="modal modal-wide cookie-modal" onClick={e=>e.stopPropagation()}>
            <h2>🍪 Cookies</h2>

            {cookieError && <div className="alert error">{cookieError}</div>}
            {cookieSuccess && <div className="alert" style={{background:"rgba(16,185,129,0.1)",color:"#34d399",border:"1px solid rgba(16,185,129,0.15)",borderRadius:8,padding:"0.75rem 1rem",marginBottom:"1rem",fontSize:"0.85rem"}}>{cookieSuccess}</div>}

            <div className="modal-actions" style={{justifyContent:"flex-start",flexWrap:"wrap"}}>
              <button className="btn primary small" onClick={async ()=>{
                setCookieError("");setCookieSuccess("");
                try {
                  const raw = await invoke<string>("export_profile_cookies", { id: cookieProfileId });
                  const cookies = JSON.parse(raw);
                  setCookieData(raw);
                  setCookieCount(cookies.length);
                  setCookieSuccess(`Exportados ${cookies.length} cookies com sucesso!`);
                } catch(e:any){setCookieError(String(e));}
              }}>📤 Export Cookies</button>
              {cookieData && <button className="btn small" onClick={()=>{
                const blob = new Blob([cookieData], {type:"application/json"});
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a"); a.href = url; a.download = `cookies-${cookieProfileId}.txt`; a.click();
                URL.revokeObjectURL(url);
              }}>💾 Download Cookies File</button>}
            </div>

            {cookieCount > 0 && <p style={{fontSize:"0.85rem",color:"#a5b4fc",marginBottom:"0.75rem"}}>{cookieCount} cookies exportados</p>}

            <textarea placeholder='Cole o JSON dos cookies aqui...' value={cookieImportText} onChange={e=>setCookieImportText(e.target.value)}/>
            <div className="modal-actions">
              <button className="btn" onClick={()=>setCookieProfileId(null)}>Fechar</button>
              <button className="btn primary" disabled={!cookieImportText.trim()} onClick={async ()=>{
                setCookieError("");setCookieSuccess("");
                try {
                  await invoke("import_profile_cookies", { id: cookieProfileId, cookiesJson: cookieImportText });
                  setCookieSuccess("Cookies importados com sucesso!");
                  setCookieImportText("");
                } catch(e:any){setCookieError(String(e));}
              }}>📥 Import Cookies</button>
            </div>
          </div>
        </div>
      )}

      {/* Extensions Modal */}
      {extProfileId && (
        <div className="modal-overlay" onClick={()=>setExtProfileId(null)}>
          <div className="modal modal-wide" onClick={e=>e.stopPropagation()}>
            <h2>🧩 Extensões</h2>
            {!showAddExt ? (
              <>
                <div className="ext-list">
                  {extensions.length === 0 && <p className="modal-hint">Nenhuma extensão instalada.</p>}
                  {extensions.map(ext => (
                    <div key={ext.id} className="ext-item">
                      <div className="ext-info">
                        <div className="ext-name">{ext.name}</div>
                        <div className="ext-version">{ext.id} • v{ext.version}</div>
                      </div>
                      <div style={{display:"flex",alignItems:"center",gap:"0.5rem"}}>
                        <label className="switch">
                          <input type="checkbox" checked={ext.enabled} onChange={e=>handleToggleExtension(ext.id, e.target.checked)}/>
                          <span className="slider"/>
                        </label>
                        <button className="btn small danger" onClick={()=>handleRemoveExtension(ext.id)}>✕</button>
                      </div>
                    </div>
                  ))}
                </div>
                <div className="modal-actions">
                  <button className="btn" onClick={()=>setExtProfileId(null)}>Fechar</button>
                  <button className="btn primary" onClick={()=>{setShowAddExt(true); setExtName(""); setExtId(""); setExtVersion("1.0"); setExtUrl("");}}>+ Adicionar</button>
                </div>
              </>
            ) : (
              <>
                <p className="modal-hint">Adicione uma extensão da Chrome Web Store.</p>
                <input placeholder="Nome da extensão" value={extName} onChange={e=>setExtName(e.target.value)}/>
                <input placeholder="Extension ID (ex: nkbihfbeogaeaoehlefnkodbefgpgknn)" value={extId} onChange={e=>setExtId(e.target.value)}/>
                <input placeholder="Versão (1.0)" value={extVersion} onChange={e=>setExtVersion(e.target.value)}/>
                <input placeholder="Install URL (opcional)" value={extUrl} onChange={e=>setExtUrl(e.target.value)}/>
                <div className="modal-actions">
                  <button className="btn" onClick={()=>setShowAddExt(false)}>Voltar</button>
                  <button className="btn primary" onClick={handleAddExtension}>Adicionar</button>
                </div>
              </>
            )}
          </div>
        </div>
      )}

      <div className="app-layout">
        {/* Sidebar */}
        <aside className="sidebar">
          <div
            className={`sidebar-group ${!activeGroup ? "active" : ""}`}
            onClick={() => setActiveGroup(null)}
          >
            <span>Todos</span>
            <span style={{marginLeft:"auto",fontSize:"0.75rem",color:"var(--muted)"}}>{profiles.length}</span>
          </div>
          {[...groups].sort((a,b) => a.sortOrder - b.sortOrder).map(g => (
            <div
              key={g.id}
              className={`sidebar-group ${activeGroup === g.id ? "active" : ""}`}
              onClick={() => setActiveGroup(g.id)}
            >
              <span className="sidebar-group-dot" style={{background:g.color}}/>
              <span>{g.name}</span>
              <span style={{marginLeft:"auto",fontSize:"0.75rem",color:"var(--muted)"}}>{profiles.filter(p => p.groupId === g.id).length}</span>
            </div>
          ))}
          <div style={{marginTop:"0.75rem",display:"flex",gap:"0.35rem"}}>
            <button className="btn small" style={{flex:1}} onClick={openNewGroup}>+ Novo Grupo</button>
            <button className="btn small" onClick={() => setShowGroupManager(true)} title="Gerenciar Grupos">⚙</button>
          </div>
        </aside>

        {/* Main area */}
        <div className="main-area">
          {loading ? <div className="loading">Carregando...</div> :
           filtered.length === 0 ? <div className="empty">{search?"Nenhum perfil encontrado":"Nenhum perfil ainda. Crie o primeiro!"}</div> :
           <div className="grid">
             {filtered.map(p => {
               const grp = getGroup(p.groupId);
               return (
               <div key={p.id} className={`card ${p.status==="in_use"?"active":""}`}>
                 <div className="card-top">
                   <div className="avatar" style={{background:p.color}}>{p.name[0]?.toUpperCase()}</div>
                   <span className={`badge ${p.status}`}>{p.status==="in_use"?"Aberto":"Disponível"}</span>
                 </div>
                 <h3>{p.name}</h3>
                 {grp && <div className="group-info"><span className="group-dot" style={{background:grp.color}}/>{grp.name}</div>}
                 {p.description && <p className="desc">{p.description}</p>}
                 {p.tags?.length > 0 && <div className="tags">{p.tags.map((t,i)=><span key={i} className="tag">{t}</span>)}</div>}
                 {p.credentials?.email && <div className="cred-info">📧 {p.credentials.email}</div>}
                 {p.proxy && <div className="proxy-info">🌐 {p.proxy.type}://{p.proxy.host}:{p.proxy.port}</div>}
                 <div className="card-actions">
                   {p.status==="available" ? <button className="btn primary small" onClick={()=>handleOpen(p.id)}>▶ Abrir</button> :
                    p.status==="in_use" ? <button className="btn danger small" onClick={()=>handleClose(p.id)}>■ Fechar</button> : null}
                   <button className="btn small" onClick={()=>openProxyModal(p)}>🌐</button>
                   <button className="btn small" onClick={()=>openExtensionsModal(p)}>🧩</button>
                   <button className="btn small" onClick={()=>{setCookieProfileId(p.id);setCookieData("");setCookieCount(0);setCookieImportText("");setCookieError("");setCookieSuccess("");}}>🍪</button>
                   <button className="btn small" onClick={()=>openEditForm(p)}>✎</button>
                   <button className="btn small danger" onClick={()=>handleDelete(p.id)}>✕</button>
                 </div>
               </div>
             )})}
           </div>}
        </div>
      </div>

      {/* Group Form Modal */}
      {showGroupForm && (
        <div className="modal-overlay" onClick={()=>{setShowGroupForm(false);setEditGroupId(null);}}>
          <div className="modal" onClick={e=>e.stopPropagation()}>
            <h2>{editGroupId?"Editar":"Novo"} Grupo</h2>
            <form onSubmit={e=>{e.preventDefault();handleSaveGroup();}}>
              <input placeholder="Nome do grupo" value={groupName} onChange={e=>setGroupName(e.target.value)} required/>
              <div className="color-picker">{COLORS.map(c=><button key={c} type="button" className={`color-btn ${groupColor===c?"sel":""}`} style={{background:c}} onClick={()=>setGroupColor(c)}/>)}</div>
              <div className="modal-actions">
                <button type="button" className="btn" onClick={()=>{setShowGroupForm(false);setEditGroupId(null);}}>Cancelar</button>
                <button type="submit" className="btn primary">{editGroupId?"Salvar":"Criar"}</button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Group Manager Modal */}
      {showGroupManager && (
        <div className="modal-overlay" onClick={()=>setShowGroupManager(false)}>
          <div className="modal" onClick={e=>e.stopPropagation()} style={{maxWidth:"460px"}}>
            <h2>Gerenciar Grupos</h2>
            {groups.length === 0 && <p className="modal-hint">Nenhum grupo criado ainda.</p>}
            {[...groups].sort((a,b) => a.sortOrder - b.sortOrder).map(g => (
              <div key={g.id} style={{display:"flex",alignItems:"center",gap:"0.5rem",padding:"0.5rem 0",borderBottom:"1px solid var(--border)"}}>
                <span className="sidebar-group-dot" style={{background:g.color,flexShrink:0}}/>
                <span style={{flex:1}}>{g.name}</span>
                <span style={{fontSize:"0.75rem",color:"var(--muted)"}}>{profiles.filter(p => p.groupId === g.id).length} perfis</span>
                <button className="btn small" onClick={()=>openEditGroup(g)}>✎</button>
                <button className="btn small danger" onClick={()=>handleDeleteGroup(g.id)}>✕</button>
              </div>
            ))}
            <div style={{marginTop:"1rem"}}>
              <button className="btn primary" onClick={openNewGroup}>+ Novo Grupo</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
