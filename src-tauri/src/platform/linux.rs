use crate::model::PinMode;
use tauri::{AppHandle, WebviewWindow};

pub fn init(_app: &AppHandle) {}

pub fn drag_note(window: &WebviewWindow, phase: &str) -> tauri::Result<()> {
    if phase == "start" { window.start_dragging() } else { Ok(()) }
}

pub fn prepare_window(_window: &WebviewWindow, _tool: bool) {}

/// On X11 (or XWayland) the window manager honors above/below hints. Pure Wayland ignores them.
pub fn set_pin_mode(window: &WebviewWindow, mode: PinMode) {
    let _ = window.set_always_on_top(mode == PinMode::Top);
    let _ = window.set_always_on_bottom(mode == PinMode::Desktop);
}

pub fn on_focus(window: &WebviewWindow) {
    let _ = window.set_always_on_bottom(false);
    let _ = window.set_always_on_top(true);
}

pub fn on_blur(window: &WebviewWindow, mode: PinMode) { set_pin_mode(window, mode); }

pub fn forget(_window: &WebviewWindow) {}
