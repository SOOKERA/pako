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
  import { around, t } from "../i18n.svelte";

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
      ? t("Turn on Private mode")
      : step === "verify"
        ? t("New recovery code")
        : step === "code"
        ? t("Your recovery code")
        : step === "change"
          ? only === "recovered"
            ? t("Choose a new PIN or password")
            : t("Change PIN or password")
          : t("Private mode is on"),
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
        app.toast(t("New recovery code kept. The old one no longer works on this PC."));
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
      app.toast(kind === "pin" ? t("New PIN saved. The old one no longer works on this PC.") : t("New password saved. The old one no longer works on this PC."));
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
          {@const [goA, goB] = around(t("They go to a {private} list in Downloads, hidden until you open it."), "{private}")}
          {@const [resetA, resetB] = around(
            t("If you forget your PIN or password and don't have your recovery code, nobody can open the private list or its files, not even us. The only way back is {reset}, which deletes them."),
            "{reset}",
          )}
          <p class="lead">{t("Keep the downloads from your browser's private windows behind a PIN or a password.")}</p>
          <ul>
            <li>{goA}<b>{t("Private")}</b>{goB}</li>
            <li>{t("Their files are encrypted on this PC: they can't be opened from File Explorer.")}</li>
            <li>{t("Private mode locks itself when you step away, lock Windows or close Pako.")}</li>
          </ul>
          <p class="warn">{resetA}<b>{t("Reset Private mode")}</b>{resetB}</p>
        {:else if step === "secret"}
          <SecretPicker bind:kind bind:secret bind:valid />
        {:else if step === "verify"}
          <p class="lead">{t("Type your PIN or password to make a new recovery code. Private mode stays locked.")}</p>
          <label class="field">
            <span>{t("Your PIN or password")}</span>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="input" type="password" autocomplete="current-password" autofocus bind:value={current} onkeydown={(e) => e.key === "Enter" && current && !busy && verify()} />
          </label>
        {:else if step === "code"}
          <RecoveryCode bind:confirmed bind:last {given} />
        {:else if step === "change"}
          {#if only !== "recovered"}
            <label class="field">
              <span>{t("Your current PIN or password")}</span>
              <input class="input" type="password" autocomplete="current-password" bind:value={current} />
            </label>
          {:else}
            <p class="lead">{t("Private mode is open with your recovery code. Choose a new PIN or password for next time.")}</p>
          {/if}
          <SecretPicker bind:kind bind:secret bind:valid />
        {:else}
          <div class="hero"><span class="lockmark ok"><Icon name="check" size={22} stroke={2.2} /></span></div>
          {@const [onA, onRest] = around(t("Downloads from private windows now go to {private} in Downloads. Open it there, or with {keys}."), "{private}")}
          {@const [onB, onC] = around(onRest, "{keys}")}
          <p class="lead">{t("Private mode is on.")}</p>
          <p class="sub">{onA}<b>{t("Private")}</b>{onB}<span class="kbd">{t("Ctrl+Shift+P")}</span>{onC}</p>
        {/if}
        {#if error}<p class="err">{error}</p>{/if}
      </div>
    {/key}
  </div>
  {#snippet actions()}
    {#if step === "about"}
      <button class="btn ghost" onclick={onclose}>{t("Not now")}</button>
      <button class="btn primary" onclick={() => (step = "secret")}>{t("Continue")}</button>
    {:else if step === "secret"}
      <button class="btn ghost" disabled={busy} onclick={() => (step = "about")}>{t("Back")}</button>
      <button class="btn primary" disabled={!valid || busy} onclick={turnOn}>{busy ? t("Turning on…") : t("Turn on Private mode")}</button>
    {:else if step === "verify"}
      <button class="btn ghost" disabled={busy} onclick={onclose}>{t("Cancel")}</button>
      <button class="btn primary" disabled={!current || busy} onclick={verify}>{busy ? t("Checking…") : t("Continue")}</button>
    {:else if step === "code"}
      {#if only !== "code"}<button class="btn ghost" disabled={busy} onclick={() => (step = "done")}>{t("Skip")}</button>{:else}<button class="btn ghost" disabled={busy} onclick={giveUp}>{t("Cancel")}</button>{/if}
      <button class="btn primary" disabled={!confirmed || busy} onclick={keepCode}>{busy ? t("Saving…") : t("I kept it")}</button>
    {:else if step === "change"}
      {#if only !== "recovered"}<button class="btn ghost" disabled={busy} onclick={onclose}>{t("Cancel")}</button>{/if}
      <button class="btn primary" disabled={!valid || busy || (only !== "recovered" && !current)} onclick={change}>{busy ? t("Saving…") : t("Save")}</button>
    {:else}
      <button class="btn primary" onclick={done}>{t("Open Private mode")}</button>
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
