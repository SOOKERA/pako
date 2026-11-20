<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Turning Private mode on: what it is, a PIN or a password, then the recovery code (offered,
  // can be skipped). Also used later for "New recovery code" (`only = "code"`) and "Change PIN or
  // password" (`only = "change"`).
  import { untrack } from "svelte";
  import { api } from "../api";
  import { app } from "../state.svelte";
  import { priv } from "../private.svelte";
  import Modal from "../Modal.svelte";
  import Icon from "../Icon.svelte";
  import SecretPicker from "./SecretPicker.svelte";
  import RecoveryCode from "./RecoveryCode.svelte";

  let { only, onclose }: { only?: "code" | "change" | "recovered"; onclose: () => void } = $props();

  type Step = "about" | "secret" | "verify" | "code" | "done" | "change";
  // "New recovery code…" while locked (from Settings): your PIN or password first.
  let step = $state<Step>(untrack(() => (only === "code" ? (priv.status.unlocked ? "code" : "verify") : only === "change" || only === "recovered" ? "change" : "about")));
  /** The code made with your PIN or password while locked. */
  let given = $state<string | null>(null);
  let kind = $state<"pin" | "password">(priv.status.lock === "password" ? "password" : "pin");
  let secret = $state("");
  let valid = $state(false);
  let current = $state("");
  let busy = $state(false);
  let error = $state("");
  let confirmed = $state(false);
  let last = $state("");

  const title = $derived(
    step === "about" || step === "secret"
      ? "Turn on Private mode"
      : step === "verify"
        ? "New recovery code"
        : step === "code"
        ? "Your recovery code"
        : step === "change"
          ? only === "recovered"
            ? "Choose a new PIN or password"
            : "Change PIN or password"
          : "Private mode is on",
  );

  async function turnOn() {
    busy = true;
    error = "";
    try {
      await api.privateSetup(secret, kind);
      await priv.load();
      step = "code";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function verify() {
    busy = true;
    error = "";
    try {
      given = await api.privateNewCode(current);
      current = "";
      step = "code";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  /** Closing "New recovery code…" before keeping it: the code shown is forgotten. */
  function giveUp() {
    if (only === "code") api.privateDropCode().catch(() => {});
    onclose();
  }

  /** Turned on: Settings closes and the private list opens, so you see Private mode is open. */
  function done() {
    // The list is asked for first, in the same moment the window closes: Settings closes with
    // it, so Private mode is never open with nothing on screen saying so.
    if (!only) {
      priv.showList++;
      priv.opened();
    }
    onclose();
  }

  async function keepCode() {
    busy = true;
    error = "";
    try {
      await api.privateConfirmCode(last);
      await priv.load();
      if (only === "code") {
        app.toast("New recovery code kept. The old one no longer works on this PC.");
        onclose();
      } else step = "done";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function change() {
    busy = true;
    error = "";
    try {
      await api.privateChange(only === "recovered" ? null : current, secret, kind);
      await priv.load();
      app.toast(kind === "pin" ? "New PIN saved. The old one no longer works on this PC." : "New password saved. The old one no longer works on this PC.");
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal {title} width={460} onclose={() => !busy && (step === "done" ? done() : giveUp())} outsideCloses={false} closable={only !== "recovered"}>
  <div class="steps">
    {#key step}
      <div class="step">
        {#if step === "about"}
          <div class="hero"><span class="lockmark"><Icon name="lock" size={22} stroke={1.8} /></span></div>
          <p class="lead">Keep the downloads from your browser's private windows behind a PIN or a password.</p>
          <ul>
            <li>They go to a <b>Private</b> list in Downloads, hidden until you open it.</li>
            <li>Their files are encrypted on this PC: they can't be opened from File Explorer.</li>
            <li>Private mode locks itself when you step away, lock Windows or close Pako.</li>
          </ul>
          <p class="warn">
            If you forget your PIN or password and don't have your recovery code, nobody can open the private list or its files, not even us. The only way back is
            <b>Reset Private mode</b>, which deletes them.
          </p>
        {:else if step === "secret"}
          <SecretPicker bind:kind bind:secret bind:valid />
        {:else if step === "verify"}
          <p class="lead">Type your PIN or password to make a new recovery code. Private mode stays locked.</p>
          <label class="field">
            <span>Your PIN or password</span>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="input" type="password" autocomplete="current-password" autofocus bind:value={current} onkeydown={(e) => e.key === "Enter" && current && !busy && verify()} />
          </label>
        {:else if step === "code"}
          <RecoveryCode bind:confirmed bind:last {given} />
        {:else if step === "change"}
          {#if only !== "recovered"}
            <label class="field">
              <span>Your current PIN or password</span>
              <input class="input" type="password" autocomplete="current-password" bind:value={current} />
            </label>
          {:else}
            <p class="lead">Private mode is open with your recovery code. Choose a new PIN or password for next time.</p>
          {/if}
          <SecretPicker bind:kind bind:secret bind:valid />
        {:else}
          <div class="hero"><span class="lockmark ok"><Icon name="check" size={22} stroke={2.2} /></span></div>
          <p class="lead">Private mode is on.</p>
          <p class="sub">
            Downloads from private windows now go to <b>Private</b> in Downloads. Open it there, or with <span class="kbd">Ctrl+Shift+P</span>.
          </p>
        {/if}
        {#if error}<p class="err">{error}</p>{/if}
      </div>
    {/key}
  </div>
  {#snippet actions()}
    {#if step === "about"}
      <button class="btn ghost" onclick={onclose}>Not now</button>
      <button class="btn primary" onclick={() => (step = "secret")}>Continue</button>
    {:else if step === "secret"}
      <button class="btn ghost" disabled={busy} onclick={() => (step = "about")}>Back</button>
      <button class="btn primary" disabled={!valid || busy} onclick={turnOn}>{busy ? "Turning on…" : "Turn on Private mode"}</button>
    {:else if step === "verify"}
      <button class="btn ghost" disabled={busy} onclick={onclose}>Cancel</button>
      <button class="btn primary" disabled={!current || busy} onclick={verify}>{busy ? "Checking…" : "Continue"}</button>
    {:else if step === "code"}
      {#if only !== "code"}<button class="btn ghost" disabled={busy} onclick={() => (step = "done")}>Skip</button>{:else}<button class="btn ghost" disabled={busy} onclick={giveUp}>Cancel</button>{/if}
      <button class="btn primary" disabled={!confirmed || busy} onclick={keepCode}>{busy ? "Saving…" : "I kept it"}</button>
    {:else if step === "change"}
      {#if only !== "recovered"}<button class="btn ghost" disabled={busy} onclick={onclose}>Cancel</button>{/if}
      <button class="btn primary" disabled={!valid || busy || (only !== "recovered" && !current)} onclick={change}>{busy ? "Saving…" : "Save"}</button>
    {:else}
      <button class="btn primary" onclick={done}>Open Private mode</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .steps {
    display: grid;
  }
  .step {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: stepIn var(--slow) var(--ease);
  }
  @keyframes stepIn {
    from {
      opacity: 0;
      transform: translateX(10px);
    }
  }
  .hero {
    display: flex;
    justify-content: center;
    padding: 4px 0 2px;
  }
  .lockmark {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    background: var(--priv-tint);
    border: 1px solid var(--priv-line);
    color: var(--priv-text);
  }
  .lead {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    line-height: 1.5;
  }
  .sub {
    margin: 0;
    color: var(--text-2);
    line-height: 1.55;
  }
  ul {
    margin: 0;
    padding-left: 18px;
    color: var(--text-2);
    line-height: 1.6;
  }
  .warn {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--raise);
    border: 1px solid var(--line);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .kbd {
    padding: 1px 6px;
    border-radius: 5px;
    border: 1px solid var(--line-strong);
    font-size: 12px;
  }
  .err {
    margin: 0;
    font-size: 12.5px;
    color: var(--st-fail-text);
  }
  @media (prefers-reduced-motion: reduce) {
    .step {
      animation: none;
    }
  }
</style>
