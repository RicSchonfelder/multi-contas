//! HermesServer — servidor HTTP local (127.0.0.1) para o Hermes orquestrar
//! perfis Chrome multi-conta via CDP.
//!
//! Autenticacao: header `Authorization: Bearer <token>`. O token e gerado
//! aleatoriamente na inicializacao (ou via env `HERMES_TOKEN`) e logado no
//! console do app.
//!
//! Rotas:
//!   GET  /api/v1/health              -> { ok, version }
//!   GET  /api/v1/profiles            -> lista perfis + porta CDP
//!   GET  /api/v1/profiles/:id        -> status de 1 perfil
//!   POST /api/v1/profiles/:id/start  -> abre perfil, retorna { cdpPort, wsUrl }
//!   POST /api/v1/profiles/:id/stop   -> fecha perfil
//!   POST /api/v1/profiles/:id/command-> proxy CDP (envia comando via WS)
//!
//! Apenas localhost. CDP nao tem auth, entao o servidor sobe o CDP sob
//! demanda (quando o Hermes pede /start) e o token protege o servidor.

use std::sync::Arc;
use std::thread;

use crate::profile::ProfileManager;
use crate::registry::ProfileRegistry;
use serde_json::{json, Value};

pub struct HermesServer {
    port: u16,
    token: String,
}

impl HermesServer {
    pub fn new() -> Self {
        let token = std::env::var("HERMES_TOKEN")
            .unwrap_or_else(|_| generate_token());
        let port = std::env::var("HERMES_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(29222);
        Self { port, token }
    }

    /// Inicia o servidor em uma thread dedicada.
    pub fn start(self, manager: Arc<ProfileManager>, registry: Arc<ProfileRegistry>) {
        let port = self.port;
        let token = Arc::new(self.token);
        println!("[HERMES] listening on 127.0.0.1:{} | token: {}...", port, &token[..8.min(token.len())]);
        thread::spawn(move || {
            let server = tiny_http::Server::http(("127.0.0.1", port));
            let server = match server {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[HERMES] falha ao ligar na porta {}: {}", port, e);
                    return;
                }
            };
            for request in server.incoming_requests() {
                handle_request(request, &token, &manager, &registry);
            }
        });
    }
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn check_auth(req: &tiny_http::Request, token: &str) -> bool {
    match req.headers().iter().find(|h| h.field.as_str() == "authorization") {
        Some(h) => {
            let v = h.value.as_str();
            v == format!("Bearer {}", token)
        }
        None => false,
    }
}

fn json_response(code: u16, body: Value) -> tiny_http::ResponseBox {
    let s = serde_json::to_string(&body).unwrap_or_else(|_| "{}".into());
    tiny_http::Response::from_string(s)
        .with_status_code(code)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .boxed()
}

fn handle_request(
    mut req: tiny_http::Request,
    token: &Arc<String>,
    manager: &Arc<ProfileManager>,
    registry: &Arc<ProfileRegistry>,
) {
    let url = req.url().to_string();
    let method = req.method().as_str().to_string();

    // /health nao exige token
    if url == "/api/v1/health" && method == "GET" {
        let _ = req.respond(json_response(200, json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") })));
        return;
    }

    if !check_auth(&req, token) {
        let _ = req.respond(json_response(401, json!({ "error": "unauthorized" })));
        return;
    }

    // Extrai :id das rotas /api/v1/profiles/:id...
    let parts: Vec<&str> = url.split('/').filter(|s| !s.is_empty()).collect();
    // parts = ["api","v1","profiles", ":id", "start"?]

    match (method.as_str(), parts.as_slice()) {
        ("GET", ["api", "v1", "profiles"]) => {
            let list: Vec<Value> = manager.list().iter().map(|p| {
                let active = registry.get(&p.id);
                json!({
                    "id": p.id,
                    "name": p.name,
                    "status": p.status,
                    "cdpPort": active.as_ref().map(|a| a.cdp_port),
                    "wsUrl": active.as_ref().and_then(|a| a.ws_url.clone()),
                })
            }).collect();
            let _ = req.respond(json_response(200, json!(list)));
        }
        ("GET", ["api", "v1", "profiles", id]) => {
            let active = registry.get(id);
            let body = json!({
                "id": id,
                "cdpPort": active.as_ref().map(|a| a.cdp_port),
                "wsUrl": active.as_ref().and_then(|a| a.ws_url.clone()),
                "active": active.is_some(),
            });
            let _ = req.respond(json_response(200, body));
        }
        ("POST", ["api", "v1", "profiles", id, "start"]) => {
            let (code, resp) = match manager.open(id, registry.clone()) {
                Ok(()) => {
                    let active = registry.get(id);
                    (200, json!({
                        "cdpPort": active.as_ref().map(|a| a.cdp_port),
                        "wsUrl": active.as_ref().and_then(|a| a.ws_url.clone()),
                    }))
                }
                Err(e) => (500, json!({ "error": e })),
            };
            let _ = req.respond(json_response(code, resp));
        }
        ("POST", ["api", "v1", "profiles", id, "stop"]) => {
            let (code, resp) = match manager.close(id, registry) {
                Ok(()) => (200, json!({ "ok": true })),
                Err(e) => (500, json!({ "error": e })),
            };
            let _ = req.respond(json_response(code, resp));
        }
        ("POST", ["api", "v1", "profiles", id, "command"]) => {
            // Lê body, encaminha via WS para o Chrome CDP
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            let (code, resp) = match registry.get(id) {
                Some(active) => match active.ws_url {
                    Some(ws) => match cdp_command(&ws, &body) {
                        Ok(r) => (200, r),
                        Err(e) => (502, json!({ "error": e })),
                    },
                    None => (409, json!({ "error": "wsUrl nao disponivel; abra o perfil primeiro" })),
                },
                None => (404, json!({ "error": "perfil nao ativo" })),
            };
            let _ = req.respond(json_response(code, resp));
        }
        _ => {
            let _ = req.respond(json_response(404, json!({ "error": "not found" })));
        }
    }
}

/// Envia um comando CDP via WebSocket para o Chrome e retorna o resultado.
fn cdp_command(ws_url: &str, command_json: &str) -> Result<Value, String> {
    use tungstenite::{connect, Message};

    let (mut socket, _resp) = connect(ws_url).map_err(|e| format!("WS connect falhou: {}", e))?;
    socket.write_message(Message::Text(command_json.to_string().into())).map_err(|e| e.to_string())?;

    // Aguarda resposta (match por id e feito pelo chamador no body)
    #[allow(deprecated)]
    let msg = socket.read_message().map_err(|e| e.to_string())?;
    match msg {
        Message::Text(t) => {
            serde_json::from_str(&t).map_err(|e| format!("parse: {}", e))
        }
        Message::Binary(b) => {
            let s = String::from_utf8_lossy(&b);
            serde_json::from_str(&s).map_err(|e| format!("parse: {}", e))
        }
        _ => Err("resposta CDP inesperada".into()),
    }
}
