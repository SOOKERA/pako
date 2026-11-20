// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Sample data used only when the page is opened in a normal browser (design preview),
// never inside the app.
import type {
  ContactNumbers,
  Download,
  FoundItem,
  MirrorView,
  NumbersBucket,
  NumbersPerson,
  NumbersRoute,
  NumbersTotals,
  NumbersView,
  ReportInfo,
  Snapshot,
  StatsDay,
  StatsPoint,
  StatsView,
} from "./api";
import type { Notice } from "./notices";
import { showcase, showcaseNotices, showcaseParts, showcaseProgress, showcaseSend, showcaseSnapshot } from "./showcase";
import { version as pakoVersion } from "../../package.json";

let seq = 0;
const base = (over: Partial<Download>): Download => ({
  id: "m" + ++seq,
  link: "https://example.com/file",
  source: { kind: "http", url: "https://example.com/file" },
  name: "file.zip",
  destDir: "C:\\Users\\me\\Downloads",
  size: 1_000_000_000,
  md5: null,
  status: "downloading",
  message: null,
  resumable: true,
  downloaded: 0,
  addedAt: 1000 + seq,
  date: Date.now() - (40 - seq) * 3_600_000 * 7,
  finishedAt: null,
  group: null,
  resolved: true,
  priority: "normal",
  notify: false,
  extract: false,
  ...over,
});

const drive = { kind: "gDrive", fileId: "x" } as const;
const GB = 1024 ** 3;
const MB = 1024 ** 2;
const chanel = "C:\\Users\\me\\Downloads\\Chanel assets";

const downloads: Download[] = [
  base({ name: "Big dataset part 1.7z", size: 44.7 * GB, downloaded: 20 * GB, source: drive, link: "https://drive.google.com/file/d/abc/view", md5: "x", alert: "important" }),
  base({ name: "Scene_02.uasset", status: "completed", size: 690 * MB, downloaded: 690 * MB, source: drive, group: "Chanel assets", destDir: chanel, md5: "x" }),
  base({ name: "Scene_03.uasset", status: "retrying", size: 782 * MB, downloaded: 296 * MB, source: drive, group: "Chanel assets", destDir: chanel, message: "No connection to the server. Trying again in 8s" }),
  base({ name: "Scene_04.uasset", status: "queued", size: 706 * MB, source: drive, group: "Chanel assets", destDir: chanel }),
  base({ name: "Project render final version for client review v3.mp4", status: "quotaExceeded", size: 16.8 * GB, downloaded: 3.7 * GB, source: drive, message: "Google's download limit for this shared file is reached for today." }),
  base({ name: "Course videos.zip", status: "needsAccount", size: 12.4 * GB, downloaded: 4.1 * GB, source: drive, message: "Google only gives this file to people it's shared with. Open it in your browser and download it there: Pako catches the download." }),
  base({ name: "Textures pack 4K.rar", status: "verifying", size: 8.2 * GB, downloaded: 8.2 * GB, link: "https://mega.nz/file/abc", message: "Checking the file" }),
  base({ name: "Brand guidelines.pdf", status: "starting", size: null, source: drive, priority: "first" }),
  base({ name: "ubuntu-24.04-desktop-amd64.iso", priority: "later", alert: "off", status: "paused", size: 5.5 * GB, downloaded: 1.1 * GB, link: "https://releases.ubuntu.com/24.04/ubuntu-24.04-desktop-amd64.iso" }),
  base({ name: "setup-tools.exe", status: "failed", size: null, message: "File not found on the server (404)." }),
  // The same file on several hosts: the first host failed, it goes on from a backup.
  base({
    name: "Holiday.zip",
    size: 2.4 * GB,
    downloaded: 0.9 * GB,
    link: "https://mirror.example.net/f/Holiday.zip",
    source: { kind: "http", url: "https://mirror.example.net/f/Holiday.zip" },
    tried: [{ link: "https://files.example.com/Holiday.zip", source: { kind: "http", url: "https://files.example.com/Holiday.zip" }, size: 2.4 * GB, md5: null, why: "File not found on the server (404)." }],
    mirrors: [{ link: "https://dl.example.org/Holiday.zip", source: { kind: "http", url: "https://dl.example.org/Holiday.zip" }, size: 2.4 * GB, md5: null }],
  }),
  base({ name: "Holiday photos 2025.zip", status: "completed", size: 2.18 * GB, downloaded: 2.18 * GB, source: drive, md5: "x", finishedAt: Date.now() }),
];

const found = (over: Partial<FoundItem> & { name: string; size?: number | null; link?: string; source?: Download["source"] }): FoundItem => {
  const template = base({ name: over.name, size: over.size ?? null, link: over.link ?? "https://example.com/" + over.name, source: over.source ?? { kind: "http", url: "https://example.com/" + over.name }, status: "queued" });
  return { id: template.id, template, state: "ready", message: null, folder: null, relDir: null, isFolder: false, dest: null, extract: null, ...over };
};

const foundItems: FoundItem[] = [
  found({ name: "Big dataset part 2.7z", size: 44.7 * GB, source: drive, link: "https://drive.google.com/file/d/b2/view" }),
  // "Download on": one link set to Home PC, one to Office PC (offline: it stayed, noted).
  found({ name: "Mountain timelapse 4K.mkv", size: 1.2 * GB, source: drive, link: "https://drive.google.com/file/d/v1/view", device: "9c1d7a4e2b6f80135c7e9a2d4f6b8c0e1a3c5e7092b4d6f8a0c2e4f6a8b0c2d4" }),
  found({ name: "Scene_05.uasset", size: 690 * MB, source: drive, folder: "Chanel assets v2", relDir: "", link: "https://drive.google.com/drive/folders/c2" }),
  found({ name: "Scene_06.uasset", size: 782 * MB, source: drive, folder: "Chanel assets v2", relDir: "", link: "https://drive.google.com/drive/folders/c2" }),
  found({ name: "notes.txt", size: 2048, source: drive, folder: "Chanel assets v2", relDir: "", link: "https://drive.google.com/drive/folders/c2" }),
  found({ name: "Google Drive folder", source: drive, state: "needsAccount", isFolder: true, message: "This folder isn't shared with \"anyone with the link\". Open it in your browser and download from there: Pako catches the download.", link: "https://drive.google.com/drive/folders/x9" }),
  found({ name: "Brand book 2026.pdf", source: drive, state: "checking", link: "https://drive.google.com/file/d/p1/view", device: "7e5c3a1f9d7b5e3c1a9f7d5b3e1c9a7f5d3b1e9c7a5f3d1b9e7c5a3f1d9b7e5c", deviceNote: "Office PC is offline: not sent. Press Download again once it's on." }),
  found({ name: "old-archive.zip", state: "broken", message: "File not found on the server (404)." }),
  found({
    name: "Big Buck Bunny 60fps 4K.mp4",
    size: 512 * MB,
    link: "https://www.youtube.com/watch?v=aqz-KE-bpKQ",
    source: { kind: "video", url: "https://www.youtube.com/watch?v=aqz-KE-bpKQ", quality: null, audio: null, duration: 635, audioSize: 10 * MB, options: [2160, 1440, 1080, 720, 480, 360].map((height, i) => ({ height, size: [2.1 * GB, 1.1 * GB, 512 * MB, 260 * MB, 120 * MB, 70 * MB][i] })) },
  }),
  found({ name: "Lo-fi mix", link: "https://youtu.be/xyz", state: "needsTools", message: "Video downloads need the video tools.", source: { kind: "video", url: "https://youtu.be/xyz" } }),
  // The same file on three hosts (one of them broken).
  found({ name: "Holiday video.mp4", size: 2.4 * GB, link: "https://files.example.com/Holiday%20video.mp4", source: { kind: "http", url: "https://files.example.com/Holiday%20video.mp4" } }),
  found({ name: "Holiday video.mp4", size: 2.4 * GB, link: "https://mirror.example.net/f/Holiday%20video.mp4", source: { kind: "http", url: "https://mirror.example.net/f/Holiday%20video.mp4" } }),
  found({ name: "Holiday video.mp4", state: "broken", message: "File not found on the server (404).", link: "https://dl.example.org/Holiday%20video.mp4", source: { kind: "http", url: "https://dl.example.org/Holiday%20video.mp4" } }),
  found({ name: "Project_files.part1.rar", size: 4 * GB }),
  found({ name: "Project_files.part2.rar", size: 4 * GB }),
  found({ name: "Project_files.part4.rar", size: 1.2 * GB }),
  // Two parts of the set on a second host.
  ...[1, 2].map((n) => found({ name: `Project_files.part${n}.rar`, size: 4 * GB, link: `https://mirror.example.net/Project_files.part${n}.rar`, source: { kind: "http", url: `https://mirror.example.net/Project_files.part${n}.rar` } })),
];

/** Groups of the same file on several hosts, as the engine would send them (first ready host chosen). */
export function mockMirrors(items: FoundItem[], names: string[]): MirrorView[] {
  return names.map((name) => {
    const rows = items.filter((f) => f.template.name === name && !f.folder);
    const chosen = rows.find((f) => f.state === "ready") ?? null;
    return {
      key: rows[0].id,
      name,
      size: rows.find((f) => f.template.size != null)?.template.size ?? null,
      ids: rows.map((f) => f.id),
      hosts: rows.map((f) => (f.template.source.kind === "gDrive" ? "drive.google.com" : new URL(f.template.link).hostname.replace(/^www\./, ""))),
      chosen: chosen?.id ?? null,
      picked: false,
    };
  });
}

/** Plausible history for the Dashboard preview (fixed, not random). */
export function mockStats(): StatsView {
  const now = Math.floor(Date.now() / 1000);
  const wave = (i: number, a: number, b: number) => Math.max(0, a + b * Math.sin(i / 7) + (b / 2) * Math.sin(i / 2.3));
  const recent: StatsPoint[] = [];
  for (let i = 0; i < 720; i++) {
    const t = now - (719 - i) * 5;
    const pako = i > 120 && i < 600 ? wave(i, 9e6, 3e6) : 0;
    recent.push([t, pako, pako + wave(i, 6e5, 4e5), wave(i, 1.5e5, 1e5)]);
  }
  const minutes: StatsPoint[] = [];
  for (let i = 0; i < 1440; i++) {
    const t = now - (1439 - i) * 60;
    const pako = (i > 300 && i < 520) || i > 1300 ? wave(i, 8e6, 3e6) : 0;
    minutes.push([t, pako, pako + wave(i, 5e5, 3e5), wave(i, 1.2e5, 8e4)]);
  }
  const days: [string, StatsDay][] = [];
  for (let i = 40; i >= 0; i--) {
    const d = new Date();
    d.setDate(d.getDate() - i);
    if (i % 6 === 3) continue; // a few days without Pako
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const pako = Math.round(wave(i * 3, 18 * GB, 12 * GB));
    days.push([key, { pako, down: pako + Math.round(wave(i, 3 * GB, 2 * GB)), up: Math.round(wave(i, 0.6 * GB, 0.4 * GB)), files: (i * 7) % 13, failed: i % 9 === 0 ? 1 : 0, max: Math.round(wave(i, 14e6, 4e6)), busySecs: 4 * 3600 }]);
  }
  return { recent, minutes, days, hasPc: true };
}

// Design preview only: ?many=3000 adds that many rows to both lists (long-list testing).
const many = typeof location !== "undefined" ? Number(new URLSearchParams(location.search).get("many") ?? 0) : 0;
for (let i = 0; i < many; i++) {
  const done = i % 3 === 0;
  downloads.push(base({ name: `Long list file ${String(i + 1).padStart(4, "0")}.zip`, status: done ? "completed" : "queued", downloaded: done ? 1_000_000_000 : 0, group: i % 40 < 4 ? `Folder ${Math.floor(i / 40)}` : null }));
  foundItems.push(found({ name: `Long list link ${String(i + 1).padStart(4, "0")}.rar`, size: (i % 50) * MB + MB, folder: i % 30 < 3 ? `Pack ${Math.floor(i / 30)}` : undefined, relDir: "", link: i % 30 < 3 ? `https://drive.google.com/drive/folders/p${Math.floor(i / 30)}` : `https://example.com/f${i}` }));
}

// Design preview only: ?privatetip adds a finished private window's download ("Done · private"),
// with the one-time Private mode tip under it.
if (typeof location !== "undefined" && new URLSearchParams(location.search).has("privatetip")) {
  downloads.unshift(base({ name: "Boarding pass.pdf", status: "completed", size: 240_000, downloaded: 240_000, private: true }));
}

const demoSnapshot: Snapshot = {
  downloads,
  found: foundItems,
  // Sample sets, as the engine would send them.
  foundSets: [
    {
      key: "PartRar|project_files",
      base: "Project_files",
      ids: foundItems.filter((f) => f.template.name.startsWith("Project_files.part")).map((f) => f.id),
      missing: ["part 3 missing"],
    },
  ],
  downloadSets: [],
  foundMirrors: mockMirrors(foundItems, ["Holiday video.mp4", "Project_files.part1.rar", "Project_files.part2.rar"]),
  settings: { downloadDir: "C:\\Users\\me\\Downloads", autoConnections: true, connections: 8, maxActive: 3, speedLimitKb: 0, verifyChecksum: true, notifyDone: true, notices: {}, quiet: false, followWindowsQuiet: true, playDoneSound: true, playSentSound: true, extractDefault: false, minimizeToTray: false, closeToTray: false, trayAsked: true, previewQuality: 480, keepAwake: true, confirmQuit: true, resumeOnStart: true, startWithWindows: false, startInTray: true, videoQuality: 1080, setupDone: true, videoAsked: false, autoUpdate: true, scheduleOn: false, scheduleStart: 1320, scheduleEnd: 420, catchClipboard: false, browserConnect: false, installerChoices: false, sendCompression: "off", sendMode: null, sendPartMb: 500, sendPartCustom: false, resumeUploadsOnStart: true, askBrowser: true, miniOnBrowser: true, miniPinned: false, miniPos: null, sendHost: "gofile", uploadDeleteDays: 7, whatsNewSeen: "0.1.1", tourOffered: true, toursDone: [] },
  dataDir: "C:\\Pako\\data",
  sevenZip: false,
  ffmpeg: false,
  version: "0.1.0",
  installed: false,
};

/** "?showcase" in the address: the website's screenshot set (showcase.ts) instead. */
export const mockSnapshot: Snapshot = showcase ? showcaseSnapshot : demoSnapshot;

export const mockProgress = showcase
  ? showcaseProgress
  : downloads
      .filter((d) => d.status === "downloading")
      .map((d) => ({ id: d.id, status: d.status, message: null, downloaded: d.downloaded, size: d.size, speed: 11_400_000, connections: 8, name: d.name }));

/** The parts map of a download (only the showcase set has one). */
export const mockParts = (id: string) => (showcase ? showcaseParts(id) : []);

const DAY = 86_400_000;

/** A stand-in QR code for the design preview (the app makes the real one). */
export function mockQr(seed: string): string {
  const n = 25;
  let h = 2166136261;
  for (const c of seed) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  const finder = (x: number, y: number) =>
    [[0, 0], [n - 7, 0], [0, n - 7]].some(([fx, fy]) => {
      const dx = x - fx, dy = y - fy;
      if (dx < 0 || dy < 0 || dx > 6 || dy > 6) return false;
      return dx === 0 || dy === 0 || dx === 6 || dy === 6 || (dx >= 2 && dx <= 4 && dy >= 2 && dy <= 4);
    });
  const inFinderArea = (x: number, y: number) => (x < 8 && y < 8) || (x >= n - 8 && y < 8) || (x < 8 && y >= n - 8);
  let rects = "";
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++) {
      h = Math.imul(h ^ (x * 31 + y), 16777619);
      const on = inFinderArea(x, y) ? finder(x, y) : (h >>> 7) % 2 === 0;
      if (on) rects += `M${x} ${y}h1v1h-1z`;
    }
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n} ${n}" shape-rendering="crispEdges"><rect width="${n}" height="${n}" fill="#fff"/><path d="${rects}" fill="#000"/></svg>`;
}

/** Send view in the design preview. */
const demoSend = {
  jobs: [
    { id: "s4", name: "Site backup.zip", files: 1840, size: 3_300_000_000, total: 3_300_000_000, sent: 0, speed: 0, status: "uploading", message: null, paths: ["D:\Site backup"], asFiles: false, filesDone: 0, link: null, canResume: false, mode: "parts", partMb: 500, parts: 0, making: true, made: 1_420_000_000 },
    { id: "s1", name: "Wedding photos", files: 214, size: 3_400_000_000, total: 3_400_000_000, sent: 1_250_000_000, speed: 11_800_000, status: "uploading", message: null, paths: [], asFiles: true, filesDone: 78, link: "https://gofile.io/d/Wd2Hp8", canResume: false },
    { id: "s3", name: "38 files", files: 38, size: 920_000_000, total: 920_000_000, sent: 870_000_000, speed: 0, status: "failed", message: "2 files didn't upload. Try again to upload just those (the link stays the same).", paths: ["C:\\Clips"], asFiles: true, filesDone: 36, link: "https://gofile.io/d/Kp4Rt1", canResume: true },
    { id: "s2", name: "Project render v3.mp4", files: 1, size: 820_000_000, total: 820_000_000, sent: 310_000_000, speed: 0, status: "failed", message: "No connection to Gofile. Check your internet, then try again.", paths: ["C:\\Video\\Project render v3.mp4"], asFiles: false, filesDone: 0, link: null, canResume: false },
  ],
  sent: [
    {
      id: "t0", name: "Site backup.zip", files: 1840, size: 3_300_000_000, date: Date.now() - 3_600_000, link: "https://gofile.io/d/Bk3Pt9",
      folderId: "f0", fileId: null, compressed: false, parts: 7, downloads: 0, canDelete: true,
      deleteAt: Date.now() + DAY * 7 - 3_600_000, expiryOnHost: true, secs: 290, peak: 14_200_000,
      contents: [{ name: "Site backup/index.html", size: 24_000 }, { name: "Site backup/media/hero.mp4", size: 1_900_000_000 }],
    },
    {
      id: "t1", name: "Holiday clips", asFiles: true, files: 12, size: 2_100_000_000, date: Date.now() - 3_600_000 * 2, link: "https://gofile.io/d/Xy7Kp2",
      folderId: "f1", fileId: "a1", compressed: false, downloads: 0, canDelete: true,
      deleteAt: Date.now() + DAY * 7 - 3_600_000 * 2, expiryOnHost: true, secs: 186, peak: 15_800_000,
      contents: ["Beach day.mp4", "Boat trip.mp4", "Dinner.mov", "Drone over the bay.mp4", "Fireworks.mp4", "Hike part 1.mp4", "Hike part 2.mp4", "Market.mov", "Morning swim.mp4", "Sunset timelapse.mp4", "Snorkel.mp4", "Waterfall.mp4"].map((name, i) => ({ name, size: 90_000_000 + ((i * 37_000_000) % 260_000_000) })),
      countHistory: [[Date.now() - 3_600_000, 0]],
    },
    {
      id: "t2", name: "Brand guidelines.pdf", files: 1, size: 48_000_000, date: Date.now() - DAY * 1.2, link: "https://drive.google.com/file/d/1Br4nd/view",
      folderId: null, fileId: "d2", compressed: false, host: "drive", canDelete: true,
      deleteAt: Date.now() + DAY * 1.8, secs: 6.1, peak: 9_400_000, contents: [{ name: "Brand guidelines.pdf", size: 48_000_000 }],
    },
    {
      id: "t3", name: "Source files and 2 more.zip", files: 386, size: 640_000_000, date: Date.now() - DAY * 6, link: "https://gofile.io/d/Hd8Wq1",
      folderId: "f3", fileId: "a3", compressed: true, downloads: 4, downloadedAt: Date.now() - DAY * 0.6, canDelete: true,
      deleteAt: null, secs: 71, peak: 12_600_000, note: "For Rado: the v2 sources and the fonts.",
      contents: [{ name: "Source files/main.psd", size: 312_000_000 }, { name: "Source files/layers/sky.psd", size: 98_000_000 }, { name: "Fonts/Inter.ttf", size: 820_000 }, { name: "Readme.txt", size: 4_000 }],
      countHistory: [[Date.now() - DAY * 6, 0], [Date.now() - DAY * 5.2, 1], [Date.now() - DAY * 3.1, 3], [Date.now() - DAY * 0.6, 4]],
    },
    {
      id: "t4", name: "Draft cut.mp4", files: 1, size: 1_300_000_000, date: Date.now() - DAY * 8, link: "https://gofile.io/d/Pz5Nc3",
      folderId: "f4", fileId: "a4", compressed: false, downloads: 0, soon: true, canDelete: true,
      deleteAt: Date.now() - 3_600_000, deleteError: "No connection to Gofile. Check your internet.", deleteTries: 2, secs: 104, peak: 14_100_000,
      contents: [{ name: "Draft cut.mp4", size: 1_300_000_000 }],
    },
    { id: "t5", name: "Old scans", asFiles: true, files: 40, size: 380_000_000, date: Date.now() - DAY * 24, link: "https://gofile.io/d/Lk2Vb7", folderId: "f5", fileId: null, compressed: false, downloads: 2, gone: true, canDelete: false },
  ],
  canDelete: true,
  keepNote: "Gofile keeps files while people download them. After 10 days without a download, they may be removed.",
  checking: false,
  checkedAt: Date.now() - 60_000 * 2,
  countsNote: null,
};
export const mockSend = (showcase ? showcaseSend : demoSend) as typeof demoSend;

// ---------- Dashboard numbers (design preview; "?nonumbers" shows the empty state) ----------

const RADO = "a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e";
const MIA = "f0e1d2c3b4a5968778695a4b3c2d1e0f1f2e3d4c5b6a79881726354a3b2c1d0e";
const HERY = "5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f";

type Kind = "download" | "upload" | "sent" | "received";
interface MockEvent {
  site?: string;
  at: number;
  day: string;
  kind: Kind;
  name: string;
  bytes: number;
  files: number;
  secs: number | null;
  peak: number;
  source: string;
  route: string;
  contact: string;
  ok: boolean;
}

const dayOf = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

/** The same "random" numbers every time for a given day (the preview doesn't jump around). */
function seeded(text: string) {
  let a = 2166136261;
  for (const ch of text) a = Math.imul(a ^ ch.charCodeAt(0), 16777619);
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

let mockEventList: MockEvent[] | null = null;
function mockEvents(): MockEvent[] {
  if (mockEventList) return mockEventList;
  const out: MockEvent[] = [];
  if (typeof location !== "undefined" && location.search.includes("nonumbers")) return (mockEventList = out);
  const now = new Date();
  const todayKey = dayOf(now);
  const DAYS = 240;
  const names = showcase
    ? {
        drive: ["Holiday photos 2025.zip", "Studio assets.zip", "Interview A-cam.mov", "Course materials.zip", "Mountain timelapse 4K.mkv"],
        video: ["Sunrise over the lake 4K.mp4", "Cooking class week 3.mp4", "Guitar lesson 12.mp4", "Night sky timelapse.mp4"],
        web: ["Wallpapers 4K.zip", "Font collection.zip", "Brushes and textures.zip", "Podcast season 2.zip", "Sample project.zip"],
        up: ["Holiday clips", "Portfolio 2025.pdf", "Source files.zip", "Wedding photos", "Draft cut.mp4"],
        direct: ["Short film final.mov", "Photos from Saturday", "Music for the trailer.zip", "Slides.pdf", "Recipes", "Raw footage"],
      }
    : {
        drive: ["Chanel shoot RAW.zip", "Brand assets 2026.zip", "Interview A-cam.mov", "Lookbook final.pdf", "Colour pass v2.mp4"],
        video: ["Live set at Antsahamanitra.mp4", "Tutorial - colour grading.mp4", "Concert highlights.mkv", "Podcast 112.m4a"],
        web: ["ubuntu-26.04-desktop.iso", "Blender 5.2.msi", "Font pack.zip", "Sample library.rar", "Driver update.exe"],
        up: ["Wedding photos", "Draft cut.mp4", "Holiday clips", "Source files.zip", "Contract scans"],
        direct: ["Project.zip", "Render final v3.mp4", "Family photos", "Song stems", "Invoice October.pdf", "Raw footage"],
      };
  for (let back = DAYS; back >= 0; back--) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - back, 12);
    const day = dayOf(d);
    const r = seeded(day);
    const today = day === todayKey;
    if (!today && r() < 0.2) continue; // a quiet day
    const busy = 0.45 + 0.55 * (1 - back / DAYS);
    const hourOf = () => (today ? Math.floor(r() * (now.getHours() + 1)) : 8 + Math.floor(r() * 15));
    const at = (h: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate(), h, Math.floor(r() * 60)).getTime();
    const pick = <T,>(list: T[]) => list[Math.floor(r() * list.length)];
    const nDl = 1 + Math.floor(r() * 5 * busy + 0.4);
    for (let i = 0; i < nDl; i++) {
      const x = r();
      const source = x < 0.38 ? "drive" : x < 0.7 ? "video" : "web";
      const avg = source === "drive" ? (8 + r() * 6) * MB : source === "video" ? (4 + r() * 4) * MB : (2 + r() * 9) * MB;
      const bytes = Math.round(source === "drive" ? (0.4 + r() * 5) * GB : source === "video" ? (0.15 + r() * 2.2) * GB : (0.05 + r() * 1.4) * GB);
      const ok = r() > 0.05 || showcase;
      // The showcase names no video sites, and only public-interest sites for the rest.
      const site = showcase
        ? source === "drive" ? "drive.google.com" : source === "video" ? "" : pick(["archive.org", "archive.org", "upload.wikimedia.org", "gutenberg.org"])
        : source === "drive" ? "drive.google.com" : source === "video" ? pick(["youtube.com", "youtube.com", "vimeo.com", "facebook.com"]) : pick(["releases.ubuntu.com", "download.blender.org", "github.com", "dafont.com", "mediafire.com"]);
      out.push({ site, at: at(hourOf()), day, kind: "download", name: pick(names[source]), bytes, files: 1, secs: bytes / avg, peak: avg * (1.25 + r() * 0.6), source, route: "", contact: "", ok });
    }
    if (r() < 0.35 * busy) {
      const route = r() < 0.65 ? "gofile" : "drive";
      const avg = route === "gofile" ? (1.8 + r() * 1.7) * MB : (3.5 + r() * 2.5) * MB;
      const bytes = Math.round((0.1 + r() * 2.4) * GB);
      out.push({ at: at(hourOf()), day, kind: "upload", name: pick(names.up), bytes, files: 1 + Math.floor(r() * 30), secs: bytes / avg, peak: avg * (1.2 + r() * 0.4), source: "", route, contact: "", ok: true });
    }
    const nDirect = r() < 0.45 ? 1 + Math.floor(r() * 2) : 0;
    for (let i = 0; i < nDirect; i++) {
      const y = r();
      const contact = y < 0.5 ? RADO : y < 0.8 ? MIA : HERY;
      // Some of Rado's go over the home network (no extra random number: the rest stays the same).
      const route = contact === RADO ? (i === 1 ? "local" : "direct6") : contact === MIA ? "direct" : "relay";
      const avg = route === "direct6" || route === "local" ? (24 + r() * 20) * MB : route === "direct" ? (12 + r() * 8) * MB : (2 + r() * 2) * MB;
      const outgoing = r() < 0.55;
      const bytes = Math.round((0.05 + r() * 3) * GB);
      out.push({ at: at(hourOf()), day, kind: outgoing ? "sent" : "received", name: pick(names.direct), bytes, files: 1 + Math.floor(r() * 3), secs: bytes / avg, peak: avg * (1.15 + r() * 0.35), source: outgoing ? "" : "direct", route, contact, ok: true });
    }
  }
  return (mockEventList = out);
}

function avgOf(timed: number, secs: number) {
  return secs > 0 && timed > 0 ? timed / secs : null;
}

const sumBytes = (x: MockEvent[]) => x.reduce((t, e) => t + e.bytes, 0);
const sumSecs = (x: MockEvent[]) => x.reduce((t, e) => t + (e.secs ?? 0), 0);
const maxPeak = (x: MockEvent[]) => x.reduce((m, e) => Math.max(m, e.peak), 0);

export function mockNumbers(period: string): NumbersView {
  const all = mockEvents();
  const now = new Date();
  const today = dayOf(now);
  const since = all.length ? all.reduce((m, e) => (e.day < m ? e.day : m), today) : null;
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 12);
  if (period === "week") start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  if (period === "month") start.setDate(1);
  const from = period === "all" ? (since ?? today) : dayOf(start);
  const list = all.filter((e) => e.day >= from && e.day <= today);
  const okList = list.filter((e) => e.ok);
  const of = (kinds: Kind[]) => okList.filter((e) => kinds.includes(e.kind));

  const totals = (kinds: Kind[]): NumbersTotals => {
    const ok = of(kinds);
    return {
      bytes: sumBytes(ok),
      files: ok.reduce((s, e) => s + e.files, 0),
      count: ok.length,
      failed: list.filter((e) => kinds.includes(e.kind)).length - ok.length,
      avg: avgOf(sumBytes(ok), sumSecs(ok)),
      peak: maxPeak(ok),
      people: new Set(ok.filter((e) => e.contact).map((e) => e.contact)).size,
      directBytes: sumBytes(ok.filter((e) => e.route === "direct6" || e.route === "direct" || e.route === "local")),
    };
  };

  // Bars: hours of today, days, or months for a long "All time".
  const firstDate = new Date(from + "T12:00:00");
  const spanDays = Math.round((new Date(today + "T12:00:00").getTime() - firstDate.getTime()) / 86400000);
  const step: NumbersView["step"] = period === "today" ? "hour" : spanDays <= 62 ? "day" : "month";
  const keys: string[] = [];
  if (step === "hour") for (let h = 0; h < 24; h++) keys.push(String(h).padStart(2, "0"));
  else if (step === "day") for (const d = new Date(firstDate); dayOf(d) <= today; d.setDate(d.getDate() + 1)) keys.push(dayOf(d));
  else for (const d = new Date(firstDate.getFullYear(), firstDate.getMonth(), 1, 12); dayOf(d) <= today; d.setMonth(d.getMonth() + 1)) keys.push(dayOf(d).slice(0, 7));
  const keyOf = (e: MockEvent) => (step === "hour" ? String(new Date(e.at).getHours()).padStart(2, "0") : step === "day" ? e.day : e.day.slice(0, 7));
  const buckets: NumbersBucket[] = keys.map((key) => {
    const es = okList.filter((e) => keyOf(e) === key);
    const k = (kinds: Kind[]) => es.filter((e) => kinds.includes(e.kind));
    const [dl, up, se, re, di] = [k(["download"]), k(["upload"]), k(["sent"]), k(["received"]), k(["sent", "received"])];
    return {
      key,
      download: sumBytes(dl),
      upload: sumBytes(up),
      sent: sumBytes(se),
      received: sumBytes(re),
      downloadAvg: avgOf(sumBytes(dl), sumSecs(dl)),
      downloadPeak: maxPeak(dl),
      uploadAvg: avgOf(sumBytes(up), sumSecs(up)),
      uploadPeak: maxPeak(up),
      directAvg: avgOf(sumBytes(di), sumSecs(di)),
      directPeak: maxPeak(di),
    };
  });

  const routeRows = (routes: string[], match: (e: MockEvent, r: string) => boolean): NumbersRoute[] =>
    routes.map((route) => {
      const es = okList.filter((e) => match(e, route));
      return { route, bytes: sumBytes(es), count: es.length, avg: avgOf(sumBytes(es), sumSecs(es)), peak: maxPeak(es) };
    });
  const uploadRoutes = routeRows(["gofile", "drive", "direct"], (e, r) => (r === "direct" ? e.kind === "sent" : e.kind === "upload" && e.route === r));
  const downloadSources = routeRows(["drive", "video", "web", "direct"], (e, r) => (r === "direct" ? e.kind === "received" : e.kind === "download" && e.source === r));

  const people: NumbersPerson[] = [RADO, MIA, HERY]
    .map((contact) => {
      const es = okList.filter((e) => e.contact === contact);
      const s = es.filter((e) => e.kind === "sent");
      const g = es.filter((e) => e.kind === "received");
      return {
        contact,
        sent: sumBytes(s),
        received: sumBytes(g),
        count: es.length,
        peakSent: maxPeak(s),
        peakReceived: maxPeak(g),
        avgSent: avgOf(sumBytes(s), sumSecs(s)),
        avgReceived: avgOf(sumBytes(g), sumSecs(g)),
        last: es.reduce((m, e) => Math.max(m, e.at), 0),
      };
    })
    .filter((p) => p.count > 0)
    .sort((a, b) => b.sent + b.received - (a.sent + a.received));

  const bySite = new Map<string, { bytes: number; count: number }>();
  for (const e of of(["download"])) {
    if (!e.site) continue;
    const s = bySite.get(e.site) ?? { bytes: 0, count: 0 };
    s.bytes += e.bytes;
    s.count++;
    bySite.set(e.site, s);
  }
  const sites = [...bySite].map(([site, s]) => ({ site, ...s })).sort((a, b) => b.bytes - a.bytes).slice(0, 6);

  const top = (xs: MockEvent[], v: (e: MockEvent) => number) => xs.reduce<MockEvent | null>((m, e) => (!m || v(e) > v(m) ? e : m), null);
  const fastest = top(of(["download"]), (e) => e.peak);
  const biggest = top(of(["download", "received"]), (e) => e.bytes);
  const perDay = new Map<string, number>();
  for (const e of okList) perDay.set(e.day, (perDay.get(e.day) ?? 0) + e.bytes);
  const busiest = [...perDay].sort((a, b) => b[1] - a[1])[0];

  return {
    period: period as NumbersView["period"],
    from,
    to: today,
    since,
    step,
    download: totals(["download"]),
    upload: totals(["upload"]),
    sent: totals(["sent"]),
    received: totals(["received"]),
    buckets,
    uploadRoutes,
    downloadSources,
    people,
    sites,
    records: {
      fastest: fastest && { name: fastest.name, value: fastest.peak, day: fastest.day },
      biggest: biggest && { name: biggest.name, value: biggest.bytes, day: biggest.day },
      busiest: busiest ? { name: "", value: busiest[1], day: busiest[0] } : null,
    },
  };
}

export function mockContactNumbers(id: string): ContactNumbers {
  const es = mockEvents().filter((e) => e.ok && e.contact === id);
  const s = es.filter((e) => e.kind === "sent");
  const g = es.filter((e) => e.kind === "received");
  return {
    sent: sumBytes(s),
    received: sumBytes(g),
    sentCount: s.length,
    receivedCount: g.length,
    peakSent: maxPeak(s),
    peakReceived: maxPeak(g),
    avgSent: avgOf(sumBytes(s), sumSecs(s)),
    avgReceived: avgOf(sumBytes(g), sumSecs(g)),
    directBytes: sumBytes(es.filter((e) => e.route !== "relay")),
    localBytes: sumBytes(es.filter((e) => e.route === "local")),
    relayBytes: sumBytes(es.filter((e) => e.route === "relay")),
    last: es.length ? Math.max(...es.map((e) => e.at)) : null,
  };
}

/** The bell in the design preview: files from Mia waiting for an answer (its Accept / Decline
 *  show with "?offer" in the address, which also puts the offer in Direct), a link that needs
 *  you, and an answered connection request from yesterday. */
const demoNotices: Notice[] = [
  {
    id: "n3",
    kind: "direct-offer",
    title: "Files for you",
    body: "Mia wants to send you Holiday photos (1.2 GB).",
    at: Date.now() - 4 * 60_000,
    read: false,
    go: { page: "direct", id: MIA, offer: "o-mia" },
  },
  {
    id: "n2",
    kind: "needs-you",
    title: "Pako needs you",
    body: "Solve a quick check to open a link from ouo.io.",
    at: Date.now() - 52 * 60_000,
    read: false,
    go: { page: "found", id: "f1" },
  },
  {
    id: "n1",
    kind: "direct-request",
    title: "Someone wants to connect",
    body: "Hery wants to connect with you in Pako.",
    at: Date.now() - 26 * 3600_000,
    read: true,
    go: { page: "direct", peer: HERY },
    answered: "Accepted",
  },
];
export const mockNotices: Notice[] = showcase ? showcaseNotices : demoNotices;
if (typeof location !== "undefined" && new URLSearchParams(location.search).get("update") === "ready") {
  mockNotices.unshift({ id: "n-update", kind: "update-ready", title: "Pako 1.1.1 is ready", body: "Restart Pako to start using it. Your downloads and lists stay.", at: Date.now() - 2 * 60_000, read: false, go: { page: "update" } });
}

/** Help › Report a problem: what a report would carry (?report=empty: no problems noted yet). */
export function mockReport(): ReportInfo {
  const empty = new URLSearchParams(location.search).get("report") === "empty";
  const now = Date.now();
  const lines: [number, ReportInfo["problems"][number]["area"], string][] = [
    [26 * 3600, "update", "Couldn't check for updates: the update page answered 503. Try again later."],
    [25 * 3600, "linkCheck", "File not found on the server (404)."],
    [22 * 3600, "download", "Server error (502). Pako tries again by itself."],
    [21 * 3600, "video", "YouTube asks to sign in to prove you're not a bot. Try again later."],
    [20 * 3600, "upload", "Gofile didn't answer. Click Try again."],
    [9 * 3600, "direct", "The connection was lost."],
    [8 * 3600, "download", "Not enough space on the disk. Free some space, then click Retry."],
    [7 * 3600, "extension", "Pako didn't start in time. Open Pako, then try again."],
    [6 * 3600, "linkCheck", "Reached the file link, but: the server answered 403 (https://cdn.example.com)."],
    [5 * 3600, "video", "This video is private."],
    [4 * 3600, "download", "Can't write to %USERPROFILE%\\Downloads\\Videos: access denied. Pick another folder."],
    [3 * 3600, "download", "Download stopped before the end. Click Retry."],
    [2 * 3600, "direct", "Their Pako couldn't save it: D:\\Users\\…\\Downloads is full."],
    [90 * 60, "upload", "1 file didn't upload."],
    [70 * 60, "download", "Server busy (408). Pako tries again by itself."],
    [50 * 60, "linkCheck", "The server answered 429 (https://drive.usercontent.google.com)."],
    [40 * 60, "download", "File not found on the server (404)."],
    [30 * 60, "video", "Age-restricted: it needs a signed-in browser, which Pako doesn't support yet."],
    [20 * 60, "download", "Server error (500). Pako tries again by itself."],
    [12 * 60, "download", "Downloaded, but couldn't rename the file: it is open in another app. Click Try again."],
    [8 * 60, "linkCheck", "File not found on the server (404)."],
    [5 * 60, "direct", "No answer."],
    [3 * 60, "download", "Couldn't reach the server (https://files.example.org). Pako tries again by itself."],
  ];
  return {
    version: pakoVersion,
    installed: true,
    windows: "Windows 11 Pro 24H2 (build 26100)",
    problems: empty ? [] : lines.map(([ago, area, message], i) => ({ time: now - ago * 1000, area, message, version: i < 2 ? "1.2.10" : pakoVersion })),
  };
}
