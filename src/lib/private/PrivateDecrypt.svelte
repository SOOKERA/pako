<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "Decrypt…": where the readable copies go, and what happens to the private ones, in one
  // step. One file, a whole folder (its files go back to their places inside it), or several
  // picked files.
  import { untrack } from "svelte";
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { api, inApp, type Download } from "../api";
  import { app } from "../state.svelte";
  import { priv } from "../private.svelte";
  import Modal from "../Modal.svelte";
  import Icon from "../Icon.svelte";

  let { items, folder = null, onclose }: { items: Download[]; folder?: string | null; onclose: () => void } = $props();

  type Then = "off" | "downloads" | "leave";
  const one = untrack(() => items.length === 1 && !folder);
  const what = untrack(() => (folder ? `the folder ${folder}` : one ? items[0].name : `${items.length} files`));
  const CHOICES: { value: Then; title: string; text: string }[] = one
    ? [
        { value: "off", title: "Take it off the private list", text: "The readable copy stays where you saved it. Pako keeps no record of it, and the encrypted copy is deleted." },
        { value: "downloads", title: "Move it to Downloads", text: "It shows in your Downloads list like any finished download. The encrypted copy is deleted." },
        { value: "leave", title: "Leave it as is", text: "It stays in Private mode, still encrypted. You also have the readable copy." },
      ]
    : [
        { value: "off", title: "Take them off the private list", text: "The readable copies stay where you saved them. Pako keeps no record of them, and the encrypted copies are deleted." },
        { value: "downloads", title: "Move them to Downloads", text: "They show in your Downloads list like any finished download. The encrypted copies are deleted." },
        { value: "leave", title: "Leave them as they are", text: "They stay in Private mode, still encrypted. You also have the readable copies." },
      ];

  let dir = $state(app.settings?.downloadDir ?? "");
  let then = $state<Then>("off");

  async function change() {
    if (!inApp) return;
    const r = await pick({ directory: true, multiple: false, title: `Where to save ${what}, readable`, defaultPath: dir || undefined });
    if (typeof r === "string") dir = r;
  }

  /** Decrypt: the dialog closes and it runs in Private mode's list (the rows and the details
   *  panel show how far it is); a short note says when it's done, or what went wrong. */
  function go() {
    if (!dir) return;
    // Read before the dialog closes (its inputs go with it).
    const chosen = then;
    const into = dir;
    const files = items;
    const inFolder = folder;
    const count = files.length;
    const first = files[0]?.name ?? "";
    onclose();
    priv
      .decrypt(files, inFolder, into, chosen)
      .then(async (last) => {
        const where = inFolder ? `${into}\\${inFolder}` : last.slice(0, Math.max(last.lastIndexOf("\\"), last.lastIndexOf("/")));
        const name = inFolder ?? (one ? first : `${count} files`);
        const verb = count === 1 && !inFolder ? "is" : "are";
        priv.say(
          chosen === "downloads" ? `${name} ${verb} in your Downloads now.` : chosen === "off" ? `${name} ${verb} decrypted and off the private list.` : `Readable copy of ${name} saved.`,
          "info",
          { label: "Show", run: () => api.openFolder(where) },
        );
        await priv.refreshList();
        if (chosen === "downloads") await app.refresh();
      })
      .catch(async (e: Error & { made?: number }) => {
        // What's done stays done; the rest is unchanged in Private mode.
        const made = e.made ?? 0;
        priv.say(made ? `${made} of ${count} decrypted. Then: ${e.message}` : e.message, "error");
        await priv.refreshList();
      });
  }
</script>

<Modal title="Decrypt {what}" width={480} {onclose}>
  <div class="dec">
    <div class="field">
      <span class="lbl">{one ? "Save the readable file in" : folder ? "Save the readable folder in" : "Save the readable files in"}</span>
      <div class="folder">
        <Icon name="folder" size={14} />
        <span class="path" title={dir}>{dir || "Pick a folder"}</span>
        <button class="btn small" onclick={change}>Change…</button>
      </div>
      {#if folder}<span class="sub">Its files go back to their places inside a folder named {folder}.</span>{/if}
    </div>

    <div class="choices" role="radiogroup" aria-label="Then">
      <span class="lbl">Then</span>
      {#each CHOICES as c (c.value)}
        <button class="choice" class:on={then === c.value} role="radio" aria-checked={then === c.value} onclick={() => (then = c.value)}>
          <span class="dot" aria-hidden="true"></span>
          <span class="txt"><b>{c.title}</b><span>{c.text}</span></span>
        </button>
      {/each}
    </div>

    <p class="note">Anyone who can open that folder can read {one ? "the file" : "the files"}. {one ? "It's" : "They're"} marked as from the internet, so Windows checks {one ? "it" : "them"} before {one ? "it runs" : "they run"}.</p>

  </div>
  {#snippet actions()}
    <button class="btn ghost" onclick={onclose}>Cancel</button>
    <button class="btn primary" disabled={!dir} onclick={go}><Icon name="unlock" size={13} />Decrypt</button>
  {/snippet}
</Modal>

<style>
  .dec {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .lbl {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 6px 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    background: var(--raise);
    color: var(--text-2);
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-size: 13px;
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .choice {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--line);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease;
  }
  .choice:hover:not(:disabled) {
    background: var(--raise);
  }
  .choice.on {
    border-color: var(--priv-line);
    background: var(--priv-tint);
  }
  .dot {
    flex: none;
    width: 14px;
    height: 14px;
    margin-top: 2px;
    border-radius: 50%;
    border: 1.5px solid var(--line-strong);
    transition:
      border-color var(--fast) ease,
      box-shadow var(--fast) ease;
  }
  .choice.on .dot {
    border-color: var(--priv);
    box-shadow: inset 0 0 0 3px var(--panel), inset 0 0 0 8px var(--priv);
  }
  .txt {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .txt b {
    font-weight: 500;
    font-size: 13px;
  }
  .txt span {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-2);
  }
  .note {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-3);
  }
  .sub {
    font-size: 12px;
    color: var(--text-3);
  }
  @media (prefers-reduced-motion: reduce) {
    .choice,
    .dot {
      transition: none;
    }
  }
</style>
