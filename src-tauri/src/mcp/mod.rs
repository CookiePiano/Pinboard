mod bridge;
mod clients;
mod protocol;

use crate::model::{now_ms, Note, PinMode};
use crate::store::{write_json, WindowState};
use crate::sync::worker::SyncMsg;
use crate::{commands, windows, AppState};
use bridge::{CreateNote, ListNotes, NoteId, Operation, Reply, Request, Runtime, UpdateNote};
use serde_json::{json, Value};
use clients::{Client, ClientStatus};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, State, WebviewWindow};

// Version 2 adds reading and changing existing notes to the original create-only grant.
// Increment this whenever the permission scope grows; old consent must not expand silently.
const GRANT_VERSION: u32 = 2;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Config {
    provisioned: bool,
    enabled: bool,
    grant_version: u32,
}

struct Control {
    provisioned: bool,
    enabled: bool,
    session: String,
    available: bool,
}

pub struct McpState {
    root: PathBuf,
    control: Mutex<Control>,
    worker_started: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    provisioned: bool,
    enabled: bool,
    available: bool,
    executable: String,
    arguments: Vec<String>,
}

fn view(state: &McpState) -> Result<McpSettings, String> {
    let control = state.control.lock().unwrap();
    view_unlocked(state, &control)
}

fn view_unlocked(state: &McpState, control: &Control) -> Result<McpSettings, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(McpSettings {
        provisioned: control.provisioned,
        enabled: control.enabled,
        available: control.available,
        executable: executable.to_string_lossy().into_owned(),
        arguments: vec!["--mcp".into(), "--mcp-data-dir".into(), state.root.parent().unwrap().to_string_lossy().into_owned()],
    })
}

fn settings_window(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != windows::SETTINGS_LABEL {
        return Err("MCP settings are only available in the settings window.".into());
    }
    Ok(())
}

pub fn backup_enabled(app: &AppHandle) -> bool {
    app.state::<McpState>().control.lock().unwrap().enabled
}

pub fn restore_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<McpState>();
    let mut control = state.control.lock().unwrap();
    // A backup is never consent to provision MCP on another PC.
    if !control.provisioned { return Ok(()); }
    write_json(&state.root.join("settings.json"), &Config { provisioned: true, enabled, grant_version: GRANT_VERSION }).map_err(|e| e.to_string())?;
    control.enabled = enabled;
    control.session = uuid::Uuid::new_v4().to_string();
    publish(&state, &control).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_mcp_settings(window: WebviewWindow, state: State<McpState>) -> Result<McpSettings, String> {
    settings_window(&window)?;
    view(&state)
}

/// Only an explicit settings action provisions a PC that has never opted in.
#[tauri::command]
pub fn provision_mcp(app: AppHandle, window: WebviewWindow, state: State<McpState>) -> Result<McpSettings, String> {
    settings_window(&window)?;
    provision(&app, &state)?;
    view(&state)
}

fn provision(app: &AppHandle, state: &McpState) -> Result<(), String> {
    {
        let mut control = state.control.lock().unwrap();
        if control.provisioned && control.available { return Ok(()); }
        bridge::prepare(&state.root).map_err(|e| e.to_string())?;
        write_json(&state.root.join("settings.json"), &Config { provisioned: true, enabled: false, grant_version: GRANT_VERSION }).map_err(|e| e.to_string())?;
        control.provisioned = true;
        control.enabled = false;
        control.available = true;
        control.session = uuid::Uuid::new_v4().to_string();
        if let Err(e) = publish(state, &control) { control.available = false; return Err(e.to_string()); }
    }
    start_worker(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_mcp_enabled(window: WebviewWindow, state: State<McpState>, enabled: bool) -> Result<McpSettings, String> {
    settings_window(&window)?;
    set_enabled(&state, enabled)?;
    view(&state)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientError {
    client: Client,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResult {
    settings: McpSettings,
    clients: Vec<ClientStatus>,
    errors: Vec<ClientError>,
}

fn client_statuses(view: &McpSettings) -> Vec<ClientStatus> {
    [Client::Codex, Client::Claude, Client::ClaudeCli, Client::Antigravity, Client::Gemini].into_iter()
        .map(|client| clients::status(client, &view.executable, &view.arguments)).collect()
}

#[tauri::command]
pub fn mcp_client_status(window: WebviewWindow, state: State<McpState>) -> Result<Vec<ClientStatus>, String> {
    settings_window(&window)?;
    Ok(client_statuses(&view(&state)?))
}

/// The one-click path: set up MCP, allow connections, and register Pinboard in each chosen app.
#[tauri::command]
pub fn connect_mcp_clients(app: AppHandle, window: WebviewWindow, state: State<McpState>, targets: Vec<Client>) -> Result<ConnectResult, String> {
    settings_window(&window)?;
    if targets.is_empty() { return Err("Select an AI app first.".into()); }
    provision(&app, &state)?;
    let settings = view(&state)?;
    let count = targets.len();
    let errors: Vec<_> = targets.into_iter()
        .filter_map(|client| clients::register(client, &settings.executable, &settings.arguments).err().map(|message| ClientError { client, message }))
        .collect();
    if errors.len() < count { set_enabled(&state, true)?; }
    let settings = view(&state)?;
    Ok(ConnectResult { clients: client_statuses(&settings), settings, errors })
}

fn set_enabled(state: &McpState, enabled: bool) -> Result<(), String> {
    {
        let mut control = state.control.lock().unwrap();
        if !control.provisioned {
            return Err(tr!("먼저 MCP 구축을 눌러 이 PC의 연결을 준비하세요.", "Set up MCP on this PC first."; "请先在此电脑上设置 MCP。", "先にこの PC で MCP をセットアップしてください。"; "Configurez d'abord MCP sur ce PC.", "Configura MCP en este PC primero."));
        }
        if !control.available {
            return Err(tr!("MCP 연결을 준비하지 못했습니다. Pinboard를 다시 실행하세요.", "MCP is unavailable. Restart Pinboard."; "MCP 不可用。请重新启动 Pinboard。", "MCP を利用できません。Pinboard を再起動してください。"; "MCP n'est pas disponible. Redémarrez Pinboard.", "MCP no está disponible. Reinicia Pinboard."));
        }
        if control.enabled != enabled {
            write_json(&state.root.join("settings.json"), &Config { provisioned: true, enabled, grant_version: GRANT_VERSION }).map_err(|e| e.to_string())?;
            control.enabled = enabled;
            // Any queued requests belong to the old grant, even if re-enabled immediately.
            control.session = uuid::Uuid::new_v4().to_string();
        }
        publish(state, &control).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn publish(state: &McpState, control: &Control) -> std::io::Result<()> {
    write_json(&state.root.join("runtime.json"), &Runtime {
        session: control.session.clone(),
        enabled: control.enabled && control.available,
        heartbeat_at: now_ms(),
    })
}

/// The bridge owns its settings and inbox; existing sync and note settings are untouched.
pub fn start(app: &AppHandle) {
    let root = app.state::<AppState>().data_dir.join("mcp");
    let state = load_state(root);
    let available = state.control.lock().unwrap().available;
    app.manage(state);
    if !available { return }
    if let Err(e) = start_worker(app) { log::warn!("cannot start MCP worker: {e}"); }
}

fn load_state(root: PathBuf) -> McpState {
    let config: Config = bridge::read_bounded(&root.join("settings.json")).unwrap_or_default();
    // Legacy settings were written automatically and are not evidence of explicit setup.
    let available = config.provisioned && match bridge::prepare(&root) {
        Ok(()) => true,
        Err(e) => { log::warn!("cannot prepare MCP bridge: {e}"); false }
    };
    McpState {
        root,
        control: Mutex::new(Control {
            provisioned: config.provisioned,
            enabled: available && config.enabled && config.grant_version == GRANT_VERSION,
            session: uuid::Uuid::new_v4().to_string(),
            available,
        }),
        worker_started: AtomicBool::new(false),
    }
}

fn start_worker(app: &AppHandle) -> std::io::Result<()> {
    if app.state::<McpState>().worker_started.swap(true, Ordering::SeqCst) { return Ok(()); }
    let worker_app = app.clone();
    let result = spawn_worker(worker_app);
    if result.is_err() {
        let state = app.state::<McpState>();
        state.worker_started.store(false, Ordering::SeqCst);
        state.control.lock().unwrap().available = false;
    }
    result.map(|_| ())
}

fn spawn_worker(app: AppHandle) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new().name("pinboard-mcp".into()).spawn(move || {
        let state = app.state::<McpState>();
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        // A failed watcher must leave a timed polling channel, not a disconnected busy loop.
        let _keepalive = tx.clone();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if event.is_ok_and(|e| matches!(e.kind, notify::EventKind::Create(_) | notify::EventKind::Modify(_) | notify::EventKind::Remove(_))) {
                let _ = tx.try_send(());
            }
        }).ok();
        if let Some(watcher) = watcher.as_mut() {
            use notify::Watcher;
            if let Err(error) = watcher.watch(&state.root.join("inbox"), notify::RecursiveMode::NonRecursive) {
                log::warn!("MCP inbox watcher unavailable; using polling: {error}");
            }
        }
        let mut heartbeat = Instant::now() - Duration::from_secs(2);
        let mut cleanup = heartbeat;
        loop {
            if heartbeat.elapsed() >= Duration::from_secs(1) {
                let mut control = state.control.lock().unwrap();
                let available = publish(&state, &control).is_ok();
                if !available && control.available { log::warn!("cannot publish MCP availability"); }
                control.available = available;
                heartbeat = Instant::now();
            }
            let mut processed = 0;
            if let Ok(entries) = std::fs::read_dir(state.root.join("inbox")) {
                for entry in entries.flatten().take(32) {
                    let Some((id, working)) = bridge::claim(&entry.path()) else { continue };
                    processed += 1;
                    let request = bridge::read_bounded::<Request>(&working);
                    let mut effects = Vec::new();
                    let (session, result) = {
                        let app_state = app.state::<AppState>();
                        let _gate = app_state.data_gate.read().unwrap();
                        // Recheck the grant immediately before writing. Disable waits only
                        // for an already-started save, never for a window to finish opening.
                        let control = state.control.lock().unwrap();
                        let session = request.as_ref().map(|r| r.session.clone()).unwrap_or_default();
                        let result = match request {
                            Ok(request) => request.validate(&control.session, control.enabled && control.available)
                                .and_then(|()| request.operation())
                                .and_then(|op| execute(&app, &id, op))
                                .map(|(value, fx)| { effects = fx; value }),
                            Err(_) => Err("Invalid or oversized note request.".to_string()),
                        };
                        (session, result)
                    };
                    let reply = match result {
                        // `noteId` keeps MCP processes started from an older executable working.
                        Ok(value) => Reply {
                            session,
                            note_id: effects.iter().any(|e| matches!(e, Effect::Created(_))).then(|| id.clone()),
                            error: None,
                            result: Some(value),
                        },
                        Err(error) => Reply { session, note_id: None, error: Some(error), result: None },
                    };
                    let _ = write_json(&state.root.join("replies").join(format!("{id}.json")), &reply);
                    let _ = std::fs::remove_file(working);
                    for effect in effects { apply_effect(&app, effect); }
                }
            }
            if cleanup.elapsed() >= Duration::from_secs(30) {
                bridge::cleanup(&state.root);
                cleanup = Instant::now();
            }
            // Keep the existing 1s heartbeat (5s lease); also recover missed watcher hints.
            if processed < 32 { let _ = rx.recv_timeout(Duration::from_secs(1).saturating_sub(heartbeat.elapsed())); }
        }
    })
}

fn persist(app: &AppHandle, id: &str, request: CreateNote) -> Result<Note, String> {
    let state = app.state::<AppState>();
    let note = {
        let mut store = state.store.lock().unwrap();
        // The UUID is generated by our bridge, never accepted from tool arguments.
        // Retries of a claimed inbox request cannot overwrite an existing note.
        if store.get(id).is_some() { return Err("This note request was already processed.".into()) }
        let mut local = state.local.lock().unwrap();
        let (mut note, mut window) = store.draft_like_last_written(&state.device_id, &local);
        note.id = id.into();
        note.body = request.body;
        if let Some(tint) = request.tint { note.tint = tint; }
        if let Some(pin) = request.pin { window.pin = pin; }
        store.put(note.clone()).map_err(|e| e.to_string())?;
        local.windows.insert(note.id.clone(), window);
        note
    };
    state.local_dirty.store(true, Ordering::SeqCst);
    let _ = state.sync_tx.send(SyncMsg::LocalChanged);
    Ok(note)
}

/// Window work that runs after the data gate, store and grant locks are released.
enum Effect {
    Created(Note),
    Show(Note),
    Hide(String),
    Pin(String, PinMode),
    Patched(Note, bool),
    Deleted(String),
}

fn apply_effect(app: &AppHandle, effect: Effect) {
    match effect {
        Effect::Created(note) | Effect::Show(note) => {
            commands::notes_changed(app, Some(&note.id), true);
            if let Err(e) = windows::open_note(app, &note, false) {
                log::warn!("MCP could not open a note window: {e}");
            }
        }
        Effect::Hide(id) => windows::hide_note(app, &id),
        Effect::Pin(id, mode) => windows::set_pin(app, &id, mode),
        Effect::Patched(note, opacity_changed) => commands::after_patch(app, &note, opacity_changed),
        Effect::Deleted(id) => commands::after_delete(app, &id),
    }
}

const NOT_FOUND: &str = "Note not found. Use list_notes to get current note ids.";
const READ_LIMIT: usize = 200_000;

fn kst(ms: i64) -> Value {
    if ms <= 0 { return Value::Null; }
    time::OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000).ok()
        .map(|t| t.to_offset(time::UtcOffset::from_hms(9, 0, 0).unwrap()))
        .and_then(|t| t.format(&time::format_description::well_known::Rfc3339).ok())
        .map_or(Value::Null, Value::String)
}

/// A note as the AI sees it. Pin and visibility are this PC's window state, like in the app.
fn describe(note: &Note, window: Option<&WindowState>, full: bool) -> Value {
    let (title, rest) = note.body.split_once('\n').unwrap_or((&note.body, ""));
    let mut value = json!({
        "id": note.id,
        "title": title.trim().chars().take(200).collect::<String>(),
        "tint": note.tint,
        "opacity": note.opacity,
        "pin": window.map(|w| w.pin).unwrap_or_default(),
        "visible": window.is_none_or(|w| w.visible),
        "createdAt": kst(note.created_at),
        "updatedAt": kst(note.updated_at),
    });
    if full {
        let truncated = note.body.chars().count() > READ_LIMIT;
        value["body"] = json!(if truncated { note.body.chars().take(READ_LIMIT).collect::<String>() } else { note.body.clone() });
        value["truncated"] = json!(truncated);
        value["font"] = json!(note.font);
        value["hasBackground"] = json!(note.background.is_some());
    } else {
        let preview: String = rest.trim().chars().take(200).collect();
        value["preview"] = json!(preview);
    }
    value
}

fn list(state: &AppState, request: &ListNotes) -> Value {
    let store = state.store.lock().unwrap();
    let local = state.local.lock().unwrap();
    let query = request.query.as_deref().map(str::trim).filter(|q| !q.is_empty()).map(str::to_lowercase);
    let include_hidden = request.include_hidden.unwrap_or(true);
    let mut notes: Vec<&Note> = store.live()
        .filter(|n| query.as_ref().is_none_or(|q| n.body.to_lowercase().contains(q.as_str())))
        .filter(|n| include_hidden || local.windows.get(&n.id).is_none_or(|w| w.visible))
        .collect();
    notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(b.created_at.cmp(&a.created_at)).then(a.id.cmp(&b.id)));
    let limit = request.limit.unwrap_or(50) as usize;
    json!({
        "total": notes.len(),
        "notes": notes.iter().take(limit).map(|n| describe(n, local.windows.get(&n.id), false)).collect::<Vec<_>>(),
    })
}

fn live_note(state: &AppState, id: &str) -> Result<Note, String> {
    state.store.lock().unwrap().get(id).filter(|n| !n.is_deleted()).cloned().ok_or_else(|| NOT_FOUND.to_string())
}

fn update(state: &AppState, request: UpdateNote) -> Result<(Value, Vec<Effect>), String> {
    let current = live_note(state, &request.id)?;
    let mut patch = commands::NotePatch { tint: request.tint, opacity: request.opacity, ..Default::default() };
    if let Some(body) = request.body { patch.body = Some(body); }
    if let Some(append) = request.append {
        let mut body = current.body.clone();
        if !body.is_empty() && !body.ends_with('\n') { body.push('\n'); }
        body.push_str(&append);
        // Appending never moves existing text, so its formatting runs stay valid.
        patch.body = Some(body);
        patch.formats = Some(current.formats.clone());
    }
    let opacity_changed = patch.opacity.is_some();
    let mut effects = Vec::new();
    let note = if patch.body.is_some() || patch.tint.is_some() || patch.opacity.is_some() {
        let (note, changed) = commands::patch_note(state, &request.id, patch)?;
        if changed { effects.push(Effect::Patched(note.clone(), opacity_changed)); }
        note
    } else { current };
    let window = {
        let mut local = state.local.lock().unwrap();
        // A note synced from another PC may not have window state here yet.
        let window = local.windows.entry(note.id.clone()).or_default();
        if let Some(pin) = request.pin { window.pin = pin; }
        if let Some(visible) = request.visible { window.visible = visible; }
        window.clone()
    };
    state.local_dirty.store(true, Ordering::SeqCst);
    if let Some(pin) = request.pin { effects.push(Effect::Pin(note.id.clone(), pin)); }
    match request.visible {
        Some(true) => effects.push(Effect::Show(note.clone())),
        Some(false) => effects.push(Effect::Hide(note.id.clone())),
        None => {}
    }
    Ok((describe(&note, Some(&window), false), effects))
}

/// Runs one tool call against the store. The caller holds the data gate and the grant.
fn execute(app: &AppHandle, request_id: &str, op: Operation) -> Result<(Value, Vec<Effect>), String> {
    let state = app.state::<AppState>();
    match op {
        Operation::ListNotes(request) => Ok((list(&state, &request), Vec::new())),
        Operation::ReadNote(NoteId { id }) => {
            let note = live_note(&state, &id)?;
            let local = state.local.lock().unwrap();
            Ok((describe(&note, local.windows.get(&id), true), Vec::new()))
        }
        Operation::CreateNote(request) => {
            let note = persist(app, request_id, request)?;
            Ok((json!({ "id": note.id }), vec![Effect::Created(note)]))
        }
        Operation::UpdateNote(request) => update(&state, request),
        Operation::DeleteNote(NoteId { id }) => {
            live_note(&state, &id)?;
            commands::tombstone_note(&state, &id)?;
            Ok((json!({ "id": id, "deleted": true }), vec![Effect::Deleted(id)]))
        }
    }
}

/// Enter stdio mode before initializing Tauri, single-instance handling, or logs.
pub fn run_if_requested() -> bool {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--mcp")) { return false }
    let root = match (args.next(), args.next(), args.next()) {
        (Some(flag), Some(dir), None) if flag == "--mcp-data-dir" && PathBuf::from(&dir).is_absolute() => PathBuf::from(dir).join("mcp"),
        _ => {
            eprintln!("Use the MCP configuration copied from Pinboard Settings > Work with AI.");
            std::process::exit(2);
        }
    };
    let call = |op| match op {
        Operation::CreateNote(note) => bridge::submit(&root, note).map(|id| json!({ "id": id })),
        op => bridge::submit_op(&root, op),
    };
    if let Err(e) = protocol::serve(std::io::stdin().lock(), std::io::stdout().lock(), call) {
        if e.kind() != std::io::ErrorKind::BrokenPipe { eprintln!("Pinboard MCP transport failed: {e}"); }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_support::app_state;

    fn note(state: &AppState, body: &str, updated_at: i64) -> Note {
        let mut note = Note::new("other-pc", "sage", 0.6);
        note.body = body.into();
        note.created_at = updated_at;
        note.updated_at = updated_at;
        state.store.lock().unwrap().put(note.clone()).unwrap();
        note
    }

    #[test]
    fn list_searches_sorts_by_edit_and_filters_hidden_and_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let state = app_state(dir.path());
        let older = note(&state, "회의 준비\n자료 확인", 1_000);
        let newer = note(&state, "장보기\n우유, 회의실 예약", 2_000);
        let mut deleted = note(&state, "회의 취소", 3_000);
        deleted.deleted_at = Some(3_001);
        state.store.lock().unwrap().put(deleted).unwrap();
        state.local.lock().unwrap().windows.insert(older.id.clone(), WindowState { visible: false, pin: PinMode::Desktop, ..Default::default() });

        let all = list(&state, &ListNotes { query: Some(" 회의 ".into()), ..Default::default() });
        assert_eq!(all["total"], 2);
        assert_eq!(all["notes"][0]["id"], newer.id);
        assert_eq!(all["notes"][0]["title"], "장보기");
        assert_eq!(all["notes"][0]["preview"], "우유, 회의실 예약");
        assert_eq!(all["notes"][1]["pin"], "desktop");
        assert_eq!(all["notes"][1]["visible"], false);
        assert_eq!(all["notes"][1]["updatedAt"], "1970-01-01T09:00:01+09:00");
        let shown = list(&state, &ListNotes { include_hidden: Some(false), ..Default::default() });
        assert_eq!(shown["total"], 1);
        assert_eq!(list(&state, &ListNotes { limit: Some(1), ..Default::default() })["notes"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn update_appends_without_breaking_formats_and_records_window_changes() {
        let dir = tempfile::tempdir().unwrap();
        let state = app_state(dir.path());
        let mut original = note(&state, "할 일", 1_000);
        original.formats = vec![crate::model::TextFormat { from: 0, to: 3, bold: Some(true), italic: None, size: None, highlight: None }];
        state.store.lock().unwrap().put(original.clone()).unwrap();

        let (value, effects) = update(&state, UpdateNote {
            id: original.id.clone(), append: Some("- 우유".into()), tint: Some("amber".into()),
            pin: Some(PinMode::Top), visible: Some(false), ..Default::default()
        }).unwrap();
        let saved = state.store.lock().unwrap().get(&original.id).cloned().unwrap();
        assert_eq!(saved.body, "할 일\n- 우유");
        assert_eq!(saved.formats, original.formats);
        assert_eq!(saved.tint, "amber");
        assert_eq!(saved.device_id, "this-pc");
        assert!(saved.updated_at > original.updated_at);
        assert_eq!((value["pin"].as_str(), value["visible"].as_bool()), (Some("top"), Some(false)));
        let window = state.local.lock().unwrap().windows.get(&original.id).cloned().unwrap();
        assert_eq!((window.pin, window.visible), (PinMode::Top, false));
        assert!(matches!(effects.as_slice(), [Effect::Patched(..), Effect::Pin(_, PinMode::Top), Effect::Hide(_)]));

        let (_, effects) = update(&state, UpdateNote { id: original.id.clone(), body: Some("새 내용".into()), ..Default::default() }).unwrap();
        let saved = state.store.lock().unwrap().get(&original.id).cloned().unwrap();
        assert_eq!(saved.body, "새 내용");
        assert!(saved.formats.is_empty());
        assert_eq!(effects.len(), 1);
        // A visibility-only change leaves the synced note untouched.
        let before = saved.updated_at;
        update(&state, UpdateNote { id: original.id.clone(), visible: Some(true), ..Default::default() }).unwrap();
        assert_eq!(state.store.lock().unwrap().get(&original.id).unwrap().updated_at, before);
        assert!(update(&state, UpdateNote { id: uuid::Uuid::new_v4().to_string(), visible: Some(true), ..Default::default() }).is_err());
    }

    #[test]
    fn delete_tombstones_once_and_read_hides_deleted_notes() {
        let dir = tempfile::tempdir().unwrap();
        let state = app_state(dir.path());
        let target = note(&state, "지울 메모", 1_000);
        assert!(live_note(&state, &target.id).is_ok());
        live_note(&state, &target.id).unwrap();
        commands::tombstone_note(&state, &target.id).unwrap();
        let saved = state.store.lock().unwrap().get(&target.id).cloned().unwrap();
        assert!(saved.is_deleted());
        assert_eq!(live_note(&state, &target.id).unwrap_err(), NOT_FOUND);
        assert_eq!(list(&state, &ListNotes::default())["total"], 0);
    }

    #[test]
    fn startup_without_explicit_setup_does_not_create_mcp_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mcp");
        let state = load_state(root.clone());
        let control = state.control.lock().unwrap();
        assert!(!control.provisioned && !control.enabled && !control.available);
        assert!(!state.worker_started.load(Ordering::SeqCst));
        assert!(!root.exists());
    }

    #[test]
    fn legacy_enabled_configuration_does_not_bypass_setup() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mcp");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("settings.json"), r#"{"enabled":true}"#).unwrap();
        let state = load_state(root.clone());
        let control = state.control.lock().unwrap();
        assert!(!control.provisioned && !control.enabled && !control.available);
        assert!(!root.join("inbox").exists());
        assert!(!root.join("runtime.json").exists());
        assert_eq!(std::fs::read_to_string(root.join("settings.json")).unwrap(), r#"{"enabled":true}"#);
    }

    #[test]
    fn explicit_setup_survives_restart_with_connections_still_off() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("mcp");
        bridge::prepare(&root).unwrap();
        write_json(&root.join("settings.json"), &Config { provisioned: true, enabled: false, grant_version: GRANT_VERSION }).unwrap();
        let state = load_state(root);
        let control = state.control.lock().unwrap();
        assert!(control.provisioned && control.available);
        assert!(!control.enabled);
    }

    #[test]
    fn expanded_scope_requires_a_new_explicit_grant() {
        for old in [r#"{"provisioned":true,"enabled":true}"#,
            r#"{"provisioned":true,"enabled":true,"grant_version":1}"#] {
            let dir = tempfile::tempdir().unwrap();
            let root = dir.path().join("mcp");
            bridge::prepare(&root).unwrap();
            std::fs::write(root.join("settings.json"), old).unwrap();
            let state = load_state(root.clone());
            {
                let control = state.control.lock().unwrap();
                assert!(control.provisioned && control.available);
                assert!(!control.enabled, "Create-only consent must not permit reading or deleting");
            }
            assert_eq!(std::fs::read_to_string(root.join("settings.json")).unwrap(), old);
            set_enabled(&state, true).unwrap();
            let restarted = load_state(root);
            assert!(restarted.control.lock().unwrap().enabled);
        }
    }

    #[test]
    fn long_first_lines_and_escaped_text_keep_replies_within_transport_limit() {
        let mut note = Note::new("test-pc", "sage", 0.6);
        note.body = "\u{0001}".repeat(crate::validation::MAX_BODY_BYTES);
        let summary = describe(&note, None, false);
        assert_eq!(summary["title"].as_str().unwrap().chars().count(), 200);
        for result in [json!({ "notes": vec![summary; 200] }), describe(&note, None, true)] {
            let reply = Reply { session: uuid::Uuid::new_v4().to_string(), note_id: None, error: None, result: Some(result) };
            assert!(serde_json::to_vec_pretty(&reply).unwrap().len() <= bridge::MAX_REPLY);
        }
    }
}
