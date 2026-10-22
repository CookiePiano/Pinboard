<script lang="ts">
  import { t } from "../i18n";
  import { ArrowUpToLine, Check, Monitor, Pin } from "@lucide/svelte";
  import type { PinMode } from "../ipc";

  let { pin, onPick, label }: { pin: PinMode; onPick: (m: PinMode) => void; label?: string } = $props();

  let modes = $derived([
    { id: "normal", icon: Pin, title: $t.pinNormal, desc: $t.pinNormalHint },
    { id: "top", icon: ArrowUpToLine, title: $t.pinTop, desc: $t.pinTopHint },
    { id: "desktop", icon: Monitor, title: $t.pinDesktop, desc: $t.pinDesktopHint },
  ] as const);
</script>

<div class="panel" role="radiogroup" aria-label={label ?? $t.notePosition}>
  {#each modes as m (m.id)}
    <button class="item" role="radio" aria-checked={pin === m.id} onclick={() => onPick(m.id)}>
      <m.icon size={15} strokeWidth={2} />
      <span class="text">
        {m.title}
        <small>{m.desc}</small>
      </span>
      {#if pin === m.id}
        <Check size={15} strokeWidth={2.25} class="check" />
      {/if}
    </button>
  {/each}
</div>

<style>
  .text {
    flex: 1;
  }

  .item[aria-checked="true"] {
    color: var(--link);
  }
</style>
