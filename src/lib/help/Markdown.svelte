<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import type { Block, Inline } from "./markdown";

  let {
    blocks,
    onlink,
  }: {
    blocks: Block[];
    /** A link was clicked (another guide, a section, or a web page). */
    onlink: (href: string) => void;
  } = $props();

  function click(e: MouseEvent, href: string) {
    e.preventDefault();
    onlink(href);
  }
</script>

<!-- One line on purpose: a line break between the parts would show as a space. -->
{#snippet line(parts: Inline[])}{#each parts as p, i (i)}{#if p.kind === "bold"}<strong>{p.text}</strong>{:else if p.kind === "italic"}<em>{p.text}</em>{:else if p.kind === "code"}<code>{p.text}</code>{:else if p.kind === "link"}<a href={p.href} class:ext={/^https?:/.test(p.href)} onclick={(e) => click(e, p.href)}>{p.text}</a>{:else}{p.text}{/if}{/each}{/snippet}

<div class="md">
  {#each blocks as b, i (i)}
    {#if b.kind === "heading"}
      {#if b.level <= 2}<h3 data-anchor={b.id}>{b.text}</h3>{:else}<h4 data-anchor={b.id}>{b.text}</h4>{/if}
    {:else if b.kind === "para"}
      <p>{@render line(b.inline)}</p>
    {:else if b.kind === "list"}
      {#if b.ordered}
        <ol start={b.start}>
          {#each b.items as it, j (j)}<li>{@render line(it)}</li>{/each}
        </ol>
      {:else}
        <ul>
          {#each b.items as it, j (j)}<li>{@render line(it)}</li>{/each}
        </ul>
      {/if}
    {:else if b.kind === "code"}
      <pre><code>{b.text}</code></pre>
    {/if}
  {/each}
</div>

<style>
  .md {
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-2);
  }
  h3,
  h4 {
    color: var(--text);
    font-weight: 600;
    scroll-margin-top: 12px;
    border-radius: var(--radius);
    transition: background var(--slow) ease;
  }
  h3 {
    margin: 26px 0 6px;
    font-size: 14.5px;
  }
  h4 {
    margin: 18px 0 4px;
    font-size: 13px;
  }
  .md > :first-child {
    margin-top: 0;
  }
  /* The section a "?" link opened: a soft glow that fades. */
  :global(.md .flash) {
    animation: flash 1.6s var(--ease-out);
  }
  @keyframes flash {
    0%,
    30% {
      background: var(--accent-soft);
      box-shadow: 0 0 0 6px var(--accent-soft);
    }
    100% {
      background: transparent;
      box-shadow: 0 0 0 6px transparent;
    }
  }
  p {
    margin: 0 0 10px;
  }
  ul,
  ol {
    margin: 0 0 10px;
    padding-left: 22px;
  }
  li {
    margin: 0 0 4px;
    padding-left: 2px;
  }
  li::marker {
    color: var(--text-4);
  }
  strong {
    color: var(--text);
    font-weight: 600;
  }
  code {
    font-family: var(--mono);
    font-size: 12px;
    padding: 1px 5px;
    border-radius: var(--radius-sm);
    background: var(--raise);
    border: 1px solid var(--line);
  }
  pre {
    margin: 0 0 10px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--raise);
    border: 1px solid var(--line);
    overflow-x: auto;
  }
  pre code {
    padding: 0;
    border: none;
    background: none;
  }
  a {
    color: var(--focus-text);
    text-decoration: none;
    border-radius: var(--radius-sm);
  }
  a:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  a.ext::after {
    content: " ↗";
    font-size: 11px;
  }
</style>
