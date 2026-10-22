import type { Tint } from "./ipc";

export const TINTS: { id: Tint }[] = [
  { id: "frost" },
  { id: "sage" },
  { id: "lilac" },
  { id: "amber" },
  { id: "rose" },
  { id: "ink" },
];

/** At or below this opacity the window drops its blur (see `BLUR_MIN` in windows.rs). */
export const BLUR_MIN = 0.3;

// Slider math produces 1 - 0.7 = 0.30000000000000004; match Rust's f32 boundary.
export const isClear = (opacity: number) => opacity <= BLUR_MIN + 1e-7;

/** Tint layer alpha for a note's opacity. Clear notes keep a trace of tint so they stay clickable. */
export function tintAlpha(opacity: number): number {
  return isClear(opacity) ? 0.02 + opacity * 0.6 : 0.06 + opacity * 0.86;
}

export const isDark = (tint: Tint) => tint === "ink";
