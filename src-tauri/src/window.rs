use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow,
};

use crate::shortcuts::ShortcutStatus;

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const EVENT_LAUNCHER_SHOWN: &str = "lux://launcher-shown";
pub const EVENT_LAUNCHER_HIDDEN: &str = "lux://launcher-hidden";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    pub shortcut: ShortcutStatus,
    pub native_glass_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    Launcher,
    Clipboard,
}

pub struct LauncherState {
    shortcut_status: Mutex<ShortcutStatus>,
    native_glass_supported: bool,
    quitting: AtomicBool,
    has_gained_focus: AtomicBool,
    shown_at: Mutex<Option<Instant>>,
    current_mode: Mutex<WindowMode>,
    frontend_ready: AtomicBool,
    show_on_ready: AtomicBool,
}

impl LauncherState {
    pub fn new(shortcut_status: ShortcutStatus, native_glass_supported: bool) -> Self {
        Self {
            shortcut_status: Mutex::new(shortcut_status),
            native_glass_supported,
            quitting: AtomicBool::new(false),
            has_gained_focus: AtomicBool::new(false),
            shown_at: Mutex::new(None),
            current_mode: Mutex::new(WindowMode::Launcher),
            frontend_ready: AtomicBool::new(false),
            show_on_ready: AtomicBool::new(false),
        }
    }

    pub fn status(&self) -> LauncherStatus {
        let shortcut = self
            .shortcut_status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        LauncherStatus {
            shortcut,
            native_glass_supported: self.native_glass_supported,
        }
    }

    pub fn mode(&self) -> WindowMode {
        *self
            .current_mode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_mode(&self, mode: WindowMode) {
        let mut guard = self
            .current_mode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = mode;
    }

    pub fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }

    pub fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::SeqCst)
    }

    pub fn record_shown(&self) {
        self.has_gained_focus.store(false, Ordering::SeqCst);
        let mut guard = self
            .shown_at
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Some(Instant::now());
    }

    pub fn record_focused(&self, focused: bool) {
        if focused {
            self.has_gained_focus.store(true, Ordering::SeqCst);
        }
    }

    pub fn reset_shown(&self) {
        self.has_gained_focus.store(false, Ordering::SeqCst);
        let mut guard = self
            .shown_at
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = None;
        let mut mode = self
            .current_mode
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *mode = WindowMode::Launcher;
    }

    pub fn should_hide_on_focus_loss(&self) -> bool {
        if self.is_quitting() {
            return false;
        }

        let gained_focus = self.has_gained_focus.load(Ordering::SeqCst);
        let elapsed = self
            .shown_at
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .map(|t| t.elapsed());

        match elapsed {
            None => false,
            Some(d) => {
                // Ignore transient focus loss during the initial window show and DOM settling window
                if d < Duration::from_millis(150) {
                    false
                } else {
                    gained_focus || d >= Duration::from_millis(1000)
                }
            }
        }
    }

    pub fn mark_frontend_ready(&self) {
        self.frontend_ready.store(true, Ordering::SeqCst);
    }

    pub fn is_frontend_ready(&self) -> bool {
        self.frontend_ready.load(Ordering::SeqCst)
    }

    pub fn set_show_on_ready(&self, show: bool) {
        self.show_on_ready.store(show, Ordering::SeqCst);
    }

    pub fn should_show_on_ready(&self) -> bool {
        self.show_on_ready.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "windows")]
mod win32_backdrop {
    use std::ffi::c_void;

    #[repr(C)]
    struct Margins {
        cx_left_width: i32,
        cx_right_width: i32,
        cy_top_height: i32,
        cy_bottom_height: i32,
    }

    const DWMWA_NCRENDERING_POLICY: u32 = 2;
    const DWMNCRP_DISABLED: u32 = 1;

    const DWMWA_TRANSITIONS_FORCEDISABLED: u32 = 3;
    const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWA_BORDER_COLOR: u32 = 34;
    const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;

    const DWMWCP_ROUND: u32 = 2;
    const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
    pub const DWMSBT_NONE: u32 = 1;
    pub const DWMSBT_MAINWINDOW: u32 = 2;
    pub const DWMSBT_TRANSIENTWINDOW: u32 = 3;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: *mut c_void,
            dw_attribute: u32,
            pv_attribute: *const c_void,
            cb_attribute: u32,
        ) -> i32;
        fn DwmExtendFrameIntoClientArea(hwnd: *mut c_void, p_mar_inset: *const Margins) -> i32;
    }

    unsafe fn set_dwm_u32(hwnd: *mut c_void, attr: u32, value: u32) -> i32 {
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attr,
                (&raw const value).cast::<c_void>(),
                std::mem::size_of::<u32>() as u32,
            )
        }
    }

    pub fn configure_window_frame(hwnd: *mut c_void) -> bool {
        unsafe {
            strip_native_frame(hwnd);

            let _ = set_dwm_u32(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1);
            let _ = set_dwm_u32(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, 1);
            let _ = set_dwm_u32(hwnd, DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED);
            let _ = set_dwm_u32(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
            let _ = set_dwm_u32(hwnd, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE);

            let res_acrylic = set_dwm_u32(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_TRANSIENTWINDOW);
            let backdrop_applied = if res_acrylic == 0 {
                true
            } else {
                set_dwm_u32(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_MAINWINDOW) == 0
            };

            let margins = Margins {
                cx_left_width: -1,
                cx_right_width: -1,
                cy_top_height: -1,
                cy_bottom_height: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

            backdrop_applied
        }
    }

    pub fn set_backdrop(hwnd: *mut c_void, backdrop_type: u32) -> bool {
        unsafe { set_dwm_u32(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, backdrop_type) == 0 }
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
        fn BringWindowToTop(hwnd: *mut c_void) -> i32;
        fn GetWindowLongA(hwnd: *mut c_void, n_index: i32) -> i32;
        fn SetWindowLongA(hwnd: *mut c_void, n_index: i32, new_long: i32) -> i32;
        fn SetWindowPos(
            hwnd: *mut c_void,
            hwnd_insert_after: *mut c_void,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
    }

    const GWL_STYLE: i32 = -16;
    const WS_CAPTION: i32 = 0x00C0_0000;
    const WS_SYSMENU: i32 = 0x0008_0000;
    const WS_THICKFRAME: i32 = 0x0004_0000;
    const WS_POPUP: i32 = -0x8000_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_FRAMECHANGED: u32 = 0x0020;

    pub fn strip_native_frame(hwnd: *mut c_void) {
        unsafe {
            let style = GetWindowLongA(hwnd, GWL_STYLE);
            let new_style = (style & !(WS_CAPTION | WS_SYSMENU | WS_THICKFRAME)) | WS_POPUP;
            SetWindowLongA(hwnd, GWL_STYLE, new_style);
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }
    }

    pub fn activate_foreground(hwnd: *mut c_void) {
        unsafe {
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

pub fn configure_native_backdrop<R: Runtime>(window: &WebviewWindow<R>) -> bool {
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        return win32_backdrop::configure_window_frame(hwnd.0);
    }
    let _ = window;
    false
}

pub fn set_native_glass<R: Runtime>(app: &AppHandle<R>, enable: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL)
        && let Ok(hwnd) = window.hwnd()
    {
        let backdrop = if enable {
            win32_backdrop::DWMSBT_TRANSIENTWINDOW
        } else {
            win32_backdrop::DWMSBT_NONE
        };
        win32_backdrop::set_backdrop(hwnd.0, backdrop);
    }
    let _ = app;
    let _ = enable;
    Ok(())
}

pub const LAUNCHER_WIDTH: f64 = 760.0;
pub const LAUNCHER_HEIGHT_IDLE: f64 = 144.0;
pub const LAUNCHER_HEIGHT_EXPANDED: f64 = 500.0;
pub const CLIPBOARD_HEIGHT: f64 = 560.0;

static RESIZE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn set_window_target_height<R: Runtime>(app: &AppHandle<R>, target_height: f64) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;

    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let current_physical = window.outer_size().map_err(|e| e.to_string())?;
    let target_physical_height = (target_height * scale_factor).round() as u32;

    if current_physical.height == target_physical_height {
        return Ok(());
    }

    // When the window is hidden, snap immediately without animation
    if !window.is_visible().unwrap_or(false) {
        let target_logical = tauri::LogicalSize::new(LAUNCHER_WIDTH, target_height);
        let _ = window.set_size(target_logical);
        return Ok(());
    }

    let generation = RESIZE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let window_clone = window.clone();
    let start_h = current_physical.height as f64;
    let end_h = target_physical_height as f64;

    std::thread::spawn(move || {
        const STEPS: usize = 9;
        const STEP_DELAY_MS: u64 = 16;

        for i in 1..=STEPS {
            if RESIZE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
                return;
            }
            let t = i as f64 / STEPS as f64;
            // Cubic ease-out: 1.0 - (1.0 - t)^3
            let factor = 1.0 - (1.0 - t).powi(3);
            let current_physical_h = (start_h + (end_h - start_h) * factor).round() as u32;
            let current_logical_h = current_physical_h as f64 / scale_factor;
            let _ = window_clone.set_size(tauri::LogicalSize::new(LAUNCHER_WIDTH, current_logical_h));

            if i < STEPS {
                std::thread::sleep(std::time::Duration::from_millis(STEP_DELAY_MS));
            }
        }

        if RESIZE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == generation {
            let _ = window_clone.set_size(tauri::LogicalSize::new(LAUNCHER_WIDTH, target_height));
        }
    });

    Ok(())
}

pub fn set_launcher_height<R: Runtime>(app: &AppHandle<R>, expanded: bool) -> Result<(), String> {
    let target_height = if expanded {
        LAUNCHER_HEIGHT_EXPANDED
    } else {
        LAUNCHER_HEIGHT_IDLE
    };
    set_window_target_height(app, target_height)
}

pub fn calculate_launcher_position(
    window_size: PhysicalSize<u32>,
    work_area_pos: PhysicalPosition<i32>,
    work_area_size: PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let x_slack = (work_area_size.width as i64 - window_size.width as i64).max(0);
    let y_slack = (work_area_size.height as i64 - window_size.height as i64).max(0);

    let x = work_area_pos.x + (x_slack / 2) as i32;
    let y = work_area_pos.y + ((y_slack * 28) / 100) as i32;

    PhysicalPosition::new(x, y)
}

fn resolve_target_monitor<R: Runtime>(window: &WebviewWindow<R>) -> Option<Monitor> {
    if let Ok(cursor) = window.cursor_position()
        && let Ok(Some(monitor)) = window.monitor_from_point(cursor.x, cursor.y)
    {
        return Some(monitor);
    }
    if let Ok(Some(monitor)) = window.current_monitor() {
        return Some(monitor);
    }
    window.primary_monitor().ok().flatten()
}

pub fn position_on_active_monitor<R: Runtime>(window: &WebviewWindow<R>) {
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let full_window_size = PhysicalSize::new(
        (LAUNCHER_WIDTH * scale_factor).round() as u32,
        (LAUNCHER_HEIGHT_EXPANDED * scale_factor).round() as u32,
    );
    if let Some(monitor) = resolve_target_monitor(window) {
        let work_area = monitor.work_area();
        let target_pos =
            calculate_launcher_position(full_window_size, work_area.position, work_area.size);
        let _ = window.set_position(target_pos);
    } else {
        let _ = window.center();
    }
}

pub fn show_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main launcher window is not available.".to_string())?;

    if let Some(state) = app.try_state::<LauncherState>() {
        state.record_shown();
    }

    position_on_active_monitor(&window);

    let mode = app
        .try_state::<LauncherState>()
        .map(|s| s.mode())
        .unwrap_or(WindowMode::Launcher);

    let target_height = match mode {
        WindowMode::Clipboard => CLIPBOARD_HEIGHT,
        WindowMode::Launcher => LAUNCHER_HEIGHT_IDLE,
    };
    let target_logical = tauri::LogicalSize::new(LAUNCHER_WIDTH, target_height);
    let _ = window.set_size(target_logical);

    if window.is_minimized().unwrap_or(false)
        && let Err(err) = window.unminimize()
    {
        eprintln!("Warning: failed to unminimize launcher window: {err}");
    }

    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        win32_backdrop::configure_window_frame(hwnd.0);
    }

    window
        .show()
        .map_err(|err| format!("Failed to show launcher window: {err}"))?;

    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        win32_backdrop::activate_foreground(hwnd.0);
    }

    if let Err(err) = window.set_focus() {
        eprintln!("Warning: failed to focus launcher window: {err}");
    }

    if let Err(err) = window.emit(EVENT_LAUNCHER_SHOWN, ()) {
        eprintln!("Warning: failed to emit launcher-shown event: {err}");
    }

    Ok(())
}

pub fn hide_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main launcher window is not available.".to_string())?;

    if let Some(state) = app.try_state::<LauncherState>() {
        state.reset_shown();
    }

    let _ = set_launcher_height(app, false);

    if let Err(err) = window.emit(EVENT_LAUNCHER_HIDDEN, ()) {
        eprintln!("Warning: failed to emit launcher-hidden event: {err}");
    }

    window
        .hide()
        .map_err(|err| format!("Failed to hide launcher window: {err}"))?;

    Ok(())
}

pub fn handle_window_focus_change<R: Runtime>(app: &AppHandle<R>, focused: bool) {
    let is_visible = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);

    // If the window is not currently visible, ignore all focus events.
    // This guarantees that stale or delayed focus-loss events from a previous hide
    // cycle cannot dismiss a newly reopened launcher or corrupt its focus state.
    if !is_visible {
        return;
    }

    if let Some(state) = app.try_state::<LauncherState>() {
        state.record_focused(focused);
    }

    if focused {
        return;
    }

    let should_hide = app
        .try_state::<LauncherState>()
        .map(|state| state.should_hide_on_focus_loss())
        .unwrap_or(true);

    if !should_hide {
        return;
    }

    if let Err(err) = hide_launcher_window(app) {
        eprintln!("Warning: failed to hide launcher on focus loss: {err}");
    }
}

pub const EVENT_OPEN_LAUNCHER: &str = "lux://open-launcher";
pub const EVENT_OPEN_CLIPBOARD: &str = "lux://open-clipboard";

pub fn toggle_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main launcher window is not available.".to_string())?;

    let is_visible = window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if is_visible && !is_minimized {
        let is_launcher = app
            .try_state::<LauncherState>()
            .map(|s| s.mode() == WindowMode::Launcher)
            .unwrap_or(true);

        if is_launcher {
            hide_launcher_window(app)
        } else {
            if let Some(state) = app.try_state::<LauncherState>() {
                state.set_mode(WindowMode::Launcher);
                state.record_shown();
            }
            let _ = window.emit(EVENT_OPEN_LAUNCHER, ());
            set_launcher_height(app, false)
        }
    } else {
        if let Some(state) = app.try_state::<LauncherState>() {
            state.set_mode(WindowMode::Launcher);
        }
        let _ = window.emit(EVENT_OPEN_LAUNCHER, ());
        show_launcher_window(app)
    }
}

pub fn toggle_clipboard_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main launcher window is not available.".to_string())?;

    let is_visible = window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if is_visible && !is_minimized {
        let is_clipboard = app
            .try_state::<LauncherState>()
            .map(|s| s.mode() == WindowMode::Clipboard)
            .unwrap_or(false);

        if is_clipboard {
            hide_launcher_window(app)
        } else {
            if let Some(state) = app.try_state::<LauncherState>() {
                state.set_mode(WindowMode::Clipboard);
                state.record_shown();
            }
            let _ = window.emit(EVENT_OPEN_CLIPBOARD, ());
            set_window_target_height(app, CLIPBOARD_HEIGHT)
        }
    } else {
        if let Some(state) = app.try_state::<LauncherState>() {
            state.set_mode(WindowMode::Clipboard);
        }
        let _ = window.emit(EVENT_OPEN_CLIPBOARD, ());
        show_launcher_window(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_horizontally_and_places_in_upper_third_vertically() {
        let window_size = PhysicalSize::new(680, 430);
        let work_pos = PhysicalPosition::new(0, 0);
        let work_size = PhysicalSize::new(1920, 1040);

        let pos = calculate_launcher_position(window_size, work_pos, work_size);
        assert_eq!(pos.x, (1920 - 680) / 2);
        assert_eq!(pos.y, ((1040 - 430) * 28) / 100);
    }

    #[test]
    fn respects_secondary_monitor_work_area_offset() {
        let window_size = PhysicalSize::new(680, 430);
        let work_pos = PhysicalPosition::new(1920, 40);
        let work_size = PhysicalSize::new(2560, 1400);

        let pos = calculate_launcher_position(window_size, work_pos, work_size);
        assert_eq!(pos.x, 1920 + (2560 - 680) / 2);
        assert_eq!(pos.y, 40 + ((1400 - 430) * 28) / 100);
    }

    #[test]
    fn clamps_to_work_area_origin_when_display_is_smaller_than_window() {
        let window_size = PhysicalSize::new(680, 430);
        let work_pos = PhysicalPosition::new(10, 20);
        let work_size = PhysicalSize::new(500, 300);

        let pos = calculate_launcher_position(window_size, work_pos, work_size);
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    }

    #[test]
    fn focus_loss_lifecycle_dismissal() {
        let state = LauncherState::new(
            ShortcutStatus {
                shortcut: "Alt + Space".to_string(),
                registered: true,
                error: None,
            },
            true,
        );

        // Before shown, does not hide (not in an active session)
        assert!(!state.should_hide_on_focus_loss());

        // Immediately when shown, window hasn't gained focus yet -> premature focus loss is ignored
        state.record_shown();
        assert!(!state.should_hide_on_focus_loss());

        // During the initial settling grace period (<150ms), transient blur from DOM unmounting is ignored
        state.record_focused(true);
        assert!(!state.should_hide_on_focus_loss());

        // After the settling grace period, losing focus dismisses the window
        std::thread::sleep(Duration::from_millis(160));
        assert!(state.should_hide_on_focus_loss());

        // Hiding resets state cleanly; unshown state does not trigger hide
        state.reset_shown();
        assert!(!state.should_hide_on_focus_loss());

        // Reopening starts with clean state
        state.record_shown();
        assert!(!state.should_hide_on_focus_loss());
        state.record_focused(true);
        std::thread::sleep(Duration::from_millis(160));
        assert!(state.should_hide_on_focus_loss());

        // Quitting prevents dismissal loops
        state.mark_quitting();
        assert!(!state.should_hide_on_focus_loss());
    }

    #[test]
    fn window_mode_state_transitions() {
        let state = LauncherState::new(
            ShortcutStatus {
                shortcut: "Alt + Space".to_string(),
                registered: true,
                error: None,
            },
            true,
        );

        // Default mode is Launcher
        assert_eq!(state.mode(), WindowMode::Launcher);

        // Switching to clipboard mode updates state
        state.set_mode(WindowMode::Clipboard);
        assert_eq!(state.mode(), WindowMode::Clipboard);

        // Resetting shown state restores Launcher mode
        state.reset_shown();
        assert_eq!(state.mode(), WindowMode::Launcher);
    }
}
