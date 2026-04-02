use serde::Serialize;
use std::env;

#[derive(Debug, Serialize)]
pub struct OsInfo {
    pub platform: String,
    pub arch: String,
    pub hostname: String,
    pub homedir: String,
    pub tmpdir: String,
}

#[tauri::command]
pub fn get_os_info() -> OsInfo {
    OsInfo {
        platform: env::consts::OS.to_string(),
        arch: env::consts::ARCH.to_string(),
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        homedir: dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        tmpdir: env::temp_dir().to_string_lossy().to_string(),
    }
}

#[tauri::command]
pub fn get_env(key: String) -> Option<String> {
    env::var(&key).ok()
}

#[tauri::command]
pub fn get_shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(target_os = "windows") {
            "powershell.exe".to_string()
        } else {
            "/bin/sh".to_string()
        }
    })
}
