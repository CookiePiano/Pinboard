<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { ArrowUpToLine, CalendarDays, ChevronLeft, ChevronRight, Monitor, Pin, StickyNote, Trash, X } from "@lucide/svelte";
  import { api, type PinMode } from "../ipc";
  import { dragWindow } from "../window-drag";
  import { language, t } from "../i18n";
  import PinPanel from "../note/PinPanel.svelte";
  import { calendarApi, type CalendarNote, type CalendarPreview } from "./api";
  import { darkMode } from "../appearance";
  import { isDark } from "../tints";
  import { calendarText as text } from "./messages";
  import { countDays, dateKey, dayRange, monthDays } from "./dates";

  const win = getCurrentWebviewWindow();
  const initial = new Date();
  let now = $state(initial);
  let month = $state(new Date(initial.getFullYear(), initial.getMonth(), 1));
  let selected = $state<string | null>(null);
  let selectedDate = $state<Date | null>(null);
  let dayNotes = $state<CalendarPreview[]>([]);
  let dayLoading = $state(false);
  let requestVersion = 0;
  let notes = $state<CalendarNote[]>([]);
  let loading = $state(true);
  let opening = $state<string | null>(null);
  let error = $state<string | null>(null);
  let dayError = $state<string | null>(null);
  let loadError = $state<string | null>(null);
  let refreshing = false;
  let refreshAgain = false;
  let refreshTimer: ReturnType<typeof setTimeout>;
  function scheduleRefresh() { clearTimeout(refreshTimer); refreshTimer = setTimeout(() => { void refresh(); }, 100); }
  let alive = true;
  let checked = $state<string[]>([]);
  let confirming = $state(false);
  let deleting = $state(false);
  let pin = $state<PinMode>("normal");
  let pinOpen = $state(false);
  const pinIcon = { normal: Pin, top: ArrowUpToLine, desktop: Monitor };
  const PinIcon = $derived(pinIcon[pin]);
  const pinTitle = $derived($t.position({ normal: $t.pinNormal, top: $t.pinTop, desktop: $t.pinDesktop }[pin]));
  const counts = $derived(countDays(notes));
  const days = $derived(monthDays(month));
  const locale = $derived(({ ko: "ko-KR", en: "en-US", zh: "zh-CN", ja: "ja-JP", fr: "fr-FR", es: "es-ES" })[$language]);
  const monthTitle = $derived(new Intl.DateTimeFormat(locale, { year: "numeric", month: "long" }).format(month));
  const weekdays = $derived(Array.from({ length: 7 }, (_, i) =>
    new Intl.DateTimeFormat(locale, { weekday: "short" }).format(new Date(2026, 0, 4 + i))));

  async function refresh() {
    if (refreshing) { refreshAgain = true; return; }
    refreshing = true;
    try {
      const current = await calendarApi.notes();
      if (alive) { notes = current; loadError = null; }
      if (alive && selectedDate) await loadDay(selectedDate, false);
    } catch (e) {
      if (alive) loadError = String(e);
    } finally {
      refreshing = false;
      if (alive) loading = false;
      if (alive && refreshAgain) { refreshAgain = false; void refresh(); }
    }
  }

  function moveMonth(delta: number) {
    month = new Date(month.getFullYear(), month.getMonth() + delta, 1);
    selected = null; selectedDate = null; dayNotes = []; dayLoading = false; requestVersion++;
    checked = []; confirming = false;
    error = null; dayError = null;
  }

  function showDay(day: Date) {
    if (loading) return;
    error = null;
    if (selected !== dateKey(day)) { checked = []; confirming = false; }
    selected = dateKey(day);
    selectedDate = day;
    month = new Date(day.getFullYear(), day.getMonth(), 1);
    dayNotes = [];
    void loadDay(day);
  }

  async function loadDay(day: Date, showLoading = true) {
    const version = ++requestVersion;
    if (showLoading) dayLoading = true;
    dayError = null;
    const { start, end } = dayRange(day);
    try {
      const result = await calendarApi.dayNotes(start, end);
      if (alive && version === requestVersion) {
        dayNotes = result;
        // Notes deleted elsewhere drop out of the selection.
        const available = new Set(result.map((note) => note.id));
        checked = checked.filter((id) => available.has(id));
        if (!checked.length) confirming = false;
      }
    }
    catch (e) { if (alive && version === requestVersion) dayError = String(e); }
    finally { if (alive && version === requestVersion) dayLoading = false; }
  }

  async function openNote(id: string) {
    if (opening) return;
    opening = id; error = null;
    try { await calendarApi.showNote(id); }
    catch (e) { error = String(e); }
    finally { opening = null; }
  }

  function toggleChecked(id: string) {
    checked = checked.includes(id) ? checked.filter((other) => other !== id) : [...checked, id];
    confirming = false;
  }

  /** The first click arms the button; the second deletes, as in a note's own delete panel. */
  async function deleteChecked() {
    if (!checked.length || deleting) return;
    if (!confirming) { confirming = true; return; }
    deleting = true; error = null;
    const failures: string[] = [];
    const failedIds: string[] = [];
    for (const id of checked) {
      try { await api.deleteNote(id); }
      catch (e) { failures.push(String(e)); failedIds.push(id); }
    }
    checked = failedIds;
    confirming = false;
    deleting = false;
    if (failures.length) error = failures.join("\n");
    if (selectedDate) await loadDay(selectedDate, false);
    void refresh();
  }

  async function setPin(mode: PinMode) {
    pinOpen = false;
    const previous = pin;
    pin = mode;
    try { await calendarApi.setPin(mode); }
    catch (e) { pin = previous; error = String(e); }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (pinOpen) pinOpen = false;
    else if (confirming) confirming = false;
    else void win.close();
  }

  onMount(() => {
    alive = true;
    calendarApi.pin().then((mode) => { if (alive) pin = mode; }).catch((e) => { if (alive) error = String(e); });
    const tick = setInterval(() => { now = new Date(); }, 30_000);
    const poll = setInterval(() => { if (document.visibilityState !== "hidden") void refresh(); }, 30_000);
    let stop: (() => void) | undefined;
    void (async () => {
      stop = await listen("notes://changed", scheduleRefresh);
      if (!alive) { stop(); return; }
      await refresh();
      await api.windowReady();
    })();
    return () => { alive = false; stop?.(); clearTimeout(refreshTimer); clearInterval(tick); clearInterval(poll); };
  });
</script>

<svelte:window onkeydown={onKey} onpointerdown={(e) => { if (pinOpen && !(e.target as Element).closest(".panel, .pin-toggle")) pinOpen = false; }} onfocus={() => { now = new Date(); void refresh(); }} />

<main class="calendar" class:dark={$darkMode}>
  <header use:dragWindow>
    <div class="title"><CalendarDays size={17} /> <span>{$text.title}</span></div>
    <div class="actions">
      <button class="icon pin-toggle" class:on={pin !== "normal"} title={pinTitle} aria-label={pinTitle} aria-expanded={pinOpen} onclick={() => { pinOpen = !pinOpen; }}><PinIcon size={16} /></button>
      <button class="icon" title={$text.close} aria-label={$text.close} onclick={() => win.close()}><X size={17} /></button>
    </div>
  </header>
  {#if pinOpen}<PinPanel {pin} label={$text.position} onPick={setPin} />{/if}
  <div class="calendar-content">
  <section class="month" aria-label={monthTitle}>
  <nav aria-label={monthTitle}>
    <h1>{monthTitle}</h1>
    <button class="today" onclick={() => { now = new Date(); month = new Date(now.getFullYear(), now.getMonth(), 1); }}>{$text.today}</button>
    <button class="icon" aria-label={$text.previous} title={$text.previous} onclick={() => moveMonth(-1)}><ChevronLeft size={17} /></button>
    <button class="icon" aria-label={$text.next} title={$text.next} onclick={() => moveMonth(1)}><ChevronRight size={17} /></button>
  </nav>
  <div class="weekdays" aria-hidden="true">{#each weekdays as day}<span>{day}</span>{/each}</div>
  <div class="days" aria-label={monthTitle}>
    {#each days as day (dateKey(day))}
      {@const key = dateKey(day)}
      {@const count = counts.get(key) ?? 0}
      <button
        class="day" class:outside={day.getMonth() !== month.getMonth()} class:has-notes={count > 0}
        class:is-today={key === dateKey(now)} class:selected={key === selected}
        aria-label={`${new Intl.DateTimeFormat(locale, { dateStyle: "full" }).format(day)}, ${$text.count(count)}`}
        aria-current={key === dateKey(now) ? "date" : undefined} aria-pressed={key === selected}
        disabled={loading} onclick={() => showDay(day)}
      >
        <span class="number">{day.getDate()}</span>
        {#if count > 0}<span class="count">{$text.count(count)}</span>{/if}
      </button>
    {/each}
  </div>
  </section>
  <section class="note-list" aria-label={$text.noteList} aria-busy={dayLoading}>
    <div class="list-heading">
      <h2>{selectedDate ? new Intl.DateTimeFormat(locale, { month: "long", day: "numeric", weekday: "short" }).format(selectedDate) : $text.noteList}</h2>
      {#if selectedDate && dayNotes.length}
        <button
          class="icon delete" class:armed={confirming} disabled={!checked.length || deleting}
          title={confirming ? `${$text.confirmDelete(checked.length)} · ${$t.deleteHint}` : $text.deleteSelected}
          aria-label={confirming ? $text.confirmDelete(checked.length) : $text.deleteSelected}
          onclick={deleteChecked}
        ><Trash size={15} />{#if confirming}<span>{$text.confirmDelete(checked.length)}</span>{/if}</button>
      {/if}
      {#if selectedDate}<span class="total">{$text.notes(dayNotes.length)}</span>{/if}
    </div>
    <div class="cards">
      {#if dayLoading}
        <p class="empty" role="status">{$text.loading}</p>
      {:else if !selectedDate}
        <div class="empty"><StickyNote size={26} strokeWidth={1.5} /><strong>{$text.chooseDate}</strong><p>{$text.chooseDateHint}</p></div>
      {:else if dayNotes.length === 0}
        <p class="empty">{$text.empty}</p>
      {:else}
        {#each dayNotes as note (note.id)}
          <div class="card-row" class:checked={checked.includes(note.id)}>
            <button class="note-card" class:dark={isDark(note.tint)} style:--card-tint="var(--tint-{$darkMode ? 'dark-' : ''}{note.tint})" disabled={opening !== null || deleting} aria-label={`${note.title} · ${$text.openNote}`} onclick={() => openNote(note.id)}>
              <h3>{note.title}</h3>
              {#if note.preview}<p>{note.preview}</p>{/if}
              <time datetime={new Date(note.createdAt).toISOString()}>{new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(note.createdAt)}</time>
            </button>
            <input class="pick" type="checkbox" checked={checked.includes(note.id)} disabled={deleting} aria-label={$text.selectNote(note.title)} title={$text.selectNote(note.title)} onchange={() => toggleChecked(note.id)} />
          </div>
        {/each}
      {/if}
    </div>
  </section>
  </div>
  {#if error || dayError || loadError}
    <div class="error" role="alert">{error ?? dayError ?? loadError}<button onclick={() => { error = null; void refresh(); }}>{$text.retry}</button></div>
  {/if}
</main>

<style>
  header { touch-action: none; }
  .calendar { position: fixed; inset: 0; display: flex; flex-direction: column; color: var(--ink); background: #f5f7f9; border: 1px solid rgb(31 37 43 / 0.14); border-radius: var(--radius-window); overflow: hidden; }
  .calendar-content { display: grid; grid-template-columns: minmax(320px, 1.2fr) minmax(240px, 1fr); flex: 1; min-height: 0; }
  .month { display: flex; flex-direction: column; min-width: 0; padding-top: 8px; }
  .note-list { display: flex; flex-direction: column; min-width: 0; min-height: 0; border-left: 1px solid rgb(127 137 145 / 0.2); background: rgb(127 137 145 / 0.04); }
  .list-heading { display: flex; align-items: baseline; gap: 8px; padding: 12px 16px 14px; }
  .list-heading h2 { margin: 0; font-size: 14px; font-weight: 650; }
  .list-heading { align-items: center; }
  .list-heading .total { margin-left: auto; font-size: 11px; color: var(--ink-soft); white-space: nowrap; }
  .icon.delete { width: auto; min-width: 28px; height: 26px; gap: 6px; padding: 0 6px; grid-auto-flow: column; color: var(--ink-soft); font-size: 11.5px; white-space: nowrap; }
  .icon.delete:disabled { opacity: 0.35; cursor: default; }
  .icon.delete:hover:not(:disabled), .icon.delete.armed { color: var(--danger); }
  .icon.delete.armed { background: rgb(201 76 66 / 0.1); }
  .card-row { position: relative; margin-bottom: 12px; }
  .card-row .note-card { margin-bottom: 0; }
  .card-row .note-card h3 { padding-right: 24px; }
  .card-row.checked .note-card { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .pick { position: absolute; top: 12px; right: 12px; width: 16px; height: 16px; margin: 0; accent-color: var(--accent); cursor: pointer; }
  .cards { flex: 1; min-height: 0; overflow: auto; padding: 0 14px 14px; }
  .empty { display: flex; flex-direction: column; gap: 12px; align-items: center; justify-content: center; min-height: 220px; margin: 0; padding: 20px; text-align: center; color: var(--ink-soft); font-size: 13px; line-height: 1.6; }
  .empty strong { font-size: 14px; font-weight: 600; }
  .empty p { margin: 0; }
  .note-card { display: flex; flex-direction: column; gap: 8px; width: 100%; min-height: 116px; margin-bottom: 12px; padding: 14px; border: 1px solid rgb(127 137 145 / 0.18); border-radius: 8px; background: rgb(var(--card-tint) / 0.65); color: var(--ink); text-align: left; cursor: pointer; }
  .note-card.dark { color: #eef1f4; }
  .note-card:hover { border-color: var(--accent); }
  .note-card h3, .note-card p { display: -webkit-box; -webkit-box-orient: vertical; overflow: hidden; margin: 0; }
  .note-card h3 { line-clamp: 2; -webkit-line-clamp: 2; font-size: 13px; font-weight: 650; line-height: 1.5; }
  .note-card p { line-clamp: 3; -webkit-line-clamp: 3; white-space: pre-line; font-size: 12px; line-height: 1.55; opacity: 0.75; }
  .note-card time { display: block; margin-top: auto; padding-top: 4px; font-size: 10px; opacity: 0.6; }
  .cards::-webkit-scrollbar { width: 8px; }
  .cards::-webkit-scrollbar-thumb { border: 2px solid transparent; border-radius: 4px; background: rgb(127 137 145 / 0.35) padding-box; }
  :global([data-theme="dark"]) .calendar { --ink: #e9edf1; --ink-soft: #b5bfc8; --ink-faint: #87929c; --accent: #8fbadb; --accent-soft: rgb(143 186 219 / 0.18); --danger: #e59a94; background: #20252a; border-color: rgb(255 255 255 / 0.14); }
  :global([data-theme="dark"]) .day { background: rgb(255 255 255 / 0.05); }
  :global([data-theme="dark"]) .day:hover { background: rgb(255 255 255 / 0.1); }
  :global([data-theme="dark"]) .day.selected { background: var(--accent-soft); }
  header { height: 36px; padding: 0 10px 0 14px; display: flex; align-items: center; justify-content: space-between; flex: none; }
  .title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600; }
  .actions { display: flex; align-items: center; gap: 2px; }
  .calendar { --link: var(--accent); }
  .icon.on { color: var(--accent); }
  .icon { display: grid; place-items: center; width: 28px; height: 28px; border: 0; padding: 0; background: transparent; border-radius: var(--radius-control); }
  .icon:hover, .today:hover { background: rgb(31 37 43 / 0.08); }
  nav { display: flex; align-items: center; gap: 4px; padding: 0 10px 10px 14px; flex: none; }
  h1 { margin: 0 auto 0 0; font-size: 18px; font-weight: 650; letter-spacing: -0.02em; }
  .today { border: 1px solid rgb(31 37 43 / 0.14); border-radius: var(--radius-control); background: transparent; height: 27px; padding: 0 10px; margin-right: 6px; font-size: 12px; }
  .weekdays, .days { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); margin: 0 12px; }
  .weekdays { flex: none; text-align: center; color: var(--ink-soft); font-size: 11px; padding-bottom: 8px; }
  .days { flex: 1; min-height: 0; grid-template-rows: repeat(6, minmax(0, 1fr)); gap: 3px; margin-bottom: 10px; }
  .day { min-width: 0; display: flex; align-items: flex-start; flex-direction: column; gap: 2px; padding: 4px; border: 1px solid transparent; border-radius: var(--radius-control); background: rgb(255 255 255 / 0.65); text-align: left; cursor: pointer; }
  .day:hover { background: #e6edf2; }
  .day.outside { background: transparent; color: var(--ink-faint); }
  .day.is-today { border-color: rgb(46 94 126 / 0.5); }
  .day.selected { background: var(--accent-soft); border-color: var(--accent); }
  .number { font-size: 12px; line-height: 15px; font-variant-numeric: tabular-nums; }
  .is-today .number { color: var(--accent); font-weight: 700; }
  .has-notes .number, .count { color: var(--danger); }
  .count { font-size: 10px; font-weight: 550; line-height: 12px; overflow-wrap: anywhere; }
  .day:disabled { cursor: default; }
  .error { color: var(--danger); max-height: 64px; padding: 0 14px 8px; font-size: 11px; overflow: auto; overflow-wrap: anywhere; }
  .error button { margin-left: 8px; border: 0; background: transparent; text-decoration: underline; cursor: pointer; }
</style>
