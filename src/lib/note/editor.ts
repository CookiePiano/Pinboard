import { Compartment, EditorSelection, EditorState, RangeSetBuilder, StateEffect, StateField, Transaction } from "@codemirror/state";
import { get } from "svelte/store";
import { t, type Messages } from "../i18n";
import { fontFamilyCss, noteFont } from "../fonts";
import { selectionTools } from "../appearance";
import type { NoteFont, TextFormat } from "../ipc";
import { equalFormats, formatRange, mapFormats, normalizeFormats, type FormatAction } from "./formatting";
import { linkTarget, trimUrl, URL_PATTERN } from "./links";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Decoration,
  EditorView,
  MatchDecorator,
  ViewPlugin,
  keymap,
  placeholder,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, invertedEffects, isolateHistory } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";

const BULLET = /^(\s*)([-*]) /;

function sameFont(a: NoteFont | null, b: NoteFont | null) {
  return a === b || (!!a && !!b && a.family === b.family && a.size === b.size && a.weight === b.weight);
}

const setFont = StateEffect.define<NoteFont | null>();
const fontOverride = StateField.define<NoteFont | null>({
  create: () => null,
  update(font, transaction) {
    for (const effect of transaction.effects) if (effect.is(setFont)) font = effect.value;
    return font;
  },
});

export function setEditorFont(view: EditorView, font: NoteFont | null) {
  if (!sameFont(view.state.field(fontOverride), font)) {
    view.dispatch({ effects: setFont.of(font), annotations: Transaction.addToHistory.of(false) });
  }
}

const setFormats = StateEffect.define<TextFormat[]>({ map: (value, changes) => mapFormats(value, changes, changes.newLength) });
export const textFormats = StateField.define<TextFormat[]>({
  create: () => [],
  update(value, transaction) {
    let next = transaction.docChanged ? mapFormats(value, transaction.changes, transaction.newDoc.length) : value;
    for (const effect of transaction.effects) if (effect.is(setFormats)) next = normalizeFormats(effect.value, transaction.newDoc.length);
    return next;
  },
  provide: (field) => EditorView.decorations.from(field, (formats) => Decoration.set(formats.map((format) => {
    const style = [
      format.bold ? "font-weight:750" : "",
      format.italic ? "font-style:italic" : "",
      format.size !== undefined ? `font-size:${format.size}pt` : "",
      format.highlight ? `background-color:${format.highlight};color:#1f252b;border-radius:2px;box-decoration-break:clone;-webkit-box-decoration-break:clone;text-shadow:none` : "",
    ].filter(Boolean).join(";");
    return Decoration.mark({ attributes: { style }, class: "cm-note-format" }).range(format.from, format.to);
  }), true)),
});

const formatHistory = invertedEffects.of((transaction) => {
  if (transaction.docChanged || transaction.effects.some((effect) => effect.is(setFormats))) {
    return [setFormats.of(transaction.startState.field(textFormats))];
  }
  return [];
});

export function applyFormat(view: EditorView, action: FormatAction) {
  const range = view.state.selection.main;
  if (range.empty) return;
  const tools = get(selectionTools), font = view.state.field(fontOverride) ?? get(noteFont);
  const formats = formatRange(view.state.field(textFormats), range.from, range.to, action, {
    step: tools.sizeStep, color: tools.highlightColor,
    baseSize: (pos) => (font.size + (view.state.doc.lineAt(pos).number === 1 ? 1 : 0)) * 0.75,
    baseBreaks: [view.state.doc.line(1).to + 1],
  }, view.state.doc.length);
  view.dispatch({ effects: setFormats.of(formats), annotations: [isolateHistory.of("full"), Transaction.userEvent.of("input.format")] });
  view.focus();
}

/** Keep the first line visually distinct without introducing checkbox widgets. */
function decorate(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  if (view.visibleRanges.some(({ from }) => from <= view.state.doc.line(1).to)) {
    builder.add(0, 0, Decoration.line({ class: "cm-lead" }));
  }
  return builder.finish();
}

const livePreview = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = decorate(view);
    }
    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged || u.selectionSet) {
        this.decorations = decorate(u.view);
      }
    }
  },
  { decorations: (v) => v.decorations },
);

const linkMatcher = new MatchDecorator({
  regexp: URL_PATTERN,
  decorate(add, from, _to, match) {
    const url = trimUrl(match[0]), href = linkTarget(url);
    if (href) add(from, from + url.length, Decoration.mark({ class: "cm-note-link", attributes: { "data-href": href } }));
  },
});

const links = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(view: EditorView) {
      this.decorations = linkMatcher.createDeco(view);
    }
    update(u: ViewUpdate) {
      this.decorations = linkMatcher.updateDeco(u, this.decorations);
    }
  },
  { decorations: (v) => v.decorations },
);

/** A plain click on a link opens it in the default browser; dragging or Shift-clicking still selects text. */
const openLinks = EditorView.domEventHandlers({
  click(event, view) {
    if (event.button !== 0 || event.shiftKey || event.altKey || !view.state.selection.main.empty) return false;
    const href = (event.target as Element).closest?.(".cm-note-link")?.getAttribute("data-href");
    if (!href) return false;
    event.preventDefault();
    openUrl(href).catch((error) => console.error("Opening link failed", error));
    return true;
  },
});

/** Enter continues an ordinary bullet list; an empty item ends it. */
function continueList(view: EditorView): boolean {
  const { state } = view;
  const range = state.selection.main;
  if (!range.empty) return false;
  const line = state.doc.lineAt(range.head);
  const marker = BULLET.exec(line.text);
  if (!marker) return false;
  if (range.head < line.from + marker[0].length) return false;

  if (line.text.trim() === marker[0].trim()) {
    view.dispatch({ changes: { from: line.from, to: line.to, insert: "" } });
    return true;
  }
  const next = `${marker[1]}${marker[2]} `;
  view.dispatch(
    state.update({
      changes: { from: range.head, insert: "\n" + next },
      selection: EditorSelection.cursor(range.head + 1 + next.length),
      scrollIntoView: true,
      userEvent: "input",
    }),
  );
  return true;
}

const highlight = HighlightStyle.define([
  { tag: tags.heading, fontWeight: "650" },
  { tag: tags.strong, fontWeight: "650" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strikethrough, textDecoration: "line-through" },
  { tag: tags.link, color: "var(--link)", textDecoration: "underline" },
  { tag: tags.url, color: "var(--link)" },
  { tag: tags.monospace, fontFamily: '"Cascadia Mono", Consolas, monospace', fontSize: "0.92em" },
  { tag: [tags.processingInstruction, tags.meta, tags.contentSeparator], color: "var(--faint)" },
]);

const theme = EditorView.theme({
  "&": { height: "100%", background: "transparent", color: "inherit" },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "var(--note-font-family, var(--font))", lineHeight: "1.6", overflow: "auto" },
  ".cm-content": { padding: "0 0 14px", caretColor: "currentColor", fontSize: "var(--note-font-size, 14.5px)", fontWeight: "var(--note-font-weight, 400)" },
  ".cm-line": { padding: "0 16px" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "currentColor", borderLeftWidth: "1.5px" },
  ".cm-placeholder": { color: "var(--faint)" },
  ".cm-note-link": { color: "var(--link)", textDecoration: "underline", textUnderlineOffset: "2px", cursor: "pointer" },
});

const locale = new Compartment();
const fontPreferences = ViewPlugin.fromClass(class {
  private stop: () => void;
  constructor(view: EditorView) {
    this.stop = noteFont.subscribe(() => this.apply(view));
  }
  apply(view: EditorView) {
      const font = view.state.field(fontOverride) ?? get(noteFont);
      view.dom.style.setProperty("--note-font-family", fontFamilyCss(font.family));
      view.dom.style.setProperty("--note-font-size", `${font.size}px`);
      view.dom.style.setProperty("--note-font-weight", String(font.weight));
      view.requestMeasure();
  }
  update(update: ViewUpdate) {
    if (update.startState.field(fontOverride) !== update.state.field(fontOverride)) this.apply(update.view);
  }
  destroy() { this.stop(); }
});
function localeExtensions(messages: Messages) {
  return [
    EditorView.contentAttributes.of({ spellcheck: "false", "aria-label": messages.noteContent }),
    placeholder(messages.notePlaceholder),
  ];
}

export function setEditorLanguage(view: EditorView, messages: Messages) {
  view.dispatch({ effects: locale.reconfigure(localeExtensions(messages)) });
}

export function createEditor(parent: HTMLElement, doc: string, onChange: (state: EditorState) => void, formats: TextFormat[] = [], onSelection: (view: EditorView) => void = () => {}, font: NoteFont | null = null): EditorView {
  let selectionFrame = 0;
  function scheduleSelection(view: EditorView) {
    if (selectionFrame) return;
    selectionFrame = requestAnimationFrame(() => { selectionFrame = 0; onSelection(view); });
  }
  return new EditorView({
    parent,
    state: EditorState.create({
      doc,
      extensions: [
        history(),
        textFormats.init(() => normalizeFormats(formats, doc.length)),
        formatHistory,
        keymap.of([{ key: "Enter", run: continueList }, ...defaultKeymap, ...historyKeymap]),
        markdown(),
        syntaxHighlighting(highlight),
        livePreview,
        links,
        openLinks,
        fontOverride.init(() => font),
        fontPreferences,
        EditorView.lineWrapping,
        locale.of(localeExtensions(get(t))),
        theme,
        ViewPlugin.define(() => ({ destroy() { cancelAnimationFrame(selectionFrame); } })),
        EditorView.domEventHandlers({ scroll: (_event, view) => { scheduleSelection(view); } }),
        EditorView.updateListener.of((u) => {
          if ((u.docChanged || u.startState.field(textFormats) !== u.state.field(textFormats)) && !u.transactions.some((transaction) => transaction.annotation(Transaction.remote))) {
            onChange(u.state);
          }
          if (u.selectionSet || u.docChanged || u.viewportChanged || u.focusChanged || u.transactions.some((tr) => tr.effects.some((e) => e.is(setFormats)))) scheduleSelection(u.view);
        }),
      ],
    }),
  });
}

/** Replaces the document after a sync, keeping the cursor where it was as far as possible. */
export function replaceDoc(view: EditorView, text: string, formats: TextFormat[] = []) {
  const changed = view.state.doc.toString() !== text;
  if (!changed && equalFormats(view.state.field(textFormats), formats)) return;
  const head = Math.min(view.state.selection.main.head, text.length);
  view.dispatch({
    changes: changed ? { from: 0, to: view.state.doc.length, insert: text } : undefined,
    effects: setFormats.of(formats),
    selection: EditorSelection.cursor(head),
    annotations: [Transaction.addToHistory.of(false), Transaction.remote.of(true)],
  });
}
