<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import Credit from "../Credit.svelte";
  // Help: the guides (docs/help, bundled at build time), What's new, and the tutorials.
  import { tick } from "svelte";
  import { app } from "../state.svelte";
  import Icon from "../Icon.svelte";
  import Modal from "../Modal.svelte";
  import SegPick from "../SegPick.svelte";
  import { bare, openLink, openWeb, SITE } from "../web";
  import { fadeIn, pop, reduced } from "../motion";
  import { tour } from "../tour/tour.svelte";
  import { PAGE_TOURS, type TourId } from "../tour/tours";
  import { guideByFile, guides as guidesShown, licence, notice, search, terms, whatsNew as whatsNewShown } from "./content";
  import { around, t, tMaybe } from "../i18n.svelte";
  import { help } from "./help.svelte";
  import Markdown from "./Markdown.svelte";

  let q = $state("");
  let pane = $state<HTMLDivElement | null>(null);
  // In the language shown (French guides where there are some): they follow a switch at once.
  const guides = $derived(guidesShown());
  const whatsNew = $derived(whatsNewShown());
  const hits = $derived(search(q));
  const page = $derived(help.page);
  const guide = $derived(page.kind === "guide" ? guideByFile(page.file) : undefined);

  /** Which nav row is lit: 0 = Start, 1… = guides, then What's new and About Pako. */
  const navIndex = $derived(
    q.trim()
      ? -1
      : page.kind === "home"
        ? 0
        : page.kind === "whatsNew"
          ? guides.length + 1
          : page.kind === "about"
            ? guides.length + 2
            : guides.findIndex((g) => g.file === (page.kind === "guide" ? page.file : "")) + 1,
  );
  /** About Pako: which legal text shows under the notice. */
  let legal = $state<"use" | "notice" | "licence">("use");
  const pageKey = $derived(q.trim() ? "search" : page.kind === "guide" ? page.file : page.kind);

  // Sentences with a key or a link in the middle, in the language shown.
  const f1 = $derived(around(t("{key} opens Help anywhere in Pako."), "{key}"));
  const src = $derived(around(t("Pako comes with no warranty. You may share and change it under the GNU GPL version 3 with Sookera's additional terms. Its {source} is on GitHub."), "{source}"));

  const versionLabel = (v: string) => (/^\d/.test(v) ? t("Pako {v}", { v }) : tMaybe(v));

  // A guide opened on a section ("?" links, search, links between guides): scroll there and light it up.
  $effect(() => {
    void help.nonce;
    const p = help.page;
    const box = pane;
    if (!box) return;
    tick().then(() => {
      const anchor = p.kind === "guide" ? p.anchor : null;
      const el = anchor ? box.querySelector<HTMLElement>(`[data-anchor="${CSS.escape(anchor)}"]`) : null;
      if (!el) {
        box.scrollTop = 0;
        return;
      }
      box.scrollTo({ top: el.offsetTop - 12, behavior: reduced() ? "auto" : "smooth" });
      el.classList.remove("flash");
      void el.offsetWidth;
      el.classList.add("flash");
    });
  });

  function open(target?: string) {
    q = "";
    help.show(target);
  }

  function link(href: string) {
    // Web pages open in the browser, the same way as everywhere in Pako (https only), and
    // Sookera's support address (in the terms of use) in the email app.
    if (/^https?:\/\//.test(href) || href.startsWith("mailto:")) openLink(href);
    else if (href.startsWith("#") && page.kind === "guide") {
      help.show(`${page.file}${href}`);
    } else {
      const file = href.split("#")[0];
      if (guideByFile(file)) open(href);
    }
  }

  function startTour(id: TourId) {
    // Help opened from Settings (its "?" and About links): Settings is still open under it, over
    // the page the tutorial would show. Its own tutorial is fine.
    if (id !== "settings" && app.modals > 1) {
      app.toast(t("Close Settings first: this tutorial shows another page."));
      return;
    }
    help.close();
    tour.start(id);
  }

  function searchKey(e: KeyboardEvent) {
    // Esc with something typed clears it first; the next Esc closes Help.
    if (e.key === "Escape" && q) {
      e.stopPropagation();
      q = "";
    } else if (e.key === "Enter" && hits[0]) {
      const h = hits[0];
      open(h.section ? `${h.guide.file}#${h.section.id}` : h.guide.file);
    }
  }
</script>

<Modal title={t("Help")} width={880} onclose={() => help.close()}>
  <div class="help">
    <nav class="hnav" aria-label={t("Help")}>
      <label class="search">
        <Icon name="search" size={13} stroke={2} />
        <input bind:value={q} placeholder={t("Search the guides")} aria-label={t("Search the guides")} spellcheck="false" onkeydown={searchKey} />
        {#if q}<button class="clear" aria-label={t("Clear search")} onclick={() => (q = "")}><Icon name="x" size={10} stroke={2.4} /></button>{/if}
      </label>
      <div class="list focus-in">
        {#if navIndex >= 0}<span class="lit" style="transform: translateY({navIndex * 34}px)" aria-hidden="true"></span>{/if}
        <button class:on={navIndex === 0} onclick={() => open()}><Icon name="help" size={15} />{t("Start here")}</button>
        {#each guides as g, i (g.file)}
          <button class:on={navIndex === i + 1} onclick={() => open(g.file)}><Icon name="file" size={15} />{g.title}</button>
        {/each}
        <button class:on={navIndex === guides.length + 1} onclick={() => open("whats-new")}><Icon name="star" size={15} />{t("What's new")}</button>
        <button class:on={navIndex === guides.length + 2} onclick={() => open("about")}><Icon name="file" size={15} />{t("About Pako")}</button>
        <button onclick={() => (help.report = true)}><Icon name="chat" size={15} />{t("Report a problem")}</button>
      </div>
      <p class="f1">{f1[0]}<kbd>F1</kbd>{f1[1]}</p>
    </nav>

    <div class="pane" bind:this={pane}>
      {#key pageKey}
        <div class="page" in:pop={{ duration: 220, from: 1, y: 6 }}>
          {#if q.trim()}
            <h2>{t("Search")}</h2>
            {#if hits.length}
              <div class="hits">
                {#each hits as h, i (i)}
                  <button class="hit" onclick={() => open(h.section ? `${h.guide.file}#${h.section.id}` : h.guide.file)}>
                    <span class="where"><span class="dim">{h.guide.title}</span>{#if h.section}<span class="sep">›</span>{h.section.title}{/if}</span>
                    <span class="snip">{h.snippet}</span>
                  </button>
                {/each}
              </div>
            {:else}
              <p class="none">{t('Nothing in the guides matches "{q}". Try fewer or other words.', { q: q.trim() })}</p>
            {/if}
          {:else if page.kind === "home"}
            <section class="card tours">
              <div class="thead">
                <div>
                  <h2>{t("Tutorials")}</h2>
                  <p class="sub">{t("A spotlight walks you through the window. The quick tour shows what each page is for; each page's own tutorial shows its details.")}</p>
                </div>
                <button class="btn primary" onclick={() => startTour("full")}><Icon name="tour" size={14} stroke={2} />{t("Quick tour")}</button>
              </div>
              <div class="pages">
                {#each PAGE_TOURS as pt (pt.id)}
                  <button class="chip" onclick={() => startTour(pt.id)}><Icon name="play" size={10} stroke={2.2} />{t(pt.label)}</button>
                {/each}
              </div>
            </section>

            <h2 class="gtitle">{t("Guides")}</h2>
            <div class="guides">
              {#each guides as g (g.file)}
                <button class="guide" onclick={() => open(g.file)}>
                  <span class="gname">{g.title}</span>
                  <span class="gabout">{g.about}.</span>
                </button>
              {/each}
            </div>

            <div class="guides extra">
              <button class="guide" onclick={() => open("whats-new")}>
                <span class="gname"><Icon name="star" size={14} />{t("What's new")}</span>
                <span class="gabout">{t("The latest changes in Pako.")}{app.version ? " " + t("This is Pako {v}.", { v: app.version }) : ""}</span>
              </button>
              <button class="guide" onclick={() => (help.report = true)}>
                <span class="gname"><Icon name="chat" size={14} />{t("Report a problem")}</span>
                <span class="gabout">{t("Something went wrong? Send Sookera a report in one click, or by email.")}</span>
              </button>
            </div>
            <div class="credit-row"><Credit /></div>
          {:else if page.kind === "whatsNew"}
            <h2>{t("What's new")}</h2>
            {#each whatsNew as w, i (w.version)}
              <h3 class="ver" class:first={i === 0}>{versionLabel(w.version)}</h3>
              <Markdown blocks={w.blocks} onlink={link} />
            {/each}
          {:else if page.kind === "about"}
            <h2>{t("About Pako")}</h2>
            {@const by = around(t("Pako by {studio}"), "{studio}")}
            <p class="sub about">{by[0]}<button class="lk" onclick={() => openWeb(SITE.studio)}>Sookera</button>{by[1]}{app.version ? " · " + t("version {v}", { v: app.version }) : ""}</p>
            <section class="card notice">
              <p class="copy">Copyright (C) 2026 Sookera</p>
              <p class="warranty">{src[0]}<button class="lk" onclick={() => openWeb(SITE.source)}>{t("source code")}</button>{src[1]}</p>
              <SegPick
                small
                label={t("Legal text")}
                options={[
                  { value: "use", label: t("Terms of use") },
                  { value: "notice", label: t("Additional terms") },
                  { value: "licence", label: t("Licence") },
                ]}
                value={legal}
                onpick={(v) => (legal = v)}
              />
            </section>
            {#key legal}
              <div class="legal" in:pop={{ duration: 200, from: 1, y: 4 }}>
                {#if legal === "use"}
                  {@const on = around(t("Pako's terms of use, as you accepted them. The newest version is always on {link}."), "{link}")}
                  <p class="sub">{on[0]}<button class="lk" onclick={() => openWeb(SITE.terms)}>{bare(SITE.terms)}</button>{on[1]}</p>
                  <Markdown blocks={terms()} onlink={link} />
                {:else if legal === "notice"}
                  <Markdown blocks={notice} onlink={link} />
                {:else}
                  {@const on = around(t("The GNU General Public License version 3, as it comes with Pako. Also on {link}."), "{link}")}
                  <p class="sub">{on[0]}<button class="lk" onclick={() => openWeb(SITE.licence)}>gnu.org</button>{on[1]}</p>
                  <pre class="licence mono">{licence}</pre>
                {/if}
              </div>
            {/key}
          {:else if guide}
            <h2>{guide.title}</h2>
            <p class="sub about">{guide.about}.</p>
            <Markdown blocks={guide.blocks} onlink={link} />
          {:else}
            <p class="none" in:fadeIn>{t("This guide isn't here any more.")}</p>
          {/if}
        </div>
      {/key}
    </div>
  </div>
</Modal>

<style>
  .notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .copy {
    margin: 0;
    font-weight: 600;
    color: var(--text);
  }
  .warranty {
    margin: 0 0 8px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .legal {
    margin-top: 16px;
  }
  .licence {
    margin: 10px 0 0;
    padding: 14px 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: var(--bg);
    color: var(--text-2);
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .lk {
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    color: var(--text-2);
    cursor: pointer;
    text-decoration: underline;
    text-decoration-color: var(--line-strong);
    text-underline-offset: 3px;
    transition:
      color var(--fast) ease,
      text-decoration-color var(--fast) ease;
  }
  .lk:hover {
    color: var(--text);
    text-decoration-color: var(--accent);
  }
  .credit-row {
    margin-top: 20px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  .help {
    display: flex;
    gap: 22px;
    height: min(580px, calc(100vh - 140px));
    padding-bottom: 14px;
  }
  .hnav {
    width: 196px;
    flex: none;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px 0 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--text-4);
    transition:
      border-color var(--fast) ease,
      box-shadow var(--fast) ease;
  }
  .search:focus-within {
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--focus-soft);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 12.5px;
  }
  .search input::placeholder {
    color: var(--text-4);
  }
  .clear {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: 9px;
    background: var(--raise-hover);
    color: var(--text-3);
  }
  /* In a low window the list scrolls by itself (the search box and the page beside it stay put). */
  .list {
    position: relative;
    flex: 0 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
  }
  /* One highlight that slides to the page you're on (rows are 32px with 2px between). */
  .lit {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 32px;
    border-radius: var(--radius);
    background: var(--active);
    pointer-events: none;
    transition: transform var(--mid) var(--ease);
  }
  .list button {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 9px;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-2);
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition:
      background var(--fast) ease,
      color var(--fast) ease;
  }
  .list button :global(svg) {
    flex: none;
    color: var(--text-4);
    transition: color var(--fast) ease;
  }
  .list button:hover:not(.on) {
    background: var(--raise);
    color: var(--text);
  }
  .list button.on {
    color: var(--text);
    font-weight: 600;
  }
  .list button.on :global(svg) {
    color: var(--accent-strong);
  }
  .f1 {
    flex: none;
    margin: auto 0 0;
    font-size: 11.5px;
    color: var(--text-4);
  }
  .pane {
    position: relative;
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 2px 10px 24px 2px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 17px;
    font-weight: 600;
    color: var(--text);
  }
  .sub {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .about {
    margin-bottom: 18px;
  }
  .card {
    padding: 16px 18px;
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
  }
  .thead {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }
  .tours h2 {
    font-size: 14px;
  }
  .pages {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 14px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px;
    border: 1px solid var(--btn-line);
    border-radius: 14px;
    background: var(--btn);
    color: var(--text-2);
    font-size: 12px;
    transition:
      background var(--fast) ease,
      color var(--fast) ease,
      border-color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .chip :global(svg) {
    color: var(--accent-strong);
  }
  .chip:hover {
    background: var(--btn-hover);
    color: var(--text);
  }
  .chip:active {
    transform: scale(0.97);
  }
  .gtitle {
    margin: 24px 0 10px;
    font-size: 14px;
  }
  .guides {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .guide {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    background: transparent;
    text-align: left;
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .guide:hover {
    background: var(--raise);
    border-color: var(--line-strong);
  }
  .guide:active {
    transform: scale(0.99);
  }
  .guides.extra {
    margin-top: 10px;
  }
  .gname {
    display: flex;
    align-items: center;
    gap: 7px;
    font-weight: 600;
    color: var(--text);
  }
  .gname :global(svg) {
    color: var(--accent-strong);
  }
  .gabout {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .ver {
    margin: 26px 0 8px;
    padding-top: 18px;
    border-top: 1px solid var(--line);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-3);
  }
  .ver.first {
    margin-top: 12px;
    padding-top: 0;
    border-top: none;
  }
  .hits {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 12px;
  }
  .hit {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
    padding: 9px 12px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    text-align: left;
    transition: background var(--fast) ease;
  }
  .hit:hover,
  .hit:focus-visible {
    background: var(--raise);
  }
  .where {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .where .dim {
    color: var(--text-3);
    font-weight: 500;
  }
  .sep {
    margin: 0 6px;
    color: var(--text-4);
  }
  .snip {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
  .none {
    margin: 12px 0 0;
    color: var(--text-3);
  }
</style>
