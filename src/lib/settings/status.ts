import type { SyncStatus } from "../ipc";
import { messages, type Messages } from "../i18n";

export type Tone = "off" | "ok" | "busy" | "error";

export function ago(at: number, now: number, t: Messages = messages.ko): string {
  const min = Math.floor((now - at) / 60_000);
  if (min < 1) return t.justNow;
  if (min < 60) return t.minutesAgo(min);
  const hours = Math.floor(min / 60);
  if (hours < 24) return t.hoursAgo(hours);
  return t.daysAgo(Math.floor(hours / 24));
}

export function describeStatus(s: SyncStatus, now: number, t: Messages = messages.ko): { tone: Tone; text: string } {
  switch (s.phase) {
    case "off":
      return { tone: "off", text: t.syncOff };
    case "syncing":
      return { tone: "busy", text: t.syncing };
    case "error":
      return { tone: "error", text: t.syncError };
    case "idle":
      return {
        tone: "ok",
        text: s.lastSyncAt ? t.syncedAgo(ago(s.lastSyncAt, now, t)) : t.syncReady,
      };
  }
}
