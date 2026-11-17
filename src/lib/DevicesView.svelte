<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // My devices: your own PCs. Left: your devices (green online, blue on the same network, grey
  // offline). Right: the picked device (DeviceDetails): files between your devices, its
  // downloads, and its settings. Pairing: "Add a device" on either PC, the same safety code on
  // both, "This is my device" on both.
  import { untrack } from "svelte";
  import { open as pick } from "@tauri-apps/plugin-dialog";
  import { app } from "./state.svelte";
  import { api, inApp, type DirectRequest } from "./api";
  import { devices, devicesApi, dropOnDevice, onLan, type Device } from "./devices.svelte";
  import { ago, initial, pathLabel } from "./direct";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import TutorialButton from "./tour/TutorialButton.svelte";
  import DeviceDetails from "./DeviceDetails.svelte";
  import { fadeIn, flipIf, flipOpts, pop, rowSlide } from "./motion";

  const direct = $derived(app.direct);
  // Devices ride on Direct: opening this page starts it, like the Direct page does.
  let woken = false;
  $effect(() => {
    if (direct && !direct.ready && !direct.error && !woken) {
      woken = true;
      api.directWake().catch(() => (woken = false));
    }
  });
  $effect(() => {
    devices.start();
    if (direct?.ready) devices.refresh();
  });

  const d = $derived(devices.data);
  const list = $derived.by(() => {
    const rank = (x: Device) => (x.asksToPair ? 0 : x.online ? 1 : 2);
    return [...(d?.devices ?? [])].sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
  });
  const current = $derived(list.find((x) => x.id === devices.picked) ?? list.find((x) => !x.asksToPair) ?? list[0] ?? null);
  $effect(() => {
    if (current && devices.picked !== current.id && !list.some((x) => x.id === devices.picked)) devices.picked = current.id;
  });

  function status(x: Device) {
    if (x.asksToPair) return "Wants to pair with this PC";
    if (x.online && !x.paired) return "Waiting for you to confirm on it";
    if (x.online) return `Online · ${pathLabel(x.path) || "connected"}`;
    if (x.waiting || !x.paired) return "Confirm on the other PC";
    if (d && !d.internet) return "Not reachable without internet";
    return `Offline · ${ago(x.lastSeen)}`;
  }
  const moving = (id: string) => (d?.transfers ?? []).filter((t) => t.contact === id && (t.state === "sending" || t.state === "receiving"));
  function rowPct(id: string) {
    const m = moving(id);
    const size = m.reduce((a, t) => a + t.size, 0);
    return size ? Math.min(100, Math.floor((m.reduce((a, t) => a + Math.min(t.done, t.size), 0) / size) * 100)) : 0;
  }

  // ---------- files dropped on a device's row: sent to it ----------
  let dropRow = $state<string | null>(null);
  const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes("Files");
  function rowOver(e: DragEvent, x: Device) {
    if (!hasFiles(e) || !x.paired) return;
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer!.dropEffect = "copy";
    dropRow = x.id;
  }
  function rowDrop(e: DragEvent, x: Device) {
    dropRow = null;
    if (!hasFiles(e) || !x.paired) return;
    devices.picked = x.id;
    dropOnDevice(e, x, (ok, message) => {
      if (!ok) app.toast(message, "error");
    });
  }

  // ---------- this PC ----------
  const shortId = (id: string) => `${id.slice(0, 6)}…${id.slice(-6)}`;
  function copyId() {
    if (d?.myId) app.run(() => api.copyText(d.myId)).then(() => app.toast("This PC's Pako ID is copied. Enter it on your other PC, under My devices › Add a device."));
  }

  // ---------- this PC's name (renamed in place) ----------
  let editingName = $state(false);
  let nameDraft = $state("");
  let nameInput: HTMLInputElement | undefined = $state();
  function editName() {
    nameDraft = d?.pcName ?? "";
    editingName = true;
    queueMicrotask(() => {
      nameInput?.focus();
      nameInput?.select();
    });
  }
  /** Enter or leaving the field saves; Esc (or an empty name) keeps the old one. */
  async function saveName() {
    if (!editingName) return;
    editingName = false;
    const n = nameDraft.trim();
    if (!n || n === d?.pcName) return;
    await app.run(async () => {
      await devicesApi.setPcName(n);
      app.toast(`This PC is now called ${n} on your devices.`);
      devices.refresh();
    });
  }
  function nameKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      saveName();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      editingName = false;
    }
  }

  // ---------- Add a device ----------
  let pcName = $state("");
  let newId = $state("");
  let newName = $state("");
  let folder = $state("");
  /** The folder was picked by hand: typing the name no longer changes it. */
  let folderPicked = $state(false);
  let addError = $state("");
  let busy = $state(false);
  /** This PC has no name for your devices yet (asked before pairing: the other PC sees it). */
  const nameMissing = $derived(!(d?.pcName ?? "").trim());
  function openAdd() {
    newId = "";
    newName = "";
    folder = "";
    folderPicked = false;
    addError = "";
    pcName = d?.pcName ?? "";
    devices.adding = true;
  }
  $effect(() => {
    if (!devices.adding || folderPicked) return;
    const n = newName;
    devicesApi.defaultFolder(n.trim() || "my device").then((f) => {
      if (!folderPicked && newName === n) folder = f;
    }, () => {});
  });
  async function chooseFolder(title: string): Promise<string | null> {
    if (!inApp) {
      app.toast("Preview mode: no folder picker.");
      return null;
    }
    const r = await pick({ directory: true, multiple: false, title });
    return typeof r === "string" ? r : null;
  }
  async function pickAddFolder() {
    const f = await chooseFolder("Where files from this device are saved");
    if (f) {
      folder = f;
      folderPicked = true;
    }
  }
  async function addDevice() {
    addError = "";
    if (nameMissing && !pcName.trim()) {
      addError = "Name this PC first: your other PC sees this name.";
      return;
    }
    busy = true;
    try {
      if (nameMissing) await devicesApi.setPcName(pcName.trim());
      await devicesApi.add(newId.trim(), newName.trim(), folder || null);
      devices.adding = false;
      devices.picked = null;
      app.toast("Paired on this PC. Now confirm on the other one: it asks you there.");
      devices.refresh();
    } catch (e) {
      addError = String(e);
    } finally {
      busy = false;
    }
  }

  // ---------- pairing questions ----------
  // `request`: a PC that isn't a contact (its name is the one it chose; `knownAs`: one of yours
  // already has that name). Else one of your Direct contacts says this PC is its device.
  type Ask = { id: string; name: string; code: string; request: boolean; knownAs: string | null };
  const asks = $derived<Ask[]>([
    ...(d?.requests ?? []).map((r: DirectRequest) => ({ id: r.peer, name: r.name, code: r.code, request: true, knownAs: r.knownAs ?? null })),
    ...list.filter((x) => x.asksToPair && !devices.notNow.includes(x.id)).map((x) => ({ id: x.id, name: x.name, code: x.code, request: false, knownAs: null })),
  ]);
  // The answers wake up a moment after a question appears (like Direct's questions): a click
  // meant for something else, as the card slides in, can't pair a PC.
  let armedAt = $state<Record<string, number>>({});
  let now = $state(Date.now());
  $effect(() => {
    const fresh = asks.filter((a) => untrack(() => armedAt[a.id]) === undefined);
    if (!fresh.length) return;
    const t0 = Date.now();
    untrack(() => (armedAt = { ...armedAt, ...Object.fromEntries(fresh.map((a) => [a.id, t0 + 700])) }));
    // Harmless if the page has gone meanwhile.
    setTimeout(() => (now = Date.now()), 720);
  });
  const armed = (a: Ask) => (armedAt[a.id] ?? Infinity) <= now;
  let answering = $state<string | null>(null);
  /** This PC's name, asked on the question when it has none yet (the other PC sees it). */
  let askName = $state("");
  let needName = $state(false);
  $effect(() => {
    if (askName.trim()) needName = false;
  });
  /** The receiving folder chosen for each question (default: <download folder>\From <name>). */
  let askFolders = $state<Record<string, string>>({});
  $effect(() => {
    for (const a of asks) {
      if (askFolders[a.id] !== undefined) continue;
      askFolders = { ...askFolders, [a.id]: "" };
      devicesApi.defaultFolder(a.name).then((f) => (askFolders = { ...askFolders, [a.id]: askFolders[a.id] || f }), () => {});
    }
  });
  async function confirm(a: Ask, yes: boolean) {
    if (!armed(a) || answering) return;
    // "Not now" on a contact's question: it leaves until Pako restarts (or you pair it later
    // with Add a device). Nothing is said to them.
    if (!a.request && !yes) {
      devices.notNow = [...devices.notNow, a.id];
      return;
    }
    // A PC without a name yet (often the second one, fresh): asked here, like Direct does.
    if (yes && nameMissing && !askName.trim()) {
      needName = true;
      return;
    }
    const f = askFolders[a.id] || null;
    answering = a.id;
    try {
      await app.run(async () => {
        if (yes && nameMissing) await devicesApi.setPcName(askName.trim());
        if (a.request) await devicesApi.answer(a.id, yes, f);
        else await devicesApi.add(a.id, null, f);
        if (yes) {
          devices.picked = a.id;
          app.toast(`${a.name} is paired. Files between your two PCs now go without asking.`);
        }
        devices.refresh();
      });
    } finally {
      answering = null;
    }
  }
  async function changeAskFolder(a: Ask) {
    const f = await chooseFolder(`Where files from ${a.name} are saved`);
    if (f) askFolders = { ...askFolders, [a.id]: f };
  }
</script>

<div class="devices">
  <div class="toolbar">
    <h1>My devices</h1>
    <TutorialButton id="devices" />
    <span class="dim small sub">Your own PCs: send files between them, see and start their downloads</span>
  </div>

  {#if !direct}
    <div class="center dim">Starting…</div>
  {:else if !direct.ready || !d}
    <div class="center dim">{direct.error ?? "Getting ready…"}</div>
  {:else}
    {#if asks.length}
      <div class="asks">
        {#each asks as a (a.id)}
          <section class="ask card" data-tour="devices-ask" transition:rowSlide={{ duration: 240 }}>
            <span class="avatar">{initial(a.name)}</span>
            <div class="asktext">
              {#if a.request}
                <div class="askhead"><b>{a.name}</b> wants to pair with this PC as one of your devices.</div>
                {#if a.knownAs}
                  <p class="warn">You already have a device or contact called {a.knownAs}. This is a different PC: if you expected {a.knownAs}, decline.</p>
                {/if}
              {:else}
                <div class="askhead"><b>{a.name}</b>, one of your Direct contacts, says this PC is one of its devices.</div>
              {/if}
              <p class="dim">Check that <b>{a.name}</b> shows the same safety code: <span class="mono code">{a.code}</span>. If it doesn't, {a.request ? "decline" : "don't confirm"}. Only for your own PCs: it can then send files here without asking, see and add to this PC's downloads, and get finished ones.</p>
              <div class="frow">
                <span class="mlabel">Its files are saved in</span>
                <span class="path mono" title={askFolders[a.id]}>{askFolders[a.id] || "…"}</span>
                <button class="btn small" onclick={() => changeAskFolder(a)}>Change…</button>
                {#if nameMissing}
                  <input class="input small" class:missing={needName} bind:value={askName} placeholder="This PC's name (like Desk PC)" maxlength="60" aria-label="This PC's name" aria-invalid={needName} />
                {/if}
                <div class="askgo">
                  <button class="btn small ghost" disabled={answering === a.id} onclick={() => confirm(a, false)}>{a.request ? "Decline" : "Not now"}</button>
                  <button class="btn small primary" class:arming={!armed(a)} aria-disabled={!armed(a)} disabled={answering === a.id} onclick={() => confirm(a, true)}><Icon name="check" size={12} stroke={2.4} />This is my device</button>
                </div>
              </div>
            </div>
          </section>
        {/each}
      </div>
    {/if}

    <div class="stage">
      {#if !list.length}
        <div class="body solo" transition:fadeIn={{ duration: 220 }}>
          <section class="card welcome" data-tour="devices-welcome">
            <div class="empty-icon"><Icon name="devices" size={26} /></div>
            <h3>Your PCs, together</h3>
            <p class="dim">Pair your own PCs once, like this one and your laptop. Then send files between them without anyone accepting, see the other PC's downloads live, and start downloads on it from here. Everything goes PC to PC, encrypted: on your home network it doesn't even use the internet.</p>
            <div class="idbox">
              <span class="mlabel">This PC's Pako ID</span>
              <div class="idrow">
                <span class="mono idtext" title={d.myId}>{d.myId}</span>
                <button class="btn small" onclick={copyId}><Icon name="copy" size={12} />Copy</button>
              </div>
              <span class="hint">On your other PC, open My devices › Add a device and enter this ID, or enter that PC's ID here.</span>
            </div>
            <button class="btn primary" onclick={openAdd}><Icon name="plus" size={14} stroke={2.2} />Add a device</button>
          </section>
        </div>
      {:else}
        <div class="body" transition:fadeIn={{ duration: 220 }}>
          <aside class="side card" data-tour="devices-list">
            <div class="phead">
              <span class="ptitle">Your devices</span>
              <span class="pcount mono">{list.length}</span>
              <div class="spacer"></div>
              <button class="btn small" onclick={openAdd} title="Pair another of your PCs"><Icon name="plus" size={12} stroke={2.2} />Add</button>
            </div>
            <div class="plist">
              {#each list as x (x.id)}
                {@const m = moving(x.id)}
                <button
                  class="person"
                  class:on={current?.id === x.id}
                  class:dropon={dropRow === x.id}
                  onclick={() => (devices.picked = x.id)}
                  ondragover={(e) => rowOver(e, x)}
                  ondragleave={() => dropRow === x.id && (dropRow = null)}
                  ondrop={(e) => rowDrop(e, x)}
                  title={x.paired ? `Drop files here to send them to ${x.name}` : undefined}
                  transition:rowSlide={{ duration: 220 }} animate:flipIf={flipOpts}>
                  <span class="avatar dev"><Icon name="devices" size={15} /><span class="dot" class:online={x.online} class:local={onLan(x)} class:wait={!x.paired}></span></span>
                  <span class="ptext">
                    <span class="pname">{x.name}</span>
                    {#if m.length}
                      <span class="pstat live"><Icon name={m.some((t) => t.outgoing) ? "up" : "down"} size={11} stroke={2.4} />{m.some((t) => t.outgoing) ? "Sending" : "Receiving"} {rowPct(x.id)}%</span>
                    {:else}
                      <span class="pstat">{status(x)}</span>
                    {/if}
                  </span>
                  {#if x.asksToPair}<span class="badge" title="Waiting for your answer" transition:pop={{ duration: 180, from: 0.92, y: 0 }}>!</span>{/if}
                  {#if m.length}<span class="rowbar" aria-hidden="true" transition:fadeIn={{ duration: 200 }}><i style:transform="scaleX({rowPct(x.id) / 100})"></i></span>{/if}
                </button>
              {/each}
            </div>
            <div class="me">
              <span class="mlabel">This PC</span>
              {#if editingName}
                <input
                  class="input mename-edit"
                  bind:this={nameInput}
                  bind:value={nameDraft}
                  maxlength="60"
                  placeholder="Like Desk PC"
                  aria-label="This PC's name"
                  onkeydown={nameKey}
                  onblur={saveName} />
              {:else}
                <button class="mename" class:unset={!d.pcName} title="Rename this PC (your other PCs see this name)" onclick={editName}>
                  <span class="mtext">{d.pcName || "Name this PC"}</span><Icon name="pencil" size={11} />
                </button>
              {/if}
              <div class="idrow">
                <span class="mono idtext" title={d.myId}>{shortId(d.myId)}</span>
                <button class="btn small" onclick={copyId}><Icon name="copy" size={12} />Copy ID</button>
              </div>
            </div>
          </aside>
          {#if current}
            {#key current.id}
              <div class="right" in:fadeIn={{ duration: 160 }}>
                <DeviceDetails device={current} state={d} />
              </div>
            {/key}
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if devices.adding && d}
  <Modal title="Add a device" width={520} onclose={() => (devices.adding = false)}>
    <div class="form">
      <p class="dim lead">Pair another of your own PCs. Do this on both PCs, or enter one PC's ID on the other: the other PC then asks you to confirm.</p>
      <span class="mlabel">This PC's Pako ID</span>
      <div class="idrow boxed">
        <span class="mono idtext" title={d.myId}>{d.myId}</span>
        <button class="btn small" onclick={copyId}><Icon name="copy" size={12} />Copy</button>
      </div>
      {#if nameMissing}
        <label class="mlabel" for="pcname">This PC's name</label>
        <input id="pcname" class="input" bind:value={pcName} placeholder="Like Desk PC (your other PC sees it)" maxlength="60" />
      {/if}
      <div class="sep"></div>
      <label class="mlabel" for="devid">The other PC's Pako ID</label>
      <input id="devid" class="input mono" bind:value={newId} placeholder="On that PC: My devices › Add a device › Copy" spellcheck="false" />
      <label class="mlabel" for="devname">Its name</label>
      <input id="devname" class="input" bind:value={newName} placeholder="Like Laptop or Home PC" maxlength="60" onkeydown={(e) => e.key === "Enter" && addDevice()} />
      <span class="mlabel">Files from it are saved in</span>
      <div class="idrow boxed">
        <span class="mono idtext" title={folder}>{folder || "…"}</span>
        <button class="btn small" onclick={pickAddFolder}><Icon name="folder" size={12} />Change…</button>
      </div>
      <p class="dim small">Files from this device are saved here, and only here. It can't pick another place on this PC.</p>
      {#if addError}<p class="err" role="alert">{addError}</p>{/if}
      <p class="dim small">The other PC asks you to confirm and shows a safety code: check that it's the same on both PCs.</p>
    </div>
    {#snippet actions()}
      <button class="btn ghost" onclick={() => (devices.adding = false)}>Cancel</button>
      <button class="btn primary" disabled={!newId.trim() || !newName.trim() || busy} onclick={addDevice}>Pair this device</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .devices {
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
    text-align: center;
    padding: 24px;
  }
  .card {
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    min-height: 0;
  }
  /* ---------- pairing questions ---------- */
  .asks {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px 0;
    /* Many questions at once (anyone on the network can ask): they scroll, the page stays. */
    max-height: 45%;
    overflow-y: auto;
  }
  .ask .warn {
    padding: 6px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--st-retry) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--st-retry) 30%, transparent);
    color: var(--text);
  }
  .ask .input.small {
    width: 210px;
    height: 28px;
    font-size: 12px;
  }
  .ask .input.missing {
    border-color: var(--st-fail-text);
  }
  /* Just shown: "This is my device" fades in and doesn't take clicks yet. */
  .askgo .btn.primary {
    transition: opacity var(--slow) var(--ease);
  }
  .askgo .btn.primary.arming {
    opacity: 0.55;
    pointer-events: none;
  }
  .ask {
    flex: none;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    border-color: var(--accent);
  }
  .asktext {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .askhead {
    line-height: 1.45;
  }
  .ask p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
  }
  .code {
    color: var(--text);
    letter-spacing: 0.04em;
    white-space: nowrap;
  }
  .frow {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
    min-width: 0;
    font-size: 12.5px;
  }
  .path {
    min-width: 0;
    flex: 0 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-2);
  }
  /* The answers end the folder's row (on a narrow window they wrap under it, on the right). */
  .askgo {
    flex: none;
    display: flex;
    gap: 8px;
    margin-left: auto;
  }
  /* ---------- the page ---------- */
  .stage {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
  }
  .stage > .body {
    grid-area: 1 / 1;
    min-width: 0;
  }
  .body {
    min-height: 0;
    display: flex;
    padding: 12px 16px 16px;
  }
  .body.solo > :global(*) {
    flex: 1;
  }
  .side {
    flex: none;
    width: 270px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .right {
    flex: 1;
    min-width: 0;
    margin-left: 12px;
    display: flex;
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
    max-width: 480px;
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
  .idbox {
    width: min(540px, 100%);
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
  .hint {
    font-size: 12px;
    color: var(--text-2);
    line-height: 1.45;
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
  }
  .pcount {
    font-size: 11px;
    color: var(--text-3);
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
  .person.dropon {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1.5px var(--accent);
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
  .avatar.dev {
    border-radius: 9px;
    color: var(--text-2);
  }
  .dot {
    position: absolute;
    right: -2px;
    bottom: -2px;
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
  .pstat.live {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  .pstat.live :global(svg) {
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
  .me {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border-top: 1px solid var(--line);
  }
  /* This PC's name: looks like text, renamed in place with a click. Both states are 28 px
     high, so nothing moves when the field opens. */
  .mename {
    align-self: flex-start;
    max-width: 100%;
    height: 28px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    /* Its hover box reaches past the text, not the text past the column. */
    margin: -3px -6px;
    padding: 0 6px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    font-weight: 500;
    text-align: left;
    transition: background var(--fast) ease;
  }
  .mename .mtext {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mename :global(svg) {
    flex: none;
    color: var(--text-3);
    opacity: 0;
    transition: opacity var(--fast) ease;
  }
  .mename:hover {
    background: var(--raise-hover);
  }
  .mename:hover :global(svg),
  .mename:focus-visible :global(svg),
  .mename.unset :global(svg) {
    opacity: 1;
  }
  .mename.unset {
    color: var(--focus-text);
    font-weight: 400;
  }
  .mename-edit {
    width: calc(100% + 12px);
    height: 28px;
    /* Its hover box reaches past the text, not the text past the column. */
    margin: -3px -6px;
    padding: 0 6px;
    font-weight: 500;
  }
  .mlabel {
    font-size: 11px;
    color: var(--text-3);
  }
  .idrow {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .idrow.boxed {
    padding: 6px 6px 6px 10px;
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    background: var(--panel);
  }
  .idtext {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn.small {
    height: 28px;
    font-size: 12px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form p {
    margin: 4px 0 0;
    line-height: 1.5;
  }
  .form .lead {
    margin: 0 0 6px;
  }
  .form .mlabel {
    margin-top: 4px;
  }
  .sep {
    height: 1px;
    margin: 8px 0 2px;
    background: var(--line);
  }
  .err {
    color: var(--st-fail-text);
    font-size: 12.5px;
  }
  @container (max-width: 760px) {
    .sub {
      display: none;
    }
  }
</style>
