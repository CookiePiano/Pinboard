<script lang="ts">
  import { t } from "../i18n";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Download, Upload, FolderOpen, RefreshCw } from "@lucide/svelte";
  import { api, type Settings, type SyncStatus } from "../ipc";
  import Segmented from "./Segmented.svelte";
  import { describeStatus } from "./status";

  let {
    settings,
    status,
    now,
    onSave,
  }: { settings: Settings; status: SyncStatus; now: number; onSave: (s: Settings) => Promise<void> } = $props();

  type StorageMode = "local" | "cloud";
  let kinds: { id: StorageMode; label: string }[] = $derived([
    { id: "local", label: $t.thisPc },
    { id: "cloud", label: $t.cloud },
  ]);

  let suggestion = $state<string | null>(null);
  let cloudSetup = $state(false);
  let saving = $state(false);
  let pickingFolder = $state(false);
  let actionError = $state<string | null>(null);
  let transferBusy = $state<"export" | "import" | null>(null);
  let transferMessage = $state<string | null>(null);
  let transferError = $state<string | null>(null);
  let transferWarnings = $state<string[]>([]);
  let mode: StorageMode = $derived(settings.sync === "folder" || cloudSetup ? "cloud" : "local");

  onMount(async () => {
    // A missing suggestion must never prevent choosing a folder manually.
    try { suggestion = await api.suggestFolder(); }
    catch { suggestion = null; }
  });

  async function save(next: Settings) {
    saving = true;
    actionError = null;
    try { await onSave(next); }
    catch (e) { actionError = String(e); }
    finally { saving = false; }
  }

  async function selectMode(next: StorageMode) {
    if (saving || pickingFolder || transferBusy) return;
    cloudSetup = next === "cloud";
    actionError = null;
    if (next === "local") await save({ ...settings, sync: "off" });
    else if (settings.folderRoot) await save({ ...settings, sync: "folder" });
    // Until a cloud folder is chosen, notes continue to be saved on this PC.
  }

  async function connectFolder(folderRoot: string) {
    if (saving) return;
    await save({ ...settings, sync: "folder", folderRoot });
  }

  async function pickFolder() {
    if (saving || pickingFolder) return;
    pickingFolder = true;
    actionError = null;
    try {
      const dir = await open({ directory: true, title: $t.pickSyncFolder, defaultPath: settings.folderRoot ?? suggestion ?? undefined });
      if (typeof dir === "string") await connectFolder(dir);
    } catch (e) { actionError = String(e); }
    finally { pickingFolder = false; }
  }

  async function syncNow() {
    actionError = null;
    try { await api.syncNow(); }
    catch (e) { actionError = String(e); }
  }

  async function preferences() {
    const stored = await api.getTransferPreferences();
    if (stored) return stored;
    try {
      const values: unknown = JSON.parse(localStorage.getItem("pinboard.mcp.providers") ?? "[]");
      return { mcpProviders: Array.isArray(values) ? values.filter((v): v is string => typeof v === "string" && ["codex", "claude", "gemini"].includes(v)) : [] };
    } catch { return { mcpProviders: [] }; }
  }

  async function transfer(kind: "export" | "import") {
    if (transferBusy || saving || pickingFolder) return;
    transferBusy = kind; transferMessage = transferError = null; transferWarnings = [];
    try {
      if (kind === "export") {
        transferMessage = $t.exportDone(await api.exportArchive(await preferences()));
      } else {
        const path = await open({ directory: false, multiple: false, title: $t.importTitle, filters: [{ name: "Pinboard ZIP", extensions: ["zip"] }] });
        if (typeof path !== "string") return;
        const result = await api.importArchive(path, await preferences());
        try { localStorage.setItem("pinboard.mcp.providers", JSON.stringify(result.preferences.mcpProviders)); } catch { /* The restored notes and settings remain available. */ }
        cloudSetup = false;
        transferMessage = $t.importDone(result.noteCount, result.backupPath);
        transferWarnings = result.warnings;
      }
    } catch (e) { transferError = String(e); }
    finally { transferBusy = null; }
  }

  let summary = $derived(describeStatus(status, now, $t));
  let busy = $derived(status.phase === "syncing");
</script>

<p class="lede">
  {$t.syncLede}
</p>

<section class="group">
  <h2>{$t.syncMethod}</h2>
  <div class="row storage-choice">
    <fieldset disabled={saving || pickingFolder || !!transferBusy}>
      <Segmented label={$t.syncMethod} value={mode} options={kinds} onChange={selectMode} />
    </fieldset>
  </div>

  <div class="transfer">
    <h3>{$t.transfer}</h3>
    <p class="hint">{$t.transferHint}</p>
    <p class="hint">{$t.importingHint}</p>
    <div class="transfer-actions">
      <button class="btn" disabled={!!transferBusy || saving || pickingFolder} onclick={() => transfer("export")}><Download size={15} />{transferBusy === "export" ? $t.exportBusy : $t.exportNotes}</button>
      <button class="btn" disabled={!!transferBusy || saving || pickingFolder} onclick={() => transfer("import")}><Upload size={15} />{transferBusy === "import" ? $t.importBusy : $t.importNotes}</button>
    </div>
    {#if transferMessage}<p class="hint receipt" role="status">{transferMessage}</p>{/if}
    {#if transferError}<p class="hint error" role="alert">{transferError}</p>{/if}
    {#each transferWarnings as warning}<p class="hint" role="status">{warning}</p>{/each}
  </div>

  {#if mode === "local"}
    <p class="hint note">{$t.localStorageHint}</p>
  {:else}
    <p class="hint note">{$t.cloudStorageHint}</p>
    <div class="row">
      <div class="row-text">
        {$t.syncFolder}
        {#if settings.folderRoot}
          <span class="path" title={settings.folderRoot}>{settings.folderRoot}</span>
        {:else}
          <span class="hint">{$t.noFolder}</span>
        {/if}
      </div>
      <button class="btn" disabled={saving || pickingFolder} onclick={pickFolder}><FolderOpen size={15} strokeWidth={1.9} />{settings.folderRoot ? $t.changeFolder : $t.pickFolder}</button>
    </div>
    {#if suggestion && settings.folderRoot !== suggestion}
      <div class="row">
        <div class="row-text">
          {$t.driveFolderFound}
          <span class="path">{suggestion}</span>
        </div>
        <button class="btn" disabled={saving || pickingFolder} onclick={() => connectFolder(suggestion!)}>{$t.useFolder}</button>
      </div>
    {/if}
    <p class="hint note">
      {$t.folderHint}
    </p>
  {/if}
  {#if actionError}<p class="hint error" role="alert">{actionError}</p>{/if}
</section>

{#if mode === "cloud" && settings.sync === "folder" && settings.folderRoot}
  <section class="group">
    <h2>{$t.status}</h2>
    <div class="row">
      <div class="row-text">
        {summary.text}
        {#if status.message}
          <span class="hint" class:error={status.phase === "error"}>{status.message}</span>
        {/if}
      </div>
      <button class="btn" disabled={busy || saving || pickingFolder || status.kind !== "folder"} onclick={syncNow}>
        <span class="spin" class:on={busy}><RefreshCw size={15} strokeWidth={1.9} /></span>
        {$t.syncNow}
      </button>
    </div>
  </section>
{/if}

<style>
  .transfer { margin: 18px 0 22px; padding: 18px 0; border-block: 1px solid var(--rule); }
  .storage-choice { border-bottom: 0; }
  h3 { margin: 0 0 8px; font-size: 13px; font-weight: 650; color: var(--text-soft); }
  .transfer p { line-height: 1.55; margin: 4px 0; }
  .transfer-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 13px; }
  .receipt { overflow-wrap: anywhere; user-select: text; }
  fieldset {
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  .note {
    max-width: 520px;
    margin: 10px 0 0;
    line-height: 1.6;
  }

  .spin {
    display: inline-grid;
  }

  .spin.on {
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
