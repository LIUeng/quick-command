mod commands;
mod errors;
mod models;
mod parser;
mod search;
mod store;

use store::Store;
use tauri::{Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

fn toggle_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn show_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed { toggle_window(app); }
        }).build())
        .setup(|app| {
            let data_path = app.path().app_data_dir()?.join("state.json");
            let store = Store::load(data_path).map_err(std::io::Error::other)?;
            let shortcut = store.data.lock().map_err(|_| std::io::Error::other("state lock failed"))?.settings.shortcut.clone();
            app.global_shortcut().register(shortcut.as_str())?;
            app.manage(store);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Focused(false)) {
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_launcher_state,
            commands::search_projects,
            commands::execute_command,
            commands::create_and_execute,
            commands::save_settings,
            commands::reindex_workspaces,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Quick Command");

    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Reopen { .. }) {
            show_window(app);
        }
    });
}
