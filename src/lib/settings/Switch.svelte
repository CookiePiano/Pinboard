<script lang="ts">
  let {
    checked,
    label,
    disabled = false,
    onChange,
  }: { checked: boolean; label: string; disabled?: boolean; onChange: (v: boolean) => void } = $props();
</script>

<button
  class="switch"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onChange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 36px;
    height: 20px;
    padding: 0;
    border: 0;
    border-radius: 10px;
    background: rgb(127 137 145 / 0.38);
    transition: background-color 160ms;
  }

  .switch[aria-checked="true"] {
    background: var(--accent);
  }

  .switch:disabled {
    opacity: 0.5;
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgb(15 22 30 / 0.3);
    transition: transform 180ms var(--ease-out);
  }

  .switch[aria-checked="true"] .knob {
    transform: translateX(16px);
  }
</style>
