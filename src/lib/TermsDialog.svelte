<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The terms of use (TERMS.md, word for word sookera.com/terms; TERMS.fr.md in French), before anything else: the first
  // step of a new Pako, and once for people who used Pako before it had terms. Only its own two
  // buttons answer it (no ×, Esc or click beside it). Nothing runs until Agree and continue
  // (src-tauri/src/consent.rs), then everything starts without a restart.
  import { app } from "./state.svelte";
  import { api } from "./api";
  import Modal from "./Modal.svelte";
  import Markdown from "./help/Markdown.svelte";
  import { terms } from "./help/content";
  import { bare, openLink, openWeb, SITE } from "./web";
  import SegPick from "./SegPick.svelte";
  import { around, i18n, t, LANGUAGES, type Lang } from "./i18n.svelte";

  /** Used Pako before it had terms (the first-start setup was done then). */
  const existing = !!app.settings?.setupDone;

  /** "Also on sookera.com/terms, …": the link sits in the middle of the sentence. */
  const web = $derived(around(t("Also on {link}, always in their newest version."), "{link}"));

  // A new Pako asks the terms before its first-start setup, where a portable Pako's language is
  // picked: the language is offered here too (shown and saved at once, like the setup's), so the
  // terms can be read in French from the start. Pako Setup's pick is already shown.
  function pickLanguage(l: Lang) {
    i18n.set(l).catch((e) => app.toast(String(e), "error"));
  }

  // The other language's terms start at their top, not where the reader was in the first.
  let box: HTMLDivElement | undefined = $state();
  $effect(() => {
    void i18n.lang;
    if (box) box.scrollTop = 0;
  });

  let busy = $state(false);
  async function agree() {
    if (busy) return;
    busy = true;
    try {
      await api.acceptTerms();
    } catch (e) {
      // Pako runs anyway this time; the message says the answer couldn't be kept.
      app.toast(t("Pako couldn't save that you agreed, so it asks again next time: {error}", { error: String(e) }), "error");
    }
    app.termsNeeded = false;
    await app.refresh();
    busy = false;
  }
</script>

<Modal title={existing ? t("Pako's terms of use") : t("Terms of use")} width={560} closable={false} outsideCloses={false} onclose={() => {}}>
  {#if existing}
    <p class="intro">{t("Pako now has terms of use. Please read them: Pako carries on as soon as you agree.")}</p>
    <p class="intro kept">{t("Your downloads and lists stay as they are. Paused and waiting downloads simply wait until you agree, and nothing is lost.")}</p>
  {:else}
    <div class="first">
      <p class="intro">{t("Before you start, please read Pako's terms of use. Pako starts as soon as you agree.")}</p>
      <SegPick label={t("Language")} options={LANGUAGES} value={i18n.lang} onpick={(l) => pickLanguage(l as Lang)} />
    </div>
  {/if}
  <!-- Focusable so the keyboard can scroll it. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="terms" role="document" aria-label={t("Terms of use")} tabindex="0" bind:this={box}>
    <Markdown blocks={terms()} onlink={openLink} />
  </div>
  <p class="web">{web[0]}<button class="lk" onclick={() => openWeb(SITE.terms)}>{bare(SITE.terms)}</button>{web[1]}</p>

  {#snippet actions()}
    <button class="btn ghost" onclick={() => api.quitNow()}>{t("Quit Pako")}</button>
    <button class="btn primary" disabled={busy} onclick={agree}>{t("Agree and continue")}</button>
  {/snippet}
</Modal>

<style>
  /* A new Pako: the first sentence, and the language beside it. */
  .first {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin: 0 0 12px;
  }
  .first .intro {
    flex: 1;
    margin: 0;
  }
  .first :global(> :last-child) {
    flex: none;
  }
  .intro {
    margin: 0 0 12px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .intro.kept {
    margin-top: -4px;
  }
  .terms {
    max-height: min(46vh, 400px);
    overflow-y: auto;
    padding: 4px 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--bg);
  }
  .terms:focus-visible {
    outline-offset: 2px;
  }
  .web {
    margin: 10px 0 4px;
    font-size: 12px;
    color: var(--text-3);
  }
  .lk {
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    color: var(--text-2);
    cursor: pointer;
    text-decoration: underline;
    text-decoration-color: var(--line-strong);
    text-underline-offset: 3px;
    transition:
      color var(--fast) ease,
      text-decoration-color var(--fast) ease;
  }
  .lk:hover {
    color: var(--text);
    text-decoration-color: var(--accent);
  }
</style>
