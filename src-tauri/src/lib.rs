pub mod search;
pub mod shortcuts;
pub mod tray;
pub mod window;

use tauri::{AppHandle, Manager, RunEvent, State, WindowEvent};
use window::{LauncherState, LauncherStatus};

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
            reveal_item_in_folder
        ])
        .setup(|app| {
            let handle = app.handle();

            let native_glass_supported = app
                .get_webview_window(window::MAIN_WINDOW_LABEL)
                .as_ref()
                .map(window::configure_native_backdrop)
                .unwrap_or(false);

            let configured_shortcut = shortcuts::resolve_configured_shortcut();
            let shortcut_status =
                shortcuts::register_launcher_shortcut(handle, &configured_shortcut);

            let tray_ok = match tray::setup_tray(handle) {
                Ok(()) => true,
                Err(err) => {
                    eprintln!("Warning: {err}");
                    false
                }
            };

            app.manage(LauncherState::new(
                shortcut_status.clone(),
                native_glass_supported,
            ));

            let search_state = search::SearchState::new();
            app.manage(search_state);
            search::start_background_indexing(handle.clone());

            if (should_show_on_startup() || (!shortcut_status.registered && !tray_ok))
                && let Err(err) = window::show_launcher_window(handle)
            {
                eprintln!("Warning: failed to show launcher window on startup: {err}");
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

    app.run(|app_handle, event| match event {
        RunEvent::ExitRequested { api, code, .. } => {
            let is_quitting = app_handle
                .try_state::<LauncherState>()
                .map(|s| s.is_quitting())
                .unwrap_or(false);

            if code.is_none() && !is_quitting {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => {
            shortcuts::unregister_all_shortcuts(app_handle);
        }
        _ => {}
    });
}
