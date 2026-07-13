use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub url: String,
    pub folder_id: Option<String>,
    pub sort_order: u32,
    pub icon: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkFolder {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkSet {
    pub id: String,
    pub profile_id: String,
    pub name: String,
    pub bookmark_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
struct BookmarkStore {
    bookmarks: HashMap<String, Bookmark>,
    folders: HashMap<String, BookmarkFolder>,
    sets: HashMap<String, BookmarkSet>,
}

pub struct BookmarkManager {
    store_path: PathBuf,
}

impl BookmarkManager {
    pub fn new() -> Self {
        let app_dir = Self::get_bm_dir();
        let _ = fs::create_dir_all(&app_dir);
        Self {
            store_path: app_dir.join("bookmarks.json"),
        }
    }

    fn get_bm_dir() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
        });
        PathBuf::from(local_app_data).join("BrowserWorkspace").join("bookmarks")
    }

    fn load(&self) -> BookmarkStore {
        if !self.store_path.exists() {
            return BookmarkStore { bookmarks: HashMap::new(), folders: HashMap::new(), sets: HashMap::new() };
        }
        fs::read_to_string(&self.store_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or(BookmarkStore { bookmarks: HashMap::new(), folders: HashMap::new(), sets: HashMap::new() })
    }

    fn save(&self, store: &BookmarkStore) -> io::Result<()> {
        let temp = self.store_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(store).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut file = File::create(&temp)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp, &self.store_path)?;
        Ok(())
    }

    pub fn list(&self, profile_id: &str, folder_id: Option<String>) -> Vec<Bookmark> {
        let store = self.load();
        store.bookmarks.values().filter(|b| {
            b.profile_id == profile_id && (folder_id.is_none() || b.folder_id == folder_id)
        }).cloned().collect()
    }

    pub fn create(&self, profile_id: &str, name: &str, url: &str, folder_id: Option<String>) -> Result<Bookmark, String> {
        let mut store = self.load();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let bm = Bookmark {
            id: id.clone(),
            profile_id: profile_id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            folder_id,
            sort_order: 0,
            icon: None,
            created_at: now.clone(),
            updated_at: now,
        };
        store.bookmarks.insert(id, bm.clone());
        self.save(&store).map_err(|e| e.to_string())?;
        Ok(bm)
    }

    pub fn update(&self, id: &str, name: Option<String>, url: Option<String>, folder_id: Option<Option<String>>, sort_order: Option<u32>) -> Result<Bookmark, String> {
        let mut store = self.load();
        let bm = store.bookmarks.get_mut(id).ok_or("Bookmark not found")?;
        if let Some(n) = name { bm.name = n; }
        if let Some(u) = url { bm.url = u; }
        if let Some(f) = folder_id { bm.folder_id = f; }
        if let Some(s) = sort_order { bm.sort_order = s; }
        bm.updated_at = chrono::Utc::now().to_rfc3339();
        let result = bm.clone();
        self.save(&store).map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut store = self.load();
        store.bookmarks.remove(id);
        self.save(&store).map_err(|e| e.to_string())
    }

    pub fn create_folder(&self, profile_id: &str, name: &str, parent_id: Option<String>) -> Result<BookmarkFolder, String> {
        let mut store = self.load();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let folder = BookmarkFolder {
            id: id.clone(),
            profile_id: profile_id.to_string(),
            name: name.to_string(),
            parent_id,
            created_at: now,
        };
        store.folders.insert(id, folder.clone());
        self.save(&store).map_err(|e| e.to_string())?;
        Ok(folder)
    }

    pub fn list_folders(&self, profile_id: &str) -> Vec<BookmarkFolder> {
        let store = self.load();
        store.folders.values().filter(|f| f.profile_id == profile_id).cloned().collect()
    }

    pub fn create_set(&self, profile_id: &str, name: &str, bookmark_ids: Vec<String>) -> Result<BookmarkSet, String> {
        let mut store = self.load();
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let set = BookmarkSet {
            id: id.clone(),
            profile_id: profile_id.to_string(),
            name: name.to_string(),
            bookmark_ids,
            created_at: now,
        };
        store.sets.insert(id, set.clone());
        self.save(&store).map_err(|e| e.to_string())?;
        Ok(set)
    }

    pub fn list_sets(&self, profile_id: &str) -> Vec<BookmarkSet> {
        let store = self.load();
        store.sets.values().filter(|s| s.profile_id == profile_id).cloned().collect()
    }

    pub fn apply_set(&self, profile_id: &str, set_id: &str) -> Result<(), String> {
        let store = self.load();
        let set = store.sets.get(set_id).ok_or("Set not found")?.clone();
        for bm_id in &set.bookmark_ids {
            if let Some(bm) = store.bookmarks.get(bm_id) {
                if bm.profile_id != profile_id {
                    // Copy bookmark to profile
                    let new_id = Uuid::new_v4().to_string();
                    let mut new_bm = bm.clone();
                    new_bm.id = new_id;
                    new_bm.profile_id = profile_id.to_string();
                    // Need mutable access
                }
            }
        }
        Ok(())
    }

    pub fn delete_set(&self, id: &str) -> Result<(), String> {
        let mut store = self.load();
        store.sets.remove(id);
        self.save(&store).map_err(|e| e.to_string())
    }
}
