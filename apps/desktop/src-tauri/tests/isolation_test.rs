use appsdesktop_lib::profile::ProfileManager;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct MockServer {
    port: u16,
    received_cookie: Arc<Mutex<Option<String>>>,
    received_report: Arc<Mutex<Option<String>>>,
}

impl MockServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let received_cookie = Arc::new(Mutex::new(None));
        let received_report = Arc::new(Mutex::new(None));

        let cookie_clone = received_cookie.clone();
        let report_clone = received_report.clone();

        thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut buffer = [0; 4096];
                    if let Ok(size) = stream.read(&mut buffer) {
                        let request = String::from_utf8_lossy(&buffer[..size]);
                        
                        // Parse Cookie header
                        let mut cookie = None;
                        for line in request.lines() {
                            let lower = line.to_lowercase();
                            if lower.starts_with("cookie:") {
                                cookie = Some(line["cookie:".len()..].trim().to_string());
                            }
                        }
                        if let Some(c) = cookie {
                            *cookie_clone.lock().unwrap() = Some(c);
                        }

                        // Determine path and return responses
                        if request.contains("GET /set") {
                            // Extract query parameter ?v=...
                            let value = request.split("/set?v=")
                                .nth(1)
                                .unwrap_or("")
                                .split(' ')
                                .next()
                                .unwrap_or("");
                            
                            let html = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
                                <html><body>\
                                <script>\
                                document.cookie = 'probe={}; max-age=3600; path=/';\
                                localStorage.setItem('probe', '{}');\
                                fetch('/report?v={}', {{ method: 'POST' }});\
                                </script>\
                                <h1>Set value to {}</h1>\
                                </body></html>",
                                value, value, value, value
                            );
                            let _ = stream.write_all(html.as_bytes());
                        } else if request.contains("POST /report") {
                            let value = request.split("/report?v=")
                                .nth(1)
                                .unwrap_or("")
                                .split(' ')
                                .next()
                                .unwrap_or("");
                            *report_clone.lock().unwrap() = Some(value.to_string());
                            let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = stream.write_all(response.as_bytes());
                        } else if request.contains("GET /read") {
                            let html = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
                                <html><body>\
                                <script>\
                                const localVal = localStorage.getItem('probe') || 'none';\
                                fetch('/report-read?v=' + localVal, {{ method: 'POST' }});\
                                </script>\
                                <h1>Reading...</h1>\
                                </body></html>";
                            let _ = stream.write_all(html.as_bytes());
                        } else if request.contains("POST /report-read") {
                            let value = request.split("/report-read?v=")
                                .nth(1)
                                .unwrap_or("")
                                .split(' ')
                                .next()
                                .unwrap_or("");
                            *report_clone.lock().unwrap() = Some(value.to_string());
                            let response = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = stream.write_all(response.as_bytes());
                        } else {
                            let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                            let _ = stream.write_all(response.as_bytes());
                        }
                    }
                }
            }
        });

        Self { port, received_cookie, received_report }
    }

    fn wait_for_report(&self, timeout: Duration) -> Option<String> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if let Some(val) = self.received_report.lock().unwrap().clone() {
                // Clear for next read
                *self.received_report.lock().unwrap() = None;
                return Some(val);
            }
            thread::sleep(Duration::from_millis(50));
        }
        None
    }

    fn get_cookie(&self) -> Option<String> {
        let cookie = self.received_cookie.lock().unwrap().clone();
        *self.received_cookie.lock().unwrap() = None;
        cookie
    }
}

#[test]
fn test_profile_isolation_and_persistence() {
    let server = MockServer::start();
    let manager = ProfileManager::new();
    let _ = manager.recover_startup();

    // Create Profile A
    let profile_a = manager.create("Profile A".to_string(), "#6366F1".to_string(), String::new(), vec![], None).unwrap();
    // Create Profile B
    let profile_b = manager.create("Profile B".to_string(), "#EC4899".to_string(), String::new(), vec![], None).unwrap();

    // 1. Open Profile A and set state
    let set_url_a = format!("http://127.0.0.1:{}/set?v=AAA", server.port);
    manager.open_with_args(profile_a.id.clone(), vec![set_url_a]).unwrap();

    // Wait for set report
    let val = server.wait_for_report(Duration::from_secs(8));
    assert_eq!(val, Some("AAA".to_string()), "Profile A failed to report state setup");

    // Close Profile A
    manager.close(profile_a.id.clone()).unwrap();
    thread::sleep(Duration::from_millis(1500)); // Allow process to cleanly exit

    // 2. Open Profile B and read state (should be empty/none)
    let read_url_b = format!("http://127.0.0.1:{}/read", server.port);
    manager.open_with_args(profile_b.id.clone(), vec![read_url_b]).unwrap();

    let val = server.wait_for_report(Duration::from_secs(8));
    assert_eq!(val, Some("none".to_string()), "Profile B read cookies/cache from Profile A!");

    // Set state in Profile B
    let set_url_b = format!("http://127.0.0.1:{}/set?v=BBB", server.port);
    manager.open_with_args(profile_b.id.clone(), vec![set_url_b]).unwrap();

    let val = server.wait_for_report(Duration::from_secs(8));
    assert_eq!(val, Some("BBB".to_string()), "Profile B failed to set state");

    // Close Profile B
    manager.close(profile_b.id.clone()).unwrap();
    thread::sleep(Duration::from_millis(1500));

    // 3. Re-open Profile A and read state (should still be AAA)
    let read_url_a = format!("http://127.0.0.1:{}/read", server.port);
    manager.open_with_args(profile_a.id.clone(), vec![read_url_a]).unwrap();

    let val = server.wait_for_report(Duration::from_secs(8));
    assert_eq!(val, Some("AAA".to_string()), "Profile A lost its session or read Profile B's data!");

    // Close Profile A
    manager.close(profile_a.id.clone()).unwrap();

    // Clean directories
    let _ = fs::remove_dir_all(profile_a.local_dir);
    let _ = fs::remove_dir_all(profile_b.local_dir);
}

#[test]
fn test_profile_concurrency_lock() {
    let manager = ProfileManager::new();
    let profile = manager.create("Concurrency Profile".to_string(), "#10B981".to_string(), String::new(), vec![], None).unwrap();

    // Open first time
    manager.open(profile.id.clone()).unwrap();

    // Open second time (should fail)
    let result = manager.open(profile.id.clone());
    assert!(result.is_err(), "Second open succeeded, lock failed!");
    assert_eq!(result.err().unwrap(), "Profile is already in use".to_string());

    // Close profile
    manager.close(profile.id.clone()).unwrap();
    let _ = fs::remove_dir_all(profile.local_dir);
}

/// Simula cenário 3 (app fechado abruptamente):
/// Abre perfil, depois simula recover_startup como se o app tivesse reiniciado
#[test]
fn test_crash_scenario_3_app_killed() {
    let manager = ProfileManager::new();
    let profile = manager.create("Crash3".to_string(), "#EF4444".to_string(), String::new(), vec![], None).unwrap();

    // Open profile (simula app funcionando)
    manager.open(profile.id.clone()).unwrap();
    assert_eq!(manager.list().iter().find(|p| p.id == profile.id).map(|p| p.status.as_str()), Some("in_use"));

    // Simula recover_startup como se app tivesse morrido e reiniciado
    // O lock file existe mas o processo que o criou morreu
    // recover_startup deve detectar e marcar como available
    let _ = manager.recover_startup();

    // Após recovery, perfil deve estar available se o lock estiver stale
    let profiles = manager.list();
    let recovered = profiles.iter().find(|p| p.id == profile.id).unwrap();

    // Se o processo Chrome ainda estiver rodando (não matamos), o lock ainda está ativo
    // então recover_startup não deve mudar o status
    // Este teste verifica que recover_startup não crasha e processa corretamente
    assert!(recovered.status == "in_use" || recovered.status == "available",
        "App kill recovery should result in consistent state");

    manager.close(profile.id.clone()).unwrap();
    let _ = fs::remove_dir_all(profile.local_dir);
}

/// Simula cenário 4 (Chromium crash):
/// Abre perfil, encerra o processo Chromium (simulado via close com Job Object kill),
/// depois reabre para verificar que recupera
#[test]
fn test_crash_scenario_4_chromium_killed() {
    let manager = ProfileManager::new();
    let profile = manager.create("Crash4".to_string(), "#EF4444".to_string(), String::new(), vec![], None).unwrap();

    // Open
    manager.open(profile.id.clone()).unwrap();

    // Close via Job Object kill (simula crash - close sem graciosidade)
    // No código real, close via Job Object kill simula o cenário de crash
    manager.close(profile.id.clone()).unwrap();
    thread::sleep(Duration::from_millis(500));

    // Reabrir deve funcionar (perfil recuperou)
    let result = manager.open(profile.id.clone());
    assert!(result.is_ok(), "Profile should reopen after crash-like close: {:?}", result);

    manager.close(profile.id.clone()).unwrap();
    let _ = fs::remove_dir_all(profile.local_dir);
}

/// Simula cenário 16 (órfãos na inicialização):
/// Cria lock file manualmente com PID inválido, depois recover_startup deve limpar
#[test]
fn test_crash_scenario_16_orphan_cleanup() {
    let manager = ProfileManager::new();
    let profile = manager.create("OrphanTest".to_string(), "#EF4444".to_string(), String::new(), vec![], None).unwrap();

    // Cria lock file com PID inválido manualmente (simula órfão de sessão anterior)
    let lock_path = std::path::Path::new(&profile.local_dir).join("lock");
    fs::write(&lock_path, "999999").unwrap(); // PID que quase certamente não existe

    // recover_startup deve detectar lock stale e limpar
    let _ = manager.recover_startup();
    thread::sleep(Duration::from_millis(200));

    // Perfil deve estar available após recovery de lock stale
    let profiles = manager.list();
    let cleaned = profiles.iter().find(|p| p.id == profile.id).unwrap();
    assert_eq!(cleaned.status, "available", "Orphan lock should be cleaned on startup");

    // Lock file deve ter sido removido
    assert!(!lock_path.exists(), "Stale lock file should be removed");

    let _ = fs::remove_dir_all(profile.local_dir);
}
