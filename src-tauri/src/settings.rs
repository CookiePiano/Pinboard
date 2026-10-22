use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Theme { #[default] System, Light, Dark }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SelectionTools {
    pub size_step: f32,
    pub highlight_color: String,
}

impl Default for SelectionTools {
    fn default() -> Self {
        Self { size_step: 1.0, highlight_color: "#ffe066".into() }
    }
}

impl SelectionTools {
    pub fn validate(&self) -> Result<(), String> {
        if !self.size_step.is_finite() || !(0.5..=12.0).contains(&self.size_step) {
            return Err(tr!("크기 조절 간격은 0.5~12pt로 입력하세요.", "Size step must be between 0.5 and 12pt."; "字号调整步长须在 0.5–12pt 之间。", "サイズの変更幅は 0.5～12pt で指定してください。"; "L'écart de taille doit être compris entre 0,5 et 12 pt.", "El paso de tamaño debe estar entre 0,5 y 12 pt.").into());
        }
        if !crate::model::hex_color(&self.highlight_color) {
            return Err(tr!("올바른 형광펜 색상을 선택하세요.", "Choose a valid highlight color."; "请选择有效的高亮颜色。", "有効な蛍光ペンの色を選んでください。"; "Choisissez une couleur de surlignage valide.", "Elige un color de resaltado válido.").into());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SyncKind {
    #[default]
    Off,
    Folder,
    Gdrive,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub language: crate::i18n::Lang,
    pub theme: Theme,
    pub selection_tools: SelectionTools,
    pub note_font: crate::fonts::NoteFont,
    pub shortcuts_enabled: bool,
    pub new_note_shortcut: String,
    #[serde(deserialize_with = "storage_sync_kind")]
    pub sync: SyncKind,
    /// Folder that holds the shared `Pinboard` directory, e.g. a Google Drive for Desktop folder.
    pub folder_root: Option<String>,
    pub gdrive_client_id: String,
    /// Legacy Drive account metadata, retained without exposing OAuth inputs in settings.
    pub gdrive_account: Option<String>,
}

fn storage_sync_kind<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<SyncKind, D::Error> {
    // Retain existing notes and credentials, but never resume the retired OAuth mode.
    Ok(match SyncKind::deserialize(deserializer)? {
        SyncKind::Gdrive => SyncKind::Off,
        kind => kind,
    })
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: crate::i18n::Lang::Ko,
            theme: Theme::System,
            selection_tools: SelectionTools::default(),
            note_font: crate::fonts::NoteFont::default(),
            shortcuts_enabled: false,
            new_note_shortcut: "Ctrl+Alt+N".into(),
            sync: SyncKind::Off,
            folder_root: None,
            gdrive_client_id: String::new(),
            gdrive_account: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;

    #[test]
    fn appearance_migrates_and_validates() {
        let mut settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.theme, Theme::System);
        assert_eq!(settings.selection_tools.size_step, 1.0);
        let legacy: SelectionTools = serde_json::from_str(r##"{"enabled":false,"bold":false,"sizeStep":2.0,"highlightColor":"#a8e6cf"}"##).unwrap();
        assert_eq!(legacy.size_step, 2.0);
        assert_eq!(legacy.highlight_color, "#a8e6cf");
        assert!(serde_json::to_value(&legacy).unwrap().get("enabled").is_none());
        assert!(settings.selection_tools.validate().is_ok());
        settings.selection_tools.size_step = f32::NAN;
        assert!(settings.selection_tools.validate().is_err());
        settings.selection_tools.size_step = 1.0;
        settings.selection_tools.highlight_color = "red;bad".into();
        assert!(settings.selection_tools.validate().is_err());
    }

    #[test]
    fn legacy_settings_default_to_korean() {
        let settings: Settings = serde_json::from_str(r#"{"defaultOpacity":0.2,"sync":"folder","folderRoot":"test"}"#).unwrap();
        assert_eq!(settings.language, Lang::Ko);
        assert!(!settings.shortcuts_enabled);
        assert_eq!(settings.new_note_shortcut, "Ctrl+Alt+N");
        assert_eq!(settings.sync, SyncKind::Folder);
    }

    #[test]
    fn added_language_settings_round_trip() {
        for (language, code) in [(Lang::Zh, "zh"), (Lang::Ja, "ja"), (Lang::Fr, "fr"), (Lang::Es, "es")] {
            let settings = Settings { language, ..Settings::default() };
            let json = serde_json::to_value(&settings).unwrap();
            assert_eq!(json["language"], code);
            assert_eq!(serde_json::from_value::<Settings>(json).unwrap(), settings);
        }
    }

    #[test]
    fn english_settings_round_trip() {
        let settings = Settings { language: Lang::En, ..Settings::default() };
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains(r#""language":"en""#));
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), settings);
    }

    #[test]
    fn legacy_oauth_mode_becomes_local_without_losing_other_settings() {
        let settings: Settings = serde_json::from_str(
            r#"{"language":"en","sync":"gdrive","folderRoot":"shared-folder","gdriveClientId":"legacy-id","gdriveAccount":"legacy@example.test","shortcutsEnabled":true,"newNoteShortcut":"Ctrl+Alt+P"}"#,
        ).unwrap();
        assert_eq!(settings.sync, SyncKind::Off);
        assert_eq!(settings.language, Lang::En);
        assert_eq!(settings.folder_root.as_deref(), Some("shared-folder"));
        assert_eq!(settings.gdrive_client_id, "legacy-id");
        assert_eq!(settings.gdrive_account.as_deref(), Some("legacy@example.test"));
        assert!(settings.shortcuts_enabled);
        assert_eq!(settings.new_note_shortcut, "Ctrl+Alt+P");
        let saved = serde_json::to_value(&settings).unwrap();
        assert_eq!(saved["sync"], "off");
    }

    #[test]
    fn missing_storage_mode_defaults_to_this_pc() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.sync, SyncKind::Off);
        assert_eq!(settings.folder_root, None);
    }

    #[test]
    fn font_preferences_preserve_legacy_settings_and_round_trip() {
        let legacy: Settings = serde_json::from_str(r#"{"language":"en","sync":"folder","folderRoot":"shared"}"#).unwrap();
        assert_eq!(legacy.note_font, crate::fonts::NoteFont::default());
        let mut settings = legacy;
        settings.note_font = crate::fonts::NoteFont { family: Some("Noto Sans KR".into()), size: 18.5, weight: 600 };
        let saved = serde_json::to_value(&settings).unwrap();
        assert_eq!(saved["noteFont"]["family"], "Noto Sans KR");
        assert_eq!(serde_json::from_value::<Settings>(saved).unwrap(), settings);
    }
}
