//! Note and settings windows: creation, placement, and keeping per-device window state.

use crate::model::{Note, PinMode};
use crate::{platform, AppState};
use std::sync::atomic::Ordering;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

pub const SETTINGS_LABEL: &str = "settings";
const CASCADE: f64 = 28.0;

pub fn note_label(id: &str) -> String {
    format!("note-{id}")
}

fn glass() -> tauri::utils::config::WindowEffectsConfig {
    EffectsBuilder::new().effect(Effect::Acrylic).build()
}

/// Notes at or below this opacity drop the blur so desktop icons show through clearly.
/// Keep in sync with `BLUR_MIN` in `src/lib/tints.ts`.
const BLUR_MIN: f32 = 0.3;

static BLURRED: std::sync::Mutex<Option<std::collections::HashMap<String, bool>>> = std::sync::Mutex::new(None);

fn update_glass<E>(cache: &mut std::collections::HashMap<String, bool>, label: &str, blur: bool, apply: impl FnOnce() -> Result<(), E>) -> Result<(), E> {
    if cache.get(label) == Some(&blur) { return Ok(()); }
    apply()?;
    cache.insert(label.to_owned(), blur);
    Ok(())
}

fn forget_glass(label: &str) {
    if let Some(cache) = BLURRED.lock().unwrap().as_mut() { cache.remove(label); }
}

/// Turns the acrylic backdrop on or off for a note's opacity, only when it changes.
pub fn apply_glass(window: &WebviewWindow, opacity: f32) {
    let blur = opacity > BLUR_MIN;
    let mut map = BLURRED.lock().unwrap();
    let map = map.get_or_insert_with(Default::default);
    if let Err(error) = update_glass(map, window.label(), blur, || window.set_effects(if blur { Some(glass()) } else { None })) {
        log::warn!("cannot apply note backdrop: {error}");
    }
}

/// Shows the window for a note, creating it if needed.
pub fn open_note(app: &AppHandle, note: &Note, focus: bool) -> tauri::Result<WebviewWindow> {
    open_note_data(app, &note.id, note.opacity, focus)
}

fn open_note_data(app: &AppHandle, id: &str, opacity: f32, focus: bool) -> tauri::Result<WebviewWindow> {
    let state = app.state::<AppState>();
    let label = note_label(id);
    let ws = {
        let mut local = state.local.lock().unwrap();
        let ws = local.windows.entry(id.to_owned()).or_default();
        ws.visible = true;
        ws.clone()
    };
    state.local_dirty.store(true, Ordering::SeqCst);

    if let Some(w) = app.get_webview_window(&label) {
        w.show()?;
        if focus {
            w.set_focus()?;
        }
        return Ok(w);
    }

    let (x, y) = match (ws.x, ws.y) {
        (Some(x), Some(y)) if on_screen(app, x, y) => (x, y),
        _ => default_position(app, ws.width),
    };
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::default())
        .on_navigation(crate::validation::navigation_allowed)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .title("Pinboard")
        .inner_size(ws.width, ws.height)
        .min_inner_size(180.0, 120.0)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .skip_taskbar(true)
        .resizable(true)
        .maximizable(false)
        .visible(false)
        .focused(focus)
        .build()?;
    // A restored note can reuse a label after its previous native window closed.
    // Cached effects belong to that window instance, not to the persisted note.
    forget_glass(&label);
    apply_glass(&window, opacity);

    if let Some(ws) = state.local.lock().unwrap().windows.get_mut(id) {
        ws.x = Some(x);
        ws.y = Some(y);
    }
    platform::prepare_window(&window, true);
    platform::set_pin_mode(&window, ws.pin);
    track(&window, id.to_owned());
    Ok(window)
}

/// Records moves and resizes, and turns close into hide so the note stays in the tray.
fn track(window: &WebviewWindow, id: String) {
    let w = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            forget_glass(w.label());
            return;
        }
        let app = w.app_handle();
        let state = app.state::<AppState>();
        let scale = w.scale_factor().unwrap_or(1.0);
        if state.restoring.load(Ordering::SeqCst) { return; }
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            // Alt+F4 follows the same flush/retry path as the editor's Hide button.
            let _ = w.emit("note://hide-requested", ());
            return;
        }
        let mut local = state.local.lock().unwrap();
        let Some(ws) = local.windows.get_mut(&id) else { return };
        match event {
            WindowEvent::Moved(pos) => {
                let p = pos.to_logical::<f64>(scale);
                // Minimized windows report a far off-screen position.
                if p.x > -10000.0 {
                    ws.x = Some(p.x);
                    ws.y = Some(p.y);
                }
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    let s = size.to_logical::<f64>(scale);
                    ws.width = s.width;
                    ws.height = s.height;
                }
            }
            WindowEvent::Focused(false) => platform::on_blur(&w, ws.pin),
            WindowEvent::Focused(true) => platform::on_focus(&w),
            _ => return,
        }
        state.local_dirty.store(true, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod glass_tests {
    use super::update_glass;
    use std::{cell::Cell, collections::HashMap};

    #[test]
    fn failed_effect_is_retried_without_discarding_the_last_success() {
        let mut cache = HashMap::new();
        assert!(update_glass(&mut cache, "note-a", true, || Err("unavailable")).is_err());
        assert!(!cache.contains_key("note-a"));
        update_glass(&mut cache, "note-a", true, || Ok::<_, &str>(())).unwrap();
        assert!(update_glass(&mut cache, "note-a", false, || Err("unavailable")).is_err());
        assert_eq!(cache.get("note-a"), Some(&true));
        update_glass(&mut cache, "note-a", false, || Ok::<_, &str>(())).unwrap();
        assert_eq!(cache.get("note-a"), Some(&false));
    }

    #[test]
    fn unchanged_effect_is_skipped_until_the_window_is_recreated() {
        let mut cache = HashMap::new();
        let calls = Cell::new(0);
        let apply = || { calls.set(calls.get() + 1); Ok::<_, ()>(()) };
        update_glass(&mut cache, "note-a", true, apply).unwrap();
        update_glass(&mut cache, "note-a", true, apply).unwrap();
        assert_eq!(calls.get(), 1);
        cache.remove("note-a");
        update_glass(&mut cache, "note-a", true, apply).unwrap();
        assert_eq!(calls.get(), 2);
        update_glass(&mut cache, "note-b", true, apply).unwrap();
        assert_eq!(calls.get(), 3);
    }
}

fn on_screen(app: &AppHandle, x: f64, y: f64) -> bool {
    app.available_monitors().unwrap_or_default().iter().any(|m| {
        let s = m.scale_factor();
        let p = m.position().to_logical::<f64>(s);
        let z = m.size().to_logical::<f64>(s);
        x + 40.0 >= p.x && x + 40.0 <= p.x + z.width && y + 20.0 >= p.y && y + 20.0 <= p.y + z.height
    })
}

/// New notes line up from the top-right corner of the primary monitor, each one cascaded.
fn default_position(app: &AppHandle, width: f64) -> (f64, f64) {
    let open = app.webview_windows().keys().filter(|l| l.starts_with("note-")).count() as f64;
    let step = (open % 8.0) * CASCADE;
    match app.primary_monitor().ok().flatten() {
        Some(m) => {
            let s = m.scale_factor();
            let p = m.position().to_logical::<f64>(s);
            let z = m.size().to_logical::<f64>(s);
            (p.x + z.width - width - 56.0 - step, p.y + 72.0 + step)
        }
        None => (120.0 + step, 120.0 + step),
    }
}

/// Places a note just below and to the right of another one.
pub fn position_near(app: &AppHandle, near: &str) -> Option<LogicalPosition<f64>> {
    let w = app.get_webview_window(&note_label(near))?;
    let scale = w.scale_factor().ok()?;
    let p = w.outer_position().ok()?.to_logical::<f64>(scale);
    Some(LogicalPosition::new(p.x + CASCADE, p.y + CASCADE))
}

pub fn set_pin(app: &AppHandle, id: &str, mode: PinMode) {
    let state = app.state::<AppState>();
    if let Some(ws) = state.local.lock().unwrap().windows.get_mut(id) {
        ws.pin = mode;
    }
    state.local_dirty.store(true, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window(&note_label(id)) {
        platform::set_pin_mode(&w, mode);
    }
}

pub fn hide_note(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    if let Some(ws) = state.local.lock().unwrap().windows.get_mut(id) {
        ws.visible = false;
    }
    state.local_dirty.store(true, Ordering::SeqCst);
    if let Some(w) = app.get_webview_window(&note_label(id)) {
        let _ = w.hide();
    }
}

pub fn close_note(app: &AppHandle, id: &str) {
    if let Some(w) = app.get_webview_window(&note_label(id)) {
        platform::forget(&w);
        let _ = w.destroy();
    }
}

pub fn show_all(app: &AppHandle) {
    let notes: Vec<_> = app.state::<AppState>().store.lock().unwrap().live().map(|n| (n.id.clone(), n.opacity)).collect();
    for (id, opacity) in notes {
        if let Err(e) = open_note_data(app, &id, opacity, false) {
            log::error!("cannot open note window: {e}");
        }
    }
}

pub fn hide_all(app: &AppHandle) {
    let ids: Vec<String> = app.state::<AppState>().store.lock().unwrap().live().map(|n| n.id.clone()).collect();
    for id in ids {
        hide_note(app, &id);
    }
}

/// Opens the notes this device shows on startup.
pub fn restore(app: &AppHandle) {
    let state = app.state::<AppState>();
    let notes: Vec<_> = state.store.lock().unwrap().live().map(|n| (n.id.clone(), n.opacity)).collect();
    for (id, opacity) in notes {
        let visible = state.local.lock().unwrap().windows.get(&id).is_none_or(|w| w.visible);
        if visible {
            if let Err(e) = open_note_data(app, &id, opacity, false) {
                log::error!("cannot open note window: {e}");
            }
        }
    }
}

/// Reflects notes changed by a sync in their windows.
pub fn apply_remote_changes(app: &AppHandle, changed: Vec<Note>, purged: Vec<String>) {
    crate::commands::notes_changed(app, None, true);
    let state = app.state::<AppState>();
    for mut note in changed {
        let label = note_label(&note.id);
        if note.is_deleted() {
            close_note(app, &note.id);
            continue;
        }
        if crate::welcome::localize(&mut note, state.settings.lock().unwrap().language) {
            if let Err(e) = state.store.lock().unwrap().put(note.clone()) {
                log::warn!("cannot localize welcome note: {e}");
            }
        }
        if let Some(w) = app.get_webview_window(&label) {
            apply_glass(&w, note.opacity);
            let _ = app.emit_to(&label, "note://updated", &note);
            continue;
        }
        let hidden = state.local.lock().unwrap().windows.get(&note.id).is_some_and(|w| !w.visible);
        if !hidden {
            if let Err(e) = open_note(app, &note, false) {
                log::error!("cannot open synced note: {e}");
            }
        }
    }
    for id in purged {
        close_note(app, &id);
        state.local.lock().unwrap().windows.remove(&id);
        state.local_dirty.store(true, Ordering::SeqCst);
    }
}

pub fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    // All callers run off the UI thread. Serialize the check/build pair so a
    // startup request and a second-instance request cannot create two settings windows.
    static OPEN_SETTINGS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _opening = OPEN_SETTINGS.lock().unwrap();
    if let Some(w) = app.get_webview_window(SETTINGS_LABEL) {
        w.unminimize()?;
        w.show()?;
        return w.set_focus();
    }
    let window = WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::default())
        .on_navigation(crate::validation::navigation_allowed)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .title(tr!("Pinboard 설정", "Pinboard Settings"; "Pinboard 设置", "Pinboard 設定"; "Paramètres de Pinboard", "Configuración de Pinboard"))
        .inner_size(920.0, 780.0)
        .min_inner_size(640.0, 460.0)
        .center()
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .resizable(true)
        .visible(false)
        .effects(glass())
        .build()?;
    platform::prepare_window(&window, false);
    let app = app.clone();
    let settings_window = window.clone();
    window.on_window_event(move |event| {
        match event {
            WindowEvent::Focused(true) => platform::on_focus(&settings_window),
            WindowEvent::Focused(false) => platform::on_blur(&settings_window, PinMode::Normal),
            _ => {}
        }
        if matches!(event, WindowEvent::Focused(false) | WindowEvent::Destroyed) {
            crate::shortcuts::set_recording(&app.state::<AppState>(), false);
        }
    });
    Ok(())
}
