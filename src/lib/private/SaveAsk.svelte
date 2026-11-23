<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A private file opened in another app was changed there: save the changes into Private
  // mode, or drop them. Either way the temporary copy is wiped.
  import { api } from "../api";
  import { app } from "../state.svelte";
  import { priv } from "../private.svelte";
  import Modal from "../Modal.svelte";
  import { around, t } from "../i18n.svelte";

  const ask = $derived(priv.saveAsks[0]);

  function answer(save: boolean) {
    const a = ask;
    priv.saveAsks = priv.saveAsks.slice(1);
    app.run(async () => {
      await api.privateKeepChanges(a.copy, save);
      if (save) app.toast(t("Your changes are saved in Private mode."));
      await priv.refreshList();
    });
  }
</script>

{#if ask}
  {@const [before, after] = around(
    t("{name} was changed in the app you opened it with. Save those changes into Private mode, or keep the file as it was? The temporary copy is deleted either way."),
    "{name}",
  )}
  <Modal title={t("Save your changes into Private mode?")} width={440} onclose={() => answer(false)} closeLabel={t("Don't save")} outsideCloses={false}>
    <p class="msg">
      {#if ask.name}{before}<b>{ask.name}</b>{after}{:else}{t(
          "A private file you opened was changed in the app you opened it with. Save those changes into Private mode, or keep the file as it was? The temporary copy is deleted either way.",
        )}{/if}
    </p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => answer(false)}>{t("Don't save")}</button>
      <button class="btn primary" onclick={() => answer(true)}>{t("Save changes")}</button>
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
