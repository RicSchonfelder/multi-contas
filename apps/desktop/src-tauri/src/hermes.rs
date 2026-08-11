use serde_json::json;
use std::io::Cursor;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::AppState;
use crate::orchestrator::ReasonCode;

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

fn ok_deprecated(body: serde_json::Value) -> Response<Cursor<Vec<u8>>> {
    let s = serde_json::to_string(&body).unwrap_or_else(|_| "{}".into());
    let data = s.into_bytes();
    let mut resp = Response::from_data(data)
        .with_status_code(StatusCode(200))
        .with_header(
            Header::from_bytes(b"Content-Type", b"application/json").unwrap(),
        )
        .with_header(
            Header::from_bytes(b"Access-Control-Allow-Origin", b"*").unwrap(),
        )
        .with_header(
            Header::from_bytes(b"Deprecation", b"true").unwrap(),
        );
    resp
}

fn err(status: u16, msg: &str) -> Response<Cursor<Vec<u8>>> {
    json_response(StatusCode(status), json!({ "error": msg }))
}

fn err_with_reason(
    status: u16,
    msg: &str,
    reason: ReasonCode,
    extra: Option<serde_json::Value>,
) -> Response<Cursor<Vec<u8>>> {
    let mut body = json!({
        "error": msg,
        "reason": reason
    });
    if let Some(e) = extra {
        if let serde_json::Value::Object(ref mut map) = body {
            if let serde_json::Value::Object(extra_map) = e {
                for (k, v) in extra_map {
                    map.insert(k, v);
                }
            }
        }
    }
    json_response(StatusCode(status), body)
}

fn limit_exceeded_error(limit: usize, active_ids: Vec<String>) -> Response<Cursor<Vec<u8>>> {
    err_with_reason(
        429,
        "max_active_profiles_reached",
        ReasonCode::LimitExceeded,
        Some(json!({
            "limit": limit,
            "active": active_ids,
            "retryAfterMs": 5000
        })),
    )
}

fn resource_exhausted_error(reasons: &str) -> Response<Cursor<Vec<u8>>> {
    err_with_reason(
        503,
        "resource_exhausted",
        ReasonCode::ResourceExhausted,
        Some(json!({ "details": reasons })),
    )
}

pub fn start(state: Arc<AppState>) {
    std::thread::spawn(move || {
        let token = read_token();
        match Server::http((Ipv4Addr::LOCALHOST, CONTROL_PORT)) {
            Ok(server) => {
                eprintln!(
                    "[HERMES] listening on 127.0.0.1:{} | token: {}",
                    CONTROL_PORT,
                    token.as_deref().map(|t| &t[..t.len().min(6)]).unwrap_or("AUSENTE")
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

    // -- Health (extended) --
    if url == "/api/v1/health" && method == Method::Get {
        let orch_status = state.orchestrator.status();
        return ok(json!({
            "ok": true,
            "version": env!("CARGO_PKG_VERSION"),
            "orchestrator": {
                "active_profiles": orch_status.active_profiles,
                "max_profiles": orch_status.max_active_profiles,
                "queue_depth": orch_status.queue_depth,
            },
            "resources": {
                "free_ram_mb": orch_status.resources.free_ram_mb,
                "load_1m": orch_status.resources.load_1m,
                "load_5m": orch_status.resources.load_5m,
                "chrome_processes": orch_status.resources.chrome_processes,
                "healthy": orch_status.resources.healthy,
            },
        }));
    }

    // -- Orchestrator status --
    if url == "/api/v1/orchestrator/status" && method == Method::Get {
        let status = state.orchestrator.status();
        return ok(serde_json::to_value(&status).unwrap_or_default());
    }

    // -- List profiles --
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
                    "priority": p.priority,
                })
            })
            .collect();
        return ok(json!({ "profiles": list }));
    }

    // -- Open batch (new) --
    if url == "/api/v1/profiles/open-batch" && method == Method::Post {
        let body = body_to_json(req);
        let profile_ids: Vec<String> = body
            .as_ref()
            .and_then(|b| b.get("profile_ids"))
            .and_then(|ids| ids.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let navigate_to = body
            .as_ref()
            .and_then(|b| b.get("navigate_to"))
            .and_then(|u| u.as_str())
            .map(String::from);

        let priority = body
            .as_ref()
            .and_then(|b| b.get("priority"))
            .and_then(|p| p.as_u64())
            .unwrap_or(128) as u8;

        if profile_ids.is_empty() {
            return err(400, "Informe 'profile_ids' (array) no corpo");
        }

        let mut opened = Vec::new();
        let mut queued = Vec::new();
        let mut rejected = Vec::new();

        for pid in &profile_ids {
            match state.orchestrator.check_can_open(pid) {
                Ok(()) => {
                    match state.manager.open(pid, state.registry.clone()) {
                        Ok(port) => {
                            state.orchestrator.record_success(pid);
                            // Navigate if requested
                            if let Some(ref nav) = navigate_to {
                                let nav_target = nav.clone();
                                let nav_port = port;
                                std::thread::spawn(move || {
                                    let _ = crate::cdp::navigate(nav_port, &nav_target);
                                });
                            }
                            opened.push(json!({
                                "id": pid,
                                "cdpPort": port,
                                "wsUrl": state.registry.get_ws_url(pid),
                            }));
                        }
                        Err(e) => {
                            state.orchestrator.record_failure(pid);
                            rejected.push(json!({
                                "id": pid,
                                "error": e,
                                "reason": "open_failed",
                            }));
                        }
                    }
                }
                Err((ReasonCode::LimitExceeded, _)) => {
                    if let Some(pos) = state.orchestrator.enqueue_open_request(pid) {
                        queued.push(json!({
                            "id": pid,
                            "position": pos,
                            "etaMs": pos as u64 * 10000,
                        }));
                    } else {
                        rejected.push(json!({
                            "id": pid,
                            "error": "queue_disabled",
                            "reason": "limit_exceeded",
                        }));
                    }
                }
                Err((reason, msg)) => {
                    rejected.push(json!({
                        "id": pid,
                        "error": msg,
                        "reason": reason,
                    }));
                }
            }
        }

        return ok(json!({
            "opened": opened,
            "queued": queued,
            "rejected": rejected,
            "resource_warnings": serde_json::Value::Array(vec![]),
        }));
    }

    // -- Open all (DEPRECATED) --
    if url == "/api/v1/profiles/open-all" && method == Method::Post {
        let profiles = state.manager.list();
        let mut opened = Vec::new();
        let mut skipped = Vec::new();

        for p in profiles {
            if p.status != "available" {
                continue;
            }
            match state.orchestrator.check_can_open(&p.id) {
                Ok(()) => match state.manager.open(&p.id, state.registry.clone()) {
                    Ok(port) => {
                        state.orchestrator.record_success(&p.id);
                        opened.push(json!({ "id": p.id, "name": p.name, "cdpPort": port }))
                    }
                    Err(e) => {
                        state.orchestrator.record_failure(&p.id);
                        skipped.push(json!({ "id": p.id, "name": p.name, "error": e }))
                    }
                },
                Err((reason, msg)) => {
                    skipped.push(json!({
                        "id": p.id,
                        "name": p.name,
                        "error": msg,
                        "reason": reason,
                    }));
                }
            }
        }

        return ok_deprecated(json!({
            "opened": opened,
            "skipped": skipped,
            "deprecated": true,
            "message": "Use POST /api/v1/profiles/open-batch instead",
        }));
    }

    // -- Open all and navigate (DEPRECATED) --
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
        let mut skipped = Vec::new();

        for p in profiles {
            if p.status != "available" {
                continue;
            }
            match state.orchestrator.check_can_open(&p.id) {
                Ok(()) => match state.manager.open(&p.id, state.registry.clone()) {
                    Ok(port) => {
                        state.orchestrator.record_success(&p.id);
                        let nav_target = target.clone();
                        std::thread::spawn(move || {
                            let _ = crate::cdp::navigate(port, &nav_target);
                        });
                        results.push(json!({
                            "id": p.id, "name": p.name,
                            "cdpPort": port, "navigatingTo": target
                        }));
                    }
                    Err(e) => {
                        state.orchestrator.record_failure(&p.id);
                        skipped.push(json!({ "id": p.id, "name": p.name, "error": e }))
                    }
                },
                Err((reason, msg)) => {
                    skipped.push(json!({
                        "id": p.id,
                        "name": p.name,
                        "error": msg,
                        "reason": reason,
                    }));
                }
            }
        }

        return ok_deprecated(json!({
            "openedAndNavigating": results,
            "skipped": skipped,
            "deprecated": true,
            "message": "Use POST /api/v1/profiles/open-batch with navigate_to instead",
        }));
    }

    // -- Profile-specific routes --
    if let Some(id) = url.strip_prefix("/api/v1/profiles/") {
        if let Some(profile_id) = id.strip_suffix("/open") {
            if method == Method::Post {
                // Check orchestrator limits
                if let Err((reason, msg)) = state.orchestrator.check_can_open(profile_id) {
                    return match reason {
                        ReasonCode::LimitExceeded => {
                            let active = state.registry.ids();
                            limit_exceeded_error(state.orchestrator.config.max_active_profiles, active)
                        }
                        ReasonCode::ResourceExhausted => resource_exhausted_error(&msg),
                        ReasonCode::CircuitBreakerOpen => {
                            err_with_reason(503, "profile_cooldown", reason, None)
                        }
                        _ => err(500, &msg),
                    };
                }

                return match state.manager.open(profile_id, state.registry.clone()) {
                    Ok(port) => {
                        state.orchestrator.record_success(profile_id);
                        ok(
                            json!({ "id": profile_id, "cdpPort": port, "wsUrl": state.registry.get_ws_url(profile_id) }),
                        )
                    }
                    Err(e) => {
                        state.orchestrator.record_failure(profile_id);
                        err(500, &e)
                    }
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
