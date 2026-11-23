<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Per-item settings, the same in both details panels: where it's saved, its place in the
  // queue, and how it tells you it's done (Off · Pop-up · Important).
  import type { Snippet } from "svelte";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import type { Priority } from "./api";
  import { ALERTS, type Alert } from "./alert";
  import Icon from "./Icon.svelte";
  import { rowSlide } from "./motion";
  import { t, type Key } from "./i18n.svelte";

  let {
    dest,
    isDefault = false,
    destLocked = false,
    priority,
    priorityOff,
    alert,
    count = 1,
    onDest,
    onResetDest,
    onPriority,
    onAlert,
    extra,
  }: {
    dest: string;
    /** Uses the tab's Save to (shown as "default"). */
    isDefault?: boolean;
    /** Already writing: the folder can't change any more. */
    destLocked?: boolean;
    priority: Priority;
    /** Why the priority doesn't apply to it (a download your browser makes), else unset. */
    priorityOff?: string;
    /** null: several items set differently. */
    alert: Alert | null;
    /** How many items these settings change (folder of files, several picked). */
    count?: number;
    onDest: (dir: string) => void;
    onResetDest?: () => void;
    onPriority: (p: Priority) => void;
    onAlert: (a: Alert) => void;
    extra?: Snippet;
  } = $props();

  const folderName = $derived(dest.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || dest);
  const levels: { key: Priority; label: Key; icon?: string }[] = [
    { key: "first", label: "First (priority)", icon: "up" },
    { key: "normal", label: "Normal (priority)" },
    { key: "later", label: "Later (priority)", icon: "down" },
  ];

  async function choose() {
    const picked = await pickFolder({ directory: true, defaultPath: dest || undefined });
    if (typeof picked === "string") onDest(picked);
  }
</script>

<section class="settings">
  <div class="label">{t("Settings")}{#if count > 1}<span class="muted">{` · ${t("for {n} files", { n: count })}`}</span>{/if}</div>

  <div class="field">
    <span class="k">{t("Save to")}</span>
    <div class="dest">
      <button class="folder" title={dest} disabled={destLocked} onclick={choose}>
        <Icon name="folder" size={14} />
        <span class="fn">{folderName}</span>
        {#if isDefault}<span class="tag">{t("default")}</span>{/if}
      </button>
      {#if onResetDest && !isDefault && !destLocked}
        <button class="icon-btn" title={t("Back to the default folder")} aria-label={t("Back to the default folder")} onclick={onResetDest}><Icon name="x" size={12} stroke={2.2} /></button>
      {/if}
    </div>
    {#if destLocked}<span class="note">{t("Already started: it stays in this folder.")}</span>{/if}
  </div>

  <div class="field">
    <span class="k">{t("Priority")}</span>
    <div class="seg" role="radiogroup" aria-label={t("Priority")}>
      {#each levels as l (l.key)}
        <button role="radio" aria-checked={priority === l.key} class:on={priority === l.key} disabled={!!priorityOff} title={priorityOff} onclick={() => onPriority(l.key)}>
          {#if l.icon}<Icon name={l.icon} size={11} stroke={2.6} />{/if}{t(l.label)}
        </button>
      {/each}
    </div>
    {#if priorityOff}<span class="note">{priorityOff}</span>{/if}
  </div>

  <div class="field" data-tour="notify-when-done">
    <span class="k">{t("Notify when done")}</span>
    <div class="seg" role="radiogroup" aria-label={t("Notify when done")}>
      {#each ALERTS as a (a.id)}
        <button role="radio" aria-checked={alert === a.id} class:on={alert === a.id} title={a.hint} onclick={() => onAlert(a.id)}>
          {#if a.id === "off"}<Icon name="bell-off" size={11} stroke={2.4} />{:else if a.id === "important"}<Icon name="bell" size={11} stroke={2.4} />{/if}{a.label}
        </button>
      {/each}
    </div>
    {#if alert === "important"}<span class="note" transition:rowSlide>{t("A big pop-up in the middle of the screen that stays until you click it.")}</span>{/if}
  </div>

  {@render extra?.()}
</section>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--line);
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .muted {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 500;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .k {
    font-size: 12px;
    color: var(--text-3);
  }
  .dest {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .folder {
    flex: 1;
    min-width: 0;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    /* a field you pick a folder in: the same edge as the other fields */
    border: 1px solid var(--control);
    border-radius: var(--radius);
    background: var(--field);
    color: var(--text);
    text-align: left;
    transition:
      border-color var(--fast) ease,
      background var(--fast) ease;
  }
  .folder:hover:not(:disabled) {
    border-color: var(--control-hover);
    background: var(--raise);
  }
  .folder:disabled {
    border-color: var(--line-strong);
    cursor: default;
    color: var(--text-2);
  }
  .fn {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    flex: none;
    font-size: 10.5px;
    color: var(--text-3);
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    background: var(--raise);
  }
  .note {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .seg {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius);
    background: var(--bar);
    border: 1px solid var(--btn-line);
  }
  .seg button {
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 12.5px;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  /* Not for this download (the browser makes it): shown, greyed, its reason under it. */
  .seg button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .seg button:hover:not(.on):not(:disabled) {
    background: var(--raise);
    color: var(--text);
  }
  .seg button.on {
    background: var(--active);
    color: var(--text);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
  }
  :global(:root[data-theme="light"]) .seg button.on {
    background: var(--bg);
    box-shadow:
      0 1px 2px rgba(20, 26, 46, 0.12),
      0 0 0 1px rgba(20, 26, 46, 0.04);
  }
</style>
