use tauri::{
    AppHandle, Manager, Runtime,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::{shortcuts, window, window::LauncherState};

const MENU_ID_OPEN: &str = "open";
const MENU_ID_SETTINGS: &str = "settings";
const MENU_ID_QUIT: &str = "quit";

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let open_item = MenuItem::with_id(app, MENU_ID_OPEN, "Open Launcher", true, None::<&str>)
        .map_err(|err| format!("Failed to create tray Open item: {err}"))?;
    let settings_item = MenuItem::with_id(app, MENU_ID_SETTINGS, "Open Settings", true, None::<&str>)
        .map_err(|err| format!("Failed to create tray Settings item: {err}"))?;
    let quit_item = MenuItem::with_id(app, MENU_ID_QUIT, "Quit Lux", true, None::<&str>)
        .map_err(|err| format!("Failed to create tray Quit item: {err}"))?;

    let menu = Menu::with_items(app, &[&open_item, &settings_item, &quit_item])
        .map_err(|err| format!("Failed to create tray menu: {err}"))?;

    let mut builder = TrayIconBuilder::with_id("lux-tray")
        .menu(&menu)
        .tooltip("Lux Launcher")
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            MENU_ID_OPEN => {
                if let Err(err) = window::show_launcher_window_explicit(app_handle) {
                    eprintln!("Warning: failed to open launcher from tray menu: {err}");
                }
            }
            MENU_ID_SETTINGS => {
                if let Err(err) = window::show_settings_window(app_handle) {
                    eprintln!("Warning: failed to open settings from tray menu: {err}");
                }
            }
            MENU_ID_QUIT => {
                if let Some(state) = app_handle.try_state::<LauncherState>() {
                    state.mark_quitting();
                }
                if let Some(state) = app_handle.try_state::<crate::search::SearchState>() {
                    if let Ok(mut g) = state.everything.lock() {
                        if let Some(mut mgr) = g.take() {
                            mgr.stop();
                        }
                    }
                }
                shortcuts::unregister_all_shortcuts(app_handle);
                app_handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && let Err(err) = window::toggle_launcher_window(tray.app_handle())
            {
                eprintln!("Warning: failed to toggle launcher from tray click: {err}");
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder
        .build(app)
        .map_err(|err| format!("Failed to build system tray icon: {err}"))?;

    Ok(())
}
