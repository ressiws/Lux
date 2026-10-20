pub mod clipboard;
pub mod search;
pub mod shortcuts;
pub mod tray;
pub mod window;

use tauri::{AppHandle, Manager, RunEvent, State, WindowEvent};
use window::{LauncherState, LauncherStatus};

use clipboard::{
    clear_clipboard_history, copy_clipboard_entry, delete_clipboard_entry, get_clipboard_history,
    toggle_pin_clipboard_entry,
};
use shortcuts::{get_shortcuts_status, update_shortcuts};

#[tauri::command]
fn hide_launcher(app: AppHandle) -> Result<(), String> {
    window::hide_launcher_window(&app)
}

#[tauri::command]
fn get_launcher_status(state: State<'_, LauncherState>) -> LauncherStatus {
    state.status()
}

#[tauri::command]
fn search_launcher(
    query: String,
    state: State<'_, search::SearchState>,
) -> search::SearchResponse {
    let (apps, files) = match state.index.read() {
        Ok(guard) => search::search_index(&guard, &query),
        Err(poisoned) => search::search_index(&poisoned.into_inner(), &query),
    };
    search::SearchResponse { apps, files }
}

#[tauri::command]
fn open_target(path: String) -> Result<(), String> {
    search::launch_target(&path)
}

#[tauri::command]
fn open_target_as_admin(path: String) -> Result<(), String> {
    search::launch_target_as_admin(&path)
}

#[tauri::command]
fn reveal_item_in_folder(path: String) -> Result<(), String> {
    search::reveal_in_folder(&path)
}

#[tauri::command]
fn set_launcher_height(expanded: bool, app: AppHandle) -> Result<(), String> {
    window::set_launcher_height(&app, expanded)
}

#[tauri::command]
fn set_native_glass(enable: bool, app: AppHandle) -> Result<(), String> {
    window::set_native_glass(&app, enable)
}

#[tauri::command]
fn reindex(app: AppHandle) -> Result<(), String> {
    search::start_background_indexing(app);
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexStats {
    total_apps: usize,
    total_files: usize,
    is_indexed: bool,
}

#[tauri::command]
fn get_index_stats(state: State<'_, search::SearchState>) -> IndexStats {
    let guard = state.index.read().unwrap_or_else(|p| p.into_inner());
    IndexStats {
        total_apps: guard.apps.len(),
        total_files: guard.files.len(),
        is_indexed: guard.is_indexed,
    }
}

#[tauri::command]
fn set_window_height(app: AppHandle, height: f64) -> Result<(), String> {
    window::set_window_target_height(&app, height)
}

#[tauri::command]
fn notify_frontend_ready(app: AppHandle, state: State<'_, LauncherState>) -> Result<(), String> {
    state.mark_frontend_ready();
    if state.should_show_on_ready() {
        state.set_show_on_ready(false);
        window::show_launcher_window(&app)?;
    }
    Ok(())
}

fn should_show_on_startup() -> bool {
    if std::env::args().any(|arg| arg == "--show") {
        return true;
    }
    matches!(
        std::env::var("LUX_SHOW_ON_STARTUP").as_deref(),
        Ok("1" | "true" | "TRUE")
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let build_result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            hide_launcher,
            get_launcher_status,
            search_launcher,
            open_target,
            open_target_as_admin,
            reveal_item_in_folder,
            set_launcher_height,
            set_window_height,
            set_native_glass,
            reindex,
            get_index_stats,
            update_shortcuts,
            get_shortcuts_status,
            get_clipboard_history,
            copy_clipboard_entry,
            delete_clipboard_entry,
            clear_clipboard_history,
            toggle_pin_clipboard_entry,
            notify_frontend_ready
        ])
        .setup(|app| {
            let handle = app.handle();

            let native_glass_supported = app
                .get_webview_window(window::MAIN_WINDOW_LABEL)
                .as_ref()
                .map(window::configure_native_backdrop)
                .unwrap_or(false);

            let (configured_shortcut, configured_clipboard_shortcut) =
                shortcuts::resolve_configured_shortcuts(handle);
            let shortcut_config = shortcuts::register_all_shortcuts(
                handle,
                &configured_shortcut,
                &configured_clipboard_shortcut,
            );

            let tray_ok = match tray::setup_tray(handle) {
                Ok(()) => true,
                Err(err) => {
                    eprintln!("Warning: {err}");
                    false
                }
            };

            app.manage(LauncherState::new(
                shortcut_config.launcher.clone(),
                native_glass_supported,
            ));

            app.manage(shortcuts::ShortcutsState::new(
                shortcut_config.launcher.clone(),
                shortcut_config.clipboard.clone(),
            ));

            let clipboard_state = clipboard::ClipboardState::new(clipboard::DEFAULT_MAX_CLIPBOARD_ENTRIES);
            let clipboard_history_arc = clipboard_state.history.clone();
            app.manage(clipboard_state);
            clipboard::start_clipboard_monitor(handle.clone(), clipboard_history_arc);

            let search_state = search::SearchState::new();
            app.manage(search_state);
            search::start_background_indexing(handle.clone());

            if (should_show_on_startup() || (!shortcut_config.launcher.registered && !tray_ok))
                && let Some(state) = handle.try_state::<LauncherState>()
            {
                if state.is_frontend_ready() {
                    if let Err(err) = window::show_launcher_window(handle) {
                        eprintln!("Warning: failed to show launcher window on startup: {err}");
                    }
                } else {
                    state.set_show_on_ready(true);
                }
            }

            Ok(())
        })
        .on_window_event(|win, event| {
            if win.label() != window::MAIN_WINDOW_LABEL {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let is_quitting = win
                        .app_handle()
                        .try_state::<LauncherState>()
                        .map(|s| s.is_quitting())
                        .unwrap_or(false);

                    if !is_quitting {
                        api.prevent_close();
                        if let Err(err) = window::hide_launcher_window(win.app_handle()) {
                            eprintln!("Warning: failed to hide launcher on close request: {err}");
                        }
                    }
                }
                WindowEvent::Focused(focused) => {
                    window::handle_window_focus_change(win.app_handle(), *focused);
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!());

    let app = match build_result {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Error while building Lux application: {err}");
            return;
        }
    };

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            shortcuts::unregister_all_shortcuts(app_handle);
        }
    });
}
