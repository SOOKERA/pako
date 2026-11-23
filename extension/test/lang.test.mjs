// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The extension speaks Pako's language (English or French), never the browser's: it asks Pako's
// helper ("language", and `lang` in every answer), keeps it for when Pako can't be reached, and
// speaks English with an older Pako. Its French (lang.js) follows Pako's rules.
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";
import { ask, loadEdition, settle } from "./fake-browser.mjs";
import { toLight } from "../light.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const source = (f) => readFileSync(join(here, "..", f), "utf8");

/** lang.js as an edition ships it, run on its own. */
function words(light = false) {
  const context = {};
  context.globalThis = context;
  vm.runInContext(light ? toLight("lang.js", source("lang.js")) : source("lang.js"), vm.createContext(context));
  return context.__pakoWords;
}
const W = words();
const placeholders = (s) => [...new Set(s.match(/\{\w+\}/g) ?? [])].sort();

test("every French entry follows Pako's rules", () => {
  const entries = Object.entries(W.FR);
  assert.ok(entries.length > 100);
  for (const [key, fr] of entries) {
    assert.ok(typeof fr === "string" && fr.trim(), `"${key}": empty French`);
    assert.ok(!/\bapps?\b/i.test(fr), `"${key}": « appli », never « app » (${fr})`);
    assert.ok(!fr.includes("'"), `"${key}": the French apostrophe is ’ (${fr})`);
    assert.ok(!fr.includes("..."), `"${key}": … not ... (${fr})`);
    assert.ok(!/[  ]/.test(fr), `"${key}": plain spaces in the dictionary (tr() adds the French ones) (${fr})`);
    assert.ok(!/ {2}/.test(fr), `"${key}": a double space (${fr})`);
    assert.ok(!/"/.test(fr), `"${key}": French quotes are « » (${fr})`);
    assert.deepEqual(placeholders(fr), placeholders(key), `"${key}": placeholders differ`);
    assert.notEqual(fr, key, `"${key}": not translated`);
  }
});

test("tr: French typography added, placeholders filled, English kept word for word", () => {
  assert.equal(W.tr("fr", "Details: {text}", { text: "x: y" }), "Détails : x: y", "never inside what is filled in");
  assert.equal(W.tr("fr", "No Pako on this PC yet?"), "Pas encore de Pako sur ce PC ?");
  assert.equal(W.tr("fr", 'To use Pako there, turn on "{name}" on this extension\'s page.', { name: "A" }), "Pour y utiliser Pako, activez « A » sur la page de cette extension.");
  assert.equal(W.tr("en", "Details: {text}", { text: "x" }), "Details: x");
  assert.equal(W.tr("de", "Open Pako"), "Open Pako", "anything but French is English");
  assert.equal(W.tr("fr", "Something nobody wrote"), "Something nobody wrote");
  assert.equal(W.maybe("fr", "Pako didn't answer."), "Pako n’a pas répondu.");
  assert.equal(W.maybe("fr", "Pako n’a pas répondu."), "Pako n’a pas répondu.", "Pako's own French stays");
  assert.equal(W.num("fr", 1.5), "1,5");
  assert.equal(W.frType("déjà {pct}%"), "déjà {pct} %", "a non-breaking space before %, as in the window");
  assert.equal(W.num("en", 1.5), "1.5");
});

/** The literal keys the scripts translate (tr("…"), kit.tr("…"), and the constants they pass). */
function keysIn(file) {
  const src = source(file);
  const keys = [];
  // (Not `WORDS.tr(lang, …)`: lang.js's own, called with the language first.)
  for (const m of src.matchAll(/(?<!WORDS\.)\btr\(\s*("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')/g)) keys.push(vm.runInNewContext(m[1]));
  for (const m of src.matchAll(/\btr\(\s*([A-Z_]+)\s*[,)]/g)) {
    const def = new RegExp(`const ${m[1]} = ("(?:[^"\\\\]|\\\\.)*")`).exec(src);
    assert.ok(def, `${file}: ${m[1]} is a string there`);
    keys.push(JSON.parse(def[1]));
  }
  return keys;
}

test("every word the extension shows has its French", () => {
  const files = ["background.js", "popup.js", "pill.js", "player.js", "caught.js"];
  for (const f of files) {
    const keys = keysIn(f);
    assert.ok(keys.length > 0, f);
    for (const k of keys) assert.ok(k in W.FR, `${f}: "${k}" has no French in lang.js`);
  }
  // The kinds of caught video, said through tr(CAUGHT_KINDS[…]) / tr(KINDS[…]).
  for (const k of ["HLS stream", "DASH stream", "Video file"]) assert.ok(k in W.FR, k);
  // The toolbar window's own page: every text and label (but the names).
  const html = source("popup.html").replace(/<!--[\s\S]*?-->/g, "");
  const texts = [...html.matchAll(/>([^<>]+)</g)].map((m) => m[1].trim()).filter(Boolean);
  const labels = [...html.matchAll(/aria-label="([^"]+)"/g)].map((m) => m[1]);
  for (const t of [...texts, ...labels]) {
    if (["Pako", "Light"].includes(t)) continue;
    assert.ok(t in W.FR, `popup.html: "${t}" has no French in lang.js`);
  }
  for (const t of ["Sent to Pako", "This download went to Pako."]) assert.ok(t in W.FR, t);
});

test("Pako Light's words have nothing of the full extension's extras", () => {
  const light = words(true);
  assert.ok(!("Download video with Pako" in light.FR));
  assert.ok(!("Record this video" in light.FR));
  assert.ok("Download link with Pako" in light.FR);
  for (const [k, v] of Object.entries(light.FR)) assert.ok(!/vidéo|qualité|lecteur|enregistr/i.test(v) || k.includes("save it"), `Light: "${k}" (${v})`);
});

const titles = (ext) => Object.fromEntries([...ext.menus].map(([id, m]) => [id, m.title]));

test("Pako says French: the right-click items, the answers and the '!' are in French", async () => {
  const ext = loadEdition({ lang: "fr", tabs: { 1: "https://news.example/" } });
  await settle();
  assert.equal(ext.langAsks.length, 1, "asked once when the background page started");
  assert.equal(ext.langAsks[0].private, true, "so that an older Pako notes nothing about it");
  assert.equal(ext.storage.pakoLang, "fr");
  await ext.listeners.installed();
  await settle();
  assert.equal(titles(ext)["pako-link"], "Télécharger le lien avec Pako");
  assert.equal(titles(ext)["pako-media"], "Télécharger l’image avec Pako");
  assert.equal(titles(ext)["pako-page"], "Envoyer cette page à Pako");
  await ext.listeners.click({ menuItemId: "pako-link", linkUrl: "ftp://x.example/a" }, { id: 1, url: "https://news.example/" });
  await settle();
  assert.equal(ext.storage.lastError, "Pako ne prend que les liens http et https.");
  const r = await ask(ext, { type: "nonsense" }, { tab: { id: 1 } });
  assert.equal(r.error, "L’extension Pako n’a pas compris cette demande.");
  // The toolbar window hears the language with its ping.
  const ping = await ask(ext, { type: "ping" });
  assert.equal(ping.lang, "fr");
});

test("Pako switches back to English: its next answer says so, and the items follow", async () => {
  let said = "fr";
  const ext = loadEdition({ lang: "fr", reply: () => ({ ok: true, lang: said }) });
  await ext.listeners.installed();
  await settle();
  assert.equal(titles(ext)["pako-link"], "Télécharger le lien avec Pako");
  said = "en";
  await ask(ext, { type: "ping" });
  await settle();
  assert.equal(ext.storage.pakoLang, "en");
  assert.equal(titles(ext)["pako-link"], "Download link with Pako");
});

test("An older Pako doesn't know the request: English, and nothing said to you", async () => {
  const ext = loadEdition({ lang: null, stored: { pakoLang: "fr" } });
  await settle();
  assert.equal(ext.langAsks.length, 1);
  assert.equal(ext.storage.pakoLang, "en", "an older Pako speaks English");
  assert.equal(ext.storage.lastError, undefined, "no '!' about it");
  assert.equal(ext.storage.badge, undefined);
  await ext.listeners.installed();
  await settle();
  assert.equal(titles(ext)["pako-link"], "Download link with Pako");
});

test("Pako can't be reached: the language last heard stays", async () => {
  const ext = loadEdition({ lang: "fr", stored: { pakoLang: "fr", langAskedAt: Date.now() } });
  await settle();
  assert.equal(ext.langAsks.length, 0, "asked a moment ago: not again at every start");
  ext.chrome.runtime.sendNativeMessage = (_h, _m, cb) => {
    ext.chrome.runtime.lastError = { message: "Specified native messaging host not found." };
    cb(undefined);
    ext.chrome.runtime.lastError = null;
  };
  await ext.listeners.installed();
  await settle();
  assert.equal(ext.storage.pakoLang, "fr");
  assert.equal(titles(ext)["pako-link"], "Télécharger le lien avec Pako");
  const r = await ask(ext, { type: "ping" });
  assert.match(r.error, /^Pako n’est pas connecté/);
  // The browser starting: asked again.
  ext.listeners.startup();
  await settle();
});

test("Firefox (lang.js loaded before the background script) and Pako Light speak it too", async () => {
  for (const opts of [{ firefox: true }, { light: true }]) {
    const ext = loadEdition({ lang: "fr", ...opts });
    await ext.listeners.installed();
    await settle();
    assert.equal(titles(ext)["pako-link"], "Télécharger le lien avec Pako", JSON.stringify(opts));
  }
  const light = loadEdition({ lang: "fr", light: true, tabs: { 1: "https://www.youtube.com/" } });
  await light.listeners.installed();
  await settle();
  await light.listeners.click({ menuItemId: "pako-page", pageUrl: "https://www.youtube.com/" }, { id: 1, url: "https://www.youtube.com/" });
  await settle();
  assert.equal(light.storage.lastError, "Pako Light ne fonctionne pas sur ce site.");
});

test("The pill's menu and the player's qualities follow Pako's language", async () => {
  // The background's menu for the pill.
  const ext = loadEdition({ lang: "fr", tabs: { 6: "https://watch.example.org/v/7" }, videos: [{ playing: true, locked: false }] });
  await settle();
  const menu = await ask(ext, { type: "pill-menu" }, { tab: { id: 6 } });
  assert.equal(menu.items[0].head, "Télécharger avec Pako");
  assert.ok(menu.items.some((i) => i.text === "Enregistrer cette vidéo" && i.side === "recharge la page"));
  // The player's own menu (pill.js + player.js, with lang.js before them).
  const changed = [];
  const chrome = {
    runtime: { sendMessage: async () => ({ ok: true }) },
    storage: { local: { get: async () => ({ pakoLang: "fr", playerButton: true }), set: async () => {} }, onChanged: { addListener: (f) => changed.push(f) } },
  };
  const context = { chrome, console, URL, setTimeout, clearTimeout, location: { href: "https://www.youtube.com/watch?v=abcdefghijk" }, document: { addEventListener() {}, getElementsByTagName: () => [] }, addEventListener() {} };
  context.globalThis = context;
  vm.createContext(context);
  for (const f of ["lang.js", "pill.js"]) vm.runInContext(source(f), context);
  let made = null;
  context.__pakoPill.make = (o) => ((made = o), { hide() {} });
  vm.runInContext(source("player.js"), context);
  await settle(5);
  const items = made.items();
  assert.equal(items[0].head, "Télécharger avec Pako");
  assert.deepEqual([...items.slice(1).map((i) => i.text)], ["Jusqu’à 1080p", "Jusqu’à 720p", "Jusqu’à 480p", "Audio seul"]);
  for (const f of changed) f({ pakoLang: { newValue: "en" } }, "local");
  assert.equal(made.items()[1].text, "Up to 1080p");
});
