import { derived } from "svelte/store";
import { language } from "../i18n";
import type { Language } from "../ipc";
import { zhFont, jaFont, frFont, esFont } from "../locales/panels";

const ko = {
  noteScope: "이 메모 전체에 적용됩니다.",
  lede: "이 PC에서 사용할 메모의 기본 글꼴을 고르세요. 메모에서 따로 지정한 글꼴 설정이 우선 적용됩니다.",
  installed: "설치된 글꼴", search: "글꼴 이름 검색", defaultFont: "기본 글꼴 (Pretendard)",
  scan: "다시 검색", scanning: "설치된 글꼴을 검색하고 있습니다…", scanError: "글꼴을 불러오지 못했습니다.",
  found: (count: number) => `설치된 글꼴 ${count}개`, noResults: "검색 결과가 없습니다.",
  unavailable: "선택한 글꼴이 이 PC에 없습니다. 기본 글꼴로 표시합니다.",
  fallback: "한글 등 지원하지 않는 문자는 기본 글꼴로 표시됩니다.",
  size: "글자 크기", weight: "굵기", regular: "보통", medium: "중간", semibold: "약간 굵게", bold: "굵게",
  preview: "미리보기", previewKo: "오늘의 생각을 메모하세요.", previewEn: "The quick brown fox jumps over the lazy dog.",
  apply: "적용", reset: "기본값 복원", saved: "메모의 기본 글꼴을 적용하고 저장했습니다.",
};
const en: typeof ko = {
  noteScope: "Applies to this entire note.",
  lede: "Choose the default note font for this PC. Individual note font settings take precedence.",
  installed: "Installed fonts", search: "Search font names", defaultFont: "Default (Pretendard)",
  scan: "Scan again", scanning: "Scanning installed fonts…", scanError: "Could not load installed fonts.",
  found: (count) => `${count} installed font families`, noResults: "No matching fonts.",
  unavailable: "The selected font is unavailable on this PC. Using the default font.",
  fallback: "Unsupported characters, including Korean, use a fallback font.",
  size: "Font size", weight: "Weight", regular: "Regular", medium: "Medium", semibold: "Semibold", bold: "Bold",
  preview: "Preview", previewKo: "오늘의 생각을 메모하세요.", previewEn: "The quick brown fox jumps over the lazy dog.",
  apply: "Apply", reset: "Restore defaults", saved: "Default note font applied and saved.",
};

const messages: Record<Language, typeof ko> = { ko, en, zh: zhFont, ja: jaFont, fr: frFont, es: esFont };
export const fontText = derived(language, (lang) => messages[lang]);
