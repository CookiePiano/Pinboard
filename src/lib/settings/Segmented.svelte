<script lang="ts" generics="T extends string">
  let {
    value,
    options,
    label,
    onChange,
  }: { value: T; options: { id: T; label: string }[]; label: string; onChange: (v: T) => void } = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
  {#each options as o (o.id)}
    <button role="radio" aria-checked={o.id === value} onclick={() => onChange(o.id)}>{o.label}</button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: 8px;
    background: rgb(127 137 145 / 0.16);
  }

  button {
    min-width: 76px;
    padding: 5px 14px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    font-size: 13px;
    color: var(--text-soft);
    transition:
      background-color 140ms,
      color 140ms;
  }

  button:hover {
    color: var(--text);
  }

  button[aria-checked="true"] {
    color: var(--text);
    font-weight: 600;
    background: var(--raised);
    box-shadow:
      0 0 0 1px rgb(31 37 43 / 0.06),
      0 1px 3px rgb(15 22 30 / 0.12);
  }
</style>
