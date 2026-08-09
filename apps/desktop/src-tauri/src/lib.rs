pub mod cdp;
pub mod hermes;
pub mod profile;
pub mod registry;

use serde_json::{json, Value};
use std::sync::Arc;

use profile::allocate_free_port;
use profile::{
    Credentials, Extension, FingerprintConfig, Profile, ProfileGroup, ProfileManager, ProxyConfig,
};
use registry::ProfileRegistry;

pub struct AppState {
    manager: ProfileManager,
    registry: Arc<ProfileRegistry>,
}

#[tauri::command]
fn list_profiles(state: tauri::State<Arc<AppState>>) -> Vec<Profile> {
    state.manager.list()
}

#[tauri::command]
fn create_profile(
    state: tauri::State<Arc<AppState>>,
    name: String,
    color: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
    credentials: Option<Credentials>,
    fingerprint: Option<FingerprintConfig>,
) -> Result<Profile, String> {
    state
        .manager
        .create(
            name,
            color,
            description.unwrap_or_default(),
            tags.unwrap_or_default(),
            credentials,
            fingerprint,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_profile(
    state: tauri::State<Arc<AppState>>,
    id: String,
    name: Option<String>,
    color: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Profile, String> {
    state
        .manager
        .update(&id, name, color, description, tags)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_profile(state: tauri::State<Arc<AppState>>, id: String) -> Result<(), String> {
    state.manager.delete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_proxy(
    state: tauri::State<Arc<AppState>>,
    id: String,
    proxy: Option<ProxyConfig>,
) -> Result<Profile, String> {
    state
        .manager
        .set_proxy(&id, proxy)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_credentials(
    state: tauri::State<Arc<AppState>>,
    id: String,
    credentials: Option<Credentials>,
) -> Result<Profile, String> {
    state
        .manager
        .set_credentials(&id, credentials)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_fingerprint(
    state: tauri::State<Arc<AppState>>,
    id: String,
    fingerprint: Option<FingerprintConfig>,
) -> Result<Profile, String> {
    state
        .manager
        .set_fingerprint(&id, fingerprint)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_profile(state: tauri::State<Arc<AppState>>, id: String) -> Result<u16, String> {
    let registry = state.registry.clone();
    state.manager.open(&id, registry).map_err(|e| e.to_string())
}

#[tauri::command]
fn close_profile(state: tauri::State<Arc<AppState>>, id: String) -> Result<(), String> {
    state.manager.close(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_profile_cookies(state: tauri::State<Arc<AppState>>, id: String) -> Result<String, String> {
    let port = state
        .registry
        .get_port(&id)
        .ok_or("Perfil não está aberto (sem porta CDP)".to_string())?;
    state
        .manager
        .export_cookies(&id, port)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn import_profile_cookies(
    state: tauri::State<Arc<AppState>>,
    id: String,
    cookies_json: String,
) -> Result<(), String> {
    let port = state
        .registry
        .get_port(&id)
        .ok_or("Perfil não está aberto (sem porta CDP)".to_string())?;
    state
        .manager
        .import_cookies(&id, port, &cookies_json)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_profile_extension(
    state: tauri::State<Arc<AppState>>,
    id: String,
    ext_id: String,
    name: String,
    description: String,
    version: String,
    install_url: String,
) -> Result<Profile, String> {
    let ext = Extension {
        id: ext_id,
        name,
        description,
        version,
        enabled: true,
        install_url,
    };
    state
        .manager
        .add_extension(&id, ext)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_profile_extension(
    state: tauri::State<Arc<AppState>>,
    id: String,
    ext_id: String,
) -> Result<Profile, String> {
    state
        .manager
        .remove_extension(&id, &ext_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_profile_extension(
    state: tauri::State<Arc<AppState>>,
    id: String,
    ext_id: String,
    enabled: bool,
) -> Result<Profile, String> {
    state
        .manager
        .toggle_extension(&id, &ext_id, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_chrome_path() -> String {
    profile::resolve_chrome()
}

#[tauri::command]
fn get_control_info() -> Value {
    let dir = profile::get_app_dir();
    let token_path = dir.join("control_token");
    let token_available = token_path.exists();
    json!({ "control_port": 29222, "token_available": token_available })
}

#[tauri::command]
fn list_groups(state: tauri::State<Arc<AppState>>) -> Vec<ProfileGroup> {
    state.manager.list_groups()
}

#[tauri::command]
fn create_group(
    state: tauri::State<Arc<AppState>>,
    name: String,
    color: String,
) -> Result<ProfileGroup, String> {
    state
        .manager
        .create_group(name, color)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_group(
    state: tauri::State<Arc<AppState>>,
    id: String,
    name: String,
    color: String,
) -> Result<ProfileGroup, String> {
    state
        .manager
        .update_group(&id, name, color)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_group(state: tauri::State<Arc<AppState>>, id: String) -> Result<(), String> {
    state.manager.delete_group(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_group(
    state: tauri::State<Arc<AppState>>,
    profile_id: String,
    group_id: Option<String>,
) -> Result<Profile, String> {
    state
        .manager
        .set_profile_group(&profile_id, group_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn allocate_port() -> Option<u16> {
    allocate_free_port().map(|(p, _listener)| p)
}

#[tauri::command]
fn export_profiles_csv(state: tauri::State<Arc<AppState>>) -> Result<String, String> {
    let profiles = state.manager.list();
    let mut csv = String::from(
        "name;color;description;tags;email;password;url;group\n",
    );
    let groups = state.manager.list_groups();
    for p in &profiles {
        let group_name = p
            .group_id
            .as_ref()
            .and_then(|gid| groups.iter().find(|g| g.id == *gid))
            .map(|g| g.name.clone())
            .unwrap_or_default();
        let tags = p.tags.join(",");
        let email = p
            .credentials
            .as_ref()
            .map(|c| c.email.clone())
            .unwrap_or_default();
        let password = p
            .credentials
            .as_ref()
            .map(|c| c.password.clone())
            .unwrap_or_default();
        let url = p
            .credentials
            .as_ref()
            .map(|c| c.url.clone())
            .unwrap_or_default();
        csv.push_str(&format!(
            "{};{};{};{};{};{};{};{}\n",
            escape_csv_field(&p.name),
            escape_csv_field(&p.color),
            escape_csv_field(&p.description),
            escape_csv_field(&tags),
            escape_csv_field(&email),
            escape_csv_field(&password),
            escape_csv_field(&url),
            escape_csv_field(&group_name),
        ));
    }
    Ok(csv)
}

fn escape_csv_field(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[tauri::command]
fn import_profiles_csv(
    state: tauri::State<Arc<AppState>>,
    csv_data: String,
) -> Result<usize, String> {
    let mut imported = 0usize;
    let groups = state.manager.list_groups();

    for line in csv_data.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let fields = parse_csv_line(trimmed);
        if fields.len() < 8 {
            continue;
        }
        let name = &fields[0];
        let color = &fields[1];
        let description = &fields[2];
        let tags_str = &fields[3];
        let email = &fields[4];
        let password = &fields[5];
        let url = &fields[6];
        let group_name = &fields[7];

        if name.is_empty() {
            continue;
        }

        let tags: Vec<String> = if tags_str.is_empty() {
            vec![]
        } else {
            tags_str.split(',').map(|s| s.trim().to_string()).collect()
        };

        let credentials = if !email.is_empty() {
            Some(Credentials {
                email: email.clone(),
                password: password.clone(),
                url: if url.is_empty() {
                    "https://accounts.google.com".to_string()
                } else {
                    url.clone()
                },
            })
        } else {
            None
        };

        let p = state
            .manager
            .create(
                name.clone(),
                if color.is_empty() { "#6366F1".to_string() } else { color.clone() },
                description.clone(),
                tags,
                credentials,
                None,
            )
            .map_err(|e| format!("Erro ao criar perfil '{}': {}", name, e))?;

        // Assign to group if specified
        if !group_name.is_empty() {
            let group_id = groups
                .iter()
                .find(|g| g.name == *group_name)
                .map(|g| g.id.clone())
                .or_else(|| {
                    state
                        .manager
                        .create_group(group_name.clone(), "#6366F1".to_string())
                        .ok()
                        .map(|g| g.id)
                });
            if let Some(gid) = group_id {
                let _ = state.manager.set_profile_group(&p.id, Some(gid));
            }
        }

        imported += 1;
    }

    Ok(imported)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_quotes => in_quotes = true,
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            ';' if !in_quotes => {
                fields.push(current.clone());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = ProfileManager::new();
    manager.recover();

    let app_state = Arc::new(AppState {
        manager,
        registry: Arc::new(ProfileRegistry::new()),
    });

    // Sobe o servidor de controle Hermes (localhost + token) em thread separada
    hermes::start(app_state.clone());

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            create_profile,
            update_profile,
            delete_profile,
            set_profile_proxy,
            set_profile_credentials,
            set_profile_fingerprint,
            open_profile,
            close_profile,
            get_chrome_path,
            get_control_info,
            add_profile_extension,
            remove_profile_extension,
            toggle_profile_extension,
            export_profile_cookies,
            import_profile_cookies,
            list_groups,
            create_group,
            update_group,
            delete_group,
            set_profile_group,
            allocate_port,
            export_profiles_csv,
            import_profiles_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
