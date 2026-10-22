<script lang="ts">
  import { t } from "../i18n";
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import type { EditorState } from "@codemirror/state";
  import type { EditorView } from "@codemirror/view";
  import { ArrowUpToLine, CalendarDays, Monitor, Palette, Pin, Plus, Settings, Trash, X } from "@lucide/svelte";
  import { calendarApi } from "../calendar/api";
  import { calendarText } from "../calendar/messages";
  import { api, type Note, type NoteFont, type PinMode, type Tint, type TextFormat } from "../ipc";
  import { noteFont } from "../fonts";
  import { darkMode, selectionTools } from "../appearance";
  import { isClear, isDark, tintAlpha } from "../tints";
  import { applyFormat, createEditor, replaceDoc, setEditorFont, setEditorLanguage, textFormats } from "./editor";
  import { fullyFormatted, type FormatAction } from "./formatting";
  import { createSaveQueue } from "./save-queue";
  import { dragWindow } from "../window-drag";
  import TintPanel from "./TintPanel.svelte";
  import PinPanel from "./PinPanel.svelte";

  let { id }: { id: string } = $props();

  const win = getCurrentWebviewWindow();
  let note = $state<Note | null>(null);
  let pin = $state<PinMode>("normal");
  let panel = $state<null | "tint" | "pin" | "delete">(null);
  let deleting = $state(false);
  let deleteError = $state<string | null>(null);
  let calendarError = $state<string | null>(null);
  let calendarOpening = $state(false);
  let focused = $state(false);
  let transferBusy = $state(false);
  let fresh = $state(false);
  let fontSaving = $state(false);
  let host: HTMLDivElement;
  let view: EditorView | undefined;
  let actionError = $state<string | null>(null);
  let bodySaveError = $state<string | null>(null);
  let opacitySaveError = $state<string | null>(null);
  let saveError = $derived(bodySaveError ?? opacitySaveError ?? actionError);
  let toolbar = $state<{ left: number; top: number; bold: boolean; italic: boolean; highlight: boolean } | null>(null);

  const bodySaves = createSaveQueue<{ body: string; formats: TextFormat[] }>(
    (patch) => api.saveNoteContent(id, patch.body, patch.formats),
    (error) => { bodySaveError = error; },
  );
  let timer: ReturnType<typeof setTimeout> | undefined;

  const opacitySaves = createSaveQueue<number>(
    (opacity) => api.updateNote(id, { opacity }),
    (error) => { opacitySaveError = error; },
  );
  let opacityTimer: ReturnType<typeof setTimeout> | undefined;
  const dirty = () => bodySaves.dirty || opacitySaves.dirty;

  function queueBody(state: EditorState) {
    bodySaves.queueLazy(() => ({ body: state.doc.toString(), formats: state.field(textFormats) }));
    clearTimeout(timer);
    timer = setTimeout(flush, 350);
  }

  async function flush() {
    clearTimeout(timer);
    clearTimeout(opacityTimer);
    await Promise.all([bodySaves.flush(), opacitySaves.flush()]);
    if (!dirty()) actionError = null;
  }

  function showSelection(editor: EditorView) {
    if (!editor.dom.isConnected) return;
    const range = editor.state.selection.main;
    if (range.empty || (!editor.hasFocus && !document.activeElement?.closest(".format-toolbar"))) { toolbar = null; return; }
    const anchor = editor.coordsAtPos(range.head);
    const bounds = editor.scrollDOM.getBoundingClientRect();
    if (!anchor || anchor.bottom < bounds.top || anchor.top > bounds.bottom) { toolbar = null; return; }
    const formats = editor.state.field(textFormats);
    toolbar = {
      left: Math.max(8, Math.min(innerWidth - 196, anchor.left - 92)),
      top: Math.max(36, Math.min(innerHeight - 42, anchor.top >= 78 ? anchor.top - 40 : anchor.bottom + 6)),
      bold: fullyFormatted(formats, range.from, range.to, "bold"),
      italic: fullyFormatted(formats, range.from, range.to, "italic"),
      highlight: fullyFormatted(formats, range.from, range.to, "highlight", $selectionTools.highlightColor),
    };
  }
  function format(action: FormatAction) { if (view) { applyFormat(view, action); showSelection(view); } }

  async function setTint(tint: Tint) {
    try {
      await flush();
      if (!saveError) note = await api.updateNote(id, { tint });
    } catch (e) { actionError = String(e); }
  }

  function setOpacity(opacity: number) {
    if (note) note.opacity = opacity;
    opacitySaves.queue(opacity);
    clearTimeout(opacityTimer);
    opacityTimer = setTimeout(() => { void opacitySaves.flush(); }, 150);
  }

  async function setPin(mode: PinMode) {
    pin = mode;
    panel = null;
    await api.setPinMode(id, mode);
  }

  async function setFont(patch: Partial<NoteFont>) {
    if (!note || fontSaving) return;
    fontSaving = true;
    try {
      await flush();
      if (saveError) return;
      const font = { ...(note.font ?? $noteFont), ...patch };
      const updated = await api.updateNote(id, { font, resetFontSizes: patch.size !== undefined });
      note = updated;
      if (view) {
        setEditorFont(view, updated.font ?? null);
        replaceDoc(view, updated.body, updated.formats ?? []);
      }
      actionError = null;
    } catch (e) { actionError = String(e); }
    finally { fontSaving = false; }
  }

  async function newNote() {
    panel = null;
    await flush();
    await api.createNote(id);
  }

  async function hide() {
    try {
      await flush();
      // Keep unsaved content and its retry action visible after a write failure.
      if (!dirty()) await api.hideNote(id);
    } catch (e) { actionError = String(e); }
  }

  async function openCalendar() {
    if (calendarOpening) return;
    panel = null;
    calendarError = null;
    calendarOpening = true;
    try { await flush(); await calendarApi.open(); }
    catch (e) { calendarError = String(e); }
    finally { calendarOpening = false; }
  }

  async function remove() {
    deleting = true;
    deleteError = null;
    try {
      await flush();
      await api.deleteNote(id);
    } catch (e) {
      deleteError = String(e);
      deleting = false;
    }
  }

  function toggle(which: "tint" | "pin" | "delete") {
    panel = panel === which ? null : which;
    deleteError = null;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && toolbar) { toolbar = null; view?.dispatch({ selection: { anchor: view.state.selection.main.head } }); }
    if (e.key === "Escape" && panel) {
      panel = null;
      view?.focus();
    }
  }

  function onPointerDown(e: PointerEvent) {
    if (panel && !(e.target as Element).closest(".panel, .tool")) panel = null;
  }

  onMount(() => {
    const stops: (() => void)[] = [];
    let alive = true;
    (async () => {
      const r = await api.getNote(id);
      if (!alive) return;
      note = r.note;
      pin = r.pin;
      fresh = Date.now() - r.note.createdAt < 4000;
      view = createEditor(host, r.note.body, queueBody, r.note.formats ?? [], showSelection, r.note.font ?? null);
      stops.push(selectionTools.subscribe(() => { if (view) showSelection(view); }));
      stops.push(t.subscribe((messages) => { if (view) setEditorLanguage(view, messages); }));
      stops.push(
        await win.listen<Note>("note://updated", (e) => {
          const opacity = opacitySaves.dirty ? note?.opacity : undefined;
          if (view) setEditorFont(view, e.payload.font ?? null);
          if (!bodySaves.dirty) {
            note = e.payload;
            if (view) replaceDoc(view, e.payload.body, e.payload.formats ?? []);
          } else if (note) {
            note.tint = e.payload.tint; note.opacity = e.payload.opacity; note.background = e.payload.background; note.font = e.payload.font;
          }
          if (note && opacity !== undefined) note.opacity = opacity;
        }),
      );
      stops.push(await win.onFocusChanged(({ payload }) => { focused = payload; if (!payload) { toolbar = null; void flush(); } }));
      stops.push(await win.listen("note://hide-requested", () => { void hide(); }));
      stops.push(await win.listen<string>("archive://flush", async (e) => {
        transferBusy = true;
        toolbar = null;
        await flush();
        while (dirty() && !saveError) await flush();
        await api.editorReady(e.payload, saveError);
      }));
      stops.push(await win.listen("archive://release", () => { transferBusy = false; }));
      await api.windowReady();
      if (fresh) view.focus();
    })();
    return () => {
      alive = false;
      clearTimeout(timer); clearTimeout(opacityTimer);
      stops.forEach((s) => s());
      view?.destroy();
    };
  });

  const pinIcon = { normal: Pin, top: ArrowUpToLine, desktop: Monitor };
  let pinLabel = $derived({ normal: $t.position($t.pinNormal), top: $t.position($t.pinTop), desktop: $t.position($t.pinDesktop) });
  let PinIcon = $derived(pinIcon[pin]);
</script>

<svelte:window onkeydown={onKey} onpointerdown={onPointerDown} />

<div
  class="note"
  inert={transferBusy}
  class:dark={$darkMode || (note && isDark(note.tint))}
  class:fresh
  class:clear={note && isClear(note.opacity)}
  class:active={focused || panel}
  style:--tint={`var(--tint-${$darkMode ? "dark-" : ""}${note?.tint ?? "frost"})`}
  style:--alpha={tintAlpha(note?.opacity ?? 0.55)}
>
  {#if note?.background}
    <img class="note-background" src={note.background.image} alt="" style:opacity={note.background.opacity} draggable="false" />
  {/if}
  <header class="bar" role="toolbar" aria-label={$t.noteContent} tabindex="-1" use:dragWindow={(error) => { actionError = String(error); }}>
    <button class="tool" title={$t.newNote} aria-label={$t.newNote} onclick={newNote}>
      <Plus size={15} strokeWidth={2} />
    </button>
    <button class="tool" title={$calendarText.title} aria-label={$calendarText.title} disabled={calendarOpening} onclick={openCalendar}>
      <CalendarDays size={15} strokeWidth={2} />
    </button>
    <span class="grip" aria-hidden="true"></span>
    <div class="tools">
      <button
        class="tool"
        title={$t.colorTransparency}
        aria-label={$t.colorTransparency}
        aria-expanded={panel === "tint"}
        onclick={() => toggle("tint")}
      >
        <Palette size={15} strokeWidth={2} />
      </button>
      <button
        class="tool"
        class:on={pin !== "normal"}
        title={pinLabel[pin]}
        aria-label={pinLabel[pin]}
        aria-expanded={panel === "pin"}
        onclick={() => toggle("pin")}
      >
        <PinIcon size={15} strokeWidth={2} />
      </button>
      <button
        class="tool"
        title={$t.openSettings}
        aria-label={$t.openSettings}
        onclick={async () => { panel = null; await flush(); await api.openSettings(id); }}
      >
        <Settings size={15} strokeWidth={2} />
      </button>
      <button
        class="tool danger"
        title={$t.deleteNote}
        aria-label={$t.deleteNote}
        aria-expanded={panel === "delete"}
        onclick={() => toggle("delete")}
      >
        <Trash size={15} strokeWidth={2} />
      </button>
      <button class="tool" title={$t.hide} aria-label={$t.hide} onclick={hide}>
        <X size={15} strokeWidth={2} />
      </button>
    </div>
  </header>

  {#if note?.conflictOf}
    <p class="conflict">{$t.conflict}</p>
  {/if}

  <div class="body" inert={fontSaving} bind:this={host}></div>

  {#if toolbar && !panel}
    <div class="format-toolbar" role="toolbar" tabindex="-1" aria-label={$t.selectionTools} style:left="{toolbar.left}px" style:top="{toolbar.top}px" onpointerdown={(e) => e.preventDefault()}>
      <button class:pressed={toolbar.bold} aria-pressed={toolbar.bold} aria-label={$t.bold} title={$t.bold} onclick={() => format("bold")}><b>B</b></button>
      <button class:pressed={toolbar.italic} aria-pressed={toolbar.italic} aria-label={$t.italic} title={$t.italic} onclick={() => format("italic")}><i>I</i></button>
        <button aria-label={$t.smaller} title={`${$t.smaller} (${$selectionTools.sizeStep}pt)`} onclick={() => format("smaller")}>−</button>
        <button aria-label={$t.larger} title={`${$t.larger} (${$selectionTools.sizeStep}pt)`} onclick={() => format("larger")}>+</button>
      <button class:pressed={toolbar.highlight} aria-pressed={toolbar.highlight} aria-label={$t.highlight} title={$t.highlight} onclick={() => format("highlight")}><span class="highlighter" style:--highlight={$selectionTools.highlightColor}>H</span></button>
    </div>
  {/if}

  {#if saveError}<button class="save-error" onclick={() => flush()} title={saveError}>{$t.noteSaveError(saveError)}</button>{/if}

  {#if calendarError}<p class="delete-error" role="alert">{calendarError}</p>{/if}

  {#if panel === "tint" && note}
    <TintPanel tint={note.tint} opacity={note.opacity} font={note.font ?? $noteFont} saving={fontSaving} onTint={setTint} onOpacity={setOpacity} onFont={setFont} />
  {:else if panel === "pin"}
    <PinPanel {pin} onPick={setPin} />
  {:else if panel === "delete"}
    <div class="panel" role="group" aria-label={$t.deleteNote}>
      <button class="item danger" disabled={deleting} onclick={remove}>
        <Trash size={15} strokeWidth={2} />
        <span>{$t.confirmDelete}<small>{$t.deleteHint}</small></span>
      </button>
      <button class="item" disabled={deleting} onclick={() => { panel = null; view?.focus(); }}>{$t.cancel}</button>
      {#if deleteError}<p class="delete-error" role="alert">{deleteError}</p>{/if}
    </div>
  {/if}

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="resize" onpointerdown={() => win.startResizeDragging("SouthEast")}></div>
</div>

<style>
  .note {
    --faint: rgb(31 37 43 / 0.42);
    --link: var(--accent);
    --hover: rgb(31 37 43 / 0.08);
    position: fixed;
    isolation: isolate;
    inset: 0;
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-window);
    color: var(--ink);
    background:
      linear-gradient(180deg, rgb(255 255 255 / 0.14), rgb(255 255 255 / 0) 34%),
      rgb(var(--tint) / var(--alpha));
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.32);
    overflow: hidden;
  }

  .note-background { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center; pointer-events: none; z-index: -1; }
  .format-toolbar { position: fixed; z-index: 30; display: flex; align-items: center; gap: 2px; max-width: calc(100% - 16px); padding: 4px; border-radius: 8px; color: #1f252b; background: #f8fafb; box-shadow: 0 0 0 1px rgb(31 37 43 / 0.12), 0 5px 16px rgb(0 0 0 / 0.22); }
  .dark .format-toolbar { background: #30373f; color: #eef1f4; }
  .format-toolbar button { width: 32px; height: 28px; flex-shrink: 1; padding: 0; border: 0; border-radius: 4px; background: transparent; font-size: 16px; }
  .format-toolbar button:hover, .format-toolbar button.pressed { background: var(--hover); }
  .format-toolbar .highlighter { font-weight: 650; text-decoration: underline; text-decoration-color: var(--highlight); text-decoration-thickness: 4px; text-underline-offset: 1px; }
  .save-error { margin: 4px 12px; padding: 0; border: 0; color: var(--danger); background: transparent; font-size: 12px; }

  .note.dark {
    --faint: rgb(238 241 244 / 0.46);
    --link: #9cc3de;
    --hover: rgb(238 241 244 / 0.12);
    color: var(--paper);
    background:
      linear-gradient(180deg, rgb(255 255 255 / 0.06), rgb(255 255 255 / 0) 34%),
      rgb(var(--tint) / var(--alpha));
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.1);
  }

  /* Without blur the wallpaper sits right behind the text, so the text gets a soft halo. */
  .note.clear {
    box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.18);
  }

  .note.clear .body {
    text-shadow:
      0 0 2px rgb(255 255 255 / 0.9),
      0 0 10px rgb(255 255 255 / 0.55);
  }

  .note.clear.dark .body {
    text-shadow:
      0 0 2px rgb(0 0 0 / 0.85),
      0 0 10px rgb(0 0 0 / 0.5);
  }

  /* New notes come into focus the way fogged glass clears. */
  .note.fresh {
    animation: clear-fog 260ms var(--ease-out) both;
  }

  @keyframes clear-fog {
    from {
      opacity: 0;
      filter: blur(8px);
    }
    to {
      opacity: 1;
      filter: blur(0);
    }
  }

  .bar {
    touch-action: none;
    position: relative;
    display: flex;
    align-items: center;
    height: 34px;
    padding: 0 6px;
    flex: none;
  }

  .grip {
    flex: 1;
    max-width: 32px;
    min-width: 0;
    height: 3px;
    margin: 0 auto;
    border-radius: 2px;
    background: currentColor;
    opacity: 0.16;
    transition: opacity 160ms;
  }

  .tools {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }

  .tool {
    display: grid;
    place-items: center;
    width: clamp(20px, 10vw, 26px);
    flex-shrink: 0;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    opacity: 0.62;
    transition:
      opacity 140ms,
      background-color 120ms;
  }

  .note:hover .tool,
  .note.active .tool,
  .tool:focus-visible,
  .tool.on {
    opacity: 0.72;
  }

  .note:hover .grip,
  .note.active .grip {
    opacity: 0.3;
  }

  .tool:hover,
  .tool[aria-expanded="true"] {
    opacity: 1 !important;
    background: var(--hover);
  }

  .tool.on {
    color: var(--link);
  }

  .danger:hover,
  .danger[aria-expanded="true"],
  .panel .danger,
  .delete-error {
    color: var(--danger);
  }

  .dark .danger:hover,
  .dark .panel .danger {
    color: #e59a94;
  }

  .delete-error {
    margin: 6px 8px;
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .conflict {
    margin: -4px 16px 6px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--faint);
  }

  .body {
    flex: 1;
    min-height: 0;
  }

  .body :global(.cm-editor) {
    user-select: text;
    cursor: text;
  }

  .body :global(.cm-lead) {
    font-size: calc(var(--note-font-size, 14.5px) + 1px);
    font-weight: max(620, var(--note-font-weight, 400));
    letter-spacing: -0.01em;
    padding-bottom: 2px;
  }

  .body :global(.cm-selectionBackground),
  .body :global(.cm-focused .cm-selectionBackground) {
    background: rgb(46 94 126 / 0.2) !important;
  }

  .dark .body :global(.cm-selectionBackground),
  .dark .body :global(.cm-focused .cm-selectionBackground) {
    background: rgb(156 195 222 / 0.28) !important;
  }

  .body :global(.cm-scroller::-webkit-scrollbar) {
    width: 8px;
  }

  .body :global(.cm-scroller::-webkit-scrollbar-thumb) {
    border-radius: 4px;
    border: 2px solid transparent;
    background: rgb(127 127 127 / 0.28) padding-box;
  }

  .resize {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 14px;
    height: 14px;
    cursor: nwse-resize;
  }
</style>
