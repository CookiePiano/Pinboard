use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Feed serialized JSON directly into the digest without allocating an image-sized buffer.
struct HashWriter<'a>(&'a mut Sha256);

impl std::io::Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub const TINTS: [&str; 6] = ["frost", "sage", "lilac", "amber", "rose", "ink"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextFormat {
    pub from: usize,
    pub to: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Absolute size in typographic points (1pt = 4/3 CSS px).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteBackground {
    pub image: String,
    pub opacity: f32,
}

pub fn hex_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|c| c.is_ascii_hexdigit())
}

/// A note as it is stored locally and synced to every backend.
///
/// Window placement and pin mode are per-device and live in `LocalState`, not here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub body: String,
    pub tint: String,
    /// Strength of the tint layer over the blurred backdrop, 0.0..=1.0.
    pub opacity: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formats: Vec<TextFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<NoteBackground>,
    /// Per-note override; absent notes follow this PC's global font preferences.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<crate::fonts::NoteFont>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub deleted_at: Option<i64>,
    #[serde(default)]
    pub device_id: String,
    /// Set on a copy made when two devices changed the same note.
    #[serde(default)]
    pub conflict_of: Option<String>,
}

impl Note {
    pub fn new(device_id: &str, tint: &str, opacity: f32) -> Self {
        let now = now_ms();
        Note {
            id: uuid::Uuid::new_v4().to_string(),
            body: String::new(),
            tint: tint.to_string(),
            opacity,
            formats: Vec::new(),
            background: None,
            font: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            device_id: device_id.to_string(),
            conflict_of: None,
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn set_font(&mut self, mut font: crate::fonts::NoteFont, reset_sizes: bool) -> Result<(), String> {
        font.validate_for_note()?;
        self.font = Some(font);
        if reset_sizes {
            for format in &mut self.formats { format.size = None; }
            self.formats.retain(|f| f.bold == Some(true) || f.italic == Some(true) || f.highlight.is_some());
        }
        Ok(())
    }

    /// Hash of the content that matters for sync. `updated_at` and `device_id` are left out
    /// so two devices that arrive at the same content agree without a conflict.
    pub fn content_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.id.as_bytes());
        h.update([0]);
        h.update(crate::welcome::sync_body(self).as_bytes());
        h.update([0]);
        h.update(self.tint.as_bytes());
        h.update([0]);
        h.update(format!("{:.3}", self.opacity).as_bytes());
        h.update([0]);
        h.update(self.created_at.to_le_bytes());
        h.update(self.deleted_at.unwrap_or(-1).to_le_bytes());
        h.update(self.conflict_of.as_deref().unwrap_or("").as_bytes());
        // Keep legacy hashes unchanged when the new optional fields are absent.
        if !self.formats.is_empty() {
            h.update(b"\0formats\0");
            serde_json::to_writer(HashWriter(&mut h), &self.formats).expect("serializable formats");
        }
        if let Some(background) = &self.background {
            h.update(b"\0background\0");
            serde_json::to_writer(HashWriter(&mut h), background).expect("serializable background");
        }
        if let Some(font) = &self.font {
            h.update(b"\0font\0");
            serde_json::to_writer(HashWriter(&mut h), font).expect("serializable font");
        }
        hex::encode(&h.finalize()[..16])
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PinMode {
    #[default]
    Normal,
    Top,
    Desktop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streamed_json_preserves_existing_sync_hashes() {
        // Reproduce the pre-streaming wire bytes as an independent compatibility oracle.
        fn legacy_hash(note: &Note) -> String {
            let mut bytes = Vec::new();
            for field in [note.id.as_str(), crate::welcome::sync_body(note), note.tint.as_str(), &format!("{:.3}", note.opacity)] {
                bytes.extend_from_slice(field.as_bytes());
                bytes.push(0);
            }
            bytes.extend_from_slice(&note.created_at.to_le_bytes());
            bytes.extend_from_slice(&note.deleted_at.unwrap_or(-1).to_le_bytes());
            bytes.extend_from_slice(note.conflict_of.as_deref().unwrap_or("").as_bytes());
            if !note.formats.is_empty() {
                bytes.extend_from_slice(b"\0formats\0");
                bytes.extend(serde_json::to_vec(&note.formats).unwrap());
            }
            if let Some(background) = &note.background {
                bytes.extend_from_slice(b"\0background\0");
                bytes.extend(serde_json::to_vec(background).unwrap());
            }
            if let Some(font) = &note.font {
                bytes.extend_from_slice(b"\0font\0");
                bytes.extend(serde_json::to_vec(font).unwrap());
            }
            hex::encode(&Sha256::digest(bytes)[..16])
        }

        let mut note = Note::new("test", "frost", 0.55);
        note.body = "Unicode: 한😀글\nQuotes: \" and backslash: \\".into();
        assert_eq!(note.content_hash(), legacy_hash(&note));
        note.formats.push(TextFormat { from: 0, to: 4, bold: Some(true), italic: Some(false), size: Some(12.5), highlight: Some("#ffe066".into()) });
        assert_eq!(note.content_hash(), legacy_hash(&note));
        note.background = Some(NoteBackground { image: format!("data:image/png;base64,{}", "dGVzdA==".repeat(131072)), opacity: 0.3 });
        assert_eq!(note.content_hash(), legacy_hash(&note));
        note.font = Some(crate::fonts::NoteFont { family: Some("Noto Sans KR".into()), size: 16.0, weight: 600 });
        note.deleted_at = Some(123);
        note.conflict_of = Some("original".into());
        assert_eq!(note.content_hash(), legacy_hash(&note));
    }

    #[test]
    fn whole_note_font_survives_storage_and_resets_only_sizes() {
        let mut note = Note::new("test", "frost", 0.55);
        note.body = "한😀글".into();
        note.formats = vec![
            TextFormat { from: 0, to: 1, bold: Some(true), italic: Some(true), size: Some(20.0), highlight: Some("#ffe066".into()) },
            TextFormat { from: 1, to: 4, bold: None, italic: None, size: Some(24.0), highlight: None },
        ];
        let before = note.content_hash();
        let font = crate::fonts::NoteFont { family: Some("Noto Sans KR".into()), size: 16.0, weight: 400 };
        note.set_font(font.clone(), false).unwrap();
        assert_eq!(note.formats[0].size, Some(20.0));
        assert_ne!(before, note.content_hash());
        note.set_font(font.clone(), true).unwrap();
        assert_eq!(note.body, "한😀글");
        assert_eq!(note.formats.len(), 1);
        assert_eq!(note.formats[0].size, None);
        assert_eq!(note.formats[0].bold, Some(true));
        assert_eq!(note.formats[0].italic, Some(true));
        assert_eq!(note.formats[0].highlight.as_deref(), Some("#ffe066"));
        let saved: Note = serde_json::from_slice(&serde_json::to_vec(&note).unwrap()).unwrap();
        assert_eq!(saved.font, Some(font));
        assert_eq!(saved, note);
    }

    #[test]
    fn invalid_note_font_leaves_content_unchanged() {
        let mut note = Note::new("test", "frost", 0.55);
        let before = note.clone();
        for size in [f32::NAN, f32::INFINITY, 7.0, 129.0] {
            assert!(note.set_font(crate::fonts::NoteFont { size, ..Default::default() }, true).is_err());
            assert_eq!(note, before);
        }
        for size in [8.0, 128.0] {
            assert!(note.set_font(crate::fonts::NoteFont { size, ..Default::default() }, true).is_ok());
        }
    }

    #[test]
    fn legacy_notes_and_new_appearance_round_trip() {
        let original = Note::new("test", "frost", 0.55);
        let json = serde_json::to_value(&original).unwrap();
        assert!(json.get("formats").is_none());
        assert!(json.get("background").is_none());
        assert!(json.get("font").is_none());
        let mut note: Note = serde_json::from_value(json).unwrap();
        assert_eq!(note, original);
        note.body = "한😀글".into();
        let plain_hash = note.content_hash();
        note.formats.push(TextFormat { from: 1, to: 3, bold: Some(true), italic: None, size: Some(12.0), highlight: Some("#ffe066".into()) });
        assert_ne!(note.content_hash(), plain_hash);
        let formatted_hash = note.content_hash();
        note.background = Some(NoteBackground { image: "data:image/webp;base64,dGVzdA==".into(), opacity: 0.3 });
        assert_ne!(note.content_hash(), formatted_hash);
        assert_eq!(serde_json::from_str::<Note>(&serde_json::to_string(&note).unwrap()).unwrap(), note);
    }

    #[test]
    fn hash_ignores_timestamps_and_device() {
        let mut a = Note::new("dev-a", "frost", 0.6);
        let mut b = a.clone();
        b.updated_at += 1000;
        b.device_id = "dev-b".into();
        assert_eq!(a.content_hash(), b.content_hash());
        a.body = "changed".into();
        assert_ne!(a.content_hash(), b.content_hash());
    }
}
