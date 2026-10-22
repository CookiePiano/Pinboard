<script lang="ts">
  import { onMount } from "svelte";
  import { Check, Copy, Plug } from "@lucide/svelte";
  import { aiText } from "../mcp/i18n";
  import { configuration, configurationPaths, mcpApi, providers, type ClientStatus, type McpClient, type McpSettings } from "../mcp/ipc";
  import BrandIcon from "../mcp/BrandIcon.svelte";
  import Switch from "./Switch.svelte";
  import { api } from "../ipc";

  let settings = $state<McpSettings | null>(null);
  let selected = $state<McpClient[]>([]);
  let busy = $state(true);
  let error = $state<string | null>(null);
  let copied = $state<McpClient | null>(null);
  let alive = true;
  let copyTimer: ReturnType<typeof setTimeout> | undefined;
  let statuses = $state<ClientStatus[]>([]);
  let connecting = $state(false);
  let connected = $state<string | null>(null);
  const chosen = $derived(providers.filter((provider) => selected.includes(provider.id)));
  const registration = (client: McpClient) => statuses.find((status) => status.client === client)?.registration ?? "missing";
  const normalizeSelection = (ids: unknown[]) => providers.filter((p) => ids.includes(p.id) || (p.id === "claude" && ids.includes("claude_cli")) || (p.id === "antigravity" && ids.includes("gemini"))).map((p) => p.id);

  /** One click: set up MCP, allow connections, and write each chosen app's configuration. */
  async function connect() {
    if (busy || connecting || !chosen.length) return;
    connecting = true; error = null; connected = null;
    try {
      const result = await mcpApi.connect(chosen.map((provider) => provider.id));
      if (!alive) return;
      settings = result.settings;
      statuses = result.clients;
      const failed = new Set(result.errors.map((e) => e.client));
      const names = chosen.filter((provider) => !failed.has(provider.id)).map((provider) => provider.name);
      if (names.length) connected = $aiText.connectedRestart(names.join(", "));
      if (result.errors.length) {
        error = result.errors.map((e) => $aiText.clientError(providers.find((p) => p.id === e.client)?.name ?? e.client, e.message)).join("\n");
      }
    } catch (e) { if (alive) error = String(e) || $aiText.failed; }
    finally { if (alive) connecting = false; }
  }

  function select(client: McpClient, checked: boolean) {
    selected = checked ? [...selected.filter((id) => id !== client), client] : selected.filter((id) => id !== client);
    copied = null;
    try { localStorage.setItem("pinboard.mcp.providers", JSON.stringify(selected)); } catch { /* Selection still works without storage. */ }
    void api.saveTransferPreferences({ mcpProviders: selected }).catch(() => { error = $aiText.failed; });
  }

  async function load() {
    busy = true;
    error = null;
    try {
      const [next, preferences] = await Promise.all([mcpApi.getSettings(), api.getTransferPreferences()]);
      if (alive) {
        settings = next;
        if (preferences) selected = normalizeSelection(preferences.mcpProviders);
      }
    } catch { if (alive) error = $aiText.failed; }
    finally { if (alive) busy = false; }
    // Status badges are informative only; a missing or unreadable config simply shows as not connected.
    try { const next = await mcpApi.clientStatus(); if (alive) statuses = next; } catch { /* Keep the badges empty. */ }
  }

  async function toggle(enabled: boolean) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      const next = await mcpApi.setEnabled(enabled);
      if (alive) settings = next;
    } catch { if (alive) error = $aiText.failed; }
    finally { if (alive) busy = false; }
  }

  async function provision() {
    if (busy) return;
    busy = true; error = null;
    try { settings = await mcpApi.provision(); }
    catch { error = $aiText.failed; }
    finally { busy = false; }
  }

  async function copy(client: McpClient) {
    if (!settings) return;
    error = null;
    try {
      await navigator.clipboard.writeText(configuration(settings, client));
      if (!alive) return;
      copied = client;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = null), 2500);
    } catch { if (alive) error = $aiText.clipboardError; }
  }

  onMount(() => {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem("pinboard.mcp.providers") ?? "[]");
      if (Array.isArray(saved)) selected = normalizeSelection(saved);
    } catch { /* Ignore invalid or unavailable local preferences. */ }
    void load();
    return () => { alive = false; clearTimeout(copyTimer); };
  });
</script>

<p class="lede">{$aiText.lede}</p>

{#if settings}
  <section class="group">
    <div class="row">
      <div class="row-text">
        {$aiText.provision}
        <span class="hint">{settings.provisioned && settings.available ? $aiText.provisionedHint : $aiText.provisionHint}</span>
      </div>
      <button class="btn" disabled={busy || (settings.provisioned && settings.available)} onclick={provision}>
        {settings.provisioned && settings.available ? $aiText.provisioned : $aiText.provision}
      </button>
    </div>
    <div class="row">
      <div class="row-text" class:inactive={!settings.provisioned}>
        {$aiText.enable}
        <span class="hint">{!settings.provisioned ? $aiText.provisionHint : !settings.available ? $aiText.unavailable : settings.enabled ? $aiText.enabled : $aiText.disabled}</span>
      </div>
      <Switch checked={settings.enabled} label={$aiText.enable} disabled={busy || !settings.provisioned || !settings.available} onChange={toggle} />
    </div>
  </section>

  <section class="group">
    <h2>{$aiText.client}</h2>
    <p class="hint selection-hint">{$aiText.selectHint}</p>
    <fieldset class="providers" aria-label={$aiText.client} disabled={busy || connecting}>
      {#each providers as provider (provider.id)}
        {@const reg = registration(provider.id)}
        <label class="provider" class:selected={selected.includes(provider.id)}>
          <BrandIcon client={provider.id} />
          <input type="checkbox" aria-label={provider.name} checked={selected.includes(provider.id)} onchange={(event) => select(provider.id, event.currentTarget.checked)} />
          <span>{provider.name}</span>
          <small class="status {reg}">{$aiText[reg]}</small>
        </label>
      {/each}
    </fieldset>
    <div class="provider-hints">
      {#each providers as provider (provider.id)}<p class="hint">{$aiText[provider.hint]}</p>{/each}
    </div>
    {#if chosen.length === 0}<p class="hint selection-hint">{$aiText.selectEmpty}</p>{/if}
    <div class="auto">
      <button class="btn primary" disabled={busy || connecting || !chosen.length} onclick={connect}>
        <Plug size={15} strokeWidth={1.9} />{connecting ? $aiText.connecting : $aiText.autoConnect}
      </button>
      <span class="hint">{$aiText.autoConnectHint}</span>
    </div>
    {#if connected}<p class="hint done" role="status">{connected}</p>{/if}
  </section>

  {#if chosen.length > 0}
    <details class="group manual">
      <summary>{$aiText.manual}</summary>
      <p class="setup-copy">{$aiText.instruction}</p>
      {#each chosen as provider (provider.id)}
        <div class="connection">
          <div class="connection-heading">
            <h3>{provider.name}</h3>
            <button class="btn" aria-label={`${provider.name} ${$aiText.copy}`} onclick={() => copy(provider.id)} disabled={!settings.enabled || busy || !settings.available}>
              {#if copied === provider.id}<Check size={15} strokeWidth={1.9} />{:else}<Copy size={15} strokeWidth={1.9} />{/if}
              {$aiText.copy}
            </button>
          </div>
          <details>
            <summary aria-label={`${provider.name} ${$aiText.show}`}>{$aiText.show}</summary>
            <p class="hint route-hint">{$aiText[provider.hint]}</p>
            <p class="config-path">{configurationPaths[provider.id]}</p>
            <textarea class="configuration" aria-label={`${provider.name} ${$aiText.config}`} disabled={!settings.enabled} readonly value={configuration(settings, provider.id)} rows={8} spellcheck="false"></textarea>
          </details>
        </div>
      {/each}
      <span class="hint copy-status" role="status">{copied ? $aiText.copied : ""}</span>
      <p class="hint keep-open">{$aiText.keepOpen}</p>
    </details>
  {/if}

  <section class="group" class:inactive={!settings.enabled}>
    <h2>{$aiText.privacy}</h2>
    <p class="permissions">{$aiText.permissions}</p>
    <p class="hint">{$aiText.privacyHint}</p>
  </section>

  <section class="group" class:inactive={!settings.enabled}>
    <h2>{$aiText.exampleTitle}</h2>
    <p class="example">“{$aiText.example}”</p>
  </section>
{:else if busy}
  <p class="hint" role="status">{$aiText.loading}</p>
{:else}
  <button class="btn" onclick={load}>{$aiText.retry}</button>
{/if}

{#if error}<p class="hint error" role="alert">{error}</p>{/if}

<style>
  .group { margin-bottom: 20px; }
  .lede { margin-bottom: 16px; }
  .copy-status:empty { display: none; }
  .selection-hint { margin: 8px 0 12px; line-height: 1.6; }
  .providers { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; padding: 0; margin: 0; border: 0; min-width: 0; }
  .provider {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 7px 8px;
    min-width: 0;
    padding: 10px 12px;
    border: 1px solid var(--rule);
    border-radius: var(--radius-control);
    background: var(--field);
    cursor: pointer;
  }
  .provider:hover { background: var(--select); }
  .provider.selected { border-color: var(--accent); background: var(--accent-soft); }
  .provider:focus-within { outline: 2px solid var(--accent); outline-offset: 3px; }
  .provider input { width: 16px; height: 16px; padding: 0; margin: 0; accent-color: var(--accent); cursor: pointer; }
  .provider span { grid-column: 1 / -1; font-weight: 600; }
  .provider-hints { margin-top: 10px; }
  .provider-hints p { margin: 3px 0; }
  .connection { padding: 9px 0; border-bottom: 1px solid var(--rule); }
  .connection:last-of-type { border-bottom: 0; }
  .connection-heading { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  h3 { margin: 0; font-size: 13.5px; font-weight: 600; }
  .route-hint { margin: 8px 0 4px; line-height: 1.6; }
  .setup-copy, .permissions { margin: 10px 0 6px; line-height: 1.6; }
  .config-path { margin: 0 0 14px; color: var(--text-soft); font-size: 12px; overflow-wrap: anywhere; user-select: text; }
  .copy-status { min-height: 18px; margin: 5px 0; }
  .configuration {
    box-sizing: border-box;
    display: block;
    width: 100%;
    margin-top: 10px;
    padding: 10px;
    resize: vertical;
    border: 1px solid var(--rule);
    border-radius: var(--radius-control);
    background: var(--field);
    color: var(--text);
    font: 11.5px/1.6 "Cascadia Mono", Consolas, monospace;
    user-select: text;
    white-space: pre;
  }
  .keep-open { margin-top: 12px; line-height: 1.6; }
  .provider .status { grid-column: 1 / -1; margin-top: -4px; font-size: 11.5px; color: var(--text-faint); }
  .provider .status.registered { color: var(--accent); font-weight: 600; }
  .provider .status.outdated { color: var(--danger); }
  .auto { display: flex; align-items: center; gap: 12px; margin-top: 14px; }
  .auto .hint { margin: 0; line-height: 1.5; }
  .auto .btn { flex: none; }
  .done { margin: 10px 0 0; color: var(--accent); line-height: 1.5; }
  .manual > summary { margin-bottom: 4px; }
  .error { white-space: pre-line; }
  .example { margin: 10px 0; color: var(--text-soft); line-height: 1.65; user-select: text; }
</style>
