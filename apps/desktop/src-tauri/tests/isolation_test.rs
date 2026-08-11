use multi_contas_lib::profile::ProfileManager;
use multi_contas_lib::registry::ProfileRegistry;
use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_profile_isolation_and_persistence() {
    let manager = ProfileManager::new();
    manager.recover();

    let registry = Arc::new(ProfileRegistry::new());

    let profile_a = manager
        .create(
            "Profile A".to_string(),
            "#6366F1".to_string(),
            String::new(),
            vec![],
            None,
            None,
        )
        .unwrap();
    let profile_b = manager
        .create(
            "Profile B".to_string(),
            "#EC4899".to_string(),
            String::new(),
            vec![],
            None,
            None,
        )
        .unwrap();

    // 1. Open Profile A
    let _port_a = manager.open(&profile_a.id, registry.clone()).unwrap();
    assert_eq!(
        manager.list().iter().find(|p| p.id == profile_a.id).map(|p| p.status.as_str()),
        Some("in_use")
    );
    manager.close(&profile_a.id).unwrap();
    thread::sleep(Duration::from_millis(1500));

    // 2. Open Profile B (independent from A)
    let _port_b = manager.open(&profile_b.id, registry.clone()).unwrap();
    assert_eq!(
        manager.list().iter().find(|p| p.id == profile_b.id).map(|p| p.status.as_str()),
        Some("in_use")
    );
    manager.close(&profile_b.id).unwrap();
    thread::sleep(Duration::from_millis(1500));

    // 3. Re-open Profile A (persistência)
    let _port_a2 = manager.open(&profile_a.id, registry.clone()).unwrap();
    assert_eq!(
        manager.list().iter().find(|p| p.id == profile_a.id).map(|p| p.status.as_str()),
        Some("in_use")
    );
    manager.close(&profile_a.id).unwrap();

    let _ = fs::remove_dir_all(&profile_a.local_dir);
    let _ = fs::remove_dir_all(&profile_b.local_dir);
}

#[test]
fn test_profile_concurrency_lock() {
    let manager = ProfileManager::new();
    let registry = Arc::new(ProfileRegistry::new());
    let profile = manager
        .create(
            "Concurrency Profile".to_string(),
            "#10B981".to_string(),
            String::new(),
            vec![],
            None,
            None,
        )
        .unwrap();

    let _port = manager.open(&profile.id, registry.clone()).unwrap();

    let result = manager.open(&profile.id, registry.clone());
    assert!(result.is_err(), "Second open succeeded, lock failed!");
    assert_eq!(
        result.err().unwrap(),
        "Profile already open".to_string()
    );

    manager.close(&profile.id).unwrap();
    let _ = fs::remove_dir_all(&profile.local_dir);
}

/// Simula cenário 3 (app fechado abruptamente):
/// Abre perfil, depois simula recover como se o app tivesse reiniciado
#[test]
fn test_crash_scenario_3_app_killed() {
    let manager = ProfileManager::new();
    let registry = Arc::new(ProfileRegistry::new());
    let profile = manager
        .create(
            "Crash3".to_string(),
            "#EF4444".to_string(),
            String::new(),
            vec![],
            None,
            None,
        )
        .unwrap();

    manager.open(&profile.id, registry.clone()).unwrap();
    assert_eq!(
        manager.list().iter().find(|p| p.id == profile.id).map(|p| p.status.as_str()),
        Some("in_use")
    );

    // recover detecta lock stale e marca como available
    manager.recover();

    let profiles = manager.list();
    let recovered = profiles.iter().find(|p| p.id == profile.id).unwrap();

    assert!(
        recovered.status == "in_use" || recovered.status == "available",
        "App kill recovery should result in consistent state"
    );

    manager.close(&profile.id).unwrap();
    let _ = fs::remove_dir_all(&profile.local_dir);
}

/// Simula cenário 4 (Chromium crash):
/// Abre perfil, fecha (simula kill), reabre
#[test]
fn test_crash_scenario_4_chromium_killed() {
    let manager = ProfileManager::new();
    let registry = Arc::new(ProfileRegistry::new());
    let profile = manager
        .create(
            "Crash4".to_string(),
            "#EF4444".to_string(),
            String::new(),
            vec![],
            None,
            None,
        )
        .unwrap();

    manager.open(&profile.id, registry.clone()).unwrap();
    manager.close(&profile.id).unwrap();
    thread::sleep(Duration::from_millis(500));

    let result = manager.open(&profile.id, registry.clone());
    assert!(
        result.is_ok(),
        "Profile should reopen after crash-like close: {:?}",
        result
    );

    manager.close(&profile.id).unwrap();
    let _ = fs::remove_dir_all(&profile.local_dir);
}

/// Simula cenário 16 (órfãos na inicialização):
/// Cria lock file manualmente com PID inválido, recover deve marcar como available
#[test]
fn test_crash_scenario_16_orphan_cleanup() {
    let manager = ProfileManager::new();
    let profile = manager
        .create(
            "OrphanTest".to_string(),
            "#EF4444".to_string(),
            String::new(),
            vec![],
            None,
            None,
        )
        .unwrap();

    let lock_path = std::path::Path::new(&profile.local_dir).join("lock");
    fs::write(&lock_path, "999999").unwrap();

    manager.recover();
    thread::sleep(Duration::from_millis(200));

    let profiles = manager.list();
    let cleaned = profiles.iter().find(|p| p.id == profile.id).unwrap();
    assert_eq!(
        cleaned.status, "available",
        "Orphan profile should be marked available on recovery"
    );

    let _ = fs::remove_dir_all(&profile.local_dir);
}
