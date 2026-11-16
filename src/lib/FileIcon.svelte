<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Flat file-type icon: a page with a folded corner in the colour of its kind and a white
  // mark (no brand logos). Design: the "File icons" board on the UI canvas.
  import { fileKind, type FileKind } from "./format";

  let {
    name = "",
    kind,
    size = 18,
    faded = false,
  }: {
    name?: string;
    /** Overrides what the name says (e.g. "folder"). */
    kind?: FileKind;
    size?: number;
    /** Paused: the icon fades a little. */
    faded?: boolean;
  } = $props();

  const k = $derived(kind ?? fileKind(name));
  const COLORS: Record<string, string> = {
    archive: "#E0A33B",
    pdf: "#E5484D",
    doc: "#3B82F6",
    sheet: "#22A06B",
    slides: "#F97316",
    video: "#8B5CF6",
    audio: "#EC4899",
    image: "#14B8A6",
    disc: "#64748B",
    app: "#475569",
    code: "#334155",
    "3d": "#0EA5E9",
    file: "#6B7280",
  };
  const PAGE = "M5.5 2h9.2L20 7.3V20.5A1.5 1.5 0 0 1 18.5 22h-13A1.5 1.5 0 0 1 4 20.5v-17A1.5 1.5 0 0 1 5.5 2z";
  const FOLD = "M14.7 2v4a1.3 1.3 0 0 0 1.3 1.3h4z";
</script>

<svg class="fi" class:faded width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  {#if k === "folder"}
    <path d="M3 6.5A1.5 1.5 0 0 1 4.5 5h4.6l2 2h8.4A1.5 1.5 0 0 1 21 8.5v10a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z" fill="#C98A1E" />
    <path d="M3 9.5A1.5 1.5 0 0 1 4.5 8h15A1.5 1.5 0 0 1 21 9.5v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z" fill="#F2B648" />
  {:else if k === "text"}
    <!-- a notepad with rings on top -->
    <rect x="4.5" y="3.5" width="15" height="18.5" rx="1.6" fill="#ECE8DF" />
    <rect x="4.5" y="3.5" width="15" height="4" rx="1.6" fill="#F2B648" /><rect x="4.5" y="5.5" width="15" height="2" fill="#F2B648" />
    <path d="M8 2v4M12 2v4M16 2v4" stroke="#5f6672" stroke-width="1.4" stroke-linecap="round" />
    <path d="M7.5 11h9M7.5 14h9M7.5 17h6" stroke="#9AA3AD" stroke-width="1.3" stroke-linecap="round" />
  {:else}
    <path d={PAGE} fill={COLORS[k] ?? COLORS.file} />
    <path d={FOLD} fill="#000" fill-opacity={k === "code" ? 0.25 : 0.22} />
    {#if k === "archive"}
      <path d="M11 3h2v1.6h-2zM11 6.2h2v1.6h-2zM11 9.4h2V11h-2z" fill="#fff" fill-opacity="0.9" />
      <rect x="9.6" y="12.2" width="4.8" height="5.6" rx="1.2" fill="#fff" /><rect x="11" y="14.6" width="2" height="1.6" rx="0.5" fill="#E0A33B" />
    {:else if k === "pdf"}
      <text x="12" y="17.2" text-anchor="middle" font-family="Inter Variable, Inter, sans-serif" font-weight="800" font-size="5.6" fill="#fff">PDF</text>
    {:else if k === "doc"}
      <path d="M7.5 11h9M7.5 14h9M7.5 17h6" stroke="#fff" stroke-width="1.6" stroke-linecap="round" />
    {:else if k === "sheet"}
      <rect x="7" y="10" width="10" height="8.5" rx="1" fill="none" stroke="#fff" stroke-width="1.4" /><path d="M7 13h10M7 15.8h10M11 10v8.5" stroke="#fff" stroke-width="1.2" />
    {:else if k === "slides"}
      <rect x="7" y="10" width="10" height="7" rx="1" fill="#fff" /><path d="M9 15.5v-2M11.5 15.5v-3.5M14 15.5v-2.5" stroke="#F97316" stroke-width="1.4" stroke-linecap="round" />
    {:else if k === "video"}
      <path d="M10 10.3v6.4a.6.6 0 0 0 .9.5l5-3.2a.6.6 0 0 0 0-1l-5-3.2a.6.6 0 0 0-.9.5z" fill="#fff" />
    {:else if k === "audio"}
      <path d="M11 17.2V10.5l5-1.3v6.5" fill="none" stroke="#fff" stroke-width="1.5" stroke-linejoin="round" /><circle cx="9.7" cy="17.2" r="1.7" fill="#fff" /><circle cx="14.7" cy="15.7" r="1.7" fill="#fff" />
    {:else if k === "image"}
      <circle cx="9.5" cy="11" r="1.6" fill="#fff" /><path d="M6.5 18.5l3.6-4 2.3 2.4 2.1-2.4 3 4z" fill="#fff" />
    {:else if k === "disc"}
      <circle cx="12" cy="14.2" r="4.6" fill="#fff" /><circle cx="12" cy="14.2" r="1.4" fill="#64748B" />
    {:else if k === "app"}
      <rect x="7" y="10" width="10" height="8" rx="1.2" fill="none" stroke="#fff" stroke-width="1.4" /><path d="M7 12.5h10" stroke="#fff" stroke-width="1.2" />
      <path d="M12 13.8v3M10.6 15.6l1.4 1.4 1.4-1.4" stroke="#fff" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none" />
    {:else if k === "code"}
      <path d="M10 11.5l-2.5 2.5 2.5 2.5M14 11.5l2.5 2.5-2.5 2.5" fill="none" stroke="#7DD3FC" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
    {:else if k === "3d"}
      <path d="M12 9.5l4 2.2v4.6l-4 2.2-4-2.2v-4.6zM12 14l4-2.3M12 14v4.5M12 14l-4-2.3" fill="none" stroke="#fff" stroke-width="1.2" stroke-linejoin="round" />
    {/if}
  {/if}
</svg>

<style>
  .fi {
    flex: none;
    display: block;
    transition: opacity var(--mid) ease;
  }
  .fi.faded {
    opacity: 0.45;
  }
</style>
