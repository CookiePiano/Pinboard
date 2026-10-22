//! Flush and temporarily freeze live editors before replacing data or leaving them.
//! Callers serialize operations with AppState::transfer_lock and run off the UI thread.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::mpsc, time::Duration};
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewWindow};

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Ready { request_id: String, label: String, error: Option<String> }

#[tauri::command]
pub fn editor_ready(app: AppHandle, window: WebviewWindow, request_id: String, error: Option<String>) {
    if window.label().starts_with("note-") || window.label() == crate::windows::SETTINGS_LABEL {
        let _ = app.emit("archive://ready", Ready { request_id, label: window.label().into(), error });
    }
}

pub(crate) struct FrozenEditors { app: AppHandle, release: bool }
impl FrozenEditors {
    /// An accepted exit must not reopen a gap for editing before the process stops.
    pub fn until_exit(mut self) { self.release = false; }
}
impl Drop for FrozenEditors {
    fn drop(&mut self) { if self.release { let _ = self.app.emit("archive://release", ()); } }
}

pub(crate) fn freeze(app: &AppHandle) -> Result<FrozenEditors, String> {
    let frozen = FrozenEditors { app: app.clone(), release: true };
    let request = uuid::Uuid::new_v4().to_string();
    let mut pending: HashSet<String> = app.webview_windows().keys().filter(|l| l.starts_with("note-") || l.as_str() == crate::windows::SETTINGS_LABEL).cloned().collect();
    let (tx, rx) = mpsc::channel();
    let expected = request.clone();
    let listener = app.listen("archive://ready", move |e| {
        if let Ok(ready) = serde_json::from_str::<Ready>(e.payload()) {
            if ready.request_id == expected { let _ = tx.send(ready); }
        }
    });
    let result = (|| {
        for label in &pending { app.emit_to(label, "archive://flush", &request).map_err(|e| e.to_string())?; }
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        while !pending.is_empty() {
            let ready = rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                .map_err(|_| tr!("메모 저장을 기다리는 중입니다. 잠시 후 다시 시도하세요.", "Waiting for notes to save. Try again shortly."; "正在等待便签保存。请稍后重试。", "メモの保存を待っています。しばらくしてから再試行してください。"; "Enregistrement des notes en cours. Réessayez dans un instant.", "Esperando a que se guarden las notas. Inténtalo de nuevo en un momento."))?;
            if pending.remove(&ready.label) {
                if let Some(error) = ready.error { return Err(error); }
            }
        }
        Ok(())
    })();
    app.unlisten(listener);
    result?;
    Ok(frozen)
}
