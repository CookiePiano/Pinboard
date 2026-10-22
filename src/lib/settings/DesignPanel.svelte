<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { ImagePlus, Trash } from "@lucide/svelte";
  import { api, type DesignSummary, type NoteDesign, type NotesChanged, type NoteBackground } from "../ipc";
  import { t } from "../i18n";
  import { isDark } from "../tints";
  import { darkMode } from "../appearance";
  import { designDirty, designSaveError, flushDesignSaves, queueDesignBackground } from "./design-saves";
  import Slider from "../Slider.svelte";

  let notes = $state<DesignSummary[]>([]);
  let selected = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let input = $state<HTMLInputElement>(null!);
  let note = $state<NoteDesign | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let alive = true;
  let loadVersion = 0;
  let detailVersion = 0;

  function title(item: DesignSummary) { return item.title.slice(0, 80) || $t.untitledNote; }
  async function selectNote(id: string) {
    selected = id;
    note = null;
    const version = ++detailVersion;
    if (!id) return;
    try {
      const result = await api.getNoteDesign(id);
      if (alive && version === detailVersion && selected === id && !designDirty(id)) note = result;
    } catch (e) { if (alive && version === detailVersion) error = String(e); }
  }
  async function load(target?: string | null, refreshDesign = true) {
    const version = ++loadVersion;
    try {
      const context = await api.designNoteSummaries();
      if (!alive || version !== loadVersion) return;
      notes = context.notes;
      const next = [target, selected, context.selected, notes[0]?.id].find((id) => id && notes.some((n) => n.id === id)) ?? "";
      if (next !== selected || (refreshDesign && !designDirty(next))) await selectNote(next);
      else if (note) {
        const summary = notes.find((n) => n.id === selected);
        if (summary) { note.title = summary.title; note.tint = summary.tint; }
      }
    } catch (e) { if (alive && version === loadVersion) error = String(e); }
  }
  async function flush() {
    clearTimeout(timer); timer = undefined;
    return flushDesignSaves();
  }
  function update(background: NoteBackground) {
    if (!note) return;
    error = null;
    note.background = background.image ? background : null;
    // Invalidate a response that began before this local edit.
    detailVersion++;
    queueDesignBackground(note.id, background);
    clearTimeout(timer);
    timer = setTimeout(() => { void flush(); }, 250);
  }
  async function attach(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file || !note) return;
    const target = note.id;
    busy = true; error = null;
    try {
      if (!["image/jpeg", "image/png", "image/webp"].includes(file.type)) throw new Error();
      const image = await createImageBitmap(file);
      const ratio = Math.min(1, 2560 / Math.max(image.width, image.height));
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, Math.round(image.width * ratio));
      canvas.height = Math.max(1, Math.round(image.height * ratio));
      canvas.getContext("2d")!.drawImage(image, 0, 0, canvas.width, canvas.height);
      image.close();
      const data = canvas.toDataURL("image/webp", 0.88);
      if (data.length > 8 * 1024 * 1024) throw new Error();
      if (alive && selected === target) { update({ image: data, opacity: note?.background?.opacity ?? 0.3 }); await flush(); }
    } catch { if (alive) error = $t.imageError; }
    finally { if (alive) { busy = false; input.value = ""; } }
  }
  onMount(() => {
    const stops: (() => void)[] = [];
    void (async () => {
      stops.push(await listen<string | null>("settings://note", async ({ payload }) => { if (!(await flush())) await load(payload); }));
      stops.push(await listen<NotesChanged>("notes://changed", ({ payload }) => {
        void load(undefined, payload.design && (payload.id === null || payload.id === selected));
      }));
      await load();
      if (!alive) stops.forEach((stop) => stop());
    })();
    return () => { alive = false; void flush(); stops.forEach((stop) => stop()); };
  });
</script>

<p class="lede">{$t.designLede}</p>
<section class="group">
  <label class="note-label" for="design-note">{$t.designNote}</label>
  <select id="design-note" class="note-select" value={selected} disabled={busy || !notes.length} onchange={async (e) => { const input = e.currentTarget; const value = input.value; if (!(await flush())) await selectNote(value); else input.value = selected; }}>
    {#each notes as item (item.id)}<option value={item.id}>{title(item)}</option>{/each}
  </select>
</section>
{#if note}
  <section class="group">
    <h2>{$t.preview}</h2>
    <div class="preview" class:dark={$darkMode || isDark(note.tint)} style:--preview-tint="var(--tint-{$darkMode ? 'dark-' : ''}{note.tint})">
      {#if note.background}<img src={note.background.image} alt="" style:opacity={note.background.opacity} />{/if}
      <span class="preview-grip"></span>
      <p>{title(note)}<small>{$t.designPreview}</small></p>
    </div>
  </section>
  <section class="group">
    <h2>{$t.backgroundImage}</h2>
    <input class="file-input" bind:this={input} type="file" accept="image/jpeg,image/png,image/webp" aria-label={$t.attachImage} onchange={attach} disabled={busy} />
    <div class="actions">
      <button class="btn" disabled={busy} onclick={() => input.click()}><ImagePlus size={15} />{note.background ? $t.replaceImage : $t.attachImage}</button>
      <button class="btn" disabled={busy || !note.background} onclick={() => { update({ image: "", opacity: 0.3 }); void flush(); }}><Trash size={15} />{$t.removeImage}</button>
    </div>
    <p class="hint">{$t.imageHint}</p>
  </section>
  <fieldset class="group dependent" disabled={!note.background || busy} class:inactive={!note.background}>
    <Slider label={$t.imageTransparency} value={1 - (note.background?.opacity ?? 0.3)} onInput={(value) => { if (note?.background) update({ ...note.background, opacity: 1 - value }); }} />
  </fieldset>
{:else}<p class="hint">{$t.noNotes}</p>{/if}
{#if error}<p class="hint error" role="alert">{error}</p>{/if}
{#if $designSaveError}<button class="hint error" onclick={() => flush()}>{$t.noteSaveError($designSaveError)}</button>{/if}

<style>
  .note-label { display: block; margin-bottom: 8px; color: var(--text-soft); font-weight: 600; }
  select { width: 100%; }
  .file-input { display: none; }
  .actions { display: flex; flex-wrap: wrap; gap: 10px; margin: 12px 0; }
  .preview { position: relative; isolation: isolate; overflow: hidden; height: 170px; margin-top: 12px; border-radius: 8px; background: rgb(var(--preview-tint)); color: var(--ink); box-shadow: inset 0 0 0 1px var(--rule); }
  .preview.dark { color: #eef1f4; }
  .preview img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: center; z-index: -1; }
  .preview-grip { display: block; width: 30px; height: 3px; margin: 14px auto; border-radius: 2px; background: currentColor; opacity: 0.25; }
  .preview p { margin: 24px; font-size: 16px; font-weight: 600; }
  .preview small { display: block; margin-top: 10px; font-size: 14px; font-weight: 400; opacity: 0.7; }
</style>
