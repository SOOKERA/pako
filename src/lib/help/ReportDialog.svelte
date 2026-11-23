<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Help › Report a problem: what happened, and what goes with it (shown in full). Send report
  // sends it to Sookera's report service (a private list Sookera works through) and
  // shows the report's reference; or the report opens in your email app, or you copy or save it.
  import { onMount } from "svelte";
  import { save as saveFile } from "@tauri-apps/plugin-dialog";
  import { api, inApp, type ReportInfo } from "../api";
  import { app } from "../state.svelte";
  import { theme } from "../theme.svelte";
  import Icon from "../Icon.svelte";
  import Modal from "../Modal.svelte";
  import { fadeIn, flipIf, flipOpts, pop, reduced, rowSlide } from "../motion";
  import { help } from "./help.svelte";
  import { areaLabel, attached, mailPlan, pakoLine, problemLine, reportText, sendFailText, sentProblems, SUPPORT_EMAIL, when, zone, type ReportParts } from "./report";
  import { around, t } from "../i18n.svelte";

  /** Problems shown before "Show all". */
  const FEW = 3;

  let info = $state<ReportInfo | null>(null);
  let include = $state(true);
  let all = $state(false);
  let busy = $state(false);
  /** Send report on its way, its reference once sent, or why it didn't go (help.svelte.ts:
   *  they outlive the dialog, so closing it meanwhile neither loses the answer nor sends twice). */
  const sending = $derived(help.reportSending);
  const sentRef = $derived(help.reportSent);
  const failed = $derived(help.reportFailed);
  let field = $state<HTMLTextAreaElement | null>(null);

  const list = $derived(info ? attached(info) : []);
  const shown = $derived(all ? list : list.slice(0, FEW));
  const look = $derived.by(() => {
    const shade = theme.shown === "light" ? t("Light (theme)") : t("Dark (theme)");
    return theme.choice === "system" ? t("{theme} (Match Windows)", { theme: shade }) : shade;
  });
  const parts = $derived<ReportParts | null>(info ? { info, look, include } : null);
  const ready = $derived(!!parts && help.reportDraft.trim().length > 0);
  /** "Say what happened first" on the buttons until something's written. */
  const first = $derived(ready ? undefined : t("Say what happened first"));
  const thanks = $derived(around(t("Thank you. Your report's reference is {ref}. If you write to {email} about it, mention it.", { email: SUPPORT_EMAIL }), "{ref}"));
  /** No motion with Windows' "Animation effects" off. */
  const motion = $derived(reduced() ? 0 : 200);

  onMount(() => {
    api.reportInfo().then(
      (i) => (info = i),
      (e) => app.toast(t("Couldn't read Pako's details: {why}", { why: String(e) }), "error"),
    );
    // Closed: the next time starts afresh (a report still on its way says how it went).
    return () => {
      help.reportSent = null;
      help.reportFailed = null;
    };
  });

  function close() {
    help.report = false;
  }

  function nudge() {
    field?.focus();
  }

  async function sendReport() {
    if (!parts || busy || help.reportSending) return;
    if (!ready) return nudge();
    help.reportSending = true;
    help.reportFailed = null;
    try {
      const ref = await api.reportSend(help.reportDraft.trim(), theme.choice, sentProblems(parts));
      help.reportDraft = "";
      // Closed meanwhile: a toast says it went.
      if (help.report) help.reportSent = ref;
      else app.toast(t("Report sent to Sookera. Its reference is #{ref}.", { ref }));
    } catch (e) {
      const why = sendFailText(e);
      if (help.report) help.reportFailed = why;
      else app.toast(t("The report wasn't sent. {why}", { why }), "error");
    } finally {
      help.reportSending = false;
    }
  }

  /** The Close button of "Sent to Sookera" takes the keyboard (Enter or Esc closes). */
  function focusNow(node: HTMLElement) {
    node.focus({ preventScroll: true });
  }

  async function send() {
    if (!parts || busy || sending) return;
    if (!ready) return nudge();
    busy = true;
    try {
      const plan = mailPlan(help.reportDraft, parts);
      // Too long for an email link: the details go on the clipboard first, the email says so.
      if (plan.clipboard) await api.copyText(plan.clipboard);
      await api.reportMail(plan.url);
      help.reportDraft = "";
      close();
      app.toast(
        plan.clipboard
          ? t("Your email app has the report. Paste the details where it says, check it, then send it.")
          : t("Your email app has the report. Check it, then send it."),
      );
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      busy = false;
    }
  }

  async function copy() {
    if (!parts || busy) return;
    if (!ready) return nudge();
    try {
      await api.copyText(reportText(help.reportDraft, parts));
      app.toast(t("Details copied. Paste them into an email to {email}.", { email: SUPPORT_EMAIL }));
    } catch (e) {
      app.toast(String(e), "error");
    }
  }

  async function saveAs() {
    if (!parts || busy) return;
    if (!ready) return nudge();
    if (!inApp) return app.toast(t("Preview mode: nothing is saved"), "error");
    const d = new Date();
    const stamp = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const path = await saveFile({
      title: t("Save the problem report"),
      defaultPath: t("Pako problem report {date}.txt", { date: stamp }),
      filters: [{ name: t("Text file"), extensions: ["txt"] }],
    });
    if (!path) return;
    try {
      await api.reportSave(path, reportText(help.reportDraft, parts));
      app.toast(t("Saved. Attach the file to an email to {email}.", { email: SUPPORT_EMAIL }));
    } catch (e) {
      app.toast(String(e), "error");
    }
  }

  function key(e: KeyboardEvent) {
    // Ctrl+Enter, wherever the keyboard is in the dialog: Send report (its button's tooltip says so).
    if (e.key === "Enter" && e.ctrlKey && !sentRef) {
      e.preventDefault();
      sendReport();
    }
  }
</script>

<svelte:window onkeydown={key} />

<Modal title={t("Report a problem")} width={580} onclose={close}>
  {#if sentRef}
  <div class="done" in:pop={{ duration: motion ? 220 : 0, from: 0.96, y: 4 }}>
    <span class="tick" aria-hidden="true"><Icon name="check" size={20} stroke={2.6} /></span>
    <h3>{t("Sent to Sookera")}</h3>
    <p>{thanks[0]}<strong class="mono">#{sentRef}</strong>{thanks[1]}</p>
  </div>
  {:else}
  <label class="lbl" for="report-what">{t("What happened?")}</label>
  <textarea
    id="report-what"
    class="input"
    rows="3"
    bind:this={field}
    bind:value={help.reportDraft}
    readonly={sending}
    placeholder={t("What you did and what you saw. For example: a Google Drive folder stopped at 80% with “Server error (502)”, and Retry didn't help.")}
  ></textarea>

  <h3 class="lbl sent">{t("Sent with your report")}</h3>
  <section class="card" aria-label={t("Sent with your report")}>
    <dl class="facts">
      <dt>Pako</dt>
      <dd>{info ? pakoLine(info) : "…"}</dd>
      <dt>Windows</dt>
      <dd>{info ? info.windows : "…"}</dd>
      <dt>{t("Theme")}</dt>
      <dd>{look}</dd>
      <dt>{t("Time zone")}</dt>
      <dd>{zone()}</dd>
    </dl>

    <div class="incl">
      <div class="itext">
        <div class="t" id="report-incl">{t("Include recent problems")}</div>
        <div class="muted">
          {#if !info}
            {t("The last problems Pako noted on this PC.")}
          {:else if list.length}
            {t("The last {n} problems Pako noted. Links show only their site; file and folder names, IDs and your user name are left out.", { n: list.length })}
          {:else}
            {t("Pako hasn't noted any problems on this PC.")}
          {/if}
        </div>
      </div>
      <button class="switch" class:on={include} role="switch" aria-checked={include} aria-labelledby="report-incl" disabled={sending} onclick={() => (include = !include)}></button>
    </div>

    {#if include && list.length}
      <div class="reveal" transition:rowSlide={{ duration: motion ? 220 : 0 }}>
        <ol class="problems">
          {#each shown as p (`${p.time}-${p.area}-${p.message}`)}
            <li transition:rowSlide={{ duration: motion }} animate:flipIf={{ ...flipOpts, duration: motion ? flipOpts.duration : 0 }}>
              <div class="meta"><span class="area">{areaLabel(p.area)}</span><span class="when mono">{when(p.time)}</span></div>
              <div class="msg">{problemLine(p, info?.version ?? "")}</div>
            </li>
          {/each}
        </ol>
        {#if list.length > FEW}
          <button class="more" aria-expanded={all} onclick={() => (all = !all)}>
            <Icon name="chevron" size={11} stroke={2.6} />{all ? t("Show fewer") : t("Show all {n}", { n: list.length })}
          </button>
        {/if}
      </div>
    {/if}
  </section>

  {#if failed}
    <p class="failed" role="alert" in:fadeIn>{failed}</p>
  {:else}
    <p class="quiet">{t("Send report sends what's shown here to Sookera, kept private and used only to fix Pako. Nothing goes until you click it.")}</p>
  {/if}
  {/if}

  {#snippet actions()}
    {#if sentRef}
      <button class="btn primary" use:focusNow onclick={close}>{t("Close")}</button>
    {:else}
      <button class="btn ghost left" disabled={!ready} title={first} onclick={saveAs}>{t("Save as a file…")}</button>
      <button class="btn ghost" disabled={!ready} title={first} onclick={copy}><Icon name="copy" size={13} stroke={2} />{t("Copy details")}</button>
      <button class="btn" disabled={!ready || busy || sending} title={first ?? t("Opens your email app, to {email}", { email: SUPPORT_EMAIL })} onclick={send}><Icon name="mail" size={14} stroke={2} />{t("Send by email")}</button>
      <button class="btn primary" disabled={!ready || sending} title={first ?? t("Ctrl+Enter")} onclick={sendReport}><Icon name="send" size={14} stroke={2} />{sending ? t("Sending…") : t("Send report")}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .lbl {
    display: block;
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-3);
  }
  textarea {
    display: block;
    min-height: 78px;
  }
  .sent {
    margin-top: 16px;
  }
  .card {
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--raise);
  }
  .facts {
    display: grid;
    /* The names column fits the longest name (French ones are longer), never under 76px. */
    grid-template-columns: minmax(76px, max-content) minmax(0, 1fr);
    row-gap: 5px;
    margin: 0;
    font-size: 12.5px;
  }
  .facts dt {
    padding-right: 12px;
    color: var(--text-3);
  }
  .facts dd {
    margin: 0;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .incl {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .t {
    font-weight: 500;
    margin-bottom: 2px;
    color: var(--text);
  }
  .muted {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  /* What opens under the switch slides open; flow-root keeps its top margin inside the slide. */
  .reveal {
    display: flow-root;
  }
  .problems {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bg);
    overflow: hidden;
  }
  .problems li {
    padding: 7px 10px 8px;
    border-top: 1px solid var(--line);
  }
  .problems li:first-child {
    border-top: none;
  }
  .meta {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 11px;
    color: var(--text-4);
  }
  .area {
    font-weight: 600;
    color: var(--text-3);
  }
  .msg {
    margin-top: 2px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }
  .more {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: 8px;
    padding: 0;
    border: none;
    background: none;
    color: var(--text-3);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
    transition: color var(--fast) ease;
  }
  .more:hover {
    color: var(--text);
  }
  .more :global(svg) {
    transition: transform var(--mid) var(--ease);
  }
  .more[aria-expanded="true"] :global(svg) {
    transform: rotate(180deg);
  }
  .failed {
    margin: 14px 0 6px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--st-fail-text);
  }
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 18px 12px 10px;
  }
  .done h3 {
    margin: 12px 0 6px;
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }
  .done p {
    margin: 0;
    max-width: 400px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .done strong {
    color: var(--text);
  }
  .tick {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    color: var(--st-done-text);
    background: color-mix(in srgb, var(--st-done) 16%, transparent);
  }
  .quiet {
    margin: 14px 0 6px;
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--text-4);
  }
  .left {
    margin-right: auto;
  }
  .btn :global(svg) {
    flex: none;
  }
</style>
