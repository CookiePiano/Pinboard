use crate::model::{Note, PinMode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Immutable content shared by background snapshots; the cache is never persisted.
#[derive(Clone, Debug)]
pub struct SharedNote(Arc<StoredNote>);

#[derive(Debug)]
struct StoredNote { note: Note, hash: OnceLock<String> }

impl From<Note> for SharedNote {
    fn from(note: Note) -> Self { Self(Arc::new(StoredNote { note, hash: OnceLock::new() })) }
}
impl std::ops::Deref for SharedNote {
    type Target = Note;
    fn deref(&self) -> &Note { &self.0.note }
}
impl std::borrow::Borrow<Note> for SharedNote {
    fn borrow(&self) -> &Note { self }
}
impl Serialize for SharedNote {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.note.serialize(serializer)
    }
}
impl SharedNote {
    pub fn content_hash(&self) -> String { self.0.hash.get_or_init(|| self.0.note.content_hash()).clone() }

    pub(crate) fn hashed(note: Note, hash: String) -> Self {
        Self(Arc::new(StoredNote { note, hash: OnceLock::from(hash) }))
    }
}

/// Writes `bytes` to `path` through a temporary sibling file so readers never see a partial file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_atomic_with(path, |file| file.write_all(bytes))
}

fn write_atomic_with(path: &Path, write: impl FnOnce(&mut fs::File) -> std::io::Result<()>) -> std::io::Result<()> {
    let dir = path.parent().expect("path has a parent");
    fs::create_dir_all(dir)?;
    let name = path.file_name().unwrap().to_string_lossy();
    let tmp = dir.join(format!(".{name}.{}.tmp", uuid::Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut f = options.open(&tmp)?;
        write(&mut f)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)
    })();
    result.inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("ignoring unreadable {}: {e}", path.display());
            None
        }
    }
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    write_atomic_with(path, |file| {
        // Embedded images can be several MiB. Keep only a small output buffer in memory.
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, value)?;
        // Flush before the file is synced and renamed; Drop would discard flush errors.
        writer.flush()
    })
}

/// Returns the note id for a `<id>.json` file name, skipping temp and hidden files.
pub fn note_id_from_file(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if name.starts_with('.') || name.starts_with('~') {
        return None;
    }
    let id = name.strip_suffix(".json")?;
    crate::validation::note_id(id).ok()?;
    Some(id.to_string())
}

/// All notes on this device, including tombstones that still need to sync.
pub struct Store {
    dir: PathBuf,
    notes: HashMap<String, SharedNote>,
}

impl Store {
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&dir)?;
        let mut notes = HashMap::new();
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            let Some(id) = note_id_from_file(&path) else { continue };
            match crate::validation::read_note(&path, &id) {
                Ok(note) => { notes.insert(note.id.clone(), note.into()); }
                Err(_) => log::warn!("ignoring invalid note file: {id}"),
            }
        }
        Ok(Store { dir, notes })
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    pub fn get(&self, id: &str) -> Option<&Note> {
        self.notes.get(id).map(|n| &**n)
    }

    pub fn all(&self) -> impl Iterator<Item = &Note> {
        self.notes.values().map(|n| &**n)
    }

    pub fn live(&self) -> impl Iterator<Item = &Note> {
        self.all().filter(|n| !n.is_deleted())
    }

    pub fn snapshot(&self) -> HashMap<String, SharedNote> {
        self.notes.clone()
    }

    pub fn shared(&self) -> impl Iterator<Item = &SharedNote> { self.notes.values() }

    pub fn hash(&self, id: &str) -> Option<String> { self.notes.get(id).map(SharedNote::content_hash) }

    /// Install an already validated, durably committed archive without another disk read.
    pub fn replace_snapshot(&mut self, notes: Vec<Note>) {
        self.notes = notes.into_iter().map(|n| (n.id.clone(), n.into())).collect();
    }

    /// Inherit only appearance and size from the newest surviving note, even if hidden.
    /// Editing an older note must not make it the template for new notes.
    pub fn draft(&self, device_id: &str, local: &LocalState) -> (Note, WindowState) {
        self.draft_from(device_id, local, None)
    }

    /// MCP notes match the note the user most recently wrote in, not merely the newest one,
    /// so an empty or hidden scratch note cannot set the style of AI-created notes.
    pub fn draft_like_last_written(&self, device_id: &str, local: &LocalState) -> (Note, WindowState) {
        let source = self.live().filter(|n| !n.body.trim().is_empty())
            .max_by(|a, b| a.updated_at.cmp(&b.updated_at).then(a.created_at.cmp(&b.created_at)).then(a.id.cmp(&b.id)))
            .map(|n| n.id.clone());
        self.draft_from(device_id, local, source.as_deref())
    }

    pub fn draft_from(&self, device_id: &str, local: &LocalState, source: Option<&str>) -> (Note, WindowState) {
        let latest = source.and_then(|id| self.get(id)).filter(|n| !n.is_deleted())
            .or_else(|| self.live().max_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id))));
        let mut ws = WindowState::default();
        let mut note = match latest {
            Some(latest) => {
                if let Some(previous) = local.windows.get(&latest.id) {
                    ws.width = previous.width;
                    ws.height = previous.height;
                }
                // Creation dates follow the PC clock, even if an older note has a future date.
                let mut note = Note::new(device_id, &latest.tint, latest.opacity);
                note.background = latest.background.clone();
                note.font = latest.font.clone();
                note
            }
            None => Note::new(device_id, "frost", 0.55),
        };
        note.updated_at = note.created_at;
        (note, ws)
    }

    pub fn put(&mut self, note: Note) -> std::io::Result<()> {
        self.put_shared(note.into())
    }

    pub fn put_shared(&mut self, note: SharedNote) -> std::io::Result<()> {
        crate::validation::note(&note)?;
        write_json(&self.path(&note.id), &note)?;
        self.notes.insert(note.id.clone(), note);
        Ok(())
    }

    /// Removes the note file entirely. Used once a tombstone has been purged everywhere.
    pub fn purge(&mut self, id: &str) -> std::io::Result<()> {
        crate::validation::note_id(id)?;
        match fs::remove_file(self.path(id)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        self.notes.remove(id);
        Ok(())
    }
}

/// Per-device window placement for one note. Never synced.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: f64,
    pub height: f64,
    pub pin: PinMode,
    pub visible: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        WindowState { x: None, y: None, width: 280.0, height: 260.0, pin: PinMode::Normal, visible: true }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct LocalState {
    #[serde(default)]
    pub windows: HashMap<String, WindowState>,
    /// Set once the welcome note has been offered on this device.
    #[serde(default)]
    pub welcomed: bool,
    /// Remember initialization without changing the user's OS startup registration.
    #[serde(default)]
    pub autostart_initialized: bool,
    /// The calendar window keeps its own pin mode, like a note.
    #[serde(default)]
    pub calendar_pin: PinMode,
}

impl LocalState {
    pub fn initialize_autostart(&mut self) -> bool {
        if self.autostart_initialized { return false; }
        // Startup is opt-in. In particular, do not disable an existing registration.
        self.autostart_initialized = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_snapshots_keep_old_content_and_hashes_without_copying_images() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().join("notes")).unwrap();
        let mut note = Note::new("test", "frost", 0.55);
        note.background = Some(crate::model::NoteBackground { image: format!("data:image/png;base64,{}", "A".repeat(1024 * 1024)), opacity: 0.3 });
        store.put(note.clone()).unwrap();
        let first = store.snapshot();
        let second = store.snapshot();
        assert!(Arc::ptr_eq(&first[&note.id].0, &second[&note.id].0));
        assert_eq!(first[&note.id].content_hash(), note.content_hash());
        assert_eq!(serde_json::to_vec(&first[&note.id]).unwrap(), serde_json::to_vec(&note).unwrap());
        note.body = "a newer edit 한😀글".into();
        store.put(note.clone()).unwrap();
        assert!(first[&note.id].body.is_empty());
        assert_eq!(store.hash(&note.id), Some(note.content_hash()));
        assert_ne!(store.hash(&note.id), Some(first[&note.id].content_hash()));
        store.purge(&note.id).unwrap();
        assert!(store.snapshot().is_empty());
        assert!(first[&note.id].body.is_empty());
    }

    #[test]
    #[ignore = "explicit performance probe; run alone with --nocapture"]
    fn snapshot_benchmark() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().join("notes")).unwrap();
        for _ in 0..10 {
            let mut note = Note::new("test", "frost", 0.55);
            note.background = Some(crate::model::NoteBackground { image: format!("data:image/png;base64,{}", "A".repeat(1536 * 1024)), opacity: 0.3 });
            store.put(note).unwrap();
        }
        for note in store.snapshot().values() { std::hint::black_box(note.content_hash()); }
        let mut old = Vec::new(); let mut shared = Vec::new();
        for _ in 0..5 {
            let start = std::time::Instant::now();
            let copied: HashMap<_, _> = store.all().map(|n| (n.id.clone(), n.clone())).collect();
            for n in copied.values() { std::hint::black_box(n.content_hash()); }
            old.push(start.elapsed().as_secs_f64() * 1000.0);
            drop(copied);
            let start = std::time::Instant::now();
            let snapshot = store.snapshot();
            for n in snapshot.values() { std::hint::black_box(n.content_hash()); }
            shared.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        println!("snapshot + hashes, five trials; copied_ms={old:?}; shared_ms={shared:?}");
    }

    #[test]
    fn streamed_json_preserves_pretty_bytes_and_large_note_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/note.json");
        let mut note = Note::new("test", "sage", 0.5);
        note.body = "한글 😀\n\"quoted\"\t\\text".into();
        note.background = Some(crate::model::NoteBackground {
            image: format!("data:image/png;base64,{}", "A".repeat(1024 * 1024)), opacity: 0.25,
        });
        note.formats.push(crate::model::TextFormat {
            from: 3, to: 5, bold: Some(true), italic: None, size: Some(12.5), highlight: Some("#ffe066".into()),
        });
        note.font = Some(crate::fonts::NoteFont { family: Some("Noto Sans KR".into()), size: 16.0, weight: 400 });
        write_json(&path, &note).unwrap();
        assert_eq!(fs::read(&path).unwrap(), serde_json::to_vec_pretty(&note).unwrap());
        assert_eq!(crate::validation::read_note(&path, &note.id).unwrap(), note);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn serialization_failure_preserves_destination_and_removes_partial_file() {
        struct Failing;
        impl Serialize for Failing {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("payload", &"x".repeat(16 * 1024))?;
                Err(serde::ser::Error::custom("intentional serialization failure"))
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        for existing in [false, true] {
            if existing { fs::write(&path, b"previous content").unwrap(); }
            assert_eq!(write_json(&path, &Failing).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
            if existing { assert_eq!(fs::read(&path).unwrap(), b"previous content"); }
            else { assert!(!path.exists()); }
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), usize::from(existing));
        }
    }

    #[test]
    fn atomic_write_failure_preserves_destination_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, b"previous content").unwrap();
        let error = write_atomic_with(&path, |file| {
            file.write_all(b"partial replacement")?;
            Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "intentional write failure"))
        }).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert_eq!(fs::read(&path).unwrap(), b"previous content");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);

        let blocked = dir.path().join("directory.json");
        fs::create_dir(&blocked).unwrap();
        assert!(write_json(&blocked, &vec![1, 2, 3]).is_err());
        assert!(blocked.is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn purge_failure_keeps_the_note_in_memory_and_allows_retry() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().join("notes")).unwrap();
        let note = Note::new("test", "frost", 0.5);
        store.put(note.clone()).unwrap();
        let path = store.path(&note.id);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(store.purge(&note.id).is_err());
        assert_eq!(store.get(&note.id), Some(&note));
        fs::remove_dir(&path).unwrap();
        store.purge(&note.id).unwrap();
        assert!(store.get(&note.id).is_none());
        store.purge(&note.id).unwrap();
    }

    #[test]
    fn malicious_ids_cannot_write_or_delete_outside_notes() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().join("settings.json");
        fs::write(&outside, b"keep me").unwrap();
        let mut store = Store::open(dir.path().join("notes")).unwrap();
        let mut note = Note::new("test", "frost", 0.5);
        for id in ["../settings", "..\\settings"] {
            note.id = id.into();
            assert!(store.put(note.clone()).is_err());
            assert!(store.purge(id).is_err());
            assert_eq!(fs::read(&outside).unwrap(), b"keep me");
        }
        assert_eq!(store.all().count(), 0);
    }

    #[test]
    fn reopening_store_does_not_trust_the_json_id_or_follow_note_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes");
        fs::create_dir(&notes).unwrap();
        let note = Note::new("test", "frost", 0.5);
        write_json(&notes.join(format!("{}.json", uuid::Uuid::new_v4())), &note).unwrap();
        let external = dir.path().join("external.json");
        write_json(&external, &note).unwrap();
        let link = notes.join(format!("{}.json", note.id));
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_file(&external, &link);
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&external, &link);
        if linked.is_ok() { assert!(crate::validation::read_note(&link, &note.id).is_err()); }
        assert_eq!(Store::open(notes).unwrap().all().count(), 0);
        assert!(external.exists());
    }

    #[test]
    fn legacy_autostart_state_is_marked_without_touching_os_registration() {
        let mut local: LocalState = serde_json::from_str(r#"{"welcomed":true}"#).unwrap();
        assert!(local.initialize_autostart());
        let mut restored: LocalState = serde_json::from_str(&serde_json::to_string(&local).unwrap()).unwrap();
        assert!(!restored.initialize_autostart());
    }

    #[test]
    fn calendar_pin_defaults_to_normal_and_round_trips() {
        let mut local: LocalState = serde_json::from_str(r#"{"welcomed":true}"#).unwrap();
        assert_eq!(local.calendar_pin, PinMode::Normal);
        local.calendar_pin = PinMode::Desktop;
        let json = serde_json::to_string(&local).unwrap();
        assert!(json.contains(r#""calendar_pin":"desktop""#));
        assert_eq!(serde_json::from_str::<LocalState>(&json).unwrap().calendar_pin, PinMode::Desktop);
    }

    #[test]
    fn fresh_and_existing_autostart_initialization_are_idempotent() {
        let mut local = LocalState::default();
        assert!(local.initialize_autostart());
        assert!(local.autostart_initialized);
        assert!(!local.initialize_autostart());
    }

    #[test]
    fn draft_inherits_latest_live_note_appearance_and_size_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().to_path_buf()).unwrap();
        let mut older = Note::new("dev", "rose", 0.8);
        older.created_at = 1;
        older.updated_at = i64::MAX;
        store.put(older).unwrap();
        let mut latest = Note::new("dev", "sage", 0.23);
        latest.created_at = 2;
        latest.body = "Do not copy text".into();
        store.put(latest.clone()).unwrap();
        let mut deleted = Note::new("dev", "ink", 1.0);
        deleted.created_at = 3;
        deleted.deleted_at = Some(4);
        store.put(deleted).unwrap();
        let mut local = LocalState::default();
        local.windows.insert(latest.id.clone(), WindowState {
            width: 415.0, height: 333.0, x: Some(400.0), y: Some(200.0),
            pin: PinMode::Desktop, visible: false,
        });
        let (draft, ws) = store.draft("new-device", &local);
        assert_eq!((draft.tint.as_str(), draft.opacity), ("sage", 0.23));
        assert_eq!((ws.width, ws.height), (415.0, 333.0));
        assert_eq!((ws.x, ws.y, ws.pin, ws.visible), (None, None, PinMode::Normal, true));
        assert!(draft.body.is_empty());
        assert_ne!(draft.id, latest.id);
        assert_eq!(draft.device_id, "new-device");
        assert!(draft.created_at > latest.created_at);
    }

    #[test]
    fn drafts_inherit_source_background_even_when_source_is_older() {
        use crate::model::{NoteBackground, TextFormat};
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().join("notes")).unwrap();
        let mut source = Note::new("dev", "rose", 0.7);
        source.created_at = 1;
        source.font = Some(crate::fonts::NoteFont { family: Some("Noto Sans KR".into()), size: 20.0, weight: 400 });
        source.body = "keep this text in the source".into();
        source.background = Some(NoteBackground { image: "data:image/webp;base64,dGVzdA==".into(), opacity: 0.22 });
        source.formats = vec![TextFormat { from: 0, to: 4, bold: Some(true), italic: None, size: None, highlight: None }];
        store.put(source.clone()).unwrap();
        let mut latest = Note::new("dev", "sage", 0.5);
        latest.created_at = 2;
        store.put(latest).unwrap();
        let local = LocalState::default();
        let (child, _) = store.draft_from("dev", &local, Some(&source.id));
        assert_eq!(child.background, source.background);
        assert_eq!(child.font, source.font);
        assert_eq!(child.tint, source.tint);
        assert!(child.body.is_empty() && child.formats.is_empty());
        store.put(child.clone()).unwrap();
        let (next, _) = store.draft_from("dev", &local, Some(&child.id));
        assert_eq!(next.background, source.background);
        assert_eq!(next.font, source.font);
        assert_eq!(store.draft("dev", &local).0.background, source.background);
    }

    #[test]
    fn last_written_draft_follows_the_most_recently_edited_note_with_text() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().join("notes")).unwrap();
        let mut edited = Note::new("dev", "rose", 0.7);
        edited.created_at = 1;
        edited.updated_at = 50;
        edited.body = "today's plan".into();
        edited.font = Some(crate::fonts::NoteFont { family: Some("Noto Sans KR".into()), size: 10.5, weight: 400 });
        store.put(edited.clone()).unwrap();
        let mut empty = Note::new("dev", "sage", 0.3);
        empty.created_at = 60;
        empty.updated_at = 60;
        store.put(empty.clone()).unwrap();
        let mut stale = Note::new("dev", "ink", 0.9);
        stale.created_at = 40;
        stale.updated_at = 40;
        stale.body = "older text".into();
        store.put(stale).unwrap();
        let mut local = LocalState::default();
        local.windows.insert(edited.id.clone(), WindowState { width: 556.0, height: 365.0, ..Default::default() });
        local.windows.insert(empty.id.clone(), WindowState { width: 509.0, height: 461.0, ..Default::default() });

        let (draft, ws) = store.draft_like_last_written("dev", &local);
        assert_eq!((draft.tint.as_str(), draft.opacity, draft.font.clone()), ("rose", 0.7, edited.font.clone()));
        assert_eq!((ws.width, ws.height), (556.0, 365.0));
        assert!(draft.body.is_empty());
        // New notes from the + button, tray or shortcut keep following the newest note.
        assert_eq!(store.draft("dev", &local).0.tint, "sage");

        let empty_store = Store::open(dir.path().join("empty")).unwrap();
        assert_eq!(empty_store.draft_like_last_written("dev", &LocalState::default()).0.tint, "frost");
    }

    #[test]
    fn draft_uses_pc_time_even_when_an_existing_note_has_a_future_date() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().join("notes")).unwrap();
        let mut future = Note::new("other-pc", "sage", 0.7);
        future.created_at += 7 * 24 * 60 * 60 * 1000;
        store.put(future.clone()).unwrap();
        let before = crate::model::now_ms();
        let (draft, _) = store.draft("this-pc", &LocalState::default());
        let after = crate::model::now_ms();
        assert!(draft.created_at >= before && draft.created_at <= after);
        assert!(draft.created_at < future.created_at);
        assert_eq!(draft.tint, future.tint);
        assert_eq!(draft.updated_at, draft.created_at);
    }

    #[test]
    fn draft_has_defaults_without_notes_and_without_local_size() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().to_path_buf()).unwrap();
        let local = LocalState::default();
        let (draft, ws) = store.draft("dev", &local);
        assert_eq!((draft.tint.as_str(), draft.opacity), ("frost", 0.55));
        assert_eq!((ws.width, ws.height), (280.0, 260.0));
        let mut remote = Note::new("other", "amber", 0.7);
        remote.created_at = i64::MAX - 1;
        store.put(remote).unwrap();
        let (draft, ws) = store.draft("dev", &local);
        assert_eq!((draft.tint.as_str(), draft.opacity), ("amber", 0.7));
        assert_eq!((ws.width, ws.height), (280.0, 260.0));
        assert!(draft.created_at <= crate::model::now_ms());
    }

    #[test]
    fn store_round_trip_and_purge() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().to_path_buf()).unwrap();
        let note = Note::new("dev", "sage", 0.5);
        let id = note.id.clone();
        store.put(note.clone()).unwrap();
        // Stray files are ignored on load.
        fs::write(dir.path().join(".x.json.tmp"), "junk").unwrap();
        fs::write(dir.path().join("readme.json"), "{}").unwrap();

        let reopened = Store::open(dir.path().to_path_buf()).unwrap();
        assert_eq!(reopened.get(&id), Some(&note));
        assert_eq!(reopened.all().count(), 1);

        store.purge(&id).unwrap();
        assert!(Store::open(dir.path().to_path_buf()).unwrap().get(&id).is_none());
    }
}
