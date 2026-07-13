pub mod profile;
pub mod cdp;

use profile::{Profile, ProfileGroup, ProfileManager, ProxyConfig, Credentials, FingerprintConfig, Extension};

struct AppState {
    manager: ProfileManager,
}

#[tauri::command]
fn list_profiles(state: tauri::State<AppState>) -> Vec<Profile> {
    state.manager.list()
}

#[tauri::command]
fn create_profile(state: tauri::State<AppState>, name: String, color: String, description: Option<String>, tags: Option<Vec<String>>, credentials: Option<Credentials>, fingerprint: Option<FingerprintConfig>) -> Result<Profile, String> {
    state.manager.create(name, color, description.unwrap_or_default(), tags.unwrap_or_default(), credentials, fingerprint).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_profile(state: tauri::State<AppState>, id: String, name: Option<String>, color: Option<String>, description: Option<String>, tags: Option<Vec<String>>) -> Result<Profile, String> {
    state.manager.update(&id, name, color, description, tags).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_profile(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.manager.delete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_proxy(state: tauri::State<AppState>, id: String, proxy: Option<ProxyConfig>) -> Result<Profile, String> {
    state.manager.set_proxy(&id, proxy).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_credentials(state: tauri::State<AppState>, id: String, credentials: Option<Credentials>) -> Result<Profile, String> {
    state.manager.set_credentials(&id, credentials).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_fingerprint(state: tauri::State<AppState>, id: String, fingerprint: Option<FingerprintConfig>) -> Result<Profile, String> {
    state.manager.set_fingerprint(&id, fingerprint).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_profile(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.manager.open(&id)
}

#[tauri::command]
fn close_profile(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.manager.close(&id)
}

#[tauri::command]
fn export_profile_cookies(state: tauri::State<AppState>, id: String) -> Result<String, String> {
    state.manager.export_cookies(&id)
}

#[tauri::command]
fn import_profile_cookies(state: tauri::State<AppState>, id: String, cookies_json: String) -> Result<(), String> {
    state.manager.import_cookies(&id, &cookies_json)
}

#[tauri::command]
fn add_profile_extension(state: tauri::State<AppState>, id: String, ext_id: String, name: String, description: String, version: String, install_url: String) -> Result<Profile, String> {
    let ext = Extension { id: ext_id, name, description, version, enabled: true, install_url };
    state.manager.add_extension(&id, ext).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_profile_extension(state: tauri::State<AppState>, id: String, ext_id: String) -> Result<Profile, String> {
    state.manager.remove_extension(&id, &ext_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_profile_extension(state: tauri::State<AppState>, id: String, ext_id: String, enabled: bool) -> Result<Profile, String> {
    state.manager.toggle_extension(&id, &ext_id, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_chrome_path() -> String {
    profile::resolve_chrome()
}

#[tauri::command]
fn list_groups(state: tauri::State<AppState>) -> Vec<ProfileGroup> {
    state.manager.list_groups()
}

#[tauri::command]
fn create_group(state: tauri::State<AppState>, name: String, color: String) -> Result<ProfileGroup, String> {
    state.manager.create_group(name, color).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_group(state: tauri::State<AppState>, id: String, name: String, color: String) -> Result<ProfileGroup, String> {
    state.manager.update_group(&id, name, color).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_group(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.manager.delete_group(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_profile_group(state: tauri::State<AppState>, profile_id: String, group_id: Option<String>) -> Result<Profile, String> {
    state.manager.set_profile_group(&profile_id, group_id).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = ProfileManager::new();
    manager.recover();

    tauri::Builder::default()
        .manage(AppState { manager })
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
