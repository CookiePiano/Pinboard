//! Sync through a folder managed by Google Drive for Desktop.

use super::{RemoteMeta, Result, SyncBackend, SyncError};
use crate::model::Note;
use crate::store::{note_id_from_file, write_json};
use std::fs;
use std::path::{Path, PathBuf};

pub struct FolderBackend {
    dir: PathBuf,
}

impl FolderBackend {
    /// Notes live in `<root>/Pinboard/notes/<id>.json`.
    pub fn new(root: &Path) -> Self {
        FolderBackend { dir: root.join("Pinboard").join("notes") }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }
}

impl SyncBackend for FolderBackend {
    fn key(&self) -> String {
        format!("folder:{}", self.dir.display())
    }

    fn list(&mut self) -> Result<Vec<RemoteMeta>> {
        fs::create_dir_all(&self.dir)?;
        let mut metas = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            let Some(id) = note_id_from_file(&path) else { continue };
            // Abort this pass on unreadable/invalid notes. Treating them as absent could
            // incorrectly purge a previously synchronized local copy.
            let note = crate::validation::read_note(&path, &id)?;
            metas.push(RemoteMeta { id, hash: note.content_hash() });
        }
        Ok(metas)
    }

    fn get(&mut self, id: &str) -> Result<Note> {
        crate::validation::note_id(id)?;
        crate::validation::read_note(&self.path(id), id).map_err(SyncError::from)
    }

    fn put(&mut self, note: &Note) -> Result<()> {
        crate::validation::note(note)?;
        write_json(&self.path(&note.id), note)?;
        Ok(())
    }

    fn remove(&mut self, id: &str) -> Result<()> {
        crate::validation::note_id(id)?;
        match fs::remove_file(self.path(id)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Note;

    #[test]
    fn invalid_remote_file_fails_sync_without_purging_the_local_copy() {
        let remote = tempfile::tempdir().unwrap();
        let local = tempfile::tempdir().unwrap();
        let mut backend = FolderBackend::new(remote.path());
        let mut store = crate::store::Store::open(local.path().join("notes")).unwrap();
        let note = Note::new("test", "frost", 0.5);
        store.put(note.clone()).unwrap();
        backend.put(&note).unwrap();
        let base = std::collections::HashMap::from([(note.id.clone(), note.content_hash())]);
        let mut malicious = note.clone();
        malicious.id = "../../settings".into();
        write_json(&backend.path(&note.id), &malicious).unwrap();
        let mut outcome = crate::sync::run(&store.snapshot(), &mut backend, &base, crate::model::now_ms());
        assert!(outcome.error.is_some());
        assert!(outcome.writes.is_empty() && outcome.purges.is_empty());
        crate::sync::apply(&mut store, &mut outcome).unwrap();
        assert_eq!(store.get(&note.id), Some(&note));
        assert!(backend.get("../../settings").is_err());
        assert!(backend.put(&malicious).is_err());
        assert!(backend.remove("../../settings").is_err());
    }

    #[test]
    fn folder_backend_round_trip() {
        let root = tempfile::tempdir().unwrap();
        let mut b = FolderBackend::new(root.path());
        let n = Note::new("dev", "rose", 0.4);
        b.put(&n).unwrap();
        // Conflict copies made by the sync client and temp files are not notes.
        fs::write(b.dir().join(format!("{} (1).json", n.id)), "{}").unwrap();
        fs::write(b.dir().join(".partial.json.tmp"), "{").unwrap();

        let metas = b.list().unwrap();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].hash, n.content_hash());
        assert_eq!(b.get(&n.id).unwrap(), n);
        b.remove(&n.id).unwrap();
        b.remove(&n.id).unwrap();
        assert!(b.list().unwrap().is_empty());
    }
}
