<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { onMount } from "svelte";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api, listen, type Settings, type WhenDone, type SendMode, type PakoLinkResult, type DriveSendStatus } from "./api";
  import { hhmm, minutesOf } from "./format";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import UpdatesSection from "./UpdatesSection.svelte";
  import AddToBrowser from "./AddToBrowser.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import HelpLink from "./help/HelpLink.svelte";
  import SegPick from "./SegPick.svelte";
  import { fadeIn, rowSlide } from "./motion";
  import { KINDS, prefOf, type NoticeKind } from "./notices";
  import { theme } from "./theme.svelte";
  import type { ThemeChoice } from "./api";

  let {
    section,
    linkResult = null,
    onclose,
  }: {
    section?: "accounts" | "browser" | "notices";
    /** Opened by the extension's "Open Pako" button: how connecting went. */
    linkResult?: PakoLinkResult | null;
    onclose: () => void;
  } = $props();

  let draft = $state<Settings>({ ...app.settings! });
  /** The settings as they were when this opened: Save sends only what was changed here. */
  const opened = $state.snapshot(app.settings!) as Settings;
  /** Theme: shows at once while picking; Save keeps it, Cancel goes back. */
  const openedTheme = theme.choice;
  let themePick = $state<ThemeChoice>(theme.choice);
  let themeKept = false;
  function pickTheme(t: ThemeChoice) {
    themePick = t;
    theme.preview(t);
  }
  function unpreview() {
    if (!themeKept && theme.choice !== openedTheme) theme.preview(openedTheme);
  }
  /** Cancel, × or Esc: a previewed theme goes back at once, while the window fades out (not after). */
  function cancel() {
    unpreview();
    onclose();
  }
  // Closed some other way (a tutorial ending, the extension's "Open Pako"): back then.
  $effect(() => unpreview);
  let accountsEl: HTMLElement | undefined = $state();
  let browserEl: HTMLElement | undefined = $state();
  let noticesEl: HTMLElement | undefined = $state();

  /** The sections, listed on the left so nothing has to be hunted for by scrolling. */
  let pane: HTMLDivElement | undefined = $state();
  let parts = $state<{ name: string; el: HTMLElement }[]>([]);
  let current = $state(0);
  let scroller: HTMLElement | null = null;
  /** While a click on the list is scrolling there, the list doesn't flicker through the sections in between. */
  let jumping = 0;

  function spot() {
    if (!scroller || !parts.length || Date.now() < jumping) return;
    const top = scroller.getBoundingClientRect().top;
    let at = 0;
    parts.forEach((p, i) => {
      if (p.el.getBoundingClientRect().top - top <= 48) at = i;
    });
    // At the very end the last section is the one in view, even when it is short.
    if (scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 2) at = parts.length - 1;
    current = at;
  }
  function go(i: number, smooth = true) {
    const p = parts[i];
    if (!p || !scroller) return;
    current = i;
    // Until the scroll ends ("scrollend" below); a long way can take more than half a second.
    jumping = Date.now() + (smooth ? 1500 : 0);
    const top = p.el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
    const calm = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    scroller.scrollTo({ top: i === 0 ? 0 : top, behavior: smooth && !calm ? "smooth" : "auto" });
  }

  onMount(() => {
    if (!pane) return;
    scroller = pane.closest<HTMLElement>(".body");
    parts = [...pane.querySelectorAll<HTMLElement>("section")].map((el) => ({ name: el.querySelector("h3")?.textContent?.trim() ?? "", el }));
    scroller?.addEventListener("scroll", spot, { passive: true });
    // A jump from the list has arrived: the section clicked stays lit, and from here the list
    // follows the scrolling again.
    const arrived = () => {
      jumping = 0;
    };
    scroller?.addEventListener("scrollend", arrived);
    const el = section === "accounts" ? accountsEl : section === "browser" ? browserEl : section === "notices" ? noticesEl : undefined;
    const i = parts.findIndex((p) => p.el === el);
    if (i > 0) setTimeout(() => go(i, false), 50);
    return () => {
      scroller?.removeEventListener("scroll", spot);
      scroller?.removeEventListener("scrollend", arrived);
    };
  });

  /** Writes the extension's files next to Pako's data and opens that folder. */
  async function openExtension() {
    await app.run(async () => api.openFolder(await api.extensionFiles()));
  }

  async function choose() {
    const picked = await pickFolder({ directory: true, defaultPath: draft.downloadDir || undefined });
    if (typeof picked === "string") draft.downloadDir = picked;
  }

  /** A whole number within limits (the engine refuses decimals and negatives). */
  function whole(v: unknown, fallback: number, min: number, max = Infinity) {
    const n = Math.round(Number(v));
    if (v == null || v === "" || !Number.isFinite(n)) return fallback;
    return Math.min(max, Math.max(min, n));
  }

  /** Save on its way: a second click does nothing. */
  let saving = false;
  async function save() {
    if (saving) return;
    saving = true;
    // What wasn't changed here keeps the value it has now, not the one copied at opening: "At
    // once" from the bottom bar, "Don't ask again" ticked in the Download with Pako window, where
    // uploads go after connecting Drive… all can change while this is open.
    const mine = $state.snapshot(draft) as Settings;
    const next = { ...(app.settings ? ($state.snapshot(app.settings) as Settings) : opened) };
    for (const k of Object.keys(mine) as (keyof Settings)[]) {
      if (JSON.stringify(mine[k]) !== JSON.stringify(opened[k])) (next as Record<string, unknown>)[k] = mine[k];
    }
    try {
      await api.setSettings({
        ...next,
        connections: whole(next.connections, 8, 1, 32),
        maxActive: whole(next.maxActive, 3, 1, 20),
        speedLimitKb: whole(next.speedLimitKb, 0, 0),
      });
      if (themePick !== openedTheme) await theme.set(themePick);
      themeKept = true;
      await app.refresh();
      app.toast("Settings saved");
      onclose();
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      saving = false;
    }
  }

  // Google Drive for uploads: its own sign-in, which only sees the files Pako uploads.
  let sendDrive = $state<DriveSendStatus | null>(null);
  let sendConnecting = $state(false);
  /** The newest check wins: an older answer arriving late never overwrites a newer one. */
  let sendAsk = 0;
  async function loadSendDrive() {
    const mine = ++sendAsk;
    try {
      const st = await api.sendDriveStatus();
      if (mine === sendAsk) sendDrive = st;
    } catch {
      // Keep what is shown (an account just connected stays shown) rather than "Checking…".
      if (mine === sendAsk && !sendDrive) sendDrive = null;
    }
  }
  onMount(() => {
    loadSendDrive();
    // Connected or disconnected from the Send tab or first-start setup while Settings is open.
    // Connecting Drive makes it the default for uploads (disconnecting goes back to Gofile): the
    // open draft follows, so Save doesn't put the old choice back.
    const un = listen("send-changed", async () => {
      await loadSendDrive();
      const fresh = await api.snapshot().catch(() => null);
      if (fresh?.settings) draft.sendHost = fresh.settings.sendHost;
    });
    return () => {
      un.then((f) => f());
    };
  });
  async function sendConnect() {
    sendConnecting = true;
    try {
      const email = await api.sendDriveSignIn();
      // Shown at once; the free space and the rest follow when Google answers.
      sendDrive = { ...(sendDrive ?? { missingNote: null, limit: null, used: null, error: null }), configured: true, signedIn: true, email };
      app.toast(`Google Drive connected for uploads (${email})`);
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      sendConnecting = false;
      loadSendDrive();
    }
  }
  async function sendDisconnect() {
    await app.run(() => api.sendDriveSignOut());
    loadSendDrive();
  }

  /** Settings › Notifications: the kinds under their headings. */
  const noticeGroups = (["Downloads", "Upload", "Direct", "Pako"] as const).map((g) => ({ name: g, kinds: KINDS.filter((k) => k.group === g) }));
  /** One switch of one kind. "Download finished › Pop-up" is the older notifyDone setting. */
  function setNotice(kind: NoticeKind, which: "popup" | "bell", on: boolean) {
    if (kind === "download-done" && which === "popup") {
      draft.notifyDone = on;
      return;
    }
    const all = { ...(draft.notices ?? {}) };
    all[kind] = { ...(all[kind] ?? {}), [which]: on };
    draft.notices = all;
  }

  /** Several files: the saved choice, else what the older compression setting meant. */
  const sendMode = $derived<SendMode>(
    draft.sendMode ?? (draft.sendCompression === "auto" || draft.sendCompression === "max" ? "compress" : "files"),
  );
</script>

<Modal title="Settings" width={780} autofocus={false} onclose={cancel}>
  <div class="set">
  <nav class="snav" data-tour="settings-nav" aria-label="Settings sections">
    {#if parts.length}<span class="hl" style="transform: translateY({current * 34}px)" aria-hidden="true"></span>{/if}
    {#each parts as p, i (p.name)}
      <button class:on={current === i} aria-current={current === i ? "true" : undefined} onclick={() => go(i)}>{p.name}</button>
    {/each}
    <div class="stut"><TutorialButton id="settings" /></div>
  </nav>
  <div class="pane" bind:this={pane}>
  <section data-tour="set-accounts" bind:this={accountsEl}>
    <h3>Accounts</h3>
    <div class="account">
      <div class="logo"><Icon name="upload" size={15} /></div>
      <div class="acc-text">
        <div class="acc-name">Google Drive for uploads</div>
        {#if !sendDrive}
          <div class="muted">Checking…</div>
        {:else if !sendDrive.configured}
          <div class="muted" in:fadeIn={{ duration: 160 }}>Not set up in this Pako. Uploads go to Gofile.</div>
        {:else if sendDrive.signedIn}
          <div class="ok" in:fadeIn={{ duration: 160 }}><span class="dot"></span>{sendDrive.email}</div>
        {:else}
          <div class="muted" in:fadeIn={{ duration: 160 }}>Upload big files to your own Drive. Pako only sees the files it uploads.</div>
        {/if}
      </div>
      {#if sendDrive?.configured}
        {#if sendDrive.signedIn}
          <button class="btn" onclick={sendDisconnect}>Disconnect</button>
        {:else}
          <button class="btn primary" disabled={sendConnecting} onclick={sendConnect}>{sendConnecting ? "Waiting for the browser…" : "Connect Google Drive"}</button>
        {/if}
      {/if}
    </div>
    <p class="muted small">Only used for uploads: downloads from Drive never need a sign-in. What you uploaded stays in your Drive if you disconnect.</p>
  </section>

  <section data-tour="set-window">
    <h3>Window</h3>
    <div class="line">
      <div>
        <div class="t">Theme</div>
        <div class="muted">Match Windows follows Windows' own light or dark mode. The sun or moon at the top of Pako switches too.</div>
      </div>
      <div class="ctrl">
        <SegPick
          label="Theme"
          options={[
            { value: "dark", label: "Dark" },
            { value: "light", label: "Light" },
            { value: "system", label: "Match Windows" },
          ]}
          value={themePick}
          onpick={(t) => pickTheme(t as ThemeChoice)}
        />
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Minimize to the taskbar tray</div>
        <div class="muted">Minimize hides Pako next to the clock; it keeps downloading. Click the tray icon for a small list, double-click it to open Pako.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.minimizeToTray} role="switch" aria-checked={draft.minimizeToTray} aria-label="Minimize to the taskbar tray" onclick={() => (draft.minimizeToTray = !draft.minimizeToTray)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Close to the taskbar tray</div>
        <div class="muted">The close button hides Pako instead of quitting. Quit from the tray icon's menu.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.closeToTray} role="switch" aria-checked={draft.closeToTray} aria-label="Close to the taskbar tray" onclick={() => (draft.closeToTray = !draft.closeToTray)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Ask before quitting while downloading or uploading</div>
        <div class="muted">Quitting stops downloads and uploads. Downloads continue next time you open Pako; uploads don't.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.confirmQuit} role="switch" aria-checked={draft.confirmQuit} aria-label="Ask before quitting while downloading or uploading" onclick={() => (draft.confirmQuit = !draft.confirmQuit)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Start Pako with Windows</div>
        <div class="muted">Pako opens when you sign in, so waiting downloads carry on by themselves.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.startWithWindows} role="switch" aria-checked={draft.startWithWindows} aria-label="Start Pako with Windows" onclick={() => (draft.startWithWindows = !draft.startWithWindows)}></button>
      </div>
    </div>
    {#if draft.startWithWindows}
      <div class="reveal" transition:rowSlide={{ duration: 220 }}>
      <div class="line sub">
        <div>
          <div class="t">Start in the tray</div>
          <div class="muted">No window at sign-in: Pako waits next to the clock.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.startInTray} role="switch" aria-checked={draft.startInTray} aria-label="Start in the tray" onclick={() => (draft.startInTray = !draft.startInTray)}></button>
        </div>
      </div>
      </div>
    {/if}
  </section>

  <section data-tour="set-notices" bind:this={noticesEl}>
    <h3>Notifications</h3>
    <p class="muted intro">Pop-ups show at the bottom right of the screen for a few seconds. The bell, at the top of Pako, keeps the important ones for 30 days.</p>
    <div class="line">
      <div>
        <div class="t">Do not disturb</div>
        <div class="muted">No pop-ups at all. The bell still collects.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.quiet} role="switch" aria-checked={draft.quiet} aria-label="Do not disturb" onclick={() => (draft.quiet = !draft.quiet)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Follow Windows' Do not disturb</div>
        <div class="muted">No pop-ups during full-screen games and videos, presentations, or Windows' Do not disturb. The bell still collects.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.followWindowsQuiet} role="switch" aria-checked={draft.followWindowsQuiet} aria-label="Follow Windows' Do not disturb" onclick={() => (draft.followWindowsQuiet = !draft.followWindowsQuiet)}></button>
      </div>
    </div>
    <div class="kinds" class:hushed={draft.quiet} role="table" aria-label="What shows as a pop-up and in the bell">
      <div class="khead" role="row">
        <span role="columnheader"><span class="sr">What</span></span>
        <span role="columnheader" class="col">Pop-up</span>
        <span role="columnheader" class="col">Bell</span>
      </div>
      {#each noticeGroups as g (g.name)}
        <div class="kgroup" role="rowgroup">
          <div class="kname" role="row"><span role="rowheader">{g.name}</span></div>
          {#each g.kinds as k (k.id)}
            {@const p = prefOf(draft, k.id)}
            <div class="krow" role="row">
              <span class="klabel" role="cell">{k.label}</span>
              <span class="col pop" role="cell">
                <button class="switch" class:on={p.popup} role="switch" aria-checked={p.popup} aria-label="{k.label}: pop-up" onclick={() => setNotice(k.id, "popup", !p.popup)}></button>
              </span>
              <span class="col" role="cell">
                <button class="switch" class:on={p.bell} role="switch" aria-checked={p.bell} aria-label="{k.label}: bell" onclick={() => setNotice(k.id, "bell", !p.bell)}></button>
              </span>
            </div>
          {/each}
        </div>
      {/each}
    </div>
    <p class="muted small">A download with "Notify when done" still gets its pop-up when Download finished is off (except during Do not disturb).</p>
  </section>

  <section data-tour="set-downloads">
    <h3>Downloads</h3>
    <label class="lbl" for="set-dir">Download folder</label>
    <div class="row">
      <input id="set-dir" class="input" bind:value={draft.downloadDir} spellcheck="false" />
      <button class="btn" onclick={choose}>Change</button>
    </div>

    <div class="line">
      <div>
        <div class="t">Connections per file</div>
        <div class="muted">Auto adds connections while it makes the file faster and stops when the server or your internet can't take more.</div>
      </div>
      <div class="ctrl">
        <span class="muted">Auto</span>
        <button class="switch" class:on={draft.autoConnections} role="switch" aria-checked={draft.autoConnections} aria-label="Auto connections" onclick={() => (draft.autoConnections = !draft.autoConnections)}></button>
        <input class="input num" type="number" min="1" max="32" bind:value={draft.connections} disabled={draft.autoConnections} aria-label="Connections per file" />
      </div>
    </div>

    <div class="line">
      <div>
        <div class="t">Downloads at the same time</div>
        <div class="muted">The rest wait in the queue and start on their own.</div>
      </div>
      <div class="ctrl"><input class="input num" type="number" min="1" max="20" bind:value={draft.maxActive} aria-label="Downloads at the same time" /></div>
    </div>

    <div class="line">
      <div>
        <div class="t">Speed limit</div>
        <div class="muted">Total for all downloads, in KB/s. 0 = no limit.</div>
      </div>
      <div class="ctrl"><input class="input num wide" type="number" min="0" bind:value={draft.speedLimitKb} aria-label="Speed limit in KB/s" /></div>
    </div>

    <div class="line">
      <div>
        <div class="t">Extract split archives</div>
        <div class="muted">When every part of a split archive (.part1.rar, .7z.001…) is downloaded, extract it into its folder. Each set can change this in its details panel.{app.sevenZip ? "" : " Needs 7-Zip (Features & updates)."}</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.extractDefault} role="switch" aria-checked={draft.extractDefault} aria-label="Extract split archives" onclick={() => (draft.extractDefault = !draft.extractDefault)}></button>
      </div>
    </div>

    <div class="line">
      <div>
        <div class="t">Video quality</div>
        <div class="muted">Used for new video links. If a video doesn't have it, Pako takes the closest lower one. Each video can change it in its details.</div>
      </div>
      <div class="ctrl">
        <select class="input vq" aria-label="Video quality" value={String(draft.videoQuality)} onchange={(e) => (draft.videoQuality = Number(e.currentTarget.value))}>
          {#each [[2160, "Up to 4K (2160p)"], [1440, "Up to 1440p"], [1080, "Up to 1080p"], [720, "Up to 720p"], [480, "Up to 480p"], [0, "Audio only (M4A)"]] as [v, label] (v)}
            <option value={String(v)}>{label}</option>
          {/each}
        </select>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Video preview quality</div>
        <div class="muted">The small player in the details panel. Lower uses less data; each video can change it.</div>
      </div>
      <div class="ctrl">
        <SegPick label="Video preview quality" options={[360, 480, 720].map((q) => ({ value: q, label: `${q}p` }))} value={draft.previewQuality} onpick={(q) => (draft.previewQuality = q)} />
      </div>
    </div>

    <div class="line">
      <div>
        <div class="t">Check Drive files when finished</div>
        <div class="muted">Compares each finished file with Google's own record of it, so a damaged file is caught.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.verifyChecksum} role="switch" aria-checked={draft.verifyChecksum} aria-label="Check Drive files when finished" onclick={() => (draft.verifyChecksum = !draft.verifyChecksum)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Keep the PC awake while downloading</div>
        <div class="muted">Windows won't go to sleep in the middle of a download. The screen can still turn off.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.keepAwake} role="switch" aria-checked={draft.keepAwake} aria-label="Keep the PC awake while downloading" onclick={() => (draft.keepAwake = !draft.keepAwake)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Only download at set hours</div>
        <div class="muted">Outside these hours downloads wait, and running ones pause and carry on when the hours start. "Download first" still starts one right away.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.scheduleOn} role="switch" aria-checked={draft.scheduleOn} aria-label="Only download at set hours" onclick={() => (draft.scheduleOn = !draft.scheduleOn)}></button>
      </div>
    </div>
    {#if draft.scheduleOn}
      <div class="reveal" transition:rowSlide={{ duration: 220 }}>
      <div class="line sub hours">
        <span class="muted">From</span>
        <input class="input num wide" type="time" aria-label="Start time" value={hhmm(draft.scheduleStart)} onchange={(e) => (draft.scheduleStart = minutesOf(e.currentTarget.value))} />
        <span class="muted">to</span>
        <input class="input num wide" type="time" aria-label="End time" value={hhmm(draft.scheduleEnd)} onchange={(e) => (draft.scheduleEnd = minutesOf(e.currentTarget.value))} />
        <span class="muted small">{draft.scheduleStart > draft.scheduleEnd ? "(overnight)" : ""}</span>
      </div>
      </div>
    {/if}
    <div class="line">
      <div>
        <div class="t">Catch copied links</div>
        <div class="muted">Links you copy in any app go to Links by themselves. Links already in Pako are skipped.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.catchClipboard} role="switch" aria-checked={draft.catchClipboard} aria-label="Catch copied links" onclick={() => (draft.catchClipboard = !draft.catchClipboard)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">When all downloads finish</div>
        <div class="muted">For this session only: Pako starts with "Do nothing" each time. Also in the bottom bar.</div>
      </div>
      <div class="ctrl">
        <select class="input when" aria-label="When all downloads finish" value={app.whenDone} onchange={(e) => app.setWhenDone(e.currentTarget.value as WhenDone)}>
          <option value="nothing">Do nothing</option>
          <option value="sleep">Sleep</option>
          <option value="shutDown">Shut down</option>
        </select>
      </div>
    </div>
  </section>

  <section data-tour="set-upload">
    <h3>Upload</h3>
    <div class="line">
      <div>
        <div class="t">Upload to</div>
        <div class="muted">
          {#if draft.sendHost === "drive"}
            Your own Google Drive: shared with anyone who has the link, kept until you delete it.
          {:else}
            Gofile: no account needed. Files are kept 10 days after their last download.
          {/if}
        </div>
      </div>
      <div class="ctrl">
        <select class="input sendmode" aria-label="Upload with" value={draft.sendHost === "drive" ? "drive" : "gofile"} onchange={(e) => (draft.sendHost = e.currentTarget.value as "gofile" | "drive")}>
          <option value="gofile">Gofile</option>
          <option value="drive">Google Drive</option>
        </select>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">When you upload several files</div>
        <div class="muted">
          {#if sendMode === "zip"}
            They're packed into one ZIP file: whoever opens the link downloads one file and opens it.
          {:else if sendMode === "compress"}
            One ZIP file, made smaller where it helps (documents, text). Takes longer to upload, and doesn't shrink videos or photos.
          {:else}
            Each file is uploaded as it is, a few at a time, into one folder. Whoever opens the link sees the files straight away.
          {/if}
          A single file goes as it is, unless you pick Compressed ZIP for it in the upload.
        </div>
      </div>
      <div class="ctrl">
        <select class="input sendmode" aria-label="When you upload several files" value={sendMode} onchange={(e) => (draft.sendMode = e.currentTarget.value as SendMode)}>
          <option value="files">As they are</option>
          <option value="zip">One ZIP</option>
          <option value="compress">Compressed ZIP</option>
        </select>
      </div>
    </div>
  </section>

  <section data-tour="set-browser" bind:this={browserEl}>
    <h3>Browser extension</h3>
    {#if linkResult}
      <div class="reveal" transition:rowSlide={{ duration: 220 }}>
      <div class="banner" class:bad={!linkResult.ok} role="status">
        <Icon name={linkResult.ok ? "check" : "alert"} size={14} />
        <span>{linkResult.ok ? "Connected. Go back to your browser and open Pako's popup again." : `Couldn't connect: ${linkResult.error ?? "unknown problem"}`}</span>
      </div>
      </div>
    {/if}
    <div class="line">
      <div>
        <div class="t">Connect Pako to your browsers</div>
        <div class="muted">Lets the Pako extension send downloads, links and videos to Pako. It talks to Pako directly on this PC: nothing goes over the internet.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.browserConnect} role="switch" aria-checked={draft.browserConnect} aria-label="Connect Pako to your browsers" onclick={() => (draft.browserConnect = !draft.browserConnect)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Ask before each browser download</div>
        <div class="muted">A small window shows the name, folder and quality first. Off: downloads from your browser start right away.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.askBrowser} role="switch" aria-checked={draft.askBrowser} aria-label="Ask before each browser download" onclick={() => (draft.askBrowser = !draft.askBrowser)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Show the small window when a browser download starts</div>
        <div class="muted">The tray's list opens at the bottom right so you can see your downloads at a glance. Pin it to keep it on top.</div>
      </div>
      <div class="ctrl">
        <button class="switch" class:on={draft.miniOnBrowser} role="switch" aria-checked={draft.miniOnBrowser} aria-label="Show the small window when a browser download starts" onclick={() => (draft.miniOnBrowser = !draft.miniOnBrowser)}></button>
      </div>
    </div>
    <div class="line">
      <div>
        <div class="t">Add Pako to your browser <HelpLink to="browser-extension.md#add-pako-to-your-browser" label="How to add Pako to your browser" /></div>
        <div class="muted">Pick your browser: Pako opens its extensions page and shows the three steps. Then right-click a link › Download with Pako, or use the Download button on YouTube videos.</div>
      </div>
      <div class="ctrl top">
        <button class="btn" onclick={openExtension}>Open the extension folder</button>
      </div>
    </div>
    <div class="browsers"><AddToBrowser /></div>
  </section>

  <UpdatesSection
    bind:autoUpdate={draft.autoUpdate}
    onsetup={() => {
      cancel();
      app.setupOpen = true;
    }}
  />
  </div>
  </div>

  {#snippet actions()}
    <button class="btn ghost" onclick={cancel}>Cancel</button>
    <button class="btn primary" data-tour="set-save" onclick={save}>Save</button>
  {/snippet}
</Modal>

<style>
  /* Sections on the left, the settings themselves on the right. */
  .set {
    display: flex;
    align-items: flex-start;
    gap: 24px;
  }
  .snav {
    position: sticky;
    top: 0;
    width: 168px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 2px 0 12px;
  }
  /* One highlight that slides to the section in view (buttons are 32px with 2px between). */
  .hl {
    position: absolute;
    top: 2px;
    left: 0;
    right: 0;
    height: 32px;
    border-radius: var(--radius);
    background: var(--active);
    pointer-events: none;
    transition: transform var(--mid) var(--ease);
  }
  .snav button {
    position: relative;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .snav button:hover:not(.on) {
    background: var(--raise);
    color: var(--text);
  }
  .snav button.on {
    color: var(--text);
    font-weight: 600;
  }
  .stut {
    margin: 10px 0 0 4px;
  }
  .pane {
    flex: 1;
    min-width: 0;
    /* room to bring the last, short section up to the top */
    padding-bottom: 8px;
  }
  .vq {
    width: 170px;
  }
  /* Rows that open under a switch slide open; flow-root keeps their top margin inside the slide. */
  .reveal {
    display: flow-root;
  }
  /* Each section is its own raised card: where one ends and the next begins shows at a glance. */
  section {
    padding: 16px 18px 18px;
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
  }
  section + section {
    margin-top: 14px;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--text);
  }
  .account {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border-radius: var(--radius-lg);
    background: var(--card);
    border: 1px solid var(--line);
  }
  .logo {
    width: 32px;
    height: 32px;
    border-radius: var(--radius);
    display: grid;
    place-items: center;
    background: rgba(90, 169, 255, 0.14);
    color: var(--src-drive);
    flex: none;
  }
  .acc-text {
    flex: 1;
    min-width: 0;
  }
  .acc-name {
    font-weight: 500;
    margin-bottom: 2px;
  }
  .ok {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--st-done);
    flex: none;
  }
  .muted {
    font-size: 12px;
    color: var(--text-3);
    line-height: 1.45;
  }
  .muted.small {
    margin: 10px 0 0;
    font-size: 11.5px;
  }
  .lbl {
    display: block;
    margin-bottom: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-3);
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .row .btn {
    height: 34px;
  }
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 14px 0 0;
    margin-top: 14px;
    border-top: 1px solid var(--line-soft);
  }
  .t {
    font-weight: 500;
    margin-bottom: 3px;
  }
  .ctrl {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  .ctrl.top {
    align-self: flex-start;
  }
  .browsers {
    margin-top: 12px;
  }
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid rgba(70, 201, 126, 0.35);
    border-radius: var(--radius-lg);
    background: rgba(70, 201, 126, 0.08);
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--st-done-text);
  }
  .banner :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .banner.bad {
    border-color: rgba(242, 75, 79, 0.35);
    background: var(--tint-fail);
    color: var(--st-fail-text);
  }
  .num {
    width: 64px;
    text-align: center;
    font-family: var(--mono);
  }
  .num.wide {
    width: 92px;
  }
  .num:disabled {
    opacity: 0.4;
  }
  .line.hours {
    justify-content: flex-start;
    gap: 10px;
    border-top: none;
    margin-top: 6px;
    padding-top: 0;
  }
  .line.hours .num.wide {
    width: 104px;
  }
  .small {
    font-size: 12px;
  }
  select.when {
    width: 140px;
  }
  select.sendmode {
    width: 214px;
  }
  /* Settings › Notifications: each kind with its two switches, under small headings. */
  .muted.intro {
    margin: 0 0 2px;
  }
  .kinds {
    margin-top: 14px;
    padding-top: 10px;
    border-top: 1px solid var(--line-soft);
  }
  .khead,
  .krow {
    display: grid;
    grid-template-columns: 1fr 72px 72px;
    align-items: center;
  }
  .khead {
    padding: 0 0 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .col {
    display: grid;
    place-items: center;
  }
  .kname {
    padding: 12px 0 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
  }
  .krow {
    min-height: 34px;
    border-top: 1px solid var(--line-soft);
  }
  .kname + .krow {
    border-top: none;
  }
  .klabel {
    color: var(--text-2);
  }
  /* Do not disturb on: the pop-up switches keep their choice but step back. */
  .col.pop {
    transition: opacity var(--mid) ease;
  }
  .kinds.hushed .col.pop {
    opacity: 0.45;
  }
</style>
