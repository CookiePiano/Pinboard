<script lang="ts">
  import { t } from "../i18n";
  import { onMount } from "svelte";
  import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
  import { api, type Language, type Settings } from "../ipc";
  import Switch from "./Switch.svelte";
  import Segmented from "./Segmented.svelte";
  import { Check } from "@lucide/svelte";
  import { captureShortcut } from "./shortcut-capture";

  const highlightPresets = [
    { color: "#ffe066", label: "highlightYellow" }, { color: "#a8e6cf", label: "highlightGreen" },
    { color: "#a8d8ff", label: "highlightBlue" }, { color: "#ffb3c7", label: "highlightPink" },
    { color: "#d4b5ff", label: "highlightPurple" }, { color: "#ffc078", label: "highlightOrange" },
  ] as const;
  const languages: { id: Language; label: string }[] = [
    { id: "ko", label: "한국어" }, { id: "en", label: "English" }, { id: "zh", label: "简体中文" },
    { id: "ja", label: "日本語" }, { id: "fr", label: "Français" }, { id: "es", label: "Español" },
  ];

  let { settings, onSave, saving = false }: { settings: Settings; onSave: (s: Settings) => Promise<void>; saving?: boolean } = $props();
  let autostart = $state<boolean | null>(null);
  let autostartError = $state<string | null>(null);
  let recording = $state(false);
  let shortcutError = $state(false);

  onMount(() => {
    isEnabled().then((enabled) => { autostart = enabled; }).catch((e) => { autostartError = String(e); });
    return () => { api.setShortcutRecording(false); };
  });

  async function setRecording(value: boolean) {
    await api.setShortcutRecording(value);
    recording = value;
    shortcutError = false;
  }

  async function setAutostart(on: boolean) {
    autostartError = null;
    autostart = null;
    try {
      await (on ? enable() : disable());
      autostart = await isEnabled();
    } catch (e) {
      autostartError = String(e);
      autostart = await isEnabled().catch(() => null);
    }
  }

  function displayShortcut(value: string) {
    return value.split("+").map((key) => key.replace(/^Key|^Digit/, "").replace(/^Super$/, "Win")).join(" + ");
  }

  async function capture(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    const captured = captureShortcut(e);
    if (captured.kind === "cancel") { await setRecording(false); return; }
    if (captured.kind === "ignore") return;
    if (captured.kind === "invalid") {
      shortcutError = true;
      return;
    }
    if (captured.kind !== "shortcut") return;
    await setRecording(false);
    shortcutError = false;
    await onSave({ ...settings, newNoteShortcut: captured.value });
  }

  function saveTools(patch: Partial<Settings["selectionTools"]>) {
    if (!saving) void onSave({ ...settings, selectionTools: { ...settings.selectionTools, ...patch } });
  }
</script>

<svelte:window onkeydowncapture={capture} onblur={() => { setRecording(false); }} />

<section class="group">
  <h2>{$t.appearance}</h2>
  <div class="row">
    <Segmented label={$t.appearance} value={settings.theme} options={[{ id: "system", label: $t.systemTheme }, { id: "light", label: $t.lightTheme }, { id: "dark", label: $t.darkTheme }]} onChange={(theme) => { if (!saving) onSave({ ...settings, theme }); }} />
  </div>
</section>

<section class="group">
  <h2>{$t.language}</h2>
  <div class="row">
    <!-- Language names stay in their own language so anyone can find theirs. -->
    <select class="note-select" aria-label={$t.language} value={settings.language} disabled={saving} onchange={(e) => onSave({ ...settings, language: e.currentTarget.value as Language })}>
      {#each languages as option (option.id)}<option value={option.id} lang={option.id}>{option.label}</option>{/each}
    </select>
  </div>
</section>

<section class="group">
  <h2>{$t.startup}</h2>
  <div class="row">
    <div class="row-text">
      {$t.autostart}
      <span class="hint" class:error={autostartError}>
        {autostartError ? $t.autostartError(autostartError) : $t.autostartHint}
      </span>
    </div>
    <Switch checked={autostart ?? false} disabled={autostart === null} label={$t.autostart} onChange={setAutostart} />
  </div>
</section>

<section class="group">
  <h2>{$t.shortcuts}</h2>
  <div class="row">
    <div class="row-text">
      {$t.enableShortcuts}
      <span class="hint">{$t.shortcutHint}</span>
    </div>
    <Switch checked={settings.shortcutsEnabled} disabled={saving} label={$t.enableShortcuts} onChange={async (shortcutsEnabled) => { await setRecording(false); onSave({ ...settings, shortcutsEnabled }); }} />
  </div>
    <div class="row shortcut-row" class:inactive={!settings.shortcutsEnabled}>
      <div class="row-text">{$t.newNote}<kbd>{displayShortcut(settings.newNoteShortcut)}</kbd></div>
      <button class="btn" class:recording disabled={saving || !settings.shortcutsEnabled} aria-pressed={recording} onclick={() => setRecording(!recording)}>
        {recording ? $t.shortcutRecording : $t.shortcutRecord}
      </button>
    </div>
    {#if recording || shortcutError}
      <p class="hint" class:error={shortcutError} role="status">{shortcutError ? $t.shortcutInvalid : $t.shortcutRecordHint}</p>
    {/if}
</section>

<section class="group">
  <h2>{$t.selectionTools}</h2>
  <fieldset class="dependent" disabled={saving}>
    <div class="row">
      <label class="row-text" for="size-step">{$t.sizeStep}</label>
      <input id="size-step" class="number-input" type="number" min="0.5" max="12" step="0.5" value={settings.selectionTools.sizeStep} onchange={(e) => { if (e.currentTarget.checkValidity()) saveTools({ sizeStep: Number(e.currentTarget.value) }); else e.currentTarget.value = String(settings.selectionTools.sizeStep); }} />
    </div>
    <div class="row color-row">
      <label class="row-text" for="highlight-color">{$t.highlightColor}</label>
      <div class="colors" role="group" aria-label={$t.highlightColor}>
        {#each highlightPresets as preset}
          <button class="swatch" style:background={preset.color} aria-label={$t[preset.label]} title={$t[preset.label]} aria-pressed={settings.selectionTools.highlightColor.toLowerCase() === preset.color} onclick={() => saveTools({ highlightColor: preset.color })}>
            {#if settings.selectionTools.highlightColor.toLowerCase() === preset.color}<Check size={15} strokeWidth={2.2} />{/if}
          </button>
        {/each}
        <input id="highlight-color" type="color" aria-label={$t.highlightCustom} title={$t.highlightCustom} value={settings.selectionTools.highlightColor} onchange={(e) => saveTools({ highlightColor: e.currentTarget.value })} />
      </div>
    </div>
  </fieldset>
</section>

<style>
  kbd {
    display: block;
    width: fit-content;
    margin-top: 5px;
    font-size: 12px;
    color: var(--text-soft);
  }

  .shortcut-row {
    flex-wrap: wrap;
    gap: 10px;
  }

  .recording {
    box-shadow: 0 0 0 1.5px var(--accent);
    color: var(--accent);
  }

  .color-row { flex-wrap: wrap; }
  .colors { display: flex; align-items: center; flex-wrap: wrap; gap: 7px; }
  .swatch { display: grid; place-items: center; width: 27px; height: 27px; border: 1px solid rgb(31 37 43 / 0.2); border-radius: 50%; color: #1f252b; padding: 0; }
  .swatch[aria-pressed="true"] { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
