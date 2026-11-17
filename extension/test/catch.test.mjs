// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Catching a download at the server's answer (1.2.1): in a browser that asks "Download …? Save /
// Save As / Open / Cancel" before any extension hears of the download (Vivaldi), a rule of the
// extension's own turns a page's file answer away, so the browser asks nothing and downloads
// nothing, and the extension hands it to Pako like a takeover. Never a form's answer (POST), a
// private window's tab, a video site or a small file; never while Pako's helper doesn't answer;
// never in a browser without such rules (the file-name step stays); never in Pako Light. What it
// can't hand to Pako goes back to the browser (a site's own sign-in; a private tab in the race).
// The sent page goes back, or stays: a form's answer behind it, the same file again (no loop).
// Run: node --test extension/test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { ask, browse, loadEdition, settle } from "./fake-browser.mjs";

const SENT_URL = "chrome-extension://1c4ea6cf-7588-4816-8277-3a2758e249c6/pako-sent.html";
/** What the extension tells the sent page that opened in this tab. */
const sentPage = (ext, tabId = 3) => ask(ext, { type: "sent-page" }, { id: ext.id, tab: { id: tabId }, url: SENT_URL });

const TAB = 3;
const PRIV = 4;
const PAGE = "https://files.example.com/downloads";
const FILE = "https://files.example.com/get/big.zip";
/** The server's answer for a file: "attachment", with its name, kind and size. */
const ATTACHMENT = [
  { name: "Content-Disposition", value: 'attachment; filename="big.zip"' },
  { name: "Content-Type", value: "application/zip" },
  { name: "Content-Length", value: "50000000" },
];

/** A Pako that answers at once ("held"), then the check's `result`. What it was told, in `said`. */
function newPako({ result = { ok: true } } = {}) {
  const said = [];
  const reply = (m) => {
    said.push(m.kind + (m.hold ? "+hold" : ""));
    if (m.kind === "held") return { ...result, holds: true };
    return m.hold ? { ok: true, held: "tok-1", holds: true } : { ok: true, holds: true };
  };
  return { said, reply };
}

/** Vivaldi with Pako's helper answering (it said it holds takeovers). */
async function vivaldi(opts = {}, pako = newPako()) {
  const ext = loadEdition({
    dnr: "full",
    vivaldi: true,
    naming: true,
    tabs: { [TAB]: PAGE, [PRIV]: PAGE },
    privateTabs: [PRIV],
    privateAllowed: true,
    reply: pako.reply,
    stored: { pakoHolds: true },
    ...opts,
  });
  await settle(30);
  return ext;
}

test("Vivaldi: a file's answer is caught and handed to Pako; the browser downloads nothing", async () => {
  const pako = newPako();
  const ext = await vivaldi({}, pako);
  // The rule: page and frame answers asked with GET, a file by its headers, never a small one,
  // never on the video sites, never in a private window's tab.
  const rule = ext.rules.get(1);
  assert.ok(rule, "the catch rule is in the browser");
  assert.equal(rule.action.type, "redirect");
  assert.equal(rule.action.redirect.extensionPath, "/pako-sent.html");
  assert.deepEqual(rule.condition.resourceTypes, ["main_frame", "sub_frame"]);
  assert.deepEqual(rule.condition.requestMethods, ["get"]);
  assert.deepEqual(rule.condition.excludedTabIds, [PRIV]);
  assert.ok(rule.condition.excludedRequestDomains.includes("youtube.com"));
  assert.ok(rule.condition.excludedInitiatorDomains.includes("youtube.com"));

  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  // The sent page goes back to the page (nothing behind it was a form's answer).
  assert.deepEqual(await sentPage(ext), { ok: true, stay: false });
  await settle(40);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
  const m = ext.sent[0];
  assert.equal(m.takeover, true);
  assert.equal(m.hold, true);
  assert.equal(m.filename, "big.zip");
  assert.equal(m.totalBytes, 50e6);
  assert.equal(m.mime, "application/zip");
  assert.equal(m.referrer, PAGE, "the page it was clicked on");
  assert.equal(m.private, undefined);
  // Nothing to cancel, pause or start again in the browser: it never had a download.
  assert.deepEqual(ext.log, []);
  assert.deepEqual(ext.restarted, []);
  assert.equal(ext.storage.badge ?? "", "");
  // A frame's answer and a kind of file the browser never shows (no "attachment") too.
  assert.equal(await browse(ext, { url: "https://files.example.com/setup.exe", type: "sub_frame", responseHeaders: [{ name: "Content-Type", value: "application/x-msdownload" }] }), "caught");
});

test("Never caught: a form's POST, a private window, a video site, a small file, a page", async () => {
  const ext = await vivaldi();
  const cases = [
    { url: FILE, method: "POST", responseHeaders: ATTACHMENT },
    { url: FILE, tabId: PRIV, responseHeaders: ATTACHMENT },
    { url: "https://rr1.googlevideo.youtube.com/file.zip", responseHeaders: ATTACHMENT },
    { url: FILE, initiator: "https://www.youtube.com", responseHeaders: ATTACHMENT },
    { url: FILE, responseHeaders: [ATTACHMENT[0], ATTACHMENT[1], { name: "Content-Length", value: "1500000" }] },
    { url: FILE, responseHeaders: [ATTACHMENT[0], ATTACHMENT[1], { name: "Content-Length", value: "2000123" }] },
    { url: PAGE, responseHeaders: [{ name: "Content-Type", value: "text/html" }] },
    { url: "https://files.example.com/doc.pdf", responseHeaders: [{ name: "Content-Type", value: "application/pdf" }] },
  ];
  for (const c of cases) assert.equal(await browse(ext, c), "download", JSON.stringify(c));
  await settle(30);
  assert.deepEqual(ext.sent, [], "nothing went to Pako at the server's answer");
  // A file just over 2 MB, or of unknown size, is caught.
  assert.equal(await browse(ext, { url: FILE, responseHeaders: [ATTACHMENT[0], { name: "Content-Length", value: "2100000" }] }), "caught");
  assert.equal(await browse(ext, { url: "https://files.example.com/x.bin", responseHeaders: [{ name: "Content-Disposition", value: "ATTACHMENT" }] }), "caught");
});

test("A site's own sign-in: caught (the browser's rules decide before the extension hears of it), then started again in the browser, not sent to Pako", async () => {
  const ext = await vivaldi();
  const url = "https://intranet.example.com/report.zip";
  const r = await browse(ext, { url, requestHeaders: [{ name: "Authorization", value: "Basic c2VjcmV0" }], responseHeaders: ATTACHMENT });
  assert.equal(r, "caught");
  await settle(40);
  assert.equal([...ext.rules.values()].filter((x) => x.action.type === "allow").length, 0, "no exception: it couldn't count for this request anyway");
  assert.ok(!ext.sent.some((m) => m.kind === "download"), "Pako can't send a site's own sign-in: not asked");
  assert.equal(ext.restarted.length, 1);
  assert.equal(ext.restarted[0].url, url);
  // Pako lists the browser's download ("Via browser") once it answers.
  assert.ok(ext.sent.some((m) => m.kind === "follow" && m.urls[0] === url));
});

test("Pako unreachable: the rule goes at once, the download is started again in the browser with '!', and the rule comes back when Pako answers", async () => {
  let up = false;
  const ext = await vivaldi({ reply: (m) => (up ? { ok: true, holds: true } : undefined) });
  assert.ok(ext.rules.get(1));
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  await settle(60);
  // Started again by the extension, which is never caught: a download request, not a page's.
  assert.deepEqual(ext.log, ["full download"]);
  assert.equal(ext.restarted[0].url, FILE);
  assert.equal(ext.restarted[0].filename, "big.zip");
  assert.equal(ext.storage.badge, "!");
  assert.equal(ext.storage.pakoAway, true);
  assert.equal(ext.rules.get(1), undefined, "no rule while Pako doesn't answer");
  assert.equal(await browse(ext, { url: FILE, type: "other", responseHeaders: ATTACHMENT }), "download");
  // The browser's own download of it is never taken over again either.
  const before = ext.sent.length;
  ext.listeners.naming({ id: 99, url: FILE, byExtensionId: ext.id, totalBytes: 50e6 }, () => {});
  await settle(30);
  assert.equal(ext.sent.length, before);
  // The next download goes the old way (the file-name step) until Pako answers.
  assert.equal(await browse(ext, { url: FILE.replace("big", "next"), responseHeaders: ATTACHMENT }), "download");
  // Pako answers again (the toolbar window, a report, the retry): the rule is back.
  up = true;
  await ask(ext, { type: "ping" });
  await settle(30);
  assert.equal(ext.storage.pakoAway, false);
  assert.ok(ext.rules.get(1), "the rule is back");
});

test("Pako says no (not its file): started again in the browser and followed; the rule stays", async () => {
  const why = "Pako couldn't get this download (the server gave Pako a web page instead of the file), so your browser downloads it.";
  const ext = await vivaldi({}, newPako({ result: { ok: false, error: why, followed: "row-1" } }));
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  await settle(60);
  assert.equal(ext.restarted.length, 1);
  assert.ok(ext.rules.get(1), "Pako was reached: the rule stays");
  assert.ok(ext.sent.some((m) => m.kind === "follow" && m.follow.pako === "row-1"), "Pako follows the browser's download");
});

test("A private window's tab caught before the rule left it out: back to that tab, where the browser downloads it (held paused for Pako); never in your normal window", async () => {
  const pako = newPako();
  const ext = await vivaldi({ privateTabs: [] }, pako);
  // A new private window's tab, in the moment before the rule could leave it out.
  const NEW = 7;
  ext.chrome.tabs.get = async (id) => ({ id, url: "about:blank", incognito: true, windowId: 2 });
  ext.chrome.tabs.update = async (id, props) => {
    ext.navigated.push({ tabId: id, ...props });
    return { id };
  };
  assert.equal(await browse(ext, { url: FILE, tabId: NEW, responseHeaders: ATTACHMENT }), "caught");
  // Its sent page stays (the tab is needed), and says what happens.
  const a = await sentPage(ext, NEW);
  assert.equal(a.stay, true);
  assert.match(a.said, /private window's download goes on in your browser/);
  await settle(40);
  assert.deepEqual(pako.said, [], "not sent to Pako from the caught answer");
  assert.deepEqual(ext.restarted, [], "never started again in the normal window");
  // Let through once, for that tab and address, before the tab asks again.
  const allow = [...ext.rules.values()].find((r) => r.action.type === "allow");
  assert.deepEqual(allow.condition, { urlFilter: `|${FILE}|`, tabIds: [NEW], resourceTypes: ["main_frame"] });
  assert.deepEqual(ext.navigated, [{ tabId: NEW, url: FILE }]);
  // The address first answers with a redirect back to itself (a cookie set on the way): not used
  // yet. The browser checks the rules again for the next request (taken out at the redirect, the
  // file was caught again in Vivaldi 8.3, and lost: the page said it went on in the browser).
  await browse(ext, { url: FILE, tabId: NEW, statusCode: 302, responseHeaders: [{ name: "Location", value: FILE }] });
  await settle(10);
  assert.equal([...ext.rules.values()].filter((r) => r.action.type === "allow").length, 1, "kept through a redirect");
  assert.equal(await browse(ext, { url: FILE, tabId: NEW, responseHeaders: ATTACHMENT }), "download");
  // Used: the exception goes (it doesn't let that address through in that tab until the
  // browser closes).
  await settle(10);
  assert.equal([...ext.rules.values()].filter((r) => r.action.type === "allow").length, 0, "one-time: gone once used");
  // The browser downloads it there: the file-name step holds it paused while Pako checks it.
  await ext.listeners.naming({ id: 8, url: FILE, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "", incognito: true }, () => {});
  await settle(40);
  assert.equal(ext.log[0], "full pause");
  assert.equal(ext.sent.find((m) => m.kind === "download").private, true);
  // The tab closed meanwhile: said in memory only, never on disk.
  const gone = await vivaldi({ privateTabs: [] });
  gone.chrome.tabs.get = async (id) => ({ id, url: "about:blank", incognito: true });
  assert.equal(await browse(gone, { url: FILE, tabId: NEW, responseHeaders: ATTACHMENT }), "caught");
  await settle(40);
  assert.match(gone.session.lastError, /couldn't hand "big.zip" back to your private window/);
  assert.equal(gone.storage.lastError ?? "", "");
  // Its sent page doesn't say it goes on in the browser: it says it didn't.
  const failed = await sentPage(gone, NEW);
  assert.equal(failed.stay, true);
  assert.doesNotMatch(failed.said, /goes on in your browser/);
  assert.match(failed.said, /couldn't go on in your browser\. Use Back and download it once more/);
  assert.equal([...gone.rules.values()].filter((r) => r.action.type === "allow").length, 0, "no exception left behind");
  // A new private tab is left out of the rule at once.
  ext.chrome.tabs.query = async () => [{ id: 9, url: PAGE, incognito: true, vivExtData: "{}" }];
  ext.listeners.tabCreated({ id: 9, incognito: true });
  await settle(30);
  assert.deepEqual(ext.rules.get(1).condition.excludedTabIds, [9]);
});

test("A private tab's one-time exception goes when its tab closes before using it, and one left before the background slept goes at its next start", async () => {
  const ext = await vivaldi({ privateTabs: [] });
  const NEW = 7;
  ext.chrome.tabs.get = async (id) => ({ id, url: "about:blank", incognito: true, windowId: 2 });
  ext.chrome.tabs.update = async (id) => ({ id });
  assert.equal(await browse(ext, { url: FILE, tabId: NEW, responseHeaders: ATTACHMENT }), "caught");
  await settle(40);
  const allows = (e) => [...e.rules.values()].filter((r) => r.action.type === "allow");
  assert.equal(allows(ext).length, 1);
  // Another tab's answer for the same address, or this tab's for another one: not used yet.
  await browse(ext, { url: FILE, tabId: TAB, responseHeaders: [{ name: "Content-Type", value: "text/html" }] });
  await browse(ext, { url: PAGE, tabId: NEW, responseHeaders: [{ name: "Content-Type", value: "text/html" }] });
  await settle(10);
  assert.equal(allows(ext).length, 1);
  ext.listeners.removed(NEW);
  await settle(10);
  assert.equal(allows(ext).length, 0, "the tab closed: gone");
  // The background slept with one still there: gone when it starts again; the catch rule stays.
  const left = { id: 1003, priority: 2, action: { type: "allow" }, condition: { urlFilter: `|${FILE}|`, tabIds: [NEW], resourceTypes: ["main_frame"] } };
  const woke = await vivaldi({ sessionRules: [left] });
  assert.equal(allows(woke).length, 0);
  assert.ok(woke.rules.get(1));
});

test("No rule when: the takeover switch is off, an older Pako, Firefox's way, Pako Light", async () => {
  const ext = await vivaldi();
  assert.ok(ext.rules.get(1));
  await ext.chrome.storage.local.set({ takeOver: false });
  await settle(30);
  assert.equal(ext.rules.get(1), undefined, "switched off: no rule");
  await ext.chrome.storage.local.set({ takeOver: true });
  await settle(30);
  assert.ok(ext.rules.get(1), "on again: the rule is back");

  const older = await vivaldi({ stored: {} });
  assert.equal(older.rules.size, 0, "a Pako that doesn't answer at once: no rule");

  const light = await vivaldi({ light: true });
  assert.equal(light.rules.size, 0, "Pako Light keeps the file-name step");
  assert.equal(await browse(light, { url: FILE, responseHeaders: ATTACHMENT }), "download");
  await light.listeners.naming({ id: 7, url: FILE, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "" }, () => {});
  await settle(40);
  assert.deepEqual(light.log, ["light pause", "light cancel", "light erase"], "Light as before: held paused until Pako has it");
});

test("A browser without rules on the server's answer (older than Chrome 128): the file-name step, as before", async () => {
  const pako = newPako();
  const ext = await vivaldi({ dnr: "old" }, pako);
  assert.equal(ext.rules.size, 0);
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "download");
  // The browser makes it a download: taken over at the file-name step, as in 1.2.0.
  const order = [];
  await ext.listeners.naming({ id: 7, url: FILE, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "" }, () => order.push("suggest"));
  await settle(40);
  assert.deepEqual(ext.log, ["full cancel", "full erase"]);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
});

test("Chrome (no question first): no rule; a browser seen asking first twice in a row learns it, one told at once unlearns it", async () => {
  const pako = newPako();
  const ext = await vivaldi({ vivaldi: false }, pako);
  assert.equal(ext.rules.size, 0, "the file-name step is already before any question");
  // The browser tells the extension more than a second after the server's answer: it asked (or
  // it was Chrome's own "download multiple files?" question: once isn't enough).
  const slowOne = async (n) => {
    assert.equal(await browse(ext, { url: FILE + n, responseHeaders: ATTACHMENT }), "download");
    await settle(1100);
    await ext.listeners.naming({ id: 7, url: FILE + n, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "" }, () => {});
    await settle(40);
  };
  await slowOne(1);
  assert.deepEqual(ext.log, ["full cancel", "full erase"], "that one is taken over as before");
  assert.notEqual(ext.storage.asksFirst, true, "once isn't enough");
  assert.equal(ext.rules.size, 0);
  await slowOne(2);
  assert.equal(ext.storage.asksFirst, true);
  assert.ok(ext.rules.get(1), "the next one is caught at the server's answer");
  // A download the rule doesn't catch (a link with the download attribute) told at once: this
  // browser doesn't ask after all.
  await browse(ext, { url: FILE + 3, type: "other", responseHeaders: ATTACHMENT });
  await ext.listeners.naming({ id: 7, url: FILE + 3, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "" }, () => {});
  await settle(40);
  assert.equal(ext.storage.asksFirst, false);
  assert.equal(ext.rules.get(1), undefined, "unlearnt: back to the file-name step");
  // Told at once (no question): nothing learnt.
  const quick = await vivaldi({ vivaldi: false });
  await browse(quick, { url: FILE, responseHeaders: ATTACHMENT });
  await quick.listeners.naming({ id: 7, url: FILE, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "" }, () => {});
  await settle(40);
  assert.notEqual(quick.storage.asksFirst, true);
  assert.equal(quick.rules.size, 0);
});

test("Two downloads told at once, both after the browser asked first: both count (learnt, not one lost)", async () => {
  const ext = await vivaldi({ vivaldi: false });
  for (const n of [1, 2]) assert.equal(await browse(ext, { url: FILE + n, responseHeaders: ATTACHMENT }), "download");
  await settle(1100);
  // The browser tells the extension of both in the same moment.
  for (const n of [1, 2]) ext.listeners.naming({ id: 7 + n, url: FILE + n, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "" }, () => {});
  await settle(60);
  assert.equal(ext.storage.askedSlow, 2);
  assert.equal(ext.storage.asksFirst, true);
  assert.ok(ext.rules.get(1), "learnt: the next one is caught at the server's answer");
});

test("Firefox (no declarativeNetRequest asked for): downloads.onCreated as before", async () => {
  const ext = await vivaldi({ dnr: false, naming: false });
  assert.equal(ext.rules.size, 0);
  await ext.listeners.download({ id: 7, url: FILE, referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "" });
  await settle(40);
  assert.ok(ext.sent.some((m) => m.kind === "download" && m.takeover));
});

test("A page that asks for its file again by itself (reloaded by going back): not handed twice, and its tab stays (no loop)", async () => {
  const pako = newPako();
  const ext = await vivaldi({}, pako);
  // A "your download starts in 5 seconds" page: caught, the sent page goes back, the page loads
  // again (no back-forward cache) and asks for the same file again.
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(ext)).stay, false, "the first time: back to the page");
  await settle(40);
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  const again = await sentPage(ext);
  assert.equal(again.stay, true, "the second time: the tab stays, so the page doesn't load again");
  assert.match(again.said, /went to Pako\. Use Back/);
  await settle(40);
  assert.deepEqual(pako.said, ["download+hold", "held"], "handed to Pako once");
  assert.deepEqual(ext.restarted, [], "nor started again in the browser");
  // Another tab, or another file: as usual.
  assert.equal(await browse(ext, { url: FILE, tabId: 5, responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(ext, 5)).stay, false);
  await settle(40);
  assert.equal(pako.said.filter((s) => s === "download+hold").length, 2);
});

test("A page that asks for its file again through a link signed anew each time (or another mirror): still the same file, not handed twice (no loop)", async () => {
  const pako = newPako();
  const ext = await vivaldi({}, pako);
  // "Your download starts in a moment": each load of the page gets a new signed address.
  const signed = (n) => `https://cdn${n}.example.net/get/big.zip?sig=${n}&expires=17${n}`;
  assert.equal(await browse(ext, { url: signed(1), responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(ext)).stay, false);
  await settle(40);
  assert.equal(await browse(ext, { url: signed(2), responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(ext)).stay, true, "the tab stays: no loop");
  await settle(40);
  assert.equal(pako.said.filter((s) => s === "download+hold").length, 1, "handed to Pako once");
  // Another file from the same page (another name, or another size): handed as usual.
  const other = ATTACHMENT.map((h) => (h.name === "Content-Disposition" ? { ...h, value: 'attachment; filename="other.zip"' } : h));
  assert.equal(await browse(ext, { url: signed(3), responseHeaders: other }), "caught");
  assert.equal((await sentPage(ext)).stay, false);
  const bigger = ATTACHMENT.map((h) => (h.name === "Content-Length" ? { ...h, value: "60000000" } : h));
  assert.equal(await browse(ext, { url: signed(4), responseHeaders: bigger }), "caught");
  await settle(40);
  assert.equal(pako.said.filter((s) => s === "download+hold").length, 3);
});

test("A form's answer read longer than the background stays awake (it sleeps after 30 s): the sent page still stays", async () => {
  const before = await vivaldi();
  before.listeners.headers({ url: "https://files.example.com/search", type: "main_frame", method: "POST", tabId: TAB, frameId: 0, statusCode: 200, responseHeaders: [{ name: "Content-Type", value: "text/html" }] });
  await settle(10);
  // The background wakes again for the download: its memory is new, the browser's is kept.
  const ext = await vivaldi({ sessionStored: before.session });
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(ext)).stay, true, "going back would send the form again");
  // A plain page after it: going back is safe again, also after the next sleep.
  ext.listeners.headers({ url: "https://files.example.com/done", type: "main_frame", method: "GET", tabId: TAB, frameId: 0, statusCode: 200, responseHeaders: [{ name: "Content-Type", value: "text/html" }] });
  await settle(10);
  const later = await vivaldi({ sessionStored: ext.session });
  assert.equal(await browse(later, { url: FILE.replace("big", "other"), responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(later)).stay, false);
});

test("A main-frame answer that never replaced the page (204, a redirect, a stopped load) isn't the page behind: a form's answer still is", async () => {
  const html = [{ name: "Content-Type", value: "text/html" }];
  const formAnswer = (ext) => ext.listeners.headers({ url: "https://files.example.com/search", type: "main_frame", method: "POST", tabId: TAB, frameId: 0, statusCode: 200, requestId: "r1", responseHeaders: html });
  const ext = await vivaldi();
  formAnswer(ext);
  // A link that answers "204 No Content" (the browser stays on the page), and a link that
  // redirects (the page it leads to, if any, is noted then).
  ext.listeners.headers({ url: "https://files.example.com/ping", type: "main_frame", method: "GET", tabId: TAB, frameId: 0, statusCode: 204, requestId: "r2", responseHeaders: [] });
  ext.listeners.headers({ url: "https://files.example.com/go", type: "main_frame", method: "GET", tabId: TAB, frameId: 0, statusCode: 302, requestId: "r3", responseHeaders: [] });
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(ext)).stay, true, "the form's answer is still behind");
  // A page whose load was stopped after its answer came (Esc): the form's answer is still behind.
  const stopped = await vivaldi();
  formAnswer(stopped);
  stopped.listeners.headers({ url: "https://files.example.com/slow", type: "main_frame", method: "GET", tabId: TAB, frameId: 0, statusCode: 200, requestId: "r4", responseHeaders: html });
  stopped.listeners.requestError({ url: "https://files.example.com/slow", type: "main_frame", tabId: TAB, requestId: "r4", error: "net::ERR_ABORTED" });
  await settle(10);
  assert.equal(await browse(stopped, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(stopped)).stay, true);
  // The same after the background slept in between (the form's answer known from the browser's memory).
  const before = await vivaldi();
  formAnswer(before);
  await settle(10);
  const woke = await vivaldi({ sessionStored: before.session });
  woke.listeners.headers({ url: "https://files.example.com/slow", type: "main_frame", method: "GET", tabId: TAB, frameId: 0, statusCode: 200, requestId: "r5", responseHeaders: html });
  woke.listeners.requestError({ url: "https://files.example.com/slow", type: "main_frame", tabId: TAB, requestId: "r5", error: "net::ERR_ABORTED" });
  await settle(10);
  assert.equal(await browse(woke, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(woke)).stay, true);
  // A page that did load (its answer, then no error): going back is safe.
  const loaded = await vivaldi();
  formAnswer(loaded);
  loaded.listeners.headers({ url: "https://files.example.com/done", type: "main_frame", method: "GET", tabId: TAB, frameId: 0, statusCode: 200, requestId: "r6", responseHeaders: html });
  loaded.listeners.requestError({ url: "https://files.example.com/img.png", type: "main_frame", tabId: TAB, requestId: "r7", error: "net::ERR_ABORTED" });
  await settle(10);
  assert.equal(await browse(loaded, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  assert.equal((await sentPage(loaded)).stay, false);
});

test("The page behind was a form's answer (POST): the sent page stays, so going back doesn't send the form again", async () => {
  const ext = await vivaldi();
  // The tab's page came from a form (its answer is a page, not a file).
  ext.listeners.headers({ url: "https://files.example.com/search", type: "main_frame", method: "POST", tabId: TAB, frameId: 0, statusCode: 200, responseHeaders: [{ name: "Content-Type", value: "text/html" }] });
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  const a = await sentPage(ext);
  assert.equal(a.stay, true);
  await settle(40);
  assert.equal(ext.sent.find((m) => m.kind === "download")?.url ?? ext.sent[0].urls[0], FILE, "handed to Pako all the same");
  // A form that answered with a redirect to a page (POST, then GET): going back is safe.
  ext.listeners.headers({ url: "https://files.example.com/done", type: "main_frame", method: "GET", tabId: TAB, frameId: 0, statusCode: 200, responseHeaders: [{ name: "Content-Type", value: "text/html" }] });
  // (Another file: another name. The same name and size would be the same file again.)
  const other = ATTACHMENT.map((h) => (h.name === "Content-Disposition" ? { ...h, value: 'attachment; filename="other.zip"' } : h));
  assert.equal(await browse(ext, { url: FILE.replace("big", "other"), responseHeaders: other }), "caught");
  assert.equal((await sentPage(ext)).stay, false);
});

test("The sent page waits for the extension to note its tab: a tab opened only for the file is known as a normal one before it closes", async () => {
  const pako = newPako();
  const ext = await vivaldi({}, pako);
  let open = true;
  // The browser answers about the tab a moment later (a background page just woken up).
  ext.chrome.tabs.get = async (id) => {
    await settle(15);
    return open ? { id, url: "about:blank", incognito: false } : undefined;
  };
  assert.equal(await browse(ext, { url: FILE, tabId: 6, responseHeaders: ATTACHMENT }), "caught");
  const a = await sentPage(ext, 6);
  assert.equal(a.stay, false);
  // Only now does the page close its tab.
  open = false;
  await settle(60);
  const m = ext.sent.find((x) => x.kind === "download");
  assert.equal(m.private, undefined, "a normal window's download, with its login");
  assert.equal(m.cookies, "SID=x");
});

// ---------- the sent page itself (pako-sent.js) ----------

const here = dirname(fileURLToPath(import.meta.url));
const SENT_JS = readFileSync(join(here, "..", "pako-sent.js"), "utf8");

/** Runs the sent page: `answer` is what the extension says (undefined: no answer). */
function openSent({ answer, length = 2, framed = false, how = "navigate" } = {}) {
  const did = [];
  const p = { hidden: true, textContent: "This download went to Pako." };
  const timers = [];
  const win = {};
  win.top = framed ? {} : win;
  const chrome = {
    runtime: {
      lastError: null,
      sendMessage: (msg, cb) => {
        did.push(`asked ${msg.type}`);
        if (answer !== undefined) cb(answer);
      },
    },
    tabs: { getCurrent: (cb) => cb({ id: 9 }), remove: (id, cb) => (did.push(`closed ${id}`), cb?.()) },
  };
  const context = {
    chrome,
    window: win,
    history: { length, back: () => did.push("back") },
    document: { readyState: "complete", getElementById: () => p, addEventListener() {} },
    performance: { getEntriesByType: () => [{ type: how }] },
    addEventListener: (name, f) => (context.onpageshow = f),
    setTimeout: (f) => timers.push(f),
  };
  context.globalThis = context;
  vm.runInContext(SENT_JS, vm.createContext(context));
  return { did, p, timers, context };
}

test("Sent page: goes back, closes a tab opened only for the file, or stays and says why when told", () => {
  assert.deepEqual(openSent({ answer: { ok: true, stay: false } }).did, ["asked sent-page", "back"]);
  assert.deepEqual(openSent({ answer: { ok: true, stay: false }, length: 1 }).did, ["asked sent-page", "closed 9"]);
  const stay = openSent({ answer: { ok: true, stay: true, said: "Use Back." }, length: 1 });
  assert.deepEqual(stay.did, ["asked sent-page"], "never closed nor gone back");
  assert.equal(stay.p.hidden, false);
  assert.equal(stay.p.textContent, "Use Back.");
  // No answer (the extension restarting): back after a while, as before.
  const quiet = openSent({});
  assert.deepEqual(quiet.did, ["asked sent-page"]);
  quiet.timers.forEach((f) => f());
  assert.deepEqual(quiet.did, ["asked sent-page", "back"]);
  // In a frame, or come back to with Back, Forward or Reload: only says it, asks nothing.
  for (const o of [{ framed: true }, { how: "back_forward" }, { how: "reload" }]) {
    const r = openSent({ answer: { ok: true, stay: false }, ...o });
    assert.deepEqual(r.did, [], JSON.stringify(o));
    assert.equal(r.p.hidden, false);
  }
  // Back here from the back-forward cache: it says it (not a blank page).
  const cached = openSent({ answer: { ok: true, stay: false } });
  cached.p.hidden = true;
  cached.context.onpageshow({ persisted: true });
  assert.equal(cached.p.hidden, false);
});

test("A Chromium browser that also names its API `browser` (Vivaldi 8.3, Chromium 156) isn't taken for Firefox", async () => {
  const pako = newPako();
  const ext = await vivaldi({ alias: true }, pako);
  assert.ok(ext.rules.get(1), "the catch rule is set");
  assert.equal(await browse(ext, { url: FILE, responseHeaders: ATTACHMENT }), "caught");
  await settle(40);
  assert.deepEqual(pako.said, ["download+hold", "held"]);
  // A private window's download (not caught): held paused, never cancelled first (it couldn't be
  // started again there: Firefox's way would lose it).
  await browse(ext, { url: FILE.replace("big", "p"), tabId: PRIV, responseHeaders: ATTACHMENT });
  await ext.listeners.naming({ id: 8, url: FILE.replace("big", "p"), referrer: PAGE, mime: "application/zip", totalBytes: 50e6, filename: "", incognito: true }, () => {});
  await settle(40);
  assert.equal(ext.log[0], "full pause");
});
