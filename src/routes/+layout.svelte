<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { theme } from "$lib/theme.svelte";
  let { children } = $props();
  // Light or dark, in every window.
  onMount(() => theme.start());
</script>

{@render children()}
