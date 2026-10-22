use crate::model::PinMode;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, WebviewWindow};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromRect, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    EVENT_SYSTEM_FOREGROUND, GWL_EXSTYLE, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_SHOWNOACTIVATE, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WM_MOVING, WM_NCDESTROY, WM_SYSCOMMAND, WM_NCLBUTTONDBLCLK, SC_MAXIMIZE,
};

/// Windows pinned to the desktop, as raw HWND values.
static PINNED: Mutex<Option<HashSet<isize>>> = Mutex::new(None);
static NOTES: Mutex<Option<HashSet<isize>>> = Mutex::new(None);

fn rect(r: RECT) -> crate::snapping::Rect {
    crate::snapping::Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

fn visible_bounds(h: HWND, fallback: RECT) -> RECT {
    let mut bounds = RECT::default();
    if unsafe { DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, &mut bounds as *mut _ as *mut c_void, std::mem::size_of::<RECT>() as u32) }.is_ok()
        && bounds.right > bounds.left && bounds.bottom > bounds.top { bounds } else { fallback }
}

fn snap_bounds(h: HWND, proposed: &mut RECT) {
    let monitor = unsafe { MonitorFromRect(proposed, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        let handles: Vec<isize> = NOTES.lock().unwrap().iter().flatten().copied().collect();
        let mut others = Vec::new();
        for raw in handles {
            let other = HWND(raw as *mut c_void);
            if other == h || !unsafe { IsWindowVisible(other) }.as_bool() || unsafe { IsIconic(other) }.as_bool() { continue; }
            let mut r = RECT::default();
            if unsafe { GetWindowRect(other, &mut r) }.is_ok() { others.push(rect(visible_bounds(other, r))); }
        }
        let threshold = (14 * unsafe { GetDpiForWindow(h) }.max(96) / 96) as i32;
        // Invisible Windows resize borders must not leave a gap at the visible edge.
        let mut outer = RECT::default();
        let visible = if unsafe { GetWindowRect(h, &mut outer) }.is_ok() {
            let bounds = visible_bounds(h, outer);
            RECT { left: proposed.left + bounds.left - outer.left, top: proposed.top + bounds.top - outer.top,
                right: proposed.right + bounds.right - outer.right, bottom: proposed.bottom + bounds.bottom - outer.bottom }
        } else { *proposed };
        let snapped = crate::snapping::snap(rect(visible), rect(info.rcWork), &others, threshold);
        let dx = snapped.left - visible.left;
        let dy = snapped.top - visible.top;
        proposed.left += dx; proposed.right += dx; proposed.top += dy; proposed.bottom += dy;
    }
}

#[derive(Clone, Copy)]
struct DragOrigin { cursor: POINT, bounds: RECT }
static DRAGS: Mutex<Option<HashMap<isize, DragOrigin>>> = Mutex::new(None);

/// Pointer capture drives position-only movement, bypassing Windows Aero Snap sizing.
pub fn drag_note(window: &WebviewWindow, phase: &str) -> tauri::Result<()> {
    let Some(h) = hwnd(window) else { return Ok(()) };
    let raw = h.0 as isize;
    let phase = phase.to_owned();
    window.run_on_main_thread(move || {
        let h = HWND(raw as *mut c_void);
        let mut cursor = POINT::default();
        if unsafe { GetCursorPos(&mut cursor) }.is_err() { return; }
        let mut drags = DRAGS.lock().unwrap();
        let drags = drags.get_or_insert_with(HashMap::new);
        if phase == "start" {
            let mut bounds = RECT::default();
            if unsafe { GetWindowRect(h, &mut bounds) }.is_ok() { drags.insert(raw, DragOrigin { cursor, bounds }); }
            return;
        }
        let Some(origin) = drags.get(&raw).copied() else { return; };
        if phase == "end" { drags.remove(&raw); }
        let dx = cursor.x - origin.cursor.x;
        let dy = cursor.y - origin.cursor.y;
        let mut current = origin.bounds;
        unsafe { let _ = GetWindowRect(h, &mut current); }
        let mut proposed = RECT { left: origin.bounds.left + dx, top: origin.bounds.top + dy,
            right: origin.bounds.left + dx + current.right - current.left,
            bottom: origin.bounds.top + dy + current.bottom - current.top };
        snap_bounds(h, &mut proposed);
        unsafe { let _ = SetWindowPos(h, None, proposed.left, proposed.top, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE); }
    })
}

unsafe extern "system" fn move_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM, id: usize, _data: usize) -> LRESULT {
    if msg == WM_SYSCOMMAND && (wp.0 as u32 & 0xfff0) == SC_MAXIMIZE { return LRESULT(0); }
    if msg == WM_NCLBUTTONDBLCLK { return LRESULT(0); }
    if msg == WM_MOVING && lp.0 != 0 {
        snap_bounds(h, unsafe { &mut *(lp.0 as *mut RECT) });
        return LRESULT(1);
    } else if msg == WM_NCDESTROY {
        unsafe { let _ = RemoveWindowSubclass(h, Some(move_proc), id); }
        if let Some(notes) = NOTES.lock().unwrap().as_mut() { notes.remove(&(h.0 as isize)); }
    }
    unsafe { DefSubclassProc(h, msg, wp, lp) }
}
/// True while "show desktop" is active and pinned notes are lifted above it.
static RAISED: AtomicBool = AtomicBool::new(false);

fn hwnd(window: &WebviewWindow) -> Option<HWND> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut c_void)),
        _ => None,
    }
}

fn pinned() -> Vec<HWND> {
    PINNED
        .lock()
        .unwrap()
        .iter()
        .flatten()
        .map(|&h| HWND(h as *mut c_void))
        .collect()
}

const KEEP: windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS =
    windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOACTIVATE.0);

fn to_bottom(h: HWND) {
    unsafe {
        let _ = SetWindowPos(h, Some(HWND_NOTOPMOST), 0, 0, 0, 0, KEEP);
        let _ = SetWindowPos(h, Some(HWND_BOTTOM), 0, 0, 0, 0, KEEP);
    }
}

fn is_desktop(h: HWND) -> bool {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(h, &mut buf) } as usize;
    matches!(String::from_utf16_lossy(&buf[..len]).as_ref(), "WorkerW" | "Progman")
}

/// HWND_NOTOPMOST demotes a focused note to the *top* of the normal group.
/// Put it beneath the new foreground window without activating either window.
fn restore_normal_order(window: &WebviewWindow) {
    let Some(h) = hwnd(window) else { return };
    let raw = h.0 as isize;
    // Queue behind Tauri's flag change when invoked from a command worker.
    let _ = window.run_on_main_thread(move || unsafe {
        let h = HWND(raw as *mut c_void);
        let foreground = GetForegroundWindow();
        if foreground.is_invalid() || foreground == h || is_desktop(foreground) { return; }
        // Inserting into the topmost group would promote a normal note again.
        // A newer focus/pin change can also supersede the queued restoration.
        if (GetWindowLongPtrW(foreground, GWL_EXSTYLE) | GetWindowLongPtrW(h, GWL_EXSTYLE))
            & WS_EX_TOPMOST.0 as isize != 0 { return; }
        let _ = SetWindowPos(h, Some(foreground), 0, 0, 0, 0, KEEP);
    });
}

/// Watches for "show desktop" (Win+D, the taskbar corner). The desktop window becomes the
/// foreground window; pinned notes are lifted above it until something else takes focus.
pub fn init(_app: &AppHandle) {
    unsafe {
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        if hook.is_invalid() {
            log::warn!("foreground hook unavailable; pinned notes hide on show desktop");
        }
    }
}

unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    let desktop = is_desktop(hwnd);

    if desktop && !RAISED.swap(true, Ordering::SeqCst) {
        for h in pinned() {
            unsafe {
                if !IsWindowVisible(h).as_bool() {
                    continue;
                }
                if IsIconic(h).as_bool() {
                    let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
                }
                let _ = SetWindowPos(h, Some(HWND_TOPMOST), 0, 0, 0, 0, KEEP);
            }
        }
    } else if !desktop && RAISED.swap(false, Ordering::SeqCst) {
        for h in pinned() {
            to_bottom(h);
        }
    }
}

/// Rounds the corners of the acrylic backdrop and, for notes, keeps the window out of
/// Alt+Tab and the taskbar. Call before the window is first shown.
pub fn prepare_window(window: &WebviewWindow, tool: bool) {
    let Some(h) = hwnd(window) else { return };
    unsafe {
        let pref = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            h,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &pref as *const _ as *const c_void,
            std::mem::size_of_val(&pref) as u32,
        );
    }
    if tool {
        tool_style(h);
    }
    if window.label().starts_with("note-") || window.label() == crate::calendar::LABEL {
        NOTES.lock().unwrap().get_or_insert_with(HashSet::new).insert(h.0 as isize);
        let raw = h.0 as isize;
        let _ = window.run_on_main_thread(move || unsafe {
            let _ = SetWindowSubclass(HWND(raw as *mut c_void), Some(move_proc), 0x5042, 0);
        });
    }
}

/// tao rewrites the extended style whenever a window flag changes (shown, always-on-top),
/// so this runs again after each of those.
fn tool_style(h: HWND) {
    unsafe {
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        let want = (ex | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize);
        if want != ex {
            SetWindowLongPtrW(h, GWL_EXSTYLE, want);
            let _ = SetWindowPos(h, None, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOZORDER | KEEP);
        }
    }
}

pub fn set_pin_mode(window: &WebviewWindow, mode: PinMode) {
    let Some(h) = hwnd(window) else { return };
    {
        let mut set = PINNED.lock().unwrap();
        let set = set.get_or_insert_with(HashSet::new);
        if mode == PinMode::Desktop {
            set.insert(h.0 as isize);
        } else {
            set.remove(&(h.0 as isize));
        }
    }
    match mode {
        PinMode::Normal => {
            let _ = window.set_always_on_top(false);
            restore_normal_order(window);
        }
        PinMode::Top => {
            let _ = window.set_always_on_top(true);
        }
        PinMode::Desktop => {
            let _ = window.set_always_on_top(false);
            if !RAISED.load(Ordering::SeqCst) {
                to_bottom(h);
            }
        }
    }
    if unsafe { GetForegroundWindow() } == h { on_focus(window); }
    tool_style(h);
}

/// Focus temporarily lifts any note above even always-on-top notes.
pub fn on_focus(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
    if let Some(h) = hwnd(window) {
        unsafe { let _ = SetWindowPos(h, Some(HWND_TOPMOST), 0, 0, 0, 0, KEEP); }
        if window.label() != crate::windows::SETTINGS_LABEL { tool_style(h); }
    }
}

/// Restore the saved mode when focus moves elsewhere.
pub fn on_blur(window: &WebviewWindow, mode: PinMode) {
    let _ = window.set_always_on_top(mode == PinMode::Top);
    if mode == PinMode::Normal { restore_normal_order(window); }
    if mode == PinMode::Desktop && !RAISED.load(Ordering::SeqCst) {
        if let Some(h) = hwnd(window) {
            to_bottom(h);
        }
    }
    if window.label() != crate::windows::SETTINGS_LABEL {
        if let Some(h) = hwnd(window) { tool_style(h); }
    }
}

pub fn forget(window: &WebviewWindow) {
    if let Some(h) = hwnd(window) {
        if let Some(drags) = DRAGS.lock().unwrap().as_mut() { drags.remove(&(h.0 as isize)); }
        if let Some(set) = NOTES.lock().unwrap().as_mut() { set.remove(&(h.0 as isize)); }
        if let Some(set) = PINNED.lock().unwrap().as_mut() {
            set.remove(&(h.0 as isize));
        }
    }
}
