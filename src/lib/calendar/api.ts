import { invoke } from "@tauri-apps/api/core";
import type { PinMode, Tint } from "../ipc";

export interface CalendarNote { id: string; createdAt: number }
export interface CalendarPreview { id: string; title: string; preview: string; tint: Tint; createdAt: number }
export const calendarApi = {
  open: () => invoke<void>("open_calendar"),
  notes: () => invoke<CalendarNote[]>("calendar_notes"),
  dayNotes: (start: number, end: number) => invoke<CalendarPreview[]>("calendar_day_notes", { start, end }),
  showNote: (id: string) => invoke<void>("show_calendar_note", { id }),
  pin: () => invoke<PinMode>("calendar_pin"),
  setPin: (mode: PinMode) => invoke<void>("set_calendar_pin", { mode }),
};
