// SPDX-FileCopyrightText: 2019-2022, The Tauri Programme in the Commons Conservancy
// SPDX-FileCopyrightText: Copyright (c) 2022-2025 trobonox <hello@trobo.dev>
//
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

/// Verify Nextcloud credentials by calling the OCS user endpoint.
/// Runs in the Rust backend so it is not subject to the webview CSP/CORS.
#[tauri::command]
fn test_nextcloud(url: String, user: String, password: String) -> Result<String, String> {
    let base = url.trim_end_matches('/');
    if base.is_empty() || user.is_empty() || password.is_empty() {
        return Err("server URL, username and app password are required".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/ocs/v2.php/cloud/user?format=json"))
        .basic_auth(&user, Some(&password))
        .header("OCS-APIRequest", "true")
        .send()
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), body));
    }
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("invalid response: {e}"))?;
    let id = v["ocs"]["data"]["id"].as_str().unwrap_or("?");
    Ok(format!("Connected as {id}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/dev/dri").exists()
            && std::env::var("WAYLAND_DISPLAY").is_err()
            && std::env::var("XDG_SESSION_TYPE").unwrap_or_default() == "x11"
        {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![test_nextcloud])
        .setup(|app| {
            #[cfg(desktop)]
            let _ = app.handle().plugin(tauri_plugin_single_instance::init(|app, _args,_cwd| {
                let _ = app.get_webview_window("main")
                       .expect("no main window")
                       .set_focus();
            }));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
