use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub description: String,
    pub install_url: String,
    pub is_required: bool,
    pub is_blocked: bool,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfileExtension {
    pub profile_id: String,
    pub extension_id: String,
    pub extension_name: String,
    pub enabled: bool,
    pub installed_at: String,
}

pub struct ExtensionManager {
    catalog_path: PathBuf,
    profile_ext_path: PathBuf,
}

impl ExtensionManager {
    pub fn new() -> Self {
        let app_dir = Self::get_ext_dir();
        let _ = fs::create_dir_all(&app_dir);
        Self {
            catalog_path: app_dir.join("catalog.json"),
            profile_ext_path: app_dir.join("profile_extensions.json"),
        }
    }

    fn get_ext_dir() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
        });
        PathBuf::from(local_app_data).join("BrowserWorkspace").join("extensions")
    }

    fn load_catalog(&self) -> HashMap<String, Extension> {
        if !self.catalog_path.exists() { return HashMap::new(); }
        fs::read_to_string(&self.catalog_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save_catalog(&self, catalog: &HashMap<String, Extension>) -> io::Result<()> {
        let temp = self.catalog_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(catalog).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut file = File::create(&temp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp, &self.catalog_path)?;
        Ok(())
    }

    fn load_profile_exts(&self) -> Vec<ProfileExtension> {
        if !self.profile_ext_path.exists() { return Vec::new(); }
        fs::read_to_string(&self.profile_ext_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save_profile_exts(&self, exts: &[ProfileExtension]) -> io::Result<()> {
        let temp = self.profile_ext_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(exts).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut file = File::create(&temp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp, &self.profile_ext_path)?;
        Ok(())
    }

    pub fn list_catalog(&self) -> Vec<Extension> {
        self.load_catalog().values().cloned().collect()
    }

    pub fn add_to_catalog(&self, ext_id: String, name: String, description: String, install_url: String, is_required: bool, is_blocked: bool) -> Result<Extension, String> {
        let mut catalog = self.load_catalog();
        let now = chrono::Utc::now().to_rfc3339();
        let ext = Extension {
            id: ext_id.clone(),
            name,
            description,
            install_url,
            is_required,
            is_blocked,
            created_at: now,
        };
        catalog.insert(ext_id, ext.clone());
        self.save_catalog(&catalog).map_err(|e| e.to_string())?;
        Ok(ext)
    }

    pub fn remove_from_catalog(&self, id: &str) -> Result<(), String> {
        let mut catalog = self.load_catalog();
        catalog.remove(id);
        self.save_catalog(&catalog).map_err(|e| e.to_string())
    }

    pub fn list_profile_extensions(&self, profile_id: &str) -> Vec<ProfileExtension> {
        self.load_profile_exts().into_iter().filter(|e| e.profile_id == profile_id).collect()
    }

    pub fn set_extension_state(&self, profile_id: &str, extension_id: &str, enabled: bool) -> Result<(), String> {
        let catalog = self.load_catalog();
        let ext = catalog.get(extension_id).ok_or("Extension not in catalog")?;
        if ext.is_blocked && enabled {
            return Err("Extension is blocked".to_string());
        }

        let mut exts = self.load_profile_exts();
        if let Some(pe) = exts.iter_mut().find(|e| e.profile_id == profile_id && e.extension_id == extension_id) {
            pe.enabled = enabled;
        } else {
            exts.push(ProfileExtension {
                profile_id: profile_id.to_string(),
                extension_id: extension_id.to_string(),
                extension_name: ext.name.clone(),
                enabled,
                installed_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        self.save_profile_exts(&exts).map_err(|e| e.to_string())
    }
}
