<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Opening Private mode: the PIN pad (or the password field) slides in over the list. Dots fill
  // as you type, a short shake on a wrong PIN, a soft settle and the lock opening on the right
  // one. "Forgot it?" takes the recovery code instead.
  import { onMount } from "svelte";
  import { api, type PrivateRefused } from "../api";
  import { priv } from "../private.svelte";
  import { pop, reduced } from "../motion";
  import Icon from "../Icon.svelte";

  let { onrecovered }: { onrecovered: () => void } = $props();

  const pin = $derived(priv.status.lock !== "password");
  let value = $state("");
  let code = $state("");
  let mode = $state<"secret" | "code">("secret");
  let busy = $state(false);
  let shake = $state(false);
  let opened = $state(false);
  let message = $state("");
  let wait = $state(0);
  let input: HTMLInputElement | undefined = $state();

  onMount(() => {
    wait = priv.status.wait;
    input?.focus();
    const t = setInterval(() => {
      if (wait > 0) wait--;
    }, 1000);
    return () => clearInterval(t);
  });

  function waitText(s: number) {
    if (s >= 90) return `${Math.ceil(s / 60)} minutes`;
    return s === 1 ? "1 second" : `${s} seconds`;
  }

  async function submit() {
    if (busy || wait > 0) return;
    const typed = mode === "code" ? code : value;
    if (!typed) return;
    busy = true;
    message = "";
    try {
      if (mode === "code") await api.privateUnlockCode(typed);
      else await api.privateUnlock(typed);
      opened = true;
      // The lock opens, then the list comes in.
      setTimeout(
        async () => {
          await priv.opened();
          if (mode === "code") onrecovered();
        },
        reduced() ? 0 : 260,
      );
    } catch (e) {
      const r = e as PrivateRefused;
      message = r?.message ?? String(e);
      wait = r?.wait ?? 0;
      if (r?.wrong !== false || mode === "secret") {
        shake = true;
        setTimeout(() => (shake = false), 420);
      }
      value = "";
      busy = false;
      setTimeout(() => input?.focus(), 0);
    }
  }

  function typedPin(e: Event) {
    const el = e.currentTarget as HTMLInputElement;
    el.value = el.value.replace(/\D/g, "").slice(0, 4);
    value = el.value;
    if (value.length === 4) submit();
  }

  function key(d: string) {
    if (busy || wait > 0) return;
    if (d === "back") value = value.slice(0, -1);
    else if (value.length < 4) value += d;
    if (value.length === 4) submit();
    input?.focus();
  }

  function onkey(e: KeyboardEvent) {
    // Tab stays in the pad (it's a dialog over the list).
    if (e.key === "Tab") {
      const card = (e.currentTarget as HTMLElement).querySelector<HTMLElement>(".card");
      const items = [...(card?.querySelectorAll<HTMLElement>("button:not([disabled]), input:not([disabled]), a[href]") ?? [])].filter((x) => x.getClientRects().length > 0);
      if (items.length) {
        const at = items.indexOf(document.activeElement as HTMLElement);
        const to = at < 0 ? 0 : (at + (e.shiftKey ? -1 : 1) + items.length) % items.length;
        e.preventDefault();
        items[to].focus();
      }
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      priv.pad = false;
    }
  }
</script>

<div class="veil" role="presentation" onkeydown={onkey} out:pop={{ duration: 200, from: 0.98, y: 18 }}>
  <div class="card" class:shake class:opened role="dialog" aria-modal="true" aria-label="Open Private mode">
    <button class="close icon-btn" aria-label="Cancel" title="Cancel (Esc)" onclick={() => (priv.pad = false)}><Icon name="x" size={14} /></button>
    <div class="mark" class:open={opened}>
      <Icon name={opened ? "unlock" : "lock"} size={22} stroke={1.8} />
    </div>
    <h2>Private downloads</h2>
    {#if mode === "secret"}
      <p class="sub">{pin ? "Type your PIN" : "Type your password"}</p>
      {#if pin}
        <div class="dots" aria-hidden="true">
          {#each [0, 1, 2, 3] as i (i)}<span class="d" class:on={value.length > i}></span>{/each}
        </div>
        <input
          bind:this={input}
          class="hidden"
          type="password"
          inputmode="numeric"
          maxlength="4"
          aria-label="PIN"
          autocomplete="off"
          disabled={busy || wait > 0}
          value={value}
          oninput={typedPin}
          onkeydown={(e) => e.key === "Enter" && submit()}
        />
        <div class="keys">
          {#each ["1", "2", "3", "4", "5", "6", "7", "8", "9"] as d (d)}
            <button class="k" disabled={busy || wait > 0} onclick={() => key(d)}>{d}</button>
          {/each}
          <span></span>
          <button class="k" disabled={busy || wait > 0} onclick={() => key("0")}>0</button>
          <button class="k" aria-label="Delete a digit" disabled={busy || wait > 0 || !value} onclick={() => key("back")}><Icon name="x" size={14} /></button>
        </div>
      {:else}
        <form
          class="pw"
          onsubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <input bind:this={input} class="input" type="password" aria-label="Password" autocomplete="current-password" disabled={busy || wait > 0} bind:value />
          <button class="btn primary" disabled={busy || wait > 0 || !value}>{busy ? "Opening…" : "Open"}</button>
        </form>
      {/if}
    {:else}
      <p class="sub">Type your recovery code</p>
      <form
        class="pw"
        onsubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <input bind:this={input} class="input mono codein" aria-label="Recovery code" placeholder="XXXX-XXXX-…" autocomplete="off" spellcheck="false" disabled={busy || wait > 0} bind:value={code} />
        <button class="btn primary" disabled={busy || wait > 0 || !code.trim()}>{busy ? "Opening…" : "Open"}</button>
      </form>
    {/if}
    <p class="msg" class:bad={!!message} aria-live="polite">
      {#if wait > 0}Wait {waitText(wait)} before trying again.{:else if message}{message}{:else}&nbsp;{/if}
    </p>
    {#if priv.status.hasRecovery}
      <button
        class="link"
        onclick={() => {
          mode = mode === "secret" ? "code" : "secret";
          message = "";
          setTimeout(() => input?.focus(), 0);
        }}>{mode === "secret" ? `Forgot your ${pin ? "PIN" : "password"}?` : `Use your ${pin ? "PIN" : "password"}`}</button
      >
    {/if}
  </div>
</div>

<style>
  .veil {
    position: absolute;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--bg) 72%, transparent);
    backdrop-filter: blur(8px);
    animation: veilIn var(--mid) var(--ease);
  }
  @keyframes veilIn {
    from {
      opacity: 0;
    }
  }
  .card {
    position: relative;
    width: 300px;
    padding: 26px 24px 18px;
    border-radius: 16px;
    background: var(--panel);
    border: 1px solid var(--priv-line);
    box-shadow: var(--shadow-pop, 0 18px 50px rgba(0, 0, 0, 0.35));
    display: flex;
    flex-direction: column;
    align-items: center;
    animation: cardIn var(--slow) var(--ease);
  }
  @keyframes cardIn {
    from {
      opacity: 0;
      transform: translateY(18px) scale(0.98);
    }
  }
  .card.shake {
    animation: shake 0.42s var(--ease);
  }
  @keyframes shake {
    15%,
    55% {
      transform: translateX(-8px);
    }
    35%,
    75% {
      transform: translateX(7px);
    }
  }
  .card.opened {
    animation: settle var(--slow) var(--ease) forwards;
  }
  @keyframes settle {
    40% {
      transform: scale(1.015);
    }
    to {
      transform: scale(0.99);
      opacity: 0;
    }
  }
  .close {
    position: absolute;
    top: 10px;
    right: 10px;
  }
  .mark {
    width: 46px;
    height: 46px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    color: var(--priv-text);
    transition:
      background var(--mid) ease,
      transform var(--mid) var(--ease);
  }
  .mark.open {
    background: var(--priv);
    color: #fff;
    transform: rotate(-6deg) scale(1.05);
  }
  h2 {
    margin: 12px 0 2px;
    font-size: 15px;
    font-weight: 600;
  }
  .sub {
    margin: 0 0 14px;
    color: var(--text-2);
    font-size: 13px;
  }
  .dots {
    display: flex;
    gap: 14px;
    margin-bottom: 16px;
  }
  .d {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1.5px solid var(--priv-line);
    transition:
      background var(--fast) ease,
      border-color var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .d.on {
    background: var(--priv);
    border-color: var(--priv);
    transform: scale(1.1);
  }
  .hidden {
    position: absolute;
    opacity: 0;
    pointer-events: none;
    width: 1px;
    height: 1px;
  }
  .keys {
    display: grid;
    grid-template-columns: repeat(3, 56px);
    gap: 8px;
  }
  .k {
    height: 44px;
    border-radius: 12px;
    border: 1px solid var(--line);
    background: var(--raise);
    color: var(--text);
    font: inherit;
    font-size: 16px;
    font-weight: 500;
    display: grid;
    place-items: center;
    transition:
      background var(--fast) ease,
      transform var(--fast) var(--ease);
  }
  .k:hover:not(:disabled) {
    background: var(--raise-hover, var(--select));
  }
  .k:active:not(:disabled) {
    transform: scale(0.95);
  }
  .k:disabled {
    opacity: 0.45;
  }
  .pw {
    display: flex;
    gap: 8px;
    width: 100%;
  }
  .pw .input {
    flex: 1;
    min-width: 0;
  }
  .codein {
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .msg {
    margin: 12px 0 0;
    min-height: 18px;
    font-size: 12.5px;
    text-align: center;
    color: var(--text-3);
  }
  .msg.bad {
    color: var(--st-fail-text);
  }
  .link {
    margin-top: 6px;
    border: none;
    background: none;
    color: var(--priv-text);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  @media (prefers-reduced-motion: reduce) {
    .veil,
    .card,
    .card.shake,
    .card.opened {
      animation: none;
    }
    .mark,
    .d {
      transition: none;
    }
  }
</style>
