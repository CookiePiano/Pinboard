//! UI language for text produced on the Rust side: tray menu, errors, the sign-in page.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Lang {
    #[default]
    Ko,
    En,
    Zh,
    Ja,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 6] = [Lang::Ko, Lang::En, Lang::Zh, Lang::Ja, Lang::Fr, Lang::Es];

    pub fn tag(self) -> &'static str {
        match self {
            Self::Ko => "ko", Self::En => "en", Self::Zh => "zh", Self::Ja => "ja", Self::Fr => "fr", Self::Es => "es",
        }
    }
}

static LANGUAGE: AtomicU8 = AtomicU8::new(Lang::Ko as u8);

pub fn set(lang: Lang) {
    LANGUAGE.store(lang as u8, Ordering::SeqCst);
}

pub fn get() -> Lang {
    match LANGUAGE.load(Ordering::SeqCst) {
        1 => Lang::En, 2 => Lang::Zh, 3 => Lang::Ja, 4 => Lang::Fr, 5 => Lang::Es, _ => Lang::Ko,
    }
}

/// Each locale is a checked `format!` string; the semicolon separates added locales.
#[macro_export]
macro_rules! tr {
    (@lang $lang:expr; $ko:literal, $en:literal; $zh:literal, $ja:literal; $fr:literal, $es:literal $(, $arg:expr)* $(,)?) => {
        match $lang {
            $crate::i18n::Lang::Ko => format!($ko $(, $arg)*),
            $crate::i18n::Lang::En => format!($en $(, $arg)*),
            $crate::i18n::Lang::Zh => format!($zh $(, $arg)*),
            $crate::i18n::Lang::Ja => format!($ja $(, $arg)*),
            $crate::i18n::Lang::Fr => format!($fr $(, $arg)*),
            $crate::i18n::Lang::Es => format!($es $(, $arg)*),
        }
    };
    ($ko:literal, $en:literal; $zh:literal, $ja:literal; $fr:literal, $es:literal $(, $arg:expr)* $(,)?) => {
        $crate::tr!(@lang $crate::i18n::get(); $ko, $en; $zh, $ja; $fr, $es $(, $arg)*)
    };
    // Keep older call sites compatible during concurrent feature development.
    ($ko:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::i18n::get() == $crate::i18n::Lang::Ko {
            format!($ko $(, $arg)*)
        } else {
            format!($en $(, $arg)*)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_round_trip_and_default_stays_korean() {
        assert_eq!(Lang::default(), Lang::Ko);
        for lang in Lang::ALL {
            let json = serde_json::to_string(&lang).unwrap();
            assert_eq!(json, format!("\"{}\"", lang.tag()));
            assert_eq!(serde_json::from_str::<Lang>(&json).unwrap(), lang);
        }
    }

    #[test]
    fn formats_arguments_in_each_language() {
        let name = "Pinboard";
        for (lang, expected) in [(Lang::Ko, "메모"), (Lang::En, "note"), (Lang::Zh, "便签"), (Lang::Ja, "メモ"), (Lang::Fr, "notes"), (Lang::Es, "notas")] {
            assert_eq!(
                tr!(@lang lang; "{name}: {} 메모", "{name}: {} note"; "{name}: {} 便签", "{name}: {} メモ"; "{name}: {} notes", "{name}: {} notas", 2),
                format!("Pinboard: 2 {expected}")
            );
        }
    }
}
