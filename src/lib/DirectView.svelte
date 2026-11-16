<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Direct: a messaging app for files. The sidebar filters contacts (Show, Tags); here, the
  // contacts under that filter (searchable, scrolling, your name and ID at the bottom), and the
  // picked contact's conversation: both directions, history kept only on this PC, with a send
  // area at the bottom that sends straight to them while they're online.
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api, listen, type DirectContact, type DirectHistory, type DirectTransfer } from "./api";
  import type { MenuItem } from "./menu";
  import { bytes, shortDate, speed } from "./format";
  import Icon from "./Icon.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import Modal from "./Modal.svelte";
  import DirectDetails from "./DirectDetails.svelte";
  import SegPick from "./SegPick.svelte";
  import { initial, pathLabel, percent, statusOf, SUGGESTED_TAGS } from "./direct";
  import { fadeIn, flipIf, flipOpts, pop, rowSlide } from "./motion";

  let { onmenu }: { onmenu: (x: number, y: number, items: MenuItem[]) => void } = $props();

  const d = $derived(app.directView);
  /** Everyone, for "do you have contacts at all". */
  const everyone = $derived(app.directContacts);
  // Search inside the current filter (shown once the list gets long, or while typing).
  let search = $state("");
  const searching = $derived(everyone.length > 8 || search.length > 0);
  const contacts = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return app.directShown;
    return app.directShown.filter((c) => c.name.toLowerCase().includes(q) || (c.theirName ?? "").toLowerCase().includes(q) || (c.tags ?? []).some((t) => t.toLowerCase().includes(q)));
  });
  const current = $derived(contacts.find((c) => c.id === app.directPicked) ?? contacts[0] ?? null);
  const FILTER_TITLES: Record<string, string> = { all: "All contacts", online: "Online", waiting: "Waiting for you", favourites: "Favourites" };
  const filterTitle = $derived(FILTER_TITLES[app.directFilter] ?? app.directFilter.replace(/^tag:/, ""));

  // ---------- the timeline: history plus what's moving, oldest first ----------
  type Row = { kind: "history"; h: DirectHistory } | { kind: "moving"; t: DirectTransfer };
  const rows = $derived.by<Row[]>(() => {
    if (!current) return [];
    const hist = (d?.history ?? []).filter((h) => h.contact === current.id).map((h) => ({ kind: "history" as const, h }));
    const moving = (d?.transfers ?? []).filter((t) => t.contact === current.id).map((t) => ({ kind: "moving" as const, t }));
    return [...hist, ...moving];
  });
  // The details panel (shown or hidden with Ctrl+I, like Downloads').
  const showDetails = $derived(app.panelOpen && !!current);
  // A column of its own when there's room; on a narrower window it slides over the conversation.
  let bodyWidth = $state(0);
  const detailsInline = $derived(bodyWidth >= 1040);
  const movingNow = $derived(current ? (d?.transfers ?? []).filter((t) => t.contact === current.id) : []);
  const endedWithThem = $derived(current ? (d?.history ?? []).filter((h) => h.contact === current.id).length : 0);
  let timeline: HTMLDivElement | undefined = $state();
  /** Whose conversation was last scrolled: a new item glides into view, a new contact opens at the end. */
  let scrolledFor: string | null = null;
  const reducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  $effect(() => {
    // Newest at the bottom, kept in view.
    void rows.length;
    const id = current?.id ?? null;
    const glide = id === scrolledFor && !reducedMotion();
    scrolledFor = id;
    queueMicrotask(() => timeline?.scrollTo({ top: timeline.scrollHeight, behavior: glide ? "smooth" : "auto" }));
  });
  /** Rows animate in and out, and move to their new place; not on very long lists. */
  const listMotion = $derived(contacts.length <= 60 ? 220 : 0);

  // Live speed per transfer, from how much moved since the last update.
  let lastDone: Record<string, { at: number; done: number; speed: number }> = {};
  function speedOf(t: DirectTransfer) {
    const now = performance.now();
    const prev = lastDone[t.id];
    if (!prev) {
      lastDone[t.id] = { at: now, done: t.done, speed: 0 };
      return 0;
    }
    if (now - prev.at > 400) {
      const s = ((t.done - prev.done) * 1000) / (now - prev.at);
      prev.speed = prev.speed ? prev.speed * 0.6 + s * 0.4 : s;
      prev.at = now;
      prev.done = t.done;
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
    const flowing = list.filter((t) => t.state === "sending" || t.state === "receiving");
    const out = list.some((t) => t.outgoing);
    const inn = list.some((t) => !t.outgoing);
    const verb = out && inn ? "Sending and receiving" : out ? "Sending" : "Receiving";
    const pct = batchPercent(contact);
    let text: string;
    if (!flowing.length) text = list.some((t) => t.state === "checking") ? "Checking the files…" : "Getting ready…";
    else {
      const rate = flowing.reduce((a, t) => a + speedOf(t), 0);
      text = `${verb} ${pct}%${rate > 0 ? " · " + speed(rate) : ""}`;
    }
    return { text, pct, icon: out && !inn ? "up" : !out && inn ? "down" : "send", count: list.length };
  }

  const plural = (n: number, one: string, many = `${one}s`) => `${n.toLocaleString()} ${n === 1 ? one : many}`;
  function resultText(h: DirectHistory) {
    if (h.result === "done") return h.outgoing ? "Delivered" : "Received";
    if (h.result === "declined") return "Declined";
    if (h.result === "cancelled") return "Cancelled";
    return h.message ?? "Didn't go through";
  }
  /** Opens a received file; a program or script is shown in its folder instead. */
  async function openReceived(id: string) {
    if (await api.directOpen(id, false)) app.toast("Programs and scripts aren't started from Pako. It's shown in its folder: open it there if you trust it.");
  }
  function movingText(t: DirectTransfer) {
    if (t.state === "offered") return "Waiting for them to accept…";
    if (t.state === "preparing") return "Getting ready…";
    if (t.state === "checking") return "Checking the files…";
    return `${percent(t.done, t.size)}% · ${bytes(t.done)} of ${bytes(t.size)}`;
  }

  // ---------- your ID and name ----------
  let myName = $state("");
  let nameSeeded = false;
  $effect(() => {
    if (!nameSeeded && d?.ready) {
      myName = d.myName ?? "";
      nameSeeded = true;
    }
  });
  async function saveName() {
    if ((d?.myName ?? "") === myName.trim()) return;
    await app.run(() => api.directSetName(myName.trim()));
  }
  function copyId() {
    if (d?.myId) app.run(() => api.copyText(d.myId!)).then(() => app.toast("Your Pako ID is copied. Send it to the people you want to connect with."));
  }
  const shortId = (id: string) => `${id.slice(0, 6)}…${id.slice(-6)}`;

  // ---------- add a contact (also asked from the sidebar) ----------
  let newId = $state("");
  let newName = $state("");
  let addError = $state("");
  async function addContact() {
    addError = "";
    try {
      await api.directAddContact(newId.trim(), newName.trim());
      app.directAdding = false;
      newId = "";
      newName = "";
      app.toast("Added. Pako is reaching them now; the first time, they're asked to accept.");
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

  function contactMenu(e: MouseEvent, c: DirectContact, atMouse = false) {
    const b = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (atMouse) e.preventDefault();
    onmenu(atMouse ? e.clientX : b.right - 240, atMouse ? e.clientY : b.bottom + 4, [
      { label: "Rename…", action: () => ((renaming = c), (renameTo = c.name)) },
      { label: "Tags…", action: () => openTags(c) },
      { label: c.favourite ? "Remove from favourites" : "Add to favourites", action: () => app.run(() => api.directSetFavourite(c.id, !c.favourite)) },
      { separator: true },
      { label: "Can send to me without asking", hint: c.autoAccept ? "✓" : "", action: () => app.run(() => api.directEditContact(c.id, { autoAccept: !c.autoAccept })) },
      { label: "Save their files in…", action: () => chooseFolder(c) },
      { separator: true },
      { label: "Clear history", action: () => app.run(() => api.directClearHistory(c.id)) },
      { label: "Remove contact", danger: true, action: () => (removing = c) },
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
  function toggleTag(t: string) {
    ticked = ticked.some((x) => x.toLowerCase() === t.toLowerCase()) ? ticked.filter((x) => x.toLowerCase() !== t.toLowerCase()) : [...ticked, t];
  }
  function addNewTag(t = newTag) {
    const name = t.trim();
    if (!name) return;
    if (!ticked.some((x) => x.toLowerCase() === name.toLowerCase())) ticked = [...ticked, name];
    newTag = "";
  }
  /** Your tags plus new ones typed here (not saved until Save). */
  const tagChoices = $derived([...app.directTags, ...ticked.filter((t) => !app.directTags.some((x) => x.toLowerCase() === t.toLowerCase()))]);
  async function saveTags() {
    if (!tagging) return;
    const c = tagging;
    tagging = null;
    await app.run(() => api.directSetTags(c.id, ticked));
  }
  async function chooseFolder(c: DirectContact) {
    const r = await pick({ directory: true, multiple: false, title: `Where ${c.name}'s files go` });
    if (typeof r === "string") await app.run(() => api.directEditContact(c.id, { folder: r }));
  }

  // ---------- the send area ----------
  let basket = $state<string[]>([]);
  let compress = $state(false);
  let sendName = $state("");
  let sending = $state(false);
  const nameOf = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;
  function addPaths(paths: string[]) {
    const have = new Set(basket.map((p) => p.toLowerCase()));
    basket = [...basket, ...paths.filter((p) => !have.has(p.toLowerCase()))];
  }
  async function chooseFiles() {
    const r = await pick({ multiple: true, directory: false, title: "Choose files to send" });
    addPaths(Array.isArray(r) ? r : r ? [r] : []);
  }
  async function chooseDir() {
    const r = await pick({ multiple: true, directory: true, title: "Choose folders to send" });
    addPaths(Array.isArray(r) ? r : r ? [r] : []);
  }
  async function send() {
    if (!current || !basket.length || sending) return;
    sending = true;
    try {
      await api.directSend(current.id, basket, compress, sendName.trim() || null);
      basket = [];
      sendName = "";
      compress = false;
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
      app.toast("Dropping files doesn't work here. Use Choose files.", "error");
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
      else app.toast(`Pako couldn't read what was dropped (${e.payload.detail}).`, "error");
    });
    return () => {
      un.then((f) => f());
    };
  });
</script>

<div class="direct" role="presentation" ondragenter={dragEnter} ondragover={dragOver} ondragleave={dragLeave} ondrop={drop}>
  {#if dropping && current}
    <div class="dropzone" aria-hidden="true" transition:fadeIn={{ duration: 120 }}>
      <div class="dropcard"><Icon name="send" size={22} />Drop files or folders to send them to {current.name}</div>
    </div>
  {/if}
  <div class="toolbar">
    <h1>Direct</h1>
    <TutorialButton id="direct" />
    <span class="dim small sub">Files straight to people you know, PC to PC</span>
  </div>

  {#if !d}
    <div class="center dim">Starting…</div>
  {:else if !d.ready}
    <div class="center dim">{d.error ?? "Direct is starting…"}</div>
  {:else}
    <!-- The welcome card and the contacts crossfade in the same place (the tutorial's example
         people going away, your first contact arriving): no jump. -->
    <div class="stage">
    {#if !everyone.length}
      <div class="body solo" transition:fadeIn={{ duration: 220 }}>
        <section class="card welcome" data-tour="direct-welcome">
          <div class="empty-icon"><Icon name="send" size={26} /></div>
          <h3>Send files straight to people you know</h3>
          <p class="dim">Files go PC to PC, encrypted, with nothing stored anywhere. Share your Pako ID with a friend, or add theirs: the first time they're asked to accept, then you connect by yourselves.</p>
          <div class="idbox">
            <span class="mlabel">Your Pako ID</span>
            <div class="idrow">
              <span class="mono idtext" title={d.myId}>{d.myId}</span>
              <button class="btn small" onclick={copyId}><Icon name="copy" size={12} />Copy</button>
            </div>
          </div>
          <button class="btn primary" onclick={() => ((app.directAdding = true), (addError = ""))}><Icon name="plus" size={14} stroke={2.2} />Add contact</button>
        </section>
      </div>
    {:else}
    <div class="body" class:withdetails={showDetails && detailsInline} bind:clientWidth={bodyWidth} transition:fadeIn={{ duration: 220 }}>
      <aside class="people card" data-tour="direct-people">
        <div class="phead">
          <span class="ptitle">{filterTitle}</span>
          <span class="pcount mono">{contacts.length}</span>
          {#if app.directExample}
            <span class="example" title="Made-up people, only shown during the tutorial" transition:pop={{ duration: 200, from: 0.92, y: 0 }}>Examples</span>
          {/if}
          <div class="spacer"></div>
          <button class="btn small" data-tour="direct-add" onclick={() => ((app.directAdding = true), (addError = ""))} title="Add a contact by their Pako ID"><Icon name="plus" size={12} stroke={2.2} />Add</button>
        </div>
        {#if searching}
          <div class="psearch" transition:rowSlide={{ duration: 200 }}>
            <Icon name="search" size={13} />
            <input bind:value={search} placeholder="Search contacts" aria-label="Search contacts" spellcheck="false" />
            {#if search}<button class="clear" aria-label="Clear the search" onclick={() => (search = "")}><Icon name="x" size={11} stroke={2.4} /></button>{/if}
          </div>
        {/if}
        <div class="plist">
          {#each contacts as c (c.id)}
            {@const mv = movingBy.get(c.id)}
            {@const m = mv ? rowMoving(c.id, mv) : null}
            <button
              class="person"
              class:on={current?.id === c.id}
              class:moving={!!m}
              onclick={() => (app.directPicked = c.id)}
              oncontextmenu={(e) => contactMenu(e, c, true)}
              transition:rowSlide={{ duration: listMotion }}
              animate:flipIf={{ ...flipOpts, duration: listMotion }}
            >
              <span class="avatar">{initial(c.name)}<span class="dot" class:online={c.online} class:wait={c.waiting}></span></span>
              <span class="ptext">
                <span class="pname">{c.name}{#if c.favourite}<span class="fav" aria-label="Favourite"><Icon name="star" size={11} stroke={2.2} /></span>{/if}</span>
                {#if m}
                  <span class="pstat live" title={m.count > 1 ? `${m.count} transfers with ${c.name}` : undefined}><Icon name={m.icon} size={11} stroke={2.4} />{m.text}</span>
                {:else}
                  <span class="pstat">{statusOf(c)}</span>
                {/if}
                {#if c.tags?.length}
                  <span class="ptags">
                    {#each c.tags.slice(0, 2) as t (t)}<span class="tchip">{t}</span>{/each}
                    {#if c.tags.length > 2}<span class="tchip more">+{c.tags.length - 2}</span>{/if}
                  </span>
                {/if}
              </span>
              {#if app.directWaitingIds.has(c.id)}<span class="badge" title="Waiting for your answer" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>!</span>{/if}
              {#if m}
                <span class="rowbar" aria-hidden="true" transition:fadeIn={{ duration: 200 }}><i style:transform="scaleX({m.pct / 100})"></i></span>
              {/if}
            </button>
          {:else}
            <div class="noone">
              {#if search}
                <p>Nobody matches "{search}".</p>
              {:else}
                <p class="dim">Nobody here right now.</p>
              {/if}
            </div>
          {/each}
        </div>
        <div class="me" data-tour="direct-me">
          <label class="mlabel" for="myname">Your name</label>
          <input id="myname" class="input" bind:value={myName} onblur={saveName} onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()} placeholder="What your contacts see" maxlength="60" />
          <span class="mlabel">Your Pako ID</span>
          <div class="idrow">
            <span class="mono idtext" title={d.myId}>{shortId(d.myId ?? "")}</span>
            <button class="btn small" onclick={copyId}><Icon name="copy" size={12} />Copy</button>
          </div>
        </div>
      </aside>

      <section class="chat card" data-tour="direct-chat">
        {#if !current}
          <div class="center">
            <p class="dim">{search ? "Nobody matches your search." : "Nobody under this filter. Pick another one on the left."}</p>
          </div>
        {:else}
          <header class="chead">
            <span class="avatar big">{initial(current.name)}<span class="dot" class:online={current.online} class:wait={current.waiting}></span></span>
            <div class="ctext">
              <div class="cname">
                {current.name}
                {#if current.theirName && current.theirName !== current.name}<span class="theirs dim">calls themselves "{current.theirName}"</span>{/if}
              </div>
              <div class="cstat dim">{statusOf(current)}{#if current.autoAccept}{" · can send without asking"}{/if}</div>
            </div>
            <div class="spacer"></div>
            {#if !current.online}
              <button class="btn small" onclick={() => app.run(() => api.directConnect(current!.id))}><Icon name="retry" size={12} />Connect</button>
            {/if}
            <button class="icon-btn" aria-label="More for {current.name}" title="More" onclick={(e) => contactMenu(e, current!)}><Icon name="settings" size={14} /></button>
            <button
              class="icon-btn"
              class:on={app.panelOpen}
              aria-label={app.panelOpen ? "Hide details (Ctrl+I)" : "Show details (Ctrl+I)"}
              title={app.panelOpen ? "Hide details (Ctrl+I)" : "Show details (Ctrl+I)"}
              onclick={() => app.togglePanel()}
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
                <rect x="3" y="5" width="18" height="14" rx="1.5" />
                <path d="M15 5v14" />
                {#if app.panelOpen}<path d="M15 5h6v14h-6z" fill="currentColor" stroke="none" />{/if}
              </svg>
            </button>
          </header>

          {#if current.declined && !current.waiting}
            <div class="safety" transition:rowSlide={{ duration: 220 }}>
              <Icon name="alert" size={14} />
              <span>{current.name} declined. For 7 days their Pako turns you away without asking them; after that, press Connect to ask again.</span>
            </div>
          {:else if current.waiting || !current.trusted}
            <div class="safety" transition:rowSlide={{ duration: 220 }}>
              <Icon name="alert" size={14} />
              <span>The first time, {current.name} is asked to accept. Compare this code with theirs once (by voice or chat): <b class="mono">{current.code}</b></span>
            </div>
          {/if}

          <div class="mid">
          {#key current.id}
          <div class="timeline" bind:this={timeline} in:fadeIn={{ duration: 160 }}>
            {#each rows as r (r.kind === "history" ? `h:${r.h.outgoing ? "out" : "in"}:${r.h.id}` : `t:${r.t.outgoing ? "out" : "in"}:${r.t.id}`)}
              {#if r.kind === "history"}
                {@const h = r.h}
                <div class="msg" class:out={h.outgoing} class:bad={h.result !== "done"} in:pop={{ duration: 240, from: 0.98, y: 8, origin: h.outgoing ? "right bottom" : "left bottom" }}>
                  <div class="bubble">
                    <div class="mtop">
                      <FileIcon name={h.name} kind={h.files > 1 ? "folder" : undefined} size={18} />
                      <MiddleText text={h.name} class="mname" />
                    </div>
                    <div class="mmeta">{plural(h.files, "file")} · {bytes(h.size)}</div>
                    <div class="mfoot">
                      <span class="res" class:ok={h.result === "done"}>{resultText(h)}</span>
                      <span class="dim">{shortDate(h.date)}{#if h.path}{" · " + pathLabel(h.path)}{/if}</span>
                      {#if !h.outgoing && h.result === "done" && h.savedTo}
                        <span class="acts">
                          <button class="link" onclick={() => app.run(() => openReceived(h.id))}>Open</button>
                          <button class="link" onclick={() => app.run(() => api.directOpen(h.id, true))}>Folder</button>
                        </span>
                      {/if}
                    </div>
                  </div>
                </div>
              {:else}
                {@const t = r.t}
                <div class="msg live" class:out={t.outgoing} in:pop={{ duration: 240, from: 0.98, y: 8, origin: t.outgoing ? "right bottom" : "left bottom" }}>
                  <div class="bubble">
                    <div class="mtop">
                      <FileIcon name={t.name} kind={t.files > 1 ? "folder" : undefined} size={18} />
                      <MiddleText text={t.name} class="mname" />
                    </div>
                    <div class="mmeta">{plural(t.files, "file")} · {bytes(t.size)}</div>
                    <div class="bar"><i style:width="{Math.min(100, (t.done / Math.max(1, t.size)) * 100)}%"></i></div>
                    <div class="mfoot">
                      <span class="dim">{movingText(t)}{#if t.state === "sending" || t.state === "receiving"}{" · " + speed(speedOf(t))}{/if}{#if t.path}{" · " + pathLabel(t.path)}{/if}</span>
                      <button class="link" onclick={() => app.run(() => api.directCancel(t.id))}>Cancel</button>
                    </div>
                  </div>
                </div>
              {/if}
            {:else}
              <div class="nothing dim">Nothing sent or received with {current.name} yet.</div>
            {/each}
          </div>
          {/key}
          {#if showDetails && !detailsInline}
            <DirectDetails drawer contact={current} moving={movingNow} {speedOf} historyCount={endedWithThem} ontags={() => openTags(current!)} onfolder={() => chooseFolder(current!)} />
          {/if}
          </div>

          <footer class="compose" data-tour="direct-send">
            {#if basket.length}
              <div class="basket" transition:rowSlide={{ duration: 220 }}>
              <div class="items">
                {#each basket as p (p)}
                  <span class="chip" transition:pop={{ duration: 180, from: 0.9, y: 0 }} animate:flipIf={flipOpts}><FileIcon name={nameOf(p)} kind={/\.[a-z0-9]{1,5}$/i.test(p) ? undefined : "folder"} size={14} /><MiddleText text={nameOf(p)} /><button aria-label="Remove {nameOf(p)}" onclick={() => (basket = basket.filter((x) => x !== p))}><Icon name="x" size={10} stroke={2.4} /></button></span>
                {/each}
              </div>
              <div class="opts">
                <SegPick
                  label="Compression"
                  options={[
                    { value: "plain", label: "As they are" },
                    { value: "zip", label: "Compressed ZIP" },
                  ]}
                  value={compress ? "zip" : "plain"}
                  onpick={(v) => (compress = v === "zip")}
                />
                <input class="input name" bind:value={sendName} placeholder="Name (optional)" maxlength="120" aria-label="Name of this send" />
                <div class="spacer"></div>
                <button class="btn ghost" onclick={() => (basket = [])}>Clear</button>
                <button class="btn primary" disabled={!current.online || sending} onclick={send}><Icon name="send" size={13} stroke={2.2} />Send to {current.name}</button>
              </div>
              </div>
            {/if}
            <div class="droprow">
              <span class="dim">
                {#if current.online}Drop files or folders here, or{:else}{current.name} is offline: files go only while you're both online. Drop files or folders here, or{/if}
              </span>
              <button class="btn small" onclick={chooseFiles}><Icon name="file" size={12} />Choose files</button>
              <button class="btn small" onclick={chooseDir}><Icon name="folder" size={12} />Choose folder</button>
            </div>
          </footer>
        {/if}
      </section>
      {#if detailsInline && current}
        <div class="dside" class:open={showDetails} aria-hidden={!showDetails} inert={!showDetails}>
          <DirectDetails contact={current} moving={movingNow} {speedOf} historyCount={endedWithThem} ontags={() => openTags(current!)} onfolder={() => chooseFolder(current!)} />
        </div>
      {/if}
    </div>
    {/if}
    </div>
  {/if}
</div>

{#if app.directAdding}
  <Modal title="Add a contact" width={480} onclose={() => (app.directAdding = false)}>
    <div class="form">
      <label class="mlabel" for="cid">Their Pako ID</label>
      <input id="cid" class="input mono" bind:value={newId} placeholder="Paste the ID they sent you" spellcheck="false" />
      <label class="mlabel" for="cname">Name</label>
      <input id="cname" class="input" bind:value={newName} placeholder="What you call them" maxlength="60" onkeydown={(e) => e.key === "Enter" && addContact()} />
      {#if addError}<p class="err">{addError}</p>{/if}
      <p class="dim small">The first time, they're asked to accept. After that you connect by yourselves whenever both Pakos are open.</p>
    </div>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (app.directAdding = false)}>Cancel</button>
      <button class="btn primary" disabled={!newId.trim() || !newName.trim()} onclick={addContact}>Add contact</button>
    {/snippet}
  </Modal>
{/if}

{#if renaming}
  <Modal title="Rename {renaming.name}" width={420} onclose={() => (renaming = null)}>
    <input class="input" bind:value={renameTo} maxlength="60" aria-label="New name" onkeydown={(e) => e.key === "Enter" && rename()} />
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (renaming = null)}>Cancel</button>
      <button class="btn primary" disabled={!renameTo.trim()} onclick={rename}>Rename</button>
    {/snippet}
  </Modal>
{/if}

{#if tagging}
  {@const tc = tagging}
  <Modal title="Tags for {tc.name}" width={440} onclose={() => (tagging = null)}>
    <div class="tagpick">
      {#each tagChoices as t (t)}
        {@const on = ticked.some((x) => x.toLowerCase() === t.toLowerCase())}
        <button class="tagopt" class:on role="checkbox" aria-checked={on} onclick={() => toggleTag(t)}>
          <span class="tick" aria-hidden="true">{#if on}<Icon name="check" size={11} stroke={3} />{/if}</span>{t}
        </button>
      {/each}
    </div>
    <div class="newrow">
      <input class="input" bind:value={newTag} maxlength="30" placeholder="New tag" aria-label="New tag" onkeydown={(e) => e.key === "Enter" && addNewTag()} />
      <button class="btn" disabled={!newTag.trim()} onclick={() => addNewTag()}>Add</button>
    </div>
    {#if !app.directTags.length}
      <div class="suggest">
        <span class="dim small">Ideas:</span>
        {#each SUGGESTED_TAGS as sug (sug)}
          <button class="sugchip" onclick={() => addNewTag(sug)}>{sug}</button>
        {/each}
      </div>
    {/if}
    <p class="dim small hintline">Tags are only on this PC: {tc.name} never sees them.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (tagging = null)}>Cancel</button>
      <button class="btn primary" onclick={saveTags}>Save</button>
    {/snippet}
  </Modal>
{/if}

{#if removing}
  {@const r = removing}
  <Modal title="Remove {r.name}?" width={460} onclose={() => (removing = null)}>
    <p class="msgtext">You won't be connected any more. For 7 days Pako turns them away without asking you; after that, if they reach you again, you're asked to accept like the first time. Files already received stay on your PC.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (removing = null)}>Cancel</button>
      <button
        class="btn danger-fill"
        onclick={() => {
          removing = null;
          if (app.directPicked === r.id) app.directPicked = null;
          app.run(() => api.directRemoveContact(r.id));
        }}><Icon name="trash" size={13} />Remove</button
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
    flex: none;
    width: 0;
    margin-left: 0;
    overflow: hidden;
    display: flex;
    transition:
      width var(--slow) var(--ease),
      margin-left var(--slow) var(--ease);
  }
  .dside.open {
    width: 288px;
    margin-left: 12px;
  }
  .dside > :global(.details) {
    width: 288px;
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
    background: var(--panel);
    border: 1px solid var(--line-soft);
  }
  .msg.out .bubble {
    background: var(--select);
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
  .mmeta {
    margin: 4px 0 0 26px;
    font-size: 12px;
    color: var(--text-3);
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
    color: var(--st-done-text);
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
    transition: width var(--mid) var(--ease-out);
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
  .chip button {
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
    align-items: center;
    gap: 8px;
  }
  .basket {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .input.name {
    width: 180px;
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
