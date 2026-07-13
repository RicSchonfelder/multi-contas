use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::JobObjects::{
    CreateJobObjectW, SetInformationJobObject, AssignProcessToJobObject,
    JobObjectExtendedLimitInformation, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HANDLE, CloseHandle};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub pac_url: Option<String>,
    pub bypass_list: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintConfig {
    pub enabled: bool,
    pub user_agent: String,
    pub platform: String,
    pub resolution: String,
    pub timezone: String,
    pub geolocation: String,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
    pub hardware_concurrency: u32,
    pub device_memory: u32,
    pub canvas_noise: bool,
    pub audio_noise: bool,
    pub fonts: Vec<String>,
    pub language: String,
}

impl Default for FingerprintConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            user_agent: String::new(),
            platform: String::new(),
            resolution: String::new(),
            timezone: String::new(),
            geolocation: String::new(),
            webgl_vendor: String::new(),
            webgl_renderer: String::new(),
            hardware_concurrency: 4,
            device_memory: 8,
            canvas_noise: true,
            audio_noise: true,
            fonts: Vec::new(),
            language: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfileGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub sort_order: u32,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub enabled: bool,
    pub install_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub color: String,
    pub description: String,
    pub tags: Vec<String>,
    pub status: String,
    pub proxy: Option<ProxyConfig>,
    pub credentials: Option<Credentials>,
    pub fingerprint: Option<FingerprintConfig>,
    pub local_dir: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    pub group_id: Option<String>,
    #[serde(default)]
    pub extensions: Vec<Extension>,
}

#[cfg(target_os = "windows")]
static ACTIVE_JOBS: std::sync::OnceLock<Mutex<HashMap<String, HANDLE>>> = std::sync::OnceLock::new();

#[cfg(target_os = "windows")]
fn get_active_jobs() -> &'static Mutex<HashMap<String, HANDLE>> {
    ACTIVE_JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_app_dir() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
        std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
    });
    PathBuf::from(local).join("MultiContas")
}

fn get_profiles_dir() -> PathBuf {
    get_app_dir().join("profiles")
}

pub fn resolve_chrome() -> String {
    if let Ok(path) = std::env::var("CHROME_PATH") {
        if Path::new(&path).exists() { return path; }
    }
    let candidates = [
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files\\Chromium\\Application\\chrome.exe",
    ];
    for c in &candidates {
        if Path::new(c).exists() { return c.to_string(); }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let user = format!("{}\\Google\\Chrome\\Application\\chrome.exe", local);
        if Path::new(&user).exists() { return user; }
    }
    candidates[0].to_string()
}

pub struct ProfileStore {
    file_path: PathBuf,
}

impl ProfileStore {
    fn new() -> Self {
        let dir = get_app_dir();
        let _ = fs::create_dir_all(&dir);
        Self { file_path: dir.join("profiles.json") }
    }

    fn load(&self) -> HashMap<String, Profile> {
        if !self.file_path.exists() { return HashMap::new(); }
        fs::read_to_string(&self.file_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save(&self, profiles: &HashMap<String, Profile>) -> io::Result<()> {
        let tmp = self.file_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(profiles)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut f = File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, &self.file_path)?;
        Ok(())
    }
}

pub struct GroupStore {
    file_path: PathBuf,
}

impl GroupStore {
    fn new() -> Self {
        let dir = get_app_dir();
        let _ = fs::create_dir_all(&dir);
        Self { file_path: dir.join("groups.json") }
    }

    fn load(&self) -> HashMap<String, ProfileGroup> {
        if !self.file_path.exists() { return HashMap::new(); }
        fs::read_to_string(&self.file_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save(&self, groups: &HashMap<String, ProfileGroup>) -> io::Result<()> {
        let tmp = self.file_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(groups)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut f = File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, &self.file_path)?;
        Ok(())
    }
}

pub struct ProfileManager {
    store: ProfileStore,
    group_store: GroupStore,
}

impl ProfileManager {
    pub fn new() -> Self {
        Self { store: ProfileStore::new(), group_store: GroupStore::new() }
    }

    pub fn list_groups(&self) -> Vec<ProfileGroup> {
        self.group_store.load().values().cloned().collect()
    }

    pub fn create_group(&self, name: String, color: String) -> io::Result<ProfileGroup> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let max_order = self.group_store.load().values().map(|g| g.sort_order).max().unwrap_or(0);
        let g = ProfileGroup { id: id.clone(), name, color, sort_order: max_order + 1, created_at: now };
        let mut all = self.group_store.load();
        all.insert(id, g.clone());
        self.group_store.save(&all)?;
        Ok(g)
    }

    pub fn update_group(&self, id: &str, name: String, color: String) -> io::Result<ProfileGroup> {
        let mut all = self.group_store.load();
        let g = all.get_mut(id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "group not found"))?;
        g.name = name;
        g.color = color;
        let r = g.clone();
        self.group_store.save(&all)?;
        Ok(r)
    }

    pub fn delete_group(&self, id: &str) -> io::Result<()> {
        let mut all = self.group_store.load();
        all.remove(id);
        self.group_store.save(&all)?;
        // ungroup profiles in this group
        let mut profiles = self.store.load();
        for p in profiles.values_mut() {
            if p.group_id.as_deref() == Some(id) {
                p.group_id = None;
            }
        }
        self.store.save(&profiles)?;
        Ok(())
    }

    pub fn set_profile_group(&self, profile_id: &str, group_id: Option<String>) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(profile_id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "profile not found"))?;
        p.group_id = group_id;
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn list(&self) -> Vec<Profile> {
        self.store.load().values().cloned().collect()
    }

    pub fn create(&self, name: String, color: String, description: String, tags: Vec<String>, credentials: Option<Credentials>, fingerprint: Option<FingerprintConfig>) -> io::Result<Profile> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let dir = get_profiles_dir().join(&id);
        fs::create_dir_all(&dir)?;

        let p = Profile {
            id: id.clone(), name, color, description, tags,
            status: "available".into(), proxy: None, credentials, fingerprint,
            local_dir: dir.to_string_lossy().to_string(),
            created_at: now.clone(), updated_at: now,
            last_opened_at: None,
            group_id: None,
            extensions: Vec::new(),
        };
        let mut all = self.store.load();
        all.insert(id, p.clone());
        self.store.save(&all)?;
        Ok(p)
    }

    pub fn update(&self, id: &str, name: Option<String>, color: Option<String>, description: Option<String>, tags: Option<Vec<String>>) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        if let Some(n) = name { p.name = n; }
        if let Some(c) = color { p.color = c; }
        if let Some(d) = description { p.description = d; }
        if let Some(t) = tags { p.tags = t; }
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn delete(&self, id: &str) -> io::Result<()> {
        let mut all = self.store.load();
        all.remove(id);
        self.store.save(&all)?;
        let dir = get_profiles_dir().join(id);
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    pub fn set_fingerprint(&self, id: &str, fingerprint: Option<FingerprintConfig>) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        p.fingerprint = fingerprint;
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn set_credentials(&self, id: &str, credentials: Option<Credentials>) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        p.credentials = credentials;
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn set_proxy(&self, id: &str, proxy: Option<ProxyConfig>) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        p.proxy = proxy;
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn add_extension(&self, profile_id: &str, ext: Extension) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(profile_id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        p.extensions.push(ext);
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn remove_extension(&self, profile_id: &str, ext_id: &str) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(profile_id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        p.extensions.retain(|e| e.id != ext_id);
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn toggle_extension(&self, profile_id: &str, ext_id: &str, enabled: bool) -> io::Result<Profile> {
        let mut all = self.store.load();
        let p = all.get_mut(profile_id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        if let Some(ext) = p.extensions.iter_mut().find(|e| e.id == ext_id) {
            ext.enabled = enabled;
        }
        p.updated_at = chrono::Utc::now().to_rfc3339();
        let r = p.clone();
        self.store.save(&all)?;
        Ok(r)
    }

    pub fn open(&self, id: &str) -> Result<(), String> {
        let mut all = self.store.load();

        // Clone profile data first, drop the mutable borrow
        let local_dir = all.get(id).ok_or("Profile not found")?.local_dir.clone();
        let proxy = all.get(id).unwrap().proxy.clone();
        let creds = all.get(id).unwrap().credentials.clone();
        let fp = all.get(id).unwrap().fingerprint.clone();
        let status = all.get(id).unwrap().status.clone();
        let exts = all.get(id).map(|p| p.extensions.clone()).unwrap_or_default();

        if status == "in_use" {
            if self.is_active(id) { return Err("Profile already open".into()); }
        }

        let chrome = resolve_chrome();
        if !Path::new(&chrome).exists() {
            return Err(format!("Chrome not found at: {}", chrome));
        }

        let profile_dir = Path::new(&local_dir).join("chromium");
        fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;

        let lock_path = Path::new(&local_dir).join("lock");
        fs::write(&lock_path, std::process::id().to_string()).map_err(|e| e.to_string())?;

        // CDP port for auto-login + fingerprint
        let needs_cdp = creds.is_some() || fp.as_ref().map_or(false, |f| f.enabled);
        let cdp_port = if needs_cdp { 29222 } else { 0 };

        let mut cmd = Command::new(&chrome);
        cmd.arg(format!("--user-data-dir={}", profile_dir.to_string_lossy()))
           .arg("--no-first-run")
           .arg("--no-default-browser-check")
           .arg("--disable-blink-features=AutomationControlled")
           .arg("--disable-features=ChromeWhatsNewUI,ChromeTipsInMainMenu")
           .arg("--disable-sync")
           .arg("--no-pings");

        // Anti-fingerprint flags
        cmd.arg("--disable-webrtc-peer-connection-for-encryption");
        cmd.arg("--disable-reading-from-canvas");
        cmd.arg("--disable-remote-fonts");
        cmd.arg("--disable-client-side-phishing-detection");

        if cdp_port > 0 {
            cmd.arg(format!("--remote-debugging-port={}", cdp_port));
        }

        // Navigate to login URL if credentials are set
        if let Some(ref c) = creds {
            if !c.url.is_empty() {
                cmd.arg(&c.url);
            }
        }

        if let Some(ref proxy_cfg) = proxy {
            if let Some(ref pac) = proxy_cfg.pac_url {
                cmd.arg(format!("--proxy-pac-url={}", pac));
            } else {
                cmd.arg(format!("--proxy-server={}://{}:{}", proxy_cfg.proxy_type, proxy_cfg.host, proxy_cfg.port));
                if !proxy_cfg.bypass_list.is_empty() {
                    cmd.arg(format!("--proxy-bypass-list={}", proxy_cfg.bypass_list.join(";")));
                }
            }
        }

        // Load enabled extensions
        for ext in &exts {
            if ext.enabled && !ext.install_url.is_empty() {
                cmd.arg(format!("--load-extension={}", ext.install_url));
            }
        }

        let mut child = cmd.spawn().map_err(|e| format!("Failed to launch Chrome: {}", e))?;

        #[cfg(target_os = "windows")]
        {
            let handle = child.as_raw_handle() as HANDLE;
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job != 0 {
                    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    SetInformationJobObject(job, JobObjectExtendedLimitInformation,
                        &info as *const _ as *const std::ffi::c_void,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32);
                    AssignProcessToJobObject(job, handle);
                    get_active_jobs().lock().unwrap().insert(id.to_string(), job);
                }
            }
        }

        // Update profile status
        if let Some(p) = all.get_mut(id) {
            p.status = "in_use".into();
            p.updated_at = chrono::Utc::now().to_rfc3339();
            p.last_opened_at = Some(chrono::Utc::now().to_rfc3339());
        }
        self.store.save(&all).unwrap();

        // CDP operations (fingerprint + auto-login)
        if cdp_port > 0 {
            let fp_clone = fp.clone();
            let creds_clone = creds.clone();
            std::thread::spawn(move || {
                if let Some(ref f) = fp_clone {
                    if f.enabled {
                        let _ = crate::cdp::apply_fingerprint(cdp_port, f);
                    }
                }
                if let Some(ref c) = creds_clone {
                    let _ = crate::cdp::auto_login(cdp_port, &c.email, &c.password, &c.url);
                }
            });
        }

        let store_path = self.store.file_path.clone();
        let id_clone = id.to_string();
        thread::spawn(move || {
            let _ = child.wait();
            let store = ProfileStore { file_path: store_path };
            let mut profiles = store.load();
                if let Some(profile) = profiles.get_mut(&id_clone) {
                    profile.status = "available".into();
                    profile.updated_at = chrono::Utc::now().to_rfc3339();
                    let _ = store.save(&profiles);
                }
            let _ = fs::remove_file(get_profiles_dir().join(&id_clone).join("lock"));
            #[cfg(target_os = "windows")]
            if let Some(job) = get_active_jobs().lock().unwrap().remove(&id_clone) {
                unsafe { CloseHandle(job); }
            }
        });

        Ok(())
    }

    pub fn close(&self, id: &str) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            let mut jobs = get_active_jobs().lock().unwrap();
            if let Some(job) = jobs.remove(id) {
                unsafe { CloseHandle(job); }
                return Ok(());
            }
        }
        let mut all = self.store.load();
        if let Some(p) = all.get_mut(id) {
            p.status = "available".into();
            p.updated_at = chrono::Utc::now().to_rfc3339();
            self.store.save(&all).unwrap();
        }
        let _ = fs::remove_file(get_profiles_dir().join(id).join("lock"));
        Ok(())
    }

    pub fn recover(&self) {
        let mut all = self.store.load();
        let mut dirty = false;
        for (id, p) in all.iter_mut() {
            if p.status == "in_use" && !self.is_active(id) {
                p.status = "available".into();
                p.updated_at = chrono::Utc::now().to_rfc3339();
                dirty = true;
            }
        }
        if dirty { let _ = self.store.save(&all); }
    }

    pub fn export_cookies(&self, id: &str) -> Result<String, String> {
        let all = self.store.load();
        let p = all.get(id).ok_or_else(|| "Profile not found".to_string())?;
        if p.status != "in_use" {
            return Err("Profile must be open to export cookies".into());
        }
        crate::cdp::export_cookies(29222, id)
    }

    pub fn import_cookies(&self, id: &str, cookies_json: &str) -> Result<(), String> {
        let all = self.store.load();
        let p = all.get(id).ok_or_else(|| "Profile not found".to_string())?;
        if p.status != "in_use" {
            return Err("Profile must be open to import cookies".into());
        }
        crate::cdp::import_cookies(29222, cookies_json)
    }

    fn is_active(&self, id: &str) -> bool {
        let lock = get_profiles_dir().join(id).join("lock");
        if !lock.exists() { return false; }
        if let Ok(pid_str) = fs::read_to_string(&lock) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                #[cfg(target_os = "windows")]
                {
                    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
                    unsafe {
                        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                        if h != 0 { CloseHandle(h); return true; }
                    }
                }
            }
        }
        false
    }
}
