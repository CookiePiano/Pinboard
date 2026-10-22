//! The untouched welcome note follows this device's language without creating sync conflicts.

use crate::i18n::Lang;
use crate::model::Note;

pub const ID: &str = "00000000-0000-4000-8000-00000000c0de";
const LEGACY_KO: &str = "Pinboard에 오신 것을 환영합니다\n\
위쪽에 마우스를 올리면 도구가 나타납니다.\n\n\
- [ ] 색과 투명도 바꿔 보기\n\
- [ ] 바탕화면에 고정해 보기\n\
- [ ] Ctrl+Alt+N으로 새 메모 만들기";
const LEGACY_EN: &str = "Welcome to Pinboard\n\
Hover over the top edge to see the tools.\n\n\
- [ ] Try changing the color and transparency\n\
- [ ] Pin a note to the desktop\n\
- [ ] Create a note with Ctrl+Alt+N";
const PREVIOUS_KO: &str = "Pinboard에 오신 것을 환영합니다\n\
위쪽 도구로 메모를 관리하세요.\n\n\
- [ ] 색과 투명도 바꿔 보기\n\
- [ ] 바탕화면에 고정해 보기\n\
- [ ] + 버튼으로 새 메모 만들기";
const PREVIOUS_EN: &str = "Welcome to Pinboard\n\
Use the tools at the top to manage your notes.\n\n\
- [ ] Try changing the color and transparency\n\
- [ ] Pin a note to the desktop\n\
- [ ] Create a note with the + button";

const KO: &str = "Pinboard에 오신 것을 환영합니다\n\
위쪽 도구로 메모를 관리하세요.\n\n\
- 색과 투명도 바꿔 보기\n\
- 바탕화면에 고정해 보기\n\
- + 버튼으로 새 메모 만들기";
const EN: &str = "Welcome to Pinboard\n\
Use the tools at the top to manage your notes.\n\n\
- Try changing the color and transparency\n\
- Pin a note to the desktop\n\
- Create a note with the + button";

const ZH: &str = "欢迎使用 Pinboard\n使用顶部工具管理便签。\n\n- 尝试更改颜色和透明度\n- 将便签固定到桌面\n- 点击 + 按钮新建便签";
const JA: &str = "Pinboard へようこそ\n上部のツールでメモを管理できます。\n\n- 色と透明度を変えてみましょう\n- メモをデスクトップに固定しましょう\n- + ボタンで新しいメモを作成しましょう";

const FR: &str = "Bienvenue dans Pinboard\nUtilisez les outils en haut pour gérer vos notes.\n\n- Essayez de changer la couleur et la transparence\n- Épinglez une note sur le bureau\n- Créez une note avec le bouton +";
const ES: &str = "Te damos la bienvenida a Pinboard\nUsa las herramientas de arriba para gestionar tus notas.\n\n- Prueba a cambiar el color y la transparencia\n- Fija una nota en el escritorio\n- Crea una nota con el botón +";

pub fn body(language: Lang) -> &'static str {
    match language { Lang::Ko => KO, Lang::En => EN, Lang::Zh => ZH, Lang::Ja => JA, Lang::Fr => FR, Lang::Es => ES }
}

fn pristine(note: &Note) -> bool {
    note.id == ID && note.created_at == 0 && note.updated_at == 0
        && note.deleted_at.is_none() && note.conflict_of.is_none()
        && [KO, EN, ZH, JA, FR, ES, PREVIOUS_KO, PREVIOUS_EN, LEGACY_KO, LEGACY_EN].contains(&note.body.as_str())
}

pub fn localize(note: &mut Note, language: Lang) -> bool {
    if pristine(note) && note.body != body(language) {
        note.body = body(language).into();
        true
    } else { false }
}

pub fn sync_body(note: &Note) -> &str {
    if pristine(note) { LEGACY_KO } else { &note.body }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn welcome() -> Note {
        let mut note = Note::new("test", "frost", 0.55);
        note.id = ID.into();
        note.created_at = 0;
        note.updated_at = 0;
        note.body = KO.into();
        note
    }

    #[test]
    fn pristine_welcome_switches_both_ways() {
        let mut note = welcome();
        assert!(localize(&mut note, Lang::En));
        assert_eq!(note.body, EN);
        assert_eq!(note.updated_at, 0);
        assert!(localize(&mut note, Lang::Ko));
        assert_eq!(note.body, KO);
    }

    #[test]
    fn all_languages_preserve_sync_identity_and_user_edits() {
        let mut note = welcome();
        let hash = note.content_hash();
        for lang in [Lang::Zh, Lang::Ja, Lang::Fr, Lang::Es, Lang::En, Lang::Ko] {
            assert!(localize(&mut note, lang));
            assert_eq!(note.body, body(lang));
            assert_eq!(note.content_hash(), hash);
        }
        for lang in [Lang::Zh, Lang::Ja, Lang::Fr, Lang::Es] {
            let mut edited = welcome();
            localize(&mut edited, lang);
            edited.body.push_str("\nMy text");
            let before = edited.clone();
            assert!(!localize(&mut edited, Lang::Ko));
            assert_eq!(edited, before);
        }
    }

    #[test]
    fn legacy_welcome_updates_without_changing_sync_identity() {
        let mut note = welcome();
        let hash = note.content_hash();
        note.body = LEGACY_EN.into();
        assert!(localize(&mut note, Lang::Ko));
        assert_eq!(note.body, KO);
        assert_eq!(note.content_hash(), hash);
    }

    #[test]
    fn authored_or_deleted_welcome_is_preserved() {
        let mut authored = welcome();
        authored.body.push_str("\nMy own text");
        let before = authored.clone();
        assert!(!localize(&mut authored, Lang::En));
        assert_eq!(authored, before);
        let mut edited = welcome();
        edited.updated_at = 1;
        assert!(!localize(&mut edited, Lang::En));
        let mut deleted = welcome();
        deleted.deleted_at = Some(1);
        assert!(!localize(&mut deleted, Lang::En));
    }

    #[test]
    fn translations_share_a_sync_hash_only_while_pristine() {
        let ko = welcome();
        let mut en = ko.clone();
        localize(&mut en, Lang::En);
        assert_eq!(ko.content_hash(), en.content_hash());
        en.updated_at = 1;
        assert_ne!(ko.content_hash(), en.content_hash());
    }
}
