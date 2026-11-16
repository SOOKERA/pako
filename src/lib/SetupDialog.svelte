<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // First start (and "Run setup again" in Settings): what Pako should be able to do, how it
  // keeps itself up to date, and how it behaves with the taskbar tray. Choices are named by
  // what they do; the tools behind them are only mentioned in small print.
  import { app } from "./state.svelte";
  import { api, type ComponentKind, type ComponentStatus, type SetupChoices, type DriveSendStatus } from "./api";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import AddToBrowser from "./AddToBrowser.svelte";
  import SegPick from "./SegPick.svelte";
  import { theme } from "./theme.svelte";
  import type { ThemeChoice } from "./api";
  import { fadeIn, rowSlide } from "./motion";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";

  const s = app.settings!;
  /** Window choices are asked the first time only (they were already answered before). */
  const askWindow = !s.trayAsked || app.setupOpen;

  let archives = $state(true);
  let preview = $state(true);
  let videos = $state(true);
  let tools = $state<ComponentStatus[]>([]);
  api.componentsStatus().then((l) => (tools = l)).catch(() => {});
  const installed = (k: ComponentKind) => !!tools.find((c) => c.kind === k)?.installed;
  const videoReady = $derived(installed("ytDlp") && installed("deno") && installed("ffmpeg"));
  let autoUpdate = $state(s.autoUpdate);
  let minimize = $state(s.trayAsked ? s.minimizeToTray : true);
  let close = $state(s.trayAsked ? s.closeToTray : true);
  let startWithWindows = $state(s.startWithWindows);
  let startInTray = $state(s.startInTray);
  /** On by default the first time; "Run setup again" shows what it is now. */
  let browsers = $state(!s.setupDone || s.browserConnect);

  // Sending with your own Google Drive: offered only when this PC has its registration.
  let sendDrive = $state<DriveSendStatus | null>(null);
  api.sendDriveStatus().then((v) => (sendDrive = v)).catch(() => {});
  let connecting = $state(false);
  let skipDrive = $state(false);
  async function connectDrive() {
    connecting = true;
    try {
      const email = await api.sendDriveSignIn();
      app.toast(`Google Drive connected for uploads (${email})`);
      sendDrive = await api.sendDriveStatus();
      // Connected here = that's where sends go (Gofile stays one click away in Send).
      await app.setSendHost("drive");
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      connecting = false;
    }
  }

  /** Rough download sizes, for the total shown at the bottom. */
  const SIZE: Record<ComponentKind, number> = { sevenZip: 2, ffmpeg: 150, ytDlp: 18, deno: 40 };

  const toInstall = $derived(
    [
      archives && !app.sevenZip ? ("sevenZip" as const) : null,
      (preview || videos) && !app.ffmpeg ? ("ffmpeg" as const) : null,
      videos && !installed("ytDlp") ? ("ytDlp" as const) : null,
      videos && !installed("deno") ? ("deno" as const) : null,
    ].filter((k): k is ComponentKind => k !== null),
  );
  const totalMb = $derived(toInstall.reduce((n, k) => n + SIZE[k], 0));

  // Look: shows and is saved as soon as it's picked (like the folder below), so the rest of
  // the setup already shows in it.
  function pickTheme(t: ThemeChoice) {
    theme.set(t).catch((e) => app.toast(String(e), "error"));
  }

  // Where downloads go: Windows' Downloads folder unless picked here (saved straight away).
  let folder = $state(s.downloadDir);
  async function changeFolder() {
    const picked = await pickFolder({ directory: true, defaultPath: folder || undefined, title: "Where should downloads go?" });
    if (typeof picked !== "string" || picked === folder || !app.settings) return;
    const next = { ...app.settings, downloadDir: picked };
    try {
      await api.setSettings(next);
      app.settings = next;
      folder = picked;
    } catch (e) {
      app.toast(String(e), "error");
    }
  }

  /** Saving: a second click (or Esc right after) does nothing. */
  let saving = false;
  async function save(useChoices: boolean) {
    if (saving) return false;
    saving = true;
    // Only the setup's own answers: saving them never depends on the download folder being there.
    const choices: SetupChoices = useChoices
      ? {
          autoUpdate,
          browserConnect: browsers,
          ...(askWindow ? { minimizeToTray: minimize, closeToTray: close, startWithWindows, startInTray } : {}),
        }
      : {};
    app.setupOpen = false;
    try {
      await app.run(() => api.saveSetup(choices));
      await app.refresh();
    } finally {
      saving = false;
    }
    return true;
  }

  async function setUp() {
    const list = [...toInstall];
    if (!(await save(true)) || !list.length) return;
    const names: Record<ComponentKind, string> = { sevenZip: "7-Zip", ffmpeg: "ffmpeg", ytDlp: "yt-dlp", deno: "Deno" };
    app.toast(list.length === 1 ? "Getting it ready in the background…" : "Getting them ready in the background…");
    // One after the other, in the background (progress in Settings › Features & updates).
    (async () => {
      for (const k of list) await app.installComponent(k, names[k]);
      if (list.includes("ytDlp") || list.includes("deno")) await api.foundRecheck([]).catch(() => {});
    })();
  }
</script>

<Modal title="Set up Pako" width={540} closeLabel="Skip for now" outsideCloses={false} onclose={() => save(false)}>
  <p class="intro">Pick what you'd like Pako to do. You can change all of this later in Settings.</p>

  <h4>Look</h4>
  <div class="look">
    <SegPick
      label="Look"
      options={[
        { value: "dark", label: "Dark" },
        { value: "light", label: "Light" },
        { value: "system", label: "Match Windows" },
      ]}
      value={theme.choice}
      onpick={(t) => pickTheme(t as ThemeChoice)}
    />
  </div>
  <p class="hint">The sun or moon at the top of Pako switches it any time.</p>

  <h4>Downloads go to</h4>
  <div class="folder">
    <Icon name="folder" size={15} />
    {#key folder}<span class="path" title={folder} in:fadeIn={{ duration: 160 }}>{folder}</span>{/key}
    <button class="btn" onclick={changeFolder}>Change…</button>
  </div>
  <p class="hint">Files people send you in Direct go in a folder inside it, like "From Rado".</p>

  <h4>Features</h4>
  <div class="opts">
    <button class="opt" onclick={() => (archives = !archives)} role="checkbox" aria-checked={archives}>
      <span class="check" class:on={archives} aria-hidden="true">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
      </span>
      <span class="body">
        <span class="t">Extract split archives</span>
        <span class="d">When every part of a set (.part1.rar, .7z.001…) is downloaded, Pako unpacks it for you.</span>
        <span class="tool">{app.sevenZip ? "Ready · uses 7-Zip" : "Uses 7-Zip · about 2 MB"}</span>
      </span>
    </button>

    <button class="opt" onclick={() => (preview = !preview)} role="checkbox" aria-checked={preview}>
      <span class="check" class:on={preview} aria-hidden="true">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
      </span>
      <span class="body">
        <span class="t">Preview more video types before downloading</span>
        <span class="d">Watch MKV, AVI and other videos in the details panel to check it's the right one.</span>
        <span class="tool">{app.ffmpeg ? "Ready · uses ffmpeg" : "Uses ffmpeg · about 150 MB"}</span>
      </span>
    </button>

    <button class="opt" onclick={() => (videos = !videos)} role="checkbox" aria-checked={videos}>
      <span class="check" class:on={videos} aria-hidden="true">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
      </span>
      <span class="body">
        <span class="t">Download videos from YouTube and other sites</span>
        <span class="d">Paste a video or playlist link, pick the quality (or audio only), and Pako does the rest.</span>
        <span class="tool">{videoReady ? "Ready · uses yt-dlp, Deno and ffmpeg" : "Uses yt-dlp, Deno and ffmpeg · about 200 MB"}</span>
      </span>
    </button>

    <button class="opt" onclick={() => (autoUpdate = !autoUpdate)} role="checkbox" aria-checked={autoUpdate}>
      <span class="check" class:on={autoUpdate} aria-hidden="true">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
      </span>
      <span class="body">
        <span class="t">Keep Pako and its tools up to date automatically</span>
        <span class="d">Once a day, also while Pako stays open in the tray. Tools update quietly; a new Pako waits for you to restart.</span>
      </span>
    </button>
  </div>

  <h4>Browser</h4>
  <div class="opts">
    <button class="opt" onclick={() => (browsers = !browsers)} role="checkbox" aria-checked={browsers}>
      <span class="check" class:on={browsers} aria-hidden="true">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--accent-ink)" stroke-width="4"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
      </span>
      <span class="body">
        <span class="t">Add Pako to my browsers (right-click › Download with Pako)</span>
        <span class="d">Links, files and videos go from your browser to Pako, plus a Download button on video players (YouTube, Vimeo, Twitch, Facebook, Instagram). Everything stays on this PC.</span>
      </span>
    </button>
    {#if browsers}
      <div class="extra" transition:rowSlide={{ duration: 220 }}>
        <span class="tool">Add Pako's extension to your browser:</span>
        <AddToBrowser flat />
      </div>
    {/if}
  </div>

  {#if sendDrive?.configured && !skipDrive}
    <div class="reveal" transition:rowSlide={{ duration: 220 }}>
    <h4>Upload</h4>
    <div class="lines drive">
      <div class="line">
        <div>
          <div class="t">Upload big files to your Google Drive</div>
          {#if sendDrive.signedIn}
            <div class="muted ok" in:fadeIn={{ duration: 160 }}><span class="dot"></span>Connected: {sendDrive.email}</div>
          {:else}
            <div class="muted">Optional. Pako only sees the files it uploads. Without it, Upload uses Gofile (no account needed).</div>
          {/if}
        </div>
        {#if !sendDrive.signedIn}
          <div class="pair">
            <button class="btn ghost small" onclick={() => (skipDrive = true)}>Not now</button>
            <button class="btn small" disabled={connecting} onclick={connectDrive}>{connecting ? "Waiting for Google…" : "Connect"}</button>
          </div>
        {/if}
      </div>
    </div>
    </div>
  {/if}

  {#if askWindow}
    <h4>Window</h4>
    <div class="lines">
      <div class="line">
        <div>
          <div class="t">Minimize to the tray</div>
          <div class="muted">Minimize hides Pako next to the clock; it keeps downloading.</div>
        </div>
        <button class="switch" class:on={minimize} role="switch" aria-checked={minimize} aria-label="Minimize to the tray" onclick={() => (minimize = !minimize)}></button>
      </div>
      <div class="line">
        <div>
          <div class="t">Close to the tray</div>
          <div class="muted">The close button hides Pako; downloads keep going. Quit from the tray icon.</div>
        </div>
        <button class="switch" class:on={close} role="switch" aria-checked={close} aria-label="Close to the tray" onclick={() => (close = !close)}></button>
      </div>
      <div class="line">
        <div>
          <div class="t">Start Pako with Windows</div>
          <div class="muted">Waiting downloads carry on by themselves after you sign in.</div>
        </div>
        <button class="switch" class:on={startWithWindows} role="switch" aria-checked={startWithWindows} aria-label="Start Pako with Windows" onclick={() => (startWithWindows = !startWithWindows)}></button>
      </div>
      {#if startWithWindows}
        <div class="line sub" transition:rowSlide={{ duration: 220 }}>
          <div>
            <div class="t">Start in the tray</div>
            <div class="muted">No window at sign-in: Pako waits next to the clock.</div>
          </div>
          <button class="switch" class:on={startInTray} role="switch" aria-checked={startInTray} aria-label="Start in the tray" onclick={() => (startInTray = !startInTray)}></button>
        </div>
      {/if}
    </div>
  {/if}

  {#snippet actions()}
    <span class="total">
      {#if totalMb}<span class="tin" transition:fadeIn={{ duration: 160 }}><Icon name="download" size={13} />Downloads about {totalMb} MB</span>{/if}
    </span>
    <button class="btn ghost" onclick={() => save(false)}>Skip for now</button>
    <button class="btn primary" onclick={setUp}>Set up</button>
  {/snippet}
</Modal>

<style>
  .lines.drive {
    margin-bottom: 20px;
  }
  .pair {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .btn.small {
    height: 28px;
    font-size: 12px;
  }
  /* flow-root keeps the card's bottom margin inside the slide */
  .reveal {
    display: flow-root;
  }
  .ok {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--st-done-text);
  }
  .ok .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--st-done);
  }
  .intro {
    margin: 0 0 16px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .folder {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--raise);
    color: var(--text-3);
  }
  .folder .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .look {
    display: flex;
  }
  .hint {
    margin: 6px 0 18px;
    font-size: 12px;
    color: var(--text-3);
  }
  h4 {
    margin: 0 0 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .opts {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    overflow: hidden;
    margin-bottom: 20px;
  }
  .opt {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    width: 100%;
    padding: 12px 14px;
    border: none;
    background: var(--raise);
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background var(--fast) ease;
  }
  .opt + .opt {
    border-top: 1px solid var(--line);
  }
  .opt:hover:not(.soon) {
    background: var(--hover);
  }
  /* the card clips what sticks out: the keyboard ring stays inside */
  .opt:focus-visible {
    outline-offset: -2px;
  }
  .opt:first-child {
    border-radius: calc(var(--radius-lg) - 1px) calc(var(--radius-lg) - 1px) 0 0;
  }
  .opt:last-child {
    border-radius: 0 0 calc(var(--radius-lg) - 1px) calc(var(--radius-lg) - 1px);
  }
  .opt:only-child {
    border-radius: calc(var(--radius-lg) - 1px);
  }
  .opt .check {
    flex: none;
    margin-top: 1px;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .t {
    font-weight: 500;
    color: var(--text);
  }
  .d,
  .muted {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .muted {
    margin-top: 3px;
  }
  .tool {
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  /* Under a choice, lined up with its text (padding + check box + gap). */
  .extra {
    display: flex;
    flex-direction: column;
    padding: 0 14px 2px 41px;
    background: var(--raise);
  }
  .extra .tool {
    margin-top: 0;
  }
  /* Same card as the choices above. */
  .lines {
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--raise);
    margin-bottom: 8px;
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
  }
  .line + .line {
    border-top: 1px solid var(--line);
  }
  .line.sub {
    padding-left: 41px;
  }
  .total {
    display: inline-flex;
    align-items: center;
    margin-right: auto;
    font-size: 12px;
    color: var(--text-3);
  }
  .tin {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
