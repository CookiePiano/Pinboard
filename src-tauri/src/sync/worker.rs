//! Background thread that decides when to sync and talks to the configured backend.

use super::folder::FolderBackend;
use super::gdrive::DriveBackend;
use super::{secrets, SyncBackend, SyncError};
use crate::model::now_ms;
use crate::settings::SyncKind;
use crate::store::{note_id_from_file, read_json, write_json};
use crate::AppState;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const LOCAL_DEBOUNCE: Duration = Duration::from_millis(1500);
const REMOTE_DEBOUNCE: Duration = Duration::from_millis(800);
const RETRY_AFTER_ERROR: Duration = Duration::from_secs(30);

pub enum SyncMsg {
    LocalChanged,
    RemoteHint,
    Now,
    Reconfigure,
    Pause(Sender<()>),
    Resume { reset: bool },
    Shutdown(Sender<()>),
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Off,
    Idle,
    Syncing,
    Error,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub kind: SyncKind,
    pub phase: Phase,
    pub last_sync_at: Option<i64>,
    pub message: Option<String>,
}

impl Default for SyncStatus {
    fn default() -> Self {
        SyncStatus { kind: SyncKind::Off, phase: Phase::Off, last_sync_at: None, message: None }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct BaseFile {
    #[serde(default)]
    bases: HashMap<String, HashMap<String, String>>,
}

struct Active {
    backend: Box<dyn SyncBackend>,
    poll: Duration,
    _watcher: Option<RecommendedWatcher>,
}

pub fn spawn(app: AppHandle, rx: Receiver<SyncMsg>, tx: Sender<SyncMsg>) {
    std::thread::Builder::new()
        .name("pinboard-sync".into())
        .spawn(move || Worker::new(app, tx).run(rx))
        .expect("spawn sync thread");
}

struct Worker {
    app: AppHandle,
    tx: Sender<SyncMsg>,
    base_path: PathBuf,
    bases: BaseFile,
    active: Option<Active>,
}

impl Worker {
    fn new(app: AppHandle, tx: Sender<SyncMsg>) -> Self {
        let base_path = app.state::<AppState>().data_dir.join("sync-state.json");
        let bases = read_json(&base_path).unwrap_or_default();
        Worker { app, tx, base_path, bases, active: None }
    }

    fn run(mut self, rx: Receiver<SyncMsg>) {
        self.reconfigure();
        let mut due: Option<Instant> = Some(Instant::now());
        let mut paused = false;
        loop {
            let wait = match (due, &self.active) {
                (Some(d), _) => d.saturating_duration_since(Instant::now()),
                (None, Some(a)) => a.poll,
                (None, None) => Duration::from_secs(3600),
            };
            let soon = |d: Duration, cur: Option<Instant>| {
                let t = Instant::now() + d;
                Some(cur.map_or(t, |c| c.min(t)))
            };
            match rx.recv_timeout(wait) {
                Ok(SyncMsg::Pause(ack)) => {
                    paused = true;
                    self.active = None;
                    due = None;
                    let _ = ack.send(());
                }
                Ok(SyncMsg::Resume { reset }) => {
                    paused = false;
                    if reset {
                        self.bases = BaseFile::default();
                        let _ = write_json(&self.base_path, &self.bases);
                    }
                    self.reconfigure();
                    due = Some(Instant::now());
                }
                Ok(SyncMsg::LocalChanged | SyncMsg::RemoteHint | SyncMsg::Now | SyncMsg::Reconfigure) if paused => {}
                Ok(SyncMsg::LocalChanged) => due = soon(LOCAL_DEBOUNCE, due),
                Ok(SyncMsg::RemoteHint) => due = soon(REMOTE_DEBOUNCE, due),
                Ok(SyncMsg::Now) => due = Some(Instant::now()),
                Ok(SyncMsg::Reconfigure) => {
                    self.reconfigure();
                    due = Some(Instant::now());
                }
                Ok(SyncMsg::Shutdown(ack)) => {
                    if self.active.is_some() {
                        let _ = self.sync();
                    }
                    let _ = ack.send(());
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {
                    due = None;
                    if self.active.is_some() && self.sync().is_err() {
                        due = Some(Instant::now() + RETRY_AFTER_ERROR);
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn reconfigure(&mut self) {
        self.active = None;
        let kind = self.app.state::<AppState>().settings.lock().unwrap().sync;
        match self.build(kind) {
            Ok(active) => {
                self.active = active;
                let phase = if self.active.is_some() { Phase::Idle } else { Phase::Off };
                self.set_status(kind, phase, None, false);
            }
            Err(e) => self.set_status(kind, Phase::Error, Some(e.to_string()), false),
        }
    }

    fn build(&self, kind: SyncKind) -> Result<Option<Active>, SyncError> {
        let settings = self.app.state::<AppState>().settings.lock().unwrap().clone();
        match kind {
            SyncKind::Off => Ok(None),
            SyncKind::Folder => {
                let root = settings
                    .folder_root
                    .filter(|r| !r.is_empty())
                    .ok_or_else(|| SyncError::Config(tr!("동기화 폴더를 선택하세요.", "Choose a sync folder."; "请选择同步文件夹。", "同期フォルダーを選んでください。"; "Choisissez un dossier de synchronisation.", "Elige una carpeta de sincronización.")))?;
                let root = PathBuf::from(root);
                if !root.is_dir() {
                    let root = root.display();
                    return Err(SyncError::Config(tr!("{root}에 접근할 수 없습니다.", "Cannot access {root}."; "无法访问 {root}。", "{root} にアクセスできません。"; "Impossible d'accéder à {root}.", "No se puede acceder a {root}.")));
                }
                let backend = FolderBackend::new(&root);
                std::fs::create_dir_all(backend.dir())?;
                let watcher = self.watch(backend.dir());
                Ok(Some(Active { backend: Box::new(backend), poll: Duration::from_secs(30), _watcher: watcher }))
            }
            SyncKind::Gdrive => {
                let account = settings.gdrive_account.ok_or(SyncError::NotSignedIn)?;
                let secret = secrets::get(secrets::GDRIVE_CLIENT_SECRET).ok_or(SyncError::NotSignedIn)?;
                let refresh = secrets::get(secrets::GDRIVE_REFRESH_TOKEN).ok_or(SyncError::NotSignedIn)?;
                let backend = DriveBackend::new(account, settings.gdrive_client_id, secret, refresh);
                Ok(Some(Active { backend: Box::new(backend), poll: Duration::from_secs(60), _watcher: None }))
            }
        }
    }

    /// Change notifications are a hint only; polling still runs in case the drive does not send them.
    fn watch(&self, dir: &Path) -> Option<RecommendedWatcher> {
        let tx = self.tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                if ev.paths.iter().any(|p| note_id_from_file(p).is_some()) {
                    let _ = tx.send(SyncMsg::RemoteHint);
                }
            }
        })
        .inspect_err(|e| log::warn!("folder watcher unavailable: {e}"))
        .ok()?;
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .inspect_err(|e| log::warn!("cannot watch {}: {e}", dir.display()))
            .ok()?;
        Some(watcher)
    }

    fn sync(&mut self) -> Result<(), ()> {
        let state = self.app.state::<AppState>();
        let kind = state.settings.lock().unwrap().sync;
        self.set_status(kind, Phase::Syncing, None, false);
        let Some(active) = self.active.as_mut() else { return Ok(()) };

        let snapshot = state.store.lock().unwrap().snapshot();
        let key = active.backend.key();
        let empty_base = HashMap::new();
        let base = self.bases.bases.get(&key).unwrap_or(&empty_base);
        let mut out = super::run(&snapshot, active.backend.as_mut(), base, now_ms());
        let applied = {
            let mut store = state.store.lock().unwrap();
            super::apply(&mut store, &mut out)
        };
        self.bases.bases.insert(key, std::mem::take(&mut out.base));
        if let Err(e) = write_json(&self.base_path, &self.bases) {
            log::error!("cannot save sync state: {e}");
        }

        let error = match applied {
            Ok((changed, purged)) => {
                if !changed.is_empty() || !purged.is_empty() {
                    crate::windows::apply_remote_changes(&self.app, changed, purged);
                }
                out.error.map(|e| e.to_string())
            }
            Err(e) => Some(SyncError::from(e).to_string()),
        };
        log::info!(
            "sync done: pushed {}, pulled {}, conflicts {}{}",
            out.pushed,
            out.pulled,
            out.conflicts,
            error.as_deref().map(|e| format!(", error: {e}")).unwrap_or_default()
        );
        match error {
            None => {
                self.set_status(kind, Phase::Idle, None, true);
                Ok(())
            }
            Some(msg) => {
                self.set_status(kind, Phase::Error, Some(msg), false);
                Err(())
            }
        }
    }

    fn set_status(&self, kind: SyncKind, phase: Phase, message: Option<String>, synced: bool) {
        let state = self.app.state::<AppState>();
        let status = {
            let mut s = state.sync_status.lock().unwrap();
            s.kind = kind;
            s.phase = phase;
            s.message = message;
            if synced {
                s.last_sync_at = Some(now_ms());
            }
            s.clone()
        };
        let _ = self.app.emit("sync://status", status);
    }
}
