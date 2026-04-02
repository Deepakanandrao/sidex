use std::collections::HashMap;

fn is_ssrf_blocked(url_str: &str) -> bool {
    let lower = url_str.to_lowercase();
    let after_scheme = if let Some(pos) = lower.find("://") {
        &lower[pos + 3..]
    } else {
        return true;
    };
    let host_end = after_scheme
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(after_scheme.len());
    let host_with_port = &after_scheme[..host_end];
    let host = if host_with_port.starts_with('[') {
        &host_with_port[1..host_with_port.rfind(']').unwrap_or(host_with_port.len())]
    } else if let Some(colon_pos) = host_with_port.rfind(':') {
        &host_with_port[..colon_pos]
    } else {
        host_with_port
    };
    if host.is_empty() {
        return true;
    }
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    if let Ok(addr) = host.parse::<std::net::IpAddr>() {
        return match addr {
            std::net::IpAddr::V4(v4) => {
                v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
            }
            std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
        };
    }
    false
}

#[tauri::command]
pub async fn fetch_url(url: String) -> Result<Vec<u8>, String> {
    if is_ssrf_blocked(&url) {
        return Err("fetch blocked: internal/private addresses are not allowed".to_string());
    }
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("fetch failed: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("read failed: {}", e))?;

    Ok(bytes.to_vec())
}

#[tauri::command]
pub async fn fetch_url_text(url: String) -> Result<String, String> {
    if is_ssrf_blocked(&url) {
        return Err("fetch blocked: internal/private addresses are not allowed".to_string());
    }
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("fetch failed: {}", e))?;

    response
        .text()
        .await
        .map_err(|e| format!("read failed: {}", e))
}

#[tauri::command]
pub async fn proxy_request(
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body: Option<String>,
) -> Result<String, String> {
    if is_ssrf_blocked(&url) {
        return Err("fetch blocked: internal/private addresses are not allowed".to_string());
    }
    let client = reqwest::Client::new();
    
    let mut req = match method.to_uppercase().as_str() {
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        _ => client.get(&url),
    };

    for (key, value) in &headers {
        req = req.header(key.as_str(), value.as_str());
    }

    if let Some(b) = body {
        req = req.body(b);
    }

    let response = req
        .send()
        .await
        .map_err(|e| format!("proxy request failed: {}", e))?;

    response
        .text()
        .await
        .map_err(|e| format!("proxy read failed: {}", e))
}
