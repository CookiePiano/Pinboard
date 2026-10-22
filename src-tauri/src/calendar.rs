//! Calendar browsing is read-only until the user opens one selected note.
use crate::{model::{Note, PinMode}, platform, windows, AppState};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "calendar";

#[tauri::command]
pub fn calendar_pin(app: AppHandle) -> PinMode {
    app.state::<AppState>().local.lock().unwrap().calendar_pin
}

#[tauri::command]
pub fn set_calendar_pin(app: AppHandle, mode: PinMode) {
    let state = app.state::<AppState>();
    state.local.lock().unwrap().calendar_pin = mode;
    state.local_dirty.store(true, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window(LABEL) {
        platform::set_pin_mode(&window, mode);
    }
}

#[tauri::command]
pub async fn open_calendar(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window.unminimize().map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        return window.set_focus().map_err(|e| e.to_string());
    }
    let window = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::default())
        .on_navigation(crate::validation::navigation_allowed)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .title(tr!("메모 달력", "Note calendar"; "便签日历", "メモカレンダー"; "Calendrier des notes", "Calendario de notas"))
        .inner_size(760.0, 460.0).min_inner_size(640.0, 420.0).center()
        .decorations(false).transparent(true).shadow(true).skip_taskbar(true)
        .resizable(true).maximizable(false).visible(false).build().map_err(|e| e.to_string())?;
    platform::prepare_window(&window, true);
    platform::set_pin_mode(&window, calendar_pin(app.clone()));
    let calendar_window = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(true) => platform::on_focus(&calendar_window),
        tauri::WindowEvent::Focused(false) => platform::on_blur(&calendar_window, calendar_pin(calendar_window.app_handle().clone())),
        tauri::WindowEvent::Destroyed => platform::forget(&calendar_window),
        _ => {}
    });
    app.state::<AppState>().focus_on_ready.lock().unwrap().insert(LABEL.into());
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarNote { id: String, created_at: i64 }

#[tauri::command]
pub fn calendar_notes(app: AppHandle) -> Vec<CalendarNote> {
    app.state::<AppState>().store.lock().unwrap().live()
        .filter(|n| n.created_at > 0)
        .map(|n| CalendarNote { id: n.id.clone(), created_at: n.created_at }).collect()
}

fn in_day(note: &Note, start: i64, end: i64) -> bool {
    !note.is_deleted() && note.created_at > 0 && note.created_at >= start && note.created_at < end
}

fn validate_day(start: i64, end: i64) -> Result<(), String> {
    if end <= start || end.saturating_sub(start) > 26 * 60 * 60 * 1000 {
        Err(tr!("올바른 날짜를 선택하세요.", "Choose a valid date."; "请选择有效日期。", "有効な日付を選んでください。"; "Choisissez une date valide.", "Elige una fecha válida.").into())
    } else { Ok(()) }
}

fn excerpt(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut result: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() { result.push('…'); }
    result
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarPreview { id: String, title: String, preview: String, tint: String, created_at: i64 }

fn preview(note: &Note) -> CalendarPreview {
    let (title, body) = note.body.split_once('\n').unwrap_or((&note.body, ""));
    let title = if title.trim().is_empty() { tr!("제목 없는 메모", "Untitled note"; "无标题便签", "無題のメモ"; "Note sans titre", "Nota sin título") } else { excerpt(title.trim(), 80) };
    CalendarPreview { id: note.id.clone(), title, preview: excerpt(body.trim(), 240), tint: note.tint.clone(), created_at: note.created_at }
}

#[tauri::command]
pub fn calendar_day_notes(app: AppHandle, start: i64, end: i64) -> Result<Vec<CalendarPreview>, String> {
    validate_day(start, end)?;
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let mut notes: Vec<&Note> = store.live().filter(|n| in_day(n, start, end)).collect();
    notes.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
    Ok(notes.into_iter().map(preview).collect())
}

#[tauri::command]
pub async fn show_calendar_note(app: AppHandle, id: String) -> Result<(), String> {
    let note = app.state::<AppState>().store.lock().unwrap().get(&id).filter(|n| !n.is_deleted()).cloned()
        .ok_or_else(|| tr!("메모를 찾을 수 없습니다.", "Note not found."; "未找到便签。", "メモが見つかりません。"; "Note introuvable.", "No se encontró la nota."))?;
    if app.get_webview_window(&windows::note_label(&id)).is_none() {
        app.state::<AppState>().focus_on_ready.lock().unwrap().insert(windows::note_label(&id));
    }
    let window = windows::open_note(&app, &note, true).map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn day_filter_uses_creation_time_and_excludes_deleted_and_welcome() {
        let mut note = Note::new("test", "frost", 0.55);
        note.created_at = 1000; note.updated_at = 9000;
        assert!(in_day(&note, 1000, 2000));
        assert!(!in_day(&note, 0, 1000));
        note.deleted_at = Some(9000);
        assert!(!in_day(&note, 1000, 2000));
        note.deleted_at = None; note.created_at = 0;
        assert!(!in_day(&note, 0, 1000));
    }
    #[test]
    fn previews_are_bounded_unicode_text_without_images_or_format_payloads() {
        let mut note = Note::new("test", "sage", 0.55);
        note.body = format!("{}\n{}", "😀".repeat(100), "본문".repeat(1000));
        let card = preview(&note);
        assert_eq!(card.title.chars().count(), 81);
        assert_eq!(card.preview.chars().count(), 241);
        assert!(card.title.ends_with('…'));
        let json = serde_json::to_value(card).unwrap();
        assert!(json.get("background").is_none());
        assert!(json.get("formats").is_none());
        note.body = "  \n본문만 있는 메모".into();
        assert!(!preview(&note).title.trim().is_empty());
    }
    #[test]
    fn validates_day_including_dst_but_rejects_unbounded_queries() {
        assert!(validate_day(0, 25 * 60 * 60 * 1000).is_ok());
        assert!(validate_day(1, 1).is_err());
        assert!(validate_day(0, i64::MAX).is_err());
    }
}
