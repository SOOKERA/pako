// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// "Download on": links on this PC downloaded by another of your PCs. Two ways:
// - "Download on ▾" (toolbar, details panel arrow, a row's menu, the Download with Pako window):
//   one action, for these links only, never remembered. Offered while one of your devices that
//   lets this PC control it is online.
// - The per-link "Download on" setting in the Links details panel: saved with the link; pressing
//   Download sends each link to its PC (`downloadSplit`).
// Each link goes with its quality (a video's: the one picked on its row, else this PC's
// default), its priority and, for a playlist's video or a Drive folder's file, that folder's
// name (a subfolder of that PC's own download folder). Never cookies, headers or a path here.
import { app } from "./state.svelte";
import { api, canDownload, type FoundItem } from "./api";
import { devicesApi, type DownloadTarget, type LinkOpts } from "./devices.svelte";
import { t } from "./i18n.svelte";
export { downloadOnMenu } from "./devices.svelte";

/** The link another PC downloads a Links row from: the link as it was pasted (a video site's
 *  page, a file's link). A file found in a Drive folder: its own Drive link. */
export function linkOf(f: FoundItem): string {
  const s = f.template.source;
  if (s.kind === "gDrive" && f.folder) {
    const key = s.resourceKey ? `?resourcekey=${encodeURIComponent(s.resourceKey)}` : "";
    const id = encodeURIComponent(s.fileId);
    // Docs, Sheets and Slides by their own link; anything else (a Drawing too, which Pako
    // reads only from its Drive link) by its Drive link.
    if (s.docType && ["document", "spreadsheets", "presentation"].includes(s.docType)) return `https://docs.google.com/${s.docType}/d/${id}/edit${key}`;
    return `https://drive.google.com/file/d/${id}/view${key}`;
  }
  return f.template.link;
}

/** What goes with a row's link: its video quality (audio only = 0; unset = that PC is told
 *  this PC's default), its priority (First when "Download first" was pressed) and the
 *  playlist's or Drive folder's name (and the first folder inside it) as a subfolder there. */
export function optsOf(f: FoundItem, first = false): LinkOpts {
  const s = f.template.source;
  const quality = s.kind === "video" ? (s.audio ? 0 : (s.quality ?? null)) : null;
  const priority = first ? "first" : f.template.priority;
  const sub = f.relDir ? f.relDir.split(/[\\/]/).filter(Boolean)[0] : "";
  const folder = f.folder ? (sub ? `${f.folder}/${sub}` : f.folder) : null;
  return { link: linkOf(f), quality, priority: priority === "normal" ? null : priority, folder };
}

class Sending {
  busy = $state(false);
}
export const sending = new Sending();

/** Sends these Links rows to a device (a file on several hosts: the host it would use here).
 *  The ones it took leave Links, like a download here; a link it didn't take, or one that is
 *  opened here first (a playlist not opened yet, a link page), keeps its row with a note.
 *  Says what happened in a toast. */
export async function downloadFoundOn(target: DownloadTarget, items: FoundItem[], o: { first?: boolean } = {}) {
  if (sending.busy) return;
  const rows = app
    .oncePerFile(items.filter(canDownload))
    .map((f) => app.mirrorOf.get(f.id)?.chosen ?? f)
    .filter(canDownload);
  if (!rows.length) return;
  const opts = new Map<string, LinkOpts>();
  const names: Record<string, string> = {};
  for (const f of rows) {
    const x = optsOf(f, o.first);
    if (!opts.has(x.link)) opts.set(x.link, x);
    names[x.link] = f.template.name;
  }
  sending.busy = true;
  try {
    const { text, failed, held } = await devicesApi.addLinks(target.id, [...opts.values()], names);
    const kept = new Set(failed);
    const heldWhy = new Map(held);
    const taken = rows.filter((f) => !kept.has(linkOf(f).trim()) && !heldWhy.has(linkOf(f).trim()));
    if (taken.length) await api.foundRemove(app.withMirrors(taken).map((f) => f.id)).catch(() => {});
    const refused = rows.filter((f) => kept.has(linkOf(f).trim()));
    if (refused.length) await api.foundDeviceNote(refused.map((f) => f.id), t("{name} didn't take it: {why}", { name: target.name, why: text })).catch(() => {});
    for (const [link, why] of held) {
      const ids = rows.filter((f) => linkOf(f).trim() === link).map((f) => f.id);
      if (ids.length) await api.foundDeviceNote(ids, why).catch(() => {});
    }
    await app.refresh();
    const stay = failed.length + held.length;
    if (stay) app.toast(t("{name}: {why}. {n} links stay here (see its row).", { name: target.name, why: text, n: stay }), "error");
    else app.toast(t("{what} on {name}.", { what: text, name: target.name }));
  } catch (e) {
    // Not sent at all (offline, refused): every row stays, saying why.
    const why = t("Not sent to {name}: {why}", { name: target.name, why: String(e) });
    await api.foundDeviceNote(rows.map((f) => f.id), why).catch(() => {});
    await app.refresh();
    app.toast(why, "error");
  } finally {
    sending.busy = false;
  }
}

/** Pressing Download (Download first, and notify) in Links: rows set to This PC download here
 *  (`here`, the usual Download), the others go to the PC each one is set to, one send per PC.
 *  Rows that can't go now (that PC offline or no longer allowed, a playlist not opened yet)
 *  stay in Links with a note. */
export async function downloadSplit(items: FoundItem[], first: boolean, here: (rows: FoundItem[]) => Promise<void>) {
  const able = items.filter(canDownload);
  if (!able.some((f) => f.device)) return here(able);
  let split;
  try {
    split = await devicesApi.foundSplit(app.withMirrors(able).map((f) => f.id));
  } catch (e) {
    app.toast(String(e), "error");
    return;
  }
  const byId = new Map(app.found.map((f) => [f.id, f] as const));
  const pick = (ids: string[]) => ids.map((id) => byId.get(id)).filter((f): f is FoundItem => !!f);
  const local = pick(split.here);
  if (local.length) await here(local);
  for (const [id, name, rows] of split.send) await downloadFoundOn({ id, name }, pick(rows), { first });
  if (split.stay.length) {
    await app.refresh();
    const n = split.stay.length;
    app.toast(t("{n} links stay in Links: {why}", { n, why: split.stay[0][1] }), "error");
  }
}
