<script lang="ts">
  import { t } from "../i18n";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { Cloud, Image, Info, Minus, SlidersHorizontal, Sparkles, Type, X } from "@lucide/svelte";
  import { api, type Settings, type SyncStatus } from "../ipc";
  import { aiText } from "../mcp/i18n";
  import { flushDesignSaves } from "./design-saves";
  import { describeStatus } from "./status";

  const win = getCurrentWebviewWindow();
  const sections = [
    { id: "general", label: "general", icon: SlidersHorizontal },
    { id: "fonts", label: "fontSettings", icon: Type },
    { id: "design", label: "design", icon: Image },
    { id: "sync", label: "sync", icon: Cloud },
    { id: "ai", label: "ai", icon: Sparkles },
    { id: "about", label: "about", icon: Info },
  ] as const;
  type Section = (typeof sections)[number]["id"];

  let section = $state<Section>("general");
  let settings = $state<Settings | null>(null);
  let status = $state<SyncStatus | null>(null);
  let now = $state(Date.now());

  let saveError = $state<string | null>(null);
  let saving = $state(false);
  let page: HTMLDivElement;
  let transferBusy = $state(false);
  async function save(next: Settings) {
    if (saving) return;
    saving = true;
    saveError = null;
    try { settings = await api.saveSettings(next); }
    catch (e) { saveError = String(e); }
    finally { saving = false; }
  }

  onMount(() => {
    const stops: (() => void)[] = [];
    const tick = setInterval(() => (now = Date.now()), 20_000);
    let resizeTimer: ReturnType<typeof setTimeout>;
    const observer = new ResizeObserver(() => {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => { if (page) void api.fitSettings(page.scrollHeight + 96).catch(() => {}); }, 80);
    });
    observer.observe(page);
    let closing = false;
    (async () => {
      stops.push(await win.onCloseRequested(async (event) => {
        if (closing) return;
        event.preventDefault();
        const error = await flushDesignSaves();
        saveError = error;
        if (!error) { closing = true; await win.close(); }
      }));
      stops.push(await win.listen<string>("archive://flush", async ({ payload }) => {
        transferBusy = true;
        const error = await flushDesignSaves();
        saveError = error;
        await api.editorReady(payload, error);
      }));
      stops.push(await win.listen("archive://release", () => { transferBusy = false; }));
      [settings, status] = await Promise.all([api.getSettings(), api.getSyncStatus()]);
      stops.push(await listen<SyncStatus>("sync://status", (e) => (status = e.payload)));
      stops.push(await listen<Settings>("settings://changed", (e) => (settings = e.payload)));
      await api.windowReady();
    })();
    return () => {
      clearInterval(tick);
      clearTimeout(resizeTimer);
      observer.disconnect();
      stops.forEach((s) => s());
    };
  });

  let current = $derived(sections.find((s) => s.id === section)!);
  let summary = $derived(status ? describeStatus(status, now, $t) : null);
</script>

<div class="shell" inert={transferBusy}>
  <nav class="side" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <img src="/pinboard.svg" alt="" width="22" height="22" />
      Pinboard
    </div>
    <ul>
      {#each sections as s (s.id)}
        <li>
          <button class:current={s.id === section} aria-current={s.id === section ? "page" : undefined} onclick={() => (section = s.id)}>
            <s.icon size={16} strokeWidth={1.9} />
            {s.label === "ai" ? $aiText.title : $t[s.label]}
          </button>
        </li>
      {/each}
    </ul>
    {#if summary}
      <button class="status" data-tone={summary.tone} onclick={() => (section = "sync")}>
        <span class="dot"></span>
        {summary.text}
      </button>
    {/if}
  </nav>

  <main>
    <header data-tauri-drag-region>
      <h1 data-tauri-drag-region>{current.label === "ai" ? $aiText.title : $t[current.label]}</h1>
      <div class="controls">
        <button aria-label={$t.minimize} onclick={() => win.minimize()}><Minus size={16} strokeWidth={1.8} /></button>
        <button aria-label={$t.close} class="close" onclick={() => win.close()}><X size={16} strokeWidth={1.8} /></button>
      </div>
    </header>
    <div class="content">
      <div class="page" bind:this={page}>
      {#if saveError}<p class="hint error" role="alert">{$t.saveError(saveError)}</p>{/if}
      {#if settings && status}
        {#if section === "general"}
          {#await import("./GeneralPanel.svelte") then { default: GeneralPanel }}
          <GeneralPanel {settings} {saving} onSave={save} />
          {/await}
        {:else if section === "fonts"}
          {#await import("./FontPanel.svelte") then { default: FontPanel }}
          <FontPanel {settings} {saving} onSave={save} />
          {/await}
        {:else if section === "design"}
          {#await import("./DesignPanel.svelte") then { default: DesignPanel }}
          <DesignPanel />
          {/await}
        {:else if section === "sync"}
          {#await import("./SyncPanel.svelte") then { default: SyncPanel }}
          <SyncPanel {settings} {status} {now} onSave={save} />
          {/await}
        {:else if section === "ai"}
          {#await import("./AiPanel.svelte") then { default: AiPanel }}
          <AiPanel />
          {/await}
        {:else}
          {#await import("./AboutPanel.svelte") then { default: AboutPanel }}
          <AboutPanel />
          {/await}
        {/if}
      {/if}
      </div>
    </div>
  </main>
</div>

<style>
  .shell {
    --text: var(--ink);
    --text-soft: var(--ink-soft);
    --text-faint: var(--ink-faint);
    --rule: rgb(31 37 43 / 0.08);
    --raised: #fff;
    --field: rgb(255 255 255 / 0.75);
    --side: rgb(229 235 240 / 0.5);
    --surface: rgb(249 250 251 / 0.88);
    --select: rgb(31 37 43 / 0.07);

    display: grid;
    grid-template-columns: 208px minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    height: 100%;
    color: var(--text);
    font-size: 13.5px;
    border-radius: var(--radius-window);
    overflow: hidden;
  }

    :global([data-theme="dark"]) .shell {
      --text: #e9edf1;
      --text-soft: #b5bfc8;
      --text-faint: #87929c;
      --rule: rgb(255 255 255 / 0.08);
      --raised: rgb(255 255 255 / 0.12);
      --field: rgb(0 0 0 / 0.22);
      --side: rgb(27 31 36 / 0.55);
      --surface: rgb(32 37 42 / 0.9);
      --select: rgb(255 255 255 / 0.08);
      --accent: #8fbadb;
      --accent-soft: rgb(143 186 219 / 0.18);
      color-scheme: dark;
  }

  .side {
    display: flex;
    flex-direction: column;
    padding: 14px 10px 12px;
    background: var(--side);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 32px;
    padding: 0 8px;
    margin-bottom: 18px;
    font-size: 14.5px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  .brand img {
    pointer-events: none;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
  }

  ul button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-soft);
    text-align: left;
  }

  ul button:hover {
    color: var(--text);
    background: var(--select);
  }

  ul button.current {
    color: var(--text);
    font-weight: 600;
    background: var(--select);
  }

  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: auto;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-soft);
    font-size: 12.5px;
    text-align: left;
  }

  .status:hover {
    background: var(--select);
  }

  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: var(--text-faint);
  }

  [data-tone="ok"] .dot {
    background: #3f8f6b;
  }
  [data-tone="busy"] .dot {
    background: var(--accent);
  }
  [data-tone="error"] .dot {
    background: var(--danger);
  }

  main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface);
  }

  header {
    display: flex;
    align-items: center;
    height: 56px;
    padding: 0 8px 0 32px;
    flex: none;
  }

  h1 {
    margin: 6px 0 0;
    font-size: 19px;
    font-weight: 650;
    letter-spacing: -0.02em;
  }

  .controls {
    display: flex;
    margin-left: auto;
    align-self: flex-start;
    margin-top: 8px;
  }

  .controls button {
    display: grid;
    place-items: center;
    width: 34px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-soft);
  }

  .controls button:hover {
    background: var(--select);
    color: var(--text);
  }

  .controls .close:hover {
    background: #c42b1c;
    color: #fff;
  }

  .content {
    min-height: 0;
    flex: 1;
    overflow: auto;
    padding: 4px 32px 32px;
  }

  .page { display: flow-root; }

  .content::-webkit-scrollbar {
    width: 10px;
  }

  .content::-webkit-scrollbar-thumb {
    border-radius: 5px;
    border: 3px solid transparent;
    background: rgb(127 127 127 / 0.3) padding-box;
  }
</style>
