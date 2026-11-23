<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // An archive locked with a password (a protected upload someone sent you, for example): type
  // it here and Pako extracts it with 7-Zip. The password is only handed to 7-Zip, never kept.
  import { onMount, tick } from "svelte";
  import { api, type Download } from "./api";
  import { app } from "./state.svelte";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { t, type Words, said } from "./i18n.svelte";

  let { d, onclose }: { d: Download; onclose: () => void } = $props();

  let password = $state("");
  let show = $state(false);
  let busy = $state(false);
  let error = $state<Words>("");
  let input: HTMLInputElement | undefined = $state();

  onMount(async () => {
    await tick();
    input?.focus();
  });

  async function go() {
    if (busy || !password) return;
    busy = true;
    error = "";
    try {
      const out = await api.extractNow(d.id, password);
      password = "";
      app.toast(t("{name} is extracted.", { name: d.name }), "info", { label: t("Show"), run: () => api.openFolder(out) });
      await app.refresh();
      onclose();
    } catch (e) {
      const why = String(e);
      error = why === "password-wrong" || why === "password-needed" ? () => t("That password isn't right. Check it and try again.") : why;
      busy = false;
      await tick();
      input?.select();
    }
  }
</script>

<Modal title={t("Password for {name}", { name: d.name })} width={440} onclose={() => !busy && onclose()} outsideCloses={!busy} closable={!busy}>
  <div class="pw">
    <p class="msg">{t("This archive is locked with a password. Type it to extract the files. Pako hands it to 7-Zip only and doesn't keep it.")}</p>
    <div class="field">
      <input
        bind:this={input}
        class="input"
        type={show ? "text" : "password"}
        aria-label={t("Password")}
        autocomplete="off"
        spellcheck="false"
        bind:value={password}
        disabled={busy}
        onkeydown={(e) => e.key === "Enter" && go()}
      />
      <button class="btn small" type="button" aria-label={show ? t("Hide the password") : t("Show the password")} aria-pressed={show} onclick={() => (show = !show)}>{show ? t("Hide") : t("Show")}</button>
    </div>
    {#if error}<p class="err" role="alert">{said(error)}</p>{/if}
  </div>
  {#snippet actions()}
    <button class="btn ghost" disabled={busy} onclick={onclose}>{t("Cancel")}</button>
    <button class="btn primary" disabled={busy || !password} onclick={go}><Icon name="unlock" size={13} />{busy ? t("Extracting…") : t("Extract")}</button>
  {/snippet}
</Modal>

<style>
  .pw {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .msg {
    margin: 0;
    color: var(--text-2);
    line-height: 1.5;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .field .input {
    flex: 1;
  }
  .err {
    margin: 0;
    font-size: 12.5px;
    color: var(--st-fail-text);
  }
</style>
