// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Marketing screenshots of Pako's real window code, with the showcase sample set (src/lib/showcase.ts).
//
//   1. npm run dev                       (the window's design preview on http://localhost:1420)
//   2. node extension/build.mjs          (only for the Pako Light popup: its copy is in the store zip)
//   3. node scripts/screenshots.mjs [--light] [name ...]   (no names = all of them)
//
// Writes docs/screenshots/<name>.png at 2× and docs/screenshots/1x/<name>.png at 1×. With --light,
// the same pictures in Pako's light theme, as <name>-light.png (the window's saved choice
// "pako.theme" in its storage, src/lib/theme.svelte.ts; the extension popups follow the browser's
// light mode). Starts nothing
// itself. Pages open with "?showcase"; motion is off (prefers-reduced-motion) and the script waits
// for numbers and bars to settle, so nothing is caught mid-animation.
import { chromium } from "playwright";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
/** "dark" (Pako's first look) or "light" (--light). */
const THEME = args.includes("--light") ? "light" : "dark";
const SUFFIX = THEME === "light" ? "-light" : "";
const BASE = (process.env.PAKO_URL ?? "http://localhost:1420").replace(/\/$/, "");
const OUT = join(root, "docs", "screenshots");
const SCALES = [
  { scale: 2, dir: OUT },
  { scale: 1, dir: join(OUT, "1x") },
];
/** The main window's design size, and the small windows' own sizes (src-tauri: tray.rs, notices.rs, ask.rs). */
const MAIN = { width: 1440, height: 900 };
const MINI = { width: 360 + 14 * 2, height: 440 + 14 * 2 };
const POPUP_W = 368;
const ASK = { width: 420, height: 324 };

/** No transitions at all (the app's own motion already stops with reduced motion). */
const STILL_CSS = `*, *::before, *::after { transition-duration: 0s !important; transition-delay: 0s !important; caret-color: transparent !important; }
::-webkit-scrollbar { width: 0 !important; height: 0 !important; }`;

async function open(browser, scale, path, viewport) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: scale,
    locale: "en-GB",
    colorScheme: THEME,
    reducedMotion: "reduce",
  });
  // The theme the window shows before its first paint (src/app.html reads it).
  await context.addInitScript((t) => {
    try {
      localStorage.setItem("pako.theme", t);
    } catch {}
  }, THEME);
  const page = await context.newPage();
  page.on("pageerror", (e) => console.warn(`  page error on ${path}: ${e.message}`));
  await page.goto(BASE + path, { waitUntil: "networkidle" });
  await page.addStyleTag({ content: STILL_CSS });
  await page.evaluate(() => document.fonts.ready);
  return { context, page };
}

/** Lets numbers ease, lists settle and live speeds measure. */
const settle = (page, ms = 1600) => page.waitForTimeout(ms);
/** Moves the pointer off anything that reacts to it. */
const park = (page, x = 2, y = 899) => page.mouse.move(x, y);

async function sidebar(page, label) {
  await page.locator(".sidebar, nav, aside").getByText(label, { exact: true }).first().click();
  await settle(page, 700);
}
async function clickRow(page, text) {
  await page.getByText(text, { exact: true }).first().click();
}
/** Home shows the list's first row in the details panel without ticking it. The focus leaves
 *  the clicked tab first, so no keyboard ring shows on it. */
async function home(page) {
  await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
  await page.keyboard.press("Home");
}
async function closePanel(page) {
  await page.keyboard.press("Control+i");
}

const main = (path = "/?showcase") => ({ path, viewport: MAIN });

/** Every picture: where it opens, what to do there, and what to keep (the whole window, or a
 *  `clip` rectangle in page pixels). */
const SHOTS = {
  "main-downloads": {
    ...main(),
    async run(page) {
      // The first row (the newest) in the details panel, without ticking it.
      await home(page);
      await park(page, 700, 860);
      await settle(page, 1800);
    },
  },
  links: {
    ...main(),
    async run(page) {
      await sidebar(page, "Links");
      // The Drive folder (first row) in the details panel, still open in the list.
      await home(page);
      await park(page, 700, 860);
      await settle(page);
    },
  },
  "links-video": {
    ...main(),
    async run(page) {
      await sidebar(page, "Links");
      await clickRow(page, "Sunrise over the lake 4K.mp4");
      await park(page, 700, 860);
      await settle(page);
    },
  },
  upload: {
    ...main(),
    async run(page) {
      await sidebar(page, "Upload");
      await closePanel(page);
      await park(page, 700, 880);
      await settle(page);
    },
  },
  "upload-details": {
    ...main(),
    async run(page) {
      await sidebar(page, "Upload");
      await page.locator("main, .content, body").getByText("Holiday clips", { exact: true }).first().scrollIntoViewIfNeeded();
      await clickRow(page, "Holiday clips");
      await park(page, 700, 880);
      await settle(page, 2000);
    },
  },
  direct: {
    ...main(),
    async run(page) {
      await sidebar(page, "Direct");
      await park(page, 700, 880);
      // The live speed is measured from two updates a second apart.
      await settle(page, 3500);
    },
  },
  dashboard: {
    ...main(),
    async run(page) {
      await sidebar(page, "Dashboard");
      await park(page, 2, 700);
      await settle(page, 2200);
    },
  },
  "dashboard-more": {
    ...main(),
    async run(page) {
      await sidebar(page, "Dashboard");
      await settle(page, 1200);
      // The lower half: speeds by route, your people, sources and records.
      await page.getByText("Sending files", { exact: true }).first().evaluate((el) => {
        const card = el.closest("section") ?? el;
        let box = card.parentElement;
        while (box && !/(auto|scroll)/.test(getComputedStyle(box).overflowY)) box = box.parentElement;
        if (box) box.scrollTop += card.getBoundingClientRect().top - box.getBoundingClientRect().top - 16;
      });
      await park(page, 2, 700);
      await settle(page, 2200);
    },
  },
  settings: {
    ...main(),
    async run(page) {
      await page.locator(".sidebar, nav, aside").getByText("Settings", { exact: true }).first().click();
      await settle(page, 600);
      await page.getByRole("dialog").getByText("Downloads", { exact: true }).first().click();
      await park(page, 2, 700);
      await settle(page, 1200);
    },
  },
  updates: {
    ...main(),
    async run(page) {
      await page.locator(".sidebar, nav, aside").getByText("Settings", { exact: true }).first().click();
      await settle(page, 600);
      await page.getByRole("dialog").getByText("Features & updates", { exact: true }).first().click();
      await settle(page, 900);
      // The video feature's tools, with their versions and official sources.
      await page.getByRole("dialog").locator(".feat").nth(2).getByRole("button", { name: "Details" }).click();
      await settle(page, 600);
      // Down to the end, so the tools' list shows in full.
      await page.getByRole("dialog").locator(".details").first().evaluate((el) => {
        let box = el.parentElement;
        while (box && !/(auto|scroll)/.test(getComputedStyle(box).overflowY)) box = box.parentElement;
        if (box) box.scrollTop = box.scrollHeight;
      });
      await park(page, 2, 700);
      await settle(page, 1200);
    },
  },
  about: {
    ...main(),
    async run(page) {
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await page.keyboard.press("F1");
      await settle(page, 700);
      await page.getByRole("dialog").getByRole("button", { name: "About Pako" }).click();
      await park(page, 2, 700);
      await settle(page, 1000);
    },
  },
  report: {
    ...main(),
    async run(page) {
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await page.keyboard.press("F1");
      await settle(page, 700);
      await page.getByRole("dialog").getByRole("button", { name: "Report a problem", exact: true }).click();
      await settle(page, 700);
      await page.locator("#report-what").fill("A Google Drive folder stopped at 80% with \u201cServer error (502)\u201d. Retry worked the second time.");
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await park(page, 2, 700);
      await settle(page, 1000);
      await page.getByRole("dialog", { name: "Report a problem" }).evaluate((el) => el.setAttribute("data-shot", ""));
    },
    // Only the Report a problem window, on a see-through background (Help under it is left out).
    transparent: true,
    css: `html, body { background: transparent !important; }
      body * { visibility: hidden !important; }
      [data-shot], [data-shot] * { visibility: visible !important; }`,
    async clip(page) {
      const b = await page.locator("[data-shot]").boundingBox();
      const p = 24;
      return { x: b.x - p, y: b.y - p, width: b.width + p * 2, height: b.height + p * 2 };
    },
  },
  bell: {
    ...main(),
    async run(page) {
      await page.getByRole("button", { name: /^Notifications/ }).click();
      await park(page, 700, 880);
      await settle(page, 1000);
    },
    // Only the bell button and its panel, on a see-through background.
    transparent: true,
    css: `html, body { background: transparent !important; }
      body * { visibility: hidden !important; }
      .bell, .bell *, .panel[role="dialog"], .panel[role="dialog"] * { visibility: visible !important; }`,
    async clip(page) {
      const a = await page.locator(".bell").first().boundingBox();
      const b = await page.locator('.panel[role="dialog"]').boundingBox();
      const p = 20;
      const x = Math.min(a.x, b.x) - p;
      const y = Math.min(a.y, b.y) - p;
      return { x, y, width: Math.max(a.x + a.width, b.x + b.width) + p - x, height: Math.max(a.y + a.height, b.y + b.height) + p - y };
    },
  },
  tray: {
    path: "/mini?showcase",
    viewport: MINI,
    transparent: true,
    async run(page) {
      await park(page, 0, 0);
      await settle(page);
    },
  },
  notification: {
    path: "/popup?showcase",
    viewport: { width: POPUP_W, height: 420 },
    transparent: true,
    css: ".frame.preview { background: transparent !important; }",
    async run(page) {
      // Pointing at the cards keeps them (they leave by themselves after a few seconds).
      await page.locator(".stack").hover();
      await page.mouse.move(POPUP_W - 1, 1);
      await page.locator(".stack").hover({ position: { x: 4, y: 4 } });
      await settle(page, 800);
    },
    async clip(page) {
      const b = await page.locator(".stack").boundingBox();
      const p = 14;
      return { x: b.x - p, y: b.y - p, width: b.width + p * 2, height: b.height + p * 2 };
    },
  },
  "download-window": {
    path: "/ask?showcase",
    viewport: ASK,
    async run(page) {
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await park(page, 0, ASK.height - 1);
      await settle(page, 900);
    },
  },
  "pako-light-popup": { extension: "light" },
  "extension-popup": { extension: "full" },
};

// ---------- the browser extension's popup, standalone ----------

/** Folders holding each edition's popup files. Pako Light comes from the store zip (built by
 *  extension/build.mjs); the full extension from extension/ itself. */
function extensionDir(edition) {
  if (edition === "full") return { dir: join(root, "extension"), cleanup: () => {} };
  const zip = join(root, "dist", "extension", "pako-light-store.zip");
  if (!existsSync(zip)) throw new Error("dist/extension/pako-light-store.zip is missing: run `node extension/build.mjs` first");
  const dir = mkdtempSync(join(tmpdir(), "pako-light-"));
  execFileSync("powershell", ["-NoProfile", "-Command", `Expand-Archive -LiteralPath '${zip}' -DestinationPath '${dir}' -Force`]);
  return { dir, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

/** Just enough of `chrome.*` for the popup to say Connected. */
const CHROME_STUB = `(() => {
  const store = { takeOver: true, playerButton: true, lastError: "", lastDetail: "" };
  window.chrome = {
    runtime: { sendMessage: async () => ({ ok: true }), sendNativeMessage: (_h, _m, cb) => cb && cb({ ok: true }), lastError: undefined },
    storage: { local: { get: async (d) => ({ ...d, ...store }), set: async (v) => Object.assign(store, v) } },
    action: { setBadgeText: () => {} },
    tabs: { query: async () => [{ id: 1, url: "https://example.com/" }], update: async () => {}, create: async () => {} },
  };
})();`;

const TYPES = { ".html": "text/html", ".css": "text/css", ".js": "text/javascript", ".png": "image/png", ".svg": "image/svg+xml" };

async function shootExtension(browser, name, edition) {
  const { dir, cleanup } = extensionDir(edition);
  try {
    for (const { scale, dir: outDir } of SCALES) {
      const context = await browser.newContext({ viewport: { width: 360, height: 420 }, deviceScaleFactor: scale, colorScheme: THEME, reducedMotion: "reduce" });
      await context.addInitScript(CHROME_STUB);
      await context.route("http://pako-extension.local/**", (route) => {
        const file = join(dir, decodeURIComponent(new URL(route.request().url()).pathname));
        if (!existsSync(file)) return route.fulfill({ status: 404, body: "" });
        route.fulfill({ status: 200, contentType: TYPES[extname(file)] ?? "application/octet-stream", body: readFileSync(file) });
      });
      const page = await context.newPage();
      await page.goto("http://pako-extension.local/popup.html");
      // The popup as the browser shows it: its own width, floating with a soft shadow.
      const frame =
        THEME === "light"
          ? "border: 1px solid #dce1ea; box-shadow: 0 10px 30px rgba(20,26,46,.14), 0 2px 6px rgba(20,26,46,.08);"
          : "border: 1px solid #262a31; box-shadow: 0 10px 30px rgba(0,0,0,.45), 0 2px 6px rgba(0,0,0,.3);";
      await page.addStyleTag({ content: `html { background: transparent; padding: 16px; } body { border-radius: 10px; ${frame} }` });
      await page.waitForFunction(() => document.getElementById("status")?.textContent === "Connected");
      await settle(page, 300);
      const box = await page.locator("body").boundingBox();
      const pad = 16;
      await page.screenshot({ path: join(outDir, `${name}${SUFFIX}.png`), omitBackground: true, clip: { x: box.x - pad, y: box.y - pad, width: box.width + pad * 2, height: box.height + pad * 2 } });
      await context.close();
    }
  } finally {
    cleanup();
  }
}

// ---------- run ----------

async function shoot(browser, name, shot) {
  if (shot.extension) return shootExtension(browser, name, shot.extension);
  for (const { scale, dir } of SCALES) {
    const { context, page } = await open(browser, scale, shot.path, shot.viewport);
    await shot.run(page);
    if (shot.css) await page.addStyleTag({ content: shot.css });
    const file = join(dir, `${name}${SUFFIX}.png`);
    if (shot.clip) {
      const c = await shot.clip(page);
      const x = Math.max(0, c.x);
      const y = Math.max(0, c.y);
      const width = Math.min(shot.viewport.width - x, c.width);
      const height = Math.min(shot.viewport.height - y, c.height);
      await page.screenshot({ path: file, omitBackground: !!shot.transparent, clip: { x, y, width, height } });
    } else {
      await page.screenshot({ path: file, omitBackground: !!shot.transparent });
    }
    await context.close();
  }
}

const wanted = args.filter((a) => !a.startsWith("--"));
const unknown = wanted.filter((n) => !SHOTS[n]);
if (unknown.length) {
  console.error(`Unknown: ${unknown.join(", ")}. Known: ${Object.keys(SHOTS).join(", ")}`);
  process.exit(1);
}
try {
  const res = await fetch(BASE + "/");
  if (!res.ok) throw new Error(String(res.status));
} catch {
  console.error(`Nothing answers on ${BASE}. Start the design preview first: npm run dev`);
  process.exit(1);
}
for (const { dir } of SCALES) mkdirSync(dir, { recursive: true });
const browser = await chromium.launch();
let failed = 0;
try {
  for (const name of wanted.length ? wanted : Object.keys(SHOTS)) {
    process.stdout.write(`${name}${SUFFIX} … `);
    try {
      await shoot(browser, name, SHOTS[name]);
      console.log("ok");
    } catch (e) {
      failed++;
      console.log("FAILED: " + (e?.message ?? e));
    }
  }
} finally {
  await browser.close();
}
process.exit(failed ? 1 : 0);
