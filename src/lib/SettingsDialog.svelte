<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts" module>
  /** Settings' tabs, in order: the window's list on the left. */
  export const SETTINGS_TABS = [
    { id: "updates", name: "Features & updates", tour: "set-updates" },
    { id: "browser", name: "Browser extension", tour: "set-browser" },
    { id: "accounts", name: "Accounts", tour: "set-accounts" },
    { id: "window", name: "Window", tour: "set-window" },
    { id: "notices", name: "Notifications", tour: "set-notices" },
    { id: "downloads", name: "Downloads", tour: "set-downloads" },
    { id: "private", name: "Private mode", tour: "set-private" },
    { id: "upload", name: "Upload", tour: "set-upload" },
  ] as const;
  export type SettingsTab = (typeof SETTINGS_TABS)[number]["id"];
</script>

<script lang="ts">
  import { onMount, tick, untrack } from "svelte";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api, listen, type Settings, type WhenDone, type SendMode, type PakoLinkResult, type DriveSendStatus, type PrivateKeep } from "./api";
  import { hhmm, minutesOf } from "./format";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import UpdatesSection from "./UpdatesSection.svelte";
  import AddToBrowser from "./AddToBrowser.svelte";
  import AdvancedFold from "./AdvancedFold.svelte";
  import SetGroup from "./SetGroup.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import HelpLink from "./help/HelpLink.svelte";
  import { help } from "./help/help.svelte";
  import SegPick from "./SegPick.svelte";
  import PartSize from "./PartSize.svelte";
  import { PARTS_NOTE, PART_DEFAULT_MB, PART_RANGE_NOTE, partOk } from "./uploads";
  import { fadeIn, pop, reduced, rowSlide } from "./motion";
  import { KINDS, prefOf, type NoticeKind } from "./notices";
  import { ALERTS, CHIMES, doneAlert, doneChime } from "./alert";
  import { theme } from "./theme.svelte";
  import { tour, targetsOf } from "./tour/tour.svelte";
  import type { ThemeChoice } from "./api";
  import { priv } from "./private.svelte";

  let {
    section,
    row,
    linkResult = null,
    onclose,
  }: {
    /** The tab to open at (else the first). */
    section?: SettingsTab;
    /** A setting to show in that tab: its title, as in the window (scrolled to and lit up briefly). */
    row?: string;
    /** Opened by the extension's "Open Pako" button: how connecting went. */
    linkResult?: PakoLinkResult | null;
    onclose: () => void;
  } = $props();

  /** Missing from settings saved before it existed: 10 minutes. */
  const withDefaults = (s: Settings): Settings => ({
    ...s,
    privateKeep: s.privateKeep ?? "10m",
    privateLockAfter: s.privateLockAfter ?? "5m",
    privateClearAfter: s.privateClearAfter ?? "never",
    privateRealNames: s.privateRealNames ?? false,
  });
  /** "Reset Private mode" asked: deletes everything in it, for good. */
  let resetting = $state(false);
  /** Private mode's list clears itself after: one of the choices, or "custom" (a number of hours). */
  const CLEAR_CHOICES = ["never", "3h", "12h", "3d"];
  const clearAtOpen = app.settings?.privateClearAfter ?? "never";
  let clearCustom = $state(!CLEAR_CHOICES.includes(clearAtOpen));
  let clearHours = $state(Number.parseInt(clearAtOpen, 10) || 24);
  let draft = $state<Settings>(withDefaults({ ...app.settings! }));
  /** The settings as they were when this opened: Save sends only what was changed here. */
  const opened = withDefaults($state.snapshot(app.settings!) as Settings);
  /** Theme: shows at once while picking; Save keeps it, Cancel goes back. */
  const openedTheme = theme.choice;
  let themePick = $state<ThemeChoice>(theme.choice);
  let themeKept = false;
  function pickTheme(t: ThemeChoice) {
    themePick = t;
    theme.preview(t);
  }
  function unpreview() {
    if (!themeKept && theme.choice !== openedTheme) theme.preview(openedTheme);
  }
  /** Cancel: a previewed theme goes back at once, while the window fades out (not after). */
  function cancel() {
    unpreview();
    onclose();
  }
  /** × or Esc: with changes not saved yet, ask first (a stray key never drops them). */
  let askingClose = $state(false);
  function tryClose() {
    if (Object.values(unsaved).some(Boolean)) askingClose = true;
    else cancel();
  }
  /** Ctrl+Shift+P (Private mode) wants the window clear: changes are saved first, never dropped. */
  async function saveAndClose() {
    askingClose = false;
    if (Object.values(unsaved).some(Boolean)) await save();
    else cancel();
  }
  // Closed some other way (a tutorial ending, the extension's "Open Pako"): back then.
  $effect(() => unpreview);

  // ---------- tabs ----------
  // svelte-ignore state_referenced_locally
  let tab = $state<SettingsTab>(section ?? "updates");
  /** The tab with the keyboard focus in the list (↑ ↓ move it, Enter or Space opens it). */
  // svelte-ignore state_referenced_locally
  let focusTab = $state<SettingsTab>(section ?? "updates");
  let root: HTMLDivElement | undefined = $state();
  let tabList: HTMLDivElement | undefined = $state();
  let pane: HTMLDivElement | undefined = $state();

  function show(t: SettingsTab) {
    focusTab = t;
    if (t === tab) return;
    tab = t;
    // Each tab starts at its top.
    if (pane) pane.scrollTop = 0;
  }
  function tabEl(t: SettingsTab) {
    return tabList?.querySelector<HTMLElement>(`#set-tab-${t}`) ?? null;
  }
  function step(by: number, from: SettingsTab) {
    const i = SETTINGS_TABS.findIndex((x) => x.id === from);
    return SETTINGS_TABS[(i + by + SETTINGS_TABS.length) % SETTINGS_TABS.length].id;
  }
  function tabKey(e: KeyboardEvent) {
    let to: SettingsTab | null = null;
    if (e.key === "ArrowDown") to = step(1, focusTab);
    else if (e.key === "ArrowUp") to = step(-1, focusTab);
    else if (e.key === "Home") to = SETTINGS_TABS[0].id;
    else if (e.key === "End") to = SETTINGS_TABS[SETTINGS_TABS.length - 1].id;
    if (!to) return;
    e.preventDefault();
    focusTab = to;
    tabEl(to)?.focus();
  }
  /** Ctrl+Tab / Ctrl+Shift+Tab, anywhere in Settings: the next or previous tab. */
  function ctrlTab(e: KeyboardEvent) {
    if (e.key !== "Tab" || !e.ctrlKey || !root) return;
    const dialog = root.closest<HTMLElement>('[role="dialog"]');
    const active = document.activeElement;
    if (!dialog || !(active === dialog || dialog.contains(active))) return;
    e.preventDefault();
    e.stopPropagation();
    const to = step(e.shiftKey ? -1 : 1, tab);
    show(to);
    tabEl(to)?.focus();
  }
  /** Leaving the list: the next Tab into it lands on the open tab. */
  function tabsLeft(e: FocusEvent) {
    if (!tabList?.contains(e.relatedTarget as Node | null)) focusTab = tab;
  }

  /** What Save would change, by tab: a dot on the tab. */
  const TAB_KEYS: Partial<Record<SettingsTab, (keyof Settings)[]>> = {
    updates: ["autoUpdate"],
    browser: ["browserConnect", "askBrowser", "miniOnBrowser"],
    window: ["minimizeToTray", "closeToTray", "confirmQuit", "startWithWindows", "startInTray"],
    notices: ["quiet", "followWindowsQuiet", "notices", "doneAlert", "doneChime", "playDoneSound", "playSentSound"],
    downloads: [
      "downloadDir", "autoConnections", "connections", "maxActive", "speedLimitKb", "extractDefault", "videoQuality", "previewQuality",
      "verifyChecksum", "resumeOnStart", "keepAwake", "scheduleOn", "scheduleStart", "scheduleEnd", "catchClipboard", "privateKeep",
    ],
    private: ["privateLockAfter", "privateClearAfter", "privateRealNames"],
    upload: ["sendHost", "sendMode", "sendPartMb", "sendPartCustom", "resumeUploadsOnStart"],
  };
  // A number field emptied and typed again ("8" vs 8) is no change.
  const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b) || (typeof a !== "object" && typeof b !== "object" && a !== "" && String(a) === String(b));
  const unsaved = $derived(
    Object.fromEntries(
      SETTINGS_TABS.map((t) => [
        t.id,
        (TAB_KEYS[t.id] ?? []).some((k) => !same(draft[k], opened[k])) || (t.id === "window" && themePick !== openedTheme),
      ]),
    ) as Record<SettingsTab, boolean>,
  );

  // ---------- showing one setting: its tab, its fold, scrolled to and lit up ----------
  let flashTimer: ReturnType<typeof setTimeout> | undefined;
  let flashed: HTMLElement | null = null;
  /** `focus`: the keyboard goes to the setting's own control too (picked from the search), so
   *  Tab, Space or a screen reader carry on from there instead of from the search box. */
  async function reach(el: HTMLElement, focus = false) {
    const t = el.closest<HTMLElement>("[data-tab]")?.dataset.tab as SettingsTab | undefined;
    if (t) show(t);
    await tick();
    // Opens the folds it is in (Advanced, What Pako can do), then waits for them to slide open.
    let opening = false;
    for (let f = el.closest<HTMLElement>("[data-fold]"); f; f = f.parentElement?.closest<HTMLElement>("[data-fold]") ?? null) {
      const b = [...f.querySelectorAll<HTMLElement>("[data-fold-toggle]")].find((x) => x.closest("[data-fold]") === f);
      if (b?.getAttribute("aria-expanded") === "false") {
        b.click();
        opening = true;
      }
    }
    if (opening) await new Promise((r) => setTimeout(r, reduced() ? 0 : 340));
    el.scrollIntoView({ block: "center", behavior: reduced() ? "auto" : "smooth" });
    if (focus) {
      // Its switch, box or button first (not the help link in its title); a greyed one can't
      // take the focus, which then stays in the search. A group: its first control, in order.
      const ok = ":is(button, input, select):not(:disabled)";
      const first = el.hasAttribute("data-group")
        ? [...el.querySelectorAll<HTMLElement>(ok)].find((c) => !c.closest(".t"))
        : (el.querySelector<HTMLElement>(`.ctrl ${ok}, .col ${ok}`) ?? el.querySelector<HTMLElement>(ok));
      first?.focus({ preventScroll: true });
    }
    clearTimeout(flashTimer);
    flashed?.classList.remove("set-flash");
    // Again from the start, also on the same row twice in a row.
    void el.offsetWidth;
    el.classList.add("set-flash");
    flashed = el;
    flashTimer = setTimeout(() => el.classList.remove("set-flash"), 1800);
  }
  $effect(() => () => clearTimeout(flashTimer));

  // ---------- search ----------
  /** A tab (`el` null), a group of a tab (`isGroup`), or one setting (`group`: its group's title). */
  type Hit = { el: HTMLElement | null; tab: SettingsTab; title: string; advanced: boolean; group: string; isGroup: boolean };
  let query = $state("");
  let hits = $state<Hit[]>([]);
  let hitAt = $state(0);
  let listOpen = $state(false);
  /** The words searched, kept while the list fades out after Esc. */
  let searched = $state("");
  const norm = (t: string) =>
    t
      .normalize("NFD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[’‘]/g, "'")
      .replace(/\s+/g, " ")
      .trim()
      .toLowerCase();
  /** Every setting as it shows in the window, read from the rows themselves (titles and their
   *  muted lines), in every tab and fold: the search can't drift from what's on screen. */
  const ROWS = ".line, .krow, .account, .feat, [data-search]";
  const TITLES = ".t, .klabel, .acc-name, .fname";
  const titleOf = (el: HTMLElement) => (el.dataset.search || el.querySelector(TITLES)?.textContent || "").replace(/\s+/g, " ").trim();
  function find(q: string): Hit[] {
    const words = norm(q).split(" ").filter(Boolean);
    if (!words.length || !root) return [];
    const found: { hit: Hit; score: number; order: number }[] = [];
    let order = 0;
    for (const t of SETTINGS_TABS) {
      const name = norm(t.name);
      if (words.every((w) => name.includes(w))) found.push({ hit: { el: null, tab: t.id, title: t.name, advanced: false, group: "", isGroup: false }, score: 0, order: order++ });
    }
    // The groups by their title ("Speed", "Quiet times"…).
    for (const el of root.querySelectorAll<HTMLElement>("[data-group]")) {
      const tabId = el.closest<HTMLElement>("[data-tab]")?.dataset.tab as SettingsTab | undefined;
      const title = el.dataset.group ?? "";
      if (!tabId || !title || !words.every((w) => norm(title).includes(w))) continue;
      found.push({ hit: { el, tab: tabId, title, advanced: !!el.closest(".adv"), group: title, isGroup: true }, score: 0.5, order: order++ });
    }
    for (const el of root.querySelectorAll<HTMLElement>(ROWS)) {
      const tabId = el.closest<HTMLElement>("[data-tab]")?.dataset.tab as SettingsTab | undefined;
      const title = titleOf(el);
      if (!tabId || !title) continue;
      const t = norm(title);
      const said = norm([...el.querySelectorAll(".muted")].map((m) => m.textContent ?? "").join(" "));
      if (!words.every((w) => t.includes(w) || said.includes(w))) continue;
      // Titles first (the ones that start with it before the others), then explanations.
      const score = t.startsWith(words.join(" ")) ? 1 : words.every((w) => t.includes(w)) ? 2 : 3;
      const group = el.closest<HTMLElement>("[data-group]")?.dataset.group ?? "";
      found.push({ hit: { el, tab: tabId, title, advanced: !!el.closest(".adv"), group, isGroup: false }, score, order: order++ });
    }
    return found
      .sort((a, b) => a.score - b.score || a.order - b.order)
      .slice(0, 8)
      .map((x) => x.hit);
  }
  function typed() {
    hits = find(query);
    hitAt = 0;
    listOpen = !!query.trim();
    if (listOpen) searched = query.trim();
  }
  function pick(h: Hit) {
    listOpen = false;
    if (h.el) reach(h.el, true);
    else {
      show(h.tab);
      tabEl(h.tab)?.focus();
    }
  }
  function searchKey(e: KeyboardEvent) {
    if (e.key === "Escape" && (query || listOpen)) {
      // Clears the search; the next Esc closes Settings.
      e.preventDefault();
      e.stopPropagation();
      query = "";
      typed();
    } else if (e.key === "ArrowDown" && hits.length) {
      e.preventDefault();
      if (listOpen) hitAt = (hitAt + 1) % hits.length;
      listOpen = true;
    } else if (e.key === "ArrowUp" && hits.length) {
      e.preventDefault();
      hitAt = (hitAt - 1 + hits.length) % hits.length;
    } else if (e.key === "Enter" && listOpen && hits[hitAt]) {
      e.preventDefault();
      pick(hits[hitAt]);
    }
  }
  const tabName = (t: SettingsTab) => SETTINGS_TABS.find((x) => x.id === t)?.name ?? "";

  // ---------- the tutorial: each Settings step shows its tab ----------
  const TOUR_TAB = Object.fromEntries(SETTINGS_TABS.map((t) => [t.tour, t.id])) as Record<string, SettingsTab>;
  $effect(() => {
    if (!tour.active) return;
    const s = tour.step;
    if (s?.view !== "settings") return;
    const t = targetsOf(s)
      .map((n) => TOUR_TAB[n])
      .find(Boolean);
    if (t) untrack(() => show(t));
  });

  onMount(() => {
    // Opened at one setting (its title, as in the window).
    if (!row) return;
    const want = norm(row);
    const el = [...(root?.querySelectorAll<HTMLElement>(ROWS) ?? [])].find((r) => norm(titleOf(r)) === want);
    if (el) setTimeout(() => reach(el), 120);
  });

  /** Writes the extension's files next to Pako's data and opens that folder. */
  async function openExtension() {
    await app.run(async () => api.openFolder(await api.extensionFiles()));
  }

  async function choose() {
    const picked = await pickFolder({ directory: true, defaultPath: draft.downloadDir || undefined });
    if (typeof picked === "string") draft.downloadDir = picked;
  }

  /** A whole number within limits (the engine refuses decimals and negatives). */
  function whole(v: unknown, fallback: number, min: number, max = Infinity) {
    const n = Math.round(Number(v));
    if (v == null || v === "" || !Number.isFinite(n)) return fallback;
    return Math.min(max, Math.max(min, n));
  }

  /** Save on its way: a second click does nothing. */
  let saving = false;
  /** Set once save() has saved and closed the window. */
  let savedOk = false;
  /** "Open Private mode…" (it's locked): Settings closes (saving any changes first, never
   *  dropping them), then the PIN pad opens over Downloads, as the Private entry does. */
  async function openPrivate() {
    if (Object.values(unsaved).some(Boolean)) {
      await save();
      if (!savedOk) return;
    } else {
      cancel();
    }
    priv.enter();
  }

  /** "Turn on…": its setup opens over Settings; changes not saved yet are saved first (and
   *  Settings closes), never dropped; if saving fails, the setup doesn't open. */
  async function turnOnPrivate() {
    if (Object.values(unsaved).some(Boolean)) {
      await save();
      if (!savedOk) return;
    }
    priv.setupOpen = true;
  }

  async function save() {
    if (saving) return;
    if (sendMode === "parts" && !partValid) {
      app.toast(`Pick a part size ${PART_RANGE_NOTE.toLowerCase()} (Upload › Advanced).`, "error");
      const el = root?.querySelector<HTMLElement>("[data-part-size]");
      if (el) reach(el, true);
      return;
    }
    saving = true;
    // What wasn't changed here keeps the value it has now, not the one copied at opening: "At
    // once" from the bottom bar, "Don't ask again" ticked in the Download with Pako window, where
    // uploads go after connecting Drive… all can change while this is open.
    const mine = $state.snapshot(draft) as Settings;
    const next = { ...(app.settings ? ($state.snapshot(app.settings) as Settings) : opened) };
    for (const k of Object.keys(mine) as (keyof Settings)[]) {
      if (JSON.stringify(mine[k]) !== JSON.stringify(opened[k])) (next as Record<string, unknown>)[k] = mine[k];
    }
    try {
      await api.setSettings({
        ...next,
        connections: whole(next.connections, 8, 1, 32),
        maxActive: whole(next.maxActive, 3, 1, 20),
        speedLimitKb: whole(next.speedLimitKb, 0, 0),
        sendPartMb: partOk(next.sendPartMb) ? next.sendPartMb : PART_DEFAULT_MB,
      });
      if (themePick !== openedTheme) await theme.set(themePick);
      themeKept = true;
      await app.refresh();
      app.toast("Settings saved");
      savedOk = true;
      onclose();
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      saving = false;
    }
  }

  // Google Drive for uploads: its own sign-in, which only sees the files Pako uploads.
  let sendDrive = $state<DriveSendStatus | null>(null);
  let sendConnecting = $state(false);
  /** The newest check wins: an older answer arriving late never overwrites a newer one. */
  let sendAsk = 0;
  async function loadSendDrive() {
    const mine = ++sendAsk;
    try {
      const st = await api.sendDriveStatus();
      if (mine === sendAsk) sendDrive = st;
    } catch {
      // Keep what is shown (an account just connected stays shown) rather than "Checking…".
      if (mine === sendAsk && !sendDrive) sendDrive = null;
    }
  }
  onMount(() => {
    loadSendDrive();
    // Connected or disconnected from the Send tab or first-start setup while Settings is open.
    // Connecting Drive makes it the default for uploads (disconnecting goes back to Gofile): the
    // open draft follows, so Save doesn't put the old choice back.
    const un = listen("send-changed", async () => {
      await loadSendDrive();
      const fresh = await api.snapshot().catch(() => null);
      if (fresh?.settings) draft.sendHost = fresh.settings.sendHost;
    });
    return () => {
      un.then((f) => f());
    };
  });
  async function sendConnect() {
    sendConnecting = true;
    try {
      const email = await api.sendDriveSignIn();
      // Shown at once; the free space and the rest follow when Google answers.
      sendDrive = { ...(sendDrive ?? { missingNote: null, limit: null, used: null, error: null }), configured: true, signedIn: true, email };
      app.toast(email ? `Google Drive connected for uploads (${email})` : "Google Drive connected for uploads");
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      sendConnecting = false;
      loadSendDrive();
    }
  }
  async function sendDisconnect() {
    await app.run(() => api.sendDriveSignOut());
    loadSendDrive();
  }

  /** Settings › Notifications: the kinds under their headings. */
  const noticeGroups = (["Downloads", "Upload", "Direct", "Pako"] as const).map((g) => ({ name: g, kinds: KINDS.filter((k) => k.group === g) }));
  /** One switch of one kind. (Download finished has no Pop-up switch: each download says how it
   *  tells you, new ones as "New downloads" says.) */
  function setNotice(kind: NoticeKind, which: "popup" | "bell", on: boolean) {
    const all = { ...(draft.notices ?? {}) };
    all[kind] = { ...(all[kind] ?? {}), [which]: on };
    draft.notices = all;
  }

  /** "ZIP in parts": what's typed in its Custom field is a size Pako takes. */
  let partValid = $state(true);

  /** Several files: the saved choice, else what the older compression setting meant. */
  const sendMode = $derived<SendMode>(
    draft.sendMode ?? (draft.sendCompression === "auto" || draft.sendCompression === "max" ? "compress" : "files"),
  );
</script>

<svelte:window onkeydowncapture={ctrlTab} />

<Modal title="Settings" width={780} autofocus={false} outsideCloses={false} onclose={tryClose} onuncover={saveAndClose}>
  <div class="set" bind:this={root}>
  <nav class="snav" data-tour="settings-nav" aria-label="Settings">
    <div class="find">
      <label class="search" class:filled={query}>
        <Icon name="search" size={13} stroke={2} />
        <input
          type="text"
          placeholder="Search settings"
          spellcheck="false"
          autocomplete="off"
          aria-label="Search settings"
          role="combobox"
          aria-expanded={listOpen}
          aria-controls="set-hits"
          aria-activedescendant={listOpen && hits.length ? `set-hit-${hitAt}` : undefined}
          bind:value={query}
          oninput={typed}
          onkeydown={searchKey}
          onfocus={() => (listOpen = !!query.trim())}
          onblur={() => (listOpen = false)}
        />
        {#if query}
          <button class="clear" aria-label="Clear the search" onmousedown={(e) => e.preventDefault()} onclick={() => ((query = ""), typed())} transition:fadeIn={{ duration: 120 }}><Icon name="x" size={11} stroke={2.2} /></button>
        {/if}
      </label>
      {#if listOpen}
        <div class="hits" id="set-hits" role="listbox" aria-label="Settings found" tabindex="-1" onmousedown={(e) => e.preventDefault()} transition:pop={{ duration: 160, from: 0.98, y: -4, origin: "top left" }}>
          {#each hits as h, i (i + h.title + h.tab + h.group)}
            <button class="hit" class:on={i === hitAt} id="set-hit-{i}" role="option" aria-selected={i === hitAt} tabindex="-1" onmouseenter={() => (hitAt = i)} onclick={() => pick(h)}>
              {#if h.isGroup}
                <span class="where">{tabName(h.tab)}{h.advanced ? " › Advanced" : ""} ›</span>
                <span class="what">{h.title}</span>
                <span class="where">group</span>
              {:else if h.el}
                <span class="where">{tabName(h.tab)}{h.advanced ? " › Advanced" : ""}{h.group ? ` › ${h.group}` : ""} ›</span>
                <span class="what">{h.title}</span>
              {:else}
                <span class="what">{h.title}</span>
                <span class="where">tab</span>
              {/if}
            </button>
          {:else}
            <div class="none">No setting matches '{searched}'</div>
          {/each}
        </div>
      {/if}
    </div>
    <div class="tabs" role="tablist" tabindex="-1" aria-orientation="vertical" aria-label="Settings sections" bind:this={tabList} onkeydown={tabKey} onfocusout={tabsLeft}>
      <span class="hl" style="transform: translateY(calc({SETTINGS_TABS.findIndex((t) => t.id === tab)} * (var(--tab-h) + var(--tab-gap))))" aria-hidden="true"></span>
      {#each SETTINGS_TABS as t (t.id)}
        <button
          id="set-tab-{t.id}"
          role="tab"
          class:on={tab === t.id}
          aria-selected={tab === t.id}
          aria-controls="set-panel-{t.id}"
          tabindex={focusTab === t.id ? 0 : -1}
          onclick={() => show(t.id)}
        >
          <span class="tname">{t.name}</span>
          {#if unsaved[t.id]}<span class="dirty" title="Changed, not saved yet" transition:pop={{ duration: 160, from: 0.4, y: 0 }}></span>{/if}
        </button>
      {/each}
    </div>
    <div class="stut"><TutorialButton id="settings" /></div>
  </nav>

  <div class="pane" bind:this={pane}>
  <!-- Every tab is in the page, the others hidden: search finds settings in all of them. -->
  <div class="panel" role="tabpanel" id="set-panel-updates" aria-labelledby="set-tab-updates" data-tab="updates" hidden={tab !== "updates"}>
    <!-- First: what people come to Settings for most (is my Pako up to date?). -->
    <UpdatesSection
      bind:autoUpdate={draft.autoUpdate}
      onsetup={() => {
        cancel();
        app.setupOpen = true;
      }}
    />
  </div>

  <!-- Second: adding Pako to the browser, the other thing people look for first. -->
  <div class="panel" role="tabpanel" id="set-panel-browser" aria-labelledby="set-tab-browser" data-tab="browser" hidden={tab !== "browser"}>
  <section data-tour="set-browser">
    <h3>Browser extension</h3>
    {#if linkResult}
      <div class="reveal" transition:rowSlide={{ duration: 220 }}>
      <div class="banner" class:bad={!linkResult.ok} role="status">
        <Icon name={linkResult.ok ? "check" : "alert"} size={14} />
        <span>{linkResult.ok ? "Connected. Go back to your browser and open Pako's popup again." : `Couldn't connect: ${linkResult.error ?? "unknown problem"}`}</span>
      </div>
      </div>
    {/if}
    <SetGroup title="Connection">
      <div class="line">
        <div>
          <div class="t">Connect Pako to your browsers</div>
          <div class="muted">Lets the Pako extension send downloads, links and videos to Pako. It talks to Pako directly on this PC: nothing goes over the internet.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.browserConnect} role="switch" aria-checked={draft.browserConnect} aria-label="Connect Pako to your browsers" onclick={() => (draft.browserConnect = !draft.browserConnect)}></button>
        </div>
      </div>
      <div class="line stack">
        <div>
          <div class="t">Add Pako to your browser <HelpLink to="browser-extension.md#add-pako-to-your-browser" label="How to add Pako to your browser" /></div>
          <div class="muted">Pick your browser: Pako opens its extensions page and shows the three steps. Then right-click a link › Download link with Pako, or use the Download button on YouTube videos.</div>
        </div>
        <div class="browsers"><AddToBrowser /></div>
        <p class="muted unlisted">
          Browser not listed? <button class="linkish" onclick={() => help.show("browser-extension.md#another-browser")}>How to add Pako to another browser</button>
        </p>
      </div>
    </SetGroup>
    <SetGroup title="Browser downloads">
      <div class="line">
        <div>
          <div class="t">Ask before each browser download</div>
          <div class="muted">A small window shows the name, folder and quality first. Off: downloads from your browser start right away.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.askBrowser} role="switch" aria-checked={draft.askBrowser} aria-label="Ask before each browser download" onclick={() => (draft.askBrowser = !draft.askBrowser)}></button>
        </div>
      </div>
      <div class="line">
        <div>
          <div class="t">Show the small window when a browser download starts</div>
          <div class="muted">The tray's list opens at the bottom right so you can see your downloads at a glance. Pin it to keep it on top. Never for a download from a private window.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.miniOnBrowser} role="switch" aria-checked={draft.miniOnBrowser} aria-label="Show the small window when a browser download starts" onclick={() => (draft.miniOnBrowser = !draft.miniOnBrowser)}></button>
        </div>
      </div>
    </SetGroup>
    <AdvancedFold tab="browser" hint="Connection: the extension's files">
      <SetGroup title="Connection">
        <div class="line">
          <div>
            <div class="t">The extension's files</div>
            <div class="muted">For adding Pako to a browser by hand: Pako writes the extension next to its data and opens that folder.</div>
          </div>
          <div class="ctrl">
            <button class="btn" onclick={openExtension}>Open the extension folder</button>
          </div>
        </div>
      </SetGroup>
    </AdvancedFold>
  </section>
  </div>

  <div class="panel" role="tabpanel" id="set-panel-accounts" aria-labelledby="set-tab-accounts" data-tab="accounts" hidden={tab !== "accounts"}>
  <section data-tour="set-accounts">
    <h3>Accounts</h3>
    <SetGroup title="Google Drive">
      <div class="account">
        <div class="logo"><Icon name="upload" size={15} /></div>
        <div class="acc-text">
          <div class="acc-name">Google Drive for uploads</div>
          {#if !sendDrive}
            <div class="muted">Checking…</div>
          {:else if !sendDrive.configured}
            <div class="muted" in:fadeIn={{ duration: 160 }}>Not set up in this Pako. Uploads go to Gofile.</div>
          {:else if sendDrive.signedIn}
            <div class="ok" in:fadeIn={{ duration: 160 }}><span class="dot"></span>{sendDrive.email || "Google account"}</div>
          {:else}
            <div class="muted" in:fadeIn={{ duration: 160 }}>Upload big files to your own Drive. Pako only sees the files it uploads.</div>
          {/if}
        </div>
        {#if sendDrive?.configured}
          {#if sendDrive.signedIn}
            <button class="btn" onclick={sendDisconnect}>Disconnect</button>
          {:else}
            <button class="btn primary" disabled={sendConnecting} onclick={sendConnect}>{sendConnecting ? "Waiting for the browser…" : "Connect Google Drive"}</button>
          {/if}
        {/if}
      </div>
      <p class="muted note attached">Only used for uploads: downloads from Drive never need a sign-in. What you uploaded stays in your Drive if you disconnect.</p>
    </SetGroup>
  </section>
  </div>

  <div class="panel" role="tabpanel" id="set-panel-window" aria-labelledby="set-tab-window" data-tab="window" hidden={tab !== "window"}>
  <section data-tour="set-window">
    <h3>Window</h3>
    <SetGroup title="Look">
      <div class="line">
        <div>
          <div class="t">Theme</div>
          <div class="muted">Match Windows follows Windows' own light or dark mode. The sun or moon at the top of Pako switches too.</div>
        </div>
        <div class="ctrl">
          <SegPick
            label="Theme"
            options={[
              { value: "dark", label: "Dark" },
              { value: "light", label: "Light" },
              { value: "system", label: "Match Windows" },
            ]}
            value={themePick}
            onpick={(t) => pickTheme(t as ThemeChoice)}
          />
        </div>
      </div>
    </SetGroup>
    <SetGroup title="Tray">
      <div class="line">
        <div>
          <div class="t">Minimize to the taskbar tray</div>
          <div class="muted">Minimize hides Pako next to the clock; it keeps downloading. Click the tray icon for a small list, double-click it to open Pako.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.minimizeToTray} role="switch" aria-checked={draft.minimizeToTray} aria-label="Minimize to the taskbar tray" onclick={() => (draft.minimizeToTray = !draft.minimizeToTray)}></button>
        </div>
      </div>
      <div class="line">
        <div>
          <div class="t">Close to the taskbar tray</div>
          <div class="muted">The close button hides Pako instead of quitting. Quit from the tray icon's menu.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.closeToTray} role="switch" aria-checked={draft.closeToTray} aria-label="Close to the taskbar tray" onclick={() => (draft.closeToTray = !draft.closeToTray)}></button>
        </div>
      </div>
    </SetGroup>
    <SetGroup title="Starting and quitting">
      <div class="line">
        <div>
          <div class="t">Start Pako with Windows</div>
          <div class="muted">Pako opens when you sign in, so waiting downloads carry on by themselves.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.startWithWindows} role="switch" aria-checked={draft.startWithWindows} aria-label="Start Pako with Windows" onclick={() => (draft.startWithWindows = !draft.startWithWindows)}></button>
        </div>
      </div>
      <div class="line">
        <div>
          <div class="t">Ask before quitting while downloading or uploading</div>
          <div class="muted">Quitting stops downloads and uploads. Downloads continue next time you open Pako; uploads don't.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.confirmQuit} role="switch" aria-checked={draft.confirmQuit} aria-label="Ask before quitting while downloading or uploading" onclick={() => (draft.confirmQuit = !draft.confirmQuit)}></button>
        </div>
      </div>
    </SetGroup>
    <AdvancedFold tab="window" hint="Tray: start in the tray">
      <SetGroup title="Tray">
        <div class="line" class:off={!draft.startWithWindows}>
          <div>
            <div class="t">Start in the tray</div>
            <div class="muted">No window at sign-in: Pako waits next to the clock.{draft.startWithWindows ? "" : " Used with Start Pako with Windows."}</div>
          </div>
          <div class="ctrl">
            <button class="switch" class:on={draft.startInTray} role="switch" aria-checked={draft.startInTray} aria-label="Start in the tray" disabled={!draft.startWithWindows} onclick={() => (draft.startInTray = !draft.startInTray)}></button>
          </div>
        </div>
      </SetGroup>
    </AdvancedFold>
  </section>
  </div>

  <div class="panel" role="tabpanel" id="set-panel-notices" aria-labelledby="set-tab-notices" data-tab="notices" hidden={tab !== "notices"}>
  <section data-tour="set-notices">
    <h3>Notifications</h3>
    <p class="muted intro">Pop-ups show at the bottom right of the screen for a few seconds. The bell, at the top of Pako, keeps the important ones for 30 days.</p>
    <SetGroup title="When a download finishes">
      <div class="line" data-tour="set-done-alert">
        <div class="grow">
          <div class="t">New downloads</div>
          <div class="muted">How a download tells you it's done. Each one can change it: the bell on its row, its details, or right-click.</div>
          <div class="pick">
            <SegPick label="New downloads" options={ALERTS.map((a) => ({ value: a.id, label: a.label, hint: a.hint }))} value={doneAlert(draft)} onpick={(a) => (draft.doneAlert = a)} />
          </div>
          {#if doneAlert(draft) === "important"}<div class="reveal" transition:rowSlide><div class="muted small">Important: a big pop-up in the middle of the screen that stays until you click it.</div></div>{/if}
        </div>
      </div>
      <div class="line">
        <div class="grow">
          <div class="t">Chime when a download finishes</div>
          <div class="muted">A short, soft chime, with the pop-up.</div>
          <div class="pick">
            <SegPick label="Chime when a download finishes" options={CHIMES.map((c) => ({ value: c.id, label: c.label }))} value={doneChime(draft)} onpick={(c) => (draft.doneChime = c)} />
          </div>
        </div>
      </div>
    </SetGroup>
    <SetGroup title="Each kind of news">
      <div class="kinds" class:hushed={draft.quiet} role="table" aria-label="What shows as a pop-up and in the bell">
        <div class="khead" role="row">
          <span role="columnheader"><span class="sr">What</span></span>
          <span role="columnheader" class="col">Pop-up</span>
          <span role="columnheader" class="col">Bell</span>
        </div>
        {#each noticeGroups as g (g.name)}
          <div class="kgroup" role="rowgroup">
            <div class="kname" role="row"><span role="rowheader">{g.name}</span></div>
            {#each g.kinds as k (k.id)}
              {@const p = prefOf(draft, k.id)}
              <div class="krow" role="row">
                <span class="klabel" role="cell">{k.label}</span>
                <span class="col pop" role="cell">
                  {#if k.id === "download-done"}
                    <span class="per" title="Set by New downloads, and for each download">Per download</span>
                  {:else}
                    <button class="switch" class:on={p.popup} role="switch" aria-checked={p.popup} aria-label="{k.label}: pop-up" onclick={() => setNotice(k.id, "popup", !p.popup)}></button>
                  {/if}
                </span>
                <span class="col" role="cell">
                  <button class="switch" class:on={p.bell} role="switch" aria-checked={p.bell} aria-label="{k.label}: bell" onclick={() => setNotice(k.id, "bell", !p.bell)}></button>
                </span>
              </div>
            {/each}
          </div>
        {/each}
      </div>
      <p class="muted note attached">While the small window is open, it shows downloads finishing instead of their pop-up. Important always shows, also during Do not disturb, but waits while something is full screen (a game, a video, a presentation).</p>
    </SetGroup>
    <SetGroup title="Sounds">
      <div class="line">
        <div>
          <div class="t">Play a sound when files arrive</div>
          <div class="muted">A short, soft chime when files arrive in Direct and My devices, and when someone offers you files. Silent during Do not disturb, Pako's or Windows'.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.playDoneSound} role="switch" aria-checked={draft.playDoneSound} aria-label="Play a sound when files arrive" onclick={() => (draft.playDoneSound = !draft.playDoneSound)}></button>
        </div>
      </div>
      <div class="line">
        <div>
          <div class="t">Play a sound when files are sent</div>
          <div class="muted">A brighter sound when an upload finishes, or when a Direct or My devices send is delivered. Silent at the same times.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.playSentSound} role="switch" aria-checked={draft.playSentSound} aria-label="Play a sound when files are sent" onclick={() => (draft.playSentSound = !draft.playSentSound)}></button>
        </div>
      </div>
    </SetGroup>
    <SetGroup title="Quiet times">
      <div class="line">
        <div>
          <div class="t">Do not disturb</div>
          <div class="muted">No pop-ups, except for downloads set to Important. The bell still collects.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.quiet} role="switch" aria-checked={draft.quiet} aria-label="Do not disturb" onclick={() => (draft.quiet = !draft.quiet)}></button>
        </div>
      </div>
    </SetGroup>
    <AdvancedFold tab="notices" hint="Quiet times: follow Windows' Do not disturb">
      <SetGroup title="Quiet times">
        <div class="line">
          <div>
            <div class="t">Follow Windows' Do not disturb</div>
            <div class="muted">No pop-ups during full-screen games and videos, presentations, or Windows' Do not disturb. The bell still collects.</div>
          </div>
          <div class="ctrl">
            <button class="switch" class:on={draft.followWindowsQuiet} role="switch" aria-checked={draft.followWindowsQuiet} aria-label="Follow Windows' Do not disturb" onclick={() => (draft.followWindowsQuiet = !draft.followWindowsQuiet)}></button>
          </div>
        </div>
      </SetGroup>
    </AdvancedFold>
  </section>
  </div>

  <div class="panel" role="tabpanel" id="set-panel-downloads" aria-labelledby="set-tab-downloads" data-tab="downloads" hidden={tab !== "downloads"}>
  <section data-tour="set-downloads">
    <h3>Downloads</h3>
    <SetGroup title="Speed">
      <div class="line">
        <div>
          <div class="t">Downloads at the same time</div>
          <div class="muted">The rest wait in the queue and start on their own.</div>
        </div>
        <div class="ctrl"><input class="input num" type="number" min="1" max="20" bind:value={draft.maxActive} aria-label="Downloads at the same time" /></div>
      </div>
      <div class="line">
        <div>
          <div class="t">Speed limit</div>
          <div class="muted">Total for all downloads, in KB/s. 0 = no limit.</div>
        </div>
        <div class="ctrl"><input class="input num wide" type="number" min="0" bind:value={draft.speedLimitKb} aria-label="Speed limit in KB/s" /></div>
      </div>
      <div class="line">
        <div>
          <div class="t">Only download at set hours</div>
          <div class="muted">Outside these hours downloads wait, and running ones pause and carry on when the hours start. "Download first" still starts one right away.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.scheduleOn} role="switch" aria-checked={draft.scheduleOn} aria-label="Only download at set hours" onclick={() => (draft.scheduleOn = !draft.scheduleOn)}></button>
        </div>
      </div>
      {#if draft.scheduleOn}
        <div class="reveal attached" transition:rowSlide={{ duration: 220 }}>
        <div class="line hours">
          <span class="muted">From</span>
          <input class="input num wide" type="time" aria-label="Start time" value={hhmm(draft.scheduleStart)} onchange={(e) => (draft.scheduleStart = minutesOf(e.currentTarget.value))} />
          <span class="muted">to</span>
          <input class="input num wide" type="time" aria-label="End time" value={hhmm(draft.scheduleEnd)} onchange={(e) => (draft.scheduleEnd = minutesOf(e.currentTarget.value))} />
          <span class="muted small">{draft.scheduleStart > draft.scheduleEnd ? "(overnight)" : ""}</span>
        </div>
        </div>
      {/if}
    </SetGroup>
    <SetGroup title="Saving files">
      <div class="folder" data-search="Download folder">
        <label class="lbl" for="set-dir">Download folder</label>
        <div class="row">
          <input id="set-dir" class="input" bind:value={draft.downloadDir} spellcheck="false" />
          <button class="btn" onclick={choose}>Change</button>
        </div>
      </div>
      <div class="line">
        <div>
          <div class="t">Extract split archives</div>
          <div class="muted">When every part of a split archive (.part1.rar, .7z.001…) is downloaded, extract it into its folder. Each set can change this in its details panel.{app.sevenZip ? "" : " Needs 7-Zip (Features & updates)."}</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.extractDefault} role="switch" aria-checked={draft.extractDefault} aria-label="Extract split archives" onclick={() => (draft.extractDefault = !draft.extractDefault)}></button>
        </div>
      </div>
    </SetGroup>
    <SetGroup title="Videos">
      <div class="line">
        <div>
          <div class="t">Video quality</div>
          <div class="muted">Used for new video links. If a video doesn't have it, Pako takes the closest lower one. Each video can change it in its details.</div>
        </div>
        <div class="ctrl">
          <select class="input vq" aria-label="Video quality" value={String(draft.videoQuality)} onchange={(e) => (draft.videoQuality = Number(e.currentTarget.value))}>
            {#each [[2160, "Up to 4K (2160p)"], [1440, "Up to 1440p"], [1080, "Up to 1080p"], [720, "Up to 720p"], [480, "Up to 480p"], [0, "Audio only (M4A)"]] as [v, label] (v)}
              <option value={String(v)}>{label}</option>
            {/each}
          </select>
        </div>
      </div>
    </SetGroup>
    <SetGroup title="While downloading">
      <div class="line">
        <div>
          <div class="t">Keep the PC awake while downloading</div>
          <div class="muted">Windows won't go to sleep in the middle of a download. The screen can still turn off.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.keepAwake} role="switch" aria-checked={draft.keepAwake} aria-label="Keep the PC awake while downloading" onclick={() => (draft.keepAwake = !draft.keepAwake)}></button>
        </div>
      </div>
      <div class="line">
        <div>
          <div class="t">When all downloads finish</div>
          <div class="muted">For this session only: Pako starts with "Do nothing" each time. Also in the bottom bar.</div>
        </div>
        <div class="ctrl">
          <select class="input when" aria-label="When all downloads finish" value={app.whenDone} onchange={(e) => app.setWhenDone(e.currentTarget.value as WhenDone)}>
            <option value="nothing">Do nothing</option>
            <option value="sleep">Sleep</option>
            <option value="shutDown">Shut down</option>
          </select>
        </div>
      </div>
      <div class="line">
        <div>
          <div class="t">Catch copied links</div>
          <div class="muted">Links you copy in any app go to Links by themselves. Links already in Pako are skipped.</div>
        </div>
        <div class="ctrl">
          <button class="switch" class:on={draft.catchClipboard} role="switch" aria-checked={draft.catchClipboard} aria-label="Catch copied links" onclick={() => (draft.catchClipboard = !draft.catchClipboard)}></button>
        </div>
      </div>
    </SetGroup>
    <SetGroup title="From private windows">
        <div class="line" class:off={priv.status.on}>
          <div>
            <div class="t">Remove private downloads from the list</div>
            <div class="muted">
              {#if priv.status.on}
                Private mode is on: downloads from private windows go to Private.
              {:else}
                Downloads from a private browser window (Done · private). Opening one from Pako, or its ✕, removes it at once. The file stays.
              {/if}
            </div>
          </div>
          <div class="ctrl">
            <select class="input keep" aria-label="Remove private downloads from the list" disabled={priv.status.on} value={draft.privateKeep ?? "10m"} onchange={(e) => (draft.privateKeep = e.currentTarget.value as PrivateKeep)}>
              <option value="10m">10 minutes after they finish</option>
              <option value="1h">1 hour after</option>
              <option value="close">When you quit Pako</option>
            </select>
          </div>
        </div>
    </SetGroup>

    <AdvancedFold tab="downloads" hint="Speed, saving files, videos and while downloading: connections, checks, previews, continuing at start">
      <SetGroup title="Speed">
        <div class="line">
          <div>
            <div class="t">Connections per file</div>
            <div class="muted">Auto adds connections while it makes the file faster and stops when the server or your internet can't take more.</div>
          </div>
          <div class="ctrl">
            <span class="muted">Auto</span>
            <button class="switch" class:on={draft.autoConnections} role="switch" aria-checked={draft.autoConnections} aria-label="Auto connections" onclick={() => (draft.autoConnections = !draft.autoConnections)}></button>
            <input class="input num" type="number" min="1" max="32" bind:value={draft.connections} disabled={draft.autoConnections} aria-label="Connections per file" />
          </div>
        </div>
      </SetGroup>
      <SetGroup title="Saving files">
        <div class="line">
          <div>
            <div class="t">Check Drive files when finished</div>
            <div class="muted">Compares each finished file with Google's own record of it, so a damaged file is caught.</div>
          </div>
          <div class="ctrl">
            <button class="switch" class:on={draft.verifyChecksum} role="switch" aria-checked={draft.verifyChecksum} aria-label="Check Drive files when finished" onclick={() => (draft.verifyChecksum = !draft.verifyChecksum)}></button>
          </div>
        </div>
      </SetGroup>
      <SetGroup title="Videos">
        <div class="line">
          <div>
            <div class="t">Video preview quality</div>
            <div class="muted">The small player in the details panel. Lower uses less data; each video can change it.</div>
          </div>
          <div class="ctrl">
            <SegPick label="Video preview quality" options={[360, 480, 720].map((q) => ({ value: q, label: `${q}p` }))} value={draft.previewQuality} onpick={(q) => (draft.previewQuality = q)} />
          </div>
        </div>
      </SetGroup>
      <SetGroup title="While downloading">
        <div class="line">
          <div>
            <div class="t">Continue downloads when Pako starts</div>
            <div class="muted">Downloads that were running when Pako closed carry on by themselves. Turned off, they're paused until you click Resume.</div>
          </div>
          <div class="ctrl">
            <button class="switch" class:on={draft.resumeOnStart} role="switch" aria-checked={draft.resumeOnStart} aria-label="Continue downloads when Pako starts" onclick={() => (draft.resumeOnStart = !draft.resumeOnStart)}></button>
          </div>
        </div>
      </SetGroup>
    </AdvancedFold>
  </section>
  </div>

  <div class="panel" role="tabpanel" id="set-panel-private" aria-labelledby="set-tab-private" data-tab="private" hidden={tab !== "private"}>
  <section data-tour="set-private">
    <h3>Private mode</h3>
    <SetGroup title="On or off">
      <div class="line">
        <div>
          <div class="t">Private mode: keep private downloads behind a PIN</div>
          <div class="muted">
            {#if priv.status.on}
              On. Downloads from private windows go to <b>Private</b> in Downloads, encrypted on this PC.{#if !priv.status.unlocked}{" "}Open it with the button, with <b>Private</b> in Downloads, or with Ctrl+Shift+P.{/if}
            {:else}
              Downloads from your browser's private windows go to a Private list in Downloads instead, their files encrypted on this PC, behind a PIN or a password.
            {/if}
          </div>
        </div>
        <div class="ctrl">
          {#if priv.status.on}
            {#if priv.status.unlocked}
              <span class="pchip"><Icon name="unlock" size={12} />Open</span>
            {:else}
              <button class="btn pchip-btn" title="Close Settings and type your PIN or password" onclick={openPrivate}><Icon name="lock" size={13} />Open Private mode…</button>
            {/if}
          {:else}
            <button class="btn" onclick={turnOnPrivate}><Icon name="lock" size={13} />Turn on…</button>
          {/if}
        </div>
      </div>
    </SetGroup>
    {#if priv.status.on}
      <div class="reveal" transition:rowSlide={{ duration: 220 }}>
      <SetGroup title="Locking">
        <div class="line">
          <div>
            <div class="t">Lock Private mode after</div>
            <div class="muted">With no keyboard or mouse use on the PC. It also locks at once when Windows locks, the PC sleeps, the screen saver starts, or Pako is minimised or closed.</div>
          </div>
          <div class="ctrl">
            <select class="input keep" aria-label="Lock Private mode after" value={draft.privateLockAfter ?? "5m"} onchange={(e) => (draft.privateLockAfter = e.currentTarget.value as Settings["privateLockAfter"])}>
              <option value="1m">1 minute</option>
              <option value="5m">5 minutes</option>
              <option value="15m">15 minutes</option>
              <option value="30m">30 minutes</option>
              <option value="never">Never for being idle</option>
            </select>
          </div>
        </div>
        <div class="line">
          <div>
            <div class="t">Clear the private list</div>
            <div class="muted">Private downloads, and their files, are deleted this long after they finished. Downloads still running stay.</div>
          </div>
          <div class="ctrl">
            {#if clearCustom}
              <input class="input num" type="number" min="1" max="8760" aria-label="Hours" bind:value={clearHours} oninput={() => (draft.privateClearAfter = `${Math.max(1, Math.round(clearHours || 1))}h`)} />
              <span class="muted">hours</span>
            {/if}
            <select
              class="input keep"
              aria-label="Clear the private list"
              value={clearCustom ? "custom" : (draft.privateClearAfter ?? "never")}
              onchange={(e) => {
                const v = e.currentTarget.value;
                clearCustom = v === "custom";
                draft.privateClearAfter = clearCustom ? `${Math.max(1, Math.round(clearHours || 24))}h` : v;
              }}
            >
              <option value="never">Never</option>
              <option value="3h">3 hours after</option>
              <option value="12h">12 hours after</option>
              <option value="3d">3 days after</option>
              <option value="custom">After a number of hours…</option>
            </select>
          </div>
        </div>
      </SetGroup>
      <SetGroup title="Opening files">
        <div class="line">
          <div>
            <div class="t">Show real names when opening private files</div>
            <div class="muted">Off: a file opened from Private mode is named like PrivateFile_2026-11-18_14h32.pdf, so its name doesn't show in the app's title or Windows' recent files.</div>
          </div>
          <div class="ctrl">
            <button class="switch" class:on={draft.privateRealNames} role="switch" aria-checked={!!draft.privateRealNames} aria-label="Show real names when opening private files" onclick={() => (draft.privateRealNames = !draft.privateRealNames)}></button>
          </div>
        </div>
      </SetGroup>
      <SetGroup title="PIN and recovery">
        <div class="line stack">
          <div>
            <div class="t">PIN, password and recovery code</div>
            <div class="muted">
              {priv.status.lock === "password" ? "A password" : "A PIN"} opens Private mode. {priv.status.hasRecovery ? "You have a recovery code." : "No recovery code yet: without one, a forgotten PIN or password can't be undone."}
            </div>
          </div>
          <div class="ctrl wrap">
            <button class="btn" onclick={() => (priv.manage = "change")}>Change PIN or password…</button>
            <button class="btn" onclick={() => (priv.manage = "code")}>New recovery code…</button>
          </div>
        </div>
        <div class="line">
          <div>
            <div class="t">Reset Private mode</div>
            <div class="muted">Deletes the private list and every file in it, for good, and turns Private mode off. The only way back when the PIN or password is forgotten and there's no recovery code.</div>
          </div>
          <div class="ctrl">
            <button class="btn danger" onclick={() => (resetting = true)}><Icon name="trash" size={13} />Reset…</button>
          </div>
        </div>
      </SetGroup>
      </div>
    {/if}
  </section>
  </div>

  <div class="panel" role="tabpanel" id="set-panel-upload" aria-labelledby="set-tab-upload" data-tab="upload" hidden={tab !== "upload"}>
  <section data-tour="set-upload">
    <h3>Upload</h3>
    <SetGroup title="Where">
      <div class="line">
        <div>
          <div class="t">Upload to</div>
          <div class="muted">
            {#if draft.sendHost === "drive"}
              Your own Google Drive: shared with anyone who has the link, kept until you delete it.
            {:else}
              Gofile: no account needed. Files are kept 10 days after their last download.
            {/if}
          </div>
        </div>
        <div class="ctrl">
          <select class="input sendmode" aria-label="Upload with" value={draft.sendHost === "drive" ? "drive" : "gofile"} onchange={(e) => (draft.sendHost = e.currentTarget.value as "gofile" | "drive")}>
            <option value="gofile">Gofile</option>
            <option value="drive">Google Drive</option>
          </select>
        </div>
      </div>
    </SetGroup>
    <SetGroup title="Several files">
      <div class="line">
        <div>
          <div class="t">When you upload several files</div>
          <div class="muted">
            {#if sendMode === "zip"}
              They're packed into one ZIP file: whoever opens the link downloads one file and opens it.
            {:else if sendMode === "compress"}
              One ZIP file, made smaller where it helps (documents, text). Takes longer to upload, and doesn't shrink videos or photos.
            {:else if sendMode === "parts"}
              One ZIP file, cut into numbered parts (their size is under Advanced). {PARTS_NOTE}
            {:else}
              Each file is uploaded as it is, a few at a time, into one folder. Whoever opens the link sees the files straight away.
            {/if}
            A single file goes as it is, unless you pick Compressed ZIP or ZIP in parts for it in the upload.
          </div>
        </div>
        <div class="ctrl">
          <select class="input sendmode" aria-label="When you upload several files" value={sendMode} onchange={(e) => (draft.sendMode = e.currentTarget.value as SendMode)}>
            <option value="files">As they are</option>
            <option value="zip">One ZIP</option>
            <option value="compress">Compressed ZIP</option>
            <option value="parts">ZIP in parts</option>
          </select>
        </div>
      </div>
    </SetGroup>
    <AdvancedFold tab="upload" hint="Several files and unfinished uploads: part size, continuing at start">
      <SetGroup title="Several files">
        <div class="line stacked" data-part-size>
          <div>
            <div class="t">Size of each part</div>
            <div class="muted">For ZIP in parts. Smaller parts start over faster when the connection drops; bigger ones make fewer files.</div>
          </div>
          <div class="ctrl">
            <PartSize bind:mb={draft.sendPartMb} bind:custom={draft.sendPartCustom} bind:valid={partValid} />
          </div>
        </div>
      </SetGroup>
      <SetGroup title="Unfinished uploads">
        <div class="line">
          <div>
            <div class="t">Continue unfinished uploads when Pako starts</div>
            <div class="muted">A ZIP in parts upload that stopped (Pako closed, PC restarted) carries on with the parts not up yet. Off: it waits for Try again.</div>
          </div>
          <div class="ctrl">
            <button class="switch" class:on={draft.resumeUploadsOnStart} role="switch" aria-checked={draft.resumeUploadsOnStart} aria-label="Continue unfinished uploads when Pako starts" onclick={() => (draft.resumeUploadsOnStart = !draft.resumeUploadsOnStart)}></button>
          </div>
        </div>
      </SetGroup>
    </AdvancedFold>
  </section>
  </div>
  </div>
  </div>

  {#snippet actions()}
    <button class="btn ghost" onclick={cancel}>Cancel</button>
    <button class="btn primary" data-tour="set-save" onclick={save}>Save</button>
  {/snippet}
</Modal>

{#if askingClose}
  <Modal title="Save changes?" width={420} onclose={() => (askingClose = false)} closeLabel="Keep editing">
    <p class="muted">Some settings are changed but not saved yet.</p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (askingClose = false)}>Keep editing</button>
      <button class="btn" onclick={() => { askingClose = false; cancel(); }}>Discard</button>
      <button class="btn primary" onclick={() => { askingClose = false; save(); }}>Save</button>
    {/snippet}
  </Modal>
{/if}
{#if resetting}
  <Modal title="Reset Private mode?" width={460} onclose={() => (resetting = false)}>
    <p class="msg">
      The private list and every file in it are deleted for good (not to the Recycle Bin), and Private mode turns off. Downloads from private windows go back to
      showing as Done · private.
    </p>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (resetting = false)}>Cancel</button>
      <button
        class="btn danger-fill"
        onclick={() => {
          resetting = false;
          app.run(async () => {
            await api.privateReset();
            await priv.load();
            app.toast("Private mode is off. Its list and files are deleted.");
          });
        }}><Icon name="trash" size={13} />Delete everything</button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  /* Search and tabs on the left, the open tab on the right, scrolling on its own. One height
     whatever the tab, so switching tabs never makes the dialog jump: as tall as the window lets
     the dialog be (174px: its title, buttons and the room around it, Modal.svelte), up to 680px. */
  .set {
    display: flex;
    gap: 24px;
    height: min(680px, calc(100vh - 174px));
  }
  .snav {
    position: relative;
    z-index: 3;
    width: 184px;
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 2px 0 12px;
  }

  /* ---------- search ---------- */
  .find {
    position: relative;
    margin-bottom: 10px;
  }
  .search {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px 0 10px;
    border-radius: var(--radius);
    border: 1px solid var(--control);
    background: var(--panel);
    color: var(--text-3);
    font-size: 12.5px;
    cursor: text;
    transition:
      border-color var(--fast) ease,
      box-shadow var(--fast) ease;
  }
  .search:focus-within {
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--focus-soft);
  }
  .search :global(svg) {
    flex: none;
  }
  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
  }
  .search input::placeholder {
    color: var(--text-3);
  }
  .clear {
    width: 18px;
    height: 18px;
    flex: none;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--raise);
    color: var(--text-3);
    cursor: pointer;
  }
  .clear:hover {
    color: var(--text);
  }
  /* The results, over the open tab: wide enough for "Notifications › Play a sound when…". */
  .hits {
    position: absolute;
    z-index: 5;
    top: calc(100% + 6px);
    left: 0;
    width: 460px;
    max-height: 316px;
    overflow-y: auto;
    padding: 5px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--line-strong);
    background: var(--menu-bg);
    backdrop-filter: blur(12px);
    box-shadow: var(--shadow-menu);
  }
  .hit {
    width: 100%;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .hit.on {
    background: var(--raise);
  }
  .where {
    flex: none;
    color: var(--text-3);
  }
  .what {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    padding: 9px 10px;
    font-size: 12.5px;
    color: var(--text-3);
    overflow-wrap: anywhere;
  }

  /* ---------- tabs ---------- */
  .tabs {
    --tab-h: 32px;
    --tab-gap: 2px;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--tab-gap);
  }
  /* One highlight that slides to the open tab. */
  .hl {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: var(--tab-h);
    border-radius: var(--radius);
    background: var(--active);
    pointer-events: none;
    transition: transform var(--mid) var(--ease);
  }
  .tabs button {
    position: relative;
    height: var(--tab-h);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    text-align: left;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .tname {
    min-width: 0;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tabs button:hover:not(.on) {
    background: var(--raise);
    color: var(--text);
  }
  .tabs button.on {
    color: var(--text);
    font-weight: 600;
  }
  /* Changed in this tab, not saved yet. */
  .dirty {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
    background: var(--accent);
  }
  .stut {
    margin: 10px 0 0 4px;
  }
  /* A short window (down to Pako's smallest, 520 px high): the search, the eight tabs and the
     Tutorial button still fit, closer together, with no scrollbar of their own. */
  @media (max-height: 640px) {
    .snav {
      padding-bottom: 4px;
    }
    .find {
      margin-bottom: 8px;
    }
    .tabs {
      --tab-h: 30px;
    }
    .stut {
      margin-top: 8px;
    }
  }
  /* Its scrollbar sits in the dialog's right margin, not against the card. The scrollbar's room
     is kept on every tab (scrollbar-gutter), so a tab long enough to scroll and a short one have
     the card at the same width: switching never moves its right edge (it moved 10px). 6px + the
     10px scrollbar (app.css) = the 16px pulled into the margin: the card ends in line with Save. */
  .pane {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    scrollbar-gutter: stable;
    overscroll-behavior: contain;
    margin-right: -16px;
    padding: 2px 6px 8px 0;
  }
  /* The open tab comes in softly (Windows' reduce motion: at once, app.css). */
  .panel:not([hidden]) {
    animation: tabIn var(--mid) var(--ease-out);
  }
  @keyframes tabIn {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
  }
  /* A setting reached from the search (or opened at): lit up for a moment, behind its words. A
     row's top padding (above its dividing line) stays out of it, and it never reaches further
     out than a fold lets show (Collapse.svelte). */
  :global(.set-flash) {
    position: relative;
    isolation: isolate;
  }
  :global(.set-flash)::after {
    content: "";
    position: absolute;
    inset: -4px;
    z-index: -1;
    border-radius: var(--radius);
    background: var(--focus-soft);
    opacity: 0;
    pointer-events: none;
    animation: setFlash 1.8s var(--ease-out);
  }
  /* A row in its group's card: lit up inside the card, clear of its hairlines. */
  :global(.line.set-flash)::after,
  :global(.folder.set-flash)::after,
  :global(.account.set-flash)::after {
    inset: 4px 6px;
  }
  @keyframes -global-setFlash {
    0%,
    35% {
      opacity: 1;
    }
    100% {
      opacity: 0;
    }
  }
  /* Windows' reduce motion: lit up without fading, then off. */
  @media (prefers-reduced-motion: reduce) {
    :global(.set-flash)::after {
      opacity: 1;
    }
  }
  .vq {
    width: 170px;
  }
  select.keep {
    width: 214px;
  }
  .msg {
    margin: 0;
    line-height: 1.55;
    color: var(--text-2);
  }
  .pchip-btn {
    border-color: var(--priv-line);
    color: var(--priv-text);
    background: var(--priv-tint);
  }
  .pchip-btn:hover {
    border-color: var(--priv);
    color: var(--priv-text);
    background: var(--priv-tint);
  }
  .pchip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px;
    border-radius: 12px;
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    color: var(--priv-text);
    font-size: 12px;
    font-weight: 500;
  }
  .line.stack {
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }
  .ctrl.wrap {
    flex-wrap: wrap;
    justify-content: flex-start;
  }
  /* Rows that open under a switch slide open; flow-root keeps their top margin inside the slide. */
  .reveal {
    display: flow-root;
  }
  /* Each tab: its title, then its groups, each a titled card (SetGroup.svelte). */
  section {
    padding: 0 0 4px;
  }
  h3 {
    margin: 2px 0 -4px 2px;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--text);
  }
  .folder {
    padding: 12px 14px;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
  }
  .logo {
    width: 32px;
    height: 32px;
    border-radius: var(--radius);
    display: grid;
    place-items: center;
    background: rgba(90, 169, 255, 0.14);
    color: var(--src-drive);
    flex: none;
  }
  .acc-text {
    flex: 1;
    min-width: 0;
  }
  .acc-name {
    font-weight: 500;
    margin-bottom: 2px;
  }
  .ok {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--st-done);
    flex: none;
  }
  .muted {
    font-size: 12px;
    color: var(--text-3);
    line-height: 1.45;
  }
  .muted.small {
    margin: 10px 0 0;
    font-size: 11.5px;
  }
  /* A note under a group's rows, part of the row above it (no hairline). */
  .muted.note {
    margin: 0;
    padding: 0 14px 12px;
    font-size: 11.5px;
  }
  .lbl {
    display: block;
    margin-bottom: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-3);
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .row .btn {
    height: 34px;
  }
  /* One setting: its words on the left, its control on the right, the same in every card. */
  .line {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 14px;
  }
  .t {
    font-weight: 500;
    margin-bottom: 3px;
  }
  .ctrl {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  /* A control too wide to sit beside its words (the part size with its Custom field): under them. */
  .line.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }
  .browsers {
    align-self: stretch;
  }
  .unlisted {
    margin: 0;
  }
  .linkish {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent-text, var(--accent));
    font: inherit;
    cursor: pointer;
  }
  .linkish:hover {
    text-decoration: underline;
  }
  .banner {
    margin-top: 14px;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid rgba(70, 201, 126, 0.35);
    border-radius: var(--radius-lg);
    background: rgba(70, 201, 126, 0.08);
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--st-done-text);
  }
  .banner :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .banner.bad {
    border-color: rgba(242, 75, 79, 0.35);
    background: var(--tint-fail);
    color: var(--st-fail-text);
  }
  .num {
    width: 64px;
    text-align: center;
    font-family: var(--mono);
  }
  .num.wide {
    width: 92px;
  }
  .num:disabled {
    opacity: 0.4;
  }
  /* A setting that only matters with another one on ("Start in the tray" without Start Pako with
     Windows): its words step back with its switch (app.css dims the switch itself). */
  .line.off > div:first-child {
    opacity: 0.55;
  }
  .line > div:first-child {
    transition: opacity var(--mid) ease;
  }
  /* The hours under "Only download at set hours": part of that row. */
  .line.hours {
    justify-content: flex-start;
    gap: 10px;
    padding-top: 0;
  }
  .line.hours .num.wide {
    width: 104px;
  }
  .small {
    font-size: 12px;
  }
  select.when {
    width: 140px;
  }
  select.sendmode {
    width: 214px;
  }
  /* Settings › Notifications: each kind with its two switches, under small headings. */
  .muted.intro {
    margin: 10px 0 -4px 2px;
  }
  .kinds {
    padding: 10px 14px 12px;
  }
  .khead,
  .krow {
    display: grid;
    grid-template-columns: 1fr 72px 72px;
    align-items: center;
  }
  .khead {
    padding: 0 1px 2px 13px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .col {
    display: grid;
    place-items: center;
  }
  /* Each group in its own soft box, so Downloads, Upload, Direct and Pako read apart at a glance
     (user, 2026-11-18). The switch columns still line up with POP-UP and BELL above. */
  .kgroup {
    margin-top: 8px;
    padding: 2px 0 2px 12px;
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    background: var(--raise);
  }
  .kname {
    padding: 10px 0 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .krow {
    min-height: 34px;
    border-top: 1px solid var(--line-soft);
  }
  .kname + .krow {
    border-top: none;
  }
  .klabel {
    color: var(--text-2);
  }
  /* Do not disturb on: the pop-up switches keep their choice but step back. */
  .col.pop {
    transition: opacity var(--mid) ease;
  }
  .kinds.hushed .col.pop {
    opacity: 0.45;
  }
  /* Download finished: no switch, it's chosen per download. */
  .per {
    font-size: 11px;
    color: var(--text-4);
    white-space: nowrap;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .pick {
    margin-top: 10px;
  }
</style>
