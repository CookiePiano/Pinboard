<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, type Settings as AppSettings } from "./lib/ipc";
  import { language } from "./lib/i18n";
  import { noteFont } from "./lib/fonts";
  import { applyAppearance, watchTheme } from "./lib/appearance";

  const label = getCurrentWindow().label;
  const noteId = label.startsWith("note-") ? label.slice(5) : null;
  let ready = $state(false);

  $effect(() => { document.documentElement.lang = $language; });

  onMount(() => {
    let alive = true;
    let stop: (() => void) | undefined;
    const stopTheme = watchTheme();
    (async () => {
      // Listen before reading, so every window follows changes made in settings.
      stop = await listen<AppSettings>("settings://changed", ({ payload }) => {
        language.set(payload.language);
        noteFont.set(payload.noteFont);
        applyAppearance(payload);
      });
      if (!alive) { stop(); return; }
      const settings = await api.getSettings();
      if (!alive) return;
      language.set(settings.language);
      noteFont.set(settings.noteFont);
      applyAppearance(settings);
      ready = true;
    })();
    return () => { alive = false; stop?.(); stopTheme(); };
  });
</script>

{#if ready && noteId}
  {#await import("./lib/note/NoteWindow.svelte") then { default: NoteWindow }}
    <NoteWindow id={noteId} />
  {/await}
{:else if ready && label === "settings"}
  {#await import("./lib/settings/Settings.svelte") then { default: Settings }}
    <Settings />
  {/await}
{:else if ready && label === "calendar"}
  {#await import("./lib/calendar/CalendarWindow.svelte") then { default: CalendarWindow }}
    <CalendarWindow />
  {/await}
{/if}
