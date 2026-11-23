<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Direct's questions, shown on any tab: "… wants to connect" (the first time only) and
  // "… wants to send you …" (unless that contact may send without asking). One at a time,
  // oldest first. The answers wake up a moment after a question appears, so a click or an Esc
  // meant for something else can't accept it, and can't decline it either (declining a request
  // turns that person away for 7 days).
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api } from "./api";
  import { bytes } from "./format";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import { around, num, t } from "./i18n.svelte";

  const request = $derived(app.direct?.requests?.[0] ?? null);
  /** "Once you accept, … in **Direct** and …": the page's name in bold, mid-sentence. */
  const onceAccepted = $derived(
    around(t("Once you accept, {name} becomes one of your contacts in {page} and you connect by yourselves whenever both Pakos are open. You're only asked this once.", { name: request?.name ?? "" }), "{page}"),
  );
  /** Contacts you muted: their offers wait in their conversation, asked only once it's open. */
  const muted = $derived(new Set((app.direct?.contacts ?? []).filter((c) => c.muted).map((c) => c.id)));
  const offer = $derived(
    request ? null : ((app.direct?.offers ?? []).find((o) => !muted.has(o.contact) || (app.view === "direct" && app.directPicked === o.contact)) ?? null),
  );
  /** A folder picked for this offer (else the contact's folder). */
  let folder = $state<string | null>(null);
  let lastOffer = "";
  $effect(() => {
    if (offer && offer.id !== lastOffer) {
      lastOffer = offer.id;
      folder = null;
    }
  });

  /** Accept answers only once the question has been on screen for a moment. */
  let armed = $state(false);
  const shown = $derived(request ? `r:${request.peer}` : offer ? `o:${offer.id}` : "");
  $effect(() => {
    // Another question (or none): the last answer is forgotten, so the same person can be
    // answered again another time.
    answered = "";
    if (!shown) return;
    armed = false;
    const timer = setTimeout(() => (armed = true), 600);
    return () => clearTimeout(timer);
  });

  /** One answer per question: the dialog stays for a moment until the engine takes the question
   *  off the list, and a second click would answer it again (an error, or the next question).
   *  An answer that didn't go through can be given again. */
  let answered = "";
  async function answer(send: () => Promise<unknown>) {
    const key = shown;
    if (!armed || !key || answered === key) return;
    answered = key;
    try {
      await send();
    } catch (e) {
      app.toast(String(e), "error");
      if (answered === key) answered = "";
    }
  }
  /** No name of yours yet: accepting asks for it first (they'll see it). */
  const nameMissing = $derived(!(app.direct?.myName ?? "").trim());
  let myName = $state("");
  /** Accept pressed with no name: said under the field, which takes the focus. */
  let needName = $state(false);
  let nameField = $state<HTMLInputElement | null>(null);
  $effect(() => {
    if (myName.trim()) needName = false;
  });
  function answerRequest(peer: string, accept: boolean) {
    if (accept && nameMissing && !myName.trim()) {
      if (!armed) return;
      needName = true;
      nameField?.focus();
      return;
    }
    return answer(async () => {
      if (accept && nameMissing) await api.directSetName(myName.trim());
      await api.directAnswerRequest(peer, accept);
    });
  }
  const answerOffer = (id: string, accept: boolean) => answer(() => api.directAnswerOffer(id, accept, accept ? folder : null));

  async function change() {
    const r = await pick({ directory: true, multiple: false, title: t("Where to save these files") });
    if (typeof r === "string") folder = r;
  }
</script>

{#if request}
  {@const r = request}
  <Modal title={t("{name} wants to connect", { name: r.name })} width={480} closeLabel={t("Decline")} onclose={() => answerRequest(r.peer, false)}>
    <div class="body">
      <p class="who dim small"><Icon name="alert" size={13} />{t("Not in your contacts. The name is the one they chose; Pako can't check it.")}</p>
      {#if r.knownAs}
        <p class="warn small">{t("You already have a contact called {name}. This is a different Pako: if you expected {name}, decline.", { name: r.knownAs })}</p>
      {/if}
      <p>{onceAccepted[0]}<b>{t("Direct")}</b>{onceAccepted[1]}</p>
      <div class="code">
        <span class="dim">{t("Safety code")}</span>
        <b class="mono">{r.code}</b>
      </div>
      <p class="dim small">{t("If you know them, check they see the same code (by voice or chat). A different code means it isn't them.")}</p>
      <p class="dim small">{t("If you decline (or close this), Pako turns {name} away without asking you for 7 days.", { name: r.name })}</p>
      {#if nameMissing}
        <label class="lbl dim small" for="reqmyname">{t("Your name (what {name} sees)", { name: r.name })}</label>
        <input id="reqmyname" class="input" class:missing={needName} bind:this={nameField} bind:value={myName} placeholder={t("What your contacts see")} maxlength="60" aria-invalid={needName} />
        {#if needName}<p class="need small" role="alert">{t("Fill in your name first: {name} sees it when you accept.", { name: r.name })}</p>{/if}
      {/if}
    </div>
    {#snippet actions()}
      <button class="btn ghost" title={t("Decline (Esc)")} onclick={() => answerRequest(r.peer, false)}>{t("Decline")}</button>
      <button class="btn primary" class:arming={!armed} aria-disabled={!armed} onclick={() => answerRequest(r.peer, true)}>{t("Accept")}</button>
    {/snippet}
  </Modal>
{:else if offer}
  {@const o = offer}
  <Modal title={t("{name} wants to send you files", { name: o.contactName })} width={500} closeLabel={t("Decline")} onclose={() => answerOffer(o.id, false)}>
    <div class="body">
      <div class="what">
        <Icon name={o.files > 1 ? "folder" : "file"} size={18} />
        <div>
          <div class="wname">{o.name}</div>
          <div class="dim small">{t("{n} files", { n: num(o.files) })} · {bytes(o.size)}</div>
        </div>
      </div>
      {#if o.files > 1}
        <ul class="sample dim small">
          {#each o.sample as s, i (i)}<li>{s}</li>{/each}
          {#if o.files > o.sample.length}<li>{t("and {n} more files", { n: num(o.files - o.sample.length) })}</li>{/if}
        </ul>
      {/if}
      <span class="lbl dim small">{t("Save to")}</span>
      <div class="field">
        <span class="path" title={folder ?? o.folder}>{folder ?? o.folder}</span>
        <button class="link" onclick={change}>{t("Change")}</button>
      </div>
      {#if o.askedBecause}
        <p class="dim small">{o.askedBecause}</p>
      {:else if !(app.direct?.contacts ?? []).some((c) => c.id === o.contact)}
        <!-- One of your own PCs (devices aren't in Direct's people): asked until it's paired both ways. -->
        <p class="dim small">{t("Files from {name} come without asking once it's paired both ways: confirm it in My devices on both PCs.", { name: o.contactName })}</p>
      {:else}
        <p class="dim small">{t('To skip this question for {name}, turn on "Can send to me without asking" in Direct.', { name: o.contactName })}</p>
      {/if}
    </div>
    {#snippet actions()}
      <button class="btn ghost" title={t("Decline (Esc)")} onclick={() => answerOffer(o.id, false)}>{t("Decline")}</button>
      <button class="btn primary" class:arming={!armed} aria-disabled={!armed} onclick={() => answerOffer(o.id, true)}><Icon name="download" size={13} stroke={2.2} />{t("Accept")}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .body p.need {
    color: var(--st-fail-text);
  }
  .input.missing {
    border-color: var(--st-fail);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .body p {
    margin: 0;
    line-height: 1.55;
    color: var(--text-2);
  }
  .dim {
    color: var(--text-3);
  }
  .body p.dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .who :global(svg) {
    flex: none;
  }
  .warn {
    padding: 8px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--st-retry) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--st-retry) 30%, transparent);
    line-height: 1.5;
  }
  .body p.warn {
    color: var(--text);
  }
  /* Just shown: Accept fades in and doesn't take clicks yet. */
  .btn.primary {
    transition:
      opacity var(--slow) var(--ease),
      background-color var(--fast) var(--ease);
  }
  .btn.primary.arming {
    opacity: 0.55;
    pointer-events: none;
  }
  .code {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line-soft);
  }
  .code b {
    font-size: 16px;
    letter-spacing: 0.06em;
    color: var(--text);
  }
  .what {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .what :global(svg) {
    color: var(--accent-strong);
  }
  .wname {
    font-weight: 600;
  }
  .sample {
    margin: 0;
    padding-left: 18px;
    line-height: 1.6;
    max-height: 120px;
    overflow: hidden;
  }
  .lbl {
    margin-top: 2px;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--control);
    background: var(--bg);
  }
  .path {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--focus-text);
    font: inherit;
    cursor: pointer;
  }
</style>
