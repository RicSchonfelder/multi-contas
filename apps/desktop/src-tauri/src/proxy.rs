use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub id: String,
    pub name: String,
    pub proxy_type: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password_encrypted: Option<String>,
    pub pac_url: Option<String>,
    pub bypass_list: Vec<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTestResult {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub ip: Option<String>,
}

pub struct ProxyManager {
    store_path: PathBuf,
}

impl ProxyManager {
    pub fn new() -> Self {
        let app_dir = Self::get_proxy_dir();
        let _ = fs::create_dir_all(&app_dir);
        Self {
            store_path: app_dir.join("proxies.json"),
        }
    }

    fn get_proxy_dir() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
        });
        PathBuf::from(local_app_data).join("BrowserWorkspace").join("proxies")
    }

    fn load(&self) -> io::Result<HashMap<String, ProxyConfig>> {
        if !self.store_path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(&self.store_path)?;
        let proxies: HashMap<String, ProxyConfig> = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(proxies)
    }

    fn save(&self, proxies: &HashMap<String, ProxyConfig>) -> io::Result<()> {
        let temp_path = self.store_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(proxies)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut file = File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp_path, &self.store_path)?;
        Ok(())
    }

    pub fn create(&self, name: String, proxy_type: String, host: String, port: u16, username: Option<String>, password: Option<String>, pac_url: Option<String>, bypass_list: Vec<String>, is_default: bool) -> Result<ProxyConfig, String> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let config = ProxyConfig {
            id: id.clone(),
            name,
            proxy_type,
            host,
            port,
            username,
            password_encrypted: password,
            pac_url,
            bypass_list,
            is_default,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut proxies = self.load().map_err(|e| e.to_string())?;
        if is_default {
            for p in proxies.values_mut() {
                p.is_default = false;
            }
        }
        proxies.insert(id, config.clone());
        self.save(&proxies).map_err(|e| e.to_string())?;
        Ok(config)
    }

    pub fn list(&self) -> Vec<ProxyConfig> {
        self.load().unwrap_or_default().values().cloned().collect()
    }

    pub fn get(&self, id: &str) -> Option<ProxyConfig> {
        self.load().ok()?.get(id).cloned()
    }

    pub fn update(&self, id: &str, name: Option<String>, host: Option<String>, port: Option<u16>, username: Option<Option<String>>, password: Option<Option<String>>, pac_url: Option<Option<String>>, bypass_list: Option<Vec<String>>, is_default: Option<bool>) -> Result<ProxyConfig, String> {
        let mut proxies = self.load().map_err(|e| e.to_string())?;
        let should_make_default = is_default.unwrap_or(false);

        if should_make_default {
            for p in proxies.values_mut() {
                p.is_default = false;
            }
        }

        let config = proxies.get_mut(id).ok_or("Proxy not found")?;
        if let Some(n) = name { config.name = n; }
        if let Some(h) = host { config.host = h; }
        if let Some(p) = port { config.port = p; }
        if let Some(u) = username { config.username = u; }
        if let Some(p) = password { config.password_encrypted = p; }
        if let Some(p) = pac_url { config.pac_url = p; }
        if let Some(b) = bypass_list { config.bypass_list = b; }
        config.is_default = should_make_default;
        config.updated_at = chrono::Utc::now().to_rfc3339();
        let result = config.clone();
        self.save(&proxies).map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut proxies = self.load().map_err(|e| e.to_string())?;
        proxies.remove(id);
        self.save(&proxies).map_err(|e| e.to_string())
    }

    pub fn test_connection(&self, id: &str) -> Result<ProxyTestResult, String> {
        let config = self.get(id).ok_or("Proxy not found")?;
        let proxy_url = format!("{}://{}:{}", config.proxy_type, config.host, config.port);
        let client = reqwest::blocking::Client::builder()
            .proxy(reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        let start = std::time::Instant::now();
        match client.get("https://httpbin.org/ip").send() {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                let ip = resp.text().ok();
                Ok(ProxyTestResult { success: true, latency_ms: Some(latency), error: None, ip })
            }
            Err(e) => Ok(ProxyTestResult { success: false, latency_ms: None, error: Some(e.to_string()), ip: None })
        }
    }

    pub fn set_default(&self, id: &str) -> Result<(), String> {
        let mut proxies = self.load().map_err(|e| e.to_string())?;
        for p in proxies.values_mut() {
            p.is_default = p.id == id;
        }
        self.save(&proxies).map_err(|e| e.to_string())
    }
}
