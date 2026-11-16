<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Help › Report a problem: what happened, and what goes with it (shown in full). Send report
  // sends exactly that to Sookera's report service (a private list Sookera works through) and
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
  import { AREA_LABEL, attached, mailPlan, problemLine, reportText, sendFailText, sentProblems, SUPPORT_EMAIL, when, zone, type ReportParts } from "./report";

  /** Problems shown before "Show all". */
  const FEW = 3;

  let info = $state<ReportInfo | null>(null);
  let include = $state(true);
  let all = $state(false);
  let busy = $state(false);
  /** Send report on its way. */
  let sending = $state(false);
  /** Sent: the report's reference. */
  let sentRef = $state<string | null>(null);
  /** Why the last Send report didn't go. */
  let failed = $state<string | null>(null);
  let field = $state<HTMLTextAreaElement | null>(null);

  const list = $derived(info ? attached(info) : []);
  const shown = $derived(all ? list : list.slice(0, FEW));
  const look = $derived(`${theme.shown === "light" ? "Light" : "Dark"}${theme.choice === "system" ? " (Match Windows)" : ""}`);
  const parts = $derived<ReportParts | null>(info ? { info, look, include } : null);
  const ready = $derived(!!parts && help.reportDraft.trim().length > 0);
  /** No motion with Windows' "Animation effects" off. */
  const motion = $derived(reduced() ? 0 : 200);

  onMount(() => {
    api.reportInfo().then(
      (i) => (info = i),
      (e) => app.toast(`Couldn't read Pako's details: ${e}`, "error"),
    );
  });

  function close() {
    help.report = false;
  }

  function nudge() {
    field?.focus();
  }

  async function sendReport() {
    if (!parts || busy || sending) return;
    if (!ready) return nudge();
    sending = true;
    failed = null;
    try {
      sentRef = await api.reportSend(help.reportDraft.trim(), theme.choice, sentProblems(parts));
      help.reportDraft = "";
    } catch (e) {
      failed = sendFailText(e);
    } finally {
      sending = false;
    }
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
          ? "Your email app has the report. Paste the details where it says, check it, then send it."
          : "Your email app has the report. Check it, then send it.",
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
      app.toast(`Details copied. Paste them into an email to ${SUPPORT_EMAIL}.`);
    } catch (e) {
      app.toast(String(e), "error");
    }
  }

  async function saveAs() {
    if (!parts || busy) return;
    if (!ready) return nudge();
    if (!inApp) return app.toast("Preview mode: nothing is saved", "error");
    const d = new Date();
    const stamp = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const path = await saveFile({
      title: "Save the problem report",
      defaultPath: `Pako problem report ${stamp}.txt`,
      filters: [{ name: "Text file", extensions: ["txt"] }],
    });
    if (!path) return;
    try {
      await api.reportSave(path, reportText(help.reportDraft, parts));
      app.toast(`Saved. Attach the file to an email to ${SUPPORT_EMAIL}.`);
    } catch (e) {
      app.toast(String(e), "error");
    }
  }

  function key(e: KeyboardEvent) {
    // Ctrl+Enter: Send report, like the other dialogs' main button.
    if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      sendReport();
    }
  }
</script>

<Modal title="Report a problem" width={580} onclose={close}>
  {#if sentRef}
  <div class="done" in:pop={{ duration: motion ? 220 : 0, from: 0.96, y: 4 }}>
    <span class="tick" aria-hidden="true"><Icon name="check" size={20} stroke={2.6} /></span>
    <h3>Sent to Sookera</h3>
    <p>Thank you. Your report's reference is <strong class="mono">#{sentRef}</strong>. If you write to {SUPPORT_EMAIL} about it, mention it.</p>
  </div>
  {:else}
  <label class="lbl" for="report-what">What happened?</label>
  <textarea
    id="report-what"
    class="input"
    rows="3"
    bind:this={field}
    bind:value={help.reportDraft}
    onkeydown={key}
    placeholder="What you did and what you saw. For example: a Google Drive folder stopped at 80% with “Server error (502)”, and Retry didn't help."
  ></textarea>

  <h3 class="lbl sent">Sent with your report</h3>
  <section class="card" aria-label="Sent with your report">
    <dl class="facts">
      <dt>Pako</dt>
      <dd>{info ? `${info.version} (${info.installed ? "Setup" : "Portable"})` : "…"}</dd>
      <dt>Windows</dt>
      <dd>{info ? info.windows : "…"}</dd>
      <dt>Theme</dt>
      <dd>{look}</dd>
      <dt>Time zone</dt>
      <dd>{zone()}</dd>
    </dl>

    <div class="incl">
      <div class="itext">
        <div class="t" id="report-incl">Include recent problems</div>
        <div class="muted">
          {#if !info}
            The last problems Pako noted on this PC.
          {:else if list.length}
            The last {list.length === 1 ? "problem" : `${list.length} problems`} Pako noted. Links show only their site; file and folder names, IDs and your user name are left out.
          {:else}
            Pako hasn't noted any problems on this PC.
          {/if}
        </div>
      </div>
      <button class="switch" class:on={include} role="switch" aria-checked={include} aria-labelledby="report-incl" onclick={() => (include = !include)}></button>
    </div>

    {#if include && list.length}
      <div class="reveal" transition:rowSlide={{ duration: motion ? 220 : 0 }}>
        <ol class="problems">
          {#each shown as p (`${p.time}-${p.area}-${p.message}`)}
            <li transition:rowSlide={{ duration: motion }} animate:flipIf={{ ...flipOpts, duration: motion ? flipOpts.duration : 0 }}>
              <div class="meta"><span class="area">{AREA_LABEL[p.area] ?? AREA_LABEL.other}</span><span class="when mono">{when(p.time)}</span></div>
              <div class="msg">{problemLine(p, info?.version ?? "")}</div>
            </li>
          {/each}
        </ol>
        {#if list.length > FEW}
          <button class="more" aria-expanded={all} onclick={() => (all = !all)}>
            <Icon name="chevron" size={11} stroke={2.6} />{all ? "Show fewer" : `Show all ${list.length}`}
          </button>
        {/if}
      </div>
    {/if}
  </section>

  {#if failed}
    <p class="failed" role="alert" in:fadeIn>{failed}</p>
  {:else}
    <p class="quiet">Send report sends exactly what's shown here to Sookera, kept private and used only to fix Pako. Nothing goes until you click it.</p>
  {/if}
  {/if}

  {#snippet actions()}
    {#if sentRef}
      <button class="btn primary" onclick={close}>Close</button>
    {:else}
      <button class="btn ghost left" disabled={!ready} title={ready ? undefined : "Say what happened first"} onclick={saveAs}>Save as a file…</button>
      <button class="btn ghost" disabled={!ready} title={ready ? undefined : "Say what happened first"} onclick={copy}><Icon name="copy" size={13} stroke={2} />Copy details</button>
      <button class="btn" disabled={!ready || busy || sending} title={ready ? `Opens your email app, to ${SUPPORT_EMAIL}` : "Say what happened first"} onclick={send}><Icon name="mail" size={14} stroke={2} />Send by email</button>
      <button class="btn primary" disabled={!ready || sending} title={ready ? "Ctrl+Enter" : "Say what happened first"} onclick={sendReport}><Icon name="send" size={14} stroke={2} />{sending ? "Sending…" : "Send report"}</button>
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
    grid-template-columns: 76px minmax(0, 1fr);
    row-gap: 5px;
    margin: 0;
    font-size: 12.5px;
  }
  .facts dt {
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
