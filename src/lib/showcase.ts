// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The showcase set: sample data for the website's screenshots (scripts/screenshots.mjs), used only
// in the design preview (a normal browser, never inside the app) and only with "?showcase" in the
// address. Everything going well; plain made-up names, no brands, no real people.
import type { DirectFrom, DirectHistory, DirectState, Download, FoundItem, Part, Progress, RemoteBook, RemoteTalk, SendState, Snapshot } from "./api";
import type { Notice, PopupCard } from "./notices";

export const showcase = typeof location !== "undefined" && new URLSearchParams(location.search).has("showcase");
/** A scene for one picture, from "?showcase=<scene>": "sets" (Links with split archives),
 *  "share" (a fresh upload and the same files received in Links), "hosts" (the same file on
 *  several hosts in Links, and a download that went on from a backup host), "local" (Direct
 *  with a contact on the same home network), "browser" (a download made by the browser for
 *  Pako), "parts" (Upload with a ZIP in parts on its way) or "video" (a video downloading at
 *  the top of the tray window, above three usual downloads); none = the usual set. */
const scene = showcase ? new URLSearchParams(location.search).get("showcase") : null;

const now = Date.now();
const MIN = 60_000;
const HOUR = 3_600_000;
const DAY = 86_400_000;
const GB = 1024 ** 3;
const MB = 1024 ** 2;
const HOME = "C:\\Users\\Sam\\Downloads";

/** The same contact IDs as the Dashboard's sample numbers (mock.ts), so "Your people" matches. */
export const ALEX = "a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e";
export const NINA = "f0e1d2c3b4a5968778695a4b3c2d1e0f1f2e3d4c5b6a79881726354a3b2c1d0e";
export const JO = "5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f";
const LEO = "7c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d";
/** Not checked lately: nothing with Maya for 45 days. */
const MAYA = "2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c";

// ---------- Downloads ----------

let seq = 0;
function dl(over: Partial<Download> & { name: string }): Download {
  seq++;
  const url = "https://files.example.com/" + encodeURIComponent(over.name);
  return {
    id: "s" + seq,
    link: url,
    source: { kind: "http", url },
    destDir: HOME,
    size: null,
    md5: null,
    status: "downloading",
    message: null,
    resumable: true,
    downloaded: 0,
    addedAt: 1000 + seq,
    date: now - seq * 7 * MIN,
    finishedAt: null,
    group: null,
    resolved: true,
    priority: "normal",
    notify: false,
    extract: false,
    ...over,
  };
}
const drive = (fileId: string) => ({ source: { kind: "gDrive", fileId } as const, link: `https://drive.google.com/file/d/${fileId}/view`, md5: "x" });
const STUDIO = HOME + "\\Studio assets";

/** "?showcase=hosts": a download whose first host failed for good, going on from the next one. */
const MIRROR_NAME = "Field recordings.zip";
const mirrorLink = (host: string, path: string) => `https://${host}/${path}/${encodeURIComponent(MIRROR_NAME)}`;
const switched: Download[] = scene === "hosts"
  ? [
      dl({
        name: "Live set.flac",
        size: 1.8 * GB,
        downloaded: 0.74 * GB,
        link: "https://files.example.com/audio/Live%20set.flac",
        source: { kind: "http", url: "https://files.example.com/audio/Live%20set.flac" },
        tried: [{ link: "https://dl.example.net/f/9Qx1/Live%20set.flac", source: { kind: "http", url: "https://dl.example.net/f/9Qx1/Live%20set.flac" }, size: 1.8 * GB, md5: null, why: "File not found on the server (404)." }],
        mirrors: [{ link: "https://share.example.org/d/Qm4t/Live%20set.flac", source: { kind: "http", url: "https://share.example.org/d/Qm4t/Live%20set.flac" }, size: 1.8 * GB, md5: null }],
      }),
    ]
  : [];

/** "?showcase=browser": a file only the browser could get ("Via browser"), and a private
 *  window's finished download. */
const viaBrowser: Download[] = scene === "browser"
  ? [
      dl({
        name: "Annual report 2025.pdf",
        size: 412 * MB,
        downloaded: 167 * MB,
        resumable: false,
        link: "https://reports.example.org/get?id=Q4-2025&t=8f2c",
        source: { kind: "http", url: "https://reports.example.org/get?id=Q4-2025&t=8f2c" },
        viaBrowser: { browserId: 41, path: HOME + "\\Annual report 2025.pdf", canResume: true, light: false },
      }),
      dl({ name: "Boarding pass.pdf", status: "completed", size: 2.6 * MB, downloaded: 2.6 * MB, private: true, finishedAt: now - 2 * MIN }),
    ]
  : [];

/** "?showcase=video": a video from a video site, about 40% down, first in the list. */
const VIDEO_DL_LINK = "https://media.example.com/watch/lake-at-sunrise";
const videoDl: Download[] = scene === "video"
  ? [
      dl({
        name: "Lake at sunrise.mp4",
        size: 1.3 * GB,
        downloaded: 0.53 * GB,
        link: VIDEO_DL_LINK,
        source: { kind: "video", url: VIDEO_DL_LINK, quality: 1080, audio: null, duration: 612 },
      }),
    ]
  : [];
/** The usual downloads shown under it in "?showcase=video". */
const VIDEO_SCENE_KEEP = new Set(["Holiday photos 2025.zip", "Interview A-cam.mov", "Wallpapers 4K.zip"]);

const allDownloads: Download[] = [
  ...switched,
  ...viaBrowser,
  dl({ name: "Holiday photos 2025.zip", size: 6.4 * GB, downloaded: 3.71 * GB, ...drive("1hP7sQ2"), notify: true }),
  dl({ name: "Project final cut.mp4", size: 3.9 * GB, downloaded: 1.21 * GB }),
  dl({ name: "Interview A-cam.mov", size: 4.6 * GB, downloaded: 2.02 * GB, ...drive("1sA9cM4"), group: "Studio assets", destDir: STUDIO }),
  dl({ name: "Interview B-cam.mov", size: 4.4 * GB, downloaded: 0.53 * GB, ...drive("1sB2cM7"), group: "Studio assets", destDir: STUDIO }),
  dl({ name: "Logo pack.zip", status: "completed", size: 240 * MB, downloaded: 240 * MB, ...drive("1sL5pK1"), group: "Studio assets", destDir: STUDIO, finishedAt: now - 9 * MIN }),
  dl({ name: "Wallpapers 4K.zip", size: 1.2 * GB, downloaded: 0.98 * GB }),
  dl({ name: "Podcast season 2.zip", status: "queued", size: 2.3 * GB }),
  dl({ name: "Course materials.zip", status: "completed", size: 2.7 * GB, downloaded: 2.7 * GB, ...drive("1cM8tR3"), date: now - 3 * HOUR, finishedAt: now - 52 * MIN }),
];
const downloads: Download[] = scene === "video" ? [...videoDl, ...allDownloads.filter((d) => VIDEO_SCENE_KEEP.has(d.name))] : allDownloads;

/** Speed (bytes/s) and connections of each running one. */
const running: Record<string, [number, number]> = {
  "Live set.flac": [22_300_000, 8],
  "Lake at sunrise.mp4": [14_600_000, 1],
  "Holiday photos 2025.zip": [26_400_000, 8],
  "Project final cut.mp4": [18_900_000, 8],
  "Interview A-cam.mov": [21_700_000, 6],
  "Interview B-cam.mov": [16_800_000, 6],
  "Wallpapers 4K.zip": [9_600_000, 4],
  "Annual report 2025.pdf": [3_100_000, 1],
};

export const showcaseProgress: Progress[] = downloads
  .filter((d) => d.status === "downloading")
  .map((d) => ({ id: d.id, status: d.status, message: null, downloaded: d.downloaded, size: d.size, speed: running[d.name]?.[0] ?? 0, connections: running[d.name]?.[1] ?? 8, name: d.name }));

/** The parts map of a running download: equal parts, the first ones furthest along. */
export function showcaseParts(id: string): Part[] {
  const d = downloads.find((x) => x.id === id);
  if (!d || !d.size || d.status !== "downloading") return [];
  const count = running[d.name]?.[1] ?? 8;
  const part = d.size / count;
  const avg = d.downloaded / d.size;
  const spread = Math.min(0.8, 2 * Math.min(avg, 1 - avg));
  return Array.from({ length: count }, (_, i) => {
    const fill = Math.max(0.04, Math.min(1, avg + spread * (0.5 - i / (count - 1))));
    const start = Math.round(i * part);
    const end = Math.round((i + 1) * part) - 1;
    return { start, end, done: Math.round(fill * part), active: fill < 1 };
  });
}

// ---------- Links ----------

let fseq = 0;
function found(over: { name: string; size?: number | null; source?: Download["source"]; link?: string; state?: FoundItem["state"]; folder?: string; relDir?: string }): FoundItem {
  fseq++;
  const link = over.link ?? "https://files.example.com/" + encodeURIComponent(over.name);
  const template = dl({ name: over.name, size: over.size ?? null, link, source: over.source ?? { kind: "http", url: link }, status: "queued", date: now - 4 * MIN + fseq * 1000 });
  template.id = "f" + fseq;
  return { id: template.id, template, state: over.state ?? "ready", message: null, folder: over.folder ?? null, relDir: over.folder ? (over.relDir ?? "") : null, isFolder: false, dest: null, extract: null };
}
const tripLink = "https://drive.google.com/drive/folders/1TrIp9";
const inTrip = (name: string, size: number, fileId: string) => found({ name, size, source: { kind: "gDrive", fileId }, link: tripLink, folder: "Summer trip" });

/** The sample Home PC's Pako ID (My devices' sample devices, devices.svelte.ts). */
const HOME_PC = "9c1d7a4e2b6f80135c7e9a2d4f6b8c0e1a3c5e7092b4d6f8a0c2e4f6a8b0c2d4";

/** A video with its qualities (for the quality picker). No site: the link is a plain address. */
const VIDEO_LINK = "https://media.example.com/watch/sunrise-over-the-lake";
const foundItems: FoundItem[] = [
  inTrip("Beach day.mp4", 1.4 * GB, "1tB1"),
  inTrip("Drone over the bay.mp4", 2.3 * GB, "1tD2"),
  inTrip("Photos.zip", 860 * MB, "1tP3"),
  inTrip("Itinerary.pdf", 2.4 * MB, "1tI4"),
  // Set to download on Home PC (Links › details › Download on): its row shows the PC mark.
  { ...found({ name: "Mountain timelapse 4K.mkv", size: 1.2 * GB, source: { kind: "gDrive", fileId: "1mT5" }, link: "https://drive.google.com/file/d/1mT5/view" }), device: HOME_PC },
  found({ name: "Brushes and textures.zip", size: 640 * MB }),
  found({ name: "Font collection.zip", size: 96 * MB }),
  found({
    name: "Sunrise over the lake 4K.mp4",
    size: 1.1 * GB,
    link: VIDEO_LINK,
    source: { kind: "video", url: VIDEO_LINK, quality: null, audio: null, duration: 754, audioSize: 12 * MB, options: [2160, 1440, 1080, 720, 480, 360].map((height, i) => ({ height, size: [2.3 * GB, 1.1 * GB, 560 * MB, 280 * MB, 130 * MB, 76 * MB][i] })) },
  }),
  found({ name: "Presentation deck.pdf", source: { kind: "gDrive", fileId: "1pD6" }, link: "https://drive.google.com/file/d/1pD6/view", state: "checking" }),
];

/** "?showcase=sets": many links at once. A complete split archive, one missing a part, a link
 *  that was a page to click through (now its file, on the host it led to, and the same file on
 *  a second host), a file that is
 *  already downloaded (Course materials.zip, see Downloads) and one link still being checked. */
const setsItems: FoundItem[] = scene === "sets"
  ? [
      ...[1, 2, 3, 4].map((n) => found({ name: `Holiday.part${n}.rar`, size: n < 4 ? 1 * GB : 420 * MB, link: `https://files.example.com/holiday/Holiday.part${n}.rar` })),
      ...["001", "002", "004"].map((n) => found({ name: `Scans.7z.${n}`, size: n < "004" ? 2 * GB : 760 * MB, link: `https://files.example.com/scans/Scans.7z.${n}` })),
      found({ name: "Field recordings.zip", size: 318 * MB, link: "https://dl.example.net/f/7Rk2/Field%20recordings.zip" }),
      // The same file on a second host: one row, "on 2 hosts".
      found({ name: "Field recordings.zip", size: 318 * MB, link: "https://files.example.com/audio/Field%20recordings.zip" }),
      found({ name: "Course materials.zip", size: 2.7 * GB, ...drive("1cM8tR3") }),
      found({ name: "Brushes and textures.zip", size: 640 * MB }),
      found({ name: "Font collection.zip", size: 96 * MB }),
      found({ name: "Presentation deck.pdf", source: { kind: "gDrive", fileId: "1pD6" }, link: "https://drive.google.com/file/d/1pD6/view", state: "checking" }),
    ]
  : [];
const setsOf = (prefix: string) => setsItems.filter((f) => f.template.name.startsWith(prefix)).map((f) => f.id);

/** "?showcase=hosts": the same file on three hosts, one row in Links ("on 3 hosts"). */
const hostsItems: FoundItem[] = scene === "hosts"
  ? [
      found({ name: MIRROR_NAME, size: 318 * MB, link: mirrorLink("dl.example.net", "f/7Rk2") }),
      found({ name: MIRROR_NAME, size: 318 * MB, link: mirrorLink("files.example.com", "audio") }),
      found({ name: MIRROR_NAME, size: 318 * MB, link: mirrorLink("share.example.org", "d/Pz8w") }),
    ]
  : [];

/** "?showcase=share": the upload made in the share pictures ("Wedding", to Google Drive), as
 *  the person it was sent to sees it after pasting its link into Links. */
const WEDDING_LINK = "https://drive.google.com/drive/folders/1Wd8Rk";
const weddingPhotos = Array.from({ length: 214 }, (_, i) => ({ name: `Photo ${String(i + 1).padStart(3, "0")}.jpg`, size: 12_400_000 + ((i * 2_750_000) % 7_300_000) }));
const weddingFiles = [{ name: "Album masters.zip", size: 840_000_000 }, { name: "Contract final.pdf", size: 2_100_000 }, ...weddingPhotos];
const shareItems: FoundItem[] = scene === "share"
  ? weddingFiles.map((f, i) => found({ name: f.name, size: f.size, source: { kind: "gDrive", fileId: "1wD" + i }, link: WEDDING_LINK, folder: "Wedding", relDir: i < 2 ? "" : "Wedding photos" }))
  : [];

export const showcaseSnapshot: Snapshot = {
  downloads,
  found: scene === "sets" ? setsItems : scene === "share" ? shareItems : scene === "hosts" ? [...foundItems.slice(0, 4), ...hostsItems, ...foundItems.slice(4)] : foundItems,
  // As the engine would send them (src-tauri/src/split.rs).
  foundSets:
    scene === "sets"
      ? [
          { key: "PartRar|holiday", base: "Holiday", ids: setsOf("Holiday.part"), missing: [] },
          { key: "Numbered|scans.7z", base: "Scans.7z", ids: setsOf("Scans.7z."), missing: ["part 003 missing"] },
        ]
      : [],
  // The same file on several hosts, as the engine would send them (src-tauri/src/found.rs).
  foundMirrors:
    scene === "sets"
      ? (() => {
          const rows = setsItems.filter((f) => f.template.name === "Field recordings.zip");
          return [{ key: rows[0].id, name: "Field recordings.zip", size: 318 * MB, ids: rows.map((f) => f.id), hosts: ["dl.example.net", "files.example.com"], chosen: rows[0].id, picked: false }];
        })()
      : scene === "hosts"
        ? [{ key: hostsItems[0].id, name: MIRROR_NAME, size: 318 * MB, ids: hostsItems.map((f) => f.id), hosts: ["dl.example.net", "files.example.com", "share.example.org"], chosen: hostsItems[0].id, picked: false }]
        : [],
  downloadSets: [],
  settings: {
    downloadDir: HOME, autoConnections: true, connections: 8, maxActive: 5, speedLimitKb: 0, verifyChecksum: true, notifyDone: true, notices: {}, quiet: false, followWindowsQuiet: true, playDoneSound: true,
    extractDefault: true, minimizeToTray: true, closeToTray: true, trayAsked: true, previewQuality: 480, keepAwake: true, confirmQuit: true, resumeOnStart: true, startWithWindows: true, startInTray: true,
    videoQuality: 1080, setupDone: true, videoAsked: true, autoUpdate: true, scheduleOn: false, scheduleStart: 1320, scheduleEnd: 420, catchClipboard: true, browserConnect: true,
    sendCompression: "off", sendMode: "files", sendPartMb: 500, sendPartCustom: false, resumeUploadsOnStart: true, askBrowser: true, askNotify: true, miniOnBrowser: true, miniPinned: false, miniPos: null, sendHost: "gofile", uploadDeleteDays: 7,
    whatsNewSeen: "1.0.0", tourOffered: true, toursDone: ["downloads", "found", "send", "direct", "devices", "dashboard", "settings", "full"],
  },
  dataDir: "C:\\Users\\Sam\\AppData\\Roaming\\Pako",
  sevenZip: true,
  ffmpeg: true,
  version: "1.0.0",
  installed: true,
};

// ---------- Upload ----------

/** Your Google Drive for uploads, connected (an address that can't belong to anyone). */
export const showcaseDrive = { configured: true, missingNote: null, signedIn: true, email: "sam@example.com", limit: 15e9, used: 6.1e9, error: null };

/** "?showcase=parts": a ZIP in parts on its way (3 of its 7 parts up). */
const inParts: SendState["jobs"] = scene === "parts"
  ? [
      {
        id: "j1", name: "Site backup.zip", files: 1840, size: 3_300_000_000, total: 3_300_000_000, sent: 1_480_000_000, speed: 14_600_000, status: "uploading", message: null,
        paths: ["D:\\Site backup"], asFiles: false, filesDone: 3, link: null, canResume: false, mode: "parts", host: "gofile", deleteDays: 7, partMb: 500, parts: 7, making: false,
      },
    ]
  : [];

/** "?showcase=share": the upload just made, at the top of the list. */
const justSent: SendState["sent"] = scene === "share"
  ? [
      {
        id: "u0", name: "Wedding", asFiles: true, files: weddingFiles.length, size: weddingFiles.reduce((s, f) => s + f.size, 0), date: now - MIN, link: WEDDING_LINK,
        folderId: "1Wd8Rk", fileId: null, compressed: false, host: "drive", canDelete: true, account: "sam@example.com",
        deleteAt: now + 7 * DAY - MIN, secs: 312, peak: 17_200_000,
        contents: weddingFiles.map((f, i) => ({ name: i < 2 ? f.name : "Wedding photos/" + f.name, size: f.size })),
      },
    ]
  : [];

export const showcaseSend: SendState = {
  jobs: inParts,
  // Pako's uploads in the sample Drive (the Drive bar says so).
  driveSends: 3,
  driveUsed: 3_900_000_000,
  sent: [
    ...justSent,
    {
      id: "u1", name: "Holiday clips", asFiles: true, files: 12, size: 2_100_000_000, date: now - 3 * HOUR, link: "https://gofile.io/d/Hc7Kp2",
      folderId: "f1", fileId: "a1", compressed: false, downloads: 5, downloadedAt: now - 35 * MIN, canDelete: true,
      deleteAt: now + 7 * DAY - 3 * HOUR, expiryOnHost: true, secs: 168, peak: 16_400_000,
      contents: ["Beach day.mp4", "Boat trip.mp4", "Dinner.mov", "Drone over the bay.mp4", "Fireworks.mp4", "Hike part 1.mp4", "Hike part 2.mp4", "Market.mov", "Morning swim.mp4", "Sunset timelapse.mp4", "Snorkel.mp4", "Waterfall.mp4"].map((name, i) => ({ name, size: 90_000_000 + ((i * 37_000_000) % 260_000_000) })),
      countHistory: [[now - 3 * HOUR, 0], [now - 2.4 * HOUR, 1], [now - 1.7 * HOUR, 3], [now - 70 * MIN, 4], [now - 35 * MIN, 5]],
    },
    {
      id: "u2", name: "Portfolio 2025.pdf", files: 1, size: 48_000_000, date: now - 1.2 * DAY, link: "https://drive.google.com/file/d/1Pf2025/view",
      folderId: null, fileId: "d2", compressed: false, host: "drive", canDelete: true,
      deleteAt: now + 1.8 * DAY, secs: 5.4, peak: 10_100_000, contents: [{ name: "Portfolio 2025.pdf", size: 48_000_000 }],
    },
    {
      id: "u3", name: "Source files.zip", files: 386, size: 640_000_000, date: now - 6 * DAY, link: "https://gofile.io/d/Sf8Wq1",
      folderId: "f3", fileId: "a3", compressed: true, downloads: 9, downloadedAt: now - 0.6 * DAY, canDelete: true,
      deleteAt: null, secs: 64, peak: 12_800_000, note: "The v2 sources and the fonts.",
      contents: [{ name: "Source files/main.psd", size: 312_000_000 }, { name: "Source files/layers/sky.psd", size: 98_000_000 }, { name: "Fonts/Inter.ttf", size: 820_000 }, { name: "Readme.txt", size: 4_000 }],
      countHistory: [[now - 6 * DAY, 0], [now - 5.2 * DAY, 2], [now - 3.1 * DAY, 5], [now - 1.4 * DAY, 7], [now - 0.6 * DAY, 9]],
    },
  ],
  canDelete: true,
  keepNote: "Gofile keeps files while people download them. After 10 days without a download, they may be removed.",
  checking: false,
  checkedAt: now - 2 * MIN,
  countsNote: null,
};

// ---------- Bell and pop-ups ----------

export const showcaseNotices: Notice[] = [
  { id: "n2", kind: "direct-offer", title: "Files for you", body: "Nina wants to send you Recipes (420 MB).", at: now - 12 * MIN, read: false, go: { page: "direct", id: NINA }, answered: "Accepted" },
  { id: "n1", kind: "direct-request", title: "Someone wants to connect", body: "Leo wants to connect with you in Pako.", at: now - 26 * HOUR, read: true, go: { page: "direct", peer: LEO }, answered: "Accepted" },
];

export const showcaseCards: PopupCard[] = [
  { id: "p1", kind: "browser", title: "Downloading in Pako", body: "Wallpapers 4K.zip", go: { page: "downloads" }, stays: false },
  { id: "p2", kind: "download-done", title: "Download finished", body: "Course materials.zip", go: { page: "downloads", id: "s8" }, stays: false },
  { id: "p3", kind: "direct-offer", title: "Files for you", body: "Nina wants to send you Recipes (420 MB).", go: { page: "direct", offer: "o-nina" }, stays: true },
];

// ---------- Direct ----------

/** "?showcase=local": Alex is on the same home network (the blue dot, "Local network"). */
const LOCAL = scene === "local";
/** One file on its way to Alex: it moves on each time Direct asks (about 40 MB/s; on the home
 *  network about 95 MB/s). */
const SENDING = { size: 3.6e9, from: 1.52e9, rate: LOCAL ? 94.5e6 : 41.5e6, since: now };
/** Alex's way: straight over the internet, or over the home network. */
const ALEX_PATH = LOCAL ? "local" : "direct";

export function showcaseDirect(): DirectState {
  const t = Date.now();
  const into = (name: string) => `${HOME}\\From ${name}`;
  const contact = (id: string, name: string, over: Partial<NonNullable<DirectState["contacts"]>[number]>) => ({
    id, name, trusted: true, autoAccept: false, saveFolder: null, lastSeen: t, added: t - 40 * DAY, online: false, path: null, waiting: false,
    code: "4821 0937 5512", note: null, folder: into(name), tags: [], favourite: false, theirName: null, ...over,
  });
  return {
    ready: true,
    error: null,
    myId: "9c8b7a6f5e4d3c2b1a0f9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a2f1e0d9c8b",
    myName: "Sam",
    tags: ["Friends", "Family", "Work"],
    contacts: [
      contact(ALEX, "Alex", { online: true, path: ALEX_PATH, autoAccept: true, favourite: true, tags: ["Friends", "Work"], added: t - 64 * DAY, code: "4821 0937 5512 0386" }),
      // Checked often for an hour, picked a quarter of an hour ago.
      contact(JO, "Jo", { online: true, path: "relay", tags: ["Work"], added: t - 31 * DAY, code: "3301 8842 1176", check: "often", checkUntil: t + 45 * MIN, checkAs: "often" }),
      contact(NINA, "Nina", { lastSeen: t - 8 * MIN, tags: ["Family"], added: t - 120 * DAY, code: "1170 6640 2093" }),
      contact(LEO, "Leo", { lastSeen: t - 22 * HOUR, tags: ["Friends"], added: t - 26 * HOUR, code: "6658 2210 9034", muted: true }),
      contact(MAYA, "Maya", { lastSeen: t - 45 * DAY, lastActivity: t - 45 * DAY, added: t - 200 * DAY, code: "5402 7719 3368", unchecked: true }),
    ],
    requests: [],
    offers: [],
    transfers: [
      { id: "x1", contact: ALEX, outgoing: true, name: "Short film final.mov", files: 1, size: SENDING.size, done: Math.min(SENDING.size * 0.97, SENDING.from + ((t - SENDING.since) / 1000) * SENDING.rate), state: "sending", path: ALEX_PATH, message: null },
      // Cut short when Nina's laptop went to sleep: it continues once she's back.
      { id: "x2", contact: NINA, outgoing: false, name: "Holiday videos", files: 14, size: 2.8e9, done: 1.74e9, state: "waiting", path: null, message: null },
    ],
    history: [
      { id: "h1", contact: ALEX, outgoing: false, name: "Photos from Saturday", files: 48, size: 412e6, date: now - 3 * DAY, path: ALEX_PATH, result: "done", message: null, savedTo: into("Alex") + "\\Photos from Saturday", durationMs: 9000, avgSpeed: 45e6 },
      { id: "h2", contact: ALEX, outgoing: true, name: "Slides.pdf", files: 1, size: 6.2e6, date: now - 2 * DAY, path: ALEX_PATH, result: "done", message: null, savedTo: null, durationMs: 300, avgSpeed: 20e6 },
      { id: "h3", contact: ALEX, outgoing: false, name: "Music for the trailer.zip", files: 1, size: 186e6, date: now - 20 * HOUR, path: ALEX_PATH, result: "done", message: null, savedTo: into("Alex") + "\\Music for the trailer.zip", durationMs: 4600, avgSpeed: 40e6 },
      { id: "h4", contact: ALEX, outgoing: true, name: "Colour notes.pdf", files: 1, size: 2.4e6, date: now - 25 * MIN, path: ALEX_PATH, result: "done", message: null, savedTo: null, durationMs: 200, avgSpeed: 12e6 },
      { id: "h5", contact: JO, outgoing: true, name: "Report Q3.xlsx", files: 1, size: 1.4e6, date: now - DAY, path: "relay", result: "done", message: null, savedTo: null, durationMs: 900, avgSpeed: 1.5e6 },
      { id: "h6", contact: NINA, outgoing: false, name: "Recipes", files: 36, size: 420e6, date: now - 10 * MIN, path: "direct", result: "done", message: null, savedTo: into("Nina") + "\\Recipes", durationMs: 14000, avgSpeed: 30e6 },
    ],
    // Sent from this PC to Rado through Home PC: there already, on its way to Rado.
    relays: [
      {
        id: "rl1", mine: true, device: HOME_PC, deviceName: "Home PC", to: RADO, toName: "Rado", name: "Wedding photos", files: 3, size: 860e6, date: now - 4 * MIN, ended: null,
        state: "sending", message: null, leg1Done: 860e6, leg1State: null, done: Math.min(860e6 * 0.92, 860e6 * 0.45 + ((t - now) / 1000) * 6e6), keep: false, reached: true,
      },
    ],
  };
}

// ---------- Direct › From: Home PC (sample devices that pass files on to their contacts) ----------

const LAPTOP_PC = "2f4a6c8e0b1d3f5a7c9e1b3d5f7a9c1e3b5d7f9a1c3e5b7d9f1a3c5e7b9d1f3a";
const OFFICE_PC = "7e5c3a1f9d7b5e3c1a9f7d5b3e1c9a7f5d3b1e9c7a5f3d1b9e7c5a3f1d9b7e5c";
const RADO = "3a5c7e9b1d3f5a7c9e1b3d5f7a9c1e3b5d7f9a1c3e5b7d9f1a3c5e7b9d1f3a5c";
const MIA = "4b6d8f0a2c4e6b8d0f2a4c6e8b0d2f4a6c8e0b2d4f6a8c0e2b4d6f8a0c2e4b6d";
const HERY = "6d8f0a2c4e6b8d0f2a4c6e8b0d2f4a6c8e0b2d4f6a8c0e2b4d6f8a0c2e4b6d8f";
const TINA = "8f0a2c4e6b8d0f2a4c6e8b0d2f4a6c8e0b2d4f6a8c0e2b4d6f8a0c2e4b6d8f0a";

/** The "From" menu: Home PC (it calls itself Living room PC), the laptop, and the office PC
 *  (offline, greyed). */
export function mockFromChoices(): DirectFrom[] {
  return [
    { id: HOME_PC, name: "Home PC", pcName: "Living room PC", directName: "Sam", contacts: 4, online: true },
    { id: LAPTOP_PC, name: "Laptop", pcName: null, directName: "Sam", contacts: 2, online: true },
    { id: OFFICE_PC, name: "Office PC", pcName: null, directName: "Sam Rakoto", contacts: 9, online: false },
  ];
}

/** Home PC's contacts, as this PC sees them. */
export function mockRemoteBook(device: string): RemoteBook {
  const t = Date.now();
  const c = (id: string, name: string, more: Partial<RemoteBook["contacts"][number]> = {}) => ({ id, name, online: false, lastSeen: t - 3 * HOUR, favourite: false, trusted: true, declined: false, ...more });
  if (device === LAPTOP_PC) return { me: { id: LAPTOP_PC, name: "Sam" }, contacts: [c(NINA, "Nina", { online: true, path: "direct" }), c(JO, "Jo")] };
  return {
    me: { id: device, name: "Sam" },
    contacts: [
      c(RADO, "Rado", { online: true, path: "direct", favourite: true }),
      c(MIA, "Mia", { online: true, path: "local" }),
      c(HERY, "Hery", { lastSeen: t - 2 * DAY }),
      c(TINA, "Tina", { lastSeen: t - 40 * MIN, theirName: "Tina R." }),
    ],
  };
}

/** Home PC's history with one of its contacts, read live: Rado's has a relayed card. */
export function mockRemoteTalk(device: string, contact: string): RemoteTalk {
  const h = (id: string, outgoing: boolean, name: string, files: number, size: number, ago: number, more: Partial<DirectHistory> = {}): DirectHistory => ({
    id, contact, outgoing, name, files, size, date: now - ago, path: "direct", result: "done", message: null, savedTo: null, ...more,
  });
  if (device !== HOME_PC) return { history: [], moving: [] };
  if (contact === RADO)
    return {
      history: [
        h("rh1", false, "Band rehearsal.mp3", 1, 48e6, 3 * DAY),
        h("rh2", true, "Flat keys.jpg", 1, 2.1e6, 2 * DAY),
        h("rh3", true, "Trip budget.xlsx", 1, 640e3, 26 * HOUR, { via: "Desk PC" }),
        h("rh4", false, "Concert tickets.pdf", 1, 380e3, 5 * HOUR),
      ],
      moving: [],
    };
  if (contact === MIA) return { history: [h("mh1", true, "Garden plans", 6, 24e6, DAY, { path: "local" })], moving: [] };
  return { history: [], moving: [] };
}
