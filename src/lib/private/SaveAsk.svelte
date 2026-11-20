<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A private file opened in another app was changed there: save the changes into Private
  // mode, or drop them. Either way the temporary copy is wiped.
  import { api } from "../api";
  import { app } from "../state.svelte";
  import { priv } from "../private.svelte";
  import Modal from "../Modal.svelte";

  const ask = $derived(priv.saveAsks[0]);

  function answer(save: boolean) {
    const a = ask;
    priv.saveAsks = priv.saveAsks.slice(1);
    app.run(async () => {
      await api.privateKeepChanges(a.copy, save);
      if (save) app.toast("Your changes are saved in Private mode.");
      await priv.refreshList();
    });
  }
</script>

{#if ask}
  <Modal title="Save your changes into Private mode?" width={440} onclose={() => answer(false)} closeLabel="Don't save" outsideCloses={false}>
    <p class="msg">
      {#if ask.name}<b>{ask.name}</b> was{:else}A private file you opened was{/if} changed in the app you opened it with. Save those changes into Private mode, or keep the
      file as it was? The temporary copy is deleted either way.
    </p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => answer(false)}>Don't save</button>
      <button class="btn primary" onclick={() => answer(true)}>Save changes</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .msg {
    margin: 0;
    line-height: 1.55;
    color: var(--text-2);
  }
</style>
