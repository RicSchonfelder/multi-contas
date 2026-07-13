use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub token_type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RefreshToken {
    pub token: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionStore {
    pub active_refresh_tokens: Vec<RefreshToken>,
    pub revoked_tokens: Vec<String>,
}

pub struct SessionManager {
    secret: String,
    store_path: PathBuf,
    revoked: Mutex<HashSet<String>>,
}

impl SessionManager {
    pub fn new() -> Self {
        let app_dir = Self::get_session_dir();
        let _ = fs::create_dir_all(&app_dir);
        let secret = Self::load_or_create_secret(&app_dir);

        let manager = Self {
            secret,
            store_path: app_dir.join("sessions.json"),
            revoked: Mutex::new(HashSet::new()),
        };

        manager.load_revoked();
        manager
    }

    fn get_session_dir() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE").unwrap_or_else(|_| "C:".to_string())
        });
        PathBuf::from(local_app_data)
            .join("BrowserWorkspace")
            .join("auth")
    }

    fn load_or_create_secret(dir: &PathBuf) -> String {
        let secret_path = dir.join("jwt_secret");
        if secret_path.exists() {
            fs::read_to_string(&secret_path).unwrap_or_default()
        } else {
            let secret = uuid::Uuid::new_v4().to_string()
                + &uuid::Uuid::new_v4().to_string()
                + &uuid::Uuid::new_v4().to_string();
            let _ = fs::write(&secret_path, &secret);
            secret
        }
    }

    fn load_revoked(&self) {
        if let Ok(content) = fs::read_to_string(&self.store_path) {
            if let Ok(store) = serde_json::from_str::<SessionStore>(&content) {
                let mut revoked = self.revoked.lock().unwrap();
                for rt in store.revoked_tokens {
                    revoked.insert(rt);
                }
            }
        }
    }

    fn save_revoked(&self, token: &str) {
        let mut store = SessionStore {
            active_refresh_tokens: Vec::new(),
            revoked_tokens: Vec::new(),
        };
        if let Ok(content) = fs::read_to_string(&self.store_path) {
            if let Ok(existing) = serde_json::from_str::<SessionStore>(&content) {
                store = existing;
            }
        }
        store.revoked_tokens.push(token.to_string());
        // Keep only last 100 revoked tokens
        if store.revoked_tokens.len() > 100 {
            store.revoked_tokens.drain(0..store.revoked_tokens.len() - 100);
        }
        let temp_path = self.store_path.with_extension("json.tmp");
        if let Ok(content) = serde_json::to_string_pretty(&store) {
            if let Ok(mut file) = File::create(&temp_path) {
                let _ = file.write_all(content.as_bytes());
                let _ = file.sync_all();
                let _ = fs::rename(&temp_path, &self.store_path);
            }
        }
    }

    pub fn create_session(&self, user_id: &str) -> Result<(String, String), String> {
        let now = Utc::now();

        let access_claims = Claims {
            sub: user_id.to_string(),
            exp: (now + Duration::minutes(15)).timestamp() as usize,
            iat: now.timestamp() as usize,
            token_type: "access".to_string(),
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| format!("JWT error: {}", e))?;

        let refresh_id = uuid::Uuid::new_v4().to_string();
        let refresh_claims = Claims {
            sub: user_id.to_string(),
            exp: (now + Duration::days(7)).timestamp() as usize,
            iat: now.timestamp() as usize,
            token_type: "refresh".to_string(),
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| format!("JWT error: {}", e))?;

        let rt = RefreshToken {
            token: refresh_id,
            expires_at: (now + Duration::days(7)).to_rfc3339(),
            created_at: now.to_rfc3339(),
        };

        let mut store = SessionStore {
            active_refresh_tokens: Vec::new(),
            revoked_tokens: Vec::new(),
        };
        if let Ok(content) = fs::read_to_string(&self.store_path) {
            if let Ok(existing) = serde_json::from_str::<SessionStore>(&content) {
                store = existing;
            }
        }
        store.active_refresh_tokens.push(rt);
        if let Ok(json) = serde_json::to_string_pretty(&store) {
            let temp_path = self.store_path.with_extension("json.tmp");
            if let Ok(mut file) = File::create(&temp_path) {
                let _ = file.write_all(json.as_bytes());
                let _ = file.sync_all();
                let _ = fs::rename(&temp_path, &self.store_path);
            }
        }

        Ok((access_token, refresh_token))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, String> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| format!("Invalid token: {}", e))?;

        let claims = token_data.claims;

        if claims.token_type == "refresh" {
            let revoked = self.revoked.lock().unwrap();
            if revoked.contains(token) {
                return Err("Token revoked".to_string());
            }
        }

        Ok(claims)
    }

    pub fn refresh_session(&self, refresh_token: &str) -> Result<(String, String), String> {
        let claims = self.verify_token(refresh_token)?;

        if claims.token_type != "refresh" {
            return Err("Invalid token type".to_string());
        }

        self.revoke_token(refresh_token);
        self.create_session(&claims.sub)
    }

    pub fn revoke_token(&self, token: &str) {
        self.revoked.lock().unwrap().insert(token.to_string());
        self.save_revoked(token);
    }

    pub fn revoke_all(&self) {
        let mut store = SessionStore {
            active_refresh_tokens: Vec::new(),
            revoked_tokens: Vec::new(),
        };
        if let Ok(content) = fs::read_to_string(&self.store_path) {
            if let Ok(existing) = serde_json::from_str::<SessionStore>(&content) {
                for rt in &existing.active_refresh_tokens {
                    self.revoked.lock().unwrap().insert(rt.token.clone());
                    store.revoked_tokens.push(rt.token.clone());
                }
            }
        }
        store.active_refresh_tokens.clear();
        if let Ok(json) = serde_json::to_string_pretty(&store) {
            let temp_path = self.store_path.with_extension("json.tmp");
            if let Ok(mut file) = File::create(&temp_path) {
                let _ = file.write_all(json.as_bytes());
                let _ = file.sync_all();
                let _ = fs::rename(&temp_path, &self.store_path);
            }
        }
    }

    pub fn is_revoked(&self, token: &str) -> bool {
        self.revoked.lock().unwrap().contains(token)
    }
}
