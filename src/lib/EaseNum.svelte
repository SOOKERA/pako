<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // A number that eases to its new value (Dashboard totals when the period changes), shown through `format`.
  import { Tween, prefersReducedMotion } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  let { value, format }: { value: number; format: (n: number) => string } = $props();

  const shown = Tween.of(() => value, { duration: () => (prefersReducedMotion.current ? 0 : 320), easing: cubicOut });
</script>

{format(shown.current)}
