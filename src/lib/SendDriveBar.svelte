<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Your Google Drive for sending (the "Pako Send" sign-in, which only sees what Pako uploads):
  // connect it, see the room left and what Pako's sends take, and make room. Shown in Send when
  // Google Drive is where sends go.
  import { onMount } from "svelte";
  import { api, listen, type DriveSendStatus } from "./api";
  import { app } from "./state.svelte";
  import { bytes } from "./format";
  import Icon from "./Icon.svelte";
  import DriveRoom from "./DriveRoom.svelte";
  import { rowSlide } from "./motion";
  import { around, t } from "./i18n.svelte";

  let st = $state<DriveSendStatus | null>(null);
  /** Accounts that Drive uploads on the list went into, other than the one connected now. */
  const others = $derived(
    st?.configured ? app.driveAccounts.filter((a) => !st?.email || a.toLowerCase() !== st.email.toLowerCase()) : [],
  );
  let asking = $state(false);
  let connecting = $state(false);

  async function refresh() {
    if (asking) return;
    asking = true;
    try {
      st = await api.sendDriveStatus();
    } catch {
      st = null;
    } finally {
      asking = false;
    }
  }
  onMount(() => {
    refresh();
    // After each send (and sign-in / sign-out) the room changes.
    const un = listen("send-changed", () => {
      if (!app.sends.some((j) => j.status === "uploading")) refresh();
    });
    return () => {
      un.then((f) => f());
    };
  });

  async function connect() {
    connecting = true;
    try {
      const email = await api.sendDriveSignIn();
      app.toast(email ? t("Google Drive connected ({email})", { email }) : t("Google Drive connected"));
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      connecting = false;
      refresh();
    }
  }

  const free = $derived(st?.limit != null && st.used != null ? Math.max(0, st.limit - st.used) : null);
  const pct = $derived(st?.limit && st.used != null ? Math.min(100, (st.used / st.limit) * 100) : 0);

  // ---------- make room: delete older sends from your Drive (DriveRoom) ----------
  let room = $state(false);
  /** Pako's uploads in your Drive (not ones still uploading): "Make room…" is off without any. */
  const sends = $derived(app.sent.filter((s) => s.host === "drive" && !s.gone && !app.sends.some((j) => j.id === s.id && j.status === "uploading")));
  /** Also opened from Upload when an upload doesn't fit. */
  export function openRoom() {
    room = true;
  }
  const connectLine = $derived(around(t("{connect} to upload to your own Drive"), "{connect}"));
  const freeLine = $derived(around(t("{free} free in your Google Drive"), "{free}"));
</script>

<div class="dbar">
  <span class="ico"><Icon name="drive" size={15} /></span>
  {#if !st}
    <span class="dim">{t("Asking Google Drive…")}</span>
  {:else if !st.configured}
    <span class="dim">{st.missingNote ?? t("Uploading to Google Drive isn't set up in this Pako.")} {t("Gofile still works.")}</span>
  {:else if !st.signedIn}
    <div class="room">
      <span class="line">{connectLine[0]}<b>{t("Connect Google Drive")}</b>{connectLine[1]}</span>
      <span class="dim small">{t("Pako only sees the files it uploads, nothing else in your Drive.")}</span>
    </div>
    <button class="btn primary small" disabled={connecting} onclick={connect}>{connecting ? t("Waiting for Google…") : t("Connect Google Drive")}</button>
  {:else}
    <div class="room">
      {#if st.error}
        <span class="dim">{st.error}</span>
      {:else if free != null}
        <span class="line">{freeLine[0]}<b>{bytes(free)}</b>{freeLine[1]} <span class="dim">· {t("{used} of {limit} used", { used: bytes(st.used ?? 0), limit: bytes(st.limit ?? 0) })}</span></span>
        <span class="meter" class:full={pct > 90}><i style:width="{pct}%"></i></span>
      {:else}
        <span class="line">{t("Your Google Drive has no storage limit")} <span class="dim">· {t("{size} used", { size: bytes(st.used ?? 0) })}</span></span>
      {/if}
    </div>
    <span class="who dim" title={t("Connected for uploads as {who}", { who: st.email || t("Google account (in a sentence)") })}>
      {#if app.driveSends}{t("Pako's uploads: {n} · {size}", { n: app.driveSends, size: bytes(app.driveUsed) })}{:else}{t("No uploads in your Drive yet")}{/if}
    </span>
    <button class="btn ghost small" disabled={!sends.length} onclick={openRoom} title={sends.length ? t("Delete older uploads from your Google Drive") : t("Nothing uploaded to Google Drive yet")}>
      <Icon name="trash" size={13} />{t("Make room…")}
    </button>
    <button class="btn ghost small icon" aria-label={t("Check again")} title={t("Check again")} disabled={asking} onclick={refresh}><Icon name="retry" size={13} /></button>
  {/if}
</div>

{#if others.length}
  <div class="acct" role="status" transition:rowSlide={{ duration: 240 }}>
    <Icon name="alert" size={13} />
    <span>
      {t("Some uploads on your list are in the Google Drive of {accounts}. Connect that account to delete them, or to let Pako delete them on their date.", { n: others.length, accounts: others.join(", ") })}
    </span>
  </div>
{/if}

{#if room}
  <DriveRoom onclose={() => (room = false)} onfreed={refresh} />
{/if}

<style>
  .dbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 16px;
    border-bottom: 1px solid var(--line);
    background: var(--head);
    font-size: 12.5px;
    white-space: nowrap;
    min-width: 0;
  }
  .acct {
    flex: none;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--line);
    background: var(--tint-fail);
    color: var(--st-fail-text);
    font-size: 12px;
    line-height: 1.45;
  }
  .acct > :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .ico {
    display: grid;
    place-items: center;
    color: var(--src-drive);
    flex: none;
  }
  .room {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
    flex: 1;
  }
  .line {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .line b {
    font-weight: 600;
  }
  .meter {
    display: block;
    width: min(260px, 100%);
    height: 4px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    background: var(--src-drive);
  }
  .meter.full i {
    background: var(--st-retry);
  }
  .dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  .who {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .btn.small {
    flex: none;
    height: 28px;
    font-size: 12px;
  }
  .btn.icon {
    width: 28px;
    padding: 0;
    justify-content: center;
  }
  @container (max-width: 760px) {
    .who {
      display: none;
    }
  }
</style>
