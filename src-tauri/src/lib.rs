#[macro_use]
mod i18n;
mod editors;
mod mcp;
mod commands;
mod calendar;
mod fonts;
mod model;
mod platform;
mod settings;
mod shortcuts;
mod store;
mod sync;
mod welcome;
mod windows;
#[cfg(any(windows, test))]
mod snapping;
mod transfer;
mod validation;
#[cfg(test)]
mod test_support;

use model::Note;
use settings::Settings;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, RwLock};
use std::time::Duration;
use store::{read_json, write_json, LocalState, Store};
use sync::worker::{SyncMsg, SyncStatus};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent};

pub struct AppState {
    pub data_gate: RwLock<()>,
    pub transfer_lock: Mutex<()>,
    pub restoring: AtomicBool,
    pub data_dir: PathBuf,
    pub device_id: String,
    pub store: Mutex<Store>,
    pub local: Mutex<LocalState>,
    pub local_dirty: AtomicBool,
    pub settings: Mutex<Settings>,
    pub settings_note: Mutex<Option<String>>,
    pub sync_status: Mutex<SyncStatus>,
    pub sync_tx: Sender<SyncMsg>,
    /// Window labels that should take focus when their page reports ready.
    pub focus_on_ready: Mutex<HashSet<String>>,
    pub active_shortcut: Mutex<Option<tauri_plugin_global_shortcut::Shortcut>>,
    pub shortcut_recording_until: AtomicI64,
}

pub fn save_settings_file(state: &AppState) -> std::io::Result<()> {
    write_json(&state.data_dir.join("settings.json"), &*state.settings.lock().unwrap())
}

fn save_local(state: &AppState) {
    if state.local_dirty.swap(false, Ordering::SeqCst) {
        let local = state.local.lock().unwrap();
        if let Err(e) = write_json(&state.data_dir.join("local-state.json"), &*local) {
            state.local_dirty.store(true, Ordering::SeqCst);
            log::error!("cannot save window state: {e}");
        }
    }
}

pub fn new_note(app: &AppHandle, near: Option<&str>) -> tauri::Result<Note> {
    let state = app.state::<AppState>();
    let _gate = state.data_gate.try_read().map_err(|_| std::io::Error::other("Restore in progress."))?;
    let note = {
        let mut store = state.store.lock().unwrap();
        let mut local = state.local.lock().unwrap();
        let (note, ws) = store.draft_from(&state.device_id, &local, near);
        store.put(note.clone())?;
        local.windows.insert(note.id.clone(), ws);
        note
    };
    state.local_dirty.store(true, Ordering::SeqCst);
    if let Some(pos) = near.and_then(|n| windows::position_near(app, n)) {
        let mut local = state.local.lock().unwrap();
        let ws = local.windows.entry(note.id.clone()).or_default();
        ws.x = Some(pos.x);
        ws.y = Some(pos.y);
    }
    state.focus_on_ready.lock().unwrap().insert(windows::note_label(&note.id));
    windows::open_note(app, &note, true)?;
    let _ = state.sync_tx.send(SyncMsg::LocalChanged);
    commands::notes_changed(app, Some(&note.id), true);
    Ok(note)
}

fn data_dir(app: &AppHandle) -> PathBuf {
    match std::env::var_os("PINBOARD_DATA_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => app.path().app_data_dir().expect("app data dir"),
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn failed_window_state_save_remains_dirty_and_retries_without_another_edit() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_support::app_state(dir.path());
        let path = dir.path().join("local-state.json");
        std::fs::create_dir(&path).unwrap();
        state.local.lock().unwrap().welcomed = true;
        state.local_dirty.store(true, Ordering::SeqCst);
        save_local(&state);
        assert!(state.local_dirty.load(Ordering::SeqCst));
        assert!(path.is_dir());
        std::fs::remove_dir(&path).unwrap();
        save_local(&state);
        assert!(!state.local_dirty.load(Ordering::SeqCst));
        assert!(read_json::<LocalState>(&path).unwrap().welcomed);
    }

    #[test]
    fn clean_window_state_does_not_rewrite_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_support::app_state(dir.path());
        let path = dir.path().join("local-state.json");
        std::fs::write(&path, b"leave unchanged until dirty").unwrap();
        save_local(&state);
        assert_eq!(std::fs::read(&path).unwrap(), b"leave unchanged until dirty");
    }
}

fn device_id(dir: &std::path::Path) -> String {
    let path = dir.join("device_id");
    if let Ok(id) = std::fs::read_to_string(&path) {
        if !id.trim().is_empty() {
            return id.trim().to_string();
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let _ = store::write_atomic(&path, id.as_bytes());
    id
}

fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "new", tr!("새 메모", "New note"; "新建便签", "新しいメモ"; "Nouvelle note", "Nueva nota"), true, None::<&str>)?,
            &MenuItem::with_id(app, "show", tr!("모든 메모 보이기", "Show all notes"; "显示所有便签", "すべてのメモを表示"; "Afficher toutes les notes", "Mostrar todas las notas"), true, None::<&str>)?,
            &MenuItem::with_id(app, "hide", tr!("모든 메모 숨기기", "Hide all notes"; "隐藏所有便签", "すべてのメモを非表示"; "Masquer toutes les notes", "Ocultar todas las notas"), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "sync", tr!("지금 동기화", "Sync now"; "立即同步", "今すぐ同期"; "Synchroniser maintenant", "Sincronizar ahora"), true, None::<&str>)?,
            &MenuItem::with_id(app, "settings", tr!("설정", "Settings"; "设置", "設定"; "Paramètres", "Configuración"), true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", tr!("Pinboard 종료", "Quit Pinboard"; "退出 Pinboard", "Pinboard を終了"; "Quitter Pinboard", "Salir de Pinboard"), true, None::<&str>)?,
        ],
    )
}

pub fn refresh_language(app: &AppHandle) -> tauri::Result<()> {
    localize_welcome(app)?;
    if let Some(window) = app.get_webview_window(calendar::LABEL) {
        window.set_title(&tr!("메모 달력", "Note calendar"; "便签日历", "メモカレンダー"; "Calendrier des notes", "Calendario de notas"))?;
    }
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(tray_menu(app)?))?;
    }
    if let Some(window) = app.get_webview_window(windows::SETTINGS_LABEL) {
        window.set_title(&tr!("Pinboard 설정", "Pinboard Settings"; "Pinboard 设置", "Pinboard 設定"; "Paramètres de Pinboard", "Configuración de Pinboard"))?;
    }
    Ok(())
}

fn localize_welcome(app: &AppHandle) -> std::io::Result<()> {
    let state = app.state::<AppState>();
    let language = state.settings.lock().unwrap().language;
    let mut store = state.store.lock().unwrap();
    if let Some(mut note) = store.get(welcome::ID).cloned() {
        if welcome::localize(&mut note, language) {
            store.put(note.clone())?;
            let _ = app.emit_to(windows::note_label(&note.id), "note://updated", &note);
        }
    }
    Ok(())
}

pub(crate) enum WindowAction {
    NewNote,
    Settings,
    ShowAll,
    HideAll,
    Quit,
}

/// WebView2 creation must not block native menu, hotkey or single-instance callbacks.
/// In particular, a second launch can arrive while the first WebView is still initializing.
pub(crate) fn dispatch_window_action(app: &AppHandle, action: WindowAction) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = match action {
            WindowAction::NewNote => new_note(&app, None).map(|_| ()).map_err(|e| e.to_string()),
            WindowAction::Settings => windows::open_settings(&app).map_err(|e| e.to_string()),
            WindowAction::ShowAll => { windows::show_all(&app); Ok(()) }
            WindowAction::HideAll => finish_editor_action(&app, false),
            WindowAction::Quit => finish_editor_action(&app, true),
        };
        if let Err(error) = result { log::error!("native window action failed: {error}"); }
    });
}

fn finish_editor_action(app: &AppHandle, quit: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let _operation = state.transfer_lock.try_lock().map_err(|_| "Backup or another editor action is already in progress.")?;
    let frozen = match editors::freeze(app) {
        Ok(frozen) => frozen,
        Err(error) => {
            // A previously hidden editor can also fail. Bring its retry UI back.
            windows::show_all(app);
            return Err(error);
        }
    };
    if quit {
        frozen.until_exit();
        app.exit(0);
    } else {
        windows::hide_all(app);
    }
    Ok(())
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = tray_menu(app)?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon"))
        .tooltip("Pinboard")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "linux"))
        .on_menu_event(|app, event| match event.id().as_ref() {
            "new" => dispatch_window_action(app, WindowAction::NewNote),
            "show" => dispatch_window_action(app, WindowAction::ShowAll),
            "hide" => dispatch_window_action(app, WindowAction::HideAll),
            "sync" => {
                let _ = app.state::<AppState>().sync_tx.send(SyncMsg::Now);
            }
            "settings" => dispatch_window_action(app, WindowAction::Settings),
            "quit" => dispatch_window_action(app, WindowAction::Quit),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = event {
                dispatch_window_action(tray.app_handle(), WindowAction::NewNote);
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if mcp::run_if_requested() {
        return;
    }
    let isolated = std::env::var_os("PINBOARD_DATA_DIR").is_some();
    let mut builder = tauri::Builder::default();
    if !isolated {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|arg| arg == "--settings") {
                  dispatch_window_action(app, WindowAction::Settings);
            } else if !args.iter().any(|arg| arg == "--autostart") {
                  dispatch_window_action(app, WindowAction::NewNote);
            }
        }));
    }
    if shortcuts::available() {
        builder = builder.plugin(shortcuts::plugin());
    }
    let app = builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            calendar::open_calendar,
            calendar::calendar_notes,
            calendar::calendar_day_notes,
            calendar::show_calendar_note,
            calendar::calendar_pin,
            calendar::set_calendar_pin,
            commands::get_note,
            commands::window_ready,
            commands::drag_note,
            commands::create_note,
            commands::update_note,
            commands::save_note_content,
            commands::delete_note,
            commands::hide_note,
            commands::set_pin_mode,
            commands::open_settings,
            commands::design_context,
            commands::design_note_summaries,
            commands::get_note_design,
            commands::get_settings,
            commands::list_fonts,
            commands::save_settings,
            commands::set_shortcut_recording,
            commands::get_sync_status,
            commands::sync_now,
            commands::has_gdrive_secret,
            commands::gdrive_connect,
            commands::gdrive_disconnect,
            commands::suggest_folder,
            commands::app_info,
            commands::open_data_dir,
            transfer::export_archive,
            transfer::import_archive,
            editors::editor_ready,
            transfer::get_transfer_preferences,
            transfer::save_transfer_preferences,
            commands::fit_settings,
            mcp::get_mcp_settings,
            mcp::provision_mcp,
            mcp::set_mcp_enabled,
            mcp::mcp_client_status,
            mcp::connect_mcp_clients,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let dir = data_dir(&handle);
            std::fs::create_dir_all(&dir)?;
            transfer::recover(&dir)?;
            log::info!("data dir: {}", dir.display());
            let notices = include_bytes!("../../THIRD-PARTY-NOTICES.txt");
            if let Err(e) = store::write_atomic(&dir.join("THIRD-PARTY-NOTICES.txt"), notices) {
                log::warn!("cannot write third-party notices: {e}");
            }

            let (tx, rx) = mpsc::channel();
            let local: LocalState = read_json(&dir.join("local-state.json")).unwrap_or_default();
            let settings: Settings = read_json(&dir.join("settings.json")).unwrap_or_default();
            i18n::set(settings.language);
            let state = AppState {
                data_gate: RwLock::new(()),
                transfer_lock: Mutex::new(()),
                restoring: AtomicBool::new(false),
                device_id: device_id(&dir),
                store: Mutex::new(Store::open(dir.join("notes"))?),
                local: Mutex::new(local),
                local_dirty: AtomicBool::new(false),
                settings: Mutex::new(settings),
                settings_note: Mutex::new(None),
                sync_status: Mutex::new(SyncStatus::default()),
                sync_tx: tx.clone(),
                focus_on_ready: Mutex::new(HashSet::new()),
                active_shortcut: Mutex::new(None),
                shortcut_recording_until: AtomicI64::new(0),
                data_dir: dir,
            };
            app.manage(state);

            platform::init(&handle);
            setup_tray(&handle)?;
            // Shortcut registration dispatches to the UI thread, so run it off-thread.
            let shortcut_app = handle.clone();
            let startup_settings = handle.state::<AppState>().settings.lock().unwrap().clone();
            std::thread::spawn(move || {
                let state = shortcut_app.state::<AppState>();
                if let Err(e) = shortcuts::apply(&shortcut_app, &startup_settings) {
                    log::warn!("global shortcut unavailable: {e}");
                    let settings = {
                        let mut settings = state.settings.lock().unwrap();
                        settings.shortcuts_enabled = false;
                        settings.clone()
                    };
                    let _ = shortcut_app.emit("settings://changed", &settings);
                }
            });

            let state = handle.state::<AppState>();
            if !isolated {
                let mut local = state.local.lock().unwrap();
                if local.initialize_autostart() {
                    state.local_dirty.store(true, Ordering::SeqCst);
                }
            }
            let first_run = !state.local.lock().unwrap().welcomed;
            if first_run {
                state.local.lock().unwrap().welcomed = true;
                state.local_dirty.store(true, Ordering::SeqCst);
                if state.store.lock().unwrap().live().next().is_none() {
                    // Same id and timestamps on every device, so welcome notes from two PCs
                    // merge into one when sync is turned on instead of conflicting.
                    let mut note = Note::new(&state.device_id, "frost", 0.55);
                    note.id = welcome::ID.into();
                    note.body = welcome::body(state.settings.lock().unwrap().language).into();
                    note.created_at = 0;
                    note.updated_at = 0;
                    state.store.lock().unwrap().put(note)?;
                }
            }
            localize_welcome(&handle)?;
            windows::restore(&handle);
            if std::env::args().any(|arg| arg == "--settings") {
                dispatch_window_action(&handle, WindowAction::Settings);
            }

            sync::worker::spawn(handle.clone(), rx, tx);
            mcp::start(&handle);

            let saver = handle.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(1));
                save_local(&saver.state::<AppState>());
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Pinboard");

    app.run(|app, event| match event {
        // Hiding or deleting the last note must not quit; the tray keeps running.
        RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
        RunEvent::Exit => {
            let state = app.state::<AppState>();
            save_local(&state);
            let (ack, done) = mpsc::channel();
            if state.sync_tx.send(SyncMsg::Shutdown(ack)).is_ok() {
                let _ = done.recv_timeout(Duration::from_secs(5));
            }
        }
        _ => {}
    });
}
