import type { CalendarNote } from "./api";

// Local getters deliberately follow Windows time zone settings, rather than UTC.
export function dateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function dayRange(date: Date): { start: number; end: number } {
  const y = date.getFullYear(), m = date.getMonth(), d = date.getDate();
  // Construct both midnights: a day need not be 24 hours across a DST change.
  return { start: new Date(y, m, d).getTime(), end: new Date(y, m, d + 1).getTime() };
}

export function monthDays(month: Date): Date[] {
  const first = new Date(month.getFullYear(), month.getMonth(), 1);
  const start = 1 - first.getDay();
  return Array.from({ length: 42 }, (_, i) => new Date(first.getFullYear(), first.getMonth(), start + i));
}

export function countDays(notes: CalendarNote[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const note of notes) {
    if (note.createdAt <= 0) continue;
    const date = new Date(note.createdAt);
    if (!Number.isFinite(date.getTime())) continue;
    const key = dateKey(date);
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return counts;
}
