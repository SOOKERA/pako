<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A new recovery code: shown once, group by group, with Copy / Save as a file / Print. It only
  // counts once its last group is typed back (`confirmed`).
  import { onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, inApp } from "../api";
  import { app } from "../state.svelte";
  import { reduced } from "../motion";
  import Icon from "../Icon.svelte";
  import { t } from "../i18n.svelte";

  let {
    confirmed = $bindable(false),
    last = $bindable(""),
    given = null,
  }: {
    confirmed?: boolean;
    last?: string;
    /** The code already made (made with your PIN or password while Private mode is locked). */
    given?: string | null;
  } = $props();

  let code = $state("");
  let shown = $state(0);
  let error = $state("");
  const groups = $derived(code ? code.split("-") : []);

  onMount(async () => {
    try {
      code = given ?? (await api.privateNewCode());
    } catch (e) {
      error = String(e);
      return;
    }
    if (reduced()) {
      shown = groups.length;
      return;
    }
    // Group by group, quickly.
    const tick = setInterval(() => {
      shown++;
      if (shown >= groups.length) clearInterval(tick);
    }, 70);
  });

  async function copy() {
    // In Pako: kept out of Windows' clipboard history and cloud clipboard, cleared after a minute.
    if (inApp) {
      try {
        await api.privateCopyCode();
      } catch (e) {
        app.toast(String(e), "error");
        return;
      }
    } else await navigator.clipboard.writeText(code).catch(() => {});
    app.toast(t("Recovery code copied for one minute. Paste it somewhere safe, away from this PC."));
  }

  async function saveFile() {
    if (!inApp) {
      app.toast(t("Preview mode: nothing is saved."));
      return;
    }
    const path = await save({ title: t("Save your recovery code"), defaultPath: t("Pako recovery code.txt"), filters: [{ name: t("Text"), extensions: ["txt"] }] });
    if (!path) return;
    app.run(async () => {
      await api.privateSaveCode(path);
      app.toast(t("Recovery code saved. Keep that file away from this PC."));
    });
  }

  function print() {
    const f = document.createElement("iframe");
    f.style.cssText = "position:fixed;width:0;height:0;border:0;opacity:0";
    document.body.appendChild(f);
    const doc = f.contentDocument!;
    doc.open();
    doc.write(
      `<!doctype html><title>${t("Pako recovery code")}</title><body style="font-family:Segoe UI,sans-serif;padding:40px"><h2>${t("Pako: your Private mode recovery code")}</h2><p style="font:600 22px Consolas,monospace;letter-spacing:2px">${code}</p><p>${t("It opens Private mode if you forget your PIN or password, and lets you choose a new one. Keep it somewhere safe, away from this PC.")}</p></body>`,
    );
    doc.close();
    f.contentWindow?.focus();
    f.contentWindow?.print();
    setTimeout(() => f.remove(), 1000);
  }

  $effect(() => {
    confirmed = !!code && last.trim().toUpperCase() === groups[groups.length - 1];
  });
</script>

<div class="rc">
  {#if error}
    <p class="err">{error}</p>
  {:else}
    <div class="code mono" aria-label={t("Your recovery code: {code}", { code })}>
      {#each groups as g, i (i)}
        <span class="g" class:in={i < shown}>{g}</span>
      {/each}
    </div>
    <div class="acts">
      <button class="btn" onclick={copy}><Icon name="copy" size={13} />{t("Copy")}</button>
      <button class="btn" onclick={saveFile}><Icon name="save" size={13} />{t("Save as a file")}</button>
      <button class="btn" onclick={print}><Icon name="printer" size={13} />{t("Print")}</button>
    </div>
    <p class="note">
      {t(
        "Pako shows it only now and never keeps it. With it, you can open Private mode if you forget your PIN or password, and choose a new one. Keep it away from this PC: anyone with it and this PC can open your private files. A newer code replaces it on this PC, but a copy of Pako's files made before still opens with the older one.",
      )}
    </p>
    <label class="field">
      <span>{t("To check you kept it, type its last group")}</span>
      <input class="input mono last" maxlength="4" bind:value={last} placeholder="····" autocomplete="off" spellcheck="false" />
    </label>
  {/if}
</div>

<style>
  .rc {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .code {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
  }
  .g {
    text-align: center;
    font-size: 17px;
    font-weight: 600;
    letter-spacing: 0.12em;
    color: var(--text);
    opacity: 0;
    transform: translateY(4px);
    transition:
      opacity var(--mid) var(--ease),
      transform var(--mid) var(--ease);
  }
  .g.in {
    opacity: 1;
    transform: none;
  }
  .acts {
    display: flex;
    gap: 8px;
  }
  .note {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .last {
    width: 110px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }
  .err {
    color: var(--st-fail-text);
    margin: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .g {
      transition: none;
    }
  }
</style>
