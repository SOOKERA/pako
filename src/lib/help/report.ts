// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Help › Report a problem: the report's text, exactly as it is copied, saved or put in the email.
// In the language Pako shows: the person sending it reads it in their email app too.
import type { Problem, ProblemArea, ReportInfo, SendFail, SentProblem } from "../api";
import { t, type Key } from "../i18n.svelte";

export const SUPPORT_EMAIL = "support@sookera.com";
/** How many of the newest problems a report carries. */
export const REPORT_PROBLEMS = 20;
/** Longer mailto: links are cut off by some email apps (and Windows). */
const MAILTO_MAX = 2000;
const onClipboard = () => t("Details are on your clipboard: paste them here.");

const AREA_LABEL: Record<ProblemArea, Key> = {
  download: "Download (problem area)",
  video: "Video (problem area)",
  linkCheck: "Link check (problem area)",
  upload: "Upload (problem area)",
  direct: "Direct (problem area)",
  update: "Update (problem area)",
  extension: "Browser extension (problem area)",
  other: "Other (problem area)",
};
/** What a problem was about: "Download", "Upload"… */
export const areaLabel = (area: ProblemArea): string => t(AREA_LABEL[area] ?? AREA_LABEL.other);

const two = (n: number) => String(n).padStart(2, "0");

/** "2026-11-16 14:02", local time. */
export function when(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())} ${two(d.getHours())}:${two(d.getMinutes())}`;
}

/** The PC's time zone right now, "UTC+03:00": the times in a report are the PC's own. */
export function zone(d = new Date()): string {
  const m = -d.getTimezoneOffset();
  const a = Math.abs(m);
  return `UTC${m < 0 ? "-" : "+"}${two(Math.floor(a / 60))}:${two(a % 60)}`;
}

/** "2026-11-16T16:21:00+03:00": the PC's own time, with its offset then. */
export function isoTime(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${two(d.getMonth() + 1)}-${two(d.getDate())}T${two(d.getHours())}:${two(d.getMinutes())}:${two(d.getSeconds())}${zone(d).slice(3)}`;
}

/** The problems a report carries: the newest ones, newest first. */
export function attached(info: ReportInfo): Problem[] {
  return info.problems.slice(-REPORT_PROBLEMS).reverse();
}

export interface ReportParts {
  info: ReportInfo;
  /** The theme: "Dark", or "Light (Match Windows)". */
  look: string;
  include: boolean;
}

/** The message as the report shows it: another version's problem says which. */
export function problemLine(p: Problem, version: string): string {
  return `${p.message}${p.version && p.version !== version ? ` (${t("Pako {v}", { v: p.version })})` : ""}`;
}

/** "1.3.4 (Setup)" or "1.3.4 (Portable)". */
export const pakoLine = (info: ReportInfo): string => t(info.installed ? "{v} (Setup)" : "{v} (Portable)", { v: info.version });

/** Everything under the description: Pako, Windows, the look, the recent problems. */
export function details({ info, look, include }: ReportParts): string {
  const lines = [
    `Pako ${pakoLine(info)}`,
    info.windows,
    t("Theme: {look}", { look }),
    t("Time zone: {zone}", { zone: zone() }),
    "",
  ];
  const list = attached(info);
  if (!include) lines.push(t("Recent problems: not included."));
  else if (!list.length) lines.push(t("Recent problems: none noted."));
  else {
    lines.push(t("Recent problems (newest first, {n} of {total}):", { n: list.length, total: info.problems.length }));
    for (const p of list) lines.push(`${when(p.time)}  ${t("{area}: {message}", { area: areaLabel(p.area), message: problemLine(p, info.version) })}`);
  }
  return lines.join("\r\n");
}

/** The whole report: what happened, then the details. */
export function reportText(description: string, parts: ReportParts): string {
  return `${t("What happened:")}\r\n${description.trim()}\r\n\r\n${details(parts)}\r\n`;
}

/** The problems Send report sends: the ones shown, when they're included. */
export function sentProblems({ info, include }: ReportParts): SentProblem[] {
  if (!include) return [];
  return attached(info).map((p) => ({ time: isoTime(p.time), area: p.area, message: p.message, version: p.version }));
}

/** What to tell the user when Send report didn't go. */
export function sendFailText(e: unknown): string {
  const f = e as Partial<SendFail> | null;
  switch (f?.kind) {
    case "offline":
      return t("Couldn't reach Sookera. Check your connection and try again, or use Send by email.");
    case "busy": {
      const min = Math.max(1, Math.ceil((f as { retryAfter: number }).retryAfter / 60));
      return t("Too many reports from this connection. Try again in {n} minutes, or use Send by email.", { n: min });
    }
    case "refused":
      return (f as { message: string }).message;
    case "server":
      return t("Sookera's report service isn't answering right now. Try again later, or use Send by email.");
    default:
      return t("Couldn't send the report ({why}). Use Send by email instead.", { why: String(e) });
  }
}

export function subject(version: string): string {
  return t("Pako {v}: problem report", { v: version });
}

function mailto(subjectText: string, body: string): string {
  return `mailto:${SUPPORT_EMAIL}?subject=${encodeURIComponent(subjectText)}&body=${encodeURIComponent(body)}`;
}

/** The email to open. When the whole report doesn't fit in a mailto: link, `clipboard` is what
 *  goes on the clipboard first (the details, or everything if even the description is too long)
 *  and the email says to paste it. */
export function mailPlan(description: string, parts: ReportParts): { url: string; clipboard: string | null } {
  const subj = subject(parts.info.version);
  const full = mailto(subj, reportText(description, parts));
  if (full.length <= MAILTO_MAX) return { url: full, clipboard: null };
  const short = mailto(subj, `${t("What happened:")}\r\n${description.trim()}\r\n\r\n${onClipboard()}\r\n`);
  if (short.length <= MAILTO_MAX) return { url: short, clipboard: details(parts) };
  return { url: mailto(subj, `${onClipboard()}\r\n`), clipboard: reportText(description, parts) };
}
