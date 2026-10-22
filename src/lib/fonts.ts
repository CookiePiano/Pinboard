import { writable } from "svelte/store";
import type { NoteFont } from "./ipc";

export const defaultNoteFont: NoteFont = { family: null, size: 14.5, weight: 400 };
export const noteFont = writable<NoteFont>({ ...defaultNoteFont });

// Quoted CSS strings keep installed family names from being interpreted as CSS syntax.
export function fontFamilyCss(family: string | null) {
  const quoted = family ? `"${family.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/[\x00-\x1f\x7f]/g, " ")}"` : null;
  return quoted ? `${quoted}, var(--font)` : "var(--font)";
}
