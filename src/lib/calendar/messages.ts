import { derived } from "svelte/store";
import { language } from "../i18n";
import type { Language } from "../ipc";
import { zhCalendar, jaCalendar, frCalendar, esCalendar } from "../locales/panels";

const ko = {
  title: "메모 달력", position: "달력 위치", today: "오늘", previous: "이전 달", next: "다음 달", close: "달력 닫기",
  count: (n: number) => `메모 ${n}개 작성됨`,
  retry: "다시 시도",
  noteList: "메모 목록", chooseDate: "날짜를 선택하세요", chooseDateHint: "선택한 날짜에 작성한 메모를 여기에서 볼 수 있습니다.", empty: "이 날짜에 작성한 메모가 없습니다.",
  selectNote: (title: string) => `${title} 선택`, deleteSelected: "선택한 메모 삭제", confirmDelete: (n: number) => `한 번 더 누르면 ${n}개 삭제`, openNote: "메모 열기", loading: "메모 목록을 불러오는 중…", notes: (n: number) => `메모 ${n}개`,
};
const en: typeof ko = {
  title: "Note calendar", position: "Calendar position", today: "Today", previous: "Previous month", next: "Next month", close: "Close calendar",
  count: (n) => `${n} ${n === 1 ? "note" : "notes"} created`,
  retry: "Retry",
  noteList: "Notes", chooseDate: "Choose a date", chooseDateHint: "Notes created on the selected date appear here.", empty: "No notes were created on this date.",
  selectNote: (title) => `Select ${title}`, deleteSelected: "Delete selected notes", confirmDelete: (n) => `Click again to delete ${n}`, openNote: "Open note", loading: "Loading notes…", notes: (n) => `${n} ${n === 1 ? "note" : "notes"}`,
};
const messages: Record<Language, typeof ko> = { ko, en, zh: zhCalendar, ja: jaCalendar, fr: frCalendar, es: esCalendar };
export const calendarText = derived(language, (lang) => messages[lang]);
