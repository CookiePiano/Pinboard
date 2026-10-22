//! Portable, versioned backups. Archives never contain credentials or per-PC MCP grants.
use crate::{model::Note, settings::{Settings, SyncKind}, store::{LocalState, read_json, write_json}, windows, AppState};
use crate::sync::worker::SyncMsg;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs::{self, File, OpenOptions}, io::{self, Write}, path::{Path, PathBuf}, sync::{mpsc, atomic::Ordering}, time::Duration};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_autostart::ManagerExt;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions, CompressionMethod};

type Res<T> = Result<T, String>;
const JOURNAL: &str = "restore-journal.json";
const FILES: [&str; 4] = ["notes", "settings.json", "local-state.json", "ui-preferences.json"];
const MAX_JSON: u64 = 64 * 1024 * 1024;
const MAX_ARCHIVE: u64 = MAX_JSON + 1024 * 1024;
fn err(e: impl std::fmt::Display) -> String { e.to_string() }

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    #[serde(default)]
    mcp_providers: Vec<String>,
}

impl Preferences {
    fn validate(&self) -> Res<()> {
        if self.mcp_providers.iter().any(|p| !["codex", "claude", "claude_cli", "antigravity", "gemini"].contains(&p.as_str())) {
            return Err("Invalid AI app preference.".into());
        }
        Ok(())
    }
}

#[tauri::command]
pub fn get_transfer_preferences(window: WebviewWindow, state: tauri::State<AppState>) -> Res<Option<Preferences>> {
    settings_window(&window)?;
    Ok(read_json(&state.data_dir.join("ui-preferences.json")))
}

#[tauri::command]
pub fn save_transfer_preferences(window: WebviewWindow, state: tauri::State<AppState>, preferences: Preferences) -> Res<()> {
    settings_window(&window)?;
    preferences.validate()?;
    let _gate = state.data_gate.try_read().map_err(|_| "Restore in progress.")?;
    write_json(&state.data_dir.join("ui-preferences.json"), &preferences).map_err(err)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Backup<N = Note> {
    format: String,
    version: u32,
    created_at: i64,
    settings: Settings,
    local_state: LocalState,
    notes: Vec<N>,
    preferences: Preferences,
    autostart: Option<bool>,
    mcp_enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    note_count: usize,
    backup_path: String,
    preferences: Preferences,
    warnings: Vec<String>,
}

use crate::validation::canonical_id;

struct LimitedWriter<W> { inner: W, remaining: u64 }
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Backup exceeds the 64 MiB limit"));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}

impl<N: std::borrow::Borrow<Note>> Backup<N> {
    /// Portable content must not grant local privileges or select a network destination.
    fn restrict_import<C>(&mut self, current: &Backup<C>) {
        self.settings.sync = SyncKind::Off;
        self.settings.folder_root = None;
        self.settings.gdrive_account = current.settings.gdrive_account.clone();
        self.settings.gdrive_client_id = current.settings.gdrive_client_id.clone();
        self.mcp_enabled = current.mcp_enabled;
        self.autostart = current.autostart;
    }

    fn validate(&self) -> Res<()> {
        if self.format != "Pinboard" || self.version != 1 {
            return Err(tr!("지원하지 않는 Pinboard 백업 형식입니다.", "Unsupported Pinboard backup format."; "不支持此 Pinboard 备份格式。", "対応していない Pinboard バックアップ形式です。"; "Format de sauvegarde Pinboard non pris en charge.", "Formato de copia de seguridad de Pinboard no compatible."));
        }
        self.settings.note_font.clone().validate()?;
        self.settings.selection_tools.validate()?;
        crate::shortcuts::parse(&self.settings.new_note_shortcut)?;
        self.preferences.validate()?;
        let mut ids = HashSet::new();
        if self.notes.len() > 10_000 || self.local_state.windows.len() > 10_000 {
            return Err("Backup contains too many notes or windows.".into());
        }
        for n in &self.notes {
            let n = n.borrow();
            crate::validation::note(n).map_err(err)?;
            if let Some(font) = &n.font { font.clone().validate_for_note()?; }
            if !canonical_id(&n.id) || !ids.insert(&n.id) || !crate::model::TINTS.contains(&n.tint.as_str())
                || !n.opacity.is_finite() || !(0.0..=1.0).contains(&n.opacity)
                || n.conflict_of.as_ref().is_some_and(|id| !canonical_id(id)) {
                return Err(tr!("백업의 메모 정보가 올바르지 않습니다.", "Invalid note in backup."; "备份中的便签信息无效。", "バックアップ内のメモ情報が無効です。"; "Note non valide dans la sauvegarde.", "Nota no válida en la copia de seguridad."));
            }
            let mut end = 0;
            let length = n.body.encode_utf16().count();
            for f in &n.formats {
                if f.from < end || f.from >= f.to || f.to > length
                    || f.size.is_some_and(|s| !s.is_finite() || !(6.0..=96.0).contains(&s))
                    || f.highlight.as_ref().is_some_and(|c| !crate::model::hex_color(c)) {
                    return Err("Invalid text formatting in backup.".into());
                }
                end = f.to;
            }
            if let Some(b) = &n.background {
                if !b.opacity.is_finite() || !(0.0..=1.0).contains(&b.opacity) || b.image.len() > 8 * 1024 * 1024
                    || !["data:image/jpeg;base64,", "data:image/png;base64,", "data:image/webp;base64,"].iter().any(|p| b.image.starts_with(p)) {
                    return Err("Invalid background image in backup.".into());
                }
            }
        }
        for (id, w) in &self.local_state.windows {
            if !canonical_id(id) || !w.width.is_finite() || !w.height.is_finite() || !(180.0..=32768.0).contains(&w.width)
                || !(120.0..=32768.0).contains(&w.height) || w.x.is_some_and(|x| !x.is_finite()) || w.y.is_some_and(|y| !y.is_finite()) {
                return Err("Invalid window placement in backup.".into());
            }
        }
        Ok(())
    }
}

fn read_archive(path: &Path) -> Res<Backup> {
    let input = File::open(path).map_err(err)?;
    if input.metadata().map_err(err)?.len() > MAX_ARCHIVE {
        return Err("Backup file exceeds the size limit.".into());
    }
    let mut zip = ZipArchive::new(input).map_err(err)?;
    if zip.len() != 1 { return Err(tr!("올바른 Pinboard ZIP 파일을 선택하세요.", "Choose a valid Pinboard ZIP file."; "请选择有效的 Pinboard ZIP 文件。", "有効な Pinboard ZIP ファイルを選んでください。"; "Choisissez un fichier ZIP Pinboard valide.", "Elige un archivo ZIP de Pinboard válido.")); }
    let file = zip.by_index(0).map_err(err)?;
    if file.name().map_err(err)? != "pinboard.json" || !file.is_file() || file.size() > MAX_JSON {
        return Err(tr!("백업 파일의 내용이 올바르지 않습니다.", "Invalid backup contents."; "备份内容无效。", "バックアップの内容が無効です。"; "Contenu de sauvegarde non valide.", "Contenido de la copia de seguridad no válido."));
    }
    let backup: Backup = crate::validation::json(file, MAX_JSON).map_err(err)?;
    backup.validate()?;
    Ok(backup)
}

fn write_archive<N: Serialize>(dir: &Path, backup: &Backup<N>) -> Res<PathBuf> {
    fs::create_dir_all(dir).map_err(err)?;
    let date = time::OffsetDateTime::now_utc().to_offset(time::UtcOffset::from_hms(9, 0, 0).unwrap());
    let stem = format!("Pinboard_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}-{:03}", date.year(), date.month() as u8, date.day(), date.hour(), date.minute(), date.second(), date.millisecond());
    let mut path = dir.join(format!("{stem}.zip"));
    let mut suffix = 2;
    while path.exists() { path = dir.join(format!("{stem}_{suffix}.zip")); suffix += 1; }
    let temp = dir.join(format!(".{}.partial", uuid::Uuid::new_v4()));
    let result = (|| {
        let file = OpenOptions::new().write(true).create_new(true).open(&temp).map_err(err)?;
        let mut zip = ZipWriter::new(file);
        zip.start_file("pinboard.json", SimpleFileOptions::default().compression_method(CompressionMethod::Deflated).large_file(true)).map_err(err)?;
        serde_json::to_writer(LimitedWriter { inner: &mut zip, remaining: MAX_JSON }, backup).map_err(err)?;
        let file = zip.finish().map_err(err)?;
        file.sync_all().map_err(err)?;
        fs::rename(&temp, &path).map_err(err)?;
        Ok(path)
    })();
    if result.is_err() { let _ = fs::remove_file(temp); }
    result
}

fn snapshot(app: &AppHandle, preferences: Preferences) -> Backup<crate::store::SharedNote> {
    let state = app.state::<AppState>();
    let store = state.store.lock().unwrap();
    let local = state.local.lock().unwrap();
    let mut settings = state.settings.lock().unwrap().clone();
    // Account credentials, tokens and machine-specific runtime files are not portable.
    settings.gdrive_account = None;
    settings.gdrive_client_id.clear();
    let mut notes: Vec<_> = store.shared().cloned().collect();
    notes.sort_by(|a, b| a.id.cmp(&b.id));
    Backup { format: "Pinboard".into(), version: 1, created_at: crate::model::now_ms(), settings,
        local_state: local.clone(), notes, preferences, autostart: app.autolaunch().is_enabled().ok(), mcp_enabled: crate::mcp::backup_enabled(app) }
}

struct PausedSync { app: AppHandle, reset: bool }
impl Drop for PausedSync {
    fn drop(&mut self) { let _ = self.app.state::<AppState>().sync_tx.send(SyncMsg::Resume { reset: self.reset }); }
}
fn pause_sync(app: &AppHandle) -> Res<PausedSync> {
    let pause = PausedSync { app: app.clone(), reset: false };
    let (tx, rx) = mpsc::channel();
    app.state::<AppState>().sync_tx.send(SyncMsg::Pause(tx)).map_err(err)?;
    rx.recv_timeout(Duration::from_secs(45)).map_err(|_| tr!("동기화가 끝난 후 다시 시도하세요.", "Try again after syncing finishes."; "请在同步完成后重试。", "同期が完了してから再試行してください。"; "Réessayez une fois la synchronisation terminée.", "Inténtalo de nuevo cuando termine la sincronización."))?;
    Ok(pause)
}

#[derive(Serialize, Deserialize)]
struct Journal { id: String, originals: Vec<bool>, committed: bool }

fn remove_known(path: &Path) -> std::io::Result<()> {
    if path.is_dir() { fs::remove_dir_all(path) } else if path.exists() { fs::remove_file(path) } else { Ok(()) }
}

/// Also runs before Store::open at startup: an interrupted replacement is rolled back.
pub fn recover(dir: &Path) -> std::io::Result<()> {
    let path = dir.join(JOURNAL);
    if !path.exists() { return Ok(()); }
    let journal: Journal = serde_json::from_reader(File::open(&path)?)?;
    if !canonical_id(&journal.id) || journal.originals.len() != FILES.len() { return Err(std::io::Error::other("Invalid restore journal")); }
    let stage = dir.join(format!(".restore-{}", journal.id));
    if !journal.committed {
        for (i, name) in FILES.iter().enumerate() {
            let old = stage.join("old").join(name);
            let destination = dir.join(name);
            if old.exists() {
                remove_known(&destination)?;
                fs::rename(old, destination)?;
            } else if !journal.originals[i] { remove_known(&destination)?; }
        }
    }
    if stage.exists() { fs::remove_dir_all(stage)?; }
    fs::remove_file(path)
}

fn replace_files(dir: &Path, backup: &Backup) -> Res<()> {
    let mut journal = Journal { id: uuid::Uuid::new_v4().to_string(), originals: FILES.iter().map(|n| dir.join(n).exists()).collect(), committed: false };
    let stage = dir.join(format!(".restore-{}", journal.id));
    fs::create_dir_all(stage.join("new/notes")).map_err(err)?;
    fs::create_dir_all(stage.join("old")).map_err(err)?;
    let result = (|| {
        for note in &backup.notes { write_json(&stage.join("new/notes").join(format!("{}.json", note.id)), note).map_err(err)?; }
        write_json(&stage.join("new/settings.json"), &backup.settings).map_err(err)?;
        write_json(&stage.join("new/local-state.json"), &backup.local_state).map_err(err)?;
        write_json(&stage.join("new/ui-preferences.json"), &backup.preferences).map_err(err)?;
        write_json(&dir.join(JOURNAL), &journal).map_err(err)?;
        for (i, name) in FILES.iter().enumerate() {
            if journal.originals[i] { fs::rename(dir.join(name), stage.join("old").join(name)).map_err(err)?; }
            fs::rename(stage.join("new").join(name), dir.join(name)).map_err(err)?;
        }
        journal.committed = true;
        write_json(&dir.join(JOURNAL), &journal).map_err(err)?;
        Ok(())
    })();
    if result.is_err() {
        if dir.join(JOURNAL).exists() { recover(dir).map_err(err)?; }
        else { let _ = fs::remove_dir_all(&stage); }
    } else if let Err(e) = recover(dir) { log::warn!("restore cleanup deferred until next startup: {e}"); }
    result
}

fn settings_window(window: &WebviewWindow) -> Res<()> {
    if window.label() != windows::SETTINGS_LABEL { return Err("Backup is only available in settings.".into()); }
    Ok(())
}

#[tauri::command]
pub async fn export_archive(app: AppHandle, window: WebviewWindow, preferences: Preferences) -> Res<String> {
    settings_window(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state.transfer_lock.try_lock().map_err(|_| "Backup is already in progress.")?;
        let _frozen = crate::editors::freeze(&app)?;
        let _paused = pause_sync(&app)?;
        let _gate = state.data_gate.write().unwrap();
        let backup = snapshot(&app, preferences);
        backup.validate()?;
        let downloads = app.path().download_dir().map_err(err)?;
        write_archive(&downloads, &backup).map(|p| p.to_string_lossy().into_owned())
    }).await.map_err(err)?
}

#[tauri::command]
pub async fn import_archive(app: AppHandle, window: WebviewWindow, path: String, preferences: Preferences) -> Res<ImportResult> {
    settings_window(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _operation = state.transfer_lock.try_lock().map_err(|_| "Backup is already in progress.")?;
        // Full parsing and validation happen before freezing or touching current data.
        let mut backup = read_archive(Path::new(&path))?;
        let _frozen = crate::editors::freeze(&app)?;
        let mut paused = pause_sync(&app)?;
        let _gate = state.data_gate.write().unwrap();
        let current = snapshot(&app, preferences);
        let saved = write_archive(&state.data_dir.join("import-backups"), &current)?;
        let mut warnings = Vec::new();
        if backup.settings.sync == SyncKind::Folder {
            warnings.push(tr!("이 PC의 Google Drive for Desktop 폴더를 다시 연결하세요. 메모는 현재 PC에 복원되었습니다.", "Reconnect the Google Drive for Desktop folder on this PC. Notes were restored locally."; "请重新连接此电脑上的 Google Drive for Desktop 文件夹。便签已恢复到本地。", "この PC の Google Drive for Desktop フォルダーを再接続してください。メモはローカルに復元されました。"; "Reconnectez le dossier Google Drive for Desktop sur ce PC. Les notes ont été restaurées localement.", "Vuelve a conectar la carpeta de Google Drive for Desktop en este PC. Las notas se restauraron localmente."));
        }
        backup.restrict_import(&current);
        backup.local_state.welcomed = true;
        backup.local_state.autostart_initialized = true;
        crate::shortcuts::apply(&app, &backup.settings)?;
        if std::env::var_os("PINBOARD_DATA_DIR").is_none() {
            if let Some(on) = backup.autostart {
                if let Err(e) = if on { app.autolaunch().enable() } else { app.autolaunch().disable() } {
                    let _ = crate::shortcuts::apply(&app, &current.settings);
                    return Err(err(e));
                }
            }
        }
        state.restoring.store(true, Ordering::SeqCst);
        // Holding all three locks also excludes the background local-state saver.
        let result = {
            let mut store = state.store.lock().unwrap();
            let mut local = state.local.lock().unwrap();
            let mut settings = state.settings.lock().unwrap();
            replace_files(&state.data_dir, &backup).and_then(|()| {
                store.replace_snapshot(backup.notes.clone());
                *local = backup.local_state.clone();
                *settings = backup.settings.clone();
                state.local_dirty.store(false, Ordering::SeqCst);
                Ok(())
            })
        };
        if let Err(e) = result {
            state.restoring.store(false, Ordering::SeqCst);
            let _ = crate::shortcuts::apply(&app, &current.settings);
            if std::env::var_os("PINBOARD_DATA_DIR").is_none() {
                if let Some(on) = current.autostart { let _ = if on { app.autolaunch().enable() } else { app.autolaunch().disable() }; }
            }
            return Err(e);
        }
        // Destroy old editors while writes are excluded; late queued writes are rejected.
        for (label, window) in app.webview_windows() {
            if label.starts_with("note-") || label == crate::calendar::LABEL {
                crate::platform::forget(&window);
                let _ = window.destroy();
            }
        }
        state.focus_on_ready.lock().unwrap().clear();
        *state.settings_note.lock().unwrap() = None;
        crate::i18n::set(backup.settings.language);
        let _ = crate::refresh_language(&app);
        if let Err(e) = crate::mcp::restore_enabled(&app, backup.mcp_enabled) { warnings.push(e); }
        state.restoring.store(false, Ordering::SeqCst);
        drop(_gate);
        windows::restore(&app);
        crate::commands::notes_changed(&app, None, true);
        let _ = app.emit("settings://changed", &backup.settings);
        let _ = app.emit("archive://restored", &backup.preferences);
        paused.reset = true;
        Ok(ImportResult { note_count: backup.notes.iter().filter(|n| !n.is_deleted()).count(), backup_path: saved.to_string_lossy().into_owned(), preferences: backup.preferences, warnings })
    }).await.map_err(err)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use std::io::Write;
    fn fixture() -> Backup {
        let mut n = Note::new("old-pc", "rose", 0.7);
        n.body = "한😀글 test".into();
        n.font = Some(crate::fonts::NoteFont { family: Some("Noto Sans KR".into()), size: 24.0, weight: 500 });
        n.formats = vec![crate::model::TextFormat { from: 1, to: 3, bold: Some(true), italic: Some(true), size: Some(19.0), highlight: Some("#ffe066".into()) }];
        n.background = Some(crate::model::NoteBackground { image: "data:image/png;base64,dGVzdA==".into(), opacity: 0.22 });
        let mut local = LocalState::default();
        local.windows.insert(n.id.clone(), crate::store::WindowState { visible: false, pin: crate::model::PinMode::Desktop, ..Default::default() });
        Backup { format: "Pinboard".into(), version: 1, created_at: 1, settings: Settings::default(), local_state: local, notes: vec![n], preferences: Preferences { mcp_providers: vec!["codex".into()] }, autostart: Some(false), mcp_enabled: true }
    }

    #[test]
    fn archive_cannot_enable_mcp_startup_or_a_network_sync_destination() {
        let mut current = fixture();
        current.mcp_enabled = false;
        current.autostart = Some(false);
        current.settings.gdrive_account = Some("current@example.test".into());
        let mut imported = fixture();
        imported.mcp_enabled = true;
        imported.autostart = Some(true);
        imported.settings.sync = SyncKind::Folder;
        imported.settings.folder_root = Some("\\\\attacker.invalid\\share".into());
        imported.settings.gdrive_account = Some("attacker@example.test".into());
        imported.restrict_import(&current);
        assert_eq!(imported.settings.sync, SyncKind::Off);
        assert_eq!(imported.settings.folder_root, None);
        assert_eq!(imported.settings.gdrive_account, current.settings.gdrive_account);
        assert!(!imported.mcp_enabled);
        assert_eq!(imported.autostart, Some(false));
    }

    #[test]
    fn rejects_compressed_archive_bombs_and_bounds_exports() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oversized.zip");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        zip.start_file("pinboard.json", SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)).unwrap();
        let block = vec![b' '; 1024 * 1024];
        for _ in 0..=64 { zip.write_all(&block).unwrap(); }
        zip.finish().unwrap();
        assert!(read_archive(&path).is_err());
        let mut writer = LimitedWriter { inner: Vec::new(), remaining: 4 };
        writer.write_all(b"1234").unwrap();
        assert!(writer.write_all(b"5").is_err());
        assert_eq!(writer.inner, b"1234");
    }
    #[test]
    fn zip_round_trip_preserves_full_notes_and_settings() {
        let dir = tempfile::tempdir().unwrap();
        let original = fixture();
        let first = write_archive(dir.path(), &original).unwrap();
        let second = write_archive(dir.path(), &original).unwrap();
        assert_ne!(first, second);
        let restored = read_archive(&first).unwrap();
        assert_eq!(serde_json::to_value(&original).unwrap(), serde_json::to_value(&restored).unwrap());
        assert!(first.file_name().unwrap().to_str().unwrap().starts_with("Pinboard_"));
    }
    #[test]
    fn invalid_archives_and_note_paths_are_rejected_without_touching_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut invalid = fixture();
        invalid.notes[0].id = "../outside".into();
        assert!(read_archive(&write_archive(dir.path(), &invalid).unwrap()).is_err());
        let mut invalid = fixture();
        invalid.version = 99;
        assert!(invalid.validate().is_err());
        invalid = fixture(); invalid.notes.push(invalid.notes[0].clone());
        assert!(invalid.validate().is_err());
        invalid = fixture(); invalid.notes[0].formats[0].to = usize::MAX;
        assert!(invalid.validate().is_err());
        invalid = fixture(); invalid.notes[0].font.as_mut().unwrap().size = 200.0;
        assert!(invalid.validate().is_err());
        let path = dir.path().join("traversal.zip");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        zip.start_file("../pinboard.json", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"{}").unwrap(); zip.finish().unwrap();
        assert!(read_archive(&path).is_err());
    }
    #[test]
    fn replace_removes_old_notes_and_recovers_on_next_open() {
        let dir = tempfile::tempdir().unwrap();
        let mut old = Store::open(dir.path().join("notes")).unwrap();
        let old_note = Note::new("pc", "frost", 0.5);
        old.put(old_note.clone()).unwrap();
        let backup = fixture();
        replace_files(dir.path(), &backup).unwrap();
        let restored = Store::open(dir.path().join("notes")).unwrap();
        assert_eq!(restored.all().count(), 1);
        assert_eq!(restored.get(&backup.notes[0].id), Some(&backup.notes[0]));
        assert!(restored.get(&old_note.id).is_none());
        assert!(!dir.path().join(JOURNAL).exists());
        let j = Journal { id: uuid::Uuid::new_v4().to_string(), originals: vec![true; FILES.len()], committed: false };
        let stage = dir.path().join(format!(".restore-{}", j.id));
        fs::create_dir_all(stage.join("old")).unwrap();
        fs::rename(dir.path().join("notes"), stage.join("old/notes")).unwrap();
        fs::create_dir_all(dir.path().join("notes")).unwrap();
        write_json(&dir.path().join(JOURNAL), &j).unwrap();
        recover(dir.path()).unwrap();
        assert_eq!(Store::open(dir.path().join("notes")).unwrap().get(&backup.notes[0].id), Some(&backup.notes[0]));
    }
}
