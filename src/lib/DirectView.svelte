<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Direct: a messaging app for files. The sidebar filters contacts (Show, Tags); here, the
  // contacts under that filter (searchable, scrolling, your name and ID at the bottom), and the
  // picked contact's conversation: both directions, history kept only on this PC, with a send
  // area at the bottom that sends straight to them while they're online.
  import { untrack } from "svelte";
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api, listen, type DirectContact, type DirectFrom, type DirectHistory, type DirectItem, type DirectRelay, type DirectTransfer, type DirectVisible, type RemoteBook, type RemoteContact, type RemoteTalk } from "./api";
  import type { MenuItem } from "./menu";
  import { bytes, shortDate, speed } from "./format";
  import Icon from "./Icon.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import Modal from "./Modal.svelte";
  import DirectDetails from "./DirectDetails.svelte";
  import TransferItems from "./TransferItems.svelte";
  import SegPick from "./SegPick.svelte";
  import { directExtra, hasList, initial, isLocal, itemsCount, nearbyNote, pathLabel, percent, sortContacts, statusOf, suggestedTags } from "./direct";
  import { around, num, t } from "./i18n.svelte";
  import { fadeIn, flipIf, flipOpts, pop, rowSlide } from "./motion";
  import PanelResizer from "./PanelResizer.svelte";
  import { panelWidths } from "./panelWidth.svelte";
  import { shownWidth } from "./panelSize";

  let { onmenu }: { onmenu: (x: number, y: number, items: MenuItem[]) => void } = $props();

  const d = $derived(app.directView);
  const extra = $derived(directExtra(d));
  /** The internet answers (without it, only contacts on the same network can be reached). */
  const internet = $derived(extra.internet !== false);
  // Direct starts the first time this page is open (until then Pako makes no Direct connection).
  let woken = false;
  $effect(() => {
    if (d && !d.ready && !d.error && !woken) {
      woken = true;
      api.directWake().catch(() => (woken = false));
    }
  });
  /** Everyone, for "do you have contacts at all". */
  const everyone = $derived(app.directContacts);

  // ---------- From: this PC, or another of your PCs (its contacts, sent to through it) ----------
  /** Your devices Direct can send from (paired, letting this PC control them, a Pako that can). */
  let fromChoices = $state<DirectFrom[]>([]);
  let fromLoaded = $state(false);
  let fromAsked = 0;
  function loadFrom() {
    const n = ++fromAsked;
    api.directFromChoices().then(
      (l) => n === fromAsked && ((fromChoices = l ?? []), (fromLoaded = true)),
      () => n === fromAsked && ((fromChoices = []), (fromLoaded = true)),
    );
  }
  $effect(() => {
    loadFrom();
    const a = listen("direct-changed", loadFrom);
    const b = listen("devices-changed", loadFrom);
    return () => {
      a.then((f) => f());
      b.then((f) => f());
    };
  });
  const from = $derived(app.directFrom ? (fromChoices.find((c) => c.id === app.directFrom) ?? null) : null);
  // A PC no longer offered (control turned off there, unpaired): back to this PC.
  $effect(() => {
    if (fromLoaded && app.directFrom && !from) app.directFrom = null;
  });
  /** That PC's Direct side, read live every few seconds while it's shown. */
  let book = $state<RemoteBook | null>(null);
  let bookError = $state("");
  const fromId = $derived(app.directFrom);
  $effect(() => {
    const dev = fromId;
    book = null;
    bookError = "";
    if (!dev) return;
    let alive = true;
    let busy = false;
    // One ask at a time, and none while the window is hidden (in the tray): each one reads that
    // PC's whole list over the network.
    const load = () => {
      if (busy || document.hidden) return;
      busy = true;
      api
        .directRemoteContacts(dev)
        .then(
          (b) => alive && ((book = b), (bookError = "")),
          (e) => alive && (bookError = String(e)),
        )
        .finally(() => (busy = false));
    };
    load();
    const timer = setInterval(load, 4000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  });
  /** One of that PC's contacts, shaped like this PC's (shown only: nothing to change here). */
  const asContact = (r: RemoteContact): DirectContact => ({
    id: r.id,
    name: r.name,
    trusted: r.trusted,
    autoAccept: false,
    saveFolder: null,
    lastSeen: r.lastSeen ?? null,
    added: 0,
    online: r.online,
    path: r.path ?? null,
    waiting: !r.trusted && !r.declined,
    code: "",
    note: null,
    folder: "",
    tags: [],
    favourite: r.favourite,
    theirName: r.theirName ?? null,
    declined: r.declined,
  });
  const remoteAll = $derived(book ? sortContacts(book.contacts.map(asContact)) : []);
  /** That PC's contacts under the filter on the left (tags and "waiting" are this PC's own). */
  const remoteShown = $derived(remoteAll.filter((c) => (app.directFilter === "online" ? c.online : app.directFilter === "favourites" ? c.favourite : true)));
  let remotePicked = $state<string | null>(null);
  $effect(() => {
    void fromId;
    remotePicked = null;
  });

  // Search inside the current filter (shown once the list gets long, or while typing).
  let search = $state("");
  const searching = $derived((fromId ? remoteAll.length : everyone.length) > 8 || search.length > 0);
  const contacts = $derived.by(() => {
    const q = (search.trim() || app.search.trim()).toLowerCase();
    const list = fromId ? remoteShown : app.directShown;
    if (!q) return list;
    return list.filter((c) => c.name.toLowerCase().includes(q) || (c.theirName ?? "").toLowerCase().includes(q) || (c.tags ?? []).some((tag) => tag.toLowerCase().includes(q)));
  });
  const picked = $derived(fromId ? remotePicked : app.directPicked);
  const current = $derived(contacts.find((c) => c.id === picked) ?? contacts[0] ?? null);
  const currentId = $derived(current?.id ?? null);
  function pickContact(id: string) {
    if (fromId) remotePicked = id;
    else app.directPicked = id;
  }
  // Nobody picked yet (or the picked one was removed): the first shown is picked for real, so a
  // contact coming online (online sorts first) doesn't swap the conversation under you.
  $effect(() => {
    if (fromId) {
      if (current && (!remotePicked || !remoteAll.some((c) => c.id === remotePicked))) remotePicked = current.id;
      return;
    }
    if (current && (!app.directPicked || !everyone.some((c) => c.id === app.directPicked))) app.directPicked = current.id;
  });
  /** That PC's history with the contact shown, read live (like My devices' Downloads tab). */
  let talk = $state<RemoteTalk | null>(null);
  $effect(() => {
    const dev = fromId;
    const who = currentId;
    talk = null;
    if (!dev || !who) return;
    let alive = true;
    let busy = false;
    const load = () => {
      if (busy || document.hidden) return;
      busy = true;
      api
        .directRemoteTalk(dev, who)
        .then((x) => alive && (talk = x), () => {})
        .finally(() => (busy = false));
    };
    load();
    const timer = setInterval(load, 2000);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  });
  /** Relays shown in a conversation: sent from this PC through `fromId` (still going, or ended
   *  before that PC had them), or passed on by this PC and not offered to them yet. */
  const relaysHere = $derived.by<DirectRelay[]>(() => {
    if (!current) return [];
    return (d?.relays ?? []).filter((r) =>
      fromId
        ? r.mine && r.device === fromId && r.to === current.id && (!r.ended || !r.reached)
        : !r.mine && !r.ended && r.to === current.id && ["receiving", "waiting", "offering"].includes(r.state),
    );
  });
  /** Your name or that PC's, and the ID shown with it. */
  const shownName = $derived(fromId ? book?.me.name || from?.directName || "" : (d?.myName ?? ""));
  const shownIdText = $derived(fromId ? (book?.me.id ?? "") : (d?.myId ?? ""));
  let fromMenu = $state(false);
  let fromSel = $state<HTMLDivElement | null>(null);
  /** Where the menu opens: under the button, as wide as it or wider (it may go past the list). */
  let menuAt = $state({ left: 0, top: 0, width: 300 });
  function toggleFromMenu() {
    if (!fromMenu && fromSel) {
      const r = fromSel.getBoundingClientRect();
      const width = Math.max(r.width, 340);
      menuAt = { left: Math.max(8, Math.min(r.left, window.innerWidth - width - 8)), top: r.bottom + 6, width };
    }
    fromMenu = !fromMenu;
  }
  function pickFrom(id: string | null) {
    fromMenu = false;
    if (id === app.directFrom) return;
    search = "";
    app.directFrom = id;
  }
  let renamingMe = $state(false);
  let meField = $state<HTMLInputElement | null>(null);
  function startRename() {
    if (fromId) return;
    renamingMe = true;
    queueMicrotask(() => {
      meField?.focus();
      meField?.select();
    });
  }
  async function endRename() {
    renamingMe = false;
    await saveName();
  }
  /** A relay on its way, in a few words: both legs ("To Home PC 100% · To Rado 45%"). */
  function relayText(r: DirectRelay) {
    const pc = r.deviceName || t("your other PC");
    const name = r.toName;
    if (!r.mine) {
      if (r.state === "receiving") return t("Arriving from {pc} {pct}% · then to {name}", { pc, pct: percent(r.leg1Done, r.size), name });
      return t("From {pc} · Waiting for {name} to be online", { pc, name });
    }
    const leg1 = t("To {pc} {pct}%", { pc, pct: r.state === "leg1" ? percent(r.leg1Done, r.size) : 100 });
    const pct = percent(r.done, r.size);
    switch (r.state) {
      case "leg1":
        return leg1;
      case "waiting":
        return t("{leg1} · Waiting for {name}", { leg1, name });
      case "offered":
        return t("{leg1} · Waiting for {name} to accept…", { leg1, name });
      case "sending":
        return t("{leg1} · To {name} {pct}%", { leg1, name, pct });
      case "paused":
        return t("{leg1} · Waiting for {name} to continue · {pct}% done", { leg1, name, pct });
      case "expired":
        return t("Expired: not accepted within 7 days");
      case "cancelled":
        return t("Cancelled");
      default:
        return r.message ?? t("Didn't go through");
    }
  }
  /** The bar follows the leg on its way: to the PC first, then to the contact (the last number
   *  in its text). */
  const relayPct = (r: DirectRelay) => (r.size > 0 ? Math.min(100, ((r.state === "leg1" || r.state === "receiving" ? r.leg1Done : r.done) / r.size) * 100) : 0);
  /** A contact of that PC with a relay on its way: what its row says. */
  function remoteMoving(id: string) {
    const list = (d?.relays ?? []).filter((r) => r.mine && !r.ended && r.device === fromId && r.to === id);
    if (!list.length) return null;
    // The leg on its way, as on its card: to the PC first, then to them.
    const first = list.some((r) => r.state === "leg1");
    const size = list.reduce((a, r) => a + r.size, 0);
    const done = list.reduce((a, r) => a + Math.min(first ? r.leg1Done : r.state === "leg1" ? 0 : r.done, r.size), 0);
    const pct = percent(done, size);
    const where = first ? (list[0].deviceName || t("your other PC")) : list[0].toName;
    return { text: t("To {pc} {pct}%", { pc: where, pct }), pct, icon: "up", count: list.length };
  }
  // The conversation shown stays connected, and is called at once if it closed for being quiet.
  const shownId = $derived(fromId ? null : (current?.id ?? null));
  $effect(() => {
    api.directShown(shownId).catch(() => {});
  });
  $effect(() => () => {
    api.directShown(null).catch(() => {});
  });
  const FILTER_TITLES = $derived<Record<string, string>>({ all: t("All contacts"), online: t("Online"), waiting: t("Waiting for you (contacts)"), favourites: t("Favourites") });
  const filterTitle = $derived(FILTER_TITLES[app.directFilter] ?? app.directFilter.replace(/^tag:/, ""));

  // ---------- the timeline: history plus what's moving, oldest first ----------
  type Row = { kind: "history"; h: DirectHistory } | { kind: "moving"; t: DirectTransfer } | { kind: "relay"; r: DirectRelay };
  const rows = $derived.by<Row[]>(() => {
    if (!current) return [];
    const relays = relaysHere.map((r) => ({ kind: "relay" as const, r }));
    if (fromId) {
      // That PC's own, read live; its send to them that this PC's relay card already shows
      // (both legs) isn't shown twice.
      const mineGoing = relays.some((x) => !x.r.ended);
      const hist = (talk?.history ?? []).map((h) => ({ kind: "history" as const, h }));
      const moving = (talk?.moving ?? []).filter((t) => !(mineGoing && t.via)).map((t) => ({ kind: "moving" as const, t: { ...t, contact: current.id } }));
      return [...hist, ...relays, ...moving];
    }
    const hist = (d?.history ?? []).filter((h) => h.contact === current.id).map((h) => ({ kind: "history" as const, h }));
    const moving = (d?.transfers ?? []).filter((t) => t.contact === current.id).map((t) => ({ kind: "moving" as const, t }));
    return [...hist, ...relays, ...moving];
  });
  // The details panel (shown or hidden with Ctrl+I, like Downloads'): this PC's contacts only.
  const showDetails = $derived(app.panelOpen && !!current && !fromId);
  // A column of its own when there's room; on a narrower window it slides over the conversation.
  let bodyWidth = $state(0);
  const detailsInline = $derived(bodyWidth >= 1040);
  // Its width, dragged on its edge and remembered. As a column: the room left after the people
  // (260 px with details), the gaps and the page's padding, the conversation keeping 420 px.
  // Over the conversation: up to the conversation's width, a strip of it still in view.
  const inlineRoom = $derived(bodyWidth > 0 ? bodyWidth - 32 - 260 - 24 : 0);
  const drawerRoom = $derived(bodyWidth > 0 ? bodyWidth - 32 - 280 - 12 : 0);
  const DRAWER_LIST = 120;
  // Its width eases only while it opens or closes (a window being resized moves it at once).
  let detailsEasing = $state(false);
  let detailsTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void showDetails;
    untrack(() => {
      detailsEasing = true;
      clearTimeout(detailsTimer);
      detailsTimer = setTimeout(() => (detailsEasing = false), 420);
    });
  });
  const detailsW = $derived(detailsInline ? shownWidth(panelWidths.of("direct", 288), inlineRoom) : shownWidth(panelWidths.of("direct", 288), drawerRoom, DRAWER_LIST));
  // Moving now (one waiting for them to come back isn't).
  const movingNow = $derived(current ? (d?.transfers ?? []).filter((t) => t.contact === current.id && t.state !== "waiting") : []);
  const endedWithThem = $derived(current ? (d?.history ?? []).filter((h) => h.contact === current.id).length : 0);

  // ---------- what was in a transfer: unfolded on its card, and in the details panel ----------
  /** A card's own key: the same while it moves and once it's in the history. */
  const cardKey = (r: Row) => (r.kind === "history" ? `${r.h.outgoing ? "o" : "i"}:${r.h.id}` : r.kind === "relay" ? `r:${r.r.id}` : `${r.t.outgoing ? "o" : "i"}:${r.t.id}`);
  /** Cards unfolded (their list of files and folders shown). */
  let unfolded = $state<Record<string, boolean>>({});
  /** The card picked (its list shows in the details panel); Esc or a click elsewhere lets go. */
  let pickedCard = $state<string | null>(null);
  $effect(() => {
    void current?.id;
    void fromId;
    untrack(() => (pickedCard = null));
  });
  const pickedInfo = $derived.by(() => {
    if (!pickedCard || fromId) return null;
    const r = rows.find((x) => cardKey(x) === pickedCard);
    if (!r) return null;
    const x = r.kind === "history" ? r.h : r.kind === "relay" ? r.r : r.t;
    if (!x.items?.length) return null;
    const outgoing = r.kind === "history" ? r.h.outgoing : r.kind === "relay" ? true : r.t.outgoing;
    const openId = r.kind === "history" && !r.h.outgoing && r.h.result === "done" && r.h.savedTo ? r.h.id : null;
    return { name: x.name, outgoing, files: x.files, size: x.size, items: x.items, more: x.more ?? 0, openId };
  });
  /** Back to the contact's details: the keyboard, if it was in the list that goes, goes back
   *  to the card (never taken from a field you're typing in). */
  function letGo() {
    const k = pickedCard;
    const a = document.activeElement;
    const lost = !a || a === document.body || !!a.closest(".details .picked");
    pickedCard = null;
    if (k && lost) timeline?.querySelector<HTMLElement>(`[data-card="${CSS.escape(k)}"]`)?.focus({ preventScroll: true });
  }
  function cardKeydown(e: KeyboardEvent, key: string) {
    if (e.target !== e.currentTarget || (e.key !== "Enter" && e.key !== " ")) return;
    e.preventDefault();
    pickedCard = key;
  }
  /** One file or folder of a received transfer: opened (a program or script is shown in its
   *  folder instead), or shown in its folder. */
  async function openPart(id: string, part: number, reveal: boolean) {
    if (await api.directOpen(id, reveal, part)) app.toast(t("Programs and scripts aren't started from Pako. It's shown in its folder: open it there if you trust it."));
  }
  let timeline: HTMLDivElement | undefined = $state();
  /** Whose conversation was last scrolled: a new item glides into view, a new contact opens at the end. */
  let scrolledFor: string | null = null;
  const reducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  /** Changes only when the conversation or its number of items changes (a progress update of a
   *  transfer must not pull the conversation back down while you read further up). */
  const scrollKey = $derived(`${current?.id ?? ""}|${rows.length}`);
  $effect(() => {
    // Newest at the bottom, kept in view.
    void scrollKey;
    const id = untrack(() => current?.id ?? null);
    const glide = id === scrolledFor && !reducedMotion();
    scrolledFor = id;
    queueMicrotask(() => timeline?.scrollTo({ top: timeline.scrollHeight, behavior: glide ? "smooth" : "auto" }));
  });
  /** Rows animate in and out, and move to their new place; not on very long lists. */
  const listMotion = $derived(contacts.length <= 60 ? 220 : 0);

  // Live speed per transfer, from how much moved since the last update.
  let lastDone: Record<string, { at: number; done: number; speed: number }> = {};
  function speedOf(x: DirectTransfer) {
    const now = performance.now();
    const prev = lastDone[x.id];
    if (!prev) {
      lastDone[x.id] = { at: now, done: x.done, speed: 0 };
      return 0;
    }
    if (now - prev.at > 400) {
      const s = ((x.done - prev.done) * 1000) / (now - prev.at);
      prev.speed = prev.speed ? prev.speed * 0.6 + s * 0.4 : s;
      prev.at = now;
      prev.done = x.done;
    }
    return prev.speed;
  }

  // ---------- what's moving with each contact (shown on their row) ----------
  const MOVING = new Set(["preparing", "sending", "receiving", "checking"]);
  /** Per contact: every transfer seen moving since that contact last had nothing moving (with
   *  its last size and progress), and the percent last shown. A finished one leaves the list of
   *  transfers but stays here as done, so the row's percent never goes back while the others
   *  carry on; the batch ends when nothing moves with them any more. */
  type Batch = { members: Map<string, { size: number; done: number; live: boolean }>; shown: number };
  const batches = new Map<string, Batch>();
  const movingBy = $derived.by(() => {
    const all = d?.transfers ?? [];
    const m = new Map<string, DirectTransfer[]>();
    for (const t of all) {
      if (!MOVING.has(t.state)) continue;
      const list = m.get(t.contact);
      if (list) list.push(t);
      else m.set(t.contact, [t]);
    }
    for (const contact of [...batches.keys()]) if (!m.has(contact)) batches.delete(contact);
    for (const [contact, list] of m) {
      let b = batches.get(contact);
      if (!b) batches.set(contact, (b = { members: new Map(), shown: 0 }));
      for (const x of b.members.values()) x.live = false;
      for (const t of list) b.members.set(t.id, { size: t.size, done: Math.min(t.done, t.size), live: true });
      // Gone from the list (finished): counted as all there.
      for (const x of b.members.values()) if (!x.live) x.done = x.size;
    }
    // Speed memory only for transfers still listed.
    const ids = new Set(all.map((t) => t.id));
    for (const id of Object.keys(lastDone)) if (!ids.has(id)) delete lastDone[id];
    return m;
  });
  /** How far a contact's batch is, in percent; never less than last shown (a transfer that
   *  joins or learns its size only holds the bar until the rest catches up). */
  function batchPercent(contact: string) {
    const b = batches.get(contact);
    if (!b) return 0;
    let size = 0;
    let done = 0;
    for (const x of b.members.values()) {
      size += x.size;
      done += x.done;
    }
    b.shown = Math.max(b.shown, percent(done, size));
    return b.shown;
  }
  /** One contact's transfers together: which way, how far, how fast. */
  function rowMoving(contact: string, list: DirectTransfer[]) {
    const flowing = list.filter((x) => x.state === "sending" || x.state === "receiving");
    const out = list.some((x) => x.outgoing);
    const inn = list.some((x) => !x.outgoing);
    const pct = batchPercent(contact);
    let text: string;
    if (!flowing.length) text = list.some((x) => x.state === "checking") ? t("Checking the files…") : t("Getting ready…");
    else {
      const rate = flowing.reduce((a, x) => a + speedOf(x), 0);
      const how = out && inn ? t("Sending and receiving {pct}%", { pct }) : out ? t("Sending {pct}%", { pct }) : t("Receiving {pct}%", { pct });
      text = `${how}${rate > 0 ? " · " + speed(rate) : ""}`;
    }
    return { text, pct, icon: out && !inn ? "up" : !out && inn ? "down" : "send", count: list.length };
  }

  const contactsText = (n: number) => t("{n} contacts", { n: num(n) });
  function resultText(h: DirectHistory) {
    if (h.result === "done") return h.outgoing ? t("Delivered") : t("Received");
    if (h.result === "declined") return t("Declined");
    if (h.result === "cancelled") return t("Cancelled");
    return h.message ?? t("Didn't go through");
  }
  /** Opens a received file; a program or script is shown in its folder instead. */
  async function openReceived(id: string) {
    if (await api.directOpen(id, false)) app.toast(t("Programs and scripts aren't started from Pako. It's shown in its folder: open it there if you trust it."));
  }
  function movingText(x: DirectTransfer) {
    if (x.state === "offered") return t("Waiting for them to accept…");
    if (x.state === "preparing") return t("Getting ready…");
    if (x.state === "checking") return t("Checking the files…");
    if (x.state === "waiting") return t("Waiting for {name} to continue · {pct}% done", { name: current?.name ?? t("them"), pct: percent(x.done, x.size) });
    return t("{pct}% · {done} of {size}", { pct: percent(x.done, x.size), done: bytes(x.done), size: bytes(x.size) });
  }

  // ---------- your ID and name ----------
  let myName = $state("");
  // Follows the engine's name whenever it changes (also when it was set elsewhere, like in the
  // Accept question), so the field never shows an old or empty name that leaving it would save.
  let seenName: string | null = null;
  $effect(() => {
    if (!d?.ready) return;
    const n = d.myName ?? "";
    if (n !== seenName) {
      seenName = n;
      myName = n;
    }
  });
  async function saveName() {
    // An emptied field puts the name back: contacts always see a name once you have one.
    if (!myName.trim()) {
      myName = d?.myName ?? "";
      return;
    }
    if ((d?.myName ?? "") === myName.trim()) return;
    await app.run(() => api.directSetName(myName.trim()));
  }
  function copyId() {
    if (d?.myId) app.run(() => api.copyText(d.myId!)).then(() => app.toast(t("Your Pako ID is copied. Send it to the people you want to connect with.")));
  }
  /** The ID under "You, sending from": this PC's, or that PC's. */
  function copyShownId() {
    if (!app.directFrom) return copyId();
    const id = book?.me.id;
    const name = fromChoices.find((c) => c.id === app.directFrom)?.name ?? t("That PC");
    if (id) app.run(() => api.copyText(id)).then(() => app.toast(t("{name}'s Pako ID is copied.", { name })));
  }
  const shortId = (id: string) => `${id.slice(0, 6)}…${id.slice(-6)}`;

  // ---------- seen by Pakos on the same network ----------
  const visible = $derived<DirectVisible>(app.settings?.directVisible ?? "home");
  /** Why Pakos on this network don't see you now (empty: they do, or you chose Never). */
  const hiddenNote = $derived(visible === "never" ? "" : nearbyNote(extra.nearby));
  async function setVisible(v: DirectVisible) {
    if (!app.settings || v === visible) return;
    const next = { ...app.settings, directVisible: v };
    app.settings = next;
    await app.run(() => api.setSettings(next));
  }

  // ---------- add a contact (also asked from the sidebar) ----------
  let newId = $state("");
  let newName = $state("");
  let addError = $state("");
  /** The add window's note, "You, sending from" in bold mid-sentence. */
  const askedOnce = $derived(
    around(t("Asked once: your contacts see this name, so they know who's asking. You can change it later: click it under {place}, at the top of the contact list."), "{place}"),
  );
  /** No name of yours yet: the add window asks for it first (contacts see it). */
  const nameMissing = $derived(!(d?.myName ?? "").trim());
  /** Add pressed with no name: said under the field, which takes the focus. */
  let needName = $state(false);
  let nameField = $state<HTMLInputElement | null>(null);
  $effect(() => {
    if (myName.trim()) needName = false;
  });
  async function addContact() {
    addError = "";
    if (nameMissing && !myName.trim()) {
      needName = true;
      nameField?.focus();
      return;
    }
    try {
      if (nameMissing) await api.directSetName(myName.trim());
      await api.directAddContact(newId.trim(), newName.trim());
      app.directAdding = false;
      newId = "";
      newName = "";
      app.toast(t("Added. Pako is reaching them now; the first time, they're asked to accept."));
    } catch (e) {
      addError = String(e);
    }
  }

  // ---------- rename ----------
  let renaming = $state<DirectContact | null>(null);
  let renameTo = $state("");
  async function rename() {
    if (!renaming) return;
    const c = renaming;
    renaming = null;
    await app.run(() => api.directEditContact(c.id, { name: renameTo }));
  }
  let removing = $state<DirectContact | null>(null);
  /** "Clear history…" asks first: the list of what went between you can't be brought back. */
  let clearing = $state<DirectContact | null>(null);

  function contactMenu(e: MouseEvent, c: DirectContact, atMouse = false) {
    const b = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (atMouse) e.preventDefault();
    onmenu(atMouse ? e.clientX : b.right - 240, atMouse ? e.clientY : b.bottom + 4, [
      { label: t("Rename…"), action: () => ((renaming = c), (renameTo = c.name)) },
      { label: t("Tags…"), action: () => openTags(c) },
      { label: c.favourite ? t("Remove from favourites") : t("Add to favourites"), action: () => app.run(() => api.directSetFavourite(c.id, !c.favourite)) },
      { label: c.muted ? t("Unmute") : t("Mute"), action: () => app.run(() => api.directSetMuted(c.id, !c.muted)) },
      ...(c.unchecked ? [{ label: t("Check now"), action: () => app.run(() => api.directConnect(c.id)) }] : []),
      ...(c.check !== "often" && c.checkAs !== "often" ? [{ label: t("Check often for 1 hour"), action: () => app.run(() => api.directSetCheck(c.id, "often", "hour")) }] : []),
      { separator: true },
      { label: t("Can send to me without asking"), hint: c.autoAccept ? "✓" : "", action: () => app.run(() => api.directEditContact(c.id, { autoAccept: !c.autoAccept })) },
      { label: t("Save their files in…"), action: () => chooseFolder(c) },
      { separator: true },
      { label: t("Clear history…"), action: () => (clearing = c) },
      { label: t("Remove contact"), danger: true, action: () => (removing = c) },
    ]);
  }

  // ---------- tags for one contact (only on this PC) ----------
  let tagging = $state<DirectContact | null>(null);
  let ticked = $state<string[]>([]);
  let newTag = $state("");
  function openTags(c: DirectContact) {
    tagging = c;
    ticked = [...(c.tags ?? [])];
    newTag = "";
  }
  function toggleTag(tag: string) {
    ticked = ticked.some((x) => x.toLowerCase() === tag.toLowerCase()) ? ticked.filter((x) => x.toLowerCase() !== tag.toLowerCase()) : [...ticked, tag];
  }
  function addNewTag(tag = newTag) {
    const name = tag.trim();
    if (!name) return;
    if (!ticked.some((x) => x.toLowerCase() === name.toLowerCase())) ticked = [...ticked, name];
    newTag = "";
  }
  /** Your tags plus new ones typed here (not saved until Save). */
  const tagChoices = $derived([...app.directTags, ...ticked.filter((tag) => !app.directTags.some((x) => x.toLowerCase() === tag.toLowerCase()))]);
  async function saveTags() {
    if (!tagging) return;
    const c = tagging;
    tagging = null;
    await app.run(() => api.directSetTags(c.id, ticked));
  }
  async function chooseFolder(c: DirectContact) {
    const r = await pick({ directory: true, multiple: false, title: t("Where {name}'s files go", { name: c.name }) });
    if (typeof r === "string") await app.run(() => api.directEditContact(c.id, { folder: r }));
  }

  // ---------- the send area ----------
  /** Files waiting to be sent, kept per contact (like a draft): files picked for one person never
   *  end up sent to another one shown later. */
  let baskets = $state<Record<string, string[]>>({});
  const basket = $derived(current ? (baskets[current.id] ?? []) : []);
  function setBasket(list: string[], id = current?.id) {
    if (id) baskets = { ...baskets, [id]: list };
  }
  let compress = $state(false);
  let sendName = $state("");
  /** Sending through another PC: keep a copy there once delivered. */
  let keepCopy = $state(false);
  /** Send is possible now: they're online here, or (through another PC) that PC is online and
   *  they said yes to it once (if they're away, it waits for them there). */
  const canSend = $derived(!current ? false : fromId ? !!from?.online && current.trusted && !current.declined : current.online);
  let sending = $state(false);
  /** The name and the compression choice belong to one person's files too: another conversation
   *  shows its own (a name typed for Alex's files never goes out with Jo's). */
  const drafts = new Map<string, { name: string; compress: boolean }>();
  let draftFor: string | null = null;
  $effect(() => {
    const id = current?.id ?? null;
    if (id === draftFor) return;
    untrack(() => {
      if (draftFor) drafts.set(draftFor, { name: sendName, compress });
      const mine = id ? drafts.get(id) : undefined;
      sendName = mine?.name ?? "";
      compress = mine?.compress ?? false;
      draftFor = id;
    });
  });
  const nameOf = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
  /** A long name in a basket chip: the start gives way, the end (with the extension) stays. */
  const tailOf = (n: string) => (n.length > 22 ? n.slice(-10) : "");
  function addPaths(paths: string[]) {
    const have = new Set(basket.map((p) => p.toLowerCase()));
    const fresh: string[] = [];
    for (const p of paths) {
      const k = p.toLowerCase();
      if (p && !have.has(k)) {
        have.add(k);
        fresh.push(p);
      }
    }
    if (fresh.length) setBasket([...basket, ...fresh]);
  }
  async function chooseFiles() {
    const r = await pick({ multiple: true, directory: false, title: t("Choose files to send") });
    addPaths(Array.isArray(r) ? r : r ? [r] : []);
  }
  async function chooseDir() {
    const r = await pick({ multiple: true, directory: true, title: t("Choose folders to send") });
    addPaths(Array.isArray(r) ? r : r ? [r] : []);
  }
  async function send() {
    if (!current || !basket.length || sending) return;
    const to = current.id;
    const paths = basket;
    sending = true;
    try {
      if (fromId) await api.directRelaySend(fromId, to, current.name, paths, compress, sendName.trim() || null, keepCopy);
      else await api.directSend(to, paths, compress, sendName.trim() || null);
      // Only what was sent leaves: files added for them while it started stay for the next send.
      const sent = new Set(paths);
      setBasket((baskets[to] ?? []).filter((p) => !sent.has(p)), to);
      if (current?.id === to) {
        sendName = "";
        compress = false;
      } else drafts.delete(to);
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      sending = false;
    }
  }

  // Files dropped from Explorer (their paths come back as "files-dropped").
  let dropping = $state(false);
  let depth = 0;
  const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes("Files");
  function dragEnter(e: DragEvent) {
    if (!hasFiles(e) || !current) return;
    depth++;
    dropping = true;
  }
  function dragOver(e: DragEvent) {
    if (!hasFiles(e) || !current) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer!.dropEffect = "copy";
  }
  function dragLeave(e: DragEvent) {
    if (!hasFiles(e)) return;
    if (--depth <= 0) {
      depth = 0;
      dropping = false;
    }
  }
  async function drop(e: DragEvent) {
    if (!hasFiles(e) || !current) return;
    e.preventDefault();
    e.stopPropagation();
    depth = 0;
    dropping = false;
    const dt = e.dataTransfer!;
    const webview = (window as { chrome?: { webview?: { postMessageWithAdditionalObjects?: (m: string, o: unknown[]) => void } } }).chrome?.webview;
    if (!webview?.postMessageWithAdditionalObjects) {
      app.toast(t("Dropping files doesn't work here. Use Choose files."), "error");
      return;
    }
    const files = Array.from(dt.files);
    type WithHandle = DataTransferItem & { getAsFileSystemHandle?: () => Promise<unknown> };
    const asked = Array.from(dt.items)
      .filter((i) => i.kind === "file")
      .map((i) => (i as WithHandle).getAsFileSystemHandle?.() ?? Promise.reject());
    const got = await Promise.allSettled(asked);
    const handles = got.flatMap((r) => (r.status === "fulfilled" && r.value ? [r.value] : []));
    try {
      webview.postMessageWithAdditionalObjects("pako-files", [...files, ...handles]);
    } catch {
      webview.postMessageWithAdditionalObjects("pako-files", files);
    }
  }
  $effect(() => {
    const un = listen<{ paths: string[]; detail: string }>("files-dropped", (e) => {
      if (e.payload.paths.length) addPaths(e.payload.paths);
      else app.toast(t("Pako couldn't read what was dropped ({detail}).", { detail: e.payload.detail }), "error");
    });
    return () => {
      un.then((f) => f());
    };
  });
</script>

{#snippet seenBy()}
  <label class="mlabel" for="seenby">{t("Be visible to Pakos on this network")}</label>
  <select
    id="seenby"
    class="input seenby"
    value={visible}
    onchange={(e) => setVisible(e.currentTarget.value as DirectVisible)}
    title={t("Pakos on the same Wi-Fi or cable can find you there, even without the internet. Either way, you still find your contacts there.")}
  >
    <option value="home">{t("On home networks")}</option>
    <option value="always">{t("Always")}</option>
    <option value="never">{t("Never")}</option>
  </select>
  {#if hiddenNote}
    <p class="seennote" transition:rowSlide={{ duration: 200 }}>{hiddenNote}</p>
  {/if}
{/snippet}

<!-- A card's chevron: unfolds what was in it (only when there's more than one thing). -->
{#snippet chev(key: string, name: string)}
  <button
    class="chev"
    class:open={unfolded[key]}
    aria-expanded={!!unfolded[key]}
    aria-controls="items-{key}"
    aria-label={unfolded[key] ? t("Hide what was in {name}", { name }) : t("Show what was in {name}", { name })}
    title={unfolded[key] ? t("Hide the files") : t("Show the files")}
    onclick={() => (unfolded[key] = !unfolded[key])}><Icon name="chevron" size={12} stroke={2.4} /></button
  >
{/snippet}

{#snippet fold(key: string, items: DirectItem[] | undefined, more: number | undefined, openId: string | null)}
  {#if unfolded[key] && items?.length}
    <div class="mlist" transition:rowSlide={{ duration: 220 }}>
      <TransferItems {items} more={more ?? 0} id="items-{key}" open={openId ? (i, reveal) => app.run(() => openPart(openId, i, reveal)) : null} />
    </div>
  {/if}
{/snippet}

<svelte:window
  onresize={() => (fromMenu = false)}
  onblur={() => (fromMenu = false)}
  onkeydown={(e) => {
    if (e.key === "Escape" && pickedCard && !app.modals && !e.defaultPrevented) letGo();
  }}
/>

<div class="direct" role="presentation" ondragenter={dragEnter} ondragover={dragOver} ondragleave={dragLeave} ondrop={drop}>
  {#if dropping && current}
    <div class="dropzone" aria-hidden="true" transition:fadeIn={{ duration: 120 }}>
      <div class="dropcard"><Icon name="send" size={22} />{t("Drop files or folders to send them to {name}", { name: current.name })}</div>
    </div>
  {/if}
  <div class="toolbar">
    <h1>{t("Direct")}</h1>
    <TutorialButton id="direct" />
    <span class="dim small sub">{t("Files straight to people you know, PC to PC")}</span>
  </div>

  {#if !d}
    <div class="center dim">{t("Starting…")}</div>
  {:else if !d.ready}
    <div class="center dim">
      {d.error ?? t("Direct is starting…")}
      {#if !d.error && extra.firstStart}
        <p class="firewall" in:fadeIn={{ duration: 200 }}>{t("Windows may ask whether Pako can use the network. Allow it on private networks, so Pakos on your home network can reach you.")}</p>
      {/if}
    </div>
  {:else}
    <!-- The welcome card and the contacts crossfade in the same place (the tutorial's example
         people going away, your first contact arriving): no jump. -->
    <div class="stage">
    {#if !everyone.length && !fromChoices.length && !fromId}
      <div class="body solo" transition:fadeIn={{ duration: 220 }}>
        <section class="card welcome" data-tour="direct-welcome">
          <div class="empty-icon"><Icon name="send" size={26} /></div>
          <h3>{t("Send files straight to people you know")}</h3>
          <p class="dim">{t("Files go PC to PC, encrypted, with nothing stored anywhere. Share your Pako ID with a friend, or add theirs: the first time they're asked to accept, then you connect by yourselves.")}</p>
          <div class="idbox">
            <span class="mlabel">{t("Your Pako ID")}</span>
            <div class="idrow">
              <span class="mono idtext" title={d.myId}>{d.myId}</span>
              <button class="btn small" onclick={copyId}><Icon name="copy" size={12} />{t("Copy")}</button>
            </div>
            {@render seenBy()}
          </div>
          <button class="btn primary" onclick={() => ((app.directAdding = true), (addError = ""))}><Icon name="plus" size={14} stroke={2.2} />{t("Add contact")}</button>
        </section>
      </div>
    {:else}
    <div class="body" class:withdetails={showDetails && detailsInline} style="--pw: {detailsW}px" bind:clientWidth={bodyWidth} transition:fadeIn={{ duration: 220 }}>
      <aside class="people card" data-tour="direct-people">
        <!-- You, sending from: your name and the PC, one button (the name renames it, the PC
             part picks another of your PCs to send from), the Pako ID under it. -->
        <div class="from" data-tour="direct-me">
          <span class="mlabel">{t("You, sending from")}</span>
          <div class="fromsel" class:solo={!fromChoices.length} class:remote={!!fromId} bind:this={fromSel}>
            {#if renamingMe}
              <input
                class="fname-in"
                bind:this={meField}
                bind:value={myName}
                onblur={endRename}
                onkeydown={(e) => {
                  if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
                  else if (e.key === "Escape") {
                    myName = d.myName ?? "";
                    (e.currentTarget as HTMLInputElement).blur();
                  }
                }}
                placeholder={t("What your contacts see")}
                maxlength="60"
                aria-label={t("Your name")}
              />
            {:else}
              <button class="fname" class:empty={!shownName} disabled={!!fromId} onclick={startRename} title={fromId ? t("The name {pc}'s contacts see. Change it on {pc}.", { pc: from?.name ?? t("that PC") }) : t("Your name: what your contacts see. Click to change it.")}>{shownName || t("Your name")}</button>
            {/if}
            <span class="fsep" aria-hidden="true">·</span>
            {#if fromChoices.length}
              <button class="fpc" aria-haspopup="menu" aria-expanded={fromMenu} onclick={toggleFromMenu} title={t("Send from this PC or another of your PCs")}>
                <span class="fpcname">{from?.name ?? t("This PC")}</span><span class="fchev" class:open={fromMenu}><Icon name="chevron" size={11} stroke={2.4} /></span>
              </button>
            {:else}
              <span class="fpc still">{t("This PC")}</span>
            {/if}
          </div>
          {#if fromMenu}
            <div class="fcatch" role="presentation" onmousedown={() => (fromMenu = false)}></div>
            <div class="frommenu" role="menu" tabindex="-1" style:left="{menuAt.left}px" style:top="{menuAt.top}px" style:width="{menuAt.width}px" transition:pop={{ duration: 160, from: 0.96, y: -4, origin: "top left" }} onkeydown={(e) => e.key === "Escape" && (fromMenu = false)}>
              <button class="fitem" class:on={!fromId} role="menuitemradio" aria-checked={!fromId} onclick={() => pickFrom(null)}>
                <span class="fic"><Icon name="devices" size={14} /></span>
                <span class="ftext">
                  <span class="ftop">{t("This PC")}</span>
                  <span class="fsub">{t('Contacts see "{name}" · {count}', { name: d.myName || t("no name yet"), count: contactsText(everyone.length) })}</span>
                </span>
                {#if !fromId}<span class="fok"><Icon name="check" size={12} stroke={2.6} /></span>{/if}
              </button>
              {#each fromChoices as c (c.id)}
                <button class="fitem" class:on={fromId === c.id} class:off={!c.online} role="menuitemradio" aria-checked={fromId === c.id} disabled={!c.online} title={c.online ? undefined : t("{name} is offline", { name: c.name })} onclick={() => pickFrom(c.id)}>
                  <span class="fic"><Icon name="devices" size={14} /></span>
                  <span class="ftext">
                    <span class="ftop">{c.name}{#if c.pcName}<span class="fcalls">&nbsp;· {t("calls itself {name}", { name: c.pcName })}</span>{/if}</span>
                    <span class="fsub">{c.online ? "" : `${t("Offline")} · `}{t('Contacts see "{name}" · {count}', { name: c.directName || t("no name yet"), count: contactsText(c.contacts) })}</span>
                  </span>
                  {#if fromId === c.id}<span class="fok"><Icon name="check" size={12} stroke={2.6} /></span>{/if}
                </button>
              {/each}
              <p class="fnote">{t("Renaming stays in My devices.")}</p>
            </div>
          {/if}
          <div class="idrow">
            <span class="mono idtext" title={shownIdText}>{shownIdText ? shortId(shownIdText) : "…"}</span>
            <button class="btn small" disabled={!shownIdText} onclick={copyShownId}><Icon name="copy" size={12} />{t("Copy ID")}</button>
          </div>
        </div>
        <div class="phead">
          <span class="ptitle">{filterTitle}</span>
          <span class="pcount mono">{contacts.length}</span>
          {#if app.directExample}
            <span class="example" title={t("Made-up people, only shown during the tutorial")} transition:pop={{ duration: 200, from: 0.92, y: 0 }}>{t("Examples")}</span>
          {/if}
          <div class="spacer"></div>
          {#if !fromId}
            <button class="btn small" data-tour="direct-add" onclick={() => ((app.directAdding = true), (addError = ""))} title={t("Add a contact by their Pako ID")}><Icon name="plus" size={12} stroke={2.2} />{t("Add")}</button>
          {/if}
        </div>
        {#if searching}
          <div class="psearch" transition:rowSlide={{ duration: 200 }}>
            <Icon name="search" size={13} />
            <input bind:value={search} placeholder={t("Search contacts")} aria-label={t("Search contacts")} spellcheck="false" />
            {#if search}<button class="clear" aria-label={t("Clear the search")} onclick={() => (search = "")}><Icon name="x" size={11} stroke={2.4} /></button>{/if}
          </div>
        {/if}
        <div class="plist">
          {#each contacts as c (c.id)}
            {@const mv = fromId ? undefined : movingBy.get(c.id)}
            {@const m = fromId ? remoteMoving(c.id) : mv ? rowMoving(c.id, mv) : null}
            <button
              class="person"
              class:on={current?.id === c.id}
              class:moving={!!m}
              onclick={() => pickContact(c.id)}
              oncontextmenu={(e) => (fromId ? e.preventDefault() : contactMenu(e, c, true))}
              transition:rowSlide={{ duration: listMotion }}
              animate:flipIf={{ ...flipOpts, duration: listMotion }}
            >
              <span class="avatar">{initial(c.name)}<span class="dot" class:online={c.online} class:local={isLocal(c)} class:wait={c.waiting}></span></span>
              <span class="ptext">
                <span class="pname">{c.name}{#if c.favourite}<span class="fav" aria-label={t("Favourite")}><Icon name="star" size={11} stroke={2.2} /></span>{/if}{#if c.muted}<span class="mutedmark" aria-label={t("Muted")} title={t("Muted: no pop-ups or sounds from {name}", { name: c.name })}><Icon name="bell-off" size={11} stroke={2.2} /></span>{/if}</span>
                {#if m}
                  <span class="pstat live" title={m.count > 1 ? t("{n} transfers with {name}", { n: num(m.count), name: c.name }) : undefined}><Icon name={m.icon} size={11} stroke={2.4} />{m.text}</span>
                {:else}
                  <span class="pstat">{statusOf(c, internet)}</span>
                {/if}
                {#if c.tags?.length}
                  <span class="ptags">
                    {#each c.tags.slice(0, 2) as tag, i (i)}<span class="tchip">{tag}</span>{/each}
                    {#if c.tags.length > 2}<span class="tchip more">+{c.tags.length - 2}</span>{/if}
                  </span>
                {/if}
              </span>
              {#if !fromId && app.directWaitingIds.has(c.id)}<span class="badge" title={t("Waiting for your answer")} transition:pop={{ duration: 180, from: 0.92, y: 0 }}>!</span>{/if}
              {#if m}
                <span class="rowbar" aria-hidden="true" transition:fadeIn={{ duration: 200 }}><i style:transform="scaleX({m.pct / 100})"></i></span>
              {/if}
            </button>
          {:else}
            <div class="noone">
              {#if search}
                <p>{t('Nobody matches "{q}".', { q: search })}</p>
              {:else if fromId && bookError}
                <p class="dim">{bookError}</p>
              {:else if fromId && !book}
                <p class="dim">{t("Reading {pc}'s contacts…", { pc: from?.name ?? t("that PC") })}</p>
              {:else if fromId}
                <p class="dim">{t("{pc} has no contacts here.", { pc: from?.name ?? t("That PC") })}</p>
              {:else}
                <p class="dim">{t("Nobody here right now.")}</p>
              {/if}
            </div>
          {/each}
        </div>
        <div class="me">
          {@render seenBy()}
        </div>
      </aside>

      <section class="chat card" data-tour="direct-chat">
        {#if !current}
          <div class="center">
            <p class="dim">{search ? t("Nobody matches your search.") : t("Nobody under this filter. Pick another one on the left.")}</p>
          </div>
        {:else}
          <header class="chead">
            <span class="avatar big">{initial(current.name)}<span class="dot" class:online={current.online} class:local={isLocal(current)} class:wait={current.waiting}></span></span>
            <div class="ctext">
              <div class="cname">
                {current.name}
                {#if current.muted}<span class="mutedmark" aria-label={t("Muted")} title={t("Muted: no pop-ups or sounds from {name}", { name: current.name })}><Icon name="bell-off" size={12} stroke={2.2} /></span>{/if}
                {#if current.theirName && current.theirName !== current.name}<span class="theirs dim">{t('calls themselves "{name}"', { name: current.theirName })}</span>{/if}
              </div>
              <div class="cstat dim">{statusOf(current, internet)}{#if current.autoAccept}{` · ${t("can send without asking")}`}{/if}{#if fromId}{` · ${t("a contact of {pc}: files go through it", { pc: from?.name ?? t("that PC") })}`}{/if}</div>
            </div>
            <div class="spacer"></div>
            {#if fromId}
              <!-- That PC's contact: shown only (it's changed on that PC). -->
            {:else}
            {#if !current.online}
              <button class="btn small" onclick={() => app.run(() => api.directConnect(current!.id))}><Icon name="retry" size={12} />{current.unchecked ? t("Check now") : t("Connect")}</button>
            {/if}
            <button class="icon-btn" aria-label={t("More for {name}", { name: current.name })} title={t("More")} onclick={(e) => contactMenu(e, current!)}><Icon name="settings" size={14} /></button>
            <button
              class="icon-btn"
              class:on={app.panelOpen}
              aria-label={app.panelOpen ? t("Hide details (Ctrl+I)") : t("Show details (Ctrl+I)")}
              title={app.panelOpen ? t("Hide details (Ctrl+I)") : t("Show details (Ctrl+I)")}
              onclick={() => app.togglePanel()}
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
                <rect x="3" y="5" width="18" height="14" rx="1.5" />
                <path d="M15 5v14" />
                {#if app.panelOpen}<path d="M15 5h6v14h-6z" fill="currentColor" stroke="none" />{/if}
              </svg>
            </button>
            {/if}
          </header>

          {#if fromId}
            <!-- (The safety code and their answer are between them and that PC.) -->
          {:else if current.declined && !current.waiting}
            <div class="safety" transition:rowSlide={{ duration: 220 }}>
              <Icon name="alert" size={14} />
              <span>{t("{name} declined. For 7 days their Pako turns you away without asking them; after that, press Connect to ask again.", { name: current.name })}</span>
            </div>
          {:else if current.waiting || !current.trusted}
            <div class="safety" transition:rowSlide={{ duration: 220 }}>
              <Icon name="alert" size={14} />
              <span>{t("The first time, {name} is asked to accept. Compare this code with theirs once (by voice or chat):", { name: current.name })} <b class="mono">{current.code}</b></span>
            </div>
          {/if}

          <div class="mid">
          {#key current.id}
          <!-- A click outside the cards lets go of the one picked (the keyboard: Esc). -->
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <div class="timeline" bind:this={timeline} in:fadeIn={{ duration: 160 }} onclick={(e) => !(e.target as Element).closest(".bubble") && (pickedCard = null)}>
            {#each rows as r (r.kind === "history" ? `h:${r.h.outgoing ? "out" : "in"}:${r.h.id}` : r.kind === "relay" ? `r:${r.r.id}` : `t:${r.t.outgoing ? "out" : "in"}:${r.t.id}`)}
              {#if r.kind === "history"}
                {@const h = r.h}
                {@const key = cardKey(r)}
                {@const openId = !h.outgoing && h.result === "done" && h.savedTo ? h.id : null}
                <div class="msg" class:out={h.outgoing} class:bad={h.result !== "done"} in:pop={{ duration: 240, from: 0.98, y: 8, origin: h.outgoing ? "right bottom" : "left bottom" }}>
                  <!-- A group, not a button: it holds buttons of its own (Cancel, Open…), which a button would hide from screen readers. -->
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
                  <div class="bubble" class:picked={pickedCard === key} role="group" tabindex="0" data-card={key} aria-current={pickedCard === key ? "true" : undefined} aria-label={h.outgoing ? t("Sent: {name}", { name: h.name }) : t("Received: {name}", { name: h.name })} onclick={() => (pickedCard = key)} onkeydown={(e) => cardKeydown(e, key)}>
                    <div class="mtop">
                      <FileIcon name={h.name} kind={h.files > 1 ? "folder" : undefined} size={18} />
                      <MiddleText text={h.name} class="mname" />
                      {#if hasList(h.items, h.more)}{@render chev(key, h.name)}{/if}
                    </div>
                    <div class="mmeta">{itemsCount(h.files, h.items, h.more)} · {bytes(h.size)}{#if h.via}<span class="via" title={t("Sent on for {name}", { name: h.via })}>{t("from {name}", { name: h.via })}</span>{/if}</div>
                    <div class="mfoot">
                      <span class="res" class:ok={h.result === "done"}>{resultText(h)}</span>
                      <span class="dim">{shortDate(h.date)}{#if h.path}{" · " + pathLabel(h.path)}{/if}</span>
                      {#if !h.outgoing && h.result === "done" && h.savedTo}
                        <span class="acts">
                          <button class="link" onclick={() => app.run(() => openReceived(h.id))}>{t("Open")}</button>
                          <button class="link" onclick={() => app.run(() => api.directOpen(h.id, true))}>{t("Folder")}</button>
                        </span>
                      {/if}
                    </div>
                    {@render fold(key, h.items, h.more, openId)}
                  </div>
                </div>
              {:else if r.kind === "moving"}
                {@const tr = r.t}
                {@const key = cardKey(r)}
                <div class="msg live" class:out={tr.outgoing} in:pop={{ duration: 240, from: 0.98, y: 8, origin: tr.outgoing ? "right bottom" : "left bottom" }}>
                  <!-- A group, not a button: it holds buttons of its own (Cancel, Open…), which a button would hide from screen readers. -->
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
                  <div class="bubble" class:picked={pickedCard === key} role="group" tabindex="0" data-card={key} aria-current={pickedCard === key ? "true" : undefined} aria-label={tr.outgoing ? t("Sending: {name}", { name: tr.name }) : t("Receiving: {name}", { name: tr.name })} onclick={() => (pickedCard = key)} onkeydown={(e) => cardKeydown(e, key)}>
                    <div class="mtop">
                      <FileIcon name={tr.name} kind={tr.files > 1 ? "folder" : undefined} size={18} />
                      <MiddleText text={tr.name} class="mname" />
                      {#if hasList(tr.items, tr.more)}{@render chev(key, tr.name)}{/if}
                    </div>
                    <div class="mmeta">{itemsCount(tr.files, tr.items, tr.more)} · {bytes(tr.size)}{#if tr.via}<span class="via" title={t("Sent on for {name}", { name: tr.via })}>{t("from {name}", { name: tr.via })}</span>{/if}</div>
                    <div class="bar" class:waiting={tr.state === "waiting"}><i style:width="{Math.min(100, (tr.done / Math.max(1, tr.size)) * 100)}%"></i></div>
                    <div class="mfoot">
                      <span class="dim">{movingText(tr)}{#if tr.state === "sending" || tr.state === "receiving"}{" · " + speed(speedOf(tr))}{/if}{#if tr.path}{" · " + pathLabel(tr.path)}{/if}</span>
                      {#if !fromId}<button class="link" onclick={() => app.run(() => api.directCancel(tr.id))}>{t("Cancel")}</button>{/if}
                    </div>
                    {@render fold(key, tr.items, tr.more, null)}
                  </div>
                </div>
              {:else if r.kind === "relay"}
                {@const x = r.r}
                {@const key = cardKey(r)}
                <div class="msg out" class:live={!x.ended} class:bad={!!x.ended} in:pop={{ duration: 240, from: 0.98, y: 8, origin: "right bottom" }}>
                  <!-- A group, not a button: it holds buttons of its own (Cancel, Open…), which a button would hide from screen readers. -->
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
                  <div class="bubble" class:picked={pickedCard === key} role="group" tabindex="0" data-card={key} aria-current={pickedCard === key ? "true" : undefined} aria-label={t("Sent through {pc}: {name}", { pc: x.deviceName, name: x.name })} onclick={() => (pickedCard = key)} onkeydown={(e) => cardKeydown(e, key)}>
                    <div class="mtop">
                      <FileIcon name={x.name} kind={x.files > 1 ? "folder" : undefined} size={18} />
                      <MiddleText text={x.name} class="mname" />
                      {#if hasList(x.items, x.more)}{@render chev(key, x.name)}{/if}
                    </div>
                    <div class="mmeta">{itemsCount(x.files, x.items, x.more)} · {bytes(x.size)}<span class="via" title={x.mine ? t("Through {pc}", { pc: x.deviceName }) : t("Sent on for {name}", { name: x.deviceName })}>{x.mine ? t("through {pc}", { pc: x.deviceName }) : t("from {name}", { name: x.deviceName })}</span></div>
                    {#if !x.ended}
                      <div class="bar" class:waiting={x.state === "paused" || x.state === "waiting"}><i style:width="{relayPct(x)}%"></i></div>
                    {/if}
                    <div class="mfoot">
                      {#if x.ended}
                        <span class="res">{relayText(x)}</span>
                        <span class="dim">{shortDate(x.ended)}</span>
                      {:else}
                        <span class="dim">{relayText(x)}</span>
                        <button class="link" onclick={() => app.run(() => api.directRelayCancel(x.id))}>{t("Cancel")}</button>
                      {/if}
                    </div>
                    {@render fold(key, x.items, x.more, null)}
                  </div>
                </div>
              {/if}
            {:else}
              <div class="nothing dim">{fromId && !talk ? t("Reading {pc}'s history with {name}…", { pc: from?.name ?? t("that PC"), name: current.name }) : fromId ? t("Nothing sent or received between {pc} and {name} yet.", { pc: from?.name ?? t("that PC"), name: current.name }) : t("Nothing sent or received with {name} yet.", { name: current.name })}</div>
            {/each}
          </div>
          {/key}
          {#if showDetails && !detailsInline}
            <DirectDetails drawer resize={{ width: detailsW, avail: drawerRoom, listMin: DRAWER_LIST }} contact={current} moving={movingNow} {speedOf} historyCount={endedWithThem} transfer={pickedInfo} onopen={(id, i, reveal) => app.run(() => openPart(id, i, reveal))} onletgo={letGo} ontags={() => openTags(current!)} onfolder={() => chooseFolder(current!)} />
          {/if}
          </div>

          <footer class="compose" data-tour="direct-send">
            {#if basket.length}
              <div class="basket" transition:rowSlide={{ duration: 220 }}>
              <div class="items">
                {#each basket as p (p)}
                  {@const nm = nameOf(p)}
                  {@const tail = tailOf(nm)}
                  <span class="chip" title={p} transition:pop={{ duration: 180, from: 0.9, y: 0 }} animate:flipIf={flipOpts}><FileIcon name={nm} kind={/\.[a-z0-9]{1,5}$/i.test(p) ? undefined : "folder"} size={14} /><span class="cn"><span class="head">{tail ? nm.slice(0, -tail.length) : nm}</span>{#if tail}<span class="tail">{tail}</span>{/if}</span><button aria-label={t("Remove {name}", { name: nm })} onclick={() => setBasket(basket.filter((x) => x !== p))}><Icon name="x" size={10} stroke={2.4} /></button></span>
                {/each}
              </div>
              <div class="opts">
                <SegPick
                  label={t("Compression")}
                  options={[
                    { value: "plain", label: t("As they are") },
                    { value: "zip", label: t("Compressed ZIP") },
                  ]}
                  value={compress ? "zip" : "plain"}
                  onpick={(v) => (compress = v === "zip")}
                />
                <input class="input name" bind:value={sendName} placeholder={t("Name (optional)")} maxlength="120" aria-label={t("Name of this send")} />
                {#if fromId}
                  <span class="keep" title={t("Once {name} has them, a copy stays on {pc} (in its folder for files from this PC). Off: the copy there is deleted.", { name: current.name, pc: from?.name ?? t("that PC") })}>
                    <button class="switch" class:on={keepCopy} role="switch" aria-checked={keepCopy} aria-label={t("Keep a copy on {pc}", { pc: from?.name ?? t("that PC") })} onclick={() => (keepCopy = !keepCopy)}></button>
                    <span class="keepl">{t("Keep a copy on {pc}", { pc: from?.name ?? t("that PC") })}</span>
                  </span>
                {/if}
                <!-- Not enough room on one line: the two buttons go to the next one, still on the right. -->
                <div class="go">
                  <button class="btn ghost" onclick={() => setBasket([])}>{t("Clear")}</button>
                  <button class="btn primary" disabled={!canSend || sending} onclick={send} title={fromId ? t("Send to {name} through {pc}", { name: current.name, pc: from?.name ?? t("that PC") }) : t("Send to {name}", { name: current.name })}><Icon name="send" size={13} stroke={2.2} /><span class="to">{t("Send to {name}", { name: current.name })}</span></button>
                </div>
              </div>
              </div>
            {/if}
            <div class="droprow">
              <span class="dim">
                {#if fromId && !from?.online}{t("{pc} is offline. Drop files or folders here, or", { pc: from?.name ?? t("That PC") })}{:else if fromId && (!current.trusted || current.declined)}{t("{name} hasn't accepted {pc} yet. Drop files or folders here, or", { name: current.name, pc: from?.name ?? t("that PC") })}{:else if fromId && !current.online}{t("{pc} passes them on once {name} is back. Drop files or folders here, or", { pc: from?.name ?? t("That PC"), name: current.name })}{:else if fromId}{t("Drop files or folders here, or")}{:else if current.online}{t("Drop files or folders here, or")}{:else}{t("{name} is offline: files go only while you're both online. Drop files or folders here, or", { name: current.name })}{/if}
              </span>
              <button class="btn small" onclick={chooseFiles}><Icon name="file" size={12} />{t("Choose files")}</button>
              <button class="btn small" onclick={chooseDir}><Icon name="folder" size={12} />{t("Choose folder")}</button>
            </div>
          </footer>
        {/if}
      </section>
      {#if detailsInline && current}
        <div class="dside" class:open={showDetails} class:easing={detailsEasing && !panelWidths.dragging} aria-hidden={!showDetails} inert={!showDetails}>
          <PanelResizer page="direct" width={detailsW} avail={inlineRoom} />
          <DirectDetails contact={current} moving={movingNow} {speedOf} historyCount={endedWithThem} transfer={pickedInfo} onopen={(id, i, reveal) => app.run(() => openPart(id, i, reveal))} onletgo={letGo} ontags={() => openTags(current!)} onfolder={() => chooseFolder(current!)} />
        </div>
      {/if}
    </div>
    {/if}
    </div>
  {/if}
</div>

{#if app.directAdding}
  <Modal title={t("Add a contact")} width={480} onclose={() => (app.directAdding = false)}>
    <div class="form">
      {#if nameMissing}
        <label class="mlabel" for="addmyname">{t("Your name")}</label>
        <input id="addmyname" class="input" class:missing={needName} bind:this={nameField} bind:value={myName} placeholder={t("What your contacts see")} maxlength="60" aria-invalid={needName} />
        {#if needName}<p class="err" role="alert">{t("Fill in your name first: they see it when you add them.")}</p>{/if}
        <p class="dim small first">{askedOnce[0]}<b>{t("You, sending from")}</b>{askedOnce[1]}</p>
      {/if}
      <label class="mlabel" for="cid">{t("Their Pako ID")}</label>
      <input id="cid" class="input mono" bind:value={newId} placeholder={t("Paste the ID they sent you")} spellcheck="false" />
      <label class="mlabel" for="cname">{t("Their name")}</label>
      <input id="cname" class="input" bind:value={newName} placeholder={t("What you call them")} maxlength="60" onkeydown={(e) => e.key === "Enter" && addContact()} />
      {#if addError}<p class="err">{addError}</p>{/if}
      <p class="dim small">{t("The first time, they're asked to accept. After that you connect by yourselves whenever both Pakos are open.")}</p>
    </div>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (app.directAdding = false)}>{t("Cancel")}</button>
      <button class="btn primary" disabled={!newId.trim() || !newName.trim()} onclick={addContact}>{t("Add contact")}</button>
    {/snippet}
  </Modal>
{/if}

{#if renaming}
  <Modal title={t("Rename {name}", { name: renaming.name })} width={420} onclose={() => (renaming = null)}>
    <input class="input" bind:value={renameTo} maxlength="60" aria-label={t("New name")} onkeydown={(e) => e.key === "Enter" && rename()} />
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (renaming = null)}>{t("Cancel")}</button>
      <button class="btn primary" disabled={!renameTo.trim()} onclick={rename}>{t("Rename")}</button>
    {/snippet}
  </Modal>
{/if}

{#if tagging}
  {@const tc = tagging}
  <Modal title={t("Tags for {name}", { name: tc.name })} width={440} onclose={() => (tagging = null)}>
    <div class="tagpick">
      {#each tagChoices as tag, i (i)}
        {@const on = ticked.some((x) => x.toLowerCase() === tag.toLowerCase())}
        <button class="tagopt" class:on role="checkbox" aria-checked={on} onclick={() => toggleTag(tag)}>
          <span class="tick" aria-hidden="true">{#if on}<Icon name="check" size={11} stroke={3} />{/if}</span>{tag}
        </button>
      {/each}
    </div>
    <div class="newrow">
      <input class="input" bind:value={newTag} maxlength="30" placeholder={t("New tag")} aria-label={t("New tag")} onkeydown={(e) => e.key === "Enter" && addNewTag()} />
      <button class="btn" disabled={!newTag.trim()} onclick={() => addNewTag()}>{t("Add")}</button>
    </div>
    {#if !app.directTags.length}
      <div class="suggest">
        <span class="dim small">{t("Ideas:")}</span>
        {#each suggestedTags() as sug (sug)}
          <button class="sugchip" onclick={() => addNewTag(sug)}>{sug}</button>
        {/each}
      </div>
    {/if}
    <p class="dim small hintline">{t("Tags are only on this PC: {name} never sees them.", { name: tc.name })}</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (tagging = null)}>{t("Cancel")}</button>
      <button class="btn primary" onclick={saveTags}>{t("Save")}</button>
    {/snippet}
  </Modal>
{/if}

{#if clearing}
  {@const c = clearing}
  <Modal title={t("Clear your history with {name}?", { name: c.name })} width={440} onclose={() => (clearing = null)}>
    <p class="msgtext">{t("The list of what you sent and received with {name} is emptied on this PC and can't be brought back. The files themselves stay where they are, and {name} stays in your contacts.", { name: c.name })}</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (clearing = null)}>{t("Cancel")}</button>
      <button
        class="btn danger-fill"
        onclick={() => {
          clearing = null;
          app.run(() => api.directClearHistory(c.id));
        }}><Icon name="trash" size={13} />{t("Clear history")}</button
      >
    {/snippet}
  </Modal>
{/if}

{#if removing}
  {@const r = removing}
  <Modal title={t("Remove {name}?", { name: r.name })} width={460} onclose={() => (removing = null)}>
    <p class="msgtext">{t("You won't be connected any more. For 7 days Pako turns them away without asking you; after that, if they reach you again, you're asked to accept like the first time. Files already received stay on your PC.")}</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (removing = null)}>{t("Cancel")}</button>
      <button
        class="btn danger-fill"
        onclick={() => {
          removing = null;
          if (app.directPicked === r.id) app.directPicked = null;
          app.run(() => api.directRemoveContact(r.id));
        }}><Icon name="trash" size={13} />{t("Remove")}</button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .direct {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    height: 52px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .dim {
    color: var(--text-3);
  }
  .small {
    font-size: 12px;
  }
  .center {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    text-align: center;
    gap: 8px;
    padding: 24px;
  }
  .center .firewall {
    font-size: 12px;
  }
  .center p {
    max-width: 440px;
    margin: 0;
    line-height: 1.55;
  }
  .empty-icon {
    width: 52px;
    height: 52px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .stage {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
  }
  /* Both in one cell: while one fades out over the other, nothing moves. */
  .stage > .body {
    grid-area: 1 / 1;
    min-width: 0;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    padding: 12px 16px 16px;
    position: relative;
  }
  .body.solo > :global(*) {
    flex: 1;
  }
  .body > .people {
    flex: none;
    width: 280px;
    transition: width var(--slow) var(--ease);
  }
  .body.withdetails > .people {
    width: 260px;
  }
  .body > .chat {
    flex: 1;
    min-width: 0;
    margin-left: 12px;
  }
  /* The details column: its width opens and closes like Downloads' side panel. */
  .dside {
    position: relative;
    flex: none;
    width: 0;
    margin-left: 0;
    overflow: hidden;
    display: flex;
  }
  /* Opening or closing: width and gap ease (dragging the edge or resizing the window: at once). */
  .dside.easing {
    transition:
      width var(--slow) var(--ease),
      margin-left var(--slow) var(--ease);
  }
  .dside.open {
    width: var(--pw, 288px);
    margin-left: 12px;
  }
  .dside > :global(.details) {
    width: var(--pw, 288px);
    flex: none;
    opacity: 0;
    transform: translateX(12px);
    transition:
      opacity var(--mid) ease,
      transform var(--slow) var(--ease);
  }
  .dside.open > :global(.details) {
    opacity: 1;
    transform: none;
  }
  .welcome {
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 10px;
    padding: 32px;
    text-align: center;
  }
  .welcome h3 {
    margin: 4px 0 0;
    font-size: 15px;
  }
  .welcome p {
    max-width: 460px;
    margin: 0;
    line-height: 1.55;
  }
  .idbox {
    width: min(520px, 100%);
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 6px 0 4px;
    padding: 10px 12px;
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    background: var(--panel);
    text-align: left;
  }
  .phead {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 44px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--line);
  }
  .ptitle {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pcount {
    font-size: 11px;
    color: var(--text-3);
  }
  .example {
    padding: 1px 7px;
    border-radius: 9px;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 10.5px;
    font-weight: 600;
  }
  .psearch {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 8px 0;
    height: 30px;
    padding: 0 6px 0 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--panel);
    color: var(--text-3);
  }
  .psearch input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
  }
  .psearch:focus-within {
    border-color: var(--focus);
  }
  .psearch .clear {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: 4px;
    background: none;
    color: var(--text-3);
    cursor: pointer;
  }
  .psearch .clear:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
  .fav {
    display: inline-flex;
    margin-left: 6px;
    color: var(--st-retry);
    vertical-align: -1px;
  }
  .mutedmark {
    display: inline-flex;
    margin-left: 6px;
    color: var(--text-3);
    vertical-align: -1px;
  }
  .ptags {
    display: flex;
    gap: 4px;
    margin-top: 3px;
    min-width: 0;
    overflow: hidden;
  }
  .tchip {
    flex: none;
    max-width: 90px;
    height: 16px;
    padding: 0 6px;
    border-radius: 8px;
    background: var(--panel);
    border: 1px solid var(--line);
    color: var(--text-2);
    font-size: 10.5px;
    line-height: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tchip.more {
    color: var(--text-3);
  }
  .theirs {
    margin-left: 8px;
    font-size: 12px;
    font-weight: 400;
  }
  .tagpick {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tagopt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px 0 6px;
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    background: var(--raise);
    color: var(--text-2);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease,
      color var(--fast) ease;
  }
  .tagopt:hover:not(.on) {
    background: var(--raise-hover);
  }
  .tagopt.on {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--text);
  }
  .tick {
    width: 16px;
    height: 16px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--control);
    color: var(--accent-ink);
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease;
  }
  .tagopt.on .tick {
    background: var(--accent);
    border-color: var(--accent);
  }
  .newrow {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .newrow .input {
    flex: 1;
  }
  .suggest {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
  .sugchip {
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--line-strong);
    border-radius: 13px;
    background: var(--raise);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    transition: background var(--fast) ease;
  }
  .sugchip:hover {
    background: var(--raise-hover);
  }
  .hintline {
    margin: 12px 0 0;
  }
  .card {
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    min-height: 0;
  }
  /* ---------- people ---------- */
  .people {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .plist {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 6px;
  }
  .person {
    position: relative;
    overflow: hidden;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: background var(--fast) ease;
  }
  .person:hover {
    background: var(--raise-hover);
  }
  .person.on {
    background: var(--active);
  }
  .avatar {
    position: relative;
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--avatar-bg);
    box-shadow: inset 0 0 0 1px var(--avatar-ring);
    color: var(--text);
    font-weight: 600;
    font-size: 13px;
  }
  .avatar.big {
    width: 36px;
    height: 36px;
    font-size: 14px;
  }
  .dot {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--st-pause);
    border: 2px solid var(--raise);
    transition: background var(--mid) ease;
  }
  .dot.online {
    background: var(--st-done);
  }
  /* On the same network (blue): only the dot says it, the text stays as it is. */
  .dot.local {
    background: var(--st-run);
  }
  .dot.wait {
    background: var(--st-retry);
  }
  .ptext {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pname {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pstat {
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* A transfer with this contact: the live line, and a thin bar along the bottom of the row. */
  .pstat.live {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  .pstat.live :global(svg) {
    flex: none;
    color: var(--st-run);
  }
  .rowbar {
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 3px;
    height: 2px;
    border-radius: 1px;
    background: var(--track);
    overflow: hidden;
    pointer-events: none;
  }
  .rowbar i {
    display: block;
    height: 100%;
    background: var(--st-run);
    transform-origin: left center;
    transition: transform var(--mid) var(--ease-out);
  }
  .badge {
    margin-left: auto;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11px;
    font-weight: 700;
    display: grid;
    place-items: center;
  }
  .noone {
    padding: 16px 10px;
    font-size: 12.5px;
    line-height: 1.5;
  }
  .noone p {
    margin: 0 0 6px;
  }
  /* ---------- You, sending from ---------- */
  .from {
    position: relative;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 12px 11px;
    border-bottom: 1px solid var(--line);
  }
  /* Your name and the PC, one button: the name renames, the PC part opens the menu. */
  .fromsel {
    display: flex;
    align-items: center;
    min-width: 0;
    height: 32px;
    padding: 0 3px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius);
    background: var(--panel);
    transition: border-color var(--fast) ease;
  }
  .fromsel:hover {
    border-color: var(--control);
  }
  .fname,
  .fpc {
    height: 24px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0 7px;
    border: none;
    border-radius: calc(var(--radius) - 2px);
    background: transparent;
    color: var(--text);
    font-size: 13px;
    white-space: nowrap;
    transition: background var(--fast) ease;
  }
  .fname {
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 600;
    cursor: text;
  }
  .fname.empty {
    color: var(--text-3);
    font-weight: 400;
  }
  .fname:disabled {
    cursor: default;
    opacity: 1;
  }
  .fname:hover:not(:disabled),
  .fpc:hover {
    background: var(--raise-hover);
  }
  .fname-in {
    flex: 1;
    min-width: 0;
    height: 24px;
    padding: 0 7px;
    border: none;
    border-radius: calc(var(--radius) - 2px);
    outline: 1px solid var(--accent);
    background: var(--raise);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
  }
  .fsep {
    flex: none;
    color: var(--text-3);
  }
  .fpc {
    flex: 1 1 auto;
    min-width: 0;
    justify-content: space-between;
    color: var(--text-2);
    cursor: pointer;
  }
  .fpc.still {
    cursor: default;
  }
  .fromsel.remote .fpc {
    color: var(--text);
  }
  .fpcname {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fchev {
    flex: none;
    display: grid;
    color: var(--text-3);
    transition: transform var(--mid) var(--ease);
  }
  .fchev.open {
    transform: rotate(180deg);
  }
  .fcatch {
    position: fixed;
    inset: 0;
    z-index: 30;
  }
  .frommenu {
    position: fixed;
    z-index: 31;
    max-width: calc(100vw - 16px);
    padding: 5px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    background: var(--menu-bg);
    backdrop-filter: blur(12px);
    box-shadow: var(--shadow-menu);
    outline: none;
  }
  .fitem {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 8px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    text-align: left;
    transition: background 80ms ease;
  }
  .fitem:hover:not(:disabled) {
    background: var(--active);
  }
  .fitem.off {
    opacity: 0.45;
    cursor: default;
  }
  .fic {
    flex: none;
    display: grid;
    color: var(--text-2);
  }
  .ftext {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .ftop {
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fcalls {
    font-weight: 400;
    color: var(--text-3);
  }
  .fsub {
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--text-3);
  }
  .fok {
    flex: none;
    display: grid;
    color: var(--accent-text);
  }
  .fnote {
    margin: 4px 8px 2px;
    padding-top: 6px;
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--text-3);
  }
  /* "from Laptop" / "through Home PC" on a card. */
  .via {
    margin-left: 6px;
    padding: 0 6px;
    border-radius: 8px;
    border: 1px solid var(--line-strong);
    color: var(--text-2);
    font-size: 10.5px;
    line-height: 14px;
    white-space: nowrap;
  }
  .keep {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }
  .me {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border-top: 1px solid var(--line);
  }
  .mlabel {
    font-size: 11px;
    color: var(--text-3);
  }
  .idrow {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  /* "Be visible to Pakos on this network": on its own at the bottom now. */
  label[for="seenby"] {
    margin-top: 0;
  }
  .seennote {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-2);
  }
  .idtext {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* ---------- the chat ---------- */
  .chat {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .chead {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--line);
  }
  .ctext {
    min-width: 0;
  }
  .cname {
    font-weight: 600;
  }
  .cstat {
    font-size: 12px;
  }
  .safety {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 12px 0;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line-soft);
    font-size: 12.5px;
    color: var(--text-2);
  }
  .safety b {
    color: var(--text);
    letter-spacing: 0.04em;
  }
  /* the conversation, and the details panel over it on a narrow window */
  .mid {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  .timeline {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
  }
  .nothing {
    margin: auto;
    font-size: 12.5px;
  }
  .msg {
    display: flex;
  }
  .msg.out {
    justify-content: flex-end;
  }
  .bubble {
    width: min(420px, 82%);
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    background: var(--bubble);
    border: 1px solid var(--bubble-line);
    box-shadow: var(--bubble-shadow);
  }
  .msg.out .bubble {
    background: var(--bubble-out);
  }
  .msg.bad .bubble {
    opacity: 0.8;
  }
  .mtop {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .mtop :global(.mname) {
    font-weight: 500;
    min-width: 0;
  }
  /* A card can be picked (its files show in the details panel). */
  .bubble {
    cursor: default;
    transition:
      border-color var(--fast) ease,
      box-shadow var(--fast) ease;
    --ti-hover: var(--active);
    --ti-meta: var(--bubble-meta);
  }
  .bubble.picked {
    border-color: var(--focus);
    box-shadow:
      0 0 0 1px var(--focus),
      var(--bubble-shadow);
  }
  .chev {
    flex: none;
    width: 22px;
    height: 22px;
    margin: -2px -4px -2px 0;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--bubble-meta);
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .chev:hover {
    background: var(--active);
    color: var(--text);
  }
  .chev :global(svg) {
    transition: transform var(--mid) var(--ease);
  }
  .chev.open :global(svg) {
    transform: rotate(180deg);
  }
  /* What was in it, unfolded under the card's foot. */
  .mlist {
    margin: 8px -4px 0 18px;
    padding-top: 6px;
    border-top: 1px solid var(--bubble-line);
  }
  .mlist :global(.tis) {
    max-height: 240px;
    overflow-y: auto;
  }
  .mmeta {
    margin: 4px 0 0 26px;
    font-size: 12px;
    color: var(--bubble-meta);
  }
  .mfoot .dim {
    color: var(--bubble-meta);
  }
  .mfoot {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 10px;
    margin: 6px 0 0 26px;
    font-size: 12px;
  }
  .res {
    color: var(--st-fail-text);
    font-weight: 500;
  }
  .res.ok {
    color: var(--delivered-text);
  }
  .acts {
    display: flex;
    gap: 10px;
    margin-left: auto;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--focus-text);
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .mfoot .link:last-child:not(.acts .link) {
    margin-left: auto;
  }
  .bar {
    height: 4px;
    margin: 8px 0 0 26px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--st-run);
    transition:
      width var(--mid) var(--ease-out),
      background-color var(--mid) var(--ease-out);
  }
  /* Cut short, waiting for the other PC: the bar rests in the paused colour. */
  .bar.waiting i {
    background: var(--st-pause);
  }
  .compose {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-top: 1px solid var(--line);
    background: var(--panel);
  }
  .items {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 260px;
    height: 28px;
    padding: 0 6px 0 8px;
    border-radius: var(--radius);
    background: var(--raise);
    border: 1px solid var(--line);
    font-size: 12.5px;
  }
  .chip :global(svg) {
    flex: none;
  }
  .cn {
    display: flex;
    min-width: 0;
    white-space: nowrap;
  }
  .cn .head {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cn .tail {
    flex: none;
  }
  .chip button {
    flex: none;
    border: none;
    background: none;
    color: var(--text-3);
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .chip button:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
  .opts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .go {
    flex: none;
    display: flex;
    gap: 8px;
    margin-left: auto;
    min-width: 0;
    max-width: 100%;
  }
  .go .btn.primary {
    min-width: 0;
  }
  .go .to {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .basket {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .input.name {
    flex: 1 1 140px;
    min-width: 120px;
    max-width: 220px;
  }
  .droprow {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }
  .btn.small {
    height: 28px;
    font-size: 12px;
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .icon-btn:hover {
    background: var(--raise-hover);
    color: var(--text);
  }
  .icon-btn.on {
    color: var(--accent-strong);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form p {
    margin: 6px 0 0;
    line-height: 1.5;
  }
  /* Your name (asked once) stands apart from the contact's own fields. */
  .form p.first {
    margin: 2px 0 10px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--line);
  }
  .input.missing {
    border-color: var(--st-fail);
  }
  .err {
    color: var(--st-fail-text);
    font-size: 12.5px;
  }
  .msgtext {
    margin: 0;
    line-height: 1.55;
    color: var(--text-2);
  }
  .dropzone {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    background: var(--drop-scrim);
    border: 2px dashed var(--accent);
    pointer-events: none;
  }
  .dropcard {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 18px;
    border-radius: var(--radius-lg);
    background: var(--raise);
    border: 1px solid var(--line-strong);
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
  }
  .dropcard :global(svg) {
    color: var(--accent);
  }
  @container (max-width: 760px) {
    .sub {
      display: none;
    }
  }
</style>
