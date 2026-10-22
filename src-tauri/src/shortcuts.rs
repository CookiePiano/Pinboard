use crate::{settings::Settings, AppState};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

pub fn parse(value: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = value.parse().map_err(|_| tr!("올바른 단축키를 지정하세요.", "Choose a valid shortcut."; "请设置有效的快捷键。", "有効なショートカットを指定してください。"; "Choisissez un raccourci valide.", "Elige un atajo válido."))?;
    if !shortcut.mods.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER) {
        return Err(tr!("Ctrl, Alt 또는 Windows 키를 함께 누르세요.", "Include Ctrl, Alt, or the Windows key."; "请同时按住 Ctrl、Alt 或 Windows 键。", "Ctrl、Alt、Windows のいずれかも押してください。"; "Incluez la touche Ctrl, Alt ou Windows.", "Incluye la tecla Ctrl, Alt o Windows."));
    }
    Ok(shortcut)
}

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            let state = app.state::<AppState>();
            let enabled = *state.active_shortcut.lock().unwrap() == Some(*shortcut)
                && crate::model::now_ms() >= state.shortcut_recording_until.load(std::sync::atomic::Ordering::SeqCst);
            if enabled && event.state() == ShortcutState::Pressed {
                crate::dispatch_window_action(app, crate::WindowAction::NewNote);
            }
        })
        .build()
}

/// The Linux global-hotkey backend needs X11, even when no shortcut is registered.
pub fn available() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("DISPLAY").is_some_and(|value| !value.is_empty())
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

pub fn set_recording(state: &AppState, recording: bool) {
    use std::sync::atomic::Ordering;
    if recording {
        state.shortcut_recording_until.store(i64::MAX, Ordering::SeqCst);
    } else {
        // Native hotkey messages can arrive after the editor's keydown handler.
        let _ = state.shortcut_recording_until.compare_exchange(
            i64::MAX, crate::model::now_ms() + 300, Ordering::SeqCst, Ordering::SeqCst,
        );
    }
}

/// Register the replacement first. A conflict leaves the previous shortcut intact.
/// Must run off the UI thread; the plugin dispatches OS operations onto that thread.
pub fn apply(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    if !available() {
        return if settings.shortcuts_enabled {
            Err(tr!(
                "이 Linux 세션에서는 전역 단축키를 사용할 수 없습니다. X11/XWayland로 실행하세요.",
                "Global shortcuts require an X11/XWayland session on Linux.";
                "Linux 全局快捷键需要 X11/XWayland 会话。",
                "Linux のグローバルショートカットには X11/XWayland セッションが必要です。"; "Les raccourcis globaux nécessitent une session X11/XWayland sous Linux.", "Los atajos globales requieren una sesión X11/XWayland en Linux."
            ))
        } else {
            Ok(())
        };
    }
    let next = if settings.shortcuts_enabled { Some(parse(&settings.new_note_shortcut)?) } else { None };
    let state = app.state::<AppState>();
    let previous = *state.active_shortcut.lock().unwrap();
    if previous == next { return Ok(()); }
    let manager = app.global_shortcut();
    if let Some(next) = next {
        manager.register(next).map_err(|e| tr!(
            "단축키를 등록하지 못했습니다. 다른 키 조합을 지정하세요: {e}",
            "Could not register the shortcut. Choose another combination: {e}"; "无法注册快捷键。请选择其他组合键：{e}", "ショートカットを登録できませんでした。別の組み合わせを指定してください：{e}"; "Impossible d'enregistrer le raccourci. Choisissez une autre combinaison : {e}", "No se pudo registrar el atajo. Elige otra combinación: {e}"
        ))?;
    }
    if let Some(previous) = previous {
        if let Err(e) = manager.unregister(previous) {
            if let Some(next) = next { let _ = manager.unregister(next); }
            return Err(e.to_string());
        }
    }
    *state.active_shortcut.lock().unwrap() = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_captured_combinations_and_rejects_typing_keys() {
        assert_eq!(parse("Ctrl+Alt+N").unwrap(), parse("Ctrl+Alt+KeyN").unwrap());
        assert!(parse("Ctrl+Shift+Digit4").is_ok());
        assert!(parse("Super+F8").is_ok());
        for value in ["", "N", "Shift+N", "Ctrl", "Ctrl+Alt", "Ctrl+N+M"] {
            assert!(parse(value).is_err(), "{value}");
        }
    }
}
