//! Isolated application state for tests that do not need a native window.
use crate::{AppState, settings::Settings, store::{LocalState, Store}};
use std::path::Path;
use std::sync::{Mutex, RwLock, atomic::{AtomicBool, AtomicI64}};

pub(crate) fn app_state(dir: &Path) -> AppState {
    AppState {
        data_gate: RwLock::new(()),
        transfer_lock: Mutex::new(()),
        restoring: AtomicBool::new(false),
        data_dir: dir.into(),
        device_id: "this-pc".into(),
        store: Mutex::new(Store::open(dir.join("notes")).unwrap()),
        local: Mutex::new(LocalState::default()),
        local_dirty: AtomicBool::new(false),
        settings: Mutex::new(Settings::default()),
        settings_note: Mutex::new(None),
        sync_status: Mutex::new(Default::default()),
        sync_tx: std::sync::mpsc::channel().0,
        focus_on_ready: Mutex::new(Default::default()),
        active_shortcut: Mutex::new(None),
        shortcut_recording_until: AtomicI64::new(0),
    }
}
