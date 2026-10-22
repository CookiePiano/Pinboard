<script lang="ts">
  import { t, language } from "../i18n";
  import { api, type FontFamily, type NoteFont, type Tint } from "../ipc";
  import { selectionTools } from "../appearance";
  import { fontText } from "../settings/font-messages";
  import { TINTS } from "../tints";
  import Slider from "../Slider.svelte";

  let {
    tint,
    opacity,
    font,
    saving = false,
    onTint,
    onOpacity,
    onFont,
  }: { tint: Tint; opacity: number; font: NoteFont; saving?: boolean; onTint: (t: Tint) => void; onOpacity: (o: number) => void; onFont: (patch: Partial<NoteFont>) => Promise<void> } = $props();

  let name = $derived($t[tint]);
  let fonts = $state<FontFamily[]>([]);
  let loading = $state(true);
  let scanError = $state<string | null>(null);
  let scanVersion = 0;
  let sizePt = $derived(Math.round(font.size * 0.75 * 1000) / 1000);

  async function scan(lang = $language) {
    const version = ++scanVersion;
    loading = true; scanError = null;
    try { const result = await api.listFonts(lang); if (version === scanVersion) fonts = result; }
    catch (e) { if (version === scanVersion) scanError = String(e); }
    finally { if (version === scanVersion) loading = false; }
  }
  $effect(() => { void scan($language); return () => { scanVersion++; }; });

  function changeSize(value: number) {
    if (Number.isFinite(value)) void onFont({ size: Math.min(96, Math.max(6, value)) / 0.75 });
  }
  function enterSize(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    if (input.value.trim() && input.checkValidity()) changeSize(input.valueAsNumber);
    else input.value = String(sizePt);
  }
</script>

<div class="panel" role="dialog" aria-label={$t.colorTransparency}>
  <div class="head">
    <span>{$t.color}</span>
    <span class="value">{name}</span>
  </div>
  <div class="swatches" role="radiogroup" aria-label={$t.color}>
    {#each TINTS as tintOption (tintOption.id)}
      <button
        role="radio"
        aria-checked={tintOption.id === tint}
        aria-label={$t[tintOption.id]}
        title={$t[tintOption.id]}
        style:--c="var(--tint-{tintOption.id})"
        onclick={() => onTint(tintOption.id)}
      ></button>
    {/each}
  </div>
  <Slider label={$t.transparency} value={1 - opacity} onInput={(v) => onOpacity(1 - v)} />
  <div class="font-controls">
    <label for="note-font-family">{$fontText.installed}</label>
    <select id="note-font-family" value={font.family ?? ""} disabled={loading || saving} onchange={(e) => onFont({ family: e.currentTarget.value || null })}>
      <option value="">{$fontText.defaultFont}</option>
      {#if font.family && !fonts.some((item) => item.family === font.family)}<option value={font.family}>{font.family}</option>{/if}
      {#each fonts as item (item.family)}<option value={item.family}>{item.label}</option>{/each}
    </select>
    {#if loading}<span class="font-hint" role="status">{$fontText.scanning}</span>{/if}
    {#if scanError}<span class="font-hint" role="alert">{$fontText.scanError} <button onclick={() => scan()} title={scanError}>{$fontText.scan}</button></span>{/if}
    <label for="note-font-size">{$fontText.size} (pt)</label>
    <div class="size-controls">
      <button disabled={saving || sizePt <= 6} aria-label={$t.smaller} title={`${$t.smaller} (${$selectionTools.sizeStep}pt)`} onclick={() => changeSize(sizePt - $selectionTools.sizeStep)}>−</button>
      <input id="note-font-size" type="number" min="6" max="96" step="any" value={sizePt} disabled={saving} onchange={enterSize} onkeydown={(e) => { if (e.key === "Enter") { enterSize(e); e.currentTarget.blur(); } }} />
      <button disabled={saving || sizePt >= 96} aria-label={$t.larger} title={`${$t.larger} (${$selectionTools.sizeStep}pt)`} onclick={() => changeSize(sizePt + $selectionTools.sizeStep)}>+</button>
    </div>
    <span class="font-hint">{$fontText.noteScope}</span>
  </div>
</div>

<style>
  .font-controls { display: grid; gap: 6px; margin-top: 12px; padding-top: 10px; border-top: 1px solid rgb(127 137 145 / 0.25); }
  .font-controls select, .font-controls input, .size-controls button { min-width: 0; width: 100%; height: 30px; border: 1px solid rgb(127 137 145 / 0.4); border-radius: var(--radius-control); color: inherit; background: transparent; font: inherit; }
  .font-controls select { padding: 0 6px; }
  .font-controls option { color: var(--ink); background: #f8fafb; }
  .size-controls { display: grid; grid-template-columns: 30px minmax(0, 1fr) 30px; gap: 6px; }
  .size-controls input { text-align: center; padding: 0 4px; appearance: textfield; }
  .size-controls input::-webkit-inner-spin-button { appearance: none; }
  .size-controls button { font-size: 18px; }
  .size-controls button:hover { background: var(--hover); }
  .font-controls :disabled { opacity: 0.5; }
  .font-hint { font-size: 11px; opacity: 0.7; line-height: 1.4; }
  .font-hint button { padding: 0; border: 0; color: inherit; background: transparent; text-decoration: underline; }
  .head {
    display: flex;
    justify-content: space-between;
    margin: 0 2px 8px;
  }

  .value {
    opacity: 0.6;
  }

  .swatches {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 6px;
    margin-bottom: 14px;
  }

  .swatches button {
    aspect-ratio: 1;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: rgb(var(--c));
    box-shadow: inset 0 0 0 1px rgb(31 37 43 / 0.14);
    transition: transform 120ms var(--ease-out);
  }

  .swatches button:hover {
    transform: scale(1.08);
  }

  .swatches button[aria-checked="true"] {
    box-shadow:
      inset 0 0 0 1px rgb(31 37 43 / 0.14),
      0 0 0 2px rgb(248 250 251),
      0 0 0 3.5px var(--accent);
  }

  :global(.dark) .swatches button[aria-checked="true"] {
    box-shadow:
      inset 0 0 0 1px rgb(255 255 255 / 0.2),
      0 0 0 2px rgb(34 39 44),
      0 0 0 3.5px #9cc3de;
  }
</style>
