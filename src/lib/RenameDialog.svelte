<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { app } from "./state.svelte";
  import { api, type Download } from "./api";
  import Modal from "./Modal.svelte";

  let { d, onclose }: { d: Download; onclose: () => void } = $props();
  let value = $state(untrack(() => d.name));
  let input: HTMLInputElement | undefined = $state();

  onMount(() =>
    setTimeout(() => {
      if (!input) return;
      input.focus();
      // Select the name without its extension, like Windows Explorer.
      const dot = value.lastIndexOf(".");
      input.setSelectionRange(0, dot > 0 ? dot : value.length);
    }, 60),
  );

  function onEnter(e: KeyboardEvent) {
    if (e.key !== "Enter") return;
    e.preventDefault();
    e.stopPropagation(); // the window would also open the focused file
    save();
  }

  async function save() {
    const name = value.trim();
    if (!name || name === d.name) return onclose();
    try {
      const final = await api.rename(d.id, name);
      if (final !== name) {
        // The app may have only cleaned characters Windows doesn't allow (like : or ?).
        // Say a file exists only when it really added a number: "name (1).zip".
        const cleaned = name
          .replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_")
          .trim()
          .replace(/[. ]+$/, "");
        const numbered = final !== cleaned && /\s\(\d+\)(\.[^.]*)?$/.test(final);
        app.toast(numbered ? `A file with that name exists: renamed to "${final}"` : `Saved as "${final}"`);
      }
      await app.refresh();
      onclose();
    } catch (e) {
      app.toast(String(e), "error");
    }
  }
</script>

<Modal title="Rename" width={460} {onclose}>
  <input bind:this={input} bind:value class="input" spellcheck="false" aria-label="New name" onkeydown={onEnter} />
  {#snippet actions()}
    <button class="btn ghost" onclick={onclose}>Cancel</button>
    <button class="btn primary" disabled={!value.trim()} onclick={save}>Rename</button>
  {/snippet}
</Modal>
