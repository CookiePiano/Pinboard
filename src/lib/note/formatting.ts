import type { TextFormat } from "../ipc";

export type FormatAction = "bold" | "italic" | "smaller" | "larger" | "highlight";
type FormatAttributes = Omit<TextFormat, "from" | "to">;

function attributes(format: Partial<TextFormat>): FormatAttributes {
  const result: FormatAttributes = {};
  if (format.bold) result.bold = true;
  if (format.italic) result.italic = true;
  if (format.size !== undefined) result.size = format.size;
  if (format.highlight) result.highlight = format.highlight;
  return result;
}

/** Append an ordered run, coalescing adjacent styles without serializing them. */
function appendFormat(result: TextFormat[], from: number, to: number, attrs: FormatAttributes) {
  if (from >= to || !(attrs.bold || attrs.italic || attrs.size !== undefined || attrs.highlight)) return;
  const last = result.at(-1);
  if (last && last.to === from && last.bold === attrs.bold && last.italic === attrs.italic
    && last.size === attrs.size && last.highlight === attrs.highlight) last.to = to;
  else result.push({ from, to, ...attrs });
}

/** Canonical non-overlapping runs; adjacent identical styles share one run. */
export function normalizeFormats(formats: TextFormat[], length: number): TextFormat[] {
  const result: TextFormat[] = [];
  for (const format of [...formats].sort((a, b) => a.from - b.from)) {
    const from = Math.max(result.at(-1)?.to ?? 0, Math.min(length, format.from));
    const to = Math.min(length, format.to);
    appendFormat(result, from, to, attributes(format));
  }
  return result;
}

export function fullyFormatted(formats: TextFormat[], from: number, to: number, key: "bold" | "italic" | "highlight", value: boolean | string = true): boolean {
  let covered = from;
  for (const format of formats) {
    if (format.to <= covered) continue;
    if (format.from > covered || format[key] !== value) return false;
    covered = format.to;
    if (covered >= to) return true;
  }
  return false;
}

/** Modify only the selection, retaining independent styles and mixed font sizes. */
export function formatRange(formats: TextFormat[], from: number, to: number, action: FormatAction, options: { step: number; color: string; baseSize: (pos: number) => number; baseBreaks?: number[] }, length: number): TextFormat[] {
  if (from >= to) return formats;
  const boundaries = [...new Set([0, length, from, to, ...(options.baseBreaks ?? []), ...formats.flatMap((f) => [f.from, f.to])])].filter((pos) => pos >= 0 && pos <= length).sort((a, b) => a - b);
  const toggle = action === "bold" || action === "italic" ? !fullyFormatted(formats, from, to, action)
    : action === "highlight" ? !fullyFormatted(formats, from, to, "highlight", options.color) : false;
  const result: TextFormat[] = [];
  let index = 0;
  for (let i = 0; i < boundaries.length - 1; i++) {
    const start = boundaries[i], end = boundaries[i + 1];
    while (index < formats.length && formats[index].to <= start) index++;
    const previous = formats[index];
    const attrs = attributes(previous && previous.from <= start ? previous : {});
    if (start >= from && end <= to) {
      if (action === "bold" || action === "italic") { if (toggle) attrs[action] = true; else delete attrs[action]; }
      else if (action === "highlight") { if (toggle) attrs.highlight = options.color; else delete attrs.highlight; }
      else attrs.size = Math.round(Math.min(96, Math.max(6, (attrs.size ?? options.baseSize(start)) + (action === "larger" ? options.step : -options.step))) * 1000) / 1000;
    }
    appendFormat(result, start, end, attrs);
  }
  return result;
}

export function mapFormats(formats: TextFormat[], changes: { mapPos: (pos: number, assoc?: number) => number; touchesRange?: (from: number, to: number) => boolean | "cover" }, length: number): TextFormat[] {
  if (!formats.length || (changes.touchesRange && !changes.touchesRange(0, formats[formats.length - 1].to))) return formats;
  // State formats are ordered. Position mapping is monotonic, including deletions.
  const result: TextFormat[] = [];
  for (const format of formats) {
    const from = Math.max(result.at(-1)?.to ?? 0, Math.min(length, changes.mapPos(format.from, -1)));
    appendFormat(result, from, Math.min(length, changes.mapPos(format.to, 1)), attributes(format));
  }
  // Preserve identity when positions/styles did not change. CodeMirror can then
  // retain its existing decoration tree instead of rebuilding every mark.
  return equalFormats(result, formats) ? formats : result;
}

export function equalFormats(a: TextFormat[], b: TextFormat[]): boolean {
  return a === b || (a.length === b.length && a.every((f, i) => {
    const g = b[i];
    return f.from === g.from && f.to === g.to && f.bold === g.bold && f.italic === g.italic && f.size === g.size && f.highlight === g.highlight;
  }));
}
