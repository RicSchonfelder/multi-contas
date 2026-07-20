//! ProfileRegistry — estado vivo de perfis Chrome abertos (multi-conta).
//!
//! Substitui o controle "1 perfil por vez" por um mapa thread-safe de N
//! perfis ativos simultaneamente. Cada entrada guarda a porta CDP dinamica,
//! o PID do processo Chrome, o wsUrl do CDP e o instante de inicio.
//!
//! Os Windows Job Objects (ACTIVE_JOBS em profile.rs) continuam sendo a
//! primitiva de kill no Windows; o registry e complementar (metadados).

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub struct ActiveProfile {
    pub profile_id: String,
    pub cdp_port: u16,
    pub child_pid: u32,
    pub ws_url: Option<String>,
    pub started_at: String,
}

pub struct ProfileRegistry {
    active: Mutex<HashMap<String, ActiveProfile>>,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        Self { active: Mutex::new(HashMap::new()) }
    }

    pub fn register(&self, p: ActiveProfile) {
        self.active.lock().unwrap().insert(p.profile_id.clone(), p);
    }

    pub fn unregister(&self, profile_id: &str) {
        self.active.lock().unwrap().remove(profile_id);
    }

    pub fn get(&self, profile_id: &str) -> Option<ActiveProfile> {
        self.active.lock().unwrap().get(profile_id).cloned()
    }

    pub fn list_active(&self) -> Vec<ActiveProfile> {
        self.active.lock().unwrap().values().cloned().collect()
    }

    pub fn is_active(&self, profile_id: &str) -> bool {
        self.active.lock().unwrap().contains_key(profile_id)
    }

    pub fn set_ws_url(&self, profile_id: &str, ws_url: String) {
        if let Some(e) = self.active.lock().unwrap().get_mut(profile_id) {
            e.ws_url = Some(ws_url);
        }
    }

    pub fn set_cdp_port(&self, profile_id: &str, port: u16) {
        if let Some(e) = self.active.lock().unwrap().get_mut(profile_id) {
            e.cdp_port = port;
        }
    }
}
