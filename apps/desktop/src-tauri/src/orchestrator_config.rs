use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorConfig {
    #[serde(default = "default_max_active_profiles")]
    pub max_active_profiles: usize,
    #[serde(default = "default_max_xfce_sessions")]
    pub max_xfce_sessions: usize,
    #[serde(default = "default_min_free_ram_mb")]
    pub min_free_ram_mb: u64,
    #[serde(default = "default_max_load_pct")]
    pub max_load_pct: f64,
    #[serde(default = "default_max_chrome_procs")]
    pub max_chrome_procs: usize,
    #[serde(default = "default_queue_enabled")]
    pub queue_enabled: bool,
}

fn default_max_active_profiles() -> usize {
    3
}
fn default_max_xfce_sessions() -> usize {
    2
}
fn default_min_free_ram_mb() -> u64 {
    512
}
fn default_max_load_pct() -> f64 {
    80.0
}
fn default_max_chrome_procs() -> usize {
    30
}
fn default_queue_enabled() -> bool {
    true
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_active_profiles: 3,
            max_xfce_sessions: 2,
            min_free_ram_mb: 512,
            max_load_pct: 80.0,
            max_chrome_procs: 30,
            queue_enabled: true,
        }
    }
}

impl OrchestratorConfig {
    pub fn config_path() -> std::path::PathBuf {
        crate::profile::get_app_dir().join("orchestrator.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let conf = OrchestratorConfig::default();
        assert_eq!(conf.max_active_profiles, 3);
        assert_eq!(conf.max_xfce_sessions, 2);
        assert_eq!(conf.min_free_ram_mb, 512);
        assert!((conf.max_load_pct - 80.0).abs() < f64::EPSILON);
        assert_eq!(conf.max_chrome_procs, 30);
        assert!(conf.queue_enabled);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let conf = OrchestratorConfig::default();
        let json = serde_json::to_string(&conf).unwrap();
        let parsed: OrchestratorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_active_profiles, conf.max_active_profiles);
        assert_eq!(parsed.queue_enabled, conf.queue_enabled);
    }

    #[test]
    fn test_partial_deserialization() {
        let json = r#"{"maxActiveProfiles": 5}"#;
        let conf: OrchestratorConfig = serde_json::from_str(json).unwrap();
        assert_eq!(conf.max_active_profiles, 5);
        assert_eq!(conf.min_free_ram_mb, 512); // default
    }
}
