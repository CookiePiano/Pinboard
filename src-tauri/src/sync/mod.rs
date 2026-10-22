//! Three-way sync between the local store and one remote backend.
//!
//! Each backend keeps a base map of `note id -> content hash` from the last successful sync.
//! Comparing local and remote hashes against that base tells which side changed.

pub mod folder;
pub mod gdrive;
pub mod oauth;
pub mod secrets;
pub mod worker;

use crate::model::Note;
use crate::store::Store;
use std::collections::{BTreeSet, HashMap, HashSet};

/// Tombstones are kept this long so every device sees the deletion before it is purged.
pub const TOMBSTONE_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;

#[derive(Debug)]
pub enum SyncError {
    Io(std::io::Error),
    /// Already localized.
    Remote(String),
    NotSignedIn,
    /// Already localized.
    Config(String),
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Io(e) => f.write_str(&tr!("파일을 읽거나 쓰지 못했습니다: {e}", "Could not read or write a file: {e}"; "无法读取或写入文件：{e}", "ファイルの読み書きができませんでした：{e}"; "Impossible de lire ou d'écrire un fichier : {e}", "No se pudo leer o escribir un archivo: {e}")),
            SyncError::Remote(m) | SyncError::Config(m) => f.write_str(m),
            SyncError::NotSignedIn => f.write_str(&tr!("Google 계정을 연결하세요.", "Connect your Google account."; "请连接 Google 账号。", "Google アカウントを接続してください。"; "Connectez votre compte Google.", "Conecta tu cuenta de Google.")),
        }
    }
}

impl std::error::Error for SyncError {}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        SyncError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, SyncError>;

#[derive(Clone, Debug)]
pub struct RemoteMeta {
    pub id: String,
    pub hash: String,
}

pub trait SyncBackend: Send {
    /// Identifies the remote store; base hashes are kept separately per key.
    fn key(&self) -> String;
    fn list(&mut self) -> Result<Vec<RemoteMeta>>;
    fn get(&mut self, id: &str) -> Result<Note>;
    fn put(&mut self, note: &Note) -> Result<()>;
    /// Removes a note for good. Only used for purged tombstones.
    fn remove(&mut self, id: &str) -> Result<()>;
}

/// A note to save locally, applied only if the local copy still has the `expect` hash.
#[derive(Debug)]
pub struct LocalWrite {
    pub note: Note,
    pub expect: Option<String>,
    /// Base entry to restore when the write is skipped, so the next run sees a conflict.
    pub prev_base: Option<String>,
}

#[derive(Debug)]
pub struct LocalPurge {
    pub id: String,
    pub expect: String,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub writes: Vec<LocalWrite>,
    pub purges: Vec<LocalPurge>,
    pub base: HashMap<String, String>,
    pub pushed: usize,
    pub pulled: usize,
    pub conflicts: usize,
    pub error: Option<SyncError>,
}

/// Compares `local` with the backend and performs the remote side of the sync.
///
/// Local changes are returned in the outcome rather than applied, so the caller can apply them
/// under its own lock and skip any note the user edited while the sync was running.
pub fn run(
    local: &HashMap<String, crate::store::SharedNote>,
    backend: &mut dyn SyncBackend,
    base: &HashMap<String, String>,
    now: i64,
) -> Outcome {
    let mut out = Outcome { base: base.clone(), ..Default::default() };
    if let Err(e) = run_inner(local, backend, &mut out, now) {
        out.error = Some(e);
    }
    out
}

fn run_inner(
    local: &HashMap<String, crate::store::SharedNote>,
    backend: &mut dyn SyncBackend,
    out: &mut Outcome,
    now: i64,
) -> Result<()> {
    let mut remote = HashMap::new();
    for meta in backend.list()? {
        crate::validation::note_id(&meta.id)?;
        remote.insert(meta.id, meta.hash);
    }
    for (id, note) in local {
        crate::validation::matching_note(note, id)?;
    }
    for id in out.base.keys() { crate::validation::note_id(id)?; }
    let ids: BTreeSet<String> = local
        .keys()
        .chain(remote.keys())
        .chain(out.base.keys())
        .cloned()
        .collect();

    for id in ids {
        let base = out.base.get(&id).cloned();
        match (local.get(&id), remote.get(&id)) {
            (None, None) => {
                out.base.remove(&id);
            }
            (Some(note), None) => {
                let lh = note.content_hash();
                if base.as_deref() == Some(lh.as_str()) {
                    // Another device purged it and we have nothing new.
                    out.purges.push(LocalPurge { id: id.clone(), expect: lh });
                    out.base.remove(&id);
                } else {
                    backend.put(note)?;
                    out.pushed += 1;
                    out.base.insert(id, lh);
                }
            }
            (None, Some(rh)) => {
                if base.as_deref() == Some(rh.as_str()) {
                    // We purged it locally and the remote copy is the one we knew.
                    backend.remove(&id)?;
                    out.base.remove(&id);
                } else {
                    pull(backend, out, &id, None, base)?;
                }
            }
            (Some(note), Some(rh)) => {
                let lh = note.content_hash();
                if &lh == rh {
                    out.base.insert(id, lh);
                } else if base.as_deref() == Some(lh.as_str()) {
                    pull(backend, out, &id, Some(lh), base)?;
                } else if base.as_deref() == Some(rh.as_str()) {
                    backend.put(note)?;
                    out.pushed += 1;
                    out.base.insert(id, lh);
                } else {
                    resolve_conflict(backend, out, note, base, now)?;
                }
            }
        }
    }

    // Index pending replacements once instead of scanning every write for each tombstone.
    let pending: HashSet<&str> = out.writes.iter().map(|write| write.note.id.as_str()).collect();
    for note in local.values() {
        let Some(deleted_at) = note.deleted_at else { continue };
        if now.saturating_sub(deleted_at) <= TOMBSTONE_TTL_MS || pending.contains(note.id.as_str()) {
            continue;
        }
        let hash = note.content_hash();
        if out.base.get(&note.id) == Some(&hash) {
            backend.remove(&note.id)?;
            out.base.remove(&note.id);
            out.purges.push(LocalPurge { id: note.id.clone(), expect: hash });
        }
    }
    Ok(())
}

fn pull(
    backend: &mut dyn SyncBackend,
    out: &mut Outcome,
    id: &str,
    expect: Option<String>,
    prev_base: Option<String>,
) -> Result<()> {
    let note = backend.get(id)?;
    crate::validation::matching_note(&note, id)?;
    out.base.insert(id.to_string(), note.content_hash());
    out.writes.push(LocalWrite { note, expect, prev_base });
    out.pulled += 1;
    Ok(())
}

fn resolve_conflict(
    backend: &mut dyn SyncBackend,
    out: &mut Outcome,
    local: &Note,
    base: Option<String>,
    now: i64,
) -> Result<()> {
    let remote = backend.get(&local.id)?;
    crate::validation::matching_note(&remote, &local.id)?;
    // An edit beats a deletion; otherwise the most recent edit wins.
    let local_wins = match (local.is_deleted(), remote.is_deleted()) {
        (true, false) => false,
        (false, true) => true,
        _ => local.updated_at >= remote.updated_at,
    };
    let (winner, loser) = if local_wins { (local, &remote) } else { (&remote, local) };

    if !loser.is_deleted() && !winner.is_deleted() {
        // Keep the losing edit as its own note. It goes to the remote first so it survives
        // even if the run stops before the local side is applied.
        let mut copy = loser.clone();
        copy.id = uuid::Uuid::new_v4().to_string();
        copy.conflict_of = Some(local.id.clone());
        copy.updated_at = now;
        backend.put(&copy)?;
        out.base.insert(copy.id.clone(), copy.content_hash());
        out.writes.push(LocalWrite { note: copy, expect: None, prev_base: None });
    }

    let lh = local.content_hash();
    if local_wins {
        backend.put(local)?;
        out.pushed += 1;
        out.base.insert(local.id.clone(), lh);
    } else {
        out.base.insert(remote.id.clone(), remote.content_hash());
        out.writes.push(LocalWrite { note: remote, expect: Some(lh), prev_base: base });
        out.pulled += 1;
    }
    out.conflicts += 1;
    Ok(())
}

/// Applies the local side of an outcome. Returns the notes that changed on this device and the
/// ids that were purged, so the UI can update their windows.
pub fn apply(
    store: &mut Store,
    out: &mut Outcome,
) -> std::io::Result<(Vec<Note>, Vec<String>)> {
    let mut changed = Vec::new();
    let mut purged = Vec::new();
    for w in out.writes.drain(..) {
        let current = store.hash(&w.note.id);
        if current != w.expect {
            // Edited while syncing. Restore the old base so the next run compares both sides.
            match w.prev_base {
                Some(b) => out.base.insert(w.note.id.clone(), b),
                None => out.base.remove(&w.note.id),
            };
            continue;
        }
        store.put(w.note.clone())?;
        changed.push(w.note);
    }
    for p in out.purges.drain(..) {
        if store.hash(&p.id).as_deref() == Some(p.expect.as_str()) {
            store.purge(&p.id)?;
            purged.push(p.id);
        }
    }
    Ok((changed, purged))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn remote_payload_cannot_substitute_another_id_during_pull_or_conflict() {
        struct Untrusted { listed: String, payload: Note }
        impl SyncBackend for Untrusted {
            fn key(&self) -> String { "untrusted".into() }
            fn list(&mut self) -> Result<Vec<RemoteMeta>> {
                Ok(vec![RemoteMeta { id: self.listed.clone(), hash: self.payload.content_hash() }])
            }
            fn get(&mut self, _: &str) -> Result<Note> { Ok(self.payload.clone()) }
            fn put(&mut self, _: &Note) -> Result<()> { panic!("invalid data must not be uploaded") }
            fn remove(&mut self, _: &str) -> Result<()> { panic!("invalid data must not cause deletion") }
        }
        let local = Note::new("test", "frost", 0.5);
        let mut backend = Untrusted { listed: local.id.clone(), payload: Note::new("attacker", "rose", 0.5) };
        for snapshot in [HashMap::new(), HashMap::from([(local.id.clone(), local.clone().into())])] {
            let outcome = run(&snapshot, &mut backend, &HashMap::new(), crate::model::now_ms());
            assert!(outcome.error.is_some());
            assert!(outcome.writes.is_empty() && outcome.purges.is_empty() && outcome.base.is_empty());
        }
        backend.listed = "../../settings".into();
        let outcome = run(&HashMap::new(), &mut backend, &HashMap::new(), crate::model::now_ms());
        assert!(outcome.error.is_some());
        assert!(outcome.writes.is_empty());
    }

    /// In-memory backend for engine tests.
    #[derive(Default)]
    pub struct MemBackend {
        pub notes: HashMap<String, Note>,
    }

    impl SyncBackend for MemBackend {
        fn key(&self) -> String {
            "mem".into()
        }
        fn list(&mut self) -> Result<Vec<RemoteMeta>> {
            Ok(self
                .notes
                .values()
                .map(|n| RemoteMeta { id: n.id.clone(), hash: n.content_hash() })
                .collect())
        }
        fn get(&mut self, id: &str) -> Result<Note> {
            self.notes.get(id).cloned().ok_or_else(|| SyncError::Remote(format!("missing {id}")))
        }
        fn put(&mut self, note: &Note) -> Result<()> {
            self.notes.insert(note.id.clone(), note.clone());
            Ok(())
        }
        fn remove(&mut self, id: &str) -> Result<()> {
            self.notes.remove(id);
            Ok(())
        }
    }

    /// One device: a store plus its base map.
    struct Device {
        _dir: tempfile::TempDir,
        store: Store,
        base: HashMap<String, String>,
    }

    impl Device {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::open(dir.path().to_path_buf()).unwrap();
            Device { _dir: dir, store, base: HashMap::new() }
        }

        fn sync(&mut self, remote: &mut MemBackend, now: i64) -> Outcome {
            let mut out = run(&self.store.snapshot(), remote, &self.base, now);
            assert!(out.error.is_none(), "{:?}", out.error);
            apply(&mut self.store, &mut out).unwrap();
            self.base = out.base.clone();
            out
        }

        fn edit(&mut self, id: &str, body: &str, at: i64) {
            let mut n = self.store.get(id).unwrap().clone();
            n.body = body.into();
            n.updated_at = at;
            self.store.put(n).unwrap();
        }
    }

    fn note(body: &str, at: i64) -> Note {
        let mut n = Note::new("d", "frost", 0.5);
        n.body = body.into();
        n.created_at = at;
        n.updated_at = at;
        n
    }

    #[test]
    fn new_notes_flow_both_ways() {
        let mut remote = MemBackend::default();
        let (mut a, mut b) = (Device::new(), Device::new());
        let n = note("from a", 1);
        a.store.put(n.clone()).unwrap();

        assert_eq!(a.sync(&mut remote, 10).pushed, 1);
        assert_eq!(b.sync(&mut remote, 11).pulled, 1);
        assert_eq!(b.store.get(&n.id).unwrap().body, "from a");

        // A second run on both sides is a no-op.
        let out = a.sync(&mut remote, 12);
        assert_eq!((out.pushed, out.pulled), (0, 0));
    }

    #[test]
    fn one_sided_edit_propagates() {
        let mut remote = MemBackend::default();
        let (mut a, mut b) = (Device::new(), Device::new());
        let n = note("v1", 1);
        a.store.put(n.clone()).unwrap();
        a.sync(&mut remote, 2);
        b.sync(&mut remote, 3);

        b.edit(&n.id, "v2", 4);
        assert_eq!(b.sync(&mut remote, 5).pushed, 1);
        assert_eq!(a.sync(&mut remote, 6).pulled, 1);
        assert_eq!(a.store.get(&n.id).unwrap().body, "v2");
    }

    #[test]
    fn concurrent_edits_keep_both() {
        let mut remote = MemBackend::default();
        let (mut a, mut b) = (Device::new(), Device::new());
        let n = note("v1", 1);
        a.store.put(n.clone()).unwrap();
        a.sync(&mut remote, 2);
        b.sync(&mut remote, 3);

        a.edit(&n.id, "a wins", 20);
        b.edit(&n.id, "b loses", 10);
        a.sync(&mut remote, 21);
        let out = b.sync(&mut remote, 22);
        assert_eq!(out.conflicts, 1);

        // The newer edit keeps the id; the older one survives as a conflict copy.
        assert_eq!(b.store.get(&n.id).unwrap().body, "a wins");
        let copy = b.store.live().find(|x| x.conflict_of.as_deref() == Some(&n.id)).unwrap();
        assert_eq!(copy.body, "b loses");

        a.sync(&mut remote, 23);
        assert_eq!(a.store.live().count(), 2);
        assert_eq!(remote.notes.len(), 2);
    }

    #[test]
    fn edit_beats_delete() {
        let mut remote = MemBackend::default();
        let (mut a, mut b) = (Device::new(), Device::new());
        let n = note("v1", 1);
        a.store.put(n.clone()).unwrap();
        a.sync(&mut remote, 2);
        b.sync(&mut remote, 3);

        let mut gone = a.store.get(&n.id).unwrap().clone();
        gone.deleted_at = Some(30);
        gone.updated_at = 30;
        a.store.put(gone).unwrap();
        b.edit(&n.id, "still needed", 10);
        a.sync(&mut remote, 31);
        b.sync(&mut remote, 32);
        a.sync(&mut remote, 33);

        let kept = a.store.get(&n.id).unwrap();
        assert!(!kept.is_deleted());
        assert_eq!(kept.body, "still needed");
    }

    #[test]
    fn deletion_propagates_then_purges() {
        let mut remote = MemBackend::default();
        let (mut a, mut b) = (Device::new(), Device::new());
        let n = note("bye", 1);
        a.store.put(n.clone()).unwrap();
        a.sync(&mut remote, 2);
        b.sync(&mut remote, 3);

        let mut gone = a.store.get(&n.id).unwrap().clone();
        gone.deleted_at = Some(100);
        gone.updated_at = 100;
        a.store.put(gone).unwrap();
        a.sync(&mut remote, 101);
        b.sync(&mut remote, 102);
        assert!(b.store.get(&n.id).unwrap().is_deleted());

        // After the TTL the first device to sync removes it everywhere.
        let later = 100 + TOMBSTONE_TTL_MS + 1;
        a.sync(&mut remote, later);
        assert!(a.store.get(&n.id).is_none());
        assert!(remote.notes.is_empty());
        b.sync(&mut remote, later + 1);
        assert!(b.store.get(&n.id).is_none());
        assert!(b.base.is_empty());
    }

    #[test]
    fn tombstones_expire_only_after_the_ttl() {
        let mut remote = MemBackend::default();
        let mut device = Device::new();
        let mut gone = note("deleted", 1);
        gone.deleted_at = Some(100);
        gone.updated_at = 100;
        device.store.put(gone.clone()).unwrap();
        for now in [99, 100, 100 + TOMBSTONE_TTL_MS] {
            device.sync(&mut remote, now);
            assert_eq!(device.store.get(&gone.id), Some(&gone));
            assert_eq!(remote.notes.get(&gone.id), Some(&gone));
        }
        device.sync(&mut remote, 101 + TOMBSTONE_TTL_MS);
        assert!(device.store.get(&gone.id).is_none());
        assert!(!remote.notes.contains_key(&gone.id));
        assert!(device.base.is_empty());
    }

    #[test]
    fn remote_edit_replaces_an_expired_tombstone_without_being_purged() {
        let mut remote = MemBackend::default();
        let mut device = Device::new();
        let mut gone = note("original", 1);
        gone.deleted_at = Some(100);
        gone.updated_at = 100;
        device.store.put(gone.clone()).unwrap();
        device.sync(&mut remote, 101);
        let live = remote.notes.get_mut(&gone.id).unwrap();
        live.deleted_at = None;
        live.body = "keep this remote edit".into();
        live.updated_at = 200;
        let out = device.sync(&mut remote, 101 + TOMBSTONE_TTL_MS);
        assert_eq!(out.pulled, 1);
        let restored = device.store.get(&gone.id).unwrap();
        assert!(!restored.is_deleted());
        assert_eq!(restored.body, "keep this remote edit");
        assert_eq!(remote.notes.get(&gone.id), Some(restored));
        assert_eq!(device.base.get(&gone.id), Some(&restored.content_hash()));
    }

    #[test]
    fn edit_during_sync_is_not_overwritten() {
        let mut remote = MemBackend::default();
        let (mut a, mut b) = (Device::new(), Device::new());
        let n = note("v1", 1);
        a.store.put(n.clone()).unwrap();
        a.sync(&mut remote, 2);
        b.sync(&mut remote, 3);
        a.edit(&n.id, "remote change", 10);
        a.sync(&mut remote, 11);

        // B starts a sync, then the user types before it is applied.
        let mut out = run(&b.store.snapshot(), &mut remote, &b.base, 12);
        b.edit(&n.id, "typed meanwhile", 12);
        apply(&mut b.store, &mut out).unwrap();
        b.base = out.base;
        assert_eq!(b.store.get(&n.id).unwrap().body, "typed meanwhile");

        // The next run treats it as a conflict and keeps both texts.
        let out = b.sync(&mut remote, 13);
        assert_eq!(out.conflicts, 1);
        let bodies: BTreeSet<String> = b.store.live().map(|x| x.body.clone()).collect();
        assert!(bodies.contains("typed meanwhile") && bodies.contains("remote change"));
    }
}
