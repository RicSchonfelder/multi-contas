use crate::profile::FingerprintConfig;
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

pub fn get_ws_url(port: u16) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{}/json", port);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match reqwest::blocking::get(&url) {
            Ok(resp) => {
                if let Ok(targets) = resp.json::<Vec<Value>>() {
                    for t in &targets {
                        if t["type"] == "page" {
                            return Ok(t["webSocketDebuggerUrl"]
                                .as_str()
                                .unwrap_or("")
                                .to_string());
                        }
                    }
                    if let Some(t) = targets.first() {
                        return Ok(t["webSocketDebuggerUrl"]
                            .as_str()
                            .map(String::from)
                            .unwrap_or_default());
                    }
                }
            }
            Err(_) => {}
        }
        if std::time::Instant::now() >= deadline {
            return Err("CDP timeout after 10s".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn send_cmd(
    ws: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    id: i64,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let msg = json!({"id": id, "method": method, "params": params});
    ws.send(Message::text(msg.to_string()))
        .map_err(|e| format!("CDP send: {}", e))?;
    loop {
        match ws.read() {
            Ok(Message::Text(t)) => {
                if let Ok(v) = serde_json::from_str::<Value>(&t) {
                    if v["id"] == json!(id) {
                        return Ok(v["result"].clone());
                    }
                }
            }
            Ok(Message::Binary(_)) => continue,
            Err(e) => return Err(format!("CDP read: {}", e)),
            _ => continue,
        }
    }
}

fn wait_for(
    ws: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    event: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        match ws.read() {
            Ok(Message::Text(t)) => {
                if let Ok(v) = serde_json::from_str::<Value>(&t) {
                    if v["method"] == event {
                        return Ok(v["params"].clone());
                    }
                }
            }
            Ok(Message::Binary(_)) => continue,
            Err(_) => return Err("CDP wait timeout".into()),
            _ => continue,
        }
    }
    Err("CDP wait timeout".into())
}

pub fn apply_fingerprint(port: u16, fp: &FingerprintConfig) -> Result<(), String> {
    std::thread::sleep(Duration::from_millis(1200));
    let ws_url = get_ws_url(port)?;
    let parsed = url::Url::parse(&ws_url).map_err(|e| format!("URL parse: {}", e))?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port2 = parsed.port().unwrap_or(port);
    let path = parsed.path();
    let addr = format!("ws://{}:{}{}", host, port2, path);
    let mut ws = tungstenite::connect(&addr)
        .map_err(|e| format!("CDP WS: {}", e))?
        .0;

    send_cmd(&mut ws, 1, "Page.enable", json!({}))?;

    // Build spoofing script
    let ua = if fp.user_agent.is_empty() {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36".to_string()
    } else {
        fp.user_agent.clone()
    };
    let platform = if fp.platform.is_empty() {
        "Win32".to_string()
    } else {
        fp.platform.clone()
    };
    let lang = if fp.language.is_empty() {
        "pt-BR".to_string()
    } else {
        fp.language.clone()
    };
    let resolution = if fp.resolution.is_empty() {
        "1920x1080".to_string()
    } else {
        fp.resolution.clone()
    };
    let vendor = if fp.webgl_vendor.is_empty() {
        "Google Inc. (Intel)".to_string()
    } else {
        fp.webgl_vendor.clone()
    };
    let renderer = if fp.webgl_renderer.is_empty() {
        "ANGLE (Intel, Intel(R) UHD Graphics Direct3D11 vs_5_0 ps_5_0)".to_string()
    } else {
        fp.webgl_renderer.clone()
    };
    let tz = if fp.timezone.is_empty() {
        "America/Sao_Paulo".to_string()
    } else {
        fp.timezone.clone()
    };
    let geo = if fp.geolocation.is_empty() {
        "-23.5505,-46.6333".to_string()
    } else {
        fp.geolocation.clone()
    };
    let fonts_json = if fp.fonts.is_empty() {
        json!([
            "Arial",
            "Courier New",
            "Georgia",
            "Times New Roman",
            "Verdana",
            "Segoe UI",
            "Roboto",
            "Open Sans"
        ])
    } else {
        json!(fp.fonts)
    };

    let spoof_js = format!(
        r#"
(function() {{
    const UA = {ua_json};
    const PLATFORM = {platform_json};
    const LANG = {lang_json};
    const RES = {resolution_json};
    const VENDOR = {vendor_json};
    const RENDERER = {renderer_json};
    const TZ = {tz_json};
    const GEO = {geo_json};
    const HARDWARE_CONCURRENCY = {hc};
    const DEVICE_MEMORY = {dm};
    const FONTS = {fonts_json};
    const CANVAS_NOISE = {canvas};
    const AUDIO_NOISE = {audio};

    // 1. Navigator overrides
    Object.defineProperty(navigator, 'userAgent', {{ get: () => UA, configurable: false }});
    Object.defineProperty(navigator, 'platform', {{ get: () => PLATFORM, configurable: false }});
    Object.defineProperty(navigator, 'language', {{ get: () => LANG, configurable: false }});
    Object.defineProperty(navigator, 'languages', {{ get: () => [LANG, 'en-US', 'en'], configurable: false }});
    Object.defineProperty(navigator, 'hardwareConcurrency', {{ get: () => HARDWARE_CONCURRENCY, configurable: false }});
    Object.defineProperty(navigator, 'deviceMemory', {{ get: () => DEVICE_MEMORY, configurable: false }});
    Object.defineProperty(navigator, 'maxTouchPoints', {{ get: () => 0, configurable: false }});

    // 2. Remove webdriver
    Object.defineProperty(navigator, 'webdriver', {{ get: () => false, configurable: false }});

    // 3. WebGL spoof
    const getParameterOrig = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = function(p) {{
        if (p === 37445) return VENDOR;  // UNMASKED_VENDOR_WEBGL
        if (p === 37446) return RENDERER;  // UNMASKED_RENDERER_WEBGL
        return getParameterOrig.call(this, p);
    }};
    const getParameterOrig2 = WebGL2RenderingContext.prototype.getParameter;
    WebGL2RenderingContext.prototype.getParameter = function(p) {{
        if (p === 37445) return VENDOR;
        if (p === 37446) return RENDERER;
        return getParameterOrig2.call(this, p);
    }};

    // 4. Canvas noise
    if (CANVAS_NOISE) {{
        const toDataURLOrig = HTMLCanvasElement.prototype.toDataURL;
        HTMLCanvasElement.prototype.toDataURL = function() {{
            const r = toDataURLOrig.apply(this, arguments);
            return r.substring(0, r.length - 8) + Math.random().toString(36).substring(2, 10);
        }};
        const toBlobOrig = HTMLCanvasElement.prototype.toBlob;
        HTMLCanvasElement.prototype.toBlob = function(cb, type, quality) {{
            toBlobOrig.call(this, function(b) {{
                if (b) {{
                    const arr = new Uint8Array(await b.arrayBuffer());
                    arr[arr.length-1] ^= 1;
                    cb(new Blob([arr], {{type: b.type}}));
                }} else {{ cb(b); }}
            }}, type, quality);
        }};
        const getImageDataOrig = CanvasRenderingContext2D.prototype.getImageData;
        CanvasRenderingContext2D.prototype.getImageData = function(x, y, w, h) {{
            const data = getImageDataOrig.call(this, x, y, w, h);
            for (let i = 0; i < data.data.length; i += 4) {{
                data.data[i] ^= 1; data.data[i+1] ^= 1; data.data[i+2] ^= 1;
            }}
            return data;
        }};
    }}

    // 5. Audio noise
    if (AUDIO_NOISE) {{
        const getChannelDataOrig = AudioBuffer.prototype.getChannelData;
        AudioBuffer.prototype.getChannelData = function(c) {{
            const data = getChannelDataOrig.call(this, c);
            const noise = new Float32Array(data.length);
            for (let i = 0; i < data.length; i += 100) noise[i] = data[i] * 0.0001;
            return noise;
        }};
    }}

    // 6. Screen
    const parts = RES.split('x').map(Number);
    if (parts.length === 2) {{
        const sw = parts[0], sh = parts[1];
        Object.defineProperty(screen, 'width', {{ get: () => sw, configurable: false }});
        Object.defineProperty(screen, 'height', {{ get: () => sh, configurable: false }});
        Object.defineProperty(screen, 'availWidth', {{ get: () => sw, configurable: false }});
        Object.defineProperty(screen, 'availHeight', {{ get: () => sh-40, configurable: false }});
        Object.defineProperty(screen, 'colorDepth', {{ get: () => 24, configurable: false }});
        Object.defineProperty(screen, 'pixelDepth', {{ get: () => 24, configurable: false }});
    }}

    // 7. Timezone via Intl
    try {{
        const dtf = Intl.DateTimeFormat;
        Intl.DateTimeFormat = function(locale, opts) {{
            return new dtf(TZ.replace('_','/'), opts);
        }};
    }} catch(e){{}}

    // 8. Geolocation
    if (GEO) {{
        const parts2 = GEO.split(',').map(Number);
        if (parts2.length === 2) {{
            const geo = navigator.geolocation;
            geo.getCurrentPosition = function(success, error, opts) {{
                success({{ coords: {{ latitude: parts2[0], longitude: parts2[1], accuracy: 100, altitude: null, altitudeAccuracy: null, heading: null, speed: null }}, timestamp: Date.now() }});
            }};
            geo.watchPosition = function(success, error, opts) {{
                success({{ coords: {{ latitude: parts2[0], longitude: parts2[1], accuracy: 100, altitude: null, altitudeAccuracy: null, heading: null, speed: null }}, timestamp: Date.now() }});
                return 0;
            }};
        }}
    }}

    // 9. Fonts spoofing (override enumerable)
    try {{
        if (document.fonts && document.fonts.ready) {{
            document.fonts.ready.then(function() {{
                const originalCheck = document.fonts.check;
                document.fonts.check = function(font) {{ return true; }};
            }});
        }}
    }} catch(e){{}}

    // 10. Plugins
    Object.defineProperty(navigator, 'plugins', {{ get: () => [1,2,3,4,5], configurable: false }});
    Object.defineProperty(navigator, 'mimeTypes', {{ get: () => [1,2,3,4], configurable: false }});
}})();
"#,
        ua_json = serde_json::to_string(&ua).unwrap_or_default(),
        platform_json = serde_json::to_string(&platform).unwrap_or_default(),
        lang_json = serde_json::to_string(&lang).unwrap_or_default(),
        resolution_json = serde_json::to_string(&resolution).unwrap_or_default(),
        vendor_json = serde_json::to_string(&vendor).unwrap_or_default(),
        renderer_json = serde_json::to_string(&renderer).unwrap_or_default(),
        tz_json = serde_json::to_string(&tz).unwrap_or_default(),
        geo_json = serde_json::to_string(&geo).unwrap_or_default(),
        hc = fp.hardware_concurrency,
        dm = fp.device_memory,
        fonts_json = serde_json::to_string(&fonts_json).unwrap_or_default(),
        canvas = if fp.canvas_noise { "true" } else { "false" },
        audio = if fp.audio_noise { "true" } else { "false" },
    );

    send_cmd(
        &mut ws,
        2,
        "Page.addScriptToEvaluateOnNewDocument",
        json!({"source": &spoof_js}),
    )?;

    Ok(())
}

pub fn export_cookies(port: u16, profile_id: &str) -> Result<String, String> {
    std::thread::sleep(Duration::from_millis(500));
    let ws_url = get_ws_url(port)?;
    let parsed = url::Url::parse(&ws_url).map_err(|e| format!("URL parse: {}", e))?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port2 = parsed.port().unwrap_or(port);
    let path = parsed.path();
    let addr = format!("ws://{}:{}{}", host, port2, path);
    let mut ws = tungstenite::connect(&addr)
        .map_err(|e| format!("CDP WS: {}", e))?
        .0;

    send_cmd(&mut ws, 1, "Network.enable", json!({}))?;

    let result = send_cmd(&mut ws, 2, "Network.getAllCookies", json!({}))?;
    let cookies = result["cookies"].as_array().cloned().unwrap_or_default();

    let output = json!({
        "cookies": cookies,
        "exportedAt": "2026-07-13T00:00:00Z"
    });

    let local = env::var("LOCALAPPDATA").unwrap_or_else(|_| "C:".to_string());
    let dir = Path::new(&local)
        .join("MultiContas")
        .join("profiles")
        .join(profile_id);
    fs::create_dir_all(&dir).map_err(|e| format!("Dir create: {}", e))?;
    let file_path = dir.join("cookies.json");
    fs::write(
        &file_path,
        serde_json::to_string_pretty(&output).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("File write: {}", e))?;

    Ok(serde_json::to_string(&cookies).map_err(|e| e.to_string())?)
}

pub fn import_cookies(port: u16, cookies_json: &str) -> Result<(), String> {
    std::thread::sleep(Duration::from_millis(500));
    let ws_url = get_ws_url(port)?;
    let parsed = url::Url::parse(&ws_url).map_err(|e| format!("URL parse: {}", e))?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port2 = parsed.port().unwrap_or(port);
    let path = parsed.path();
    let addr = format!("ws://{}:{}{}", host, port2, path);
    let mut ws = tungstenite::connect(&addr)
        .map_err(|e| format!("CDP WS: {}", e))?
        .0;

    send_cmd(&mut ws, 1, "Network.enable", json!({}))?;

    let cookies: Vec<Value> =
        serde_json::from_str(cookies_json).map_err(|e| format!("JSON parse: {}", e))?;

    for (i, cookie) in cookies.iter().enumerate() {
        let name = cookie["name"].as_str().unwrap_or("");
        let value = cookie["value"].as_str().unwrap_or("");
        let domain = cookie["domain"].as_str().unwrap_or("");
        let cpath = cookie["path"].as_str().unwrap_or("/");
        let secure = cookie
            .get("secure")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let http_only = cookie
            .get("httpOnly")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let same_site = cookie
            .get("sameSite")
            .and_then(|v| v.as_str())
            .unwrap_or("None");
        let expires = cookie
            .get("expires")
            .or_else(|| cookie.get("expiry"))
            .and_then(|v| v.as_f64())
            .unwrap_or(-1.0);

        let mut params = json!({
            "name": name,
            "value": value,
            "domain": domain,
            "path": cpath,
            "secure": secure,
            "httpOnly": http_only,
            "sameSite": same_site,
        });
        if expires > 0.0 {
            params["expires"] = json!(expires);
        }

        send_cmd(&mut ws, (i + 2) as i64, "Network.setCookie", params)?;
    }

    Ok(())
}

pub fn auto_login(port: u16, email: &str, password: &str, url: &str) -> Result<(), String> {
    std::thread::sleep(Duration::from_millis(1500));

    let ws_url = get_ws_url(port)?;
    let parsed = url::Url::parse(&ws_url).map_err(|e| format!("URL parse: {}", e))?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port2 = parsed.port().unwrap_or(port);
    let path = parsed.path();
    let addr = format!("ws://{}:{}{}", host, port2, path);

    let mut ws = tungstenite::connect(&addr)
        .map_err(|e| format!("CDP WS: {}", e))?
        .0;

    // Navigate
    send_cmd(&mut ws, 1, "Page.enable", json!({}))?;

    let nav_url = if url.is_empty() {
        "https://accounts.google.com"
    } else {
        url
    };
    send_cmd(&mut ws, 2, "Page.navigate", json!({"url": nav_url}))?;

    // Wait for page load
    wait_for(&mut ws, "Page.loadEventFired", Duration::from_secs(15))?;

    // Inject and run auto-fill script
    let js = format!(
        r#"
(async () => {{
    const wait = (ms) => new Promise(r => setTimeout(r, ms));
    const findVisible = (sel, root = document) => {{
        const el = root.querySelector(sel);
        if (el) {{
            const rect = el.getBoundingClientRect();
            return rect.width > 0 && rect.height > 0 ? el : null;
        }}
        return null;
    }};
    const typeText = (field, text) => {{
        field.focus();
        field.select();
        const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
        nativeInputValueSetter.call(field, '');
        field.dispatchEvent(new Event('input', {{ bubbles: true }}));
        field.dispatchEvent(new Event('change', {{ bubbles: true }}));
        for (const ch of text) {{
            nativeInputValueSetter.call(field, field.value + ch);
            field.dispatchEvent(new Event('input', {{ bubbles: true }}));
            field.dispatchEvent(new Event('keydown', {{ bubbles: true }}));
            field.dispatchEvent(new Event('keyup', {{ bubbles: true }}));
        }}
        field.dispatchEvent(new Event('change', {{ bubbles: true }}));
    }};

    // Step 1: Fill email
    const emailField = findVisible('input[type="email"]') || findVisible('#identifierId') || findVisible('input[name="identifier"]');
    if (emailField) {{
        typeText(emailField, {email_json});
        await wait(1000);
        const nextBtn = document.querySelector('#identifierNext') || document.querySelector('[jscontroller="soHxf"]') || document.querySelector('button[jsname="V67aGc"]') || document.querySelector('span[jsname="V67aGc"]');
        if (nextBtn) {{ nextBtn.click(); }}
        else {{ 
            const form = emailField.closest('form');
            if (form) {{ 
                const ev = new KeyboardEvent('keydown', {{ key: 'Enter', keyCode: 13, bubbles: true }});
                emailField.dispatchEvent(ev);
            }}
        }}
    }}

    // Step 2: Wait for password page and fill
    for (let attempts = 0; attempts < 30; attempts++) {{
        await wait(1000);
        const passField = findVisible('input[type="password"]') || findVisible('input[name="Passwd"]') || findVisible('#Passwd');
        if (passField) {{
            typeText(passField, {pass_json});
            await wait(1000);
            const passBtn = document.querySelector('#passwordNext') || document.querySelector('button[jsname="V67aGc"]') || document.querySelector('[jscontroller="soHxf"]');
            if (passBtn) passBtn.click();
            break;
        }}
    }}
}})();
"#,
        email_json = serde_json::to_string(email).unwrap_or_default(),
        pass_json = serde_json::to_string(password).unwrap_or_default(),
    );

    send_cmd(
        &mut ws,
        3,
        "Runtime.evaluate",
        json!({
            "expression": &js,
            "awaitPromise": true,
            "userGesture": true
        }),
    )?;

    Ok(())
}

/// Navega o perfil (porta CDP) para uma URL.
pub fn navigate(port: u16, url: &str) -> Result<(), String> {
    let ws_url = get_ws_url(port)?;
    let parsed = url::Url::parse(&ws_url).map_err(|e| format!("URL parse: {}", e))?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port2 = parsed.port().unwrap_or(port);
    let path = parsed.path();
    let addr = format!("ws://{}:{}{}", host, port2, path);
    let mut ws = tungstenite::connect(&addr)
        .map_err(|e| format!("CDP WS: {}", e))?
        .0;

    send_cmd(&mut ws, 1, "Page.enable", json!({}))?;
    send_cmd(&mut ws, 2, "Page.navigate", json!({ "url": url }))?;
    // aguarda o carregamento (best-effort)
    let _ = wait_for(&mut ws, "Page.loadEventFired", Duration::from_secs(20));
    Ok(())
}

/// Envia um comentário no YouTube a partir deste perfil (porta CDP).
/// Tenta primeiro o live-chat (transmissão ao vivo) e, se não achar,
/// cai na caixa de comentários normal do vídeo.
pub fn send_comment(port: u16, text: &str) -> Result<(), String> {
    let ws_url = get_ws_url(port)?;
    let parsed = url::Url::parse(&ws_url).map_err(|e| format!("URL parse: {}", e))?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port2 = parsed.port().unwrap_or(port);
    let path = parsed.path();
    let addr = format!("ws://{}:{}{}", host, port2, path);
    let mut ws = tungstenite::connect(&addr)
        .map_err(|e| format!("CDP WS: {}", e))?
        .0;

    send_cmd(&mut ws, 1, "Page.enable", json!({}))?;

    let txt_json = serde_json::to_string(text).unwrap_or_default();
    let js = format!(
        r#"
(async () => {{
    const wait = (ms) => new Promise(r => setTimeout(r, ms));
    const SELS = [
        '#live-chat-input .yt-live-chat-text-input-field-renderer',
        '#input.yt-live-chat-text-input-field-renderer',
        '#contenteditable-root',          // live-chat (Polymer)
        'yt-formatted-string#contenteditable-root',
        '#stamina-comment-text-input',    // fallback comment box
        'ytd-commentbox #contenteditable-root',
        '#contenteditable-textarea'
    ];
    let el = null;
    for (const s of SELS) {{
        const found = document.querySelector(s);
        if (found && found.offsetParent !== null) {{ el = found; break; }}
    }}
    if (!el) return "NO_INPUT";

    el.focus();
    // insere o texto respeitando o React/Polymer
    const setter = Object.getOwnPropertyDescriptor(window.HTMLElement.prototype, 'innerText');
    if (setter && setter.set) setter.set.call(el, {text});
    else el.innerText = {text};

    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
    await wait(400);

    // tenta enviar: Enter (live-chat) ou botão
    const sendBtns = [
        '#send-button button',
        'button[aria-label="Enviar"]',
        'yt-button-renderer#send-button button',
        'ytd-button-renderer#submit-button button'
    ];
    for (const b of sendBtns) {{
        const btn = document.querySelector(b);
        if (btn && btn.offsetParent !== null) {{ btn.click(); return "SENT_BUTTON"; }}
    }}
    const ev = new KeyboardEvent('keydown', {{ key: 'Enter', keyCode: 13, bubbles: true, cancelable: true }});
    el.dispatchEvent(ev);
    return "SENT_ENTER";
}})();
"#,
        text = txt_json
    );

    let result = send_cmd(
        &mut ws,
        2,
        "Runtime.evaluate",
        json!({
            "expression": &js,
            "awaitPromise": true,
            "returnByValue": true,
            "userGesture": true
        }),
    )?;

    let outcome = result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");

    if outcome == "NO_INPUT" {
        return Err(
            "Caixa de comentário não encontrada (vídeo pode não ter iniciado ou não é comentável)"
                .into(),
        );
    }
    Ok(())
}
