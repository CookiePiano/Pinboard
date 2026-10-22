<script lang="ts">
  let { label, value, onInput }: { label: string; value: number; onInput: (v: number) => void } = $props();
  const uid = `slider-${Math.random().toString(36).slice(2, 8)}`;
  let percent = $derived(Math.round(value * 100));
</script>

<div class="slider">
  <label for={uid}>{label}</label>
  <output for={uid}>{percent}%</output>
  <input
    id={uid}
    type="range"
    min="0"
    max="100"
    step="1"
    value={percent}
    style:--fill="{percent}%"
    oninput={(e) => onInput(Number((e.currentTarget as HTMLInputElement).value) / 100)}
  />
</div>

<style>
  .slider {
    display: grid;
    grid-template-columns: 1fr auto;
    row-gap: 6px;
    padding: 0 2px;
  }

  output {
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }

  input {
    grid-column: 1 / -1;
    appearance: none;
    width: 100%;
    height: 16px;
    margin: 0;
    background: transparent;
  }

  input::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(90deg, var(--accent) var(--fill), rgb(127 137 145 / 0.3) var(--fill));
  }

  input::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border-radius: 50%;
    background: #fff;
    box-shadow:
      0 0 0 1px rgb(31 37 43 / 0.18),
      0 1px 3px rgb(15 22 30 / 0.25);
  }

  input:focus-visible {
    outline: none;
  }

  input:focus-visible::-webkit-slider-thumb {
    box-shadow: 0 0 0 3px var(--accent-soft), 0 0 0 1px var(--accent);
  }
</style>
