//! A local inbox between stdio clients and the running app. No listening socket,
//! AI credentials, or direct access to the note store is needed by the client.
use crate::model::{now_ms, PinMode, TINTS};
use crate::store::write_json;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const MAX_FRAME: usize = 128 * 1024;
/// Replies can carry a whole note or a list, so they get a larger bound than requests.
pub const MAX_REPLY: usize = 2 * 1024 * 1024;
pub const MAX_BODY_CHARS: usize = 20_000;
pub const REQUEST_TTL_MS: i64 = 15_000;
const HEARTBEAT_TTL_MS: i64 = 5_000;

fn check_body(body: &str, allow_empty: bool) -> Result<(), String> {
    if !allow_empty && body.trim().is_empty() {
        return Err("The note body must not be empty.".into());
    }
    if body.chars().count() > MAX_BODY_CHARS {
        return Err("The note body must be at most 20,000 characters.".into());
    }
    Ok(())
}

fn check_tint(tint: &Option<String>) -> Result<(), String> {
    if tint.as_ref().is_some_and(|t| !TINTS.contains(&t.as_str())) {
        return Err("Unknown tint. Use frost, sage, lilac, amber, rose, or ink.".into());
    }
    Ok(())
}

fn check_id(id: &str) -> Result<(), String> {
    match uuid::Uuid::parse_str(id) {
        Ok(parsed) if parsed.to_string() == id => Ok(()),
        _ => Err("Unknown note ID. Use an id returned by list_notes.".into()),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNote {
    pub body: String,
    #[serde(default)]
    pub tint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<PinMode>,
}

impl CreateNote {
    pub fn validate(&self) -> Result<(), String> {
        check_body(&self.body, false)?;
        check_tint(&self.tint)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListNotes {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub include_hidden: Option<bool>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NoteId {
    pub id: String,
}

/// Every field except `id` is optional; omitted fields stay as they are.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateNote {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<PinMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
}

/// One MCP tool call, as forwarded to the running app.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum Operation {
    ListNotes(ListNotes),
    ReadNote(NoteId),
    CreateNote(CreateNote),
    UpdateNote(UpdateNote),
    DeleteNote(NoteId),
}

impl Operation {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::ListNotes(list) => {
                if list.limit.is_some_and(|l| !(1..=200).contains(&l)) {
                    return Err("limit must be between 1 and 200.".into());
                }
                if list.query.as_ref().is_some_and(|q| q.chars().count() > 200) {
                    return Err("query must be at most 200 characters.".into());
                }
                Ok(())
            }
            Self::ReadNote(note) | Self::DeleteNote(note) => check_id(&note.id),
            Self::CreateNote(note) => note.validate(),
            Self::UpdateNote(update) => {
                check_id(&update.id)?;
                if update.body.is_some() && update.append.is_some() {
                    return Err("Use either body (replace) or append, not both.".into());
                }
                if let Some(body) = &update.body { check_body(body, true)?; }
                if let Some(append) = &update.append { check_body(append, false)?; }
                check_tint(&update.tint)?;
                if update.opacity.is_some_and(|o| !o.is_finite() || !(0.0..=1.0).contains(&o)) {
                    return Err("opacity must be between 0 and 1.".into());
                }
                if update.body.is_none() && update.append.is_none() && update.tint.is_none()
                    && update.opacity.is_none() && update.pin.is_none() && update.visible.is_none() {
                    return Err("Nothing to update. Provide body, append, tint, opacity, pin, or visible.".into());
                }
                Ok(())
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub session: String,
    pub enabled: bool,
    pub heartbeat_at: i64,
}

impl Runtime {
    fn check(&self) -> Result<(), String> {
        if !self.enabled {
            return Err("MCP is off. Turn it on in Pinboard Settings > Work with AI.".into());
        }
        let age = now_ms().saturating_sub(self.heartbeat_at);
        if !(0..=HEARTBEAT_TTL_MS).contains(&age) || uuid::Uuid::parse_str(&self.session).is_err() {
            return Err("Pinboard is not responding. Open Pinboard and try again.".into());
        }
        Ok(())
    }
}

/// `note` is the original create-only form; MCP processes started from an older
/// executable keep sending it, so exactly one of `note` or `op` must be present.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub session: String,
    pub expires_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<CreateNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op: Option<Operation>,
}

impl Request {
    pub fn validate(&self, session: &str, enabled: bool) -> Result<(), String> {
        if !enabled || self.session != session {
            return Err("The MCP connection was turned off or restarted. Try again after enabling it.".into());
        }
        let remaining = self.expires_at.saturating_sub(now_ms());
        if !(1..=REQUEST_TTL_MS + HEARTBEAT_TTL_MS).contains(&remaining) {
            return Err("The note request expired. Please try again.".into());
        }
        self.operation()?.validate()
    }

    pub fn operation(&self) -> Result<Operation, String> {
        match (&self.note, &self.op) {
            (Some(note), None) => Ok(Operation::CreateNote(note.clone())),
            (None, Some(op)) => Ok(op.clone()),
            _ => Err("Invalid note request.".into()),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    pub session: String,
    pub note_id: Option<String>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

pub fn read_bounded<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    read_limited(path, MAX_FRAME)
}

fn read_limited<T: DeserializeOwned>(path: &Path, limit: usize) -> io::Result<T> {
    // Never follow a symlink placed in the inbox or runtime paths.
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > limit as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid MCP file"));
    }
    let mut bytes = Vec::new();
    File::open(path)?.take((limit + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "MCP file is too large"));
    }
    serde_json::from_slice(&bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid MCP JSON"))
}

pub fn prepare(root: &Path) -> io::Result<()> {
    for path in [root.to_path_buf(), root.join("inbox"), root.join("replies")] {
        fs::create_dir_all(&path)?;
        if !fs::symlink_metadata(&path)?.file_type().is_dir() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid MCP directory"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(())
}

pub fn runtime(root: &Path) -> Result<Runtime, String> {
    let value: Runtime = read_bounded(&root.join("runtime.json"))
        .map_err(|_| "Open Pinboard and enable MCP in Settings > Work with AI.".to_string())?;
    value.check()?;
    Ok(value)
}

struct Pending {
    request: PathBuf,
    reply: PathBuf,
}

impl Drop for Pending {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.request);
        let _ = fs::remove_file(&self.reply);
    }
}

/// Creates a note. Without a pin it uses the original request form, which an app that
/// has not been restarted since an update still understands.
pub fn submit(root: &Path, note: CreateNote) -> Result<String, String> {
    note.validate()?;
    let value = if note.pin.is_none() { exchange(root, Some(note), None)? } else { exchange(root, None, Some(Operation::CreateNote(note)))? };
    value.get("id").and_then(Value::as_str).map(String::from).ok_or_else(|| "Pinboard returned an invalid response.".into())
}

/// Runs any tool call in the app and returns its JSON result.
pub fn submit_op(root: &Path, op: Operation) -> Result<Value, String> {
    op.validate()?;
    exchange(root, None, Some(op))
}

fn exchange(root: &Path, note: Option<CreateNote>, op: Option<Operation>) -> Result<Value, String> {
    let connection = runtime(root)?;
    let id = uuid::Uuid::new_v4().to_string();
    let pending = Pending {
        request: root.join("inbox").join(format!("{id}.json")),
        reply: root.join("replies").join(format!("{id}.json")),
    };
    let request = Request {
        session: connection.session.clone(),
        expires_at: now_ms() + REQUEST_TTL_MS,
        note,
        op,
    };
    write_json(&pending.request, &request).map_err(|_| "Could not send the request to Pinboard.".to_string())?;
    let start = Instant::now();
    loop {
        if let Ok(reply) = read_limited::<Reply>(&pending.reply, MAX_REPLY) {
            if reply.session != connection.session {
                return Err("The MCP connection changed. Please try again.".into());
            }
            return match (reply.result, reply.note_id, reply.error) {
                (_, _, Some(error)) => Err(error),
                (Some(result), _, None) => Ok(result),
                // An app from before full note control replies with only the created ID.
                (None, Some(id), None) if uuid::Uuid::parse_str(&id).is_ok() => Ok(json!({ "id": id })),
                _ => Err("Pinboard returned an invalid response.".into()),
            };
        }
        if start.elapsed() >= Duration::from_millis(REQUEST_TTL_MS as u64) {
            return Err("Pinboard did not respond in time. Check the app before retrying.".into());
        }
        let current = runtime(root)?;
        if current.session != connection.session {
            return Err("The MCP connection was restarted. Please try again.".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Claims only canonical UUID filenames. The model cannot supply a path or note ID.
pub fn claim(path: &Path) -> Option<(String, PathBuf)> {
    let name = path.file_name()?.to_str()?;
    let id = name.strip_suffix(".json")?;
    if uuid::Uuid::parse_str(id).ok()?.to_string() != id {
        return None;
    }
    if !fs::symlink_metadata(path).ok()?.file_type().is_file() {
        return None;
    }
    let working = path.with_extension("working");
    fs::rename(path, &working).ok()?;
    Some((id.to_owned(), working))
}

/// Remove expired files left behind by an interrupted client. Never recurse.
pub fn cleanup(root: &Path) {
    for folder in ["inbox", "replies"] {
        let Ok(entries) = fs::read_dir(root.join(folder)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            if uuid::Uuid::parse_str(stem).is_err()
                || !matches!(path.extension().and_then(|s| s.to_str()), Some("json" | "working"))
            {
                continue;
            }
            let Ok(meta) = fs::symlink_metadata(&path) else { continue };
            if meta.file_type().is_file()
                && meta.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > Duration::from_secs(60))
            {
                let _ = fs::remove_file(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_empty_oversized_and_unknown_fields() {
        assert!(CreateNote { body: " \n".into(), tint: None, pin: None }.validate().is_err());
        assert!(CreateNote { body: "가".repeat(20_001), tint: None, pin: None }.validate().is_err());
        assert!(CreateNote { body: "가".repeat(20_000), tint: Some("sage".into()), pin: None }.validate().is_ok());
        assert!(CreateNote { body: "hello".into(), tint: Some("red".into()), pin: None }.validate().is_err());
        assert!(serde_json::from_str::<CreateNote>(r#"{"body":"hello","id":"existing-note"}"#).is_err());
    }

    #[test]
    fn disabled_restarted_and_expired_requests_cannot_create_notes() {
        let session = uuid::Uuid::new_v4().to_string();
        let mut request = Request {
            session: session.clone(), expires_at: now_ms() + 1_000,
            note: Some(CreateNote { body: "hello".into(), tint: None, pin: None }), op: None,
        };
        assert!(request.validate(&session, true).is_ok());
        assert!(request.validate(&session, false).is_err());
        assert!(request.validate("new-session", true).is_err());
        request.expires_at = now_ms() - 1;
        assert!(request.validate(&session, true).is_err());
        request.expires_at = i64::MAX;
        assert!(request.validate(&session, true).is_err());
    }

    #[test]
    fn closed_app_and_disabled_bridge_do_not_enqueue_notes() {
        let dir = tempfile::tempdir().unwrap();
        prepare(dir.path()).unwrap();
        let note = CreateNote { body: "hello".into(), tint: None, pin: None };
        assert!(submit(dir.path(), note.clone()).is_err());
        write_json(&dir.path().join("runtime.json"), &Runtime {
            session: uuid::Uuid::new_v4().to_string(), enabled: false, heartbeat_at: now_ms(),
        }).unwrap();
        assert!(submit(dir.path(), note).is_err());
        assert_eq!(fs::read_dir(dir.path().join("inbox")).unwrap().count(), 0);
    }

    #[test]
    fn local_roundtrip_returns_only_created_note_id_and_cleans_files() {
        let dir = tempfile::tempdir().unwrap();
        prepare(dir.path()).unwrap();
        let root = dir.path().to_path_buf();
        let session = uuid::Uuid::new_v4().to_string();
        write_json(&root.join("runtime.json"), &Runtime {
            session: session.clone(), enabled: true, heartbeat_at: now_ms(),
        }).unwrap();
        let app = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                for entry in fs::read_dir(root.join("inbox")).unwrap().flatten() {
                    if let Some((id, path)) = claim(&entry.path()) {
                        let request: Request = read_bounded(&path).unwrap();
                        request.validate(&session, true).unwrap();
                        assert_eq!(request.note.as_ref().unwrap().body, "회의 준비\n- [ ] 자료 확인");
                        write_json(&root.join("replies").join(format!("{id}.json")), &Reply {
                            session: session.clone(), note_id: Some(id.clone()), error: None, result: None,
                        }).unwrap();
                        fs::remove_file(path).unwrap();
                        return id;
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            panic!("No MCP request arrived");
        });
        let id = submit(dir.path(), CreateNote { body: "회의 준비\n- [ ] 자료 확인".into(), tint: Some("amber".into()), pin: None }).unwrap();
        assert_eq!(id, app.join().unwrap());
        assert_eq!(fs::read_dir(dir.path().join("inbox")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(dir.path().join("replies")).unwrap().count(), 0);
    }

    #[test]
    fn legacy_and_operation_requests_both_parse_and_validate() {
        let session = uuid::Uuid::new_v4().to_string();
        let expires = now_ms() + 1_000;
        let legacy = format!(r#"{{"session":"{session}","expiresAt":{expires},"note":{{"body":"hi","tint":"sage"}}}}"#);
        let legacy: Request = serde_json::from_str(&legacy).unwrap();
        assert!(matches!(legacy.operation().unwrap(), Operation::CreateNote(ref n) if n.body == "hi"));
        assert!(legacy.validate(&session, true).is_ok());
        let id = uuid::Uuid::new_v4().to_string();
        let op = format!(r#"{{"session":"{session}","expiresAt":{expires},"op":{{"tool":"update_note","id":"{id}","append":"- milk","pin":"top"}}}}"#);
        let op: Request = serde_json::from_str(&op).unwrap();
        assert!(matches!(op.operation().unwrap(), Operation::UpdateNote(ref u) if u.pin == Some(PinMode::Top)));
        assert!(op.validate(&session, true).is_ok());
        let both = Request { session: session.clone(), expires_at: expires, note: legacy.note.clone(), op: op.op.clone() };
        assert!(both.validate(&session, true).is_err());
        let neither = Request { session: session.clone(), expires_at: expires, note: None, op: None };
        assert!(neither.validate(&session, true).is_err());
    }

    #[test]
    fn operations_reject_bad_ids_empty_updates_and_bad_values() {
        let id = uuid::Uuid::new_v4().to_string();
        assert!(Operation::ReadNote(NoteId { id: "../settings".into() }).validate().is_err());
        assert!(Operation::DeleteNote(NoteId { id: id.to_uppercase() }).validate().is_err());
        assert!(Operation::DeleteNote(NoteId { id: crate::welcome::ID.into() }).validate().is_ok());
        assert!(Operation::UpdateNote(UpdateNote { id: id.clone(), ..Default::default() }).validate().is_err());
        assert!(Operation::UpdateNote(UpdateNote { id: id.clone(), body: Some(String::new()), ..Default::default() }).validate().is_ok());
        assert!(Operation::UpdateNote(UpdateNote { id: id.clone(), body: Some("a".into()), append: Some("b".into()), ..Default::default() }).validate().is_err());
        assert!(Operation::UpdateNote(UpdateNote { id: id.clone(), opacity: Some(1.5), ..Default::default() }).validate().is_err());
        assert!(Operation::UpdateNote(UpdateNote { id: id.clone(), tint: Some("red".into()), ..Default::default() }).validate().is_err());
        assert!(Operation::UpdateNote(UpdateNote { id, visible: Some(false), ..Default::default() }).validate().is_ok());
        assert!(Operation::ListNotes(ListNotes { limit: Some(0), ..Default::default() }).validate().is_err());
        assert!(Operation::ListNotes(ListNotes::default()).validate().is_ok());
        assert!(serde_json::from_str::<Operation>(r#"{"tool":"list_notes","path":"C:/"}"#).is_err());
    }

    #[test]
    fn replies_stay_readable_by_older_and_newer_bridges() {
        let session = "s".to_string();
        let old = serde_json::to_string(&Reply { session: session.clone(), note_id: Some("x".into()), error: None, result: None }).unwrap();
        assert!(!old.contains("result"));
        let new: Reply = serde_json::from_str(r#"{"session":"s","noteId":null,"error":null,"result":{"notes":[]}}"#).unwrap();
        assert_eq!(new.result, Some(json!({ "notes": [] })));
    }
}