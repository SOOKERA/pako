<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // "ZIP in parts": the size of each part (500 MB, 1 GB, 2 GB or Custom, typed in MB). Used by
  // the New upload basket and Settings › Upload. `mb` only ever gets a size Pako takes; `valid`
  // says whether what's typed is one.
  import { tick, untrack } from "svelte";
  import SegPick from "./SegPick.svelte";
  import { rowSlide } from "./motion";
  import { PART_CHOICES, PART_DEFAULT_MB, PART_RANGE_NOTE, partLabel, partOk } from "./uploads";
  import { t } from "./i18n.svelte";

  let {
    mb = $bindable(),
    custom = $bindable(),
    valid = $bindable(true),
    label,
  }: {
    mb: number;
    custom: boolean;
    valid?: boolean;
    /** For screen readers. */
    label?: string;
  } = $props();

  const isPreset = (n: number) => PART_CHOICES.some((c) => c.value === n);
  /** A size that isn't one of the choices shows as Custom (older or typed-in ones). */
  const picked = $derived(custom || !isPreset(mb) ? "custom" : String(mb));
  /** What's typed in the Custom field (opened on Custom: the size it has). */
  let text = $state(untrack(() => (picked === "custom" && partOk(mb) ? String(mb) : "")));
  let field = $state<HTMLInputElement | null>(null);
  const typed = $derived(/^\d+$/.test(text.trim()) ? Number(text.trim()) : null);
  $effect(() => {
    const ok = picked !== "custom" || partOk(typed);
    valid = ok;
    if (picked === "custom" && partOk(typed) && typed !== mb) mb = typed;
  });

  const options = $derived([...PART_CHOICES.map((c) => ({ value: String(c.value), label: partLabel(c.value) })), { value: "custom", label: t("Custom") }]);

  async function pick(v: string) {
    if (v === "custom") {
      custom = true;
      if (!text) text = String(partOk(mb) ? mb : PART_DEFAULT_MB);
      await tick();
      field?.select();
      return;
    }
    custom = false;
    mb = Number(v);
  }
</script>

<div class="parts">
  <div class="row">
    <SegPick label={label ?? t("Size of each part")} {options} value={picked} onpick={pick} />
    {#if picked === "custom"}
      <label class="custom" class:bad={!valid}>
        <input
          class="input num"
          bind:this={field}
          bind:value={text}
          inputmode="numeric"
          maxlength="5"
          spellcheck="false"
          aria-label={t("Part size in MB")}
          aria-invalid={!valid}
        />
        <span class="unit">{t("MB")}</span>
      </label>
    {/if}
  </div>
  {#if !valid}
    <span class="range" role="alert" transition:rowSlide={{ duration: 180 }}>{t(PART_RANGE_NOTE)}</span>
  {/if}
</div>

<style>
  .parts {
    display: grid;
    gap: 6px;
    justify-items: start;
    min-width: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .custom {
    position: relative;
    display: inline-flex;
    align-items: center;
  }
  .custom .input {
    width: 92px;
    /* As tall as the choices next to it. */
    height: 32px;
    padding-right: 34px;
    font-variant-numeric: tabular-nums;
  }
  .unit {
    position: absolute;
    right: 10px;
    font-size: 12px;
    color: var(--text-3);
    pointer-events: none;
  }
  .bad .input,
  .bad .input:focus {
    border-color: var(--st-fail-text);
  }
  .bad .input:focus {
    box-shadow: 0 0 0 3px var(--tint-fail);
  }
  .range {
    font-size: 12px;
    color: var(--st-fail-text);
  }
</style>
