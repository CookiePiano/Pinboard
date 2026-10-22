//! Installed font families from the OS, without reading or copying font files.

use crate::i18n::Lang;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NoteFont {
    /// None uses the bundled font and the existing system fallbacks.
    pub family: Option<String>,
    pub size: f32,
    pub weight: u16,
}

impl Default for NoteFont {
    fn default() -> Self {
        Self {
            family: None,
            size: 14.5,
            weight: 400,
        }
    }
}

impl NoteFont {
    /// Note controls display points; storage retains the existing CSS pixel unit.
    pub fn validate_for_note(&mut self) -> Result<(), String> {
        let size = self.size;
        let mut family_and_weight = self.clone();
        family_and_weight.size = 14.5;
        family_and_weight.validate()?;
        if !size.is_finite() || !(8.0..=128.0).contains(&size) {
            return Err(tr!("글자 크기는 6~96pt로 입력하세요.", "Font size must be between 6 and 96pt."; "字号须在 6–96pt 之间。", "文字サイズは 6～96pt で指定してください。"; "La taille de police doit être comprise entre 6 et 96 pt.", "El tamaño de fuente debe estar entre 6 y 96 pt."));
        }
        self.family = family_and_weight.family;
        Ok(())
    }

    pub fn validate(&mut self) -> Result<(), String> {
        self.family = self
            .family
            .take()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty());
        if self
            .family
            .as_ref()
            .is_some_and(|name| name.chars().count() > 256 || name.chars().any(char::is_control))
            || !self.size.is_finite()
            || !(10.0..=36.0).contains(&self.size)
            || !matches!(self.weight, 400 | 500 | 600 | 700)
        {
            return Err(tr!("글꼴 설정을 확인하세요. 크기는 10~36px, 굵기는 400~700을 사용할 수 있습니다.", "Check the font settings. Use a size of 10–36px and a weight of 400, 500, 600, or 700."; "请检查字体设置。字号须为 10–36px，字重须为 400、500、600 或 700。", "フォント設定を確認してください。サイズは 10～36px、太さは 400、500、600、700 を指定できます。"; "Vérifiez les paramètres de police. Utilisez une taille de 10 à 36 px et une graisse de 400, 500, 600 ou 700.", "Revisa la configuración de fuente. Usa un tamaño de 10 a 36 px y un grosor de 400, 500, 600 o 700."));
        }
        Ok(())
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FontFamily {
    pub family: String,
    pub label: String,
}

pub fn installed(language: Lang) -> Result<Vec<FontFamily>, String> {
    installed_with_refresh(language, false)
}

pub fn installed_with_refresh(language: Lang, force: bool) -> Result<Vec<FontFamily>, String> {
    // Hold the lock through the scan so concurrent windows share one OS enumeration.
    type Entries = Vec<(Lang, std::time::Instant, Vec<FontFamily>)>;
    static CACHE: std::sync::Mutex<Entries> = std::sync::Mutex::new(Vec::new());
    let mut cache = CACHE.lock().unwrap();
    if !force {
        if let Some((_, _, fonts)) = cache.iter().find(|(lang, at, _)| *lang == language && at.elapsed() < std::time::Duration::from_secs(300)) {
            return Ok(fonts.clone());
        }
    }
    let fonts = scan_installed(language)?;
    cache.retain(|(lang, _, _)| *lang != language);
    cache.push((language, std::time::Instant::now(), fonts.clone()));
    Ok(fonts)
}

fn scan_installed(language: Lang) -> Result<Vec<FontFamily>, String> {
    let mut fonts = scan(language)?;
    fonts.retain(|font| !font.family.is_empty() && !font.family.starts_with('@'));
    fonts.sort_by_cached_key(|font| font.family.to_lowercase());
    fonts.dedup_by(|a, b| a.family.eq_ignore_ascii_case(&b.family));
    fonts.sort_by_cached_key(|font| font.label.to_lowercase());
    if fonts.is_empty() {
        return Err(tr!(
            "설치된 글꼴을 찾지 못했습니다. 다시 검색하세요.",
            "No installed fonts found. Scan again."; "未找到已安装的字体。请重新扫描。", "インストール済みフォントが見つかりませんでした。再検索してください。"; "Aucune police installée trouvée. Relancez la recherche.", "No se encontraron fuentes instaladas. Vuelve a buscar."
        ));
    }
    Ok(fonts)
}

#[cfg(windows)]
fn scan(language: Lang) -> Result<Vec<FontFamily>, String> {
    use windows::core::{w, BOOL, PCWSTR};
    use windows::Win32::Graphics::DirectWrite::{
        DWriteCreateFactory, IDWriteFactory, IDWriteLocalizedStrings, DWRITE_FACTORY_TYPE_SHARED,
    };

    unsafe fn name(
        strings: &IDWriteLocalizedStrings,
        locale: PCWSTR,
    ) -> windows::core::Result<String> {
        let mut index = 0;
        let mut found = BOOL::default();
        strings.FindLocaleName(locale, &mut index, &mut found)?;
        if !found.as_bool() {
            index = 0;
        }
        let len = strings.GetStringLength(index)?;
        let mut buffer = vec![0u16; len as usize + 1];
        strings.GetString(index, &mut buffer)?;
        Ok(String::from_utf16_lossy(&buffer[..len as usize]))
    }

    // DirectWrite owns the buffers and COM references; only copied names cross the thread boundary.
    unsafe {
        let factory: IDWriteFactory =
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).map_err(|e| e.to_string())?;
        let mut collection = None;
        factory
            .GetSystemFontCollection(&mut collection, true)
            .map_err(|e| e.to_string())?;
        let collection = collection.ok_or_else(|| {
            tr!(
                "글꼴 목록을 읽지 못했습니다.",
                "Could not read the font collection."; "无法读取字体列表。", "フォント一覧を読み込めませんでした。"; "Impossible de lire la liste des polices.", "No se pudo leer la lista de fuentes."
            )
        })?;
        let mut fonts = Vec::new();
        for i in 0..collection.GetFontFamilyCount() {
            let result = (|| -> windows::core::Result<FontFamily> {
                let strings = collection.GetFontFamily(i)?.GetFamilyNames()?;
                let family = name(&strings, w!("en-us"))?;
                let label = match language {
                    Lang::Ko => name(&strings, w!("ko-kr"))?,
                    Lang::En => family.clone(),
                    Lang::Zh => name(&strings, w!("zh-cn"))?,
                    Lang::Ja => name(&strings, w!("ja-jp"))?,
                    Lang::Fr => name(&strings, w!("fr-fr"))?,
                    Lang::Es => name(&strings, w!("es-es"))?,
                };
                Ok(FontFamily { family, label })
            })();
            match result {
                Ok(font) => fonts.push(font),
                Err(e) => log::warn!("cannot read installed font family {i}: {e}"),
            }
        }
        Ok(fonts)
    }
}

#[cfg(target_os = "linux")]
fn scan(_language: Lang) -> Result<Vec<FontFamily>, String> {
    let output = std::process::Command::new("fc-list")
        .args(["--format", "%{family[0]}\n"])
        .output()
        .map_err(|e| format!("fc-list: {e}"))?;
    if !output.status.success() {
        return Err(tr!(
            "설치된 글꼴을 읽지 못했습니다. fontconfig를 확인하세요.",
            "Could not read installed fonts. Check fontconfig."; "无法读取已安装的字体。请检查 fontconfig。", "インストール済みフォントを読み込めませんでした。fontconfig を確認してください。"; "Impossible de lire les polices installées. Vérifiez fontconfig.", "No se pudieron leer las fuentes instaladas. Revisa fontconfig."
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| FontFamily {
            family: name.to_owned(),
            label: name.to_owned(),
        })
        .collect())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn scan(_language: Lang) -> Result<Vec<FontFamily>, String> {
    Err(tr!(
        "이 운영체제에서는 글꼴 검색을 지원하지 않습니다.",
        "Font scanning is not supported on this operating system."; "此操作系统不支持字体扫描。", "この OS はフォント検索に対応していません。"; "La recherche de polices n'est pas prise en charge sur ce système d'exploitation.", "La búsqueda de fuentes no es compatible con este sistema operativo."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_font_preferences() {
        for size in [9.0, 37.0, f32::NAN, f32::INFINITY] {
            assert!(NoteFont {
                size,
                ..NoteFont::default()
            }
            .validate()
            .is_err());
        }
        assert!(NoteFont {
            family: Some("bad\nname".into()),
            ..NoteFont::default()
        }
        .validate()
        .is_err());
        assert!(NoteFont {
            weight: 450,
            ..NoteFont::default()
        }
        .validate()
        .is_err());
        assert!(NoteFont {
            family: Some("맑은 고딕".into()),
            size: 36.0,
            weight: 700
        }
        .validate()
        .is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn enumerates_real_windows_fonts() {
        let fonts = installed(Lang::Ko).unwrap();
        assert!(fonts.iter().any(|font| font.family == "Segoe UI"));
        assert!(fonts
            .iter()
            .all(|font| !font.family.chars().any(char::is_control)));
        println!("Windows installed font families: {}", fonts.len());
    }
}
