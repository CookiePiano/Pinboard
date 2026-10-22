<script lang="ts">
  import { t } from "../i18n";
  import { onMount } from "svelte";
  import { FolderOpen } from "@lucide/svelte";
  import { api, type AppInfo } from "../ipc";

  let info = $state<AppInfo | null>(null);
  onMount(async () => (info = await api.appInfo()));
</script>

{#if info}
  <section class="group">
    <div class="identity">
      <img src="/pinboard.svg" alt="" width="44" height="44" />
      <div>
        <p class="name">Pinboard</p>
        <p class="hint">{$t.versionNotes(info.version, info.noteCount)}</p>
      </div>
    </div>
  </section>

  <section class="group">
    <h2>{$t.data}</h2>
    <div class="row">
      <div class="row-text">
        {$t.localData}
        <span class="path" title={info.dataDir}>{info.dataDir}</span>
      </div>
      <button class="btn" onclick={() => api.openDataDir()}><FolderOpen size={15} strokeWidth={1.9} />{$t.open}</button>
    </div>
  </section>

{/if}

<style>
  .identity {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .identity p {
    margin: 0;
  }

  .name {
    font-size: 16px;
    font-weight: 650;
  }

</style>
