use crate::model::{now_ms, Note, PinMode, TINTS};
use crate::settings::{Settings, SyncKind};
use crate::sync::gdrive::http_client;
use crate::sync::worker::{SyncMsg, SyncStatus};
use crate::sync::{oauth, secrets};
use crate::windows;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

type Res<T> = Result<T, String>;

#[tauri::command]
pub async fn fit_settings(window: WebviewWindow, height: f64) -> Res<()> {
    if window.label() != windows::SETTINGS_LABEL || !height.is_finite() { return Err("Invalid settings size.".into()); }
    let Some(monitor) = window.current_monitor().map_err(err)? else { return Ok(()) };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let max_height = (area.size.height as f64 / scale - 24.0).max(460.0);
    let size = window.inner_size().map_err(err)?.to_logical::<f64>(scale);
    let desired = height.ceil().max(780.0).min(max_height);
    window.set_size(tauri::LogicalSize::new(size.width, desired)).map_err(err)?;
    let position = window.outer_position().map_err(err)?.to_logical::<f64>(scale);
    let top = area.position.y as f64 / scale + 12.0;
    let bottom = top + max_height;
    if position.y < top || position.y + desired > bottom {
        window.set_position(tauri::LogicalPosition::new(position.x, (bottom - desired).max(top))).map_err(err)?;
    }
    Ok(())
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteView {
    note: Note,
    pin: PinMode,
}

#[tauri::command]
pub fn get_note(state: State<AppState>, id: String) -> Res<NoteView> {
    let note = state.store.lock().unwrap().get(&id).cloned().ok_or(tr!("메모를 찾을 수 없습니다.", "Note not found."; "未找到便签。", "メモが見つかりません。"; "Note introuvable.", "No se encontró la nota."))?;
    let pin = state.local.lock().unwrap().windows.get(&id).map(|w| w.pin).unwrap_or_default();
    Ok(NoteView { note, pin })
}

/// Shows a window once its page has rendered, so it never flashes empty.
#[tauri::command]
pub fn window_ready(window: WebviewWindow, state: State<AppState>) -> Res<()> {
    // Isolated UI verification must not steal the user's focus or keystrokes.
    if std::env::var_os("PINBOARD_DATA_DIR").is_some() && std::env::var("PINBOARD_TEST_HIDDEN").as_deref() == Ok("1") {
        return Ok(());
    }
    window.show().map_err(err)?;
    let label = window.label();
    crate::platform::prepare_window(&window, label.starts_with("note-") || label == crate::calendar::LABEL);
    let focus = state.focus_on_ready.lock().unwrap().remove(window.label());
    if focus || label == windows::SETTINGS_LABEL {
        window.set_focus().map_err(err)?;
    }
    Ok(())
}

/// Creating a window from a synchronous command deadlocks on Windows, so this one is async.
#[tauri::command]
pub async fn create_note(app: AppHandle, near: Option<String>) -> Res<Note> {
    crate::new_note(&app, near.as_deref()).map_err(err)
}

#[tauri::command]
pub fn drag_note(window: WebviewWindow, phase: String) -> Res<bool> {
    if !(window.label().starts_with("note-") || window.label() == crate::calendar::LABEL) || !matches!(phase.as_str(), "start" | "move" | "end") {
        return Err("Invalid note drag request.".into());
    }
    crate::platform::drag_note(&window, &phase).map_err(err)?;
    // Windows uses captured pointer movement for snapping. GTK owns the drag
    // itself and may consume pointerup, so the page must release its capture.
    Ok(cfg!(windows))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NotePatch {
    pub(crate) body: Option<String>,
    pub(crate) tint: Option<String>,
    pub(crate) opacity: Option<f32>,
    pub(crate) formats: Option<Vec<crate::model::TextFormat>>,
    // An empty image removes a background. Omission preserves it.
    pub(crate) background: Option<crate::model::NoteBackground>,
    pub(crate) font: Option<crate::fonts::NoteFont>,
    #[serde(default)]
    pub(crate) reset_font_sizes: bool,
}

#[tauri::command]
pub fn update_note(app: AppHandle, window: WebviewWindow, state: State<AppState>, id: String, patch: NotePatch) -> Res<crate::store::SharedNote> {
    let _gate = state.data_gate.try_read().map_err(|_| "Restore in progress.")?;
    if state.restoring.load(Ordering::SeqCst) { return Err("Restore in progress.".into()); }
    let opacity_changed = patch.opacity.is_some();
    let (note, changed) = patch_note_shared(&state, &id, patch)?;
    if changed { publish_patch(&app, &note, opacity_changed, window.label() != windows::note_label(&id)); }
    Ok(note)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAck { updated_at: i64 }

/// Content saves already live in the originating editor. Do not echo images back to it.
#[tauri::command]
pub async fn save_note_content(app: AppHandle, window: WebviewWindow, id: String, body: String, formats: Vec<crate::model::TextFormat>) -> Res<SaveAck> {
    if window.label() != windows::note_label(&id) { return Err("Invalid note window.".into()); }
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _gate = state.data_gate.try_read().map_err(|_| "Restore in progress.")?;
        if state.restoring.load(Ordering::SeqCst) { return Err("Restore in progress.".into()); }
        let (note, changed) = patch_note_shared(&state, &id, NotePatch { body: Some(body), formats: Some(formats), ..Default::default() })?;
        if changed {
            let _ = state.sync_tx.send(SyncMsg::LocalChanged);
            notes_changed(&app, Some(&id), false);
        }
        Ok(SaveAck { updated_at: note.updated_at })
    }).await.map_err(err)?
}

#[derive(Serialize, Clone)]
pub struct NotesChanged<'a> { id: Option<&'a str>, design: bool }

pub(crate) fn notes_changed(app: &AppHandle, id: Option<&str>, design: bool) {
    let change = NotesChanged { id, design };
    let _ = app.emit_to(windows::SETTINGS_LABEL, "notes://changed", &change);
    let _ = app.emit_to(crate::calendar::LABEL, "notes://changed", &change);
}

/// Window, event and sync side of an edit. MCP calls it after releasing its locks.
pub(crate) fn after_patch(app: &AppHandle, note: &Note, opacity_changed: bool) {
    publish_patch(app, note, opacity_changed, true);
}

fn publish_patch(app: &AppHandle, note: &Note, opacity_changed: bool, echo: bool) {
    if opacity_changed {
        if let Some(w) = app.get_webview_window(&windows::note_label(&note.id)) {
            windows::apply_glass(&w, note.opacity);
        }
    }
    let _ = app.state::<AppState>().sync_tx.send(SyncMsg::LocalChanged);
    if echo { let _ = app.emit_to(windows::note_label(&note.id), "note://updated", note); }
    notes_changed(app, Some(&note.id), true);
}

/// Store side of an edit, shared by note windows and MCP. The caller holds the data gate.
/// Returns whether anything changed.
pub(crate) fn patch_note(state: &AppState, id: &str, patch: NotePatch) -> Res<(Note, bool)> {
    patch_note_shared(state, id, patch).map(|(note, changed)| ((*note).clone(), changed))
}

fn patch_note_shared(state: &AppState, id: &str, patch: NotePatch) -> Res<(crate::store::SharedNote, bool)> {
    let note = {
        let mut store = state.store.lock().unwrap();
        let mut note = store.get(id).cloned().ok_or(tr!("메모를 찾을 수 없습니다.", "Note not found."; "未找到便签。", "メモが見つかりません。"; "Note introuvable.", "No se encontró la nota."))?;
        let before = store.hash(id).expect("existing note has a hash");
        if let Some(body) = patch.body {
            note.body = body;
            if patch.formats.is_none() { note.formats.clear(); }
        }
        if let Some(formats) = patch.formats {
            let length = note.body.encode_utf16().count();
            let mut end = 0;
            for f in &formats {
                if f.from < end || f.from >= f.to || f.to > length
                    || f.size.is_some_and(|s| !s.is_finite() || !(6.0..=96.0).contains(&s))
                    || f.highlight.as_ref().is_some_and(|c| !crate::model::hex_color(c)) {
                    return Err(tr!("텍스트 서식 범위가 올바르지 않습니다.", "Invalid text formatting range."; "文本格式范围无效。", "テキストの書式範囲が無効です。"; "Plage de mise en forme du texte non valide.", "Rango de formato de texto no válido.").into());
                }
                end = f.to;
            }
            note.formats = formats;
        }
        if let Some(mut background) = patch.background {
            if background.image.is_empty() { note.background = None; }
            else {
                if !background.opacity.is_finite() || !background.image.starts_with("data:image/")
                    || !["data:image/jpeg;base64,", "data:image/png;base64,", "data:image/webp;base64,"].iter().any(|p| background.image.starts_with(p))
                    || background.image.len() > 8 * 1024 * 1024 {
                    return Err(tr!("배경 이미지를 읽을 수 없습니다.", "Cannot read the background image."; "无法读取背景图片。", "背景画像を読み込めません。"; "Impossible de lire l'image d'arrière-plan.", "No se puede leer la imagen de fondo.").into());
                }
                background.opacity = background.opacity.clamp(0.0, 1.0);
                note.background = Some(background);
            }
        }
        if let Some(font) = patch.font { note.set_font(font, patch.reset_font_sizes)?; }
        if let Some(tint) = patch.tint.filter(|t| TINTS.contains(&t.as_str())) {
            note.tint = tint;
        }
        if let Some(o) = patch.opacity {
            note.opacity = o.clamp(0.0, 1.0);
        }
        let hash = note.content_hash();
        if hash == before {
            return Ok((crate::store::SharedNote::hashed(note, hash), false));
        }
        note.updated_at = now_ms();
        note.device_id = state.device_id.clone();
        let note = crate::store::SharedNote::hashed(note, hash);
        store.put_shared(note.clone()).map_err(err)?;
        note
    };
    Ok((note, true))
}

#[tauri::command]
pub fn delete_note(app: AppHandle, state: State<AppState>, id: String) -> Res<()> {
    let _gate = state.data_gate.try_read().map_err(|_| "Restore in progress.")?;
    tombstone_note(&state, &id)?;
    after_delete(&app, &id);
    Ok(())
}

/// Store side of a delete, shared by note windows, the calendar and MCP. The caller holds the data gate.
pub(crate) fn tombstone_note(state: &AppState, id: &str) -> Res<()> {
    {
        let mut store = state.store.lock().unwrap();
        let mut note = store.get(id).cloned().ok_or(tr!("메모를 찾을 수 없습니다.", "Note not found."; "未找到便签。", "メモが見つかりません。"; "Note introuvable.", "No se encontró la nota."))?;
        let now = now_ms();
        note.deleted_at = Some(now);
        note.updated_at = now;
        note.device_id = state.device_id.clone();
        store.put(note).map_err(err)?;
    }
    state.local.lock().unwrap().windows.remove(id);
    state.local_dirty.store(true, Ordering::SeqCst);
    Ok(())
}

pub(crate) fn after_delete(app: &AppHandle, id: &str) {
    windows::close_note(app, id);
    let _ = app.state::<AppState>().sync_tx.send(SyncMsg::LocalChanged);
    notes_changed(app, Some(id), true);
}

#[tauri::command]
pub fn hide_note(app: AppHandle, id: String) {
    windows::hide_note(&app, &id);
}

#[tauri::command]
pub fn set_pin_mode(app: AppHandle, id: String, mode: PinMode) {
    windows::set_pin(&app, &id, mode);
}

#[tauri::command]
pub async fn open_settings(app: AppHandle, id: Option<String>) -> Res<()> {
    *app.state::<AppState>().settings_note.lock().unwrap() = id.clone();
    let _ = app.emit_to(windows::SETTINGS_LABEL, "settings://note", &id);
    windows::open_settings(&app).map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignContext { notes: Vec<Note>, selected: Option<String> }

#[tauri::command]
pub fn design_context(state: State<AppState>) -> DesignContext {
    let mut notes: Vec<Note> = state.store.lock().unwrap().live().cloned().collect();
    notes.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
    DesignContext { notes, selected: state.settings_note.lock().unwrap().clone() }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesignSummary { id: String, title: String, tint: String, created_at: i64 }

fn design_summary(note: &Note) -> DesignSummary {
    DesignSummary { id: note.id.clone(), title: note.body.split('\n').next().unwrap_or("").trim().chars().take(80).collect(), tint: note.tint.clone(), created_at: note.created_at }
}

#[derive(Serialize)]
pub struct DesignSummaries { notes: Vec<DesignSummary>, selected: Option<String> }

#[tauri::command]
pub fn design_note_summaries(state: State<AppState>) -> DesignSummaries {
    let mut notes: Vec<_> = state.store.lock().unwrap().live().map(design_summary).collect();
    notes.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.cmp(&b.id)));
    DesignSummaries { notes, selected: state.settings_note.lock().unwrap().clone() }
}

#[derive(Serialize)]
pub struct NoteDesign { #[serde(flatten)] summary: DesignSummary, background: Option<crate::model::NoteBackground> }

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn design_lists_and_save_acks_have_bounded_payloads() {
        let mut note = Note::new("test", "frost", 0.55);
        note.body = format!("{}\n{}", "😀".repeat(1000), "body".repeat(250_000));
        note.background = Some(crate::model::NoteBackground { image: "data:image/png;base64,AAAA".into(), opacity: 0.3 });
        let summary = serde_json::to_value(design_summary(&note)).unwrap();
        assert!(summary["title"].as_str().unwrap().chars().count() <= 80);
        assert!(summary.get("body").is_none() && summary.get("formats").is_none() && summary.get("background").is_none());
        let ack = serde_json::to_value(SaveAck { updated_at: 42 }).unwrap();
        assert_eq!(ack, serde_json::json!({ "updatedAt": 42 }));
    }
}

#[tauri::command]
pub fn get_note_design(state: State<AppState>, id: String) -> Res<NoteDesign> {
    let store = state.store.lock().unwrap();
    let note = store.get(&id).filter(|n| !n.is_deleted()).ok_or("Note not found.")?;
    Ok(NoteDesign { summary: design_summary(note), background: note.background.clone() })
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub async fn list_fonts(language: crate::i18n::Lang, force: Option<bool>) -> Res<Vec<crate::fonts::FontFamily>> {
    tauri::async_runtime::spawn_blocking(move || if force.unwrap_or(false) { crate::fonts::installed_with_refresh(language, true) } else { crate::fonts::installed(language) }).await.map_err(err)?
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Res<Settings> {
    let _gate = state.data_gate.try_read().map_err(|_| "Restore in progress.")?;
    let mut settings = settings;
    settings.note_font.validate()?;
    settings.selection_tools.validate()?;
    // Async commands execute off the UI thread; shortcut registration dispatches to it.
    settings.new_note_shortcut = settings.new_note_shortcut.trim().to_string();
    crate::shortcuts::parse(&settings.new_note_shortcut)?;
    let (reconfigure, language_changed) = {
        let mut cur = state.settings.lock().unwrap();
        // The signed-in account is owned by the connect and disconnect commands.
        settings.gdrive_account = cur.gdrive_account.clone();
        let changed = cur.sync != settings.sync
            || cur.folder_root != settings.folder_root
            || cur.gdrive_client_id != settings.gdrive_client_id;
        let language_changed = cur.language != settings.language;
        crate::shortcuts::apply(&app, &settings)?;
        if let Err(e) = crate::store::write_json(&state.data_dir.join("settings.json"), &settings) {
            if let Err(rollback) = crate::shortcuts::apply(&app, &cur) {
                log::error!("cannot restore shortcut after failed settings save: {rollback}");
            }
            return Err(err(e));
        }
        *cur = settings.clone();
        (changed, language_changed)
    };
    crate::i18n::set(settings.language);
    if language_changed {
        if let Err(e) = crate::refresh_language(&app) {
            log::warn!("cannot refresh native language: {e}");
        }
    }
    if reconfigure || language_changed {
        let _ = state.sync_tx.send(SyncMsg::Reconfigure);
    }
    let _ = app.emit("settings://changed", &settings);
    Ok(settings)
}

#[tauri::command]
pub fn set_shortcut_recording(window: WebviewWindow, state: State<AppState>, recording: bool) {
    if window.label() == windows::SETTINGS_LABEL {
        crate::shortcuts::set_recording(&state, recording);
    }
}

#[tauri::command]
pub fn get_sync_status(state: State<AppState>) -> SyncStatus {
    state.sync_status.lock().unwrap().clone()
}

#[tauri::command]
pub fn sync_now(state: State<AppState>) {
    let _ = state.sync_tx.send(SyncMsg::Now);
}

#[tauri::command]
pub fn has_gdrive_secret() -> bool {
    secrets::get(secrets::GDRIVE_CLIENT_SECRET).is_some()
}

/// Signs in with Google in the browser and switches sync to Google Drive.
/// An empty `client_secret` reuses the one already stored.
#[tauri::command]
pub async fn gdrive_connect(app: AppHandle, client_id: String, client_secret: String) -> Res<String> {
    let client_id = client_id.trim().to_string();
    let client_secret = match client_secret.trim() {
        "" => secrets::get(secrets::GDRIVE_CLIENT_SECRET).ok_or(tr!("클라이언트 보안 비밀번호를 입력하세요.", "Enter the client secret."; "请输入客户端密钥。", "クライアントシークレットを入力してください。"; "Saisissez le secret client.", "Introduce el secreto de cliente."))?,
        s => s.to_string(),
    };
    if client_id.is_empty() {
        return Err(tr!("클라이언트 ID를 입력하세요.", "Enter the client ID."; "请输入客户端 ID。", "クライアント ID を入力してください。"; "Saisissez l'ID client.", "Introduce el ID de cliente.").into());
    }
    let opener = app.clone();
    let (id, secret) = (client_id.clone(), client_secret.clone());
    let tokens = tauri::async_runtime::spawn_blocking(move || {
        oauth::sign_in(
            &http_client(),
            &id,
            &secret,
            |url| opener.opener().open_url(url, None::<&str>).map_err(err),
            Duration::from_secs(300),
        )
    })
    .await
    .map_err(err)?
    .map_err(err)?;

    let refresh = tokens.refresh_token.ok_or(tr!("Google이 갱신 토큰을 보내지 않았습니다. 다시 시도하세요.", "Google did not return a refresh token. Try again."; "Google 未返回刷新令牌。请重试。", "Google から更新トークンが返されませんでした。再試行してください。"; "Google n'a pas renvoyé de jeton d'actualisation. Réessayez.", "Google no devolvió un token de actualización. Inténtalo de nuevo."))?;
    secrets::set(secrets::GDRIVE_CLIENT_SECRET, &client_secret)?;
    secrets::set(secrets::GDRIVE_REFRESH_TOKEN, &refresh)?;
    let account = tokens.email.unwrap_or_else(|| tr!("Google 계정", "Google account"; "Google 账号", "Google アカウント"; "Compte Google", "Cuenta de Google"));

    let state = app.state::<AppState>();
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.gdrive_client_id = client_id;
        s.gdrive_account = Some(account.clone());
        s.sync = SyncKind::Gdrive;
        s.clone()
    };
    crate::save_settings_file(&state).map_err(err)?;
    let _ = state.sync_tx.send(SyncMsg::Reconfigure);
    let _ = app.emit("settings://changed", &settings);
    Ok(account)
}

#[tauri::command]
pub fn gdrive_disconnect(app: AppHandle, state: State<AppState>) -> Res<()> {
    secrets::delete(secrets::GDRIVE_REFRESH_TOKEN)?;
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.gdrive_account = None;
        if s.sync == SyncKind::Gdrive {
            s.sync = SyncKind::Off;
        }
        s.clone()
    };
    crate::save_settings_file(&state).map_err(err)?;
    let _ = state.sync_tx.send(SyncMsg::Reconfigure);
    let _ = app.emit("settings://changed", &settings);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    data_dir: String,
    note_count: usize,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<AppState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        data_dir: state.data_dir.display().to_string(),
        note_count: state.store.lock().unwrap().live().count(),
    }
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle, state: State<AppState>) -> Res<()> {
    app.opener().open_path(state.data_dir.display().to_string(), None::<&str>).map_err(err)
}


/// Finds a Google Drive for Desktop folder to offer as the sync folder.
#[tauri::command]
pub fn suggest_folder() -> Option<String> {
    let candidates: Vec<std::path::PathBuf> = if cfg!(windows) {
        ('C'..='Z')
            .flat_map(|d| ["내 드라이브", "My Drive"].map(|n| format!("{d}:\\{n}").into()))
            .collect()
    } else {
        let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
        vec![home.join("GoogleDrive"), home.join("Google Drive")]
    };
    candidates.into_iter().find(|p| p.is_dir()).map(|p| p.display().to_string())
}
