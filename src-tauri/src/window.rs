use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow,
    Window,
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

pub const TOGGLE_COOLDOWN_MS: u128 = 100;
pub const WINDOW_POSITION_FILE: &str = "window_position.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistentPosition {
    pub center_x: i32,
    pub center_y: i32,
}

pub static IS_PROGRAMMATIC_MOVING: AtomicBool = AtomicBool::new(false);

pub struct LauncherState {
    shortcut_status: Mutex<ShortcutStatus>,
    native_glass_supported: bool,
    quitting: AtomicBool,
    has_gained_focus: AtomicBool,
    shown_at: Mutex<Option<Instant>>,
    current_mode: Mutex<WindowMode>,
    frontend_ready: AtomicBool,
    show_on_ready: AtomicBool,
    settings_shown_at: Mutex<Option<Instant>>,
    settings_has_gained_focus: AtomicBool,
    saved_position: Mutex<Option<PersistentPosition>>,
    idle_height: Mutex<f64>,
    last_toggle_instant: Mutex<Option<Instant>>,
    last_settings_toggle_instant: Mutex<Option<Instant>>,
}

impl LauncherState {
    pub fn new(
        shortcut_status: ShortcutStatus,
        native_glass_supported: bool,
        saved_position: Option<PersistentPosition>,
    ) -> Self {
        Self {
            shortcut_status: Mutex::new(shortcut_status),
            native_glass_supported,
            quitting: AtomicBool::new(false),
            has_gained_focus: AtomicBool::new(false),
            shown_at: Mutex::new(None),
            current_mode: Mutex::new(WindowMode::Launcher),
            frontend_ready: AtomicBool::new(false),
            show_on_ready: AtomicBool::new(false),
            settings_shown_at: Mutex::new(None),
            settings_has_gained_focus: AtomicBool::new(false),
            saved_position: Mutex::new(saved_position),
            idle_height: Mutex::new(LAUNCHER_HEIGHT_IDLE),
            last_toggle_instant: Mutex::new(None),
            last_settings_toggle_instant: Mutex::new(None),
        }
    }

    pub fn idle_height(&self) -> f64 {
        *self.idle_height.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_idle_height(&self, height: f64) {
        if height > 0.0 {
            *self.idle_height.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = height;
        }
    }

    pub fn saved_position(&self) -> Option<PersistentPosition> {
        *self.saved_position.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn set_saved_position(&self, pos: Option<PersistentPosition>) {
        *self.saved_position.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = pos;
    }

    pub fn try_acquire_toggle_cooldown(&self) -> bool {
        let mut guard = self
            .last_toggle_instant
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(last) = *guard {
            if last.elapsed().as_millis() < TOGGLE_COOLDOWN_MS {
                return false;
            }
        }
        *guard = Some(Instant::now());
        true
    }

    pub fn try_acquire_settings_toggle_cooldown(&self) -> bool {
        let mut guard = self
            .last_settings_toggle_instant
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(last) = *guard {
            if last.elapsed().as_millis() < TOGGLE_COOLDOWN_MS {
                return false;
            }
        }
        *guard = Some(Instant::now());
        true
    }

    pub fn cancel_toggle_cooldown(&self) {
        let mut guard = self
            .last_toggle_instant
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = None;
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

    pub fn update_shortcut_status(&self, status: ShortcutStatus) {
        let mut guard = self
            .shortcut_status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = status;
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

    pub fn record_settings_shown(&self) {
        self.settings_has_gained_focus.store(false, Ordering::SeqCst);
        let mut guard = self
            .settings_shown_at
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = Some(Instant::now());
    }

    pub fn record_settings_focused(&self, focused: bool) {
        if focused {
            self.settings_has_gained_focus.store(true, Ordering::SeqCst);
        }
    }

    pub fn reset_settings_shown(&self) {
        self.settings_has_gained_focus.store(false, Ordering::SeqCst);
        let mut guard = self
            .settings_shown_at
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = None;
    }

    pub fn should_hide_settings_on_focus_loss(&self) -> bool {
        if self.is_quitting() {
            return false;
        }

        let gained_focus = self.settings_has_gained_focus.load(Ordering::SeqCst);
        let elapsed = self
            .settings_shown_at
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
    const DWMSBT_NONE: u32 = 1;

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

    pub fn configure_window_frame(hwnd: *mut c_void) {
        unsafe {
            strip_native_frame(hwnd);

            let _ = set_dwm_u32(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, 1);
            let _ = set_dwm_u32(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, 1);
            let _ = set_dwm_u32(hwnd, DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED);
            let _ = set_dwm_u32(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
            let _ = set_dwm_u32(hwnd, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE);
            let _ = set_dwm_u32(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_NONE);

            let margins = Margins {
                cx_left_width: -1,
                cx_right_width: -1,
                cy_top_height: -1,
                cy_bottom_height: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        }
    }

    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
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
        fn GetCursorPos(lp_point: *mut POINT) -> i32;
    }

    pub fn get_global_cursor_pos() -> Option<(i32, i32)> {
        let mut pt = POINT { x: 0, y: 0 };
        unsafe {
            if GetCursorPos(&mut pt) != 0 {
                Some((pt.x, pt.y))
            } else {
                None
            }
        }
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

    pub const SWP_NOACTIVATE: u32 = 0x0010;

    pub fn set_window_rect(hwnd: *mut c_void, x: i32, y: i32, cx: i32, cy: i32) {
        unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                x,
                y,
                cx,
                cy,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}

pub fn configure_native_backdrop<R: Runtime>(window: &WebviewWindow<R>) -> bool {
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        win32_backdrop::configure_window_frame(hwnd.0);
    }
    let _ = window;
    false
}

pub fn set_native_glass<R: Runtime>(_app: &AppHandle<R>, _enable: bool) -> Result<(), String> {
    Ok(())
}

pub const LAUNCHER_WIDTH: f64 = 640.0;
pub const LAUNCHER_HEIGHT_IDLE: f64 = 96.0;
pub const LAUNCHER_HEIGHT_EXPANDED: f64 = 500.0;
pub const CLIPBOARD_HEIGHT: f64 = 560.0;

pub const SETTINGS_WINDOW_LABEL: &str = "settings";
pub const SETTINGS_WIDTH: f64 = 480.0;
pub const SETTINGS_HEIGHT: f64 = 540.0;

static RESIZE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub fn cancel_resize_animation() {
    RESIZE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
}


pub fn get_window_position_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join(WINDOW_POSITION_FILE))
}

pub fn load_persisted_position_from_path(path: &Path) -> Option<PersistentPosition> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<PersistentPosition>(&content).ok()
}

pub fn save_persisted_position_to_path(path: &Path, pos: &PersistentPosition) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string(pos).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

pub fn handle_window_moved<R: Runtime>(window: &Window<R>, pos: PhysicalPosition<i32>) {
    if IS_PROGRAMMATIC_MOVING.load(Ordering::SeqCst) {
        return;
    }
    if !window.is_visible().unwrap_or(false) {
        return;
    }
    if let Ok(size) = window.outer_size() {
        if size.width == 0 || size.height == 0 {
            return;
        }
        let center_x = pos.x + (size.width as i32 / 2);
        let center_y = pos.y + (size.height as i32 / 2);
        let new_pos = PersistentPosition { center_x, center_y };
        if let Some(state) = window.app_handle().try_state::<LauncherState>() {
            state.set_saved_position(Some(new_pos));
        }
        if let Some(path) = get_window_position_path(window.app_handle()) {
            let _ = save_persisted_position_to_path(&path, &new_pos);
        }
    }
}

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

    let saved_pos = app
        .try_state::<LauncherState>()
        .and_then(|s| s.saved_position());

    let width_physical = (LAUNCHER_WIDTH * scale_factor).round() as u32;
    let work_area = resolve_resize_monitor(&window).map(|m| (m.work_area().position, m.work_area().size));

    // When the window is hidden, snap immediately without animation
    if !window.is_visible().unwrap_or(false) {
        if let Some((work_pos, work_size)) = work_area {
            let target_physical_size = PhysicalSize::new(width_physical, target_physical_height);
            let target_pos = calculate_window_position(target_physical_size, work_pos, work_size, saved_pos);
            IS_PROGRAMMATIC_MOVING.store(true, Ordering::SeqCst);
            #[cfg(target_os = "windows")]
            if let Ok(hwnd) = window.hwnd() {
                win32_backdrop::set_window_rect(
                    hwnd.0,
                    target_pos.x,
                    target_pos.y,
                    width_physical as i32,
                    target_physical_height as i32,
                );
                IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
                return Ok(());
            }
            let target_logical = tauri::LogicalSize::new(LAUNCHER_WIDTH, target_height);
            let _ = window.set_size(target_logical);
            let _ = window.set_position(target_pos);
            IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
        } else {
            IS_PROGRAMMATIC_MOVING.store(true, Ordering::SeqCst);
            let target_logical = tauri::LogicalSize::new(LAUNCHER_WIDTH, target_height);
            let _ = window.set_size(target_logical);
            let _ = window.center();
            IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
        }
        return Ok(());
    }

    let generation = RESIZE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    let window_clone = window.clone();
    let start_h = current_physical.height as f64;
    let end_h = target_physical_height as f64;

    IS_PROGRAMMATIC_MOVING.store(true, Ordering::SeqCst);
    std::thread::spawn(move || {
        struct MovingGuard;
        impl Drop for MovingGuard {
            fn drop(&mut self) {
                IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
            }
        }
        let _moving_guard = MovingGuard;

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

            if let Some((work_pos, work_size)) = work_area {
                let current_size = PhysicalSize::new(width_physical, current_physical_h);
                let current_pos = calculate_window_position(current_size, work_pos, work_size, saved_pos);
                #[cfg(target_os = "windows")]
                if let Ok(hwnd) = window_clone.hwnd() {
                    win32_backdrop::set_window_rect(
                        hwnd.0,
                        current_pos.x,
                        current_pos.y,
                        width_physical as i32,
                        current_physical_h as i32,
                    );
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let current_logical_h = current_physical_h as f64 / scale_factor;
                    let _ = window_clone.set_size(tauri::LogicalSize::new(LAUNCHER_WIDTH, current_logical_h));
                    let _ = window_clone.set_position(current_pos);
                }
            } else {
                let current_logical_h = current_physical_h as f64 / scale_factor;
                let _ = window_clone.set_size(tauri::LogicalSize::new(LAUNCHER_WIDTH, current_logical_h));
                let _ = window_clone.center();
            }

            if i < STEPS {
                std::thread::sleep(std::time::Duration::from_millis(STEP_DELAY_MS));
            }
        }

        if RESIZE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == generation {
            if let Some((work_pos, work_size)) = work_area {
                let end_size = PhysicalSize::new(width_physical, target_physical_height);
                let end_pos = calculate_window_position(end_size, work_pos, work_size, saved_pos);
                #[cfg(target_os = "windows")]
                if let Ok(hwnd) = window_clone.hwnd() {
                    win32_backdrop::set_window_rect(
                        hwnd.0,
                        end_pos.x,
                        end_pos.y,
                        width_physical as i32,
                        target_physical_height as i32,
                    );
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = window_clone.set_size(tauri::LogicalSize::new(LAUNCHER_WIDTH, target_height));
                    let _ = window_clone.set_position(end_pos);
                }
            } else {
                let _ = window_clone.set_size(tauri::LogicalSize::new(LAUNCHER_WIDTH, target_height));
                let _ = window_clone.center();
            }
        }
    });

    Ok(())
}

pub fn set_launcher_height<R: Runtime>(app: &AppHandle<R>, expanded: bool) -> Result<(), String> {
    let target_height = if expanded {
        LAUNCHER_HEIGHT_EXPANDED
    } else {
        app.try_state::<LauncherState>()
            .map(|s| s.idle_height())
            .unwrap_or(LAUNCHER_HEIGHT_IDLE)
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
    let y = work_area_pos.y + (y_slack / 2) as i32;

    PhysicalPosition::new(x, y)
}

pub fn calculate_window_position(
    window_size: PhysicalSize<u32>,
    work_area_pos: PhysicalPosition<i32>,
    work_area_size: PhysicalSize<u32>,
    saved_center: Option<PersistentPosition>,
) -> PhysicalPosition<i32> {
    match saved_center {
        Some(saved) => {
            let raw_x = saved.center_x - (window_size.width as i32 / 2);
            let raw_y = saved.center_y - (window_size.height as i32 / 2);

            let min_x = work_area_pos.x;
            let max_x = work_area_pos.x + (work_area_size.width as i32 - window_size.width as i32).max(0);
            let min_y = work_area_pos.y;
            let max_y = work_area_pos.y + (work_area_size.height as i32 - window_size.height as i32).max(0);

            let clamped_x = raw_x.clamp(min_x, max_x);
            let clamped_y = raw_y.clamp(min_y, max_y);

            PhysicalPosition::new(clamped_x, clamped_y)
        }
        None => calculate_launcher_position(window_size, work_area_pos, work_area_size),
    }
}

pub fn resolve_target_monitor_for_position<R: Runtime>(
    window: &WebviewWindow<R>,
    pos: Option<PersistentPosition>,
) -> Option<Monitor> {
    if let Some(target) = pos {
        if let Ok(Some(monitor)) = window.monitor_from_point(target.center_x as f64, target.center_y as f64) {
            return Some(monitor);
        }

        if let Ok(monitors) = window.available_monitors()
            && !monitors.is_empty()
        {
            let closest = monitors.into_iter().min_by_key(|m| {
                let area = m.work_area();
                let mon_center_x = area.position.x + (area.size.width as i32 / 2);
                let mon_center_y = area.position.y + (area.size.height as i32 / 2);
                let dx = (target.center_x - mon_center_x) as i64;
                let dy = (target.center_y - mon_center_y) as i64;
                dx * dx + dy * dy
            });
            if closest.is_some() {
                return closest;
            }
        }
    }

    resolve_target_monitor(window)
}

fn resolve_target_monitor<R: Runtime>(window: &WebviewWindow<R>) -> Option<Monitor> {
    #[cfg(target_os = "windows")]
    if let Some((x, y)) = win32_backdrop::get_global_cursor_pos()
        && let Ok(Some(monitor)) = window.monitor_from_point(x as f64, y as f64)
    {
        return Some(monitor);
    }
    #[cfg(not(target_os = "windows"))]
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

fn resolve_resize_monitor<R: Runtime>(window: &WebviewWindow<R>) -> Option<Monitor> {
    let saved_pos = window
        .app_handle()
        .try_state::<LauncherState>()
        .and_then(|s| s.saved_position());

    if let Ok(Some(monitor)) = window.current_monitor() {
        return Some(monitor);
    }
    resolve_target_monitor_for_position(window, saved_pos)
}

pub fn position_window_centered<R: Runtime>(
    window: &WebviewWindow<R>,
    width: f64,
    height: f64,
) {
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    let window_physical_size = PhysicalSize::new(
        (width * scale_factor).round() as u32,
        (height * scale_factor).round() as u32,
    );
    let saved_pos = window
        .app_handle()
        .try_state::<LauncherState>()
        .and_then(|s| s.saved_position());

    if let Some(monitor) = resolve_target_monitor_for_position(window, saved_pos) {
        let work_area = monitor.work_area();
        let target_pos = calculate_window_position(
            window_physical_size,
            work_area.position,
            work_area.size,
            saved_pos,
        );

        IS_PROGRAMMATIC_MOVING.store(true, Ordering::SeqCst);
        #[cfg(target_os = "windows")]
        if let Ok(hwnd) = window.hwnd() {
            win32_backdrop::set_window_rect(
                hwnd.0,
                target_pos.x,
                target_pos.y,
                window_physical_size.width as i32,
                window_physical_size.height as i32,
            );
            IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
            return;
        }
        let target_logical = tauri::LogicalSize::new(width, height);
        let _ = window.set_size(target_logical);
        let _ = window.set_position(target_pos);
        IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
    } else {
        IS_PROGRAMMATIC_MOVING.store(true, Ordering::SeqCst);
        let target_logical = tauri::LogicalSize::new(width, height);
        let _ = window.set_size(target_logical);
        let _ = window.center();
        IS_PROGRAMMATIC_MOVING.store(false, Ordering::SeqCst);
    }
}

pub fn position_on_active_monitor<R: Runtime>(window: &WebviewWindow<R>, target_height: f64) {
    position_window_centered(window, LAUNCHER_WIDTH, target_height);
}

pub fn show_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    cancel_resize_animation();

    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main launcher window is not available.".to_string())?;

    if let Some(state) = app.try_state::<LauncherState>() {
        state.record_shown();
    }

    let mode = app
        .try_state::<LauncherState>()
        .map(|s| s.mode())
        .unwrap_or(WindowMode::Launcher);

    let target_height = match mode {
        WindowMode::Clipboard => CLIPBOARD_HEIGHT,
        WindowMode::Launcher => app
            .try_state::<LauncherState>()
            .map(|s| s.idle_height())
            .unwrap_or(LAUNCHER_HEIGHT_IDLE),
    };

    position_on_active_monitor(&window, target_height);

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
    cancel_resize_animation();

    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main launcher window is not available.".to_string())?;

    if let Some(state) = app.try_state::<LauncherState>() {
        state.reset_shown();
    }

    if let Err(err) = window.emit(EVENT_LAUNCHER_HIDDEN, ()) {
        eprintln!("Warning: failed to emit launcher-hidden event: {err}");
    }

    let hide_res = window
        .hide()
        .map_err(|err| format!("Failed to hide launcher window: {err}"));

    // Snap to idle height immediately now that the window is hidden, without any animation
    let _ = set_launcher_height(app, false);

    hide_res
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
pub const EVENT_OPEN_SETTINGS: &str = "lux://open-settings";

pub fn show_launcher_window_explicit<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main launcher window is not available.".to_string())?;

    if let Some(state) = app.try_state::<LauncherState>() {
        state.set_mode(WindowMode::Launcher);
        state.record_shown();
    }

    let _ = window.emit(EVENT_OPEN_LAUNCHER, ());
    show_launcher_window(app)
}

pub fn show_settings_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = app
        .get_webview_window(SETTINGS_WINDOW_LABEL)
        .ok_or_else(|| "Settings window is not available.".to_string())?;

    if let Some(state) = app.try_state::<LauncherState>() {
        let _ = state.try_acquire_settings_toggle_cooldown();
        state.record_settings_shown();
    }

    let is_visible = window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if is_minimized {
        let _ = window.unminimize();
    }

    if !is_visible {
        position_window_centered(&window, SETTINGS_WIDTH, SETTINGS_HEIGHT);
    }

    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        win32_backdrop::configure_window_frame(hwnd.0);
    }

    window
        .show()
        .map_err(|err| format!("Failed to show settings window: {err}"))?;

    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        win32_backdrop::activate_foreground(hwnd.0);
    }

    if let Err(err) = window.set_focus() {
        eprintln!("Warning: failed to focus settings window: {err}");
    }

    let _ = window.emit(EVENT_OPEN_SETTINGS, ());

    Ok(())
}

pub fn hide_settings_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(state) = app.try_state::<LauncherState>() {
        state.reset_settings_shown();
    }

    let window = app
        .get_webview_window(SETTINGS_WINDOW_LABEL)
        .ok_or_else(|| "Settings window is not available.".to_string())?;

    window
        .hide()
        .map_err(|err| format!("Failed to hide settings window: {err}"))?;

    Ok(())
}

pub fn handle_settings_focus_change<R: Runtime>(app: &AppHandle<R>, focused: bool) {
    let is_visible = app
        .get_webview_window(SETTINGS_WINDOW_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);

    if !is_visible {
        return;
    }

    if let Some(state) = app.try_state::<LauncherState>() {
        state.record_settings_focused(focused);
    }

    if focused {
        return;
    }

    let should_hide = app
        .try_state::<LauncherState>()
        .map(|state| state.should_hide_settings_on_focus_loss())
        .unwrap_or(true);

    if !should_hide {
        return;
    }

    if let Err(err) = hide_settings_window(app) {
        eprintln!("Warning: failed to hide settings on focus loss: {err}");
    }
}

pub fn toggle_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(state) = app.try_state::<LauncherState>() {
        if !state.try_acquire_toggle_cooldown() {
            return Ok(());
        }
    }

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
    let clipboard_enabled = app
        .try_state::<crate::features::FeaturesState>()
        .map(|f| f.is_clipboard_history_enabled())
        .unwrap_or(true);
    if !clipboard_enabled {
        return Err("Clipboard History is disabled in Settings.".to_string());
    }

    if let Some(state) = app.try_state::<LauncherState>() {
        if !state.try_acquire_toggle_cooldown() {
            return Ok(());
        }
    }

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
    fn centers_horizontally_and_vertically() {
        let window_size = PhysicalSize::new(680, 430);
        let work_pos = PhysicalPosition::new(0, 0);
        let work_size = PhysicalSize::new(1920, 1040);

        let pos = calculate_launcher_position(window_size, work_pos, work_size);
        assert_eq!(pos.x, (1920 - 680) / 2);
        assert_eq!(pos.y, (1040 - 430) / 2);
    }

    #[test]
    fn respects_secondary_monitor_work_area_offset() {
        let window_size = PhysicalSize::new(680, 430);
        let work_pos = PhysicalPosition::new(1920, 40);
        let work_size = PhysicalSize::new(2560, 1400);

        let pos = calculate_launcher_position(window_size, work_pos, work_size);
        assert_eq!(pos.x, 1920 + (2560 - 680) / 2);
        assert_eq!(pos.y, 40 + (1400 - 430) / 2);
    }

    #[test]
    fn compact_and_expanded_launchers_share_vertical_center() {
        let work_pos = PhysicalPosition::new(100, 200);
        let work_size = PhysicalSize::new(1920, 1080);

        let compact_size = PhysicalSize::new(680, 144);
        let expanded_size = PhysicalSize::new(680, 500);

        let compact_pos = calculate_launcher_position(compact_size, work_pos, work_size);
        let expanded_pos = calculate_launcher_position(expanded_size, work_pos, work_size);

        let compact_center_y = compact_pos.y + (compact_size.height as i32 / 2);
        let expanded_center_y = expanded_pos.y + (expanded_size.height as i32 / 2);
        let expected_screen_center_y = work_pos.y + (work_size.height as i32 / 2);

        assert_eq!(compact_center_y, expected_screen_center_y);
        assert_eq!(expanded_center_y, expected_screen_center_y);
        assert_eq!(compact_center_y, expanded_center_y);
    }

    #[test]
    fn dynamic_height_resizing_keeps_center_anchored_without_drift() {
        let work_pos = PhysicalPosition::new(0, 40);
        let work_size = PhysicalSize::new(2560, 1400);
        let screen_center_y = work_pos.y + (work_size.height as i32 / 2);

        // Sequence of height transitions: minimal idle (78, 96, 114), hints idle (124, 144, 166),
        // search (500), clipboard (560), back to minimal idle (96)
        let heights = [78, 96, 114, 124, 144, 166, 250, 380, 500, 560, 300, 96];
        for h in heights {
            let size = PhysicalSize::new(760, h);
            let pos = calculate_launcher_position(size, work_pos, work_size);
            let center_y = pos.y + (h as i32 / 2);
            assert_eq!(
                center_y, screen_center_y,
                "Height {h} drifted from screen center {screen_center_y}"
            );
        }
    }

    #[test]
    fn launcher_state_idle_height_management() {
        let state = LauncherState::new(
            ShortcutStatus {
                shortcut: "Alt + Space".to_string(),
                registered: true,
                error: None,
            },
            false,
            None,
        );
        assert_eq!(state.idle_height(), LAUNCHER_HEIGHT_IDLE);

        state.set_idle_height(68.0);
        assert_eq!(state.idle_height(), 68.0);

        state.set_idle_height(54.0);
        assert_eq!(state.idle_height(), 54.0);

        state.set_idle_height(76.0);
        assert_eq!(state.idle_height(), 76.0);

        // Negative or zero heights should be ignored
        state.set_idle_height(0.0);
        assert_eq!(state.idle_height(), 76.0);
        state.set_idle_height(-10.0);
        assert_eq!(state.idle_height(), 76.0);
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
            None,
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
    fn settings_focus_loss_lifecycle_dismissal() {
        let state = LauncherState::new(
            ShortcutStatus {
                shortcut: "Alt + Space".to_string(),
                registered: true,
                error: None,
            },
            true,
            None,
        );

        // Before shown, does not hide (not in an active session)
        assert!(!state.should_hide_settings_on_focus_loss());

        // Immediately when shown, window hasn't gained focus yet -> premature focus loss is ignored
        state.record_settings_shown();
        assert!(!state.should_hide_settings_on_focus_loss());

        // During initial settling grace period (<150ms), transient blur is ignored
        state.record_settings_focused(true);
        assert!(!state.should_hide_settings_on_focus_loss());

        // After settling grace period, losing focus dismisses settings
        std::thread::sleep(Duration::from_millis(160));
        assert!(state.should_hide_settings_on_focus_loss());

        // Hiding resets state cleanly; unshown state does not trigger hide
        state.reset_settings_shown();
        assert!(!state.should_hide_settings_on_focus_loss());

        // Reopening starts with clean state
        state.record_settings_shown();
        assert!(!state.should_hide_settings_on_focus_loss());
        state.record_settings_focused(true);
        std::thread::sleep(Duration::from_millis(160));
        assert!(state.should_hide_settings_on_focus_loss());

        // Quitting prevents dismissal loops
        state.mark_quitting();
        assert!(!state.should_hide_settings_on_focus_loss());
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
            None,
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

    #[test]
    fn persistent_position_fallback_matches_centered() {
        let window_size = PhysicalSize::new(760, 144);
        let work_pos = PhysicalPosition::new(0, 0);
        let work_size = PhysicalSize::new(1920, 1080);

        let default_pos = calculate_launcher_position(window_size, work_pos, work_size);
        let calculated_pos = calculate_window_position(window_size, work_pos, work_size, None);

        assert_eq!(default_pos, calculated_pos);
    }

    #[test]
    fn persistent_position_custom_coordinates_and_clamping() {
        let work_pos = PhysicalPosition::new(0, 0);
        let work_size = PhysicalSize::new(1920, 1080);

        let custom_center = PersistentPosition {
            center_x: 500,
            center_y: 400,
        };

        let compact_size = PhysicalSize::new(760, 144);
        let expanded_size = PhysicalSize::new(760, 500);

        let compact_pos = calculate_window_position(compact_size, work_pos, work_size, Some(custom_center));
        let expanded_pos = calculate_window_position(expanded_size, work_pos, work_size, Some(custom_center));

        // Horizontal position is anchored to custom_center
        assert_eq!(compact_pos.x, 500 - (760 / 2));
        assert_eq!(expanded_pos.x, 500 - (760 / 2));

        // Both compact and expanded share vertical center at 400
        let compact_center_y = compact_pos.y + (144 / 2);
        let expanded_center_y = expanded_pos.y + (500 / 2);
        assert_eq!(compact_center_y, 400);
        assert_eq!(expanded_center_y, 400);
        assert_eq!(compact_center_y, expanded_center_y);

        // Off-screen clamp: position way outside top-left
        let outside_top_left = PersistentPosition {
            center_x: -1000,
            center_y: -1000,
        };
        let clamped_pos = calculate_window_position(compact_size, work_pos, work_size, Some(outside_top_left));
        assert_eq!(clamped_pos.x, 0);
        assert_eq!(clamped_pos.y, 0);

        // Off-screen clamp: position way outside bottom-right
        let outside_bottom_right = PersistentPosition {
            center_x: 5000,
            center_y: 5000,
        };
        let clamped_br = calculate_window_position(compact_size, work_pos, work_size, Some(outside_bottom_right));
        assert_eq!(clamped_br.x, 1920 - 760);
        assert_eq!(clamped_br.y, 1080 - 144);
    }

    #[test]
    fn persistent_position_serialization_roundtrip() {
        let pos = PersistentPosition {
            center_x: 800,
            center_y: 600,
        };
        let json = serde_json::to_string(&pos).unwrap();
        let parsed: PersistentPosition = serde_json::from_str(&json).unwrap();
        assert_eq!(pos, parsed);
    }
    #[test]
    fn test_cooldown_mechanism_blocks_rapid_sub_100ms_toggles() {
        let state = LauncherState::new(
            ShortcutStatus {
                shortcut: "Alt+Space".to_string(),
                registered: true,
                error: None,
            },
            true,
            None,
        );

        // First attempt succeeds
        assert!(state.try_acquire_toggle_cooldown());

        // Immediate sub-100ms attempt is blocked
        assert!(!state.try_acquire_toggle_cooldown(), "Sub-100ms toggle must be debounced");

        // After cooldown is canceled/cleared, it succeeds again
        state.cancel_toggle_cooldown();
        assert!(state.try_acquire_toggle_cooldown());
    }

    #[test]
    fn test_settings_cooldown_mechanism_blocks_rapid_toggles() {
        let state = LauncherState::new(
            ShortcutStatus {
                shortcut: "Alt+Space".to_string(),
                registered: true,
                error: None,
            },
            true,
            None,
        );

        assert!(state.try_acquire_settings_toggle_cooldown());
        assert!(!state.try_acquire_settings_toggle_cooldown(), "Sub-100ms settings toggle must be debounced");
    }

    #[test]
    fn test_cancel_resize_animation_bumps_generation() {
        let gen_before = RESIZE_GENERATION.load(Ordering::SeqCst);
        IS_PROGRAMMATIC_MOVING.store(true, Ordering::SeqCst);

        cancel_resize_animation();

        let gen_after = RESIZE_GENERATION.load(Ordering::SeqCst);
        assert!(gen_after > gen_before, "Generation must increment to cancel animations");
        assert!(!IS_PROGRAMMATIC_MOVING.load(Ordering::SeqCst), "Programmatic moving flag must reset");
    }
}
