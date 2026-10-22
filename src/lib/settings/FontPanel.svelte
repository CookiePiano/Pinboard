<script lang="ts">
  import { RefreshCw } from "@lucide/svelte";
  import { tick, untrack } from "svelte";
  import { api, type FontFamily, type NoteFont, type Settings } from "../ipc";
  import { defaultNoteFont, fontFamilyCss } from "../fonts";
  import { fontText } from "./font-messages";

  let { settings, onSave, saving = false }: { settings: Settings; onSave: (s: Settings) => Promise<void>; saving?: boolean } = $props();
  let draft = $state<NoteFont>({ ...untrack(() => settings.noteFont) });
  let fonts = $state<FontFamily[]>([]);
  let search = $state("");
  let loading = $state(true);
  let scanError = $state<string | null>(null);
  let scanned = $state(false);
  let saved = $state(false);
  let scanId = 0;
  let scanLanguage = $derived(settings.language);

  const same = (a: NoteFont, b: NoteFont) => a.family === b.family && a.size === b.size && a.weight === b.weight;
  let dirty = $derived(!same(draft, settings.noteFont));
  let query = $derived(search.trim().toLocaleLowerCase());
  let matches = $derived(fonts.filter((font) => `${font.label} ${font.family}`.toLocaleLowerCase().includes(query)));
  let selected = $derived(fonts.find((font) => font.family === draft.family));
  let missing = $derived(scanned && draft.family !== null && !selected);
  let selectedOutsideSearch = $derived(draft.family !== null && !matches.some((font) => font.family === draft.family));

  async function scan(language = settings.language, force = false) {
    const id = ++scanId;
    loading = true;
    scanError = null;
    try {
      const result = await api.listFonts(language, force);
      if (id !== scanId) return;
      fonts = result;
      scanned = true;
    } catch (e) {
      if (id === scanId) scanError = String(e);
    } finally {
      if (id === scanId) loading = false;
    }
  }

  $effect(() => {
    const language = scanLanguage;
    untrack(() => { scan(language); });
    return () => { scanId++; };
  });

  // Follow confirmed settings, including changes received from another window.
  $effect(() => {
    const confirmed = settings.noteFont;
    draft = { ...confirmed };
  });

  function change(patch: Partial<NoteFont>) {
    draft = { ...draft, ...patch };
    saved = false;
  }

  function changeSize(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const value = input.valueAsNumber;
    if (Number.isFinite(value) && value >= 10 && value <= 36) change({ size: value });
    else input.value = String(draft.size);
  }

  async function apply() {
    const requested = { ...draft };
    saved = false;
    await onSave({ ...settings, noteFont: requested });
    await tick();
    saved = same(settings.noteFont, requested);
  }
</script>

<p class="lede">{$fontText.lede}</p>

<section class="group font-group">
  <div class="font-heading">
    <label for="font-family">{$fontText.installed}</label>
    <button class="btn" disabled={loading || saving} onclick={() => scan(settings.language, true)}>
      <RefreshCw size={13} strokeWidth={1.8} />{$fontText.scan}
    </button>
  </div>
  <div class="field font-fields">
    <input type="search" aria-label={$fontText.search} placeholder={$fontText.search} bind:value={search} disabled={saving} />
    <select id="font-family" value={draft.family ?? ""} disabled={loading || saving} onchange={(e) => change({ family: e.currentTarget.value || null })}>
      <option value="">{$fontText.defaultFont}</option>
      {#if selectedOutsideSearch}
        <option value={draft.family!}>{selected?.label ?? draft.family}</option>
      {/if}
      {#each matches as font (font.family)}
        <option value={font.family}>{font.label}</option>
      {/each}
    </select>
  </div>
  <p class="hint scan-status" role="status">
    {loading ? $fontText.scanning : query && matches.length === 0 ? $fontText.noResults : scanned ? $fontText.found(fonts.length) : ""}
  </p>
  {#if scanError}<p class="hint error" role="alert">{$fontText.scanError} {scanError}</p>{/if}
  {#if missing}<p class="hint" role="status">{$fontText.unavailable}</p>{/if}
</section>

<section class="group font-options" aria-label={$fontText.preview}>
  <div class="field">
    <label for="font-size">{$fontText.size} (px)</label>
    <input id="font-size" type="number" min="10" max="36" step="0.5" value={draft.size} disabled={saving} onchange={changeSize} />
  </div>
  <div class="field">
    <label for="font-weight">{$fontText.weight}</label>
    <select id="font-weight" value={draft.weight} disabled={saving} onchange={(e) => change({ weight: Number(e.currentTarget.value) })}>
      <option value={400}>{$fontText.regular}</option>
      <option value={500}>{$fontText.medium}</option>
      <option value={600}>{$fontText.semibold}</option>
      <option value={700}>{$fontText.bold}</option>
    </select>
  </div>
</section>

<section class="group font-group">
  <h2>{$fontText.preview}</h2>
  <div class="font-preview" style:font-family={fontFamilyCss(missing ? null : draft.family)} style:font-size="{draft.size}px" style:font-weight={draft.weight}>
    <p lang="ko">{$fontText.previewKo}</p>
    <p lang="en">{$fontText.previewEn}</p>
    <p>0123456789 !? @#</p>
  </div>
  <p class="hint">{$fontText.fallback}</p>
</section>

<div class="font-actions">
  <button class="btn primary" disabled={!dirty || saving} onclick={apply}>{$fontText.apply}</button>
  <button class="btn" disabled={saving || same(draft, defaultNoteFont)} onclick={() => change({ ...defaultNoteFont })}>{$fontText.reset}</button>
</div>
{#if saved && !dirty}<p class="hint" role="status">{$fontText.saved}</p>{/if}

<style>
  .lede { margin-bottom: 16px; }
  .font-group { margin-bottom: 18px; }
  .font-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 8px; }
  .font-heading label { font-size: 13px; font-weight: 650; color: var(--text-soft); }
  .font-fields { gap: 8px; }
  .font-fields input { width: 100%; min-width: 0; }
  select {
    width: 100%; min-width: 0; height: 34px; padding: 0 10px; border: 1px solid var(--rule);
    border-radius: var(--radius-control); font: inherit; font-size: 13px;
    color: var(--text); background: var(--field);
  }
  select:disabled, input:disabled { opacity: 0.55; }
  .scan-status { min-height: 18px; margin-bottom: 0; }
  .font-options { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 16px; margin-bottom: 18px; }
  .font-options input { width: 100%; min-width: 0; }
  .font-preview {
    padding: 12px 14px; border: 1px solid var(--rule); border-radius: var(--radius-control);
    background: var(--field); line-height: 1.6; overflow-wrap: anywhere; user-select: text;
  }
  .font-preview p { margin: 0; }
  .font-actions { display: flex; flex-wrap: wrap; gap: 10px; }
</style>
