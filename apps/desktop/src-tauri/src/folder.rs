use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFolder {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub color: Option<String>,
    pub sort_order: u32,
    pub created_at: String,
    pub updated_at: String,
}

pub struct FolderManager {
    store_path: PathBuf,
}

impl FolderManager {
    pub fn new() -> Self {
        let app_dir = Self::get_folder_dir();
        let _ = fs::create_dir_all(&app_dir);
        Self {
            store_path: app_dir.join("folders.json"),
        }
    }

    fn get_folder_dir() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
        });
        PathBuf::from(local_app_data).join("BrowserWorkspace").join("folders")
    }

    fn load(&self) -> HashMap<String, ProfileFolder> {
        if !self.store_path.exists() { return HashMap::new(); }
        fs::read_to_string(&self.store_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    fn save(&self, folders: &HashMap<String, ProfileFolder>) -> io::Result<()> {
        let temp = self.store_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(folders).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut file = File::create(&temp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp, &self.store_path)?;
        Ok(())
    }

    pub fn create(&self, name: &str, parent_id: Option<String>, color: Option<String>) -> Result<ProfileFolder, String> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let folder = ProfileFolder {
            id: id.clone(),
            name: name.to_string(),
            parent_id,
            color,
            sort_order: 0,
            created_at: now.clone(),
            updated_at: now,
        };
        let mut folders = self.load();
        folders.insert(id, folder.clone());
        self.save(&folders).map_err(|e| e.to_string())?;
        Ok(folder)
    }

    pub fn list_all(&self) -> Vec<ProfileFolder> {
        self.load().values().cloned().collect()
    }

    pub fn update(&self, id: &str, name: Option<String>, color: Option<String>) -> Result<ProfileFolder, String> {
        let mut folders = self.load();
        let folder = folders.get_mut(id).ok_or("Folder not found")?;
        if let Some(n) = name { folder.name = n; }
        if let Some(c) = color { folder.color = Some(c); }
        folder.updated_at = chrono::Utc::now().to_rfc3339();
        let result = folder.clone();
        self.save(&folders).map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut folders = self.load();
        folders.remove(id);
        self.save(&folders).map_err(|e| e.to_string())
    }
}
