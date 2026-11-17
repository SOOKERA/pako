// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// My devices: your own PCs, paired both ways. The state comes from Direct's node (devices are
// contacts marked "this is my device"); this file holds it for the page and the sidebar, and
// the design preview's sample devices.
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { showcase } from "./showcase";
import type { MenuItem } from "./menu";
import { inApp, listen, type DirectContact, type DirectHistory, type DirectRelay, type DirectRequest, type DirectTransfer, type Priority, type Status } from "./api";

export interface Device extends DirectContact {
  /** You said "this is my device". */
  device: boolean;
  /** Their Pako says this PC is one of its devices. */
  theirDevice: boolean;
  /** "This device can control my Pako" (you allow it). */
  control: boolean;
  /** They let this PC control their Pako. */
  theirControl: boolean;
  /** Both sides said "this is my device". */
  paired: boolean;
  /** One of your contacts says this PC is its device: confirm here (or not). */
  asksToPair: boolean;
  /** Its own name for its PC, as it said last (your name for it follows until you rename it). */
  theirPcName?: string | null;
}

/** One download in another device's list. */
export interface RemoteItem {
  id: string;
  name: string;
  size: number;
  done: number;
  speed: number;
  status: Status;
  message: string | null;
  /** "first" or "later" (left out for Normal, and by older Pakos). */
  priority?: "first" | "later" | null;
}

/** One link sent to a device, with what goes with it (only the link and these travel). */
export interface LinkOpts {
  link: string;
  /** A video's height (0 = audio only). */
  quality?: number | null;
  priority?: Priority | null;
  /** A subfolder of that PC's own download folder ("Playlist name"). */
  folder?: string | null;
}

/** One link of a send, and the downloads it became there (their ids in its list). */
export interface SentLink extends LinkOpts {
  name?: string | null;
  ids?: string[];
  failed?: boolean;
}

/** One send to a device: a card in My devices › Links. */
export interface SentCard {
  id: string;
  device: string;
  date: number;
  links: SentLink[];
  /** What that PC said ("Added 3 links to the downloads"). */
  text: string;
}

/** A PC a link can be set to download on (paired, letting this PC control it). */
export interface DeviceChoice {
  id: string;
  name: string;
  online: boolean;
}

/** Where picked Links rows go when Download is pressed. */
export interface DeviceSplit {
  here: string[];
  /** One order per PC: [its ID, its name, the rows]. */
  send: [string, string, string[]][];
  /** Rows that stay in Links: [row, why]. */
  stay: [string, string][];
}

/** One video or file of a playlist, a Drive folder or a link page, read on this PC. */
export interface PickItem {
  link: string;
  name: string;
  video: boolean;
  duration: number | null;
  size: number | null;
  /** The playlist's or folder's name: its subfolder there. */
  folder: string | null;
}

/** One line of a device's history: an order given or taken, a permission change. */
export interface DeviceLog {
  device: string;
  date: number;
  /** Done from this PC (else that device did it here). */
  byMe: boolean;
  text: string;
  ok: boolean;
}

export interface DevicesState {
  myId: string;
  /** Your name for Direct's people. */
  myName: string;
  /** This PC's name, as your devices see it ("" until set). */
  pcName: string;
  internet: boolean;
  devices: Device[];
  /** Someone new asks to pair as your device. */
  requests: DirectRequest[];
  transfers: DirectTransfer[];
  history: DirectHistory[];
  log: DeviceLog[];
  /** Links sent from this PC to your devices, a card per send (oldest first). */
  sent: SentCard[];
  /** Each device's download list (while you look at it and it lets you). */
  remote: Record<string, { items: RemoteItem[]; at: number }>;
  /** Direct relays through your devices, both ways (older Pakos: missing). */
  relays?: DirectRelay[];
}

function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inApp) return tauriInvoke<T>(cmd, args);
  return preview<T>(cmd, args);
}

export const devicesApi = {
  state: () => call<DevicesState>("devices_state"),
  defaultFolder: (name: string) => call<string>("devices_default_folder", { name }),
  add: (id: string, name: string | null, folder: string | null) => call<void>("devices_add", { id, name, folder }),
  answer: (peer: string, accept: boolean, folder: string | null) => call<void>("devices_answer", { peer, accept, folder }),
  setControl: (id: string, on: boolean) => call<void>("devices_set_control", { id, on }),
  setFolder: (id: string, folder: string) => call<void>("devices_set_folder", { id, folder }),
  rename: (id: string, name: string) => call<void>("devices_rename", { id, name }),
  watch: (id: string, on: boolean) => call<void>("devices_watch", { id, on }),
  clearLog: (id: string) => call<void>("devices_clear_log", { id }),
  /** My devices › Links › Send: files and single videos in the pasted text go at once, with
   *  the quality (videos), priority and subfolder picked; what's picked here first (a
   *  playlist, a Drive folder, a link page) comes back in `pick`. */
  download: (id: string, text: string, quality: number | null, priority: Priority | null, folder: string | null) =>
    call<{ text: string | null; pick: string[] }>("devices_download", { id, text, quality, priority, folder }),
  /** A playlist, a Drive folder or a link page read on this PC: its videos or files. */
  pick: (link: string) => call<{ title: string; items: PickItem[] }>("devices_pick", { link }),
  /** Clears My devices › Links for a device. */
  clearSent: (id: string) => call<void>("devices_clear_sent", { id }),
  /** Pause, resume, remove (off the list) or get one download of a device's list, or set its priority. */
  item: (id: string, item: string, action: "pause" | "resume" | "remove" | "get" | Priority) => call<string>("devices_item", { id, item, action }),
  /** Links › "Download on" (per link): devices that let this PC control them, online or not. */
  choices: () => call<DeviceChoice[]>("devices_choices"),
  /** Pressing Download in Links: where each picked row goes. */
  foundSplit: (ids: string[]) => call<DeviceSplit>("devices_found_split", { ids }),
  /** Names this PC for your devices (they see it at once). Returns the name kept. */
  setPcName: (name: string) => call<string>("devices_set_pc_name", { name }),
  /** "Download on ▾": your devices this PC may start downloads on, connected now. */
  targets: () => call<DownloadTarget[]>("devices_targets"),
  /** "Download on <device>" in Links: these links, as they are. Says what happened there, and
   *  the links it didn't take (their rows stay here). */
  addLinks: (id: string, links: LinkOpts[], names: Record<string, string> | null = null) =>
    call<{ text: string; failed: string[]; held: [string, string][] }>("devices_add_links", { id, links, names }),
  /** "Download on <device>" in the Download with Pako window (that download leaves this PC),
   *  with the quality chosen in the window (videos). */
  sendAsk: (id: string, device: string, quality: number | null) => call<string>("devices_send_ask", { id, device, quality }),
};

/** A device "Download on ▾" offers. */
export interface DownloadTarget {
  id: string;
  name: string;
}

/** The devices "Download on ▾" offers, kept up to date (empty: the menu isn't shown). Never
 *  remembered as a choice: picking one is for that one action, This PC stays the default. */
class Targets {
  list = $state<DownloadTarget[]>([]);
  private started = false;
  /** Only the latest answer counts (an older one arriving late would show a stale list). */
  private asked = 0;
  private load() {
    const n = ++this.asked;
    devicesApi.targets().then(
      (l) => n === this.asked && (this.list = l),
      () => n === this.asked && (this.list = []),
    );
  }
  start() {
    if (this.started) return;
    this.started = true;
    listen("direct-changed", () => this.load());
    listen("devices-changed", () => this.load());
    this.load();
  }
}
export const targets = new Targets();

/** Links › "Download on" (per link): the PCs a link can be set to, kept up to date. */
class Choices {
  list = $state<DeviceChoice[]>([]);
  private started = false;
  private asked = 0;
  private load() {
    const n = ++this.asked;
    devicesApi.choices().then(
      (l) => n === this.asked && (this.list = l),
      () => n === this.asked && (this.list = []),
    );
  }
  start() {
    if (this.started) return;
    this.started = true;
    listen("direct-changed", () => this.load());
    listen("devices-changed", () => this.load());
    this.load();
  }
}
export const choices = new Choices();

/** The menu: This PC first (the usual download), then each device. `long`: the items say
 *  "Download on …" (when the button itself is only an arrow). */
export function downloadOnMenu(targets: DownloadTarget[], here: (() => void) | null, pick: (t: DownloadTarget) => void, long = false): MenuItem[] {
  return [
    ...(here ? [{ label: long ? "Download on this PC" : "This PC", icon: "download", action: here }, { separator: true }] : []),
    ...targets.map((t) => ({ label: long ? `Download on ${t.name}` : t.name, icon: "devices", action: () => pick(t) })),
  ];
}


class Devices {
  data = $state<DevicesState | null>(null);
  /** The device shown on the right. */
  picked = $state<string | null>(null);
  /** The "Add a device" window. */
  adding = $state(false);
  /** Contacts saying this PC is theirs that you answered "Not now" (until Pako restarts). */
  notNow = $state<string[]>([]);
  /** Questions for you: new PCs asking to pair, and contacts that say this PC is theirs. */
  waiting = $derived((this.data?.requests.length ?? 0) + (this.data?.devices.filter((d) => d.asksToPair && !this.notNow.includes(d.id)).length ?? 0));
  /** Files moving between your devices right now. */
  moving = $derived((this.data?.transfers ?? []).filter((t) => t.state === "sending" || t.state === "receiving").length);
  private asking = false;
  private again = false;
  private started = false;

  async refresh() {
    if (this.asking) {
      this.again = true;
      return;
    }
    this.asking = true;
    try {
      this.data = await devicesApi.state();
    } catch {
      // Direct isn't running yet: the next change asks again.
    } finally {
      this.asking = false;
      if (this.again) {
        this.again = false;
        this.refresh();
      }
    }
  }

  /** Follows Direct's changes (devices ride on it) and the lists devices send. Once. */
  start() {
    if (this.started) return;
    this.started = true;
    listen("direct-changed", () => this.refresh());
    listen("devices-changed", () => this.refresh());
    this.refresh();
  }
}

export const devices = new Devices();

/** The sidebar's and the list's dot: grey offline, green online, blue on the same network. */
export const onLan = (d: Device) => d.online && d.path === "local";

// ---------- design preview: sample devices ----------

const HOME = "9c1d7a4e2b6f80135c7e9a2d4f6b8c0e1a3c5e7092b4d6f8a0c2e4f6a8b0c2d4";
const LAPTOP = "2f4a6c8e0b1d3f5a7c9e1b3d5f7a9c1e3b5d7f9a1c3e5b7d9f1a3c5e7b9d1f3a";
const OFFICE = "7e5c3a1f9d7b5e3c1a9f7d5b3e1c9a7f5d3b1e9c7a5f3d1b9e7c5a3f1d9b7e5c";
const now = Date.now();
/** The sample user's download folder ("?showcase": the website's pictures use Sam's). */
const DL = showcase ? "C:\\Users\\Sam\\Downloads" : "C:\\Users\\me\\Downloads";
const contact = (id: string, name: string, more: Partial<Device>): Device => ({
  id,
  name,
  trusted: true,
  autoAccept: false,
  saveFolder: `${DL}\\From ${name}`,
  lastSeen: now - 60_000,
  added: now - 86400000 * 20,
  online: true,
  path: "local",
  waiting: false,
  code: "3907 1146 8820 5531 0274",
  note: null,
  folder: `${DL}\\From ${name}`,
  tags: [],
  favourite: false,
  theirName: name,
  declined: false,
  device: true,
  theirDevice: true,
  control: true,
  theirControl: true,
  paired: true,
  asksToPair: false,
  ...more,
});
let mockDone = 0.42e9;
let mockLog: DeviceLog[] = [
  { device: HOME, date: now - 3600_000 * 5, byMe: true, text: "Paired as your device", ok: true },
  { device: HOME, date: now - 3600_000 * 2, byMe: true, text: "Added 1 link to the downloads", ok: true },
  { device: HOME, date: now - 3600_000, byMe: true, text: "Paused Holiday video 4K.mp4", ok: true },
  { device: LAPTOP, date: now - 86400000, byMe: false, text: "Added 2 links to the downloads", ok: true },
];
let mockControl: Record<string, boolean> = {};
/** The sample sends (My devices › Links): one finished, followed live. */
let mockSent: SentCard[] = [
  {
    id: "s1",
    device: HOME,
    date: now - 3600_000 * 2,
    text: "Added 3 links to the downloads",
    links: [
      { link: "https://files.example.com/Family%20recipes.pdf", name: "Family recipes.pdf", ids: ["r4"] },
      { link: "https://releases.example.com/ubuntu-26.04-desktop-amd64.iso", name: showcase ? "Photo archive 2025.zip" : "Ubuntu 26.04 desktop amd64.iso", ids: ["r1"] },
      { link: "https://learn.example.com/course/week3.zip", name: "Course - week 3.zip", ids: ["r3"] },
    ],
  },
];
/** "?pcname=none": this PC has no name yet. */
let mockPcName = typeof location !== "undefined" && new URLSearchParams(location.search).get("pcname") === "none" ? "" : "Desk PC";

function mockState(): DevicesState {
  const q = typeof location !== "undefined" ? new URLSearchParams(location.search) : new URLSearchParams();
  if (q.get("devices") === "none") return { myId: HOME, myName: "Sam", pcName: mockPcName, internet: true, devices: [], requests: [], transfers: [], history: [], log: [], sent: [], remote: {} };
  // "?showcase": the file on its way goes to Home PC over the home network (faster).
  mockDone = Math.min(2.4e9, mockDone + (showcase ? 46e6 : 37e6));
  const list = [
    contact(HOME, "Home PC", { control: mockControl[HOME] ?? true }),
    contact(LAPTOP, "Laptop", { path: "direct", code: "6612 0093 4471 2280 9135", control: mockControl[LAPTOP] ?? true }),
    contact(OFFICE, "Office PC", { online: false, path: null, lastSeen: now - 3600_000 * 3, code: "1188 4520 7763 0912 3346", control: mockControl[OFFICE] ?? false }),
  ];
  return {
    myId: "5b3d1f9e7c5a3b1d9f7e5c3a1b9d7f5e3c1a9b7d5f3e1c9a7b5d3f1e9c7a5b3d",
    myName: "Sam",
    pcName: mockPcName,
    internet: true,
    devices: list,
    requests: q.has("pair") ? [{ peer: "0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f9", name: "Living room PC", code: "5520 1187 3346 9021 7745", knownAs: null }] : [],
    transfers: [{ id: "m1", contact: showcase ? HOME : LAPTOP, outgoing: true, name: "Project backup", files: 312, size: 2.4e9, done: mockDone, state: "sending", path: showcase ? "local" : "direct", message: null }],
    history: [
      { id: "h1", contact: HOME, outgoing: true, name: "Tax papers 2026.pdf", files: 1, size: 3.4e6, date: now - 86400000 * 2, path: "local", result: "done", message: null, savedTo: null },
      { id: "h2", contact: HOME, outgoing: false, name: "Holiday photos", files: 148, size: 1.9e9, date: now - 3600_000 * 6, path: "local", result: "done", message: null, savedTo: `${DL}\\From Home PC\\Holiday photos` },
      { id: "h3", contact: HOME, outgoing: false, name: "Movie night.mkv", files: 1, size: 4.1e9, date: now - 3600_000, path: "local", result: "done", message: null, savedTo: `${DL}\\From Home PC\\Movie night.mkv` },
      { id: "h4", contact: LAPTOP, outgoing: false, name: "Slides final.pptx", files: 1, size: 18e6, date: now - 86400000, path: "direct", result: "done", message: null, savedTo: `${DL}\\From Laptop\\Slides final.pptx` },
    ],
    log: mockLog,
    sent: mockSent,
    remote: {
      [HOME]: {
        at: Date.now(),
        items: [
          { id: "r1", name: showcase ? "Photo archive 2025.zip" : "Ubuntu 26.04 desktop amd64.iso", size: 6.1e9, done: Math.min(6.1e9, 2.2e9 + mockDone), speed: 38e6, status: "downloading", message: null },
          { id: "r2", name: "Holiday video 4K.mp4", size: 2.8e9, done: 1.1e9, speed: 0, status: "paused", message: null, priority: "later" },
          { id: "r3", name: "Course - week 3.zip", size: 740e6, done: 0, speed: 0, status: "queued", message: null, priority: "first" },
          { id: "r4", name: "Family recipes.pdf", size: 12e6, done: 12e6, speed: 0, status: "completed", message: null },
          // The website's pictures show everything going well.
          ...(showcase ? [] : [{ id: "r5", name: "Old drivers.exe", size: 0, done: 0, speed: 0, status: "failed" as Status, message: "The server says this file doesn't exist any more." }]),
        ],
      },
      [LAPTOP]: { at: Date.now(), items: [{ id: "l1", name: "Lecture recording.mp4", size: 980e6, done: 980e6, speed: 0, status: "completed", message: null }] },
    },
    // Sent from Direct to Rado through Home PC (the same as Direct's sample).
    relays: [
      { id: "rl1", mine: true, device: HOME, deviceName: "Home PC", to: "rado", toName: "Rado", name: "Wedding photos", files: 3, size: 860e6, date: now - 4 * 60_000, ended: null, state: "sending", message: null, leg1Done: 860e6, leg1State: null, done: 400e6, keep: false, reached: true },
    ],
  };
}

function preview<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const id = String(args?.id ?? "");
  if (cmd === "devices_state") return Promise.resolve(mockState() as T);
  if (cmd === "devices_default_folder") return Promise.resolve(`${DL}\\From ${String(args?.name ?? "").trim() || "my device"}` as T);
  if (cmd === "devices_set_control") {
    mockControl = { ...mockControl, [id]: !!args?.on };
    mockLog = [...mockLog, { device: id, date: Date.now(), byMe: true, text: args?.on ? "Allowed to control this Pako" : "No longer allowed to control this Pako", ok: true }];
    return Promise.resolve(undefined as T);
  }
  if (cmd === "devices_set_pc_name") {
    const n = String(args?.name ?? "").trim().slice(0, 60);
    if (!n) return Promise.reject("Give this PC a name (like Desk PC).");
    mockPcName = n;
    return Promise.resolve(n as T);
  }
  // The online devices that let this PC control them (Office PC is offline).
  if (cmd === "devices_targets") return Promise.resolve([{ id: HOME, name: "Home PC" }, { id: LAPTOP, name: "Laptop" }] as T);
  if (cmd === "devices_choices")
    return Promise.resolve([
      { id: HOME, name: "Home PC", online: true },
      { id: LAPTOP, name: "Laptop", online: true },
      { id: OFFICE, name: "Office PC", online: false },
    ] as T);
  if (cmd === "devices_found_split") return Promise.resolve({ here: (args?.ids as string[]) ?? [], send: [], stay: [] } as T);
  if (cmd === "devices_clear_sent") {
    mockSent = mockSent.filter((c) => c.device !== id);
    return Promise.resolve(undefined as T);
  }
  // A playlist read "on this PC" (a moment, like yt-dlp).
  if (cmd === "devices_pick")
    return new Promise((ok) =>
      setTimeout(
        () =>
          ok({
            title: "Piano lessons for beginners",
            items: [
              ["Lesson 1 - Sitting and hand position", 612],
              ["Lesson 2 - The C major scale", 734],
              ["Lesson 3 - Reading notes", 905],
              ["Lesson 4 - First chords", 822],
              ["Lesson 5 - Both hands together", 1011],
            ].map(([name, duration], i) => ({ link: `https://www.youtube.com/watch?v=lesson${i + 1}`, name, video: true, duration, size: null, folder: "Piano lessons for beginners" })),
          } as T),
        600,
      ),
    );
  if (cmd === "devices_download" || cmd === "devices_add_links" || cmd === "devices_send_ask") return Promise.reject("Preview mode: nothing is sent to the other PC.");
  if (cmd === "devices_item") return Promise.reject("Preview mode: nothing is sent to the other PC.");
  if (cmd === "devices_add" || cmd === "devices_answer") return Promise.reject("Preview mode: nothing is paired.");
  return Promise.resolve(undefined as T);
}

// ---------- sending files to a device (no Accept on the other side) ----------

/** Where files dropped from Explorer go: their paths come back a moment later ("files-dropped"). */
let dropTarget: Device | null = null;
let onSent: ((ok: boolean, message: string) => void) | null = null;

/** Sends files and folders to one of your devices at once (no basket: it's your own PC). */
export async function sendToDevice(d: Device, paths: string[]): Promise<void> {
  if (!paths.length) return;
  if (!d.online) throw `${d.name} is offline: files go only while both PCs are on.`;
  if (!inApp) throw "Preview mode: nothing is sent.";
  await tauriInvoke<string>("direct_send", { contact: d.id, paths, compress: false, name: null });
}

/** Files dropped on a device (its row or its page): read by the window, then sent to it. */
export async function dropOnDevice(e: DragEvent, d: Device, done: (ok: boolean, message: string) => void) {
  e.preventDefault();
  e.stopPropagation();
  const dt = e.dataTransfer;
  const webview = (window as { chrome?: { webview?: { postMessageWithAdditionalObjects?: (m: string, o: unknown[]) => void } } }).chrome?.webview;
  if (!dt || !webview?.postMessageWithAdditionalObjects) return done(false, "Dropping files doesn't work here. Use Choose files.");
  const files = Array.from(dt.files);
  type WithHandle = DataTransferItem & { getAsFileSystemHandle?: () => Promise<unknown> };
  const asked = Array.from(dt.items)
    .filter((i) => i.kind === "file")
    .map((i) => (i as WithHandle).getAsFileSystemHandle?.() ?? Promise.reject());
  const got = await Promise.allSettled(asked);
  const handles = got.flatMap((r) => (r.status === "fulfilled" && r.value ? [r.value] : []));
  dropTarget = d;
  onSent = done;
  // The paths come back at once. If they never do, this drop is forgotten soon: files dropped
  // later on another page (Direct, Upload) must never go to this device.
  dropAt = Date.now();
  try {
    webview.postMessageWithAdditionalObjects("pako-files", [...files, ...handles]);
  } catch {
    try {
      webview.postMessageWithAdditionalObjects("pako-files", files);
    } catch (err) {
      dropTarget = null;
      onSent = null;
      done(false, `Pako couldn't read what was dropped (${String(err)}). Use Choose files.`);
    }
  }
}
let dropAt = 0;

if (typeof window !== "undefined") {
  listen<{ paths: string[]; detail: string }>("files-dropped", (e) => {
    const d = Date.now() - dropAt < 5000 ? dropTarget : null;
    const done = onSent;
    dropTarget = null;
    onSent = null;
    if (!d || !done) return; // dropped on another page (Direct, Upload): theirs
    if (!e.payload.paths.length) return done(false, `Pako couldn't read what was dropped (${e.payload.detail}).`);
    sendToDevice(d, e.payload.paths).then(
      () => done(true, ""),
      (err) => done(false, String(err)),
    );
  });
}
