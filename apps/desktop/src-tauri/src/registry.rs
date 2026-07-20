use chrono::Utc;
use std::collections::HashMap;
use std::sync::Mutex;

/// Estado de um perfil aberto (Chrome rodando) controlado via CDP.
#[derive(Clone, Debug)]
pub struct ActiveProfile {
    pub port: u16,
    pub ws_url: Option<String>,
    pub pid: Option<u32>,
    pub opened_at: String,
}

/// Registro em memória de todos os perfis ativos.
/// Compartilhado entre o app Tauri e o servidor de controle Hermes.
pub struct ProfileRegistry {
    map: Mutex<HashMap<String, ActiveProfile>>,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        ProfileRegistry {
            map: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, id: &str, port: u16, pid: Option<u32>) {
        let mut m = self.map.lock().unwrap();
        m.insert(
            id.to_string(),
            ActiveProfile {
                port,
                ws_url: None,
                pid,
                opened_at: Utc::now().to_rfc3339(),
            },
        );
    }

    pub fn set_ws_url(&self, id: &str, ws_url: &str) {
        let mut m = self.map.lock().unwrap();
        if let Some(p) = m.get_mut(id) {
            p.ws_url = Some(ws_url.to_string());
        }
    }

    pub fn get(&self, id: &str) -> Option<ActiveProfile> {
        self.map.lock().unwrap().get(id).cloned()
    }

    pub fn get_port(&self, id: &str) -> Option<u16> {
        self.map.lock().unwrap().get(id).map(|p| p.port)
    }

    pub fn get_ws_url(&self, id: &str) -> Option<String> {
        self.map
            .lock()
            .unwrap()
            .get(id)
            .and_then(|p| p.ws_url.clone())
    }

    pub fn remove(&self, id: &str) {
        self.map.lock().unwrap().remove(id);
    }

    /// (id, port) de todos os perfis ativos.
    pub fn list(&self) -> Vec<(String, u16)> {
        self.map
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.port))
            .collect()
    }

    pub fn ids(&self) -> Vec<String> {
        self.map.lock().unwrap().keys().cloned().collect()
    }
}
