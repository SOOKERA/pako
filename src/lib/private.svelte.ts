// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Private mode in the window: whether it's on and open, the private list, the PIN pad.
import { api, listen, type Download, type PrivateStatus, type SendMode } from "./api";
import { app } from "./state.svelte";
import { devices } from "./devices.svelte";
import { leaveWords, lockedNote, type WorkKind } from "./privateLeave";

/** Something running on private files while Private mode is open: a send with Direct, an
 *  upload, or Decrypt…. Its progress shows on the files' rows and in the details panel. */
export interface PrivateWork {
  key: string;
  kind: WorkKind;
  /** The private files it works on. */
  ids: string[];
  /** What it's called on screen ("Holiday.mp4", "3 files", "the folder Trip"). */
  what: string;
  /** Who it goes to (Direct: the contact or PC's name). */
  to?: string;
  /** The Direct transfer's or the upload's id. */
  job?: string;
  /** Decrypt…: bytes of the files already done, and of all. */
  done: number;
  total: number;
  /** When it started. */
  at: number;
  /** Direct or Upload has listed it at least once (a send not listed yet isn't finished). */
  seen?: boolean;
}

/** Leaving would lock Private mode while something runs: where you were going. */
interface LeaveAsk {
  words: string;
  go: () => void;
}

interface Progress {
  id: string;
  status: Download["status"];
  message: string | null;
  downloaded: number;
  size: number | null;
  name: string;
  speed: number;
}

class PrivateMode {
  status = $state<PrivateStatus>({ on: false, unlocked: false, lock: "", hasRecovery: false, wait: 0 });
  /** The private list (only while open). */
  list = $state<Download[]>([]);
  /** Live speeds of private downloads. */
  speeds = $state<Record<string, number>>({});
  /** The Private entry is picked: the Downloads page shows the private list. */
  showing = $state(false);
  /** The private list's details panel is shown: the same switch as every page's (the title
   *  bar's Details panel button, Ctrl+I), so one button shows or hides it everywhere. */
  get details() {
    return app.panelOpen;
  }
  set details(v: boolean) {
    if (app.panelOpen !== v) app.togglePanel();
  }
  /** The PIN or password pad is over the list. */
  pad = $state(false);
  /** Turning it on (Settings or the tip). */
  setupOpen = $state(false);
  /** Settings' "Change PIN or password…" / "New recovery code…". */
  manage = $state<"change" | "code" | null>(null);
  /** Sends, uploads and Decrypt… running on private files (see PrivateWork). */
  work = $state<PrivateWork[]>([]);
  /** The private video playing in Pako's player (its name), or null. */
  playing = $state<string | null>(null);
  /** "Leaving locks Private mode and stops …": asked before leaving while something runs. */
  leaveAsk = $state<LeaveAsk | null>(null);
  /** Locking right now (the window doesn't take a move it makes meanwhile for leaving). */
  locking = $state(false);
  /** Notes on screen that name a private file: they go the moment Private mode locks. */
  private named: number[] = [];

  /** A note that names a private file (it leaves the screen when Private mode locks). */
  say(text: string, kind: "info" | "error" = "info", action?: { label: string; run: () => void }) {
    this.named.push(app.toast(text, kind, action));
  }
  private forgetNamed() {
    if (!this.named.length) return;
    const gone = new Set(this.named);
    this.named = [];
    app.toasts = app.toasts.filter((t) => !gone.has(t.id));
  }
  /** Decrypt…: the current file's bytes so far, per piece of work. */
  private decryptAt = $state<Record<string, number>>({});
  /** "Save your changes into Private mode?" waiting (newest last). */
  saveAsks = $state<{ copy: string; name: string }[]>([]);
  private started = false;

  get open() {
    return this.status.unlocked;
  }

  async load() {
    try {
      this.status = await api.privateStatus();
    } catch {
      /* an older Pako window during an update: no Private mode */
    }
    await this.refreshList();
  }

  async refreshList() {
    if (!this.status.unlocked) {
      this.list = [];
      return;
    }
    try {
      this.list = await api.privateDownloads();
    } catch {
      this.list = [];
    }
  }

  start() {
    if (this.started) return;
    this.started = true;
    // A bell line, a pop-up or the tray window leading elsewhere: like any other way off the
    // private lists (the normal list itself, from the normal list, stays open).
    app.leaveGuard = (page, go) => {
      if (this.status.unlocked && (this.showing || page !== "downloads")) this.leaveTo(go);
      else go();
    };
    // Its status first: the tip only shows while Private mode is off.
    this.load().then(() => setTimeout(() => this.maybeTip(), 400));
    listen<PrivateStatus>("private-changed", (e) => {
      const was = this.status.unlocked;
      this.status = e.payload;
      if (!e.payload.unlocked) {
        // Locked: back to the normal list, nothing private left in the window (what was running
        // stopped with the lock; locking by itself never asks).
        this.showing = false;
        this.list = [];
        this.work = [];
        this.playing = null;
        this.leaveAsk = null;
        this.forgetNamed();
        this.speeds = {};
        this.dropFromBasket();
        // No question naming a private file stays on screen once it's locked (the copies are
        // saved back by Pako itself while locked), and a new recovery code shown is closed.
        this.saveAsks = [];
        if (this.manage === "code") this.manage = null;
        if (was) app.view = app.view === "downloads" ? "downloads" : app.view;
      } else {
        this.refreshList();
      }
    });
    listen("list-changed", () => {
      if (this.status.unlocked) this.refreshSoon();
      setTimeout(() => this.maybeTip(), 400);
    });
    listen<Progress[]>("progress", (e) => {
      if (!this.list.length) return;
      const next: Record<string, number> = {};
      for (const p of e.payload) {
        const d = this.list.find((x) => x.id === p.id);
        if (!d) continue;
        d.status = p.status;
        d.message = p.message;
        d.downloaded = p.downloaded;
        d.size = p.size;
        next[p.id] = p.speed ?? 0;
      }
      this.speeds = next;
    });
    // Sends and uploads: seen, finished (see prune). Asked a moment after the window's own
    // lists are refreshed, and every few seconds while one waits to be listed.
    const pruneSoon = () => setTimeout(() => this.prune(), 400);
    for (const ev of ["direct-changed", "devices-changed", "send-changed", "send-progress", "send-done"]) listen(ev, pruneSoon);
    setInterval(() => this.prune(), 5000);
    // Decrypt…: the file being decrypted moves its work on.
    listen<{ id: string; done: number; total: number }>("private-decrypt", (e) => {
      const w = this.work.find((x) => x.kind === "decrypt" && x.ids.includes(e.payload.id));
      if (w) this.decryptAt = { ...this.decryptAt, [w.key]: e.payload.done };
    });
    listen<{ copy: string; name: string }>("private-save-ask", (e) => {
      this.saveAsks = [...this.saveAsks, e.payload];
    });
  }

  /** The row the one-time tip sits under (the first "Done · private" one), until closed. */
  tipRow = $state<string | null>(null);
  private tipDone = false;
  /** Once, on the first "Done · private" row (Private mode off): where to keep these behind a
   *  PIN. Drawn right under that row (Table); marked seen when it shows, so it never comes back. */
  maybeTip() {
    if (this.tipDone || this.status.on || app.settings?.privateTipSeen) return;
    const row = app.downloads.find((d) => d.private && !d.vault && d.status === "completed");
    if (!row) return;
    this.tipDone = true;
    this.tipRow = row.id;
  }
  /** The tip was drawn on screen: it never comes back. */
  tipShown() {
    api.privateTipSeen().catch(() => {});
  }
  /** The tip's ✕, or its row left the list: gone for good. */
  closeTip() {
    this.tipRow = null;
  }

  private timer: ReturnType<typeof setTimeout> | undefined;
  refreshSoon() {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.refreshList(), 120);
  }

  /** The Private entry (or Ctrl+Shift+P): the list if open, else the pad. */
  enter() {
    if (!this.status.on) return;
    app.view = "downloads";
    if (this.status.unlocked) {
      this.showing = true;
      this.pad = false;
    } else {
      this.pad = true;
    }
  }

  /** The header's "Normal list": back to the normal list, Private mode stays open (the only
   *  way off the private list that doesn't lock it). */
  leave() {
    this.showing = false;
    this.pad = false;
  }

  /** Ctrl+Shift+P: open it, or lock it when it's showing. */
  toggle() {
    if (!this.status.on) return;
    if (this.status.unlocked && this.showing) this.lockByHand();
    else this.enter();
  }

  /** Lock (or Ctrl+Shift+P): locks at once; what it stopped is said after, naming nothing
   *  private ("Private mode locked. 1 send stopped."). */
  async lockByHand() {
    const n = lockedNote(this.running().map((w) => w.kind), !!this.playing);
    await this.lockNow();
    if (n) app.toast(n);
  }

  /** Locked: private files waiting in Upload's basket leave it (their names too). */
  dropFromBasket() {
    const kept = app.basket.filter((b) => !b.private);
    if (kept.length === app.basket.length) return;
    app.basket = kept;
    app.toast("Private mode locked, so its files left New upload. Open it and add them again to upload them.");
  }

  async lockNow() {
    this.locking = true;
    this.forgetNamed();
    this.leaveAsk = null;
    this.work = [];
    this.playing = null;
    this.showing = false;
    this.pad = false;
    this.list = [];
    this.dropFromBasket();
    this.saveAsks = [];
    if (this.manage === "code") this.manage = null;
    try {
      await api.privateLock().catch(() => {});
      this.status = { ...this.status, unlocked: false };
    } finally {
      this.locking = false;
    }
  }

  /** Turned on (setup's last step): Settings closes and the private list shows, so you always
   *  see that Private mode is open (bumped here, done by the page). */
  showList = $state(0);

  /** Opened with the PIN, password or recovery code: show the list. */
  async opened() {
    this.status = await api.privateStatus();
    await this.refreshList();
    this.pad = false;
    this.showing = true;
  }

  // ---------- work running on private files ----------

  private transfers() {
    return [...(app.direct?.transfers ?? []), ...(devices.data?.transfers ?? [])];
  }

  /** How far a piece of work is (0..1; null = not known yet) and its words on the row. */
  progress(w: PrivateWork): { frac: number | null; label: string; failed: boolean } {
    if (w.kind === "decrypt") {
      const done = w.done + (this.decryptAt[w.key] ?? 0);
      return { frac: w.total ? Math.min(1, done / w.total) : null, label: "Decrypting", failed: false };
    }
    if (w.kind === "send") {
      const who = w.to ?? "them";
      const t = this.transfers().find((x) => x.id === w.job);
      if (!t) return { frac: null, label: `Sending to ${who}`, failed: false };
      const label = t.state === "offered" ? `Waiting for ${who} to accept` : t.state === "waiting" ? `Waiting for ${who} to come back` : `Sending to ${who}`;
      return { frac: t.size ? Math.min(1, t.done / t.size) : null, label, failed: false };
    }
    const j = app.sends.find((x) => x.id === w.job);
    if (!j) return { frac: null, label: "Uploading", failed: false };
    if (j.status === "failed") return { frac: null, label: j.message ?? "The upload stopped", failed: true };
    const total = j.total ?? j.size;
    return { frac: total ? Math.min(1, j.sent / total) : null, label: "Uploading", failed: false };
  }

  /** Its send or upload as Direct / Upload lists it now (undefined: not listed). */
  private jobOf(w: PrivateWork): { failed: boolean } | undefined {
    if (w.kind === "send") return this.transfers().some((x) => x.id === w.job) ? { failed: false } : undefined;
    const j = app.sends.find((x) => x.id === w.job);
    return j ? { failed: j.status === "failed" } : undefined;
  }

  /** Still running (what leaving would stop): Decrypt… until it ends, a send or an upload while
   *  Direct or Upload lists it (or before they first do). A failed upload isn't. */
  running(): PrivateWork[] {
    return this.work.filter((w) => {
      if (w.kind === "decrypt") return true;
      const j = this.jobOf(w);
      return j ? !j.failed : !w.seen;
    });
  }

  /** After Direct or Upload changed: what they list now is seen; what they no longer list is
   *  finished and leaves (a failed upload stays shown, red, until it's tried again or removed
   *  in Upload). One never listed within a minute didn't start: it leaves too. */
  private prune() {
    if (!this.work.length) return;
    const now = Date.now();
    let changed = false;
    const next: PrivateWork[] = [];
    for (const w of this.work) {
      if (w.kind === "decrypt") {
        next.push(w);
        continue;
      }
      const listed = !!this.jobOf(w);
      if (listed && !w.seen) {
        next.push({ ...w, seen: true });
        changed = true;
      } else if (!listed && (w.seen || now - w.at > 60_000)) {
        changed = true;
      } else next.push(w);
    }
    if (changed) this.work = next;
  }

  /** The work on one private file (its row and its details panel), a failed upload included. */
  workOn(id: string): PrivateWork[] {
    return this.work.filter((w) => w.ids.includes(id));
  }

  /** A send with Direct started (`job`: its transfer's id). */
  sending(items: Download[], what: string, to: string, job: string, device: boolean) {
    if (device) devices.start();
    this.work = [...this.work, { key: `send:${job}`, kind: "send", ids: items.map((d) => d.id), what, to, job, done: 0, total: 0, at: Date.now() }];
  }

  /** Uploads private files from the private list: no trip to Upload, its progress shows here. */
  async upload(items: Download[], what: string, paths: string[], mode: SendMode | null, name: string | null, days: number, partMb: number | null, password: string | null) {
    const job = await api.sendStart(paths, mode, name, days, null, partMb, password);
    this.work = [...this.work, { key: `upload:${job}`, kind: "upload", ids: items.map((d) => d.id), what, job, done: 0, total: 0, at: Date.now() }];
    await app.refreshSends();
  }

  /** Decrypt…, running here (its dialog closes; the rows show how far it is). Each file in turn;
   *  what's done stays done if one fails (`made` on the error). */
  async decrypt(items: Download[], folder: string | null, dir: string, then: "off" | "downloads" | "leave"): Promise<string> {
    const key = `decrypt:${Date.now()}`;
    const total = items.reduce((n, d) => n + (d.size ?? d.downloaded), 0);
    const what = folder ? `the folder ${folder}` : items.length === 1 ? items[0].name : `${items.length} files`;
    this.work = [...this.work, { key, kind: "decrypt", ids: items.map((d) => d.id), what, done: 0, total, at: Date.now() }];
    let made = 0;
    let last = "";
    try {
      for (const d of items) {
        this.decryptAt = { ...this.decryptAt, [key]: 0 };
        last = await api.privateDecrypt(d.id, dir, then, !!folder);
        made++;
        this.work = this.work.map((w) => (w.key === key ? { ...w, done: w.done + (d.size ?? d.downloaded) } : w));
      }
      return last;
    } catch (e) {
      throw Object.assign(new Error(String(e)), { made });
    } finally {
      this.work = this.work.filter((w) => w.key !== key);
      const { [key]: _gone, ...rest } = this.decryptAt;
      this.decryptAt = rest;
    }
  }

  /** What leaving would stop, in words ("sending Holiday.mp4", "2 things in progress"), or "". */
  stopsWords(): string {
    return leaveWords(this.running().map((w) => ({ kind: w.kind, what: w.what })), this.playing);
  }

  /** Leaving the private screens (another page, Settings, Help, a sidebar entry): Private mode
   *  locks. While something runs, asked first ("Leave and lock" / "Stay"); with nothing
   *  running, locked at once. `go` takes you where you were going. Auto-lock never asks. */
  leaveTo(go: () => void) {
    if (!this.status.unlocked) {
      go();
      return;
    }
    const words = this.stopsWords();
    if (words) {
      this.leaveAsk = { words, go };
      return;
    }
    const locked = this.lockNow();
    go();
    return locked;
  }

  /** "Leave and lock" in that question. */
  async leaveAnyway() {
    const go = this.leaveAsk?.go;
    const locked = this.lockNow();
    go?.();
    await locked;
  }
}

export const priv = new PrivateMode();
