<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // The next send, before anything uploads: picked and dropped files and folders gather here,
  // with where they go and how, then one Send makes one send with one link.
  import { api, type DriveSendStatus, type SendMode } from "./api";
  import { app, type BasketItem } from "./state.svelte";
  import { bytes } from "./format";
  import Icon from "./Icon.svelte";
  import FileIcon from "./FileIcon.svelte";
  import MiddleText from "./MiddleText.svelte";
  import SegPick from "./SegPick.svelte";
  import PartSize from "./PartSize.svelte";
  import { rowSlide, fadeIn } from "./motion";
  import { DELETE_CHOICES, PARTS_NOTE, PART_DEFAULT_MB, partOk } from "./uploads";

  let {
    onaddfiles = () => {},
    onaddfolder = () => {},
    onsend,
    onroom,
    basket = null,
    onremove = null,
  }: {
    onaddfiles?: () => void;
    onaddfolder?: () => void;
    /** Its own files instead of Upload's basket (Private mode's Upload…): no Add buttons and no
     *  paths shown (a private file's place on disk means nothing to you). */
    basket?: BasketItem[] | null;
    onremove?: ((path: string) => void) | null;
    /** `mode` null = the setting; `name` null = Pako's usual name; `deleteDays` 0 = keep;
     *  `partMb` = "ZIP in parts"' part size (null when not in parts). */
    onsend: (mode: SendMode | null, name: string | null, deleteDays: number, partMb: number | null, password: string | null) => Promise<void>;
    /** "Make room…" in your Google Drive. */
    onroom: () => void;
  } = $props();

  const MODES: [SendMode, string][] = [
    ["files", "As they are"],
    ["zip", "One ZIP"],
    ["compress", "Compressed ZIP"],
    ["parts", "ZIP in parts"],
  ];
  /** One file: as it is, compressed into a ZIP (a plain ZIP of one file adds nothing), or a ZIP
   *  in parts (a big file over a shaky connection). */
  const SINGLE_MODES: [SendMode, string][] = [
    ["files", "As it is"],
    ["compress", "Compressed ZIP"],
    ["parts", "ZIP in parts"],
  ];
  /** The setting (older compression setting carried over until a choice is saved). */
  const settingMode = $derived<SendMode>(
    app.settings?.sendMode ?? (app.settings?.sendCompression === "auto" || app.settings?.sendCompression === "max" ? "compress" : "files"),
  );
  /** For this send only; null = the setting. */
  let picked = $state<SendMode | null>(null);
  const mode = $derived<SendMode>(picked ?? settingMode);
  let name = $state("");
  let sending = $state(false);

  const own = $derived(basket !== null);
  const items = $derived(basket ?? app.basket);
  const measured = $derived(items.every((b) => b.size != null));
  const totalSize = $derived(items.reduce((t, b) => t + (b.size ?? 0), 0));
  const totalFiles = $derived(items.reduce((t, b) => t + (b.files ?? (b.dir ? 0 : 1)), 0));
  const unreadable = $derived(items.filter((b) => b.error));
  /** One file on its own: no folder or ZIP to name, no "several files" choice. */
  const single = $derived(items.length === 1 && !items[0].dir);
  const several = $derived(!single && (totalFiles > 1 || items.some((b) => b.dir)));
  /** One file goes as it is unless "Compressed ZIP" or "ZIP in parts" is picked for it here (the usual setting for
   *  several files doesn't apply to a single one). */
  const singleMode = $derived<SendMode>(picked === "compress" || picked === "parts" ? picked : "files");
  /** "ZIP in parts" is the way this upload goes. */
  const inParts = $derived(single ? singleMode === "parts" : several && mode === "parts");
  /** Its part size: the setting, changed here for this upload only. */
  let partMb = $state(partOk(app.settings?.sendPartMb) ? app.settings!.sendPartMb : PART_DEFAULT_MB);
  let partCustom = $state(app.settings?.sendPartCustom ?? false);
  let partValid = $state(true);
  /** "Protect with a password" (this upload only, never kept). */
  let protect = $state(false);
  let pass1 = $state("");
  let pass2 = $state("");
  const passOk = $derived(pass1.length >= 8 && pass1 === pass2);
  const blocked = $derived((inParts && !partValid) || (protect && (!passOk || !app.sevenZip)));

  /** The name Pako would use (shown as the name field's hint). */
  const usualName = $derived.by(() => {
    if (!items.length) return "";
    if ((single ? singleMode : mode) === "files") return items.length === 1 && items[0].dir ? items[0].name : `${totalFiles.toLocaleString()} files`;
    if (items.length === 1) return `${items[0].name}.zip`;
    const stem = items[0].name.includes(".") ? items[0].name.replace(/\.[^.]+$/, "") : items[0].name;
    return `${stem} and ${items.length - 1} more.zip`;
  });

  const plural = (n: number, one: string, many = `${one}s`) => `${n.toLocaleString()} ${n === 1 ? one : many}`;

  /** What "Delete after" means for where it goes. */
  const deleteNote = $derived(
    app.uploadDeleteDays === 0
      ? app.sendHost === "drive"
        ? "Stays in your Drive until you delete it"
        : "Kept until you delete it (Gofile may remove files nobody downloads for 10 days)"
      : app.sendHost === "drive"
        ? "Pako deletes it from your Drive on that day (or the next time it runs)"
        : "Deleted on that day: by Gofile if it accepts the date (even with your PC off), else by Pako",
  );

  // ---------- Google Drive: does it fit? ----------
  let drive = $state<DriveSendStatus | null>(null);
  let need = $state<number | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    // Asked again when the basket, the way it's sent or the destination changes.
    const host = app.sendHost;
    const paths = items.filter((b) => !b.error).map((b) => b.path);
    const m = single ? singleMode : mode;
    void measured;
    void app.driveTick;
    clearTimeout(timer);
    need = null;
    if (host !== "drive" || !paths.length) return;
    // An answer for an older basket (it changed while Pako measured) is dropped.
    let stale = false;
    timer = setTimeout(async () => {
      try {
        const st = await api.sendDriveStatus();
        if (stale) return;
        drive = st;
        if (st.configured && st.signedIn) {
          const n = await api.sendMeasure(paths, m);
          if (!stale) need = n;
        }
      } catch {
        if (!stale) drive = null;
      }
    }, 400);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });
  const free = $derived(drive?.limit != null && drive.used != null ? Math.max(0, drive.limit - drive.used) : null);
  const fits = $derived(need == null || free == null ? null : need <= free);

  async function send() {
    if (sending || !items.length || unreadable.length || blocked) return;
    sending = true;
    try {
      // A single file always says how it goes; several files use the choice or the setting.
      const how = single ? singleMode : picked && picked !== settingMode ? picked : null;
      const named = single && singleMode === "files" ? null : name.trim() || null;
      // In parts, the part size always goes with it (the one shown here).
      await onsend(inParts ? "parts" : how, named, app.uploadDeleteDays, inParts ? partMb : null, protect ? pass1 : null);
      if (!items.length) {
        name = "";
        picked = null;
        protect = false;
        pass1 = "";
        pass2 = "";
      }
    } finally {
      sending = false;
    }
  }
</script>

<section class="basket" data-tour="send-basket" aria-label="New upload">
  <header>
    <h2>New upload</h2>
    <span class="dim">
      {plural(items.length, "item")}{#if measured}{` · ${plural(totalFiles, "file")} · ${bytes(totalSize)}`}{:else}{" · measuring…"}{/if}
    </span>
    <span class="grow"></span>
    {#if !own}
      <button class="btn ghost small" onclick={onaddfolder}><Icon name="folder" size={13} />Add folder</button>
      <button class="btn ghost small" onclick={onaddfiles}><Icon name="plus" size={13} stroke={2.2} />Add files</button>
    {/if}
  </header>

  <ul class="items">
    {#each items as b (b.path)}
      <li class:bad={!!b.error} transition:rowSlide>
        <FileIcon name={b.name} kind={b.dir ? "folder" : undefined} size={18} />
        <div class="nm-wrap">
          <MiddleText text={b.name} class="nm" />
          <span class="sub" title={own || b.private ? undefined : b.path}>
            {#if b.error}{b.error}{:else if b.dir}{b.files == null ? "Counting files…" : plural(b.files, "file")}{:else if own || b.private}From Private mode{:else}{b.path}{/if}
          </span>
        </div>
        <span class="size mono">{b.size == null ? "…" : bytes(b.size)}</span>
        <button class="icon-btn" aria-label="Remove {b.name} from this upload" title="Remove from this upload" onclick={() => (onremove ? onremove(b.path) : app.removeFromBasket(b.path))}><Icon name="x" size={13} /></button>
      </li>
    {/each}
  </ul>

  <div class="opts" data-tour="send-options">
    <span class="lbl">Upload to</span>
    <SegPick
      label="Upload to"
      options={[
        { value: "gofile", label: "Gofile", icon: "globe" },
        { value: "drive", label: "Google Drive", icon: "drive" },
      ]}
      value={app.sendHost}
      onpick={(v) => app.setSendHost(v)}
    />
    {#key app.sendHost}
      <span class="note dim" in:fadeIn={{ duration: 160 }}>{app.sendHost === "drive" ? "From your own Drive, shared with anyone who has the link" : "Free, no account needed"}</span>
    {/key}

    <span class="lbl">Delete after</span>
    <SegPick label="Delete after" options={DELETE_CHOICES} value={app.uploadDeleteDays} onpick={(v) => app.setUploadDeleteDays(v)} />
    {#key deleteNote}
      <span class="note dim" in:fadeIn={{ duration: 160 }} title={deleteNote}>{deleteNote}</span>
    {/key}

    {#if single}
      <span class="lbl">Compression</span>
      <SegPick label="How the file is uploaded" options={SINGLE_MODES.map(([value, label]) => ({ value, label }))} value={singleMode} onpick={(v) => (picked = v)} />
      {#key singleMode}
        <span class="note dim" in:fadeIn={{ duration: 160 }}>{singleMode === "compress" ? "Packed into a smaller ZIP file (helps documents, not videos or photos)" : singleMode === "parts" ? "Packed into a ZIP, cut into numbered parts" : "Uploaded as it is"}</span>
      {/key}
      {#if singleMode !== "files"}
        <span class="lbl">Name</span>
        <input class="input" bind:value={name} placeholder={usualName} maxlength="120" spellcheck="false" aria-label="Name of this upload" />
        <span class="note dim">The ZIP file's name, optional</span>
      {/if}
    {/if}

    {#if several}
      <span class="lbl">Several files</span>
      <SegPick label="How several files are uploaded" options={MODES.map(([value, label]) => ({ value, label }))} value={mode} onpick={(v) => (picked = v)} />
      {#key mode}
        <span class="note dim" in:fadeIn={{ duration: 160 }}>{mode === "files" ? "One link to a folder with the files as they are" : mode === "zip" ? "Packed into one ZIP file, not compressed (fastest)" : mode === "parts" ? "Packed into one ZIP, cut into numbered parts" : "Packed into one smaller ZIP file (takes longer)"}</span>
      {/key}

      <span class="lbl">Name</span>
      <input class="input" bind:value={name} placeholder={usualName} maxlength="120" spellcheck="false" aria-label="Name of this upload" />
      <span class="note dim">{mode === "files" ? "The folder's name" : "The ZIP file's name"}, optional</span>
    {/if}

    <span class="lbl">Password</span>
    <div class="protect">
      <button class="switch" class:on={protect} role="switch" aria-checked={protect} aria-label="Protect with a password" onclick={() => (protect = !protect)}></button>
      <span>Protect with a password</span>
    </div>
    {#key protect}
      <span class="note dim" in:fadeIn={{ duration: 160 }}>{protect ? "Locked before it leaves this PC; names hidden too" : "Anyone with the link can open it"}</span>
    {/key}
    {#if protect}
      <span class="lbl top" transition:fadeIn={{ duration: 160 }}></span>
      <div class="pwds" transition:rowSlide={{ duration: 200 }}>
        <input class="input" type="password" autocomplete="new-password" placeholder="Password" aria-label="Password for this upload" bind:value={pass1} />
        <input class="input" type="password" autocomplete="new-password" placeholder="Type it again" aria-label="Type the password again" bind:value={pass2} />
      </div>
      <span class="explain dim" transition:fadeIn={{ duration: 160 }}>
        {#if !app.sevenZip}
          This needs 7-Zip: Settings › Features & updates › What Pako can do.
        {:else if pass1 && pass1.length < 8}
          At least 8 characters.
        {:else if pass2 && pass1 !== pass2}
          They don't match yet.
        {:else}
          It goes up as a 7-Zip archive (.7z). People open it with 7-Zip (free) or Pako, and this password. Pako doesn't keep it: tell them yourself, not in the same message as the link.
        {/if}
      </span>
    {/if}

    {#if inParts}
      <span class="lbl top" transition:fadeIn={{ duration: 160 }}>Part size</span>
      <div class="partsize" transition:fadeIn={{ duration: 160 }}>
        <PartSize bind:mb={partMb} bind:custom={partCustom} bind:valid={partValid} label="Size of each part" />
      </div>
      <span class="explain dim" transition:fadeIn={{ duration: 160 }}>{PARTS_NOTE}</span>
    {/if}
  </div>

  {#if app.sendHost === "drive" && items.length}
    <div class="fit" class:ok={fits === true} class:no={fits === false}>
      {#if drive && !drive.configured}
        <Icon name="alert" size={13} /><span>Uploading to Google Drive isn't set up in this Pako. Choose Gofile to upload.</span>
      {:else if drive && !drive.signedIn}
        <Icon name="alert" size={13} /><span>Connect Google Drive first (the bar above), or choose Gofile.</span>
      {:else if fits === true}
        <Icon name="check" size={13} stroke={2.4} /><span>This upload needs {bytes(need ?? 0)}. {bytes(free ?? 0)} free in your Google Drive.</span>
      {:else if fits === false}
        <Icon name="alert" size={13} /><span>This upload needs {bytes(need ?? 0)}, but only {bytes(free ?? 0)} is free in your Google Drive.</span>
        <button class="btn ghost small" onclick={onroom}><Icon name="trash" size={12} />Make room…</button>
        <button class="btn ghost small" onclick={() => app.setSendHost("gofile")}>Upload to Gofile instead</button>
      {:else}
        <span class="dim">Checking your Google Drive's free space…</span>
      {/if}
    </div>
  {/if}

  <footer>
    {#if unreadable.length}
      <span class="warn">{unreadable.length === 1 ? "1 item can't be read" : `${unreadable.length} items can't be read`}. Remove {unreadable.length === 1 ? "it" : "them"} to upload the rest.</span>
    {/if}
    <span class="grow"></span>
    <button class="btn ghost" disabled={sending} onclick={() => app.clearBasket()}>Clear</button>
    <button class="btn primary" data-tour="send-go" disabled={sending || !items.length || unreadable.length > 0 || blocked} onclick={send}>
      <Icon name="upload" size={14} stroke={2.2} />{sending ? "Starting…" : "Upload"}
    </button>
  </footer>
</section>

<style>
  /* A raised card like the Settings sections. */
  .basket {
    margin: 14px 0 6px;
    padding: 14px 16px 16px;
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  h2 {
    margin: 0 4px 0 0;
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
  }
  .dim {
    color: var(--text-3);
  }
  header .dim {
    font-size: 12.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .grow {
    flex: 1;
  }
  .btn.small {
    height: 28px;
    font-size: 12px;
    flex: none;
  }
  .items {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    max-height: 232px;
    overflow-y: auto;
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    background: var(--panel);
  }
  .items li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto 28px;
    align-items: center;
    gap: 12px;
    min-height: 46px;
    padding: 6px 8px 6px 12px;
  }
  .items li + li {
    border-top: 1px solid var(--line-soft);
  }
  .nm-wrap {
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .nm-wrap :global(.nm) {
    min-width: 0;
    color: var(--text);
    font-weight: 500;
  }
  .sub {
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bad .sub {
    color: var(--st-fail-text);
  }
  .size {
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
  }
  /* Options: labels on the left, controls on one column, a short note after each. */
  .opts {
    display: grid;
    grid-template-columns: 96px auto minmax(0, 1fr);
    align-items: center;
    gap: 10px 12px;
    margin-top: 14px;
  }
  .lbl {
    font-size: 12.5px;
    color: var(--text-2);
  }
  /* Next to a control that can grow below (the part size's range note): lined up with its
     first row, so it doesn't move when the note shows. */
  .lbl.top {
    align-self: start;
    margin-top: 8.5px;
  }
  .note {
    font-size: 12px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .opts > .input {
    width: 280px;
    height: 30px;
  }
  /* The part size spans the control and note columns (its Custom field and range note sit in
     it); the explanation goes under it, in the same columns. */
  .partsize {
    grid-column: 2 / -1;
    min-width: 0;
  }
  .protect {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .pwds {
    grid-column: 2 / -1;
    display: flex;
    gap: 8px;
  }
  .pwds .input {
    flex: 1;
    min-width: 0;
  }
  .explain {
    grid-column: 2 / -1;
    margin-top: -4px;
    font-size: 12px;
    line-height: 1.45;
  }
  .fit {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
    padding: 8px 12px;
    border: 1px solid var(--line-soft);
    border-radius: var(--radius);
    background: var(--panel);
    font-size: 12.5px;
    color: var(--text-2);
  }
  .fit.ok :global(svg) {
    color: var(--st-done-text);
  }
  .fit.no {
    border-color: rgba(242, 75, 79, 0.3);
    background: var(--tint-fail);
  }
  .fit.no :global(svg) {
    color: var(--st-fail-text);
  }
  .fit span {
    flex: 1;
    min-width: 200px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 16px;
  }
  .warn {
    font-size: 12px;
    color: var(--st-fail-text);
  }
  @container (max-width: 760px) {
    .opts {
      grid-template-columns: 96px minmax(0, 1fr);
    }
    .note {
      display: none;
    }
    .opts > .input {
      width: 100%;
    }
  }
  /* A small window (or the details panel open): each label sits above its choices, so the
     widest row of choices (Several files, four of them) fits inside the card. */
  @container (max-width: 580px) {
    .basket {
      padding: 12px 12px 14px;
    }
    .opts {
      grid-template-columns: minmax(0, 1fr);
      gap: 6px;
    }
    .opts .lbl:not(:first-child) {
      margin-top: 6px;
    }
    .opts .lbl.top {
      align-self: auto;
    }
    .partsize,
    .explain {
      grid-column: 1;
    }
    .explain {
      margin-top: 0;
    }
  }
</style>
