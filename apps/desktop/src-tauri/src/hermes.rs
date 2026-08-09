use serde_json::json;
use std::io::Cursor;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::AppState;

const CONTROL_PORT: u16 = 29222;

fn read_token() -> Option<String> {
    let dir = crate::profile::get_app_dir();
    let p = dir.join("control_token");
    std::fs::read_to_string(p)
        .ok()
        .map(|s| s.trim().to_string())
}

fn body_to_json(req: &mut tiny_http::Request) -> Option<serde_json::Value> {
    let mut buf = String::new();
    if req.as_reader().read_to_string(&mut buf).is_err() {
        return None;
    }
    if buf.trim().is_empty() {
        return None;
    }
    serde_json::from_str(&buf).ok()
}

fn build(status: StatusCode, body: String, cors: bool) -> Response<Cursor<Vec<u8>>> {
    let data = body.into_bytes();
    let mut resp = Response::from_data(data).with_status_code(status);
    resp = resp.with_header(
        Header::from_bytes(b"Content-Type", b"application/json").unwrap(),
    );
    if cors {
        resp = resp.with_header(
            Header::from_bytes(b"Access-Control-Allow-Origin", b"*").unwrap(),
        );
    }
    resp
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response<Cursor<Vec<u8>>> {
    let s = serde_json::to_string(&body).unwrap_or_else(|_| "{}".into());
    build(status, s, true)
}

fn ok(body: serde_json::Value) -> Response<Cursor<Vec<u8>>> {
    json_response(StatusCode(200), body)
}

fn err(status: u16, msg: &str) -> Response<Cursor<Vec<u8>>> {
    json_response(StatusCode(status), json!({ "error": msg }))
}

pub fn start(state: Arc<AppState>) {
    std::thread::spawn(move || {
        let token = read_token();
        match Server::http((Ipv4Addr::LOCALHOST, CONTROL_PORT)) {
            Ok(server) => {
                eprintln!(
                    "[HERMES] listening on 127.0.0.1:{} | token: {}",
                    CONTROL_PORT,
                    token.as_deref().map(|t| &t[..6]).unwrap_or("AUSENTE")
                );
                for mut req in server.incoming_requests() {
                    let resp = handle(&state, &token, &mut req);
                    let _ = req.respond(resp);
                }
            }
            Err(e) => {
                eprintln!(
                    "[HERMES] Falha ao iniciar servidor na {}: {}",
                    CONTROL_PORT, e
                );
            }
        }
    });
}

fn header_field_matches(h: &tiny_http::Header, target: &str) -> bool {
    h.field
        .as_str()
        .to_string()
        .eq_ignore_ascii_case(target)
}

fn authorized(token: &Option<String>, req: &tiny_http::Request) -> bool {
    match token {
        None => false,
        Some(t) => {
            let header = req.headers().iter().find(|h| {
                header_field_matches(h, "X-Control-Token")
                    || header_field_matches(h, "Authorization")
            });
            match header {
                Some(h) => {
                    let v = String::from_utf8_lossy(h.value.as_ref());
                    let v = v.trim().trim_start_matches("Bearer ").trim();
                    let field_ok =
                        header_field_matches(h, "X-Control-Token")
                            || header_field_matches(h, "Authorization");
                    field_ok && v == t.as_str()
                }
                None => false,
            }
        }
    }
}

fn handle(
    state: &Arc<AppState>,
    token: &Option<String>,
    req: &mut tiny_http::Request,
) -> Response<Cursor<Vec<u8>>> {
    let url = req.url().to_string();
    let method = req.method().clone();

    if method == Method::Options {
        return ok(json!({ "ok": true }));
    }

    if !authorized(token, req) {
        return err(403, "Token de controle inválido ou ausente");
    }

    if url == "/api/v1/health" && method == Method::Get {
        return ok(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }));
    }

    if url == "/api/v1/profiles" && method == Method::Get {
        let profiles = state.manager.list();
        let list: Vec<serde_json::Value> = profiles
            .iter()
            .map(|p| {
                let port = state.registry.get_port(&p.id);
                json!({
                    "id": p.id,
                    "name": p.name,
                    "status": p.status,
                    "cdpPort": port,
                    "wsUrl": state.registry.get_ws_url(&p.id),
                })
            })
            .collect();
        return ok(json!({ "profiles": list }));
    }

    if url == "/api/v1/profiles/open-all" && method == Method::Post {
        let profiles = state.manager.list();
        let mut opened = Vec::new();
        for p in profiles {
            if p.status == "available" {
                match state.manager.open(&p.id, state.registry.clone()) {
                    Ok(port) => opened.push(json!({ "id": p.id, "name": p.name, "cdpPort": port })),
                    Err(e) => opened.push(json!({ "id": p.id, "name": p.name, "error": e })),
                }
            }
        }
        return ok(json!({ "opened": opened }));
    }

    if url == "/api/v1/profiles/open-all-and-navigate" && method == Method::Post {
        let body = body_to_json(req);
        let target = body
            .as_ref()
            .and_then(|b| b.get("url"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        if target.is_empty() {
            return err(400, "Informe 'url' no corpo");
        }
        let profiles = state.manager.list();
        let mut results = Vec::new();
        for p in profiles {
            if p.status == "available" {
                match state.manager.open(&p.id, state.registry.clone()) {
                    Ok(port) => {
                        let nav_target = target.clone();
                        std::thread::spawn(move || {
                            let _ = crate::cdp::navigate(port, &nav_target);
                        });
                        results.push(json!({
                            "id": p.id, "name": p.name,
                            "cdpPort": port, "navigatingTo": target
                        }));
                    }
                    Err(e) => results.push(json!({ "id": p.id, "name": p.name, "error": e })),
                }
            }
        }
        return ok(json!({ "openedAndNavigating": results }));
    }

    if let Some(id) = url.strip_prefix("/api/v1/profiles/") {
        if let Some(profile_id) = id.strip_suffix("/open") {
            if method == Method::Post {
                return match state.manager.open(profile_id, state.registry.clone()) {
                    Ok(port) => ok(
                        json!({ "id": profile_id, "cdpPort": port, "wsUrl": state.registry.get_ws_url(profile_id) }),
                    ),
                    Err(e) => err(500, &e),
                };
            }
        }
        if let Some(profile_id) = id.strip_suffix("/close") {
            if method == Method::Post {
                return match state.manager.close(profile_id) {
                    Ok(()) => {
                        state.registry.remove(profile_id);
                        ok(json!({ "id": profile_id, "ok": true }))
                    }
                    Err(e) => err(500, &e),
                };
            }
        }
        if let Some(profile_id) = id.strip_suffix("/navigate") {
            if method == Method::Post {
                let body = body_to_json(req);
                let target = body
                    .as_ref()
                    .and_then(|b| b.get("url"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();
                if target.is_empty() {
                    return err(400, "Informe 'url' no corpo");
                }
                let port = match state.registry.get_port(profile_id) {
                    Some(p) => p,
                    None => return err(409, "Perfil não está aberto (ou sem porta CDP)"),
                };
                return match crate::cdp::navigate(port, &target) {
                    Ok(()) => ok(json!({ "id": profile_id, "navigatedTo": target })),
                    Err(e) => err(500, &e),
                };
            }
        }
        if let Some(profile_id) = id.strip_suffix("/comment") {
            if method == Method::Post {
                let body = body_to_json(req);
                let text = body
                    .as_ref()
                    .and_then(|b| b.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                if text.is_empty() {
                    return err(400, "Informe 'text' no corpo");
                }
                let port = match state.registry.get_port(profile_id) {
                    Some(p) => p,
                    None => return err(409, "Perfil não está aberto (ou sem porta CDP)"),
                };
                return match crate::cdp::send_comment(port, &text) {
                    Ok(()) => ok(json!({ "id": profile_id, "commented": text })),
                    Err(e) => err(500, &e),
                };
            }
        }
    }

    err(404, "Rota não encontrada")
}
