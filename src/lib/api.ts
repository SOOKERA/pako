// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import { mockContactNumbers, mockNotices, mockNumbers, mockParts, mockProgress, mockQr, mockReport, mockSend, mockSnapshot, mockStats } from "./mock";
import { showcase, showcaseDirect, showcaseDrive } from "./showcase";
import type { Go, Notice, NoticePref, PopupCard } from "./notices";

/** False when the page is opened in a normal browser for a design preview. */
export const inApp = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inApp) return tauriInvoke<T>(cmd, args);
  if (cmd === "snapshot") return Promise.resolve(structuredClone(mockSnapshot) as T);
  if (cmd === "add_links" || cmd === "found_add")
    return Promise.resolve({ added: 0, errors: ["Preview mode: links aren't checked or downloaded"] } as T);
  if (cmd === "found_download") return Promise.resolve(0 as T);
  if (cmd === "stats") return Promise.resolve(mockStats() as T);
  if (cmd === "numbers_view") return Promise.resolve(mockNumbers(String(args?.period ?? "month")) as T);
  if (cmd === "numbers_contact") return Promise.resolve(mockContactNumbers(String(args?.id ?? "")) as T);
  if (cmd === "numbers_export") return Promise.reject("Preview mode: nothing is saved");
  if (cmd === "numbers_reset") return Promise.resolve(undefined as T);
  // Showcase: every tool in place and up to date, and Pako just checked (the latest version).
  if (showcase && (cmd === "components_status" || cmd === "components_check"))
    return Promise.resolve(
      (["sevenZip", "ffmpeg", "ytDlp", "deno"] as const).map((kind) => {
        const v = { sevenZip: "26.06", ffmpeg: "9.0", ytDlp: "2026.11.12", deno: "2.9.8" }[kind];
        return {
          kind,
          installed: v,
          latest: v,
          update: false,
          installing: false,
          name: { sevenZip: "7-Zip", ffmpeg: "ffmpeg", ytDlp: "yt-dlp", deno: "Deno" }[kind],
          source: { sevenZip: "github.com/ip7z/7zip", ffmpeg: "github.com/BtbN/FFmpeg-Builds", ytDlp: "github.com/yt-dlp/yt-dlp", deno: "github.com/denoland/deno" }[kind],
          size: { sevenZip: 1_600_000, ffmpeg: 96_000_000, ytDlp: 18_000_000, deno: 42_000_000 }[kind],
        };
      }) as T,
    );
  if (showcase && (cmd === "update_status" || cmd === "update_check"))
    return Promise.resolve({ current: "1.0.0", latest: null, notes: null, ready: false, busy: false, error: null, checked: true } as T);
  if (cmd === "components_status" || cmd === "components_check")
    return Promise.resolve(
      (["sevenZip", "ffmpeg", "ytDlp", "deno"] as const).map((kind) => ({
        kind,
        installed: kind === "ytDlp" ? "2026.09.30" : null,
        latest: cmd === "components_check" ? { sevenZip: "26.06", ffmpeg: "9.0", ytDlp: "2026.10.26", deno: "2.9.8" }[kind] : null,
        update: cmd === "components_check" && kind === "ytDlp",
        installing: false,
        name: { sevenZip: "7-Zip", ffmpeg: "ffmpeg", ytDlp: "yt-dlp", deno: "Deno" }[kind],
        source: { sevenZip: "github.com/ip7z/7zip", ffmpeg: "github.com/BtbN/FFmpeg-Builds", ytDlp: "github.com/yt-dlp/yt-dlp", deno: "github.com/denoland/deno" }[kind],
        size: kind === "ytDlp" ? 18_000_000 : null,
      })) as T,
    );
  if (cmd === "update_status" || cmd === "update_check")
    return Promise.resolve({ current: "0.1.0", latest: null, notes: null, ready: false, busy: false, error: null, checked: cmd === "update_check" } as T);
  if (cmd === "parts") return Promise.resolve(mockParts(String(args?.id ?? "")) as T);
  if (cmd === "browsers_found")
    return Promise.resolve([
      { id: "chrome", name: "Google Chrome", family: "chromium" },
      { id: "edge", name: "Microsoft Edge", family: "chromium" },
      { id: "firefox", name: "Firefox", family: "firefox" },
    ] as T);
  if (cmd === "add_to_browser")
    return Promise.resolve(
      (args?.id === "firefox"
        ? { folder: "", copied: false, opened: "needsSigning" }
        : { folder: "C:\\Pako\\data\\extension\\chromium", copied: true, opened: "extensions" }) as T,
    );
  if (cmd === "send_state") return Promise.resolve(structuredClone(mockSend) as T);
  if (cmd === "direct_state") return Promise.resolve(mockDirect() as T);
  if (cmd === "direct_send") return Promise.reject("Preview mode: nothing is sent");
  if (cmd.startsWith("direct_")) return Promise.resolve(undefined as T);
  if (cmd === "send_start") return Promise.reject("Preview mode: nothing is uploaded");
  if (cmd === "sent_check") return Promise.resolve(true as T);
  // Design preview: the Uploaded list's changes stay until the page reloads.
  if (cmd === "sent_set_expiry" || cmd === "sent_set_note" || cmd === "sent_remove") {
    const i = mockSend.sent.findIndex((s) => s.id === args?.id);
    if (i >= 0 && cmd === "sent_set_expiry") Object.assign(mockSend.sent[i], { deleteAt: args?.at ?? null, deleteError: null });
    if (i >= 0 && cmd === "sent_set_note") Object.assign(mockSend.sent[i], { note: String(args?.note ?? "").trim() || null });
    if (i >= 0 && cmd === "sent_remove") mockSend.sent.splice(i, 1);
    return Promise.resolve(undefined as T);
  }
  if (cmd === "sent_qr") return Promise.resolve(mockQr(String(args?.id ?? "")) as T);
  // Design preview: the bell's lines (changes stay until the page reloads).
  if (cmd === "notices_list") return Promise.resolve(structuredClone(mockNotices) as T);
  if (cmd === "notices_read") {
    const ids = args?.ids as string[] | null;
    for (const n of mockNotices) if (!ids || ids.includes(n.id)) n.read = true;
    return Promise.resolve(undefined as T);
  }
  if (cmd === "notices_clear") {
    mockNotices.splice(0);
    return Promise.resolve(undefined as T);
  }
  if (cmd === "popup_take") return Promise.resolve([] as T);
  // Design preview: Help › Report a problem (nothing is saved or opened).
  if (cmd === "report_info") return Promise.resolve(mockReport() as T);
  if (cmd === "report_save") return Promise.reject("Preview mode: nothing is saved");
  // Design preview: ?send=offline / busy / refused shows that answer; else a reference.
  if (cmd === "report_send") {
    const k = new URLSearchParams(location.search).get("send");
    const fail: Record<string, SendFail> = { offline: { kind: "offline" }, server: { kind: "server" }, busy: { kind: "busy", retryAfter: 1500 }, refused: { kind: "refused", message: "The report is too long. Shorten what happened, or turn off Include recent problems." } };
    return new Promise((ok, no) => setTimeout(() => (k && fail[k] ? no(fail[k]) : ok("42" as T)), 700));
  }
  // Design preview: ?drive=off (not set up), ?drive=out (not connected); else connected.
  if (cmd === "send_drive_status") {
    const d = new URLSearchParams(location.search).get("drive");
    if (d === "off") return Promise.resolve({ configured: false, missingNote: "Uploading to Google Drive isn't set up on this PC yet.", signedIn: false, email: null, limit: null, used: null, error: null } as T);
    if (d === "out") return Promise.resolve({ configured: true, missingNote: null, signedIn: false, email: null, limit: null, used: null, error: null } as T);
    if (showcase) return Promise.resolve(showcaseDrive as T);
    return Promise.resolve({ configured: true, missingNote: null, signedIn: true, email: "me@gmail.com", limit: 15e9, used: 6.1e9, error: null } as T);
  }
  if (cmd === "send_measure") return Promise.resolve(4.2e9 as T);
  // Design preview: the basket's sizes (folders end without an extension).
  if (cmd === "send_sizes") {
    const paths = (args?.paths as string[]) ?? [];
    return Promise.resolve(
      paths.map((path) => {
        const dir = !/\.[a-z0-9]{2,4}$/i.test(path);
        return { path, dir, size: dir ? 3_400_000_000 : path.endsWith(".zip") ? 840_000_000 : 2_100_000, files: dir ? 214 : 1, error: null };
      }) as T,
    );
  }
  // Design preview of the Download with Pako window: ?video shows a video, ?n=3 several waiting.
  if (cmd === "ask_current") {
    const q = new URLSearchParams(location.search);
    const video = q.has("video");
    return Promise.resolve({
      id: "preview",
      stem: video ? "Lofi rain cafe - 3 hours of calm music" : "Project files (final)",
      ext: video ? "mp4" : "zip",
      site: video ? "youtube.com" : "drive.usercontent.google.com",
      size: video ? 1_420_000_000 : 734_003_200,
      dir: video ? "D:\\Downloads\\Videos" : showcase ? "C:\\Users\\Sam\\Downloads" : "C:\\Users\\me\\Downloads",
      video,
      quality: 1080,
      sizes: [[1080, 1_420_000_000], [720, 690_000_000], [480, 310_000_000], [0, 182_000_000]],
      position: 1,
      total: Number(q.get("n") ?? 1),
      running: q.has("running") ? 2 : 0,
      notify: true,
    } as T);
  }
  return Promise.resolve(undefined as T);
}

export function listen<T>(event: string, cb: EventCallback<T>): Promise<UnlistenFn> {
  if (inApp) return tauriListen<T>(event, cb);
  if (event === "progress") {
    setTimeout(() => cb({ event, id: 0, payload: mockProgress as T }), 300);
  }
  // Showcase: the file on its way in Direct moves on, so its live speed shows.
  if (showcase && event === "direct-changed") {
    const t = setInterval(() => cb({ event, id: 0, payload: undefined as T }), 1000);
    return Promise.resolve(() => clearInterval(t));
  }
  return Promise.resolve(() => {});
}

export type Status =
  | "queued"
  | "starting"
  | "downloading"
  | "retrying"
  | "paused"
  | "verifying"
  | "completed"
  | "failed"
  | "quotaExceeded"
  | "needsAccount";

export type Source =
  | { kind: "http"; url: string }
  | { kind: "gDrive"; fileId: string; resourceKey?: string; docType?: string }
  | {
      kind: "video";
      url: string;
      /** Height in lines; null = the default from Settings; 0 = audio only. */
      quality?: number | null;
      /** Audio only: "m4a" or "mp3". */
      audio?: string | null;
      /** Qualities the video has, best first. */
      options?: VideoOption[];
      audioSize?: number | null;
      duration?: number | null;
    };

/** One quality a video comes in. */
export interface VideoOption {
  height: number;
  size: number | null;
}

export interface Download {
  id: string;
  link: string;
  source: Source;
  name: string;
  destDir: string;
  size: number | null;
  md5: string | null;
  status: Status;
  message: string | null;
  resumable: boolean;
  downloaded: number;
  addedAt: number;
  /** When it was added to the downloads (ms). */
  date: number;
  finishedAt: number | null;
  group: string | null;
  resolved: boolean;
  priority: Priority;
  /** Notify when this one finishes. */
  notify: boolean;
  /** Part of a split archive: extract the set when every part is done. */
  extract: boolean;
  /** From the browser, waiting for the Download with Pako window's answer. */
  ask?: boolean;
}

/** Queue order: First, then Normal, then Later. */
export type Priority = "first" | "normal" | "later";

export interface Settings {
  downloadDir: string;
  autoConnections: boolean;
  connections: number;
  maxActive: number;
  speedLimitKb: number;
  verifyChecksum: boolean;
  /** A pop-up when any download finishes: "Download finished › Pop-up" in Settings › Notifications. */
  notifyDone: boolean;
  /** Settings › Notifications: each kind's Pop-up and Bell, where changed from the defaults. */
  notices: Record<string, NoticePref>;
  /** Do not disturb: no pop-ups (the bell keeps collecting). */
  quiet: boolean;
  /** No pop-ups while Windows says not now (full screen, presenting, Windows' Do not disturb). */
  followWindowsQuiet: boolean;
  /** Split archives: extract when all parts are done (the default for new sets). */
  extractDefault: boolean;
  /** Minimize hides Pako in the taskbar tray. */
  minimizeToTray: boolean;
  /** Closing the window hides Pako in the tray instead of quitting. */
  closeToTray: boolean;
  /** The first-start question about the tray was answered. */
  trayAsked: boolean;
  /** Video preview height: 360, 480 or 720. */
  previewQuality: number;
  /** Windows doesn't sleep while downloads run. */
  keepAwake: boolean;
  /** Quitting while downloads run asks first. */
  confirmQuit: boolean;
  /** Pako starts when you sign in to Windows... */
  startWithWindows: boolean;
  /** ...straight to the tray. */
  startInTray: boolean;
  /** The browser extension can reach Pako (registered with the browsers). */
  browserConnect: boolean;
  /** Send (older setting, only carried over): "off", "auto" or "max". */
  sendCompression: "off" | "auto" | "max";
  /** Send, several files: as they are (default), one ZIP, or a compressed ZIP. null = not chosen yet. */
  sendMode: SendMode | null;
  /** Where Send uploads to: Gofile (default, no account) or your Google Drive. */
  sendHost: SendHost;
  /** "Delete after" in the New upload basket (days; 0 = keep). The last choice is remembered. */
  uploadDeleteDays: number;
  /** The first-start setup was done (or skipped). */
  setupDone: boolean;
  /** Video downloading was offered once. */
  videoAsked: boolean;
  /** Pako and its tools update themselves (checked once a day at start). */
  autoUpdate: boolean;
  /** Only download between scheduleStart and scheduleEnd (minutes after midnight). */
  scheduleOn: boolean;
  scheduleStart: number;
  scheduleEnd: number;
  /** Links copied anywhere go to Links by themselves. */
  catchClipboard: boolean;
  /** Video quality picked by default (height in lines; 0 = audio only). */
  videoQuality: number;
  /** Downloads from the browser open the "Download with Pako" window first. */
  askBrowser: boolean;
  /** That window's "Notify me when it's done" (remembered). */
  askNotify: boolean;
  /** The small window opens by itself when a browser download starts. */
  miniOnBrowser: boolean;
  /** The small window stays open and on top. */
  miniPinned: boolean;
  /** Where the pinned small window was moved to; null = next to the tray. */
  miniPos: [number, number] | null;
  /** The last Pako version whose "What's new" was shown (null = never). */
  whatsNewSeen: string | null;
  /** "Take a quick tour?" was offered once. */
  tourOffered: boolean;
  /** Page tutorials gone through once. */
  toursDone: string[];
  /** Light or dark (changed only with `api.themeSet`). */
  theme?: ThemeChoice;
}

/** Settings › Window › Theme: "system" follows Windows' app mode. */
export type ThemeChoice = "dark" | "light" | "system";

/** "When all downloads finish". */
export type WhenDone = "nothing" | "sleep" | "shutDown";

export interface Progress {
  id: string;
  status: Status;
  message: string | null;
  downloaded: number;
  size: number | null;
  speed: number;
  connections: number;
  name: string;
}

/** A link in the Found tab, before it is picked and downloaded. */
/** "opening": a link page (ad-gate shortener) Pako is working through to reach the file. */
export type FoundState = "checking" | "opening" | "ready" | "broken" | "needsAccount" | "needsTools";

export interface FoundItem {
  id: string;
  /** The download it becomes (name, size, source). */
  template: Download;
  state: FoundState;
  message: string | null;
  /** The Drive folder it came from, and the subfolder inside it. */
  folder: string | null;
  relDir: string | null;
  /** A Drive folder Google won't list for Pako (opened in the browser instead). */
  isFolder: boolean;
  /** Its own save folder; null = the tab's Save to. */
  dest: string | null;
  /** Split archive: extract after the last part; null = the default from Settings. */
  extract: boolean | null;
}

/** Ready, or still being checked: can be downloaded. */
export function canDownload(f: FoundItem) {
  return !f.isFolder && (f.state === "ready" || f.state === "checking");
}

/** A split archive (name.part1.rar, name.7z.001…), worked out by the engine. */
export interface SetView {
  key: string;
  base: string;
  ids: string[];
  /** Plain words: "part 3 missing". Empty when complete as far as Pako can tell. */
  missing: string[];
}

export interface Snapshot {
  downloads: Download[];
  found: FoundItem[];
  foundSets: SetView[];
  downloadSets: SetView[];
  settings: Settings;
  dataDir: string;
  /** 7-Zip is installed (needed to extract split archives). */
  sevenZip: boolean;
  /** ffmpeg is installed (MKV/AVI previews). */
  ffmpeg: boolean;
  /** Pako's own version. */
  version: string;
  /** Installed with Pako Setup (false = the portable Pako.exe). */
  installed: boolean;
}

export type ComponentKind = "sevenZip" | "ffmpeg" | "ytDlp" | "deno";

/** What first-start setup saves (left out = not asked this time). */
export interface SetupChoices {
  autoUpdate?: boolean;
  minimizeToTray?: boolean;
  closeToTray?: boolean;
  startWithWindows?: boolean;
  startInTray?: boolean;
  /** "Add Pako to my browsers": the extension's line to Pako. */
  browserConnect?: boolean;
}

/** A browser the Pako extension can be added to. */
export interface Browser {
  id: "chrome" | "edge" | "brave" | "vivaldi" | "firefox";
  name: string;
  /** "chromium": Load unpacked. "firefox": the signed .xpi. */
  family: "chromium" | "firefox";
}

/** What "Add to <browser>" did. */
export interface AddedToBrowser {
  /** Chromium: the extension's folder (on the clipboard when `copied`). */
  folder: string;
  copied: boolean;
  /** extensions = the browser's extensions page opened; folder = the folder opened instead;
   *  firefox = Firefox offers to add Pako; needsSigning = no signed Firefox file yet. */
  opened: "extensions" | "folder" | "firefox" | "needsSigning";
}

/** What a pako:// link did (the extension's "Open Pako" button). */
export interface PakoLinkResult {
  action: "connect";
  ok: boolean;
  error?: string;
}

export interface ComponentStatus {
  kind: ComponentKind;
  installed: string | null;
  latest: string | null;
  /** A newer version than the installed one (compared the way Pako's updates compare). */
  update: boolean;
  /** Being installed right now (also from an earlier window: the engine keeps track). */
  installing: boolean;
  /** The tool's own name ("7-Zip"). */
  name: string;
  /** Where it comes from ("github.com/yt-dlp/yt-dlp"). */
  source: string;
  /** Space it takes when installed. */
  size: number | null;
}

/** Pako's own update. */
export interface UpdateView {
  current: string;
  latest: string | null;
  notes: string | null;
  /** Downloaded and checked: restart to use it. */
  ready: boolean;
  busy: boolean;
  error: string | null;
  checked: boolean;
}

/** One day of Dashboard history. */
export interface StatsDay {
  pako: number;
  down: number;
  up: number;
  files: number;
  failed: number;
  max: number;
  busySecs: number;
}

/** [unix seconds, Pako bytes/s, whole PC down bytes/s, whole PC up bytes/s] */
export type StatsPoint = [number, number, number, number];

export interface StatsView {
  recent: StatsPoint[];
  minutes: StatsPoint[];
  days: [string, StatsDay][];
  hasPc: boolean;
}

// ---------- Dashboard: your Pako in numbers ----------

export type NumbersPeriod = "today" | "week" | "month" | "all";

/** One kind of transfer over a period. Speeds are bytes per second. */
export interface NumbersTotals {
  bytes: number;
  files: number;
  /** Downloads, links or sends. */
  count: number;
  failed: number;
  /** Average speed while moving (null = not known). */
  avg: number | null;
  peak: number;
  /** Different people (Direct). */
  people: number;
  /** Bytes that went straight between the two PCs (not through a relay). */
  directBytes: number;
}

/** One bar of the charts: an hour of today, a day or a month. */
export interface NumbersBucket {
  key: string;
  download: number;
  upload: number;
  sent: number;
  received: number;
  downloadAvg: number | null;
  downloadPeak: number;
  uploadAvg: number | null;
  uploadPeak: number;
  directAvg: number | null;
  directPeak: number;
}

/** Uploads: "gofile", "drive", "direct". Downloads: "drive", "video", "web", "direct", "earlier". */
export interface NumbersRoute {
  route: string;
  bytes: number;
  count: number;
  avg: number | null;
  peak: number;
}

export interface NumbersPerson {
  contact: string;
  sent: number;
  received: number;
  count: number;
  peakSent: number;
  peakReceived: number;
  avgSent: number | null;
  avgReceived: number | null;
  last: number;
}

export interface NumbersRecord {
  name: string;
  value: number;
  day: string;
}

export interface NumbersView {
  period: NumbersPeriod;
  from: string;
  to: string;
  /** The first day anything was counted. */
  since: string | null;
  step: "hour" | "day" | "month" | "";
  download: NumbersTotals;
  upload: NumbersTotals;
  sent: NumbersTotals;
  received: NumbersTotals;
  buckets: NumbersBucket[];
  uploadRoutes: NumbersRoute[];
  downloadSources: NumbersRoute[];
  people: NumbersPerson[];
  /** The sites downloaded from most (the last 12 months at most). */
  sites: { site: string; bytes: number; count: number }[];
  records: { fastest: NumbersRecord | null; biggest: NumbersRecord | null; busiest: NumbersRecord | null };
}

/** Everything with one Direct contact, all time. */
export interface ContactNumbers {
  sent: number;
  received: number;
  sentCount: number;
  receivedCount: number;
  peakSent: number;
  peakReceived: number;
  avgSent: number | null;
  avgReceived: number | null;
  directBytes: number;
  relayBytes: number;
  last: number | null;
}

export interface AddResult {
  added: number;
  errors: string[];
  /** Links already in the list (not added twice). */
  skipped?: number;
}

export interface Part {
  start: number;
  end: number;
  done: number;
  active: boolean;
}

/** How several files are sent. */
export type SendMode = "files" | "zip" | "compress";
export type SendHost = "gofile" | "drive";
/** A picked file or folder in the Send basket, measured: folders count all their files. */
export interface PathSize {
  path: string;
  dir: boolean;
  size: number;
  files: number;
  /** It couldn't be read (moved, no access...). */
  error: string | null;
}

/** A send in progress (or one that failed). */
export interface SendJob {
  id: string;
  name: string;
  files: number;
  /** Bytes of the files themselves. */
  size: number;
  /** Exact upload size when known. */
  total: number | null;
  sent: number;
  speed: number;
  status: "uploading" | "failed";
  message: string | null;
  paths: string[];
  /** Several files sent as they are, into one folder. */
  asFiles: boolean;
  filesDone: number;
  /** The folder's link, once the first file is up. */
  link: string | null;
  /** "Try again" sends only the files that didn't upload. */
  canResume: boolean;
  /** How several files were sent: "Try again" sends them the same way. */
  mode?: SendMode;
  /** Where it goes. */
  host?: SendHost;
  /** Its "Delete after" (days; 0 = keep): "Try again" keeps it. */
  deleteDays?: number;
}

/** Something sent before, with its link. */
export interface SentItem {
  id: string;
  name: string;
  size: number;
  files: number;
  date: number;
  link: string;
  folderId: string | null;
  fileId: string | null;
  compressed: boolean;
  /** Several files sent as they are (a Gofile folder), not one ZIP. */
  asFiles?: boolean;
  /** "drive" = sent with your Google Drive; none = Gofile. */
  host?: "drive" | null;
  /** Not all files are up (cancelled, failed or Pako closed): the ones that are stay listed. */
  incomplete?: boolean;
  /** Pako has the Gofile key for it, so it can be deleted from Gofile. */
  canDelete?: boolean;
  /** Times downloaded, as Gofile said at the last check (missing = not known). */
  downloads?: number | null;
  /** When Pako saw that count go up (ms). */
  downloadedAt?: number | null;
  /** Gofile says it's no longer there (deleted or expired). */
  gone?: boolean;
  /** Close to Gofile's keep limit (10 days without a download). */
  soon?: boolean;
  /** When it gets deleted ("Delete after", ms); missing = kept. */
  deleteAt?: number | null;
  /** The service deletes it itself on that day (Gofile folders), even with the PC off. */
  expiryOnHost?: boolean;
  /** Why Pako couldn't delete it yet (tried again every hour). */
  deleteError?: string | null;
  deleteTries?: number;
  /** Your note. */
  note?: string | null;
  /** The files inside (names, sizes), when known. */
  contents?: { name: string; size: number }[];
  /** Seconds spent uploading, and the top speed (bytes/s). */
  secs?: number | null;
  peak?: number | null;
  /** Download counts seen over time: [when (ms), count]. */
  countHistory?: [number, number][];
  /** The Google account a Drive upload went into. */
  account?: string | null;
}

export interface SendState {
  jobs: SendJob[];
  sent: SentItem[];
  canDelete: boolean;
  keepNote: string;
  /** Download counts are being asked for right now. */
  checking?: boolean;
  /** When they were last asked for (ms). */
  checkedAt?: number | null;
  /** Set when Gofile didn't share the counts at the last check. */
  countsNote?: string | null;
  /** What Pako's sends take in your Google Drive (items still listed), and how many. */
  driveUsed?: number;
  driveSends?: number;
  /** Something in the list of uploads couldn't be read. */
  listNote?: string | null;
  /** The Google accounts the Drive uploads on the list went into. */
  driveAccounts?: string[];
}

/** Your Google Drive for uploads. */
export interface DriveSendStatus {
  /** This PC has the "Pako Send" registration (else connecting can't work). */
  configured: boolean;
  missingNote: string | null;
  signedIn: boolean;
  email: string | null;
  /** null = no limit, or not known. */
  limit: number | null;
  used: number | null;
  error: string | null;
}

// ---------- Direct (Pako to Pako) ----------

export interface DirectContact {
  /** Their Pako ID. */
  id: string;
  name: string;
  /** The first connection went through (both said yes). */
  trusted: boolean;
  /** "Can send to me without asking". */
  autoAccept: boolean;
  saveFolder: string | null;
  /** ms */
  lastSeen: number | null;
  added: number;
  online: boolean;
  /** "direct6", "direct" or "relay" while connected. */
  path: string | null;
  /** Waiting for them to accept the first connection. */
  waiting: boolean;
  /** The safety code to compare once. */
  code: string;
  note: string | null;
  /** Where their files go. */
  folder: string;
  /** Your tags for them (only on this PC). */
  tags: string[];
  favourite: boolean;
  /** The name they chose for themselves (your own name for them is `name`). */
  theirName: string | null;
  /** They declined connecting (or removed you): Pako only asks again when you press Connect. */
  declined?: boolean;
}

export interface DirectRequest {
  peer: string;
  name: string;
  code: string;
  /** One of your contacts already has this name (this is a different Pako). */
  knownAs?: string | null;
}

export interface DirectOffer {
  id: string;
  contact: string;
  contactName: string;
  name: string;
  files: number;
  size: number;
  sample: string[];
  folder: string;
  /** Why you're asked although this contact may send without asking (big for the drive). */
  askedBecause?: string | null;
}

export interface DirectTransfer {
  id: string;
  contact: string;
  outgoing: boolean;
  name: string;
  files: number;
  size: number;
  done: number;
  /** offered, preparing, sending, receiving, checking */
  state: string;
  path: string | null;
  message: string | null;
}

export interface DirectHistory {
  id: string;
  contact: string;
  outgoing: boolean;
  name: string;
  files: number;
  size: number;
  /** ms */
  date: number;
  path: string | null;
  /** done, declined, cancelled, failed */
  result: string;
  message: string | null;
  savedTo: string | null;
  /** How long the bytes moved (ms), average and best speed (bytes/s). For a future Dashboard. */
  durationMs?: number;
  avgSpeed?: number;
  peakSpeed?: number;
}

export interface DirectState {
  ready: boolean;
  error: string | null;
  myId?: string;
  myName?: string;
  /** Your tags, in your order. */
  tags?: string[];
  contacts?: DirectContact[];
  requests?: DirectRequest[];
  offers?: DirectOffer[];
  transfers?: DirectTransfer[];
  history?: DirectHistory[];
}

/** Design preview: two contacts (one online) and a short history. */
function mockDirect(): DirectState {
  if (showcase) return showcaseDirect();
  const now = Date.now();
  const rado = "a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e";
  const mia = "f0e1d2c3b4a5968778695a4b3c2d1e0f1f2e3d4c5b6a79881726354a3b2c1d0e";
  const hery = "5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f";
  // Preview only: "?nocontacts" in the address shows the first-time state, "?manycontacts" a long list.
  const empty = typeof location !== "undefined" && location.search.includes("nocontacts");
  const many = typeof location !== "undefined" && location.search.includes("manycontacts");
  const extra: DirectContact[] = many
    ? ["Aina", "Bako", "Chloé", "Dina", "Eric", "Fara", "Gaby", "Hery", "Ivo", "Jessy", "Kanto", "Lova", "Mamy", "Nina", "Onja", "Patrick", "Rija", "Sandy", "Tiana", "Ulrich", "Voahangy", "Willy", "Yvan"].map((name, i) => ({
        id: `${(i + 10).toString(16).padStart(2, "0")}`.repeat(32),
        name,
        trusted: true,
        autoAccept: false,
        saveFolder: null,
        lastSeen: now - 3600000 * (i + 1),
        added: now - 86400000 * (i + 2),
        online: i % 5 === 0,
        path: i % 5 === 0 ? (i % 2 ? "direct" : "relay") : null,
        waiting: i === 7,
        code: "0000 0000 0000",
        note: null,
        folder: `C:\\Users\\me\\Downloads\\From ${name}`,
        tags: [["Friends"], ["Work"], ["Family"], ["Work", "Clients"], []][i % 5],
        favourite: i % 7 === 2,
        theirName: null,
      }))
    : [];
  if (empty) {
    return {
      ready: true,
      error: null,
      myId: "9c8b7a6f5e4d3c2b1a0f9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a2f1e0d9c8b",
      myName: "",
      tags: [],
      contacts: [],
      requests: [],
      offers: [],
      transfers: [],
      history: [],
    };
  }
  return {
    ready: true,
    error: null,
    myId: "9c8b7a6f5e4d3c2b1a0f9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a2f1e0d9c8b",
    myName: "Sam",
    tags: ["Friends", "Family", "Work", "Clients"],
    contacts: [
      { id: rado, name: "Rado", trusted: true, autoAccept: true, saveFolder: null, lastSeen: now, added: now - 86400000 * 3, online: true, path: "direct6", waiting: false, code: "4821 0937 5512 0386 7741", note: null, folder: "C:\\Users\\me\\Downloads\\From Rado", tags: ["Friends", "Work"], favourite: true, theirName: "Rado R." },
      { id: mia, name: "Mia", trusted: true, autoAccept: false, saveFolder: null, lastSeen: now - 3600000 * 20, added: now - 86400000 * 9, online: false, path: null, waiting: false, code: "1170 6640 2093", note: null, folder: "C:\\Users\\me\\Downloads\\From Mia", tags: ["Family"], favourite: false, theirName: null },
      { id: hery, name: "Hery", trusted: true, autoAccept: false, saveFolder: "D:\\Projects\\From Hery", lastSeen: now - 3600000 * 3, added: now - 86400000 * 40, online: true, path: "relay", waiting: false, code: "3301 8842 1176", note: null, folder: "D:\\Projects\\From Hery", tags: ["Work", "Clients"], favourite: false, theirName: "Hery A." },
      ...extra,
    ],
    requests: [],
    // "?offer" in the address: Mia's files wait for an answer (the bell's first line).
    offers:
      typeof location !== "undefined" && location.search.includes("offer")
        ? [{ id: "o-mia", contact: mia, contactName: "Mia", name: "Holiday photos", files: 48, size: 1.3e9, sample: ["IMG_0412.jpg", "IMG_0413.jpg", "IMG_0415.jpg"], folder: "C:\\Users\\me\\Downloads\\From Mia" }]
        : [],
    transfers: [{ id: "t3", contact: rado, outgoing: true, name: "Render final v3.mp4", files: 1, size: 2.4e9, done: 0.9e9, state: "sending", path: "direct6", message: null }],
    history: [
      { id: "t1", contact: rado, outgoing: true, name: "Wedding photos", files: 214, size: 3.2e9, date: now - 86400000, path: "direct6", result: "done", message: null, savedTo: null },
      { id: "t2", contact: rado, outgoing: false, name: "Project.zip", files: 1, size: 1.2e9, date: now - 7200000, path: "relay", result: "done", message: null, savedTo: "C:\\Users\\me\\Downloads\\From Rado\\Project.zip" },
      { id: "t4", contact: mia, outgoing: true, name: "Contract final.pdf", files: 1, size: 2.1e6, date: now - 86400000 * 2, path: "direct", result: "declined", message: "They declined it.", savedTo: null },
    ],
  };
}

export const api = {
  snapshot: () => invoke<Snapshot>("snapshot"),
  foundAdd: (text: string) => invoke<AddResult>("found_add", { text }),
  foundDownload: (ids: string[], first: boolean, dest: string | null, notify = false) =>
    invoke<number>("found_download", { ids, first, dest, notify }),
  foundRemove: (ids: string[]) => invoke<void>("found_remove", { ids }),
  /** Brings a waiting link check to the front. */
  gateFocus: (id: string) => invoke<void>("gate_focus", { id }),
  foundRestore: (items: FoundItem[], at: number) => invoke<void>("found_restore", { items, at }),
  foundSetVideo: (ids: string[], quality: number | null, audio: string | null) => invoke<void>("found_set_video", { ids, quality, audio }),
  foundRecheck: (ids: string[]) => invoke<void>("found_recheck", { ids }),
  restoreDownloads: (list: Download[]) => invoke<void>("restore_downloads", { list }),
  moveBefore: (ids: string[], target: string | null) => invoke<void>("move_before", { ids, target }),
  openSettings: () => invoke<void>("open_settings"),
  askCurrent: () => invoke<AskView | null>("ask_current"),
  askAnswer: (answer: AskAnswer) => invoke<void>("ask_answer", { answer }),
  miniPin: (on: boolean) => invoke<void>("mini_pin", { on }),
  miniHide: () => invoke<void>("mini_hide"),
  /** Help: What's new was shown for this version, and/or the quick tour was offered. */
  helpSeen: (seen: { whatsNew?: string; tourOffered?: boolean; tourDone?: string }) =>
    invoke<void>("help_seen", { whatsNew: seen.whatsNew ?? null, tourOffered: seen.tourOffered ?? null, tourDone: seen.tourDone ?? null }),
  /** Paths a finished row drags out (files, or the folder of a finished folder row). */
  dragPaths: (ids: string[], folder: boolean) => invoke<string[]>("drag_paths", { ids, folder }),
  sendState: () => invoke<SendState>("send_state"),
  sendDriveStatus: () => invoke<DriveSendStatus>("send_drive_status"),
  sendDriveSignIn: () => invoke<string>("send_drive_sign_in"),
  sendDriveSignOut: () => invoke<void>("send_drive_sign_out"),
  /** How much a send of these files would upload. */
  sendMeasure: (paths: string[], mode: SendMode | null = null) => invoke<number>("send_measure", { paths, mode }),
  /** `name` = the folder or ZIP name you gave the send (null = Pako's usual name). */
  /** `deleteDays`: "Delete after" for this upload (0 = keep; null = the setting). */
  /** `host`: where this upload goes ("Try again" keeps it; null = the setting). */
  sendStart: (
    paths: string[],
    mode: SendMode | null = null,
    name: string | null = null,
    deleteDays: number | null = null,
    host: SendHost | null = null,
  ) => invoke<string>("send_start", { paths, mode, name, deleteDays, host }),
  /** Sizes of the files and folders in the Send basket. */
  sendSizes: (paths: string[]) => invoke<PathSize[]>("send_sizes", { paths }),
  sendResume: (id: string) => invoke<boolean>("send_resume", { id }),
  sendCancel: (id: string) => invoke<void>("send_cancel", { id }),
  sendDismiss: (id: string) => invoke<void>("send_dismiss", { id }),
  sentRemove: (id: string, online: boolean) => invoke<void>("sent_remove", { id, online }),
  /** When an upload gets deleted (ms); null = keep it. */
  sentSetExpiry: (id: string, at: number | null) => invoke<void>("sent_set_expiry", { id, at }),
  sentSetNote: (id: string, note: string) => invoke<void>("sent_set_note", { id, note }),
  /** The link as a QR code (SVG), made on this PC. */
  sentQr: (id: string) => invoke<string>("sent_qr", { id }),
  /** Asks Gofile how often each sent item was downloaded (in the background). False = not now. */
  sentCheck: (button: boolean) => invoke<boolean>("sent_check", { button }),
  copyText: (text: string) => invoke<void>("copy_text", { text }),
  sentOpen: (id: string) => invoke<void>("sent_open", { id }),
  quitNow: () => invoke<void>("quit_now"),
  setWhenDone: (action: WhenDone) => invoke<void>("set_when_done", { action }),
  cancelWhenDone: () => invoke<void>("cancel_when_done"),
  foundExisting: (ids: string[], dest: string | null) => invoke<string[]>("found_existing", { ids, dest }),
  foundSet: (ids: string[], o: { dest?: string | null; priority?: Priority; notify?: boolean; extract?: boolean }) =>
    invoke<void>("found_set", {
      ids,
      setDest: "dest" in o,
      dest: o.dest ?? null,
      priority: o.priority ?? null,
      notify: o.notify ?? null,
      extract: o.extract ?? null,
    }),
  setExtract: (ids: string[], on: boolean) => invoke<void>("set_extract", { ids, on }),
  hideToTray: () => invoke<void>("hide_to_tray"),
  stats: () => invoke<StatsView>("stats"),
  numbersView: (period: NumbersPeriod) => invoke<NumbersView>("numbers_view", { period }),
  numbersContact: (id: string) => invoke<ContactNumbers>("numbers_contact", { id }),
  /** Saves the detailed list as a CSV file; returns how many rows. */
  numbersExport: (path: string) => invoke<number>("numbers_export", { path }),
  numbersReset: () => invoke<void>("numbers_reset"),
  previewUrl: (id: string, quality: number) =>
    invoke<{ url: string; converted: boolean; quality: number }>("preview_url", { id, quality }),
  componentsStatus: () => invoke<ComponentStatus[]>("components_status"),
  componentsCheck: () => invoke<ComponentStatus[]>("components_check"),
  componentInstall: (kind: ComponentKind) => invoke<string>("component_install", { kind }),
  componentRemove: (kind: ComponentKind) => invoke<void>("component_remove", { kind }),
  /** Removes a feature's tools together (all or none). */
  componentsRemove: (kinds: ComponentKind[]) => invoke<void>("components_remove", { kinds }),
  updateStatus: () => invoke<UpdateView>("update_status"),
  updateCheck: (get: boolean) => invoke<UpdateView>("update_check", { get }),
  updateRestart: () => invoke<void>("update_restart"),
  showMain: () => invoke<void>("show_main"),
  /** The bell: its lines (newest first), mark read (all with no ids), clear. */
  noticesList: () => invoke<Notice[]>("notices_list"),
  noticesRead: (ids: string[] | null) => invoke<void>("notices_read", { ids }),
  noticesClear: () => invoke<void>("notices_clear"),
  /** The pop-up window: take the waiting cards, say how tall the cards are (0 = hide), whether
   *  it's no time for pop-ups now (true = hidden, cards kept), open Pako on a card's page. */
  popupTake: () => invoke<PopupCard[]>("popup_take"),
  popupFit: (height: number) => invoke<void>("popup_fit", { height }),
  popupQuiet: () => invoke<boolean>("popup_quiet"),
  popupOpen: (go: Go, id: string | null) => invoke<void>("popup_open", { go, id }),
  pauseAll: () => invoke<void>("pause_all"),
  resumeAll: () => invoke<void>("resume_all"),
  setNotify: (ids: string[], on: boolean) => invoke<void>("set_notify", { ids, on }),
  setFolder: (ids: string[], dir: string) => invoke<void>("set_folder", { ids, dir }),
  addLinks: (text: string, dest: string | null) => invoke<AddResult>("add_links", { text, dest }),
  pause: (ids: string[]) => invoke<void>("pause", { ids }),
  resume: (ids: string[]) => invoke<void>("resume", { ids }),
  downloadFirst: (ids: string[]) => invoke<void>("download_first", { ids }),
  setPriority: (ids: string[], priority: Priority) => invoke<void>("set_priority", { ids, priority }),
  startOver: (ids: string[]) => invoke<void>("start_over", { ids }),
  remove: (ids: string[], deleteFiles: boolean) => invoke<void>("remove", { ids, deleteFiles }),
  rename: (id: string, name: string) => invoke<string>("rename", { id, name }),
  moveInQueue: (ids: string[], top: boolean) => invoke<void>("move_in_queue", { ids, top }),
  parts: (id: string) => invoke<Part[]>("parts", { id }),
  clearCompleted: () => invoke<void>("clear_completed"),
  /** The Drive file's (or not-public folder's) own page in the browser: a download or a Links item. */
  openInDrive: (id: string) => invoke<void>("open_in_drive", { id }),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  /** Light or dark: the saved choice (each window shows it, see theme.svelte.ts). */
  themeGet: () => invoke<ThemeChoice>("theme_get"),
  /** Saves the choice and tells every window ("theme-changed"). */
  themeSet: (theme: ThemeChoice) => invoke<void>("theme_set", { theme }),
  /** What shows now: the windows' own background follows (no flash when resizing). */
  themeShown: (light: boolean) => invoke<void>("theme_shown", { light }),
  /** First-start setup: saves only its answers (works with the download folder unavailable). */
  saveSetup: (choices: SetupChoices) => invoke<void>("save_setup", { choices }),
  openItem: (id: string, reveal: boolean) => invoke<void>("open_item", { id, reveal }),
  openLink: (id: string) => invoke<void>("open_link", { id }),
  openFolder: (path: string) => invoke<void>("open_folder", { path }),
  extensionFiles: () => invoke<string>("extension_files"),
  browsersFound: () => invoke<Browser[]>("browsers_found"),
  addToBrowser: (id: Browser["id"]) => invoke<AddedToBrowser>("add_to_browser", { id }),
  /** The Firefox signing steps: the README, selected in its folder. */
  extensionSteps: () => invoke<void>("extension_steps"),
  /** The last pako:// result, once. */
  takePakoLink: () => invoke<PakoLinkResult | null>("take_pako_link"),
  // Direct
  directState: () => invoke<DirectState>("direct_state"),
  directSetName: (name: string) => invoke<void>("direct_set_name", { name }),
  directAddContact: (id: string, name: string) => invoke<void>("direct_add_contact", { id, name }),
  directEditContact: (id: string, change: { name?: string; autoAccept?: boolean; folder?: string }) =>
    invoke<void>("direct_edit_contact", { id, name: change.name ?? null, autoAccept: change.autoAccept ?? null, folder: change.folder ?? null }),
  directRemoveContact: (id: string) => invoke<void>("direct_remove_contact", { id }),
  directSetFavourite: (id: string, on: boolean) => invoke<void>("direct_set_favourite", { id, on }),
  directSetTags: (id: string, tags: string[]) => invoke<void>("direct_set_tags", { id, tags }),
  directAddTag: (name: string) => invoke<string>("direct_add_tag", { name }),
  directRenameTag: (old: string, newName: string) => invoke<void>("direct_rename_tag", { old, new: newName }),
  directDeleteTag: (name: string) => invoke<void>("direct_delete_tag", { name }),
  directConnect: (id: string) => invoke<void>("direct_connect", { id }),
  directAnswerRequest: (peer: string, accept: boolean) => invoke<void>("direct_answer_request", { peer, accept }),
  directAnswerOffer: (id: string, accept: boolean, folder: string | null = null) => invoke<void>("direct_answer_offer", { id, accept, folder }),
  directSend: (contact: string, paths: string[], compress: boolean, name: string | null) =>
    invoke<string>("direct_send", { contact, paths, compress, name }),
  directCancel: (id: string) => invoke<void>("direct_cancel", { id }),
  directClearHistory: (contact: string | null) => invoke<void>("direct_clear_history", { contact }),
  /** True: it's a program or script, so it was shown in its folder instead of opened. */
  directOpen: (id: string, reveal: boolean) => invoke<boolean>("direct_open", { id, reveal }),
  // Help › Report a problem
  /** Pako's version, Setup or portable, Windows' version and the problems noted (oldest first). */
  reportInfo: () => invoke<ReportInfo>("report_info"),
  /** Saves the report as a text file. */
  reportSave: (path: string, text: string) => invoke<void>("report_save", { path, text }),
  /** Opens the email app on a new message (a mailto: link to support@sookera.com). */
  reportMail: (url: string) => invoke<void>("report_mail", { url }),
  /** Sends the report to Sookera's report service, as the window shows it: its reference back,
   *  or a SendFail. */
  reportSend: (description: string, theme: string, problems: SentProblem[]) => invoke<string>("report_send", { description, theme, problems }),
};

/** Where in Pako a problem happened (src-tauri/src/errorlog.rs). */
export type ProblemArea = "download" | "video" | "linkCheck" | "upload" | "direct" | "update" | "extension" | "other";

/** One problem Pako noted: the message you saw, trimmed of links' paths, IDs and your name. */
export interface Problem {
  /** ms since 1970. */
  time: number;
  area: ProblemArea;
  message: string;
  version: string;
}

/** A problem as Send report sends it: the time with the PC's offset ("2026-11-16T16:21:00+03:00"). */
export interface SentProblem {
  time: string;
  area: ProblemArea;
  message: string;
  version: string;
}

/** Why Send report didn't go (src-tauri/src/errorlog.rs SendFail). */
export type SendFail =
  | { kind: "offline" }
  | { kind: "server" }
  | { kind: "busy"; retryAfter: number }
  | { kind: "refused"; message: string };

export interface ReportInfo {
  version: string;
  /** Installed with Pako Setup (false = the portable Pako.exe). */
  installed: boolean;
  /** "Windows 11 Pro 24H2 (build 26100)". */
  windows: string;
  /** Oldest first. */
  problems: Problem[];
}

export function isActive(s: Status) {
  return s === "starting" || s === "downloading" || s === "retrying" || s === "verifying";
}

export function isProblem(s: Status) {
  return s === "failed" || s === "quotaExceeded" || s === "needsAccount";
}

/** What the Download with Pako window shows for one download from the browser. */
export interface AskView {
  id: string;
  /** The name without its extension (that part can be edited). */
  stem: string;
  /** "zip", "mp4"... (a video's follows its quality). */
  ext: string;
  site: string;
  size: number | null;
  dir: string;
  video: boolean;
  /** Videos: the picked height (0 = audio only). */
  quality: number;
  /** Videos: each offered quality and its likely size. */
  sizes: [number, number | null][];
  position: number;
  total: number;
  /** Downloads running now: then "Download now" and "Add to queue". */
  running: number;
  notify: boolean;
}

export interface AskAnswer {
  id: string;
  action: "now" | "queue" | "cancel";
  stem?: string;
  dir?: string;
  quality?: number | null;
  notify?: boolean;
  dontAsk?: boolean;
}
