// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
import type { Download, Status } from "./api";

const UNITS = ["B", "KB", "MB", "GB", "TB"];

export function bytes(n: number | null | undefined): string {
  if (n == null) return "?";
  let i = 0;
  let v = n;
  // 1023.96 KB would round to "1024 KB": it shows as "1.0 MB" instead.
  while (Math.round(v) >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  const digits = i === 0 ? 0 : v >= 99.95 ? 0 : 1;
  return `${v.toFixed(digits)} ${UNITS[i]}`;
}

/** "20.0 of 44.7 GB" — both numbers in the size's unit. */
export function bytesOf(done: number, total: number): string {
  let i = 0;
  let t = total;
  // The same step up as bytes(): 1023.96 KB reads "1.0 MB", never "1024 KB".
  while (Math.round(t) >= 1024 && i < UNITS.length - 1) {
    t /= 1024;
    i++;
  }
  const d = done / 1024 ** i;
  const digits = i === 0 ? 0 : t >= 99.95 ? 0 : 1;
  return `${d.toFixed(digits)} of ${t.toFixed(digits)} ${UNITS[i]}`;
}

export function speed(n: number): string {
  return `${bytes(n)}/s`;
}

export function timeLeft(remaining: number, bytesPerSec: number): string {
  if (bytesPerSec <= 0 || remaining <= 0) return "";
  const s = Math.round(remaining / bytesPerSec);
  if (s < 60) return `${s} s left`;
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min left`;
  const h = Math.floor(m / 60);
  const mm = m % 60;
  if (h < 48) return mm ? `${h} h ${mm} min left` : `${h} h left`;
  return `${Math.round(h / 24)} days left`;
}

export function extension(name: string): string {
  const i = name.lastIndexOf(".");
  if (i <= 0 || name.length - i > 8) return "";
  return name.slice(i + 1).toUpperCase();
}

/** Same idea as the engine's link finder: every http(s) link in any text, no duplicates. */
export function findLinks(text: string): string[] {
  const out: string[] = [];
  for (const m of text.matchAll(/https?:\/\/[^\s<>"'`[\]{}|\\^]+/gi)) {
    let link = m[0];
    for (;;) {
      let t = link.replace(/[.,;:!?*]+$/, "");
      if (t.endsWith(")") && !t.includes("(")) t = t.slice(0, -1);
      if (t === link) break;
      link = t;
    }
    if (link.length > 8 && !out.includes(link)) out.push(link);
  }
  return out;
}

// ---------- source ----------

export interface SourceInfo {
  label: string;
  color: string;
  key: string;
}

export function sourceOf(d: Download): SourceInfo {
  if (d.source.kind === "gDrive") return { label: "Google Drive", color: "var(--src-drive)", key: "drive" };
  if (d.source.kind === "video") {
    let h = "";
    try {
      h = new URL(d.link).hostname.replace(/^(www|m|music)\./, "");
    } catch {
      h = "";
    }
    const named: Record<string, string> = { "youtube.com": "YouTube", "youtu.be": "YouTube", "vimeo.com": "Vimeo", "dailymotion.com": "Dailymotion", "twitch.tv": "Twitch", "tiktok.com": "TikTok", "x.com": "X", "twitter.com": "X", "instagram.com": "Instagram", "facebook.com": "Facebook", "reddit.com": "Reddit" };
    return { label: named[h] ?? (h || "Video"), color: "var(--src-video)", key: "video" };
  }
  let host = "";
  try {
    host = new URL(d.link).hostname.replace(/^www\./, "");
  } catch {
    host = "web";
  }
  if (/(^|\.)mega\.(nz|io|co\.nz)$/.test(host)) return { label: "Mega", color: "var(--src-mega)", key: "mega" };
  return { label: host || "web", color: "var(--src-web)", key: "web" };
}

// ---------- file type ----------

const ICONS = {
  archive: "M4 7h16v13H4zM3 4h18v3H3zM10 11h4",
  video: "M4 5h16v14H4zM4 9h16M4 15h16M8 5v4M16 5v4M8 15v4M16 15v4",
  audio: "M9 18V5l11-2v13M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0zM20 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
  image: "M4 5h16v14H4zM4 16l5-5 4 4 3-3 4 4M15 9h.01",
  disc: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 14a2 2 0 1 0 0-4 2 2 0 0 0 0 4z",
  app: "M4 5h16v14H4zM4 9h16M7 7h.01M10 7h.01",
  cube: "M12 3l8 4.5v9L12 21l-8-4.5v-9zM12 12l8-4.5M12 12v9M12 12L4 7.5",
  doc: "M6 3h8l4 4v14H6zM14 3v4h4M9 12h6M9 16h6",
  code: "M6 3h8l4 4v14H6zM14 3v4h4M10 12l-2 2 2 2M14 12l2 2-2 2",
  file: "M6 3h8l4 4v14H6zM14 3v4h4",
  folder: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
};

const KIND: Record<string, keyof typeof ICONS> = {};
const assign = (kind: keyof typeof ICONS, exts: string) => exts.split(" ").forEach((e) => (KIND[e] = kind));
assign("archive", "ZIP 7Z RAR TAR GZ TGZ BZ2 XZ ZST CAB");
assign("video", "MP4 MKV MOV AVI WEBM WMV FLV M4V TS MPG MPEG");
assign("audio", "MP3 WAV FLAC AAC OGG M4A OPUS WMA");
assign("image", "JPG JPEG PNG GIF WEBP BMP TIF TIFF HEIC PSD EXR HDR SVG RAW CR2 NEF");
assign("disc", "ISO IMG DMG VHD VHDX");
assign("app", "EXE MSI APK APPX MSIX BAT CMD");
assign("cube", "UASSET UMAP FBX OBJ BLEND GLB GLTF USD USDZ MAX MA MB C4D STL ABC");
assign("doc", "PDF DOC DOCX XLS XLSX PPT PPTX TXT RTF ODT ODS CSV MD EPUB");
assign("code", "JSON XML HTML JS TS PY RS CPP H CS JAVA");

export type FileKind =
  | "archive" | "text" | "pdf" | "doc" | "sheet" | "slides" | "video" | "audio"
  | "image" | "disc" | "app" | "code" | "3d" | "file" | "folder";

const FILE_KIND: Record<string, FileKind> = {};
const kinds = (kind: FileKind, exts: string) => exts.split(" ").forEach((e) => (FILE_KIND[e] = kind));
kinds("archive", "ZIP 7Z RAR TAR GZ TGZ BZ2 XZ ZST CAB Z01 Z02 R00 R01 001 002 003");
kinds("text", "TXT MD LOG NFO INI CFG");
kinds("pdf", "PDF");
kinds("doc", "DOC DOCX ODT RTF PAGES EPUB");
kinds("sheet", "XLS XLSX ODS CSV NUMBERS");
kinds("slides", "PPT PPTX ODP KEY");
kinds("video", "MP4 MKV MOV AVI WEBM WMV FLV M4V TS MPG MPEG 3GP");
kinds("audio", "MP3 WAV FLAC AAC OGG M4A OPUS WMA");
kinds("image", "JPG JPEG PNG GIF WEBP BMP TIF TIFF HEIC PSD EXR HDR SVG RAW CR2 NEF");
kinds("disc", "ISO IMG DMG VHD VHDX");
kinds("app", "EXE MSI APK APPX MSIX BAT CMD");
kinds("code", "JSON XML HTML JS TS PY RS CPP H CS JAVA YAML YML");
kinds("3d", "UASSET UMAP FBX OBJ BLEND GLB GLTF USD USDZ MAX MA MB C4D STL ABC");

/** The kind of file, for its flat icon (split-archive parts count as archives). */
export function fileKind(name: string): FileKind {
  const ext = extension(name);
  if (FILE_KIND[ext]) return FILE_KIND[ext];
  if (/\.part\d+\.rar$/i.test(name) || /^Z\d{2,}$|^R\d{2,}$|^\d{3,}$/.test(ext)) return "archive";
  return "file";
}

/** Videos the details panel can preview. */
export function isVideo(name: string) {
  return /\.(mp4|m4v|webm|mkv|avi|mov|wmv|flv|ts|mpg|mpeg|3gp)$/i.test(name);
}

export function typeIcon(name: string, isFolder = false): string {
  if (isFolder) return ICONS.folder;
  return ICONS[KIND[extension(name)] ?? "file"];
}

// ---------- status ----------

export interface StatusLook {
  label: string;
  /** Color of the file-type icon and progress bar. */
  color: string;
  /** Color of the status text. */
  text: string;
  /** Faint row background for rows that need you. */
  tint: boolean;
}

export function statusLook(s: Status, message?: string | null): StatusLook {
  switch (s) {
    case "downloading":
      return { label: "Downloading", color: "var(--st-run)", text: "var(--text)", tint: false };
    case "starting":
      return { label: "Starting", color: "var(--st-run)", text: "var(--text-2)", tint: false };
    case "verifying":
      return { label: "Checking file", color: "var(--st-check)", text: "var(--st-check-text)", tint: false };
    case "retrying":
      return {
        label: message?.startsWith("No internet") ? "No internet" : "Reconnecting",
        color: "var(--st-retry)",
        text: "var(--st-retry-text)",
        tint: false,
      };
    case "queued": {
      // From the browser, waiting for the Download with Pako window's answer.
      if (message === "Waiting for you") return { label: "Waiting for you", color: "var(--st-wait)", text: "var(--st-wait-text)", tint: false };
      // Outside the download hours (Settings): "Scheduled · 22:00".
      const at = message?.startsWith("Waiting for the scheduled hours") ? message.match(/\((\d\d:\d\d)\)/)?.[1] : undefined;
      return { label: at ? `Scheduled · ${at}` : "Waiting its turn", color: "var(--st-wait)", text: "var(--st-wait-text)", tint: false };
    }
    case "paused":
      return { label: "Paused", color: "var(--st-pause)", text: "var(--text-3)", tint: false };
    case "completed":
      return { label: "Done", color: "var(--st-done)", text: "var(--st-done-text)", tint: false };
    case "failed":
      return { label: "Failed", color: "var(--st-fail)", text: "var(--st-fail-text)", tint: true };
    case "quotaExceeded":
      return { label: "Google limit", color: "var(--st-limit)", text: "var(--st-limit-text)", tint: true };
    case "needsAccount":
      return { label: "Only in the browser", color: "var(--st-limit)", text: "var(--st-limit-text)", tint: true };
  }
}

/** A download's look: its status, said the "Via browser" way when the browser downloads it for
 *  Pako (people expect Pako's multi-part speed, so the list says when the browser is the limit),
 *  and "Done · private" for a private window's finished download (it leaves the list soon). */
export function downloadLook(d: Pick<Download, "status" | "message" | "viaBrowser" | "private">): StatusLook {
  const look = statusLook(d.status, d.message);
  if (d.status === "completed" && d.private) return { ...look, label: "Done · private" };
  if (!d.viaBrowser) return look;
  if (d.message === "Waiting for your browser") return { ...look, label: "Waiting for your browser", text: "var(--text-3)" };
  if (d.status === "downloading") return { ...look, label: "Via browser" };
  if (d.status === "paused") return { ...look, label: "Paused · via browser" };
  return look;
}

/** A status colour used for words (a headline): its "-text" shade, so it stays readable on light
 *  panels (the plain shades are for dots and bars and are too pale as text there). */
export function wordColor(look: StatusLook): string {
  const m = look.color.match(/^var\(--st-(run|check|retry|wait|done|fail|limit)\)$/);
  if (m) return `var(--st-${m[1]}-text)`;
  return look.color === "var(--st-pause)" ? "var(--text-3)" : look.color;
}

/** Most urgent first: what a folder row shows. */
const URGENCY: Status[] = [
  "failed",
  "quotaExceeded",
  "needsAccount",
  "retrying",
  "verifying",
  "downloading",
  "starting",
  "queued",
  "paused",
  "completed",
];

export function mostUrgent(statuses: Status[]): Status {
  for (const s of URGENCY) if (statuses.includes(s)) return s;
  return "completed";
}

/** What a Sent item's downloads look like: "Not downloaded yet", "Downloaded 3 times", or
 *  "Gone from Gofile". Null when Pako doesn't know yet. */
export function sentLook(s: { downloads?: number | null; gone?: boolean }): { label: string; text: string; dot: string } | null {
  if (s.gone) return { label: "Gone from Gofile", text: "var(--st-fail-text)", dot: "var(--st-fail)" };
  if (s.downloads == null) return null;
  if (s.downloads === 0) return { label: "Not downloaded yet", text: "var(--text-3)", dot: "var(--st-pause)" };
  const label = s.downloads === 1 ? "Downloaded once" : `Downloaded ${s.downloads.toLocaleString()} times`;
  return { label, text: "var(--st-done-text)", dot: "var(--st-done)" };
}

// ---------- middle ellipsis ----------

let canvas: HTMLCanvasElement | null = null;
const widthCache = new Map<string, number>();

function measure(text: string, font: string): number {
  const key = font + "|" + text;
  const hit = widthCache.get(key);
  if (hit !== undefined) return hit;
  canvas ??= document.createElement("canvas");
  const ctx = canvas.getContext("2d")!;
  ctx.font = font;
  const w = ctx.measureText(text).width;
  if (widthCache.size > 5000) widthCache.clear();
  widthCache.set(key, w);
  return w;
}

/** Forget measured widths (fonts finished loading: widths measured with the stand-in font are wrong). */
export function clearWidths() {
  widthCache.clear();
}

/** The text's characters as people see them (an accent or an emoji is never cut in half). */
const segmenter = typeof Intl !== "undefined" && "Segmenter" in Intl ? new Intl.Segmenter(undefined, { granularity: "grapheme" }) : null;
export function graphemes(text: string): string[] {
  return segmenter ? Array.from(segmenter.segment(text), (s) => s.segment) : Array.from(text);
}

/** "Project render final v3.mp4" -> "Project render fin…v3.mp4": keeps the end (version, extension),
 *  cut between whole characters. */
export function middleEllipsis(text: string, maxWidth: number, font: string): string {
  if (maxWidth <= 0 || measure(text, font) <= maxWidth) return text;
  const g = graphemes(text);
  // The end kept (version, extension): shorter when even "…" and it don't fit, so the end is
  // never cut without its "…".
  let keepEnd = Math.min(12, Math.floor(g.length / 3));
  while (keepEnd > 1 && measure("…" + g.slice(g.length - keepEnd).join(""), font) > maxWidth) keepEnd--;
  const end = g.slice(g.length - keepEnd).join("");
  if (measure("…" + end, font) > maxWidth) return "…";
  let lo = 0;
  let hi = g.length - keepEnd;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (measure(g.slice(0, mid).join("") + "…" + end, font) <= maxWidth) lo = mid;
    else hi = mid - 1;
  }
  return g.slice(0, lo).join("").trimEnd() + "…" + end;
}

/** "Today 14:32", "Yesterday 09:10", "12 Nov 18:05", or "3 Mar 2025" for older years. */
export function shortDate(ms: number): string {
  if (!ms) return "—";
  const d = new Date(ms);
  const now = new Date();
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
  if (d.toDateString() === now.toDateString()) return `Today ${time}`;
  if (d.toDateString() === yesterday.toDateString()) return `Yesterday ${time}`;
  if (d.getFullYear() === now.getFullYear()) return `${d.toLocaleDateString([], { day: "numeric", month: "short" })} ${time}`;
  return d.toLocaleDateString([], { day: "numeric", month: "short", year: "numeric" });
}

/** Hover text for a date: "Added Sunday, 15 November 2026 at 14:32" (+ finished). */
export function dateTitle(added: number, finished: number | null = null): string {
  const full = (ms: number) => new Date(ms).toLocaleString([], { dateStyle: "full", timeStyle: "short" });
  return `Added ${full(added)}` + (finished ? `\nFinished ${full(finished)}` : "");
}

/** "22:00" from minutes after midnight, and back. */
export function hhmm(m: number): string {
  return `${String(Math.floor(m / 60) % 24).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;
}
export function minutesOf(t: string): number {
  const [h, m] = t.split(":").map((x) => parseInt(x, 10) || 0);
  return (h % 24) * 60 + (m % 60);
}

/** Inside the download hours right now (always true when the schedule is off). */
export function scheduleOpen(s: { scheduleOn: boolean; scheduleStart: number; scheduleEnd: number } | null | undefined, now = new Date()): boolean {
  if (!s?.scheduleOn || s.scheduleStart === s.scheduleEnd) return true;
  const m = now.getHours() * 60 + now.getMinutes();
  return s.scheduleStart < s.scheduleEnd ? m >= s.scheduleStart && m < s.scheduleEnd : m >= s.scheduleStart || m < s.scheduleEnd;
}

/** "1:02:05" / "4:31" for a video's length in seconds. */
export function duration(sec: number | null | undefined): string {
  if (!sec || sec <= 0) return "—";
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${r}` : `${m}:${r}`;
}

/** The quality a video will download in: the chosen one, else the closest lower, else the smallest. */
export function pickQuality(options: { height: number; size: number | null }[], want: number) {
  if (!want) return null;
  return options.find((o) => o.height <= want) ?? options[options.length - 1] ?? null;
}
