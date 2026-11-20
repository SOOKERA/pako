// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Private mode in the design preview (a normal browser): a pretend vault, PIN 1234.
// "?private=on" starts with Private mode on and locked, "?private=open" open.
import type { Download } from "./api";

const q = typeof location !== "undefined" ? new URLSearchParams(location.search) : new URLSearchParams();
const start = q.get("private");

const day = 86_400_000;
function row(id: string, name: string, size: number, ago: number, status: Download["status"] = "completed", downloaded = size): Download {
  const date = Date.now() - ago;
  return {
    id,
    link: "https://example.com/" + name,
    source: { type: "http", url: "https://example.com/" + name },
    name,
    destDir: "",
    size,
    md5: null,
    status,
    message: null,
    resumable: true,
    segments: [],
    downloaded,
    addedAt: date,
    date,
    finishedAt: status === "completed" ? Math.floor(date / 1000) + 60 : null,
    group: null,
    resolved: true,
    priority: "normal",
    notify: false,
    extract: false,
    validator: null,
    private: true,
    vault: true,
  } as unknown as Download;
}

/** A file of a private folder download (`folder`: its place in that folder, folder first). */
function inFolder(id: string, name: string, size: number, ago: number, folder: string): Download {
  const d = row(id, name, size, ago);
  d.group = folder.split("\\")[0];
  d.vaultFolder = folder;
  return d;
}

const state = {
  on: start === "on" || start === "open",
  unlocked: start === "open",
  lock: "pin" as "pin" | "password",
  hasRecovery: start === "on" || start === "open",
  fails: 0,
  list: [
    row("p1", "Lake at sunrise.mp4", 812_000_000, 2 * 3600_000),
    row("p2", "Tax return 2026.pdf", 1_240_000, day),
    row("p3", "Family photos.zip", 3_400_000_000, 3 * day),
    row("p4", "Course notes.docx", 88_000, 5 * day),
    row("p5", "Concert recording.mkv", 2_100_000_000, 40 * 60_000, "downloading", 960_000_000),
    inFolder("p6", "IMG_0412.jpg", 4_200_000, 26 * 3600_000, "Weekend trip"),
    inFolder("p7", "IMG_0413.jpg", 3_900_000, 26 * 3600_000, "Weekend trip"),
    inFolder("p8", "Route.gpx", 120_000, 26 * 3600_000, "Weekend trip\\Maps"),
  ],
};

function status() {
  return { on: state.on, unlocked: state.unlocked, lock: state.lock, hasRecovery: state.hasRecovery, wait: 0 };
}

const CODE = "K7QM-3FXA-9TWP-HD4R-N8VC-2JEB-6YKS-QZ5G";

export function privateMock(cmd: string, args?: Record<string, unknown>): Promise<unknown> | undefined {
  const slow = <T,>(v: T, ms = 450) => new Promise<T>((ok) => setTimeout(() => ok(v), ms));
  switch (cmd) {
    case "private_status":
      return Promise.resolve(status());
    case "private_setup":
      state.on = true;
      state.unlocked = true;
      state.lock = (args?.kind as "pin" | "password") ?? "pin";
      return slow(undefined);
    case "private_unlock":
      if (String(args?.secret) === "1234" || String(args?.secret) === "my password") {
        state.unlocked = true;
        state.fails = 0;
        return slow(undefined);
      }
      state.fails++;
      return slow(null).then(() => Promise.reject({ wrong: true, wait: state.fails > 3 ? 5 : 0, message: state.fails > 3 ? "That's not it. Wait 5 seconds before trying again." : "That's not it. Try again." }));
    case "private_unlock_code":
      if (String(args?.code).toUpperCase().replace(/[\s-]/g, "") === CODE.replace(/-/g, "")) {
        state.unlocked = true;
        return slow(undefined);
      }
      return slow(null).then(() => Promise.reject({ wrong: false, wait: 0, message: "That code has a typo: check each group." }));
    case "private_lock":
      state.unlocked = false;
      return Promise.resolve();
    case "private_change":
      state.lock = (args?.kind as "pin" | "password") ?? state.lock;
      return slow(undefined);
    case "private_new_code":
      // Locked (from Settings): the PIN (1234) makes it.
      if (!state.unlocked && args?.current !== "1234") return slow(null).then(() => Promise.reject("That PIN or password isn't right."));
      return Promise.resolve(CODE);
    case "private_drop_code":
      return Promise.resolve(undefined);
    case "private_confirm_code":
      if (String(args?.last).toUpperCase() !== CODE.slice(-4)) return Promise.reject("That's not the last group of your code.");
      state.hasRecovery = true;
      return slow(undefined);
    case "private_reset":
      state.on = false;
      state.unlocked = false;
      state.list = [];
      return Promise.resolve();
    case "private_downloads":
      return Promise.resolve(state.unlocked ? structuredClone(state.list) : []);
    case "private_remove":
      state.list = state.list.filter((d) => !((args?.ids as string[]) ?? []).includes(d.id));
      return Promise.resolve();
    case "private_open":
      return Promise.reject("Preview mode: nothing is opened.");
    case "private_decrypt": {
      // Pretends to decrypt (the private copies stay or go as picked), so the dialog can be tried.
      const id = String(args?.id);
      const d = state.list.find((x) => x.id === id);
      if (args?.then !== "leave") state.list = state.list.filter((x) => x.id !== id);
      // Slow enough to see its progress on the row, and to try leaving meanwhile.
      return slow(`${String(args?.dir)}\\${d?.name ?? "file"}`, 4000);
    }
    case "private_hand_over":
      // Pretend hand-over paths (nothing is read); Direct then "sends" in the preview (api.ts).
      return Promise.resolve(((args?.ids as string[]) ?? []).map((id) => `C:\\preview\\private\\${id}`));
    case "private_move_in":
      return Promise.reject("Preview mode: nothing is moved.");
    case "private_play":
      return Promise.reject("Preview mode: nothing plays.");
    case "private_save_code":
      return Promise.reject("Preview mode: nothing is saved.");
    case "private_copy_code":
    case "private_tip_seen":
    case "private_keep_changes":
    case "private_playing":
      return Promise.resolve();
  }
  return undefined;
}
