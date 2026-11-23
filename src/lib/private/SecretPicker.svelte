<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Picking a PIN or a password for Private mode (turning it on, changing it, after the
  // recovery code). Typed twice; says plainly what each one protects.
  import { untrack } from "svelte";
  import { t } from "../i18n.svelte";

  let {
    kind = $bindable<"pin" | "password">("pin"),
    secret = $bindable(""),
    valid = $bindable(false),
  }: { kind?: "pin" | "password"; secret?: string; valid?: boolean } = $props();

  let first = $state("");
  let again = $state("");
  let firstEl: HTMLInputElement | undefined = $state();

  const ok = $derived(kind === "pin" ? /^\d{4}$/.test(first) : [...first].length >= 10);
  /** A rough hint only: long, and not just letters or just digits. */
  const strong = $derived([...first].length >= 14 && /[^a-z]/i.test(first) && /[a-z]/i.test(first));
  const same = $derived(first === again);
  $effect(() => {
    valid = ok && same;
    secret = ok && same ? first : "";
  });
  // Switching between PIN and password starts the fields over.
  $effect(() => {
    kind;
    untrack(() => {
      first = "";
      again = "";
      firstEl?.focus();
    });
  });

  function digitsOnly(e: Event) {
    if (kind !== "pin") return;
    const el = e.currentTarget as HTMLInputElement;
    el.value = el.value.replace(/\D/g, "").slice(0, 4);
  }
</script>

<div class="picker">
  <div class="seg" role="radiogroup" aria-label={t("PIN or password")}>
    <button role="radio" aria-checked={kind === "pin"} class:on={kind === "pin"} onclick={() => (kind = "pin")}>{t("4-digit PIN")}</button>
    <button role="radio" aria-checked={kind === "password"} class:on={kind === "password"} onclick={() => (kind = "password")}>{t("Password")}</button>
    <span class="thumb" class:right={kind === "password"} aria-hidden="true"></span>
  </div>

  <label class="field">
    <span>{kind === "pin" ? t("Your new PIN") : t("Your new password")}</span>
    <input
      bind:this={firstEl}
      class="input"
      type="password"
      inputmode={kind === "pin" ? "numeric" : "text"}
      maxlength={kind === "pin" ? 4 : 200}
      autocomplete="new-password"
      bind:value={first}
      oninput={digitsOnly}
    />
  </label>
  <label class="field">
    <span>{t("Type it again")}</span>
    <input
      class="input"
      type="password"
      inputmode={kind === "pin" ? "numeric" : "text"}
      maxlength={kind === "pin" ? 4 : 200}
      autocomplete="new-password"
      bind:value={again}
      oninput={digitsOnly}
    />
  </label>
  <p class="state" class:bad={again.length > 0 && !same}>
    {#if first && !ok}
      {kind === "pin" ? t("The PIN is 4 digits.") : t("At least 10 characters.")}
    {:else if again && !same}
      {t("They don't match yet.")}
    {:else if kind === "password" && ok && !strong}
      {t("Longer is safer: a few words in a row, with a digit or a sign, is hard to guess.")}
    {:else}
      &nbsp;
    {/if}
  </p>

  {#if kind === "pin"}
    <p class="note">
      {t(
        "A PIN slows down someone at this PC: Pako waits longer after each wrong try. Someone who gets Pako's files and can use your Windows account could try all 10,000 PINs in minutes. A password is the real protection.",
      )}
    </p>
  {:else}
    <p class="note">{t("At least 10 characters. A long one you can remember is best: a few words in a row, with a digit or a sign, works well.")}</p>
  {/if}
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 3px;
    border-radius: calc(var(--radius) + 2px);
    background: var(--raise);
    border: 1px solid var(--line);
  }
  .seg button {
    position: relative;
    z-index: 1;
    height: 30px;
    border: none;
    background: transparent;
    color: var(--text-2);
    font: inherit;
    font-weight: 500;
    border-radius: var(--radius);
    transition: color var(--fast) ease;
  }
  .seg button.on {
    color: var(--text);
  }
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc(50% - 3px);
    border-radius: var(--radius);
    background: var(--bg);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.18);
    transition: transform var(--mid) var(--ease);
  }
  .thumb.right {
    transform: translateX(100%);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .state {
    margin: -2px 0 0;
    min-height: 16px;
    font-size: 12px;
    color: var(--text-3);
  }
  .state.bad {
    color: var(--st-fail-text);
  }
  .note {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  @media (prefers-reduced-motion: reduce) {
    .thumb {
      transition: none;
    }
  }
</style>
