<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Pako logo: the box (mark) and the "Pak" + taped-o wordmark.
  // Drawn without fonts so it looks the same everywhere. Geometry comes from the final logo board.
  // The box uses the accent colour; the word uses the text colour, the o the accent.
  let {
    size = 20,
    word = true,
    digits = false,
  }: {
    /** Height of the box in px. The word is sized to match. */
    size?: number;
    /** Show "Pako" next to the box. */
    word?: boolean;
    /** Show 0101 on the box (only readable from about 40 px). */
    digits?: boolean;
  } = $props();

  // One id per instance, so several logos on a page don't share clip paths or masks.
  const uid = "pk" + Math.random().toString(36).slice(2, 8);
  const wordH = $derived(Math.round(size * 0.66));
</script>

<span class="logo" style="gap: {Math.round(size * 0.45)}px">
  <svg width={size} height={size} viewBox="4 4 56 56" aria-hidden="true">
    <defs>
      <clipPath id="{uid}s">
        <path d="M30.257 6.98 Q32 6 33.743 6.98 L54.257 18.52 Q56 19.5 56 21.5 L56 42.5 Q56 44.5 54.257 45.48 L33.743 57.02 Q32 58 30.257 57.02 L9.743 45.48 Q8 44.5 8 42.5 L8 21.5 Q8 19.5 9.743 18.52 Z" />
      </clipPath>
      <mask id="{uid}m" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
        <rect width="64" height="64" fill="#fff" />
        <path d="M14.35 13 L40.95 27.97 V37.97 L47.05 34.53 V24.53 L20.45 9.56" fill="none" stroke="#000" stroke-width="2.2" stroke-linejoin="round" />
        <g transform="matrix(1 0.5625 0 1.0417 8 19.5)">
          <path d="M10 4.5 H14 V12 H18 L12 19.5 L6 12 H10 Z" fill="#000" stroke="#000" stroke-width="1.2" stroke-linejoin="round" />
        </g>
        {#if digits}
          <g transform="matrix(1 -0.5625 0 1.0417 32 33)" fill="none" stroke="#000" stroke-width="1.25" stroke-linejoin="round">
            <rect x="5.275" y="15.225" width="2.75" height="3.75" rx="1.375" ry="1.6" />
            <path d="M9.45 16.35 L11.125 14.85 V19.6" />
            <rect x="12.875" y="15.225" width="2.75" height="3.75" rx="1.375" ry="1.6" />
            <path d="M17.05 16.35 L18.725 14.85 V19.6" />
          </g>
        {/if}
      </mask>
    </defs>
    <g clip-path="url(#{uid}s)">
      <g mask="url(#{uid}m)">
        <path d="M32 6 L56 19.5 V44.5 L32 58 L8 44.5 V19.5 Z" fill="var(--accent)" />
        <path d="M8 19.5 L32 33 L56 19.5 V44.5 L32 58 L8 44.5 Z" fill="#000" fill-opacity="0.14" />
        <path d="M56 19.5 L32 33 V58 L56 44.5 Z" fill="#000" fill-opacity="0.1628" />
      </g>
    </g>
  </svg>
  {#if word}
    <svg class="word" height={wordH} viewBox="0 10 222 70" aria-label="Pako" role="img">
      <defs>
        <clipPath id="{uid}x"><rect x="0" y="30" width="222" height="50" /></clipPath>
      </defs>
      <g fill="none" stroke="currentColor" stroke-width="13">
        <!-- P -->
        <path d="M6.5 10 V80" />
        <path d="M6.5 16.5 H26 A16.5 16.5 0 0 1 26 49.5 H6.5" />
        <!-- a -->
        <circle cx="83" cy="55" r="18.5" />
        <path d="M101.5 30 V80" />
        <!-- k -->
        <path d="M122.5 10 V80" />
        <g clip-path="url(#{uid}x)">
          <path d="M122.5 70.8 L161 25" />
          <path d="M138 52 L164 86" />
        </g>
      </g>
      <!-- the o: the box seen from the front, with its tape -->
      <rect x="177.5" y="35.5" width="39" height="39" rx="3.5" fill="none" stroke="var(--accent)" stroke-width="11" />
      <rect x="191.5" y="39" width="11" height="13.5" rx="1" fill="var(--accent)" />
    </svg>
  {/if}
</span>

<style>
  .logo {
    display: inline-flex;
    align-items: center;
    flex: none;
    /* Let clicks reach the parent: in the title bar that is the window's drag area. */
    pointer-events: none;
  }
  .word {
    display: block;
    width: auto;
    overflow: visible;
  }
</style>
