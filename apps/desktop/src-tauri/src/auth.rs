use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use totp_rs::{Secret, TOTP};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthStore {
    pub master_password_hash: String,
    pub email: String,
    pub mfa_secret: Option<String>,
    pub mfa_enabled: bool,
    pub recovery_codes: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct LoginAttempt {
    pub count: u32,
    pub locked_until: Option<Instant>,
}

pub struct AuthManager {
    store_path: PathBuf,
    login_attempts: Mutex<HashMap<String, LoginAttempt>>,
    max_attempts: u32,
    lockout_duration: Duration,
}

impl AuthManager {
    pub fn new() -> Self {
        let app_dir = Self::get_auth_dir();
        let _ = fs::create_dir_all(&app_dir);
        Self {
            store_path: app_dir.join("auth.json"),
            login_attempts: Mutex::new(HashMap::new()),
            max_attempts: 5,
            lockout_duration: Duration::from_secs(300),
        }
    }

    fn get_auth_dir() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
        });
        PathBuf::from(local_app_data).join("BrowserWorkspace").join("auth")
    }

    fn load_store(&self) -> io::Result<Option<AuthStore>> {
        if !self.store_path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.store_path)?;
        let store: AuthStore = serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(store))
    }

    fn save_store(&self, store: &AuthStore) -> io::Result<()> {
        let temp_path = self.store_path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(store)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut file = File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp_path, &self.store_path)?;
        Ok(())
    }

    pub fn is_setup(&self) -> bool {
        self.load_store().ok().flatten().is_some()
    }

    pub fn setup(&self, password: &str, email: &str) -> Result<AuthSetupResult, String> {
        if self.is_setup() {
            return Err("Auth already configured".to_string());
        }

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Hash error: {}", e))?
            .to_string();

        let secret = Secret::generate_secret().to_encoded().to_string();
        let recovery_codes: Vec<String> = (0..8)
            .map(|_| {
                let code: String = (0..12)
                    .map(|_| {
                        let charset = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
                        charset[rand::thread_rng().gen_range(0..charset.len())] as char
                    })
                    .collect();
                code
            })
            .collect();

        let now = chrono::Utc::now().to_rfc3339();
        let store = AuthStore {
            master_password_hash: hash,
            email: email.to_string(),
            mfa_secret: Some(secret.clone()),
            mfa_enabled: false,
            recovery_codes: recovery_codes.clone(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.save_store(&store).map_err(|e| e.to_string())?;

        let totp_uri = self.generate_totp_uri(&secret.clone(), email);

        Ok(AuthSetupResult {
            recovery_codes,
            totp_uri,
            mfa_secret: secret,
        })
    }

    fn generate_totp_uri(&self, secret: &str, email: &str) -> String {
        let decoded = Secret::Encoded(secret.to_string())
            .to_bytes()
            .unwrap_or_default();
        let totp = TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            decoded,
            Some("BrowserWorkspace".to_string()),
            email.to_string(),
        )
        .unwrap();
        totp.get_url()
    }

    pub fn verify_password(&self, password: &str) -> Result<(), String> {
        let store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;

        let parsed_hash =
            PasswordHash::new(&store.master_password_hash).map_err(|e| format!("Parse error: {}", e))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| "Invalid password".to_string())
    }

    pub fn check_lockout(&self, identifier: &str) -> Result<(), String> {
        let mut attempts = self.login_attempts.lock().unwrap();
        let now = Instant::now();
        if let Some(attempt) = attempts.get(identifier) {
            if let Some(locked_until) = attempt.locked_until {
                if now < locked_until {
                    let remaining = locked_until.duration_since(now).as_secs();
                    return Err(format!("Account locked. Try again in {} seconds", remaining));
                } else {
                    attempts.remove(identifier);
                }
            }
        }
        Ok(())
    }

    pub fn record_failed_attempt(&self, identifier: &str) -> Result<(), String> {
        let mut attempts = self.login_attempts.lock().unwrap();
        let entry = attempts
            .entry(identifier.to_string())
            .or_insert(LoginAttempt {
                count: 0,
                locked_until: None,
            });
        entry.count += 1;
        if entry.count >= self.max_attempts {
            entry.locked_until = Some(Instant::now() + self.lockout_duration);
            return Err("Too many failed attempts. Account locked for 5 minutes.".to_string());
        }
        Ok(())
    }

    pub fn reset_attempts(&self, identifier: &str) {
        self.login_attempts.lock().unwrap().remove(identifier);
    }

    pub fn verify_totp(&self, code: &str) -> Result<(), String> {
        let store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;

        if !store.mfa_enabled {
            return Ok(());
        }

        let secret = store.mfa_secret.ok_or("MFA not configured")?;
        let decoded = Secret::Encoded(secret)
            .to_bytes()
            .map_err(|_| "Invalid secret".to_string())?;
        let totp = TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            decoded,
            Some("BrowserWorkspace".to_string()),
            store.email,
        )
        .unwrap();

        match totp.check_current(code) {
            Ok(true) => Ok(()),
            Ok(false) => Err("Invalid TOTP code".to_string()),
            Err(e) => Err(format!("TOTP error: {}", e)),
        }
    }

    pub fn enable_mfa(&self) -> Result<String, String> {
        let mut store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;
        store.mfa_enabled = true;
        store.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_store(&store).map_err(|e| e.to_string())?;
        let secret = store.mfa_secret.clone().unwrap_or_default();
        Ok(self.generate_totp_uri(&secret, &store.email))
    }

    pub fn disable_mfa(&self) -> Result<(), String> {
        let mut store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;
        store.mfa_enabled = false;
        store.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_store(&store).map_err(|e| e.to_string())
    }

    pub fn get_recovery_codes(&self) -> Result<Vec<String>, String> {
        let store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;
        Ok(store.recovery_codes)
    }

    pub fn recover_access(&self, recovery_code: &str, new_password: &str) -> Result<(), String> {
        let mut store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;

        let idx = store.recovery_codes.iter().position(|c| c == recovery_code);
        match idx {
            Some(i) => {
                store.recovery_codes.remove(i);
                let salt = SaltString::generate(&mut OsRng);
                let argon2 = Argon2::default();
                let hash = argon2
                    .hash_password(new_password.as_bytes(), &salt)
                    .map_err(|e| format!("Hash error: {}", e))?
                    .to_string();
                store.master_password_hash = hash;
                store.updated_at = chrono::Utc::now().to_rfc3339();
                self.save_store(&store).map_err(|e| e.to_string())?;
                Ok(())
            }
            None => Err("Invalid recovery code".to_string()),
        }
    }

    pub fn change_password(&self, old: &str, new: &str) -> Result<(), String> {
        self.verify_password(old)?;
        let mut store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(new.as_bytes(), &salt)
            .map_err(|e| format!("Hash error: {}", e))?
            .to_string();
        store.master_password_hash = hash;
        store.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_store(&store).map_err(|e| e.to_string())
    }

    pub fn get_mfa_status(&self) -> Result<bool, String> {
        let store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;
        Ok(store.mfa_enabled)
    }

    pub fn get_totp_uri(&self) -> Result<String, String> {
        let store = self
            .load_store()
            .map_err(|e| e.to_string())?
            .ok_or("Auth not configured")?;
        let secret = store.mfa_secret.clone().ok_or("MFA not configured")?;
        Ok(self.generate_totp_uri(&secret, &store.email))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthSetupResult {
    pub recovery_codes: Vec<String>,
    pub totp_uri: String,
    pub mfa_secret: String,
}
