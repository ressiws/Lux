pub mod clipboard;
pub mod clipboard_filters;
pub mod color;
pub mod everything;
pub mod features;
pub mod preview;
pub mod search;
pub mod shortcuts;
pub mod tray;
pub mod window;

use tauri::{AppHandle, Manager, RunEvent, State, WindowEvent};
use window::{LauncherState, LauncherStatus};

use clipboard::{
    clear_clipboard_history, clear_clipboard_pins, copy_clipboard_entry, delete_clipboard_entry,
    get_clipboard_history, get_filtered_clipboard_history, toggle_pin_clipboard_entry,
};
use clipboard_filters::{
    get_clipboard_filters_config, save_clipboard_filters_config, validate_clipboard_filter,
};
use features::{get_features_config, set_feature_enabled};
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
    let trimmed = query.trim();

    let apps = match state.index.read() {
        Ok(guard) => search::search_apps(&guard.apps, trimmed),
        Err(poisoned) => search::search_apps(&poisoned.into_inner().apps, trimmed),
    };

    if trimmed.is_empty() {
        return search::SearchResponse {
            apps,
            files: Vec::new(),
            backend: None,
        };
    }

    let everything_result = if state.is_everything_active() {
        let guard = state.everything.lock().ok();
        guard.and_then(|g| {
            g.as_ref().map(|mgr| mgr.search(trimmed, 50))
        })
    } else {
        None
    };

    if let Some(res) = everything_result {
        match res {
            Ok(mut files) => {
                let app_paths: std::collections::HashSet<String> =
                    apps.iter().map(|a| a.path.clone()).collect();
                files.retain(|f| !app_paths.contains(&f.path));
                return search::SearchResponse {
                    apps,
                    files,
                    backend: Some("everything".to_string()),
                };
            }
            Err(err) => {
                eprintln!("[Search Backend] Everything search failed for query '{trimmed}': {err}. Falling back to native search index.");
            }
        }
    }

    let (_, files) = match state.index.read() {
        Ok(guard) => search::search_index(&guard, &query),
        Err(poisoned) => search::search_index(&poisoned.into_inner(), &query),
    };
    search::SearchResponse {
        apps,
        files,
        backend: Some("native".to_string()),
    }
}

#[tauri::command]
fn browse_launcher(
    category: String,
    state: State<'_, search::SearchState>,
) -> search::SearchResponse {
    let (apps, files) = match state.index.read() {
        Ok(guard) => search::browse_index(&guard, &category),
        Err(poisoned) => search::browse_index(&poisoned.into_inner(), &category),
    };
    search::SearchResponse {
        apps,
        files,
        backend: Some("native".to_string()),
    }
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
fn get_image_thumbnail(path: String) -> Option<String> {
    search::get_image_thumbnail(path)
}

#[tauri::command]
fn get_default_browser() -> search::DefaultBrowserInfo {
    search::get_default_browser_info()
}

#[tauri::command]
fn set_launcher_height(
    expanded: bool,
    idle_height: Option<f64>,
    app: AppHandle,
    state: State<'_, LauncherState>,
) -> Result<(), String> {
    if let Some(h) = idle_height {
        state.set_idle_height(h);
    }
    window::set_launcher_height(&app, expanded)
}

#[tauri::command]
fn set_launcher_idle_height(
    height: f64,
    state: State<'_, LauncherState>,
) -> Result<(), String> {
    state.set_idle_height(height);
    Ok(())
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
    is_indexing: bool,
    backend: &'static str,
}

#[tauri::command]
fn get_index_stats(state: State<'_, search::SearchState>) -> IndexStats {
    let guard = state.index.read().unwrap_or_else(|p| p.into_inner());
    let (total_files, backend) = if state.is_everything_active() {
        let count = {
            let everything_guard = state.everything.lock().ok();
            everything_guard
                .and_then(|g| g.as_ref().and_then(|mgr| mgr.get_total_database_items()))
                .unwrap_or(guard.files.len() as u32) as usize
        };
        (count, "everything")
    } else {
        (guard.files.len(), "native")
    };

    IndexStats {
        total_apps: guard.apps.len(),
        total_files,
        is_indexed: guard.is_indexed,
        is_indexing: state.is_indexing(),
        backend,
    }
}

#[tauri::command]
fn set_window_height(app: AppHandle, height: f64) -> Result<(), String> {
    if (height - window::CLIPBOARD_HEIGHT).abs() < 1.0 {
        if let Some(state) = app.try_state::<window::LauncherState>() {
            state.set_mode(window::WindowMode::Clipboard);
        }
    }
    window::set_window_target_height(&app, height)
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    window::show_settings_window(&app)
}

#[tauri::command]
fn hide_settings(app: AppHandle) -> Result<(), String> {
    window::hide_settings_window(&app)
}

#[tauri::command]
fn notify_frontend_ready(
    app: AppHandle,
    state: State<'_, LauncherState>,
    idle_height: Option<f64>,
) -> Result<(), String> {
    if let Some(h) = idle_height {
        state.set_idle_height(h);
    }
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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(settings_win) = app.get_webview_window(window::SETTINGS_WINDOW_LABEL) {
                if settings_win.is_visible().unwrap_or(false) {
                    let _ = settings_win.unminimize();
                    let _ = settings_win.set_focus();
                    return;
                }
            }

            if let Some(state) = app.try_state::<window::LauncherState>() {
                if state.mode() == window::WindowMode::Clipboard {
                    let _ = window::show_launcher_window(app);
                } else {
                    let _ = window::show_launcher_window_explicit(app);
                }
            } else {
                let _ = window::show_launcher_window_explicit(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            hide_launcher,
            get_launcher_status,
            search_launcher,
            browse_launcher,
            open_target,
            open_target_as_admin,
            reveal_item_in_folder,
            set_launcher_height,
            set_launcher_idle_height,
            set_window_height,
            set_native_glass,
            reindex,
            get_index_stats,
            update_shortcuts,
            get_shortcuts_status,
            get_clipboard_history,
            get_filtered_clipboard_history,
            get_clipboard_filters_config,
            save_clipboard_filters_config,
            validate_clipboard_filter,
            copy_clipboard_entry,
            delete_clipboard_entry,
            clear_clipboard_history,
            toggle_pin_clipboard_entry,
            notify_frontend_ready,
            open_settings,
            hide_settings,
            get_features_config,
            set_feature_enabled,
            get_image_thumbnail,
            clear_clipboard_pins,
            get_default_browser
        ])
        .setup(|app| {
            let handle = app.handle();

            let features_config = features::load_or_default_features_config(handle);
            let features_state = features::FeaturesState::new(&features_config);
            app.manage(features_state);

            let native_glass_supported = app
                .get_webview_window(window::MAIN_WINDOW_LABEL)
                .as_ref()
                .map(window::configure_native_backdrop)
                .unwrap_or(false);

            if let Some(settings_win) = app.get_webview_window(window::SETTINGS_WINDOW_LABEL) {
                window::configure_native_backdrop(&settings_win);
            }

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

            let pos_path = window::get_window_position_path(handle);
            let saved_position = pos_path.as_deref().and_then(window::load_persisted_position_from_path);

            app.manage(LauncherState::new(
                shortcut_config.launcher.clone(),
                native_glass_supported,
                saved_position,
            ));

            app.manage(shortcuts::ShortcutsState::new(
                shortcut_config.launcher.clone(),
                shortcut_config.clipboard.clone(),
            ));

            let filters_config = clipboard_filters::load_or_default_filters_config(handle);
            let filters_state = clipboard_filters::ClipboardFiltersState::new(filters_config);
            app.manage(filters_state);

            let clipboard_state = clipboard::ClipboardState::new(clipboard::DEFAULT_MAX_CLIPBOARD_ENTRIES);
            if features_config.clipboard_history {
                clipboard_state.start_monitor(handle.clone());
            }
            app.manage(clipboard_state);

            let search_state = search::SearchState::new();

            // Initialize managed Everything search engine
            let app_local_data = handle
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let resource_dir = handle.path().resource_dir().ok();

            match everything::EverythingManager::new(resource_dir.as_deref(), &app_local_data) {
                Ok(mut mgr) => {
                    if let Err(err) = mgr.start_or_connect() {
                        eprintln!("Warning: Failed to start Everything engine: {err}");
                    }
                    if let Ok(mut g) = search_state.everything.lock() {
                        *g = Some(mgr);
                    }
                }
                Err(err) => {
                    eprintln!("Warning: Failed to initialize Everything manager: {err}");
                }
            }

            let index_path = search::get_search_index_path(handle);
            let home_dir = search::get_user_home_dir();
            let current_fingerprint = search::compute_discovery_fingerprint(&home_dir);

            let mut needs_background_reindex = true;

            if let Some(ref path) = index_path {
                if let Some(persisted) = search::load_persisted_index_from_path(path) {
                    let up_to_date = persisted.is_up_to_date(&current_fingerprint);
                    if let Ok(mut lock) = search_state.index.write() {
                        lock.apps = persisted.apps;
                        lock.files = persisted.files;
                        lock.is_indexed = true;
                    }
                    if up_to_date {
                        needs_background_reindex = false;
                    }
                }
            }

            app.manage(search_state);

            if needs_background_reindex {
                search::start_background_indexing(handle.clone());
            }

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
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let is_quitting = win
                        .app_handle()
                        .try_state::<LauncherState>()
                        .map(|s| s.is_quitting())
                        .unwrap_or(false);

                    if !is_quitting {
                        api.prevent_close();
                        match win.label() {
                            window::MAIN_WINDOW_LABEL => {
                                if let Err(err) = window::hide_launcher_window(win.app_handle()) {
                                    eprintln!("Warning: failed to hide launcher on close request: {err}");
                                }
                            }
                            window::SETTINGS_WINDOW_LABEL => {
                                if let Err(err) = window::hide_settings_window(win.app_handle()) {
                                    eprintln!("Warning: failed to hide settings on close request: {err}");
                                }
                            }
                            _ => {}
                        }
                    }
                }
                WindowEvent::Focused(focused) => {
                    match win.label() {
                        window::MAIN_WINDOW_LABEL => {
                            window::handle_window_focus_change(win.app_handle(), *focused);
                        }
                        window::SETTINGS_WINDOW_LABEL => {
                            window::handle_settings_focus_change(win.app_handle(), *focused);
                        }
                        _ => {}
                    }
                }
                WindowEvent::Moved(pos) => {
                    window::handle_window_moved(win, *pos);
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
            if let Some(state) = app_handle.try_state::<search::SearchState>() {
                if let Ok(mut g) = state.everything.lock() {
                    if let Some(mut mgr) = g.take() {
                        mgr.stop();
                    }
                }
            }
        }
    });
}
