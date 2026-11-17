// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// All window state in one place: the list, live progress, filters, selection, folders, panels.
import { api, canDownload, isActive, isProblem, listen, type ComponentKind, type Download, type FoundItem, type MirrorView, type Progress, type SetView, type Settings, type Status, type UpdateView, type WhenDone, type SendJob, type SentItem, type SendMode, type SendHost, type PathSize, type DirectState, inApp } from "./api";
import { mostUrgent, sourceOf } from "./format";
import { matchesFilter, sortContacts, type DirectFilter } from "./direct";
import { devices } from "./devices.svelte";
import type { Go, Notice } from "./notices";

export type Library = "all" | "active" | "waiting" | "done" | "problems";
/** The two lists, switched from the top of the sidebar. */
export type View = "downloads" | "found" | "send" | "direct" | "devices" | "dashboard";
export type FoundFilter = "all" | "ready" | "needs" | "broken" | "checking";

/** The same file on several hosts: its rows, the one that downloads, and each row's host. */
export interface FoundMirror {
  key: string;
  /** Every host's row, in the order the links came in. */
  items: FoundItem[];
  /** The row that downloads (null = every host is broken). */
  chosen: FoundItem | null;
  /** The user picked that host. */
  picked: boolean;
  hostOf: Map<string, string>;
}

/** One line in the Found table: a link, a Drive folder header, or a file inside it. A file on
 *  several hosts is one line (`mirror`), showing the host it will use. */
export type FoundRow =
  | { kind: "file"; f: FoundItem; mirror?: FoundMirror }
  | { kind: "child"; f: FoundItem; folderKey: string; last: boolean; mirror?: FoundMirror }
  | { kind: "folder"; key: string; name: string; items: FoundItem[]; open: boolean; set?: { missing: string[] } };

/** The host a download (or one of its backups) uses, in the window's words. */
export function hostName(x: Pick<Download, "link" | "source">): string {
  return sourceOf(x as Download).label;
}

/** A download that left a host that failed for good: "Holiday.zip: continuing from B (A didn't
 *  work)". Null while it hasn't (or when every host failed: its message says so). */
export function hostSwitch(d: Download): string | null {
  const tried = d.tried ?? [];
  if (!tried.length || d.status === "failed" || d.status === "completed") return null;
  const from = tried.map(hostName).join(", ");
  return `${d.name}: continuing from ${hostName(d)} (${from} didn't work)`;
}

/** One host's row of a file on several hosts, in the host list's words. */
export function hostLook(f: FoundItem): { label: "Ready" | "Checking" | "Broken"; color: string } {
  if (f.state === "ready") return { label: "Ready", color: "var(--st-done-text)" };
  if (f.state === "checking" || f.state === "opening") return { label: "Checking", color: "var(--st-check-text)" };
  return { label: "Broken", color: "var(--st-fail-text)" };
}

function matchesFound(f: FoundItem, filter: FoundFilter) {
  switch (filter) {
    case "all":
      return true;
    case "ready":
      return f.state === "ready";
    case "needs":
      return f.state === "needsAccount";
    case "broken":
      return f.state === "broken";
    case "checking":
      return f.state === "checking" || f.state === "opening";
  }
}

/** One line in the table: a file, a folder header, or a file inside a folder. */
export type Row =
  | { kind: "file"; d: Download; inFolder: false }
  | { kind: "child"; d: Download; inFolder: true; folderKey: string; last: boolean }
  | { kind: "folder"; key: string; name: string; files: Download[]; open: boolean };

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "error";
  /** A button in the toast (e.g. Undo). */
  action?: { label: string; run: () => void };
}

/** Columns a list can be sorted by (click the header; again to reverse; a third time = queue order). */
export type SortKey = "name" | "source" | "size" | "progress" | "speed" | "status" | "link" | "date";
export interface Sort {
  key: SortKey;
  dir: 1 | -1;
}

/** One step of a component install (7-Zip, ffmpeg…), as the engine reports it. */
export interface ComponentProgress {
  stage: string;
  done: number;
  total: number | null;
}

/** Window preferences are saved as "pako.<name>" ("zd.<name>" up to Pako 0.1.2: moved over when read). */
export const LS = {
  get<T>(k: string, fallback: T): T {
    try {
      let v = localStorage.getItem("pako." + k);
      if (v === null) {
        const old = localStorage.getItem("zd." + k);
        if (old !== null) {
          localStorage.setItem("pako." + k, old);
          localStorage.removeItem("zd." + k);
          v = old;
        }
      }
      return v ? (JSON.parse(v) as T) : fallback;
    } catch {
      return fallback;
    }
  },
  set(k: string, v: unknown) {
    try {
      localStorage.setItem("pako." + k, JSON.stringify(v));
      localStorage.removeItem("zd." + k);
    } catch {
      /* not critical */
    }
  },
};

function folderKeyOf(d: Download): string | null {
  if (!d.group) return null;
  // Group files of the same Drive folder download (same top folder on disk).
  const dir = d.destDir.replace(/[\\/]+$/, "");
  const idx = dir.toLowerCase().lastIndexOf(("\\" + d.group).toLowerCase());
  const root = idx >= 0 ? dir.slice(0, idx + d.group.length + 1) : dir;
  return d.group + "|" + root.toLowerCase();
}

function matchesLibrary(s: Status, lib: Library) {
  switch (lib) {
    case "all":
      return true;
    case "active":
      return isActive(s);
    case "waiting":
      return s === "queued" || s === "paused";
    case "done":
      return s === "completed";
    case "problems":
      return isProblem(s);
  }
}

/** What happens to running uploads when Pako quits or restarts: they don't continue. */
export function uploadsStopNote(u: number) {
  return u === 1
    ? "An upload is running. It stops and doesn't continue after Pako closes: upload it again from Upload. One file or a ZIP starts over from the beginning; of several files, the ones already up stay on Gofile."
    : `${u} uploads are running. They stop and don't continue after Pako closes: upload them again from Upload. One file or a ZIP starts over from the beginning; of several files, the ones already up stay on Gofile.`;
}

/** One file or folder in the Send basket. `size` / `files` are null until measured. */
export interface BasketItem {
  path: string;
  name: string;
  dir: boolean;
  size: number | null;
  files: number | null;
  error: string | null;
}

class AppState {
  downloads = $state<Download[]>([]);
  live = $state<Record<string, Progress>>({});
  settings = $state<Settings | null>(null);
  /** 7-Zip is installed (extracting split archives works). */
  sevenZip = $state(false);
  /** ffmpeg is installed (MKV/AVI previews). */
  ffmpeg = $state(false);
  version = $state("");
  /** Installed with Pako Setup (false = the portable Pako.exe). */
  installed = $state(false);
  /** The terms of use wait to be accepted (the window shows only them; see consent.rs). */
  termsNeeded = $state(false);
  loaded = $state(false);

  view = $state<View>("downloads");

  // ---------- Send ----------
  sends = $state<SendJob[]>([]);
  sent = $state<SentItem[]>([]);
  sendCanDelete = $state(false);
  sendKeepNote = $state("");
  /** Download counts are being asked for right now. */
  sentChecking = $state(false);
  sentCheckedAt = $state<number | null>(null);
  /** Set when Gofile didn't share the counts at the last check. */
  sentCountsNote = $state<string | null>(null);
  /** What Pako's sends take in your Google Drive (items still listed), and how many. */
  driveUsed = $state(0);
  driveSends = $state(0);
  /** Something in the list of uploads couldn't be read (said once, until closed). */
  sentListNote = $state<string | null>(null);
  sentListNoteClosed = $state(false);
  /** The Google accounts the Drive uploads on the list went into. */
  driveAccounts = $state<string[]>([]);
  /** Uploads whose "Delete after" is being changed right now (their buttons wait). */
  changingSent = $state<string[]>([]);
  /** Where new sends go: Gofile (default) or your Google Drive. */
  sendHost = $derived<SendHost>(this.settings?.sendHost === "drive" ? "drive" : "gofile");
  /** Uploads running right now. */
  uploadingCount = $derived(this.sends.filter((s) => s.status === "uploading").length);

  // ---------- Direct (Pako to Pako) ----------
  direct = $state<DirectState | null>(null);
  /** Example people shown during the Direct tutorial when you have no contacts yet (never saved). */
  directExample = $state<DirectState | null>(null);
  /** What the Direct page shows: the examples during its tutorial, else your real contacts. */
  directView = $derived(this.directExample ?? this.direct);
  private filterBeforeExample: DirectFilter | null = null;
  /** Someone asks to connect, or offers files: shown on the Direct tab's badge. */
  directWaiting = $derived((this.direct?.requests?.length ?? 0) + (this.direct?.offers?.length ?? 0));
  /** Transfers moving right now. */
  directMoving = $derived((this.direct?.transfers ?? []).filter((t) => t.state === "sending" || t.state === "receiving").length);
  /** The contact picked in the sidebar (shared by the sidebar and the Direct tab). */
  directPicked = $state<string | null>(null);
  /** Contacts in the sidebar's order: online first. */
  directContacts = $derived(sortContacts(this.directView?.contacts ?? []));
  /** The sidebar's filter for Direct ("all", "online", "waiting", "favourites", "tag:<name>"). */
  directFilter = $state<DirectFilter>("all");
  /** Your tags for contacts, in your order. */
  directTags = $derived(this.directView?.tags ?? []);
  /** Contacts with something waiting for your answer (files offered). */
  directWaitingIds = $derived(new Set((this.directView?.offers ?? []).map((o) => o.contact)));
  /** Contacts under the current filter, in the sidebar's order. */
  directShown = $derived(this.directContacts.filter((c) => matchesFilter(c, this.directFilter, this.directWaitingIds)));
  /** How many contacts each filter shows. */
  directCounts = $derived.by(() => {
    const list = this.directContacts;
    const tags: Record<string, number> = {};
    for (const t of this.directTags) tags[t] = list.filter((c) => matchesFilter(c, `tag:${t}`, this.directWaitingIds)).length;
    return {
      all: list.length,
      online: list.filter((c) => c.online).length,
      favourites: list.filter((c) => c.favourite).length,
      tags,
    };
  });
  /** "Add contact" asked from the sidebar: the Direct tab opens its form. */
  directAdding = $state(false);
  /** Direct › From: the device whose contacts are shown and sent to (null: this PC). */
  directFrom = $state<string | null>(null);
  private directAsking = false;
  private directAgain = false;
  /** The Direct tutorial with no contacts yet: show example people so each step has something to
   *  point at (on), then put your real page back (off). Returns whether examples are shown. Only
   *  once Direct is running: while it starts (or if it couldn't), the page says so instead. */
  directExamples(on: boolean): boolean {
    if (on && this.direct?.ready && !(this.direct.contacts ?? []).length) {
      if (this.directExample) return true;
      this.directExample = exampleDirect(this.direct, this.settings?.downloadDir ?? "");
      this.filterBeforeExample = this.directFilter;
      this.directFilter = "all";
      this.directPicked = null;
      return true;
    }
    if (!on && this.directExample) {
      this.directExample = null;
      this.directPicked = null;
      if (this.filterBeforeExample) this.directFilter = this.filterBeforeExample;
      this.filterBeforeExample = null;
    }
    return false;
  }

  async refreshDirect() {
    // One question at a time; a change during it asks once more after.
    if (this.directAsking) {
      this.directAgain = true;
      return;
    }
    this.directAsking = true;
    try {
      this.direct = await api.directState();
    } catch {
      // Not there yet: the next change asks again.
    } finally {
      this.directAsking = false;
      if (this.directAgain) {
        this.directAgain = false;
        this.refreshDirect();
      }
    }
  }
  // ---------- Dashboard ----------
  /** Nothing counted yet on the Dashboard (null: not known yet). Its tutorial's steps follow it. */
  numbersEmpty = $state<boolean | null>(null);
  async checkNumbers() {
    try {
      this.numbersEmpty = (await api.numbersView("today")).since === null;
    } catch {
      // Not known: the tutorial looks at the page instead.
    }
  }

  /** Total upload speed. */
  uploadSpeed = $derived(this.sends.reduce((t, s) => t + (s.status === "uploading" ? s.speed : 0), 0));

  async refreshSends() {
    try {
      const s = await api.sendState();
      this.sends = s.jobs;
      this.sent = s.sent;
      this.sendCanDelete = s.canDelete;
      this.sendKeepNote = s.keepNote;
      this.sentChecking = !!s.checking;
      this.sentCheckedAt = s.checkedAt ?? null;
      this.sentCountsNote = s.countsNote ?? null;
      this.driveUsed = s.driveUsed ?? 0;
      this.driveSends = s.driveSends ?? 0;
      this.sentListNote = s.listNote ?? null;
      this.driveAccounts = s.driveAccounts ?? [];
    } catch {
      // the preview or an old Pako: no Send
    }
  }

  /** Asks Gofile, in the background, how often each sent item was downloaded (at most every
   *  few minutes). `button` = the refresh button. False = not now (asked a moment ago). */
  async checkSent(button = false): Promise<boolean> {
    if (!this.sent.length) return false;
    try {
      return await api.sentCheck(button);
    } catch {
      return false;
    }
  }

  /** Where new sends go (remembered). */
  async setSendHost(host: SendHost) {
    if (!this.settings || this.sendHost === host) return;
    const next = { ...this.settings, sendHost: host };
    this.settings = next;
    await this.run(() => api.setSettings(next));
  }

  /** "Delete after" for new uploads (days; 0 = keep), remembered. */
  uploadDeleteDays = $derived<number>([0, 3, 7, 30].includes(this.settings?.uploadDeleteDays ?? -1) ? this.settings!.uploadDeleteDays : 7);
  async setUploadDeleteDays(days: number) {
    if (!this.settings || this.uploadDeleteDays === days) return;
    const next = { ...this.settings, uploadDeleteDays: days };
    this.settings = next;
    await this.run(() => api.setSettings(next));
  }
  /** The uploaded item shown in the details panel (Upload tab). */
  sentPicked = $state<string | null>(null);
  /** "Delete now" asked for this uploaded item (the confirm dialog). */
  sentDeleting = $state<SentItem | null>(null);
  /** Changes when an upload gets deleted (null = keep it), said in a toast. */
  async setSentExpiry(s: SentItem, at: number | null) {
    // One change at a time per upload: a second click waits for the first.
    if (this.changingSent.includes(s.id)) return;
    this.changingSent = [...this.changingSent, s.id];
    try {
      await this.run(async () => {
        await api.sentSetExpiry(s.id, at);
        await this.refreshSends();
        this.toast(at ? `"${s.name}" will be deleted on ${new Date(at).toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" })}` : `"${s.name}" is kept until you delete it`);
      });
    } finally {
      this.changingSent = this.changingSent.filter((id) => id !== s.id);
    }
  }

  /** Sends files / folders (paths from the pickers). */
  /** `mode` = this send only ("files" / "zip" / "compress" / "parts"); null = the setting.
   *  `partMb`: "ZIP in parts"' part size (null = the setting). */
  /** `deleteDays` and `host`: kept by "Try again" (null = the settings). */
  async send(
    paths: string[],
    mode: SendMode | null = null,
    name: string | null = null,
    deleteDays: number | null = null,
    host: SendHost | null = null,
    partMb: number | null = null,
  ) {
    if (!paths.length) return;
    this.view = "send";
    await this.run(() => api.sendStart(paths, mode, name, deleteDays, host, partMb));
    await this.refreshSends();
  }

  // ---------- the Send basket: what the next send holds ----------
  /** Files and folders picked or dropped, not sent yet. Kept while you switch tabs. */
  basket = $state<BasketItem[]>(
    inApp
      ? []
      : [
          { path: "D:\\Videos\\Wedding photos", name: "Wedding photos", dir: true, size: 3_400_000_000, files: 214, error: null },
          { path: "D:\\Docs\\Contract final.pdf", name: "Contract final.pdf", dir: false, size: 2_100_000, files: 1, error: null },
          { path: "D:\\Music\\Album masters.zip", name: "Album masters.zip", dir: false, size: 840_000_000, files: 1, error: null },
        ],
  );
  /** Adds picked or dropped files and folders (each path once), then measures them. */
  async addToBasket(paths: string[]) {
    const have = new Set(this.basket.map((b) => b.path.toLowerCase()));
    const fresh = [...new Set(paths)].filter((p) => p && !have.has(p.toLowerCase()));
    this.view = "send";
    if (!fresh.length) return;
    const nameOf = (p: string) => p.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || p;
    this.basket = [...this.basket, ...fresh.map((path) => ({ path, name: nameOf(path), dir: false, size: null, files: null, error: null }))];
    try {
      const sizes = await api.sendSizes(fresh);
      const by = new Map(sizes.map((s: PathSize) => [s.path, s]));
      this.basket = this.basket.map((b) => {
        const s = by.get(b.path);
        return s ? { ...b, dir: s.dir, size: s.size, files: s.files, error: s.error } : b;
      });
    } catch (e) {
      this.toast(String(e), "error");
    }
  }
  removeFromBasket(path: string) {
    this.basket = this.basket.filter((b) => b.path !== path);
  }
  clearBasket() {
    this.basket = [];
  }

  async copyLink(link: string) {
    try {
      await api.copyText(link);
      this.toast("Link copied");
    } catch (e) {
      this.toast(String(e), "error");
    }
  }
  found = $state<FoundItem[]>([]);
  foundFilter = $state<FoundFilter>("all");
  /** Ticks the user set by hand. Rows without one are ticked when they can download. */
  picks = $state<Map<string, boolean>>(new Map());
  foundCollapsed = $state<Set<string>>(new Set());
  /** "Save to" for the Found tab; null = the download folder from Settings. */
  foundDest = $state<string | null>(null);
  /** Found row shown in the details panel: an item id, or "g:" + folder key. */
  foundFocus = $state<string | null>(null);

  /** Header sorting (null = queue order for Downloads, found order for Found). */
  sortDownloads = $state<Sort | null>(LS.get("sortDownloads", { key: "date", dir: -1 }));
  sortFound = $state<Sort | null>(LS.get("sortFound", null));

  library = $state<Library>("all");
  sourceFilter = $state<string | null>(null);
  search = $state("");
  selected = $state<Set<string>>(new Set());
  /** Last clicked download (shown in the details panel). */
  focus = $state<string | null>(null);
  /** Last clicked folder (shown in the details panel when no file is focused). */
  focusFolder = $state<string | null>(null);
  anchor: string | null = null;
  collapsed = $state<Set<string>>(new Set(LS.get<string[]>("collapsed", [])));
  panelOpen = $state<boolean>(LS.get("panelOpen", true));
  columns = $state<Record<string, number>>({ date: 112, ...LS.get("columns", { source: 128, progress: 156, speed: 92, status: 122 }) });
  toasts = $state<Toast[]>([]);
  private toastSeq = 0;

  // ---------- derived ----------

  counts = $derived.by(() => {
    const c = { all: 0, active: 0, waiting: 0, done: 0, problems: 0 } as Record<Library, number>;
    for (const d of this.downloads) {
      c.all++;
      for (const lib of ["active", "waiting", "done", "problems"] as Library[]) if (matchesLibrary(d.status, lib)) c[lib]++;
    }
    return c;
  });

  sources = $derived.by(() => {
    const map = new Map<string, { key: string; label: string; color: string; count: number }>();
    for (const d of this.downloads) {
      const s = sourceOf(d);
      const e = map.get(s.key) ?? { key: s.key, label: s.key === "web" ? "Web links" : s.label, color: s.color, count: 0 };
      e.count++;
      map.set(s.key, e);
    }
    return [...map.values()];
  });

  /** Downloads that pass the filters, in queue order. */
  visible = $derived.by(() => {
    const q = this.search.trim().toLowerCase();
    const list = this.downloads.filter(
      (d) =>
        matchesLibrary(d.status, this.library) &&
        (!this.sourceFilter || sourceOf(d).key === this.sourceFilter) &&
        (!q || d.name.toLowerCase().includes(q) || (d.group ?? "").toLowerCase().includes(q)),
    );
    const sort = this.sortDownloads;
    if (!sort) return list;
    const pct = (d: Download) => (d.status === "completed" ? 1 : d.size ? d.downloaded / d.size : 0);
    const val = (d: Download): string | number => {
      switch (sort.key) {
        case "name":
          return d.name.toLowerCase();
        case "source":
          return sourceOf(d).label.toLowerCase();
        case "size":
          return d.size ?? d.downloaded;
        case "progress":
          return pct(d);
        case "date":
          return d.date || d.addedAt;
        case "speed":
          return this.live[d.id]?.speed ?? 0;
        default:
          return d.status;
      }
    };
    return [...list].sort((a, b) => {
      const x = val(a);
      const y = val(b);
      return (x < y ? -1 : x > y ? 1 : 0) * sort.dir;
    });
  });

  /** Table rows: files, and folders with their files under them. */
  rows = $derived.by(() => {
    const rows: Row[] = [];
    const folders = new Map<string, Download[]>();
    const order: (Download | string)[] = [];
    for (const d of this.visible) {
      const key = folderKeyOf(d);
      if (!key) {
        order.push(d);
        continue;
      }
      if (!folders.has(key)) {
        folders.set(key, []);
        order.push(key);
      }
      folders.get(key)!.push(d);
    }
    for (const item of order) {
      if (typeof item !== "string") {
        rows.push({ kind: "file", d: item, inFolder: false });
        continue;
      }
      const files = folders.get(item)!;
      const open = !this.collapsed.has(item);
      rows.push({ kind: "folder", key: item, name: files[0].group ?? "Folder", files, open });
      if (open) {
        files.forEach((d, i) =>
          rows.push({ kind: "child", d, inFolder: true, folderKey: item, last: i === files.length - 1 }),
        );
      }
    }
    return rows;
  });

  /** Download ids that pass the filters. */
  visibleIds = $derived(this.visible.map((d) => d.id));
  /** Download ids in the order they appear on screen, a closed folder's files at its place
   *  (for Shift-click ranges). */
  screenIds = $derived(
    this.rows.flatMap((r) => (r.kind === "folder" ? (r.open ? [] : r.files.map((d) => d.id)) : [r.d.id])),
  );

  /** Selected downloads that are shown under the current filter/search. Actions use only
   *  these: rows hidden by a filter stay selected but are never paused/removed unseen. */
  selectedDownloads = $derived(this.visible.filter((d) => this.selected.has(d.id)));
  selectedIds = $derived(this.selectedDownloads.map((d) => d.id));

  focused = $derived(this.downloads.find((d) => d.id === this.focus) ?? null);

  // ---------- Found tab ----------

  /** Rows each filter shows: a file on several hosts counts once (as in the list). */
  foundCounts = $derived.by(() => {
    const keys = { all: new Set<string>(), ready: new Set<string>(), needs: new Set<string>(), broken: new Set<string>(), checking: new Set<string>() } as Record<FoundFilter, Set<string>>;
    for (const f of this.found) {
      const key = this.mirrorOf.get(f.id)?.key ?? f.id;
      for (const k of ["all", "ready", "needs", "broken", "checking"] as FoundFilter[]) if (matchesFound(f, k)) keys[k].add(key);
    }
    const c = {} as Record<FoundFilter, number>;
    for (const k of Object.keys(keys) as FoundFilter[]) c[k] = keys[k].size;
    return c;
  });

  foundVisible = $derived.by(() => {
    const q = this.search.trim().toLowerCase();
    const list = this.found.filter(
      (f) =>
        matchesFound(f, this.foundFilter) &&
        (!q || f.template.name.toLowerCase().includes(q) || (f.folder ?? "").toLowerCase().includes(q)),
    );
    const sort = this.sortFound;
    if (!sort) return list;
    const order: Record<string, number> = { ready: 0, checking: 1, opening: 2, needsAccount: 3, broken: 4 };
    const val = (f: FoundItem): string | number => {
      switch (sort.key) {
        case "name":
          return f.template.name.toLowerCase();
        case "source":
          return sourceOf(f.template).label.toLowerCase();
        case "size":
          return f.template.size ?? -1;
        case "date":
          return f.template.date || f.template.addedAt;
        default:
          return order[f.state] ?? 9;
      }
    };
    return [...list].sort((a, b) => {
      const x = val(a);
      const y = val(b);
      return (x < y ? -1 : x > y ? 1 : 0) * sort.dir;
    });
  });

  /** Split archives, as the engine sees them (Found links, and downloads per folder). */
  foundSetViews = $state<SetView[]>([]);
  downloadSets = $state<SetView[]>([]);
  /** Found sets with their items, for the rows and the details panel. */
  foundSets = $derived(
    this.foundSetViews.map((s) => ({ ...s, members: this.found.filter((f) => s.ids.includes(f.id)) })),
  );

  /** The same file on several hosts, as the engine sees it. */
  foundMirrorViews = $state<MirrorView[]>([]);
  /** Each grouped row's group (every host's row points to the same one). */
  mirrorOf = $derived.by(() => {
    const out = new Map<string, FoundMirror>();
    const byId = new Map(this.found.map((f) => [f.id, f]));
    for (const m of this.foundMirrorViews) {
      const items = m.ids.map((id) => byId.get(id)).filter((f): f is FoundItem => !!f);
      if (items.length < 2) continue;
      const g: FoundMirror = {
        key: m.key,
        items,
        chosen: (m.chosen && byId.get(m.chosen)) || null,
        picked: m.picked,
        hostOf: new Map(m.ids.map((id, i) => [id, m.hosts[i] ?? ""])),
      };
      for (const f of items) out.set(f.id, g);
    }
    return out;
  });

  /** Visible rows, a file on several hosts once: the row of the host it will use (or the first
   *  of its rows that passes the filter). */
  foundShown = $derived.by(() => {
    const seen = new Set<string>();
    const shown = new Set(this.foundVisible.map((f) => f.id));
    const out: { f: FoundItem; mirror?: FoundMirror }[] = [];
    for (const f of this.foundVisible) {
      const m = this.mirrorOf.get(f.id);
      if (!m) {
        out.push({ f });
        continue;
      }
      if (seen.has(m.key)) continue;
      seen.add(m.key);
      out.push({ f: m.chosen && shown.has(m.chosen.id) ? m.chosen : f, mirror: m });
    }
    return out;
  });

  foundRows = $derived.by(() => {
    const rows: FoundRow[] = [];
    const groups = new Map<string, { f: FoundItem; mirror?: FoundMirror }[]>();
    const order: ({ f: FoundItem; mirror?: FoundMirror } | string)[] = [];
    const setOf = new Map<string, string>();
    const missingOf = new Map<string, string[]>();
    for (const s of this.foundSets) {
      missingOf.set(s.key, s.missing);
      for (const f of s.members) setOf.set(f.id, s.key);
    }
    for (const it of this.foundShown) {
      const f = it.f;
      const setKey = setOf.get(f.id);
      if (setKey) {
        if (!groups.has(setKey)) {
          groups.set(setKey, []);
          order.push(setKey);
        }
        groups.get(setKey)!.push(it);
        continue;
      }
      if (!f.folder) {
        order.push(it);
        continue;
      }
      const key = f.folder + "|" + f.template.link;
      if (!groups.has(key)) {
        groups.set(key, []);
        order.push(key);
      }
      groups.get(key)!.push(it);
    }
    for (const item of order) {
      if (typeof item !== "string") {
        rows.push({ kind: "file", f: item.f, mirror: item.mirror });
        continue;
      }
      // A part on several hosts counts once (the host it will use).
      const members = groups.get(item)!;
      const items = members.map((m) => m.f);
      const open = !this.foundCollapsed.has(item);
      const set = missingOf.get(item);
      const name = set ? (this.foundSets.find((s) => s.key === item)?.base ?? "Archive") : (items[0].folder ?? "Folder");
      rows.push({ kind: "folder", key: item, name, items, open, set: set ? { missing: set } : undefined });
      if (open) members.forEach((m, i) => rows.push({ kind: "child", f: m.f, folderKey: item, last: i === members.length - 1, mirror: m.mirror }));
    }
    return rows;
  });

  /** Rows with a file on several hosts once (its first row given). */
  oncePerFile(items: FoundItem[]): FoundItem[] {
    const seen = new Set<string>();
    return items.filter((f) => {
      const m = this.mirrorOf.get(f.id);
      if (!m) return true;
      if (seen.has(m.key)) return false;
      seen.add(m.key);
      return true;
    });
  }

  /** Rows plus every other host's row of the files on several hosts among them. */
  withMirrors(items: FoundItem[]): FoundItem[] {
    const out = new Map<string, FoundItem>();
    for (const f of items) {
      const m = this.mirrorOf.get(f.id);
      for (const x of m ? m.items : [f]) out.set(x.id, x);
    }
    return [...out.values()];
  }

  /** Same file on several hosts: download from these rows' hosts (a split set's parts on one
   *  host at once). A host's row in the details panel stays there. */
  async pickHost(ids: string[]) {
    if (!ids.length) return;
    const focus = this.foundFocus;
    const follow = !!focus && !focus.startsWith("g:") && ids.some((id) => this.mirrorOf.get(id)?.items.some((f) => f.id === focus));
    await this.run(() => api.foundPickHost(ids));
    await this.refresh();
    if (follow) this.foundFocus = ids[0];
  }

  /** What the Found details panel shows and changes: one link, or the files of a folder
   *  that pass the filter/search (the same ones as its row). */
  foundFocusItems = $derived.by((): FoundItem[] => {
    const k = this.foundFocus;
    if (!k) return [];
    if (k.startsWith("g:")) {
      const key = k.slice(2);
      const set = this.foundSets.find((s) => s.key === key);
      if (set) {
        // A part on several hosts once (the row it shows).
        const shown = new Set(this.foundShown.map((r) => r.f.id));
        return set.members.filter((f) => shown.has(f.id));
      }
      return this.foundVisible.filter((f) => f.folder && f.folder + "|" + f.template.link === key);
    }
    const f = this.found.find((x) => x.id === k);
    return f ? [f] : [];
  });

  /** Ticked rows that can download (what the Download button acts on). A file on several hosts
   *  counts once: the engine downloads it from its chosen host. */
  picked = $derived(this.foundShown.map((r) => r.f).filter((f) => this.isTicked(f) && canDownload(f)));
  pickedSize = $derived(this.picked.reduce((s, f) => s + (f.template.size ?? 0), 0));
  /** Every ticked row (what Remove acts on), with all hosts of a file on several hosts. */
  ticked = $derived(this.withMirrors(this.foundShown.map((r) => r.f).filter((f) => this.isTicked(f))));

  isTicked(f: FoundItem) {
    const own = this.picks.get(f.id);
    if (own !== undefined) return own;
    // A host that joined a file on several hosts after you ticked or unticked it: the file's
    // tick, not a fresh one (else unticking a file would come undone when its next host is checked).
    for (const x of this.mirrorOf.get(f.id)?.items ?? []) {
      const p = this.picks.get(x.id);
      if (p !== undefined) return p;
    }
    return canDownload(f);
  }
  setTicked(items: FoundItem[], on: boolean) {
    const next = new Map(this.picks);
    // Every host of a file on several hosts together: picking another host keeps the tick.
    for (const f of this.withMirrors(items)) next.set(f.id, on);
    this.picks = next;
  }
  toggleFoundFolder(key: string) {
    const next = new Set(this.foundCollapsed);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    this.foundCollapsed = next;
  }

  /** Links being added right now (pastes still opening): Links shows "Adding links…". */
  adding = $state(0);

  /** Pasted links go to the Links tab ("found" in the code). It opens straight away, before
   *  the links are opened (a Drive folder or a link page can take seconds), so you see where
   *  they're going. */
  async addToFound(text: string) {
    this.view = "found";
    this.foundFocus = null;
    this.adding++;
    try {
      const before = new Set(this.found.map((f) => f.id));
      const r = await api.foundAdd(text);
      // A long list that can't be opened (offline, say): the first few, then how many more.
      for (const e of r.errors.slice(0, 3)) this.toast(e, "error");
      if (r.errors.length > 3) {
        const more = r.errors.length - 3;
        this.toast(more === 1 ? "1 more link couldn't be added" : `${more} more links couldn't be added`, "error");
      }
      if (r.skipped) this.toast(r.skipped === 1 ? "1 link was already in Links" : `${r.skipped} links were already in Links`);
      await this.refresh();
      // New links land at the end of the list: show the first one.
      const first = this.foundRows.find((row) => (row.kind === "folder" ? row.items : [row.f]).some((f) => !before.has(f.id)));
      if (first && this.view === "found") {
        const key = first.kind === "folder" ? "g:" + first.key : first.f.id;
        this.foundFocus = key;
        this.reveal(key);
      }
      return r.added;
    } catch (e) {
      this.toast(String(e), "error");
      return 0;
    } finally {
      this.adding--;
    }
  }

  totalSpeed = $derived(Object.values(this.live).reduce((s, p) => s + p.speed, 0));
  runningCount = $derived(this.downloads.filter((d) => isActive(d.status)).length);
  waitingCount = $derived(this.downloads.filter((d) => d.status === "queued").length);

  // ---------- loading ----------

  async refresh() {
    try {
      const s = await api.snapshot();
      // Keep the live numbers already shown, so nothing jumps back for a moment. Only while
      // it runs: after Start over a waiting item really is back at zero.
      for (const d of s.downloads) {
        const p = this.live[d.id];
        if (p && isActive(d.status)) {
          d.downloaded = Math.max(d.downloaded, p.downloaded);
        }
      }
      this.downloads = s.downloads;
      this.found = s.found ?? [];
      this.foundSetViews = s.foundSets ?? [];
      this.foundMirrorViews = s.foundMirrors ?? [];
      this.downloadSets = s.downloadSets ?? [];
      const foundIds = new Set(this.found.map((f) => f.id));
      const keepPicks = [...this.picks].filter(([id]) => foundIds.has(id));
      if (keepPicks.length !== this.picks.size) this.picks = new Map(keepPicks);
      // An "At once" change not saved yet stays on screen.
      this.settings = this.pendingMax !== null && s.settings ? { ...s.settings, maxActive: this.pendingMax } : s.settings;
      this.sevenZip = s.sevenZip ?? false;
      this.ffmpeg = s.ffmpeg ?? false;
      this.version = s.version ?? "";
      this.installed = s.installed ?? false;
      this.termsNeeded = s.termsNeeded ?? false;
      this.loaded = true;
      // Forget selections of downloads that are gone.
      const ids = new Set(s.downloads.map((d) => d.id));
      const kept = [...this.selected].filter((id) => ids.has(id));
      if (kept.length !== this.selected.size) this.selected = new Set(kept);
      if (this.focus && !ids.has(this.focus)) this.focus = null;
    } catch (e) {
      this.toast(String(e), "error");
    }
  }

  private refreshTimer: ReturnType<typeof setTimeout> | undefined;
  refreshSoon() {
    clearTimeout(this.refreshTimer);
    this.refreshTimer = setTimeout(() => this.refresh(), 100);
  }

  start() {
    this.refresh();
    const u1 = listen("list-changed", () => this.refreshSoon());
    const u2 = listen<Progress[]>("progress", (e) => {
      const next: Record<string, Progress> = {};
      for (const p of e.payload) next[p.id] = p;
      this.live = next;
      for (const d of this.downloads) {
        const p = next[d.id];
        if (p) {
          d.status = p.status;
          d.message = p.message;
          d.downloaded = p.downloaded;
          d.size = p.size;
          d.name = p.name;
        }
      }
    });
    // Late steps (after the install already ended) are ignored.
    const u3 = listen<ComponentProgress & { kind: ComponentKind }>("component-progress", (e) => {
      const { kind, ...p } = e.payload;
      if (this.componentBusy[kind]) this.componentBusy = { ...this.componentBusy, [kind]: p };
    });
    // Once a day at start: the tools updated themselves, and/or a new Pako is ready.
    const u4 = listen<string[]>("tools-updated", (e) => {
      const n = e.payload;
      this.toast(`Updated ${n.length === 1 ? n[0] : n.slice(0, -1).join(", ") + " and " + n[n.length - 1]}`);
    });
    const u5 = listen<UpdateView>("update-status", (e) => {
      const before = this.update;
      this.update = e.payload;
      const u = e.payload;
      if (u.latest && !(before?.latest === u.latest && before?.ready === u.ready)) {
        if (u.ready) this.toast(`Pako ${u.latest} is ready`, "info", { label: "Restart now", run: () => this.restartForUpdate() });
        else this.toast(`Pako ${u.latest} is available (Settings › Features & updates)`);
      }
    });
    api.updateStatus().then((u) => (this.update = u)).catch(() => {});
    // "When all downloads finish": the countdown, and back to "Do nothing" once it fired or stopped.
    const u6 = listen<{ phase: string; action?: WhenDone; seconds?: number }>("when-done", (e) => {
      const p = e.payload;
      if (p.phase === "countdown" && p.action && p.action !== "nothing") {
        this.whenDoneCountdown = { action: p.action, endsAt: Date.now() + (p.seconds ?? 60) * 1000 };
      } else {
        this.whenDoneCountdown = null;
        if (p.phase === "fired") this.whenDone = "nothing";
      }
    });
    this.refreshSends();
    this.refreshDirect();
    this.refreshNotices();
    const u12 = listen("notices-changed", () => this.refreshNotices());
    // A pop-up was clicked: Pako opened, now show its page.
    const u13 = listen<Go>("notice-go", (e) => this.goTo(e.payload));
    const u11 = listen("direct-changed", () => this.refreshDirect());
    // Connecting or disconnecting Drive also changes where uploads go: read the settings again.
    const u8 = listen("send-changed", () => {
      this.refreshSends();
      this.refresh();
    });
    const u9 = listen<SendJob[]>("send-progress", (e) => (this.sends = e.payload));
    const u10 = listen<{ name: string; link: string }>("send-done", (e) => {
      this.refreshSends();
      if (location.pathname.startsWith("/mini")) return;
      this.toast(`${e.payload.name}: link copied. Paste it to anyone.`, "info", {
        label: "Show",
        run: () => (this.view = "send"),
      });
    });
    const u7 = listen<{ added: number; skipped?: number }>("clipboard-caught", (e) => {
      const n = e.payload.added;
      const more = e.payload.skipped ?? 0;
      this.refreshSoon();
      const caught = n === 1 ? "1 link caught from the clipboard" : `${n} links caught from the clipboard`;
      const skipped = more > 0 ? (more === 1 ? ". 1 more link was skipped (too many at once)" : `. ${more} more links were skipped (too many at once)`) : "";
      this.toast(caught + skipped, "info", {
        label: "Show",
        run: () => (this.view = "found"),
      });
    });
    return () => {
      u12.then((f) => f());
      u13.then((f) => f());
      u11.then((f) => f());
      u8.then((f) => f());
      u9.then((f) => f());
      u10.then((f) => f());
      u6.then((f) => f());
      u7.then((f) => f());
      u1.then((f) => f());
      u2.then((f) => f());
      u3.then((f) => f());
      u4.then((f) => f());
      u5.then((f) => f());
    };
  }

  // ---------- sorting ----------

  /** Header click: sort by it, again = reverse, a third time = back to the normal order. */
  toggleSort(list: "downloads" | "found", key: SortKey) {
    const cur = list === "downloads" ? this.sortDownloads : this.sortFound;
    // Dates start with the newest; everything else A→Z / small→big.
    const first: 1 | -1 = key === "date" ? -1 : 1;
    const next: Sort | null = !cur || cur.key !== key ? { key, dir: first } : cur.dir === first ? { key, dir: -first as 1 | -1 } : null;
    if (list === "downloads") {
      this.sortDownloads = next;
      LS.set("sortDownloads", next);
    } else {
      this.sortFound = next;
      LS.set("sortFound", next);
    }
  }

  // ---------- keyboard: moving the focus ----------

  /** Rows in screen order, as focus keys ("g:<folder>" for a folder row, else the id). */
  private navKeys(): string[] {
    if (this.view === "send" || this.view === "dashboard") return [];
    if (this.view === "found") return this.foundRows.map((r) => (r.kind === "folder" ? "g:" + r.key : r.f.id));
    return this.rows.map((r) => (r.kind === "folder" ? "g:" + r.key : r.d.id));
  }
  private currentKey(): string | null {
    if (this.view === "found") return this.foundFocus;
    return this.focus ?? (this.focusFolder ? "g:" + this.focusFolder : null);
  }
  private setKey(key: string) {
    if (this.view === "found") {
      this.foundFocus = key;
    } else if (key.startsWith("g:")) {
      this.focusFolder = key.slice(2);
      this.focus = null;
    } else {
      this.focus = key;
      this.focusFolder = null;
    }
    // Keep the focused row in sight (the list scrolls to it, drawn or not).
    this.reveal(key);
  }
  private itemsOf(key: string): string[] {
    if (this.view === "found") {
      if (!key.startsWith("g:")) return [key];
      const r = this.foundRows.find((x) => x.kind === "folder" && x.key === key.slice(2));
      return r && r.kind === "folder" ? r.items.map((f) => f.id) : [];
    }
    if (!key.startsWith("g:")) return [key];
    const r = this.rows.find((x) => x.kind === "folder" && x.key === key.slice(2));
    return r && r.kind === "folder" ? r.files.map((d) => d.id) : [];
  }
  /** Where Shift+↑/↓ ranges start (a focus key). Mouse clicks in either list set it too. */
  navAnchor: string | null = null;

  /** ↑/↓ (delta ±1), Home/End (delta ±Infinity). With Shift, the rows passed over are selected
   *  (Downloads) or ticked (Found). */
  moveFocus(delta: number, extend: boolean) {
    const keys = this.navKeys();
    if (!keys.length) return;
    const cur = this.currentKey();
    const at = cur ? keys.indexOf(cur) : -1;
    const target = Number.isFinite(delta)
      ? Math.min(keys.length - 1, Math.max(0, at < 0 ? (delta > 0 ? 0 : keys.length - 1) : at + delta))
      : delta > 0
        ? keys.length - 1
        : 0;
    const key = keys[target];
    if (extend) {
      const from = this.navAnchor && keys.includes(this.navAnchor) ? keys.indexOf(this.navAnchor) : Math.max(at, 0);
      const [lo, hi] = from < target ? [from, target] : [target, from];
      const ids = keys.slice(lo, hi + 1).flatMap((k) => this.itemsOf(k));
      if (this.view === "found") {
        this.setTicked(this.found.filter((f) => ids.includes(f.id)), true);
      } else {
        this.selected = new Set(ids);
      }
      if (!this.navAnchor || !keys.includes(this.navAnchor)) this.navAnchor = keys[Math.max(at, 0)];
    } else {
      this.navAnchor = key;
      if (this.view === "downloads") this.anchor = key.startsWith("g:") ? null : key;
    }
    this.setKey(key);
  }

  /** Space: tick / untick the focused row (selection in Downloads, pick in Found). */
  toggleFocused() {
    const key = this.currentKey();
    if (!key) return;
    const ids = this.itemsOf(key);
    if (this.view === "found") {
      const items = this.found.filter((f) => ids.includes(f.id));
      this.setTicked(items, !items.every((f) => this.isTicked(f)));
    } else {
      const next = new Set(this.selected);
      const all = ids.every((id) => next.has(id));
      for (const id of ids) {
        if (all) next.delete(id);
        else next.add(id);
      }
      this.selected = next;
    }
  }

  /** A queue action (move to top/bottom) while the list is sorted: the rows don't move on screen,
   *  so say why and offer the queue order. */
  queueHint() {
    if (!this.sortDownloads) return;
    this.toast("Done. The list is sorted by " + this.sortDownloads.key + ", so the queue order isn't shown.", "info", {
      label: "Show queue order",
      run: () => {
        this.sortDownloads = null;
        LS.set("sortDownloads", null);
      },
    });
  }

  // ---------- undo ----------

  /** Removes Found links (Remove, Delete, Clear broken, right-click) and offers Undo. */
  async removeFound(ids: string[], what?: string) {
    const gone = new Set(ids);
    const items = this.found.filter((f) => gone.has(f.id));
    if (!items.length) return;
    const at = this.found.findIndex((f) => gone.has(f.id));
    await this.run(() => api.foundRemove(ids));
    await this.refresh();
    if (this.foundFocus && (gone.has(this.foundFocus) || this.foundFocusItems.length === 0)) this.foundFocus = null;
    const n = items.length;
    this.offerUndo(what ?? (n === 1 ? "1 link removed" : `${n} links removed`), () => api.foundRestore(items, at));
  }

  /** Things that can still be undone (removing Found links, or finished downloads from the
   *  list), newest last. Each has its own toast; Ctrl+Z takes the newest. */
  private undos: { toast: number; run: () => Promise<unknown>; until: number; view: View }[] = [];

  /** Offers Undo for a few seconds, in a toast (that undoes this action) and with Ctrl+Z. */
  offerUndo(text: string, run: () => Promise<unknown>) {
    const until = Date.now() + 8000;
    const toast = this.toast(text, "info", { label: "Undo", run: () => this.undoOne(toast) });
    this.undos = [...this.undos.filter((u) => u.until > Date.now()), { toast, run, until, view: this.view }];
  }

  /** Ctrl+Z: the newest action, only in the list it happened in. */
  async undo() {
    const u = this.undos.filter((x) => x.until > Date.now()).at(-1);
    if (u && u.view === this.view) await this.undoOne(u.toast);
  }

  private async undoOne(toast: number) {
    const u = this.undos.find((x) => x.toast === toast);
    this.undos = this.undos.filter((x) => x.toast !== toast && x.until > Date.now());
    this.toasts = this.toasts.filter((t) => t.id !== toast);
    if (!u || Date.now() > u.until) return;
    await this.run(async () => {
      await u.run();
      await this.refresh();
    });
  }

  // ---------- selection ----------

  /** Click on a row. Ctrl toggles, Shift selects a range, plain click selects only this one. */
  click(id: string, e: { ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }) {
    const ids = this.screenIds;
    if (e.shiftKey && this.anchor && ids.includes(this.anchor) && ids.includes(id)) {
      const a = ids.indexOf(this.anchor);
      const b = ids.indexOf(id);
      const [lo, hi] = a < b ? [a, b] : [b, a];
      const next = e.ctrlKey || e.metaKey ? new Set(this.selected) : new Set<string>();
      for (const x of ids.slice(lo, hi + 1)) next.add(x);
      this.selected = next;
    } else if (e.ctrlKey || e.metaKey) {
      this.toggle(id);
      this.anchor = id;
    } else {
      this.selected = new Set([id]);
      this.anchor = id;
    }
    // Shift+↑/↓ goes on from where the mouse put the anchor.
    this.navAnchor = this.anchor;
    this.focus = id;
    this.focusFolder = null;
  }

  toggle(id: string) {
    const next = new Set(this.selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    this.selected = next;
  }

  /** Folder checkbox: all of its files on, or all off. */
  toggleFolder(files: Download[]) {
    const next = new Set(this.selected);
    const all = files.every((f) => next.has(f.id));
    for (const f of files) {
      if (all) next.delete(f.id);
      else next.add(f.id);
    }
    this.selected = next;
  }

  selectAll() {
    this.selected = new Set(this.visibleIds);
  }

  clearSelection() {
    this.selected = new Set();
  }

  /** Right-click: act on the selection if the row is in it, else on this row only. */
  contextTargets(id: string): string[] {
    if (!this.selected.has(id)) {
      this.selected = new Set([id]);
      this.anchor = id;
      this.navAnchor = id;
    }
    this.focus = id;
    return this.selectedIds;
  }

  /** "When all downloads finish" (this session only; Pako starts with "Do nothing"). */
  whenDone = $state<WhenDone>("nothing");
  /** The countdown before sleeping / shutting down, while it runs. */
  whenDoneCountdown = $state<{ action: WhenDone; endsAt: number } | null>(null);

  async setWhenDone(a: WhenDone) {
    this.whenDone = a;
    await this.run(() => api.setWhenDone(a));
    if (a !== "nothing" && this.runningCount === 0 && this.waitingCount === 0) {
      this.toast(a === "sleep" ? "The PC will sleep after the next downloads finish" : "The PC will shut down after the next downloads finish");
    }
  }

  async cancelWhenDone() {
    this.whenDoneCountdown = null;
    this.whenDone = "nothing";
    await this.run(() => api.cancelWhenDone());
  }

  // ---------- notifications (the bell in the top bar) ----------

  /** The bell's lines, newest first. */
  notices = $state<Notice[]>([]);
  noticesUnread = $derived(this.notices.filter((n) => !n.read).length);
  /** The bell's panel is open (the list behind it takes no shortcuts). */
  bellOpen = $state(false);

  async refreshNotices() {
    try {
      this.notices = await api.noticesList();
    } catch {
      /* the bell keeps what it shows */
    }
  }

  /** Marks these lines as read (all of them with null). Shown at once, saved behind. */
  async readNotices(ids: string[] | null) {
    const unread = this.notices.filter((n) => !n.read && (!ids || ids.includes(n.id)));
    if (!unread.length) return;
    this.notices = this.notices.map((n) => (unread.includes(n) ? { ...n, read: true } : n));
    await this.run(() => api.noticesRead(ids));
  }

  async clearNotices() {
    this.notices = [];
    await this.run(() => api.noticesClear());
  }

  /** Shows where a notification leads: its page, and its item there (selected and scrolled to). */
  goTo(go: Go) {
    const id = go.id;
    switch (go.page) {
      case "downloads": {
        this.view = "downloads";
        const d = id ? this.downloads.find((x) => x.id === id) : undefined;
        if (!d) break;
        // Hidden by a filter, a search or a closed folder: show it.
        if (!this.visibleIds.includes(d.id)) {
          this.library = "all";
          this.sourceFilter = null;
          this.search = "";
        }
        const key = folderKeyOf(d);
        if (key && this.collapsed.has(key)) {
          const next = new Set(this.collapsed);
          next.delete(key);
          this.collapsed = next;
        }
        this.selected = new Set([d.id]);
        this.anchor = d.id;
        this.navAnchor = d.id;
        this.focus = d.id;
        this.focusFolder = null;
        this.reveal(d.id);
        break;
      }
      case "found": {
        this.view = "found";
        if (!id || !this.found.some((f) => f.id === id)) break;
        if (!this.foundVisible.some((f) => f.id === id)) {
          this.foundFilter = "all";
          this.search = "";
        }
        this.foundFocus = id;
        this.reveal(id);
        break;
      }
      case "send":
        this.view = "send";
        if (id && this.sent.some((s) => s.id === id)) this.sentPicked = id;
        break;
      case "direct":
        this.view = "direct";
        if (id && (this.direct?.contacts ?? []).some((c) => c.id === id)) {
          // Hidden by a filter or a tag: show everyone, or another conversation would open.
          if (!this.directShown.some((c) => c.id === id)) this.directFilter = "all";
          this.directPicked = id;
        }
        break;
      case "dashboard":
        this.view = "dashboard";
        break;
      case "devices":
        this.view = "devices";
        if (id ?? go.peer) devices.picked = (id ?? go.peer)!;
        break;
      case "update":
        this.restartForUpdate();
        break;
    }
  }

  /** A row the visible list should scroll to (by its data-key). `n` changes on every ask. */
  revealKey = $state<{ key: string; n: number } | null>(null);
  reveal(key: string) {
    this.revealKey = { key, n: (this.revealKey?.n ?? 0) + 1 };
  }

  /** Dialogs open right now (each Modal counts itself): shortcuts are off behind them. */
  modals = $state(0);

  /** When a folder was last opened: its files slide in, so the rows below must not also
   *  play their "move to the new place" animation (they'd jump up first). */
  folderOpenedAt = $state(0);

  toggleFolderOpen(key: string) {
    const next = new Set(this.collapsed);
    if (next.has(key)) {
      next.delete(key);
      this.folderOpenedAt = Date.now();
    } else next.add(key);
    this.collapsed = next;
    // A folder of a private window's downloads (its name, its place) is never written to disk.
    const hidden = new Set(this.downloads.filter((d) => d.private).map(folderKeyOf));
    LS.set("collapsed", [...next].filter((k) => !hidden.has(k)));
  }

  togglePanel() {
    this.panelOpen = !this.panelOpen;
    LS.set("panelOpen", this.panelOpen);
  }

  setColumn(name: string, width: number) {
    this.columns = { ...this.columns, [name]: Math.round(width) };
    LS.set("columns", this.columns);
  }

  // ---------- actions ----------

  /** Component installs running. Kept here, not in Settings, so closing and reopening
   *  Settings can't start the same install twice. */
  componentBusy = $state<Partial<Record<ComponentKind, ComponentProgress>>>({});
  /** "Run setup again" from Settings. */
  setupOpen = $state(false);
  /** Pako's own update (null until known). */
  update = $state<UpdateView | null>(null);

  /** Restarts into the downloaded update. Asks first if downloads run (they stop and continue
   *  after the restart), uploads run (they stop and don't continue) or files move in Direct or
   *  between your devices (they stop and have to be sent again), as Quit does. */
  async restartForUpdate() {
    const n = this.runningCount;
    const u = this.uploadingCount;
    const d = this.directMoving;
    if (n || u || d) {
      const lines: string[] = [];
      if (n) lines.push(`${n === 1 ? "A download is" : `${n} downloads are`} running. ${n === 1 ? "It continues" : "They continue"} where ${n === 1 ? "it" : "they"} left off after the restart.`);
      if (u) lines.push(uploadsStopNote(u));
      if (d) lines.push(`Files going to or from your contacts in Direct stop${n || u ? " too" : ""}: they have to be sent again.`);
      if (!(await askFirst("Restart Pako?", `${lines.join("\n\n")}\n\nRestart now?`))) return;
    }
    await this.run(() => api.updateRestart());
  }

  /** Video downloads: yt-dlp, Deno and ffmpeg (whatever is missing), then the video rows
   *  waiting for them are checked. */
  async installVideoTools() {
    const list: [ComponentKind, string][] = [
      ["ytDlp", "yt-dlp"],
      ["deno", "Deno"],
      ["ffmpeg", "ffmpeg"],
    ];
    const status = await api.componentsStatus().catch(() => []);
    for (const [k, name] of list) {
      if (!status.find((c) => c.kind === k)?.installed) await this.installComponent(k, name);
    }
    await api.foundRecheck([]).catch(() => {});
    await this.refresh();
  }

  /** The video tools are being installed. */
  get videoToolsBusy() {
    return !!(this.componentBusy.ytDlp || this.componentBusy.deno || this.componentBusy.ffmpeg);
  }

  async installComponent(kind: ComponentKind, name: string) {
    if (this.componentBusy[kind]) return;
    this.componentBusy = { ...this.componentBusy, [kind]: { stage: "Starting", done: 0, total: null } };
    try {
      const v = await api.componentInstall(kind);
      this.toast(`${name} ${v} installed`);
      await this.refresh();
    } catch (e) {
      this.toast(`${name}: ${e}`, "error");
    } finally {
      const next = { ...this.componentBusy };
      delete next[kind];
      this.componentBusy = next;
    }
  }

  async run(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      this.toast(String(e), "error");
    }
  }

  /** "At once" picked with − / + and not saved yet. */
  private pendingMax: number | null = null;
  private maxTimer: ReturnType<typeof setTimeout> | undefined;
  /** The last "At once" save: the next one waits for it, so saves never cross. */
  private maxSaving: Promise<unknown> = Promise.resolve();

  /** − / + of "At once": shown at once, saved a moment after the last click (quick clicks
   *  make one save, and saves go one after the other). */
  setMaxActive(n: number) {
    if (!this.settings) return;
    const maxActive = Math.min(20, Math.max(1, n));
    if (maxActive === this.settings.maxActive) return;
    this.settings = { ...this.settings, maxActive };
    this.pendingMax = maxActive;
    clearTimeout(this.maxTimer);
    this.maxTimer = setTimeout(() => {
      this.maxSaving = this.maxSaving.then(async () => {
        const max = this.pendingMax;
        if (max === null || !this.settings) return;
        await this.run(() => api.setSettings({ ...this.settings!, maxActive: max }));
        if (this.pendingMax === max) this.pendingMax = null;
      });
    }, 300);
  }

  /** Shows a toast; returns its id. */
  toast(text: string, kind: "info" | "error" = "info", action?: Toast["action"]) {
    const id = ++this.toastSeq;
    // The newest few: a burst never covers the window (an Undo that scrolled away still works
    // with Ctrl+Z).
    this.toasts = [...this.toasts, { id, text, kind, action }].slice(-5);
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), kind === "error" ? 9000 : action ? 8000 : 3500);
    return id;
  }

  folderStatus(files: Download[]): Status {
    return mostUrgent(files.map((f) => f.status));
  }
}

export const app = new AppState();

/** A yes / no question in Windows' own message box, named "Pako" (the web view's own box would
 *  say "tauri.localhost says"). */
async function askFirst(title: string, message: string): Promise<boolean> {
  if (!inApp) return confirm(message);
  try {
    const { confirm: ask } = await import("@tauri-apps/plugin-dialog");
    return await ask(message, { title, kind: "warning" });
  } catch {
    return confirm(message);
  }
}

/** Example people for the Direct tutorial: your own name and ID, three made-up contacts with a little
 *  history, one of them receiving a file right now. Shown only while the tutorial runs. */
function exampleDirect(real: DirectState, downloadDir: string): DirectState {
  const now = Date.now();
  const id = (c: string) => c.repeat(64);
  const alex = id("a");
  const sam = id("b");
  const jo = id("c");
  // `\\` is one backslash in the text (`\F` alone would lose it: "…DownloadsFrom Alex").
  const into = (name: string) => `${downloadDir || "Downloads"}\\From ${name}`;
  const base = { trusted: true, saveFolder: null, waiting: false, code: "0000 0000 0000 0000 0000", note: null, theirName: null, declined: false };
  return {
    ...real,
    tags: ["Friends", "Work", "Family"],
    contacts: [
      { ...base, id: alex, name: "Alex", autoAccept: true, lastSeen: now, added: now - 86400000 * 12, online: true, path: "direct", folder: into("Alex"), tags: ["Friends"], favourite: true },
      { ...base, id: sam, name: "Sam", autoAccept: false, lastSeen: now, added: now - 86400000 * 30, online: true, path: "relay", folder: into("Sam"), tags: ["Work"], favourite: false },
      { ...base, id: jo, name: "Jo", autoAccept: false, lastSeen: now - 3600000 * 26, added: now - 86400000 * 4, online: false, path: null, folder: into("Jo"), tags: ["Family"], favourite: false },
    ],
    requests: [],
    offers: [],
    transfers: [{ id: "ex-t1", contact: alex, outgoing: true, name: "Holiday video.mp4", files: 1, size: 1.8e9, done: 0.7e9, state: "sending", path: "direct", message: null }],
    history: [
      { id: "ex-h1", contact: alex, outgoing: false, name: "Photos from Saturday", files: 48, size: 412e6, date: now - 86400000 * 2, path: "direct", result: "done", message: null, savedTo: into("Alex"), durationMs: 9000, avgSpeed: 45e6 },
      { id: "ex-h2", contact: alex, outgoing: true, name: "Slides.pdf", files: 1, size: 6.2e6, date: now - 3600000 * 5, path: "direct", result: "done", message: null, savedTo: null, durationMs: 300, avgSpeed: 20e6 },
      { id: "ex-h3", contact: sam, outgoing: true, name: "Report Q3.xlsx", files: 1, size: 1.4e6, date: now - 86400000, path: "relay", result: "done", message: null, savedTo: null, durationMs: 900, avgSpeed: 1.5e6 },
    ],
  };
}
