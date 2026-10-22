//! Validation at every boundary where a note can enter storage or synchronization.
use crate::model::{Note, TINTS};
use serde::de::DeserializeOwned;
use std::{fs, io::{self, Read}, path::Path};

pub const MAX_NOTE_JSON: u64 = 16 * 1024 * 1024;
pub const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_FORMATS: usize = 20_000;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn canonical_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|value| value.to_string() == id)
}

pub fn note_id(id: &str) -> io::Result<()> {
    if !canonical_id(id) { return Err(invalid("Invalid note ID")); }
    Ok(())
}

pub fn note(note: &Note) -> io::Result<()> {
    note_id(&note.id)?;
    if note.conflict_of.as_deref().is_some_and(|id| !canonical_id(id))
        || note.device_id.len() > 256
        || !TINTS.contains(&note.tint.as_str())
        || !note.opacity.is_finite() || !(0.0..=1.0).contains(&note.opacity)
        || note.body.len() > MAX_BODY_BYTES || note.formats.len() > MAX_FORMATS
    {
        return Err(invalid("Invalid or oversized note"));
    }
    if let Some(font) = &note.font {
        font.clone().validate_for_note().map_err(|_| invalid("Invalid note font"))?;
    }
    let length = note.body.encode_utf16().count();
    let mut end = 0;
    for format in &note.formats {
        if format.from < end || format.from >= format.to || format.to > length
            || format.size.is_some_and(|size| !size.is_finite() || !(6.0..=96.0).contains(&size))
            || format.highlight.as_deref().is_some_and(|color| !crate::model::hex_color(color))
        {
            return Err(invalid("Invalid note formatting"));
        }
        end = format.to;
    }
    if let Some(background) = &note.background {
        if !background.opacity.is_finite() || !(0.0..=1.0).contains(&background.opacity)
            || background.image.len() > 8 * 1024 * 1024
            || !["data:image/png;base64,", "data:image/jpeg;base64,", "data:image/webp;base64,"]
                .iter().any(|prefix| background.image.starts_with(prefix))
        {
            return Err(invalid("Invalid note background"));
        }
    }
    Ok(())
}

pub fn matching_note(note: &Note, expected: &str) -> io::Result<()> {
    self::note(note)?;
    if note.id != expected { return Err(invalid("Note ID does not match its file")); }
    Ok(())
}

/// Bound the bytes actually read, not only the size advertised by a file or HTTP peer.
pub fn json<T: DeserializeOwned>(reader: impl Read, limit: u64) -> io::Result<T> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit { return Err(invalid("JSON exceeds the size limit")); }
    serde_json::from_slice(&bytes).map_err(|_| invalid("Invalid JSON"))
}

pub fn read_note(path: &Path, expected: &str) -> io::Result<Note> {
    note_id(expected)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_NOTE_JSON {
        return Err(invalid("Invalid note file"));
    }
    let note = json(fs::File::open(path)?, MAX_NOTE_JSON)?;
    matching_note(&note, expected)?;
    Ok(note)
}

/// Production windows may display bundled pages only. Web links use the scoped opener.
pub fn navigation_allowed(url: &url::Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() { return false; }
    let bundled = url.port().is_none() && matches!((url.scheme(), url.host_str()),
        ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost")));
    #[cfg(debug_assertions)]
    let development = url.scheme() == "http" && url.host_str() == Some("localhost") && url.port() == Some(1420);
    #[cfg(not(debug_assertions))]
    let development = false;
    (bundled || development) && matches!(url.path(), "" | "/" | "/index.html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_root_without_slash_supports_linux_startup() {
        // Tauri simplifies its default index.html page to this custom-scheme URL.
        let root = url::Url::parse("tauri://localhost").unwrap();
        assert_eq!(root.path(), "");
        assert!(navigation_allowed(&root));
    }

    #[test]
    fn navigation_rejects_external_origins_and_active_content() {
        for allowed in ["tauri://localhost/", "http://tauri.localhost/index.html", "https://tauri.localhost/"] {
            assert!(navigation_allowed(&url::Url::parse(allowed).unwrap()));
        }
        for denied in ["https://example.com/", "http://tauri.localhost.evil.test/", "http://tauri.localhost:8080/",
            "http://user@tauri.localhost/", "data:text/html,<script></script>", "javascript:alert(1)", "file:///C:/test.html"] {
            assert!(!navigation_allowed(&url::Url::parse(denied).unwrap()), "accepted {denied}");
        }
    }

    #[test]
    fn paths_aliases_and_cross_note_substitution_are_rejected() {
        for id in ["../settings", "..\\settings", "C:\\other", "\\\\server\\file", "note:stream", "not-an-id",
            "550E8400-E29B-41D4-A716-446655440000", "550e8400e29b41d4a716446655440000"] {
            assert!(note_id(id).is_err(), "accepted {id}");
        }
        let note = Note::new("test", "frost", 0.5);
        assert!(matching_note(&note, &note.id).is_ok());
        assert!(matching_note(&note, &uuid::Uuid::new_v4().to_string()).is_err());
    }

    #[test]
    fn rejects_untrusted_presentation_and_resource_exhaustion_inputs() {
        let valid = Note::new("test", "frost", 0.5);
        let mut bad = valid.clone();
        bad.body = "x".repeat(MAX_BODY_BYTES + 1);
        assert!(note(&bad).is_err());
        bad = valid.clone();
        bad.opacity = f32::NAN;
        assert!(note(&bad).is_err());
        bad = valid.clone();
        bad.background = Some(crate::model::NoteBackground { image: "data:image/svg+xml,<svg/>".into(), opacity: 0.5 });
        assert!(note(&bad).is_err());
        bad = valid;
        bad.formats.push(crate::model::TextFormat { from: 0, to: usize::MAX, bold: None, italic: None, size: None, highlight: None });
        assert!(note(&bad).is_err());
        assert!(json::<serde_json::Value>(std::io::Cursor::new(b"{}        "), 4).is_err());
        assert!(json::<serde_json::Value>(std::io::Cursor::new(b"{}"), 2).is_ok());
    }
}
