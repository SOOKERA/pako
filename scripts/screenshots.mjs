// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Marketing screenshots of Pako's real window code, with the showcase sample set (src/lib/showcase.ts).
//
//   1. npm run dev                       (the window's design preview on http://localhost:1420)
//   2. node extension/build.mjs          (only for the Pako Light popup: its copy is in the store zip)
//   3. node scripts/screenshots.mjs [--light] [name ...]   (no names = all of them)
//
// video-preview-site also needs ffmpeg (on the PATH, in FFMPEG, or the copy Pako installed in
// %LOCALAPPDATA%\Pako\components\ffmpeg): it makes a short sample clip to play.
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

async function open(browser, scale, path, viewport, before, columns) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: scale,
    locale: "en-GB",
    timezoneId: "UTC",
    colorScheme: THEME,
    reducedMotion: "reduce",
  });
  // The theme the window shows before its first paint (src/app.html reads it).
  await context.addInitScript((t) => {
    try {
      localStorage.setItem("pako.theme", t);
    } catch {}
  }, THEME);
  // The list's columns as a person set them (only for pictures that need room for names).
  if (columns)
    await context.addInitScript((c) => {
      try {
        localStorage.setItem("pako.listColumns", JSON.stringify(c));
      } catch {}
    }, columns);
  if (before) await before(context);
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
/** The share pictures: smaller windows (each the size where its page reads best), cropped to
 *  about 4:3 without the title bar and the sidebar. */
const SHARE_DROP = { width: 1100, height: 700 };
const SHARE_LINK = { width: 1200, height: 800 };
const SHARE_OPEN = { width: 1100, height: 700 };
/** The page and its details panel below the title bar, right of the sidebar; `bar` keeps the
 *  bottom bar (32 px). */
const pageClip = (v, bar = true) => () => ({ x: 220, y: 40, width: v.width - 220, height: v.height - 40 - (bar ? 0 : 32) });
/** Only the element marked data-shot shows (on a see-through background). */
const SHOT_ONLY = `html, body { background: transparent !important; }
      body * { visibility: hidden !important; }
      [data-shot], [data-shot] * { visibility: visible !important; }`;
/** A clip around the first element matching `selector`, with `pad` page pixels around it. */
const around = (selector, pad) => async (page) => {
  const b = await page.locator(selector).first().boundingBox();
  return { x: b.x - pad, y: b.y - pad, width: b.width + pad * 2, height: b.height + pad * 2 };
};

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
    columns: { found: { w: {}, hidden: ["link"] } },
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
  "links-sets": {
    ...main("/?showcase=sets"),
    async run(page) {
      await sidebar(page, "Links");
      // The complete split archive (first row) in the details panel.
      await home(page);
      await park(page, 700, 860);
      await settle(page);
    },
  },
  "links-duplicate": {
    ...main("/?showcase=sets"),
    async run(page) {
      await sidebar(page, "Links");
      // The main button (Add to queue, as downloads are running): Course materials.zip is
      // already in Downloads, so Pako asks first.
      await page.locator(".split .btn.primary").first().click();
      await page.getByRole("dialog").waitFor();
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await park(page, 2, 700);
      await settle(page, 1000);
      await page.getByRole("dialog").evaluate((el) => el.setAttribute("data-shot", ""));
    },
    // Only the question, on a see-through background (Links under it is left out).
    transparent: true,
    css: SHOT_ONLY,
    clip: around("[data-shot]", 24),
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
  // How sharing works, in three steps: tight crops of the window, about 4:3.
  "share-step-drop": {
    path: "/?showcase=share",
    viewport: SHARE_DROP,
    async run(page) {
      await sidebar(page, "Upload");
      await closePanel(page);
      // Three things picked, going to Google Drive, under one name.
      await page.getByRole("radio", { name: "Google Drive", exact: true }).click();
      await page.getByLabel("Name of this upload").fill("Wedding");
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await park(page, 2, SHARE_DROP.height - 1);
      await settle(page, 1200);
    },
    // From under the title bar to just below the panel, without the sidebar.
    async clip(page) {
      const b = await page.locator(".basket").boundingBox();
      return { x: 220, y: 40, width: SHARE_DROP.width - 220, height: b.y + b.height + 16 - 40 };
    },
  },
  "share-step-link": {
    path: "/?showcase=share",
    viewport: SHARE_LINK,
    columns: { send: { w: {}, hidden: ["link"] } },
    async run(page) {
      await sidebar(page, "Upload");
      // The upload just made: the basket is empty again, its link was copied.
      await page.getByRole("button", { name: "Clear", exact: true }).first().click();
      await settle(page, 600);
      await clickRow(page, "Wedding");
      await settle(page, 900);
      await page.locator(".panel").getByRole("button", { name: "Copy link" }).first().click();
      await park(page, 2, SHARE_LINK.height - 1);
      await settle(page, 900);
      // The details panel from its top: its title, link, QR code and "Deletes in 7 days".
      await page.evaluate(() => {
        for (const el of document.querySelectorAll(".panel, .panel *")) if (el.scrollTop) el.scrollTop = 0;
      });
      await settle(page, 400);
    },
    clip: pageClip(SHARE_LINK, false),
  },
  "share-step-open": {
    path: "/?showcase=share",
    viewport: SHARE_OPEN,
    columns: { found: { w: {}, hidden: ["link", "source"] } },
    async run(page) {
      await sidebar(page, "Links");
      await home(page);
      await park(page, 2, SHARE_OPEN.height - 1);
      await settle(page);
    },
    clip: pageClip(SHARE_OPEN),
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
  /** The tray window with a video downloading at the top (its quality isn't on the small rows). */
  "tray-video": {
    path: "/mini?showcase=video",
    viewport: MINI,
    transparent: true,
    async run(page) {
      await park(page, 0, 0);
      await settle(page);
    },
  },
  // ---------- 1.3.0: Private mode, Protect with a password, Important, Extract… ----------
  // Private mode in the preview is a pretend vault (src/lib/privateMock.ts, PIN 1234).
  "private-list": {
    ...main("/?showcase&private=open"),
    async run(page) {
      await sidebar(page, "Private");
      await settle(page, 900);
      // A finished file picked: its details panel on the right.
      await page.getByRole("option", { name: /^Tax return 2026\.pdf/ }).first().click();
      await park(page, 700, 860);
      await settle(page, 1200);
    },
  },
  "private-pad": {
    ...main("/?showcase&private=on"),
    async run(page) {
      await sidebar(page, "Private");
      await settle(page, 900);
      // Two of the four digits typed.
      await page.keyboard.type("12");
      await park(page, 2, 899);
      await settle(page, 700);
    },
  },
  "upload-protect": {
    path: "/?showcase=share",
    viewport: SHARE_DROP,
    async run(page) {
      await sidebar(page, "Upload");
      await closePanel(page);
      await page.getByLabel("Name of this upload").fill("Wedding");
      await page.getByRole("switch", { name: "Protect with a password" }).click();
      await page.getByLabel("Password for this upload").fill("wedding-2026");
      await page.getByLabel("Type the password again").fill("wedding-2026");
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await park(page, 2, SHARE_DROP.height - 1);
      await settle(page, 1200);
    },
    async clip(page) {
      const b = await page.locator(".basket").boundingBox();
      return { x: 220, y: 40, width: SHARE_DROP.width - 220, height: b.y + b.height + 16 - 40 };
    },
  },
  "important-card": {
    path: "/important",
    viewport: { width: 500, height: 260 },
    transparent: true,
    css: ".frame.preview { background: transparent !important; }",
    async run(page) {
      await park(page, 0, 0);
      await settle(page, 900);
    },
  },
  "extract-password": {
    ...main(),
    async run(page) {
      // A finished archive locked with a password: Extract… asks for it.
      await page.getByRole("row", { name: /^Course materials\.zip/ }).first().click({ button: "right" });
      await page.getByRole("menuitem", { name: "Extract…" }).click();
      await settle(page, 900);
      await page.getByRole("dialog").locator('input[type="password"]').fill("course2026");
      await park(page, 2, 899);
      await settle(page, 700);
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
  // ---------- new since 1.0.1 ----------
  "links-mirrors": {
    ...main("/?showcase=hosts"),
    async run(page) {
      await sidebar(page, "Links");
      // The file on three hosts in the details panel (Hosts), and its row's host list open.
      await clickRow(page, "Field recordings.zip");
      await settle(page, 700);
      await page.getByRole("button", { name: /^Pick the host/ }).first().click();
      await page.getByRole("menu").first().waitFor();
      await park(page, 700, 880);
      await settle(page, 1000);
    },
  },
  "download-fallback": {
    ...main("/?showcase=hosts"),
    async run(page) {
      // The download that went on from its next host: first in the list, in the details panel.
      await home(page);
      await park(page, 700, 880);
      await settle(page, 1800);
    },
    // Its row ("from …") and the details panel down to Backups and Tried, without the sidebar.
    async clip(page) {
      // Ends halfway between Tried and the next line (Saved to).
      const tried = await page.locator(".panel dt", { hasText: "Tried" }).first().boundingBox();
      const next = await page.locator(".panel dt", { hasText: "Saved to" }).first().boundingBox();
      return { x: 220, y: 40, width: MAIN.width - 220, height: Math.round((tried.y + tried.height + next.y) / 2) - 40 };
    },
  },
  "video-preview-site": {
    ...main(),
    async before(context) {
      // The short clip the site would send (a still sample picture, about 20 seconds).
      await context.route("http://127.0.0.1:1/**", (route) => serveClip(route));
    },
    async run(page) {
      await sidebar(page, "Links");
      await clickRow(page, "Sunrise over the lake 4K.mp4");
      await settle(page, 600);
      await page.getByRole("button", { name: "Play a preview" }).click();
      const video = page.locator(".preview video");
      await video.waitFor();
      // A few seconds in, paused, so the player shows its time and controls.
      await video.evaluate(async (v) => {
        const el = /** @type {HTMLVideoElement} */ (v);
        if (el.readyState < 2) await new Promise((r) => el.addEventListener("loadeddata", r, { once: true }));
        el.pause();
        el.currentTime = 6.4;
        await new Promise((r) => el.addEventListener("seeked", r, { once: true }));
      });
      await park(page, 700, 880);
      await settle(page, 1200);
    },
  },
  "direct-local": {
    ...main("/?showcase=local"),
    async run(page) {
      await sidebar(page, "Direct");
      await park(page, 700, 880);
      // The live speed is measured from two updates a second apart.
      await settle(page, 3500);
    },
  },
  "devices-files": {
    ...main(),
    async run(page) {
      await sidebar(page, "My devices");
      await clickRow(page, "Home PC");
      await park(page, 700, 880);
      await settle(page, 3500);
    },
  },
  "devices-downloads": {
    ...main(),
    async run(page) {
      await sidebar(page, "My devices");
      await clickRow(page, "Home PC");
      await settle(page, 600);
      await page.locator(".tabs").getByText("Downloads", { exact: true }).first().click();
      await park(page, 700, 880);
      await settle(page, 3000);
    },
  },
  "devices-links": {
    ...main(),
    async run(page) {
      await sidebar(page, "My devices");
      await clickRow(page, "Home PC");
      await settle(page, 600);
      await page.locator(".tabs").getByText("Links", { exact: true }).first().click();
      // A finished send followed live, and a playlist read here, its videos to pick.
      await page.getByText(/^Send \d+ to Home PC$/).first().waitFor();
      await park(page, 700, 880);
      await settle(page, 3000);
    },
  },
  "links-download-on": {
    ...main(),
    async run(page) {
      await sidebar(page, "Links");
      // The link set to Home PC in the details panel (its Download on), and the toolbar's
      // "Download on" menu open.
      await clickRow(page, "Mountain timelapse 4K.mkv");
      await settle(page, 700);
      await page.locator('[data-tour="found-download-on"]').click();
      await page.getByRole("menu").first().waitFor();
      await park(page, 700, 880);
      await settle(page, 1000);
    },
  },
  "update-ready": {
    path: "/popup?update=ready",
    viewport: { width: POPUP_W, height: 260 },
    transparent: true,
    css: ".frame.preview { background: transparent !important; }",
    async run(page) {
      await page.locator(".stack").waitFor();
      // Pointing at the card keeps it.
      await page.locator(".stack").hover({ position: { x: 4, y: 4 } });
      await settle(page, 800);
    },
    async clip(page) {
      const b = await page.locator(".stack").boundingBox();
      const p = 14;
      return { x: b.x - p, y: b.y - p, width: b.width + p * 2, height: b.height + p * 2 };
    },
  },
  // ---------- new in 1.2.0 ----------
  "browser-handoff": {
    // Downloads with a "Via browser" row, and the Download with Pako window just opened on top
    // for a file clicked in the browser (its own page, in a frame where Windows would show it).
    ...main("/?showcase=browser"),
    async run(page) {
      await closePanel(page);
      await page.evaluate(
        ({ w, h, theme }) => {
          const frame = document.createElement("iframe");
          frame.src = "/ask?showcase=browser";
          const edge = theme === "light" ? "1px solid #c9d0dc" : "1px solid #343943";
          const shadow = theme === "light" ? "0 18px 50px rgba(20,26,46,.22), 0 3px 10px rgba(20,26,46,.12)" : "0 18px 50px rgba(0,0,0,.6), 0 3px 10px rgba(0,0,0,.4)";
          Object.assign(frame.style, {
            position: "fixed", zIndex: "9999", width: w + "px", height: h + "px", left: Math.round(220 + (1440 - 220 - w) / 2) + "px", top: "330px",
            border: edge, borderRadius: "8px", boxShadow: shadow, background: "transparent", colorScheme: theme,
          });
          frame.setAttribute("data-ask", "");
          document.body.appendChild(frame);
        },
        { w: ASK.width, h: ASK.height, theme: THEME },
      );
      const ask = page.frameLocator("[data-ask]");
      await ask.getByText("Conference talk slides.pdf").waitFor();
      await page.frameLocator("[data-ask]").locator("body").evaluate(() => {
        const s = document.createElement("style");
        s.textContent = "*, *::before, *::after { transition-duration: 0s !important; caret-color: transparent !important; }";
        document.head.appendChild(s);
        /** @type {HTMLElement} */ (document.activeElement)?.blur();
      });
      await park(page, 700, 880);
      await settle(page, 1800);
    },
  },
  "via-browser": {
    ...main("/?showcase=browser"),
    async run(page) {
      // The file only the browser could get (first row) in the details panel, with its note.
      await home(page);
      await park(page, 700, 860);
      await settle(page, 1800);
    },
  },
  "direct-resume": {
    ...main(),
    async run(page) {
      await sidebar(page, "Direct");
      // Nina's laptop went to sleep: the transfer waits and goes on once she's back.
      await clickRow(page, "Nina");
      await park(page, 700, 880);
      await settle(page, 1800);
    },
  },
  "direct-from-pc": {
    ...main(),
    async run(page) {
      await sidebar(page, "Direct");
      // Sending from Home PC to Rado (the card: to Home PC done, on its way to Rado), and the
      // "sending from" menu open again: This PC, Home PC, Laptop, Office PC (offline).
      const pcButton = page.locator("button.fpc");
      await pcButton.click();
      await page.locator(".frommenu .fitem", { hasText: "Home PC" }).click();
      await settle(page, 900);
      await clickRow(page, "Rado");
      await settle(page, 900);
      await pcButton.click();
      await page.locator(".frommenu").waitFor();
      await park(page, 700, 880);
      await settle(page, 3500);
    },
  },
  "direct-from-home": {
    ...main(),
    async run(page) {
      await sidebar(page, "Direct");
      await page.locator("button.fpc").click();
      await page.locator(".frommenu .fitem", { hasText: "Home PC" }).click();
      await settle(page, 900);
      await clickRow(page, "Rado");
      await park(page, 700, 880);
      await settle(page, 3500);
    },
  },
  "upload-zip-parts": {
    ...main("/?showcase=parts"),
    async run(page) {
      await sidebar(page, "Upload");
      await closePanel(page);
      await page.getByRole("radio", { name: "ZIP in parts", exact: true }).click();
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await park(page, 700, 880);
      await settle(page);
    },
  },
  "settings-tabs": {
    ...main(),
    async run(page) {
      await page.locator(".sidebar, nav, aside").getByText("Settings", { exact: true }).first().click();
      await settle(page, 600);
      await page.getByRole("dialog").getByRole("tab", { name: "Downloads" }).click();
      await settle(page, 600);
      const fold = page.getByRole("dialog").locator("[data-fold-toggle]:visible").first();
      await fold.click();
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await settle(page, 600);
      // Down to the Advanced part, its title near the top, so what it holds shows.
      await fold.evaluate((el) => {
        let box = el.parentElement;
        while (box && !/(auto|scroll)/.test(getComputedStyle(box).overflowY)) box = box.parentElement;
        if (box) box.scrollTop += el.getBoundingClientRect().top - box.getBoundingClientRect().top - 120;
      });
      await park(page, 2, 700);
      await settle(page, 1200);
    },
  },
  "settings-search": {
    ...main(),
    async run(page) {
      await page.locator(".sidebar, nav, aside").getByText("Settings", { exact: true }).first().click();
      await settle(page, 600);
      // "start" typed, its first find picked: Pako goes to that setting and lights it up.
      await page.getByRole("dialog").getByLabel("Search settings").fill("start");
      await page.getByRole("dialog").locator(".hits .hit").first().waitFor();
      await settle(page, 400);
      await page.getByRole("dialog").locator(".hits .hit").first().click();
      await page.locator(".set-flash").first().evaluate((el) => el.setAttribute("data-lit", ""));
      // Kept lit (it fades after a moment in the window).
      await page.addStyleTag({ content: "[data-lit]::after { animation: none !important; opacity: 1 !important; }" });
      await page.evaluate(() => /** @type {HTMLElement} */ (document.activeElement)?.blur());
      await park(page, 2, 700);
      await settle(page, 2000);
      await page.locator("[data-lit]").evaluate((el) => el.classList.add("set-flash"));
      await settle(page, 300);
    },
  },
  "direct-check-contact": {
    ...main(),
    async run(page) {
      await sidebar(page, "Direct");
      // Jo: checked often for an hour (picked a quarter of an hour ago), and Mute under it.
      await clickRow(page, "Jo");
      await park(page, 700, 880);
      await settle(page, 1800);
    },
  },
  "pako-light-popup": { extension: "light" },
  "extension-popup": { extension: "full" },
  "extension-videos": { extension: "full", videos: true },
};

// ---------- the sample clip for video-preview-site ----------

/** A made-up "Sunrise over the lake" picture (no real footage, no logos). */
const CLIP_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="854" height="480" viewBox="0 0 854 480">
  <defs>
    <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#1d2a55"/><stop offset=".45" stop-color="#7a4f86"/><stop offset=".78" stop-color="#f08a5d"/><stop offset="1" stop-color="#ffd08a"/>
    </linearGradient>
    <linearGradient id="lake" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#f3a66e"/><stop offset=".35" stop-color="#8a5a7e"/><stop offset="1" stop-color="#1b2448"/>
    </linearGradient>
    <radialGradient id="glow" cx=".5" cy=".5" r=".5">
      <stop offset="0" stop-color="#fff3c4" stop-opacity=".95"/><stop offset=".35" stop-color="#ffd27a" stop-opacity=".55"/><stop offset="1" stop-color="#ffb36b" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="854" height="300" fill="url(#sky)"/>
  <circle cx="470" cy="292" r="150" fill="url(#glow)"/>
  <circle cx="470" cy="292" r="34" fill="#fff1c2"/>
  <path d="M0 250 L90 205 L170 240 L260 190 L350 236 L430 214 L520 250 L610 196 L700 232 L780 204 L854 236 L854 300 L0 300 Z" fill="#5a3f6e" opacity=".75"/>
  <path d="M0 272 L120 236 L210 262 L320 228 L420 270 L540 244 L640 268 L760 238 L854 262 L854 300 L0 300 Z" fill="#3a2d55"/>
  <rect y="300" width="854" height="180" fill="url(#lake)"/>
  <g fill="#fff1c2" opacity=".75">
    <rect x="440" y="306" width="60" height="3" rx="1.5"/><rect x="430" y="318" width="80" height="3" rx="1.5"/><rect x="446" y="332" width="48" height="2.5" rx="1.25"/>
    <rect x="424" y="346" width="92" height="2.5" rx="1.25" opacity=".7"/><rect x="452" y="362" width="36" height="2" rx="1" opacity=".6"/><rect x="436" y="380" width="68" height="2" rx="1" opacity=".45"/>
  </g>
  <path d="M0 430 Q120 410 250 428 T520 432 T854 420 L854 480 L0 480 Z" fill="#141a33"/>
</svg>`;

let clip = null;
/** Makes the sample clip once (VP9 WebM, 20 seconds), with ffmpeg. */
async function sampleClip(browser) {
  if (clip) return clip;
  const ff = [process.env.FFMPEG, join(process.env.LOCALAPPDATA ?? "", "Pako", "components", "ffmpeg", "ffmpeg.exe")].find((f) => f && existsSync(f)) ?? "ffmpeg";
  const dir = mkdtempSync(join(tmpdir(), "pako-clip-"));
  const png = join(dir, "frame.png");
  const out = join(dir, "clip.webm");
  const page = await browser.newPage({ viewport: { width: 854, height: 480 } });
  await page.setContent(`<style>html,body{margin:0}</style>${CLIP_SVG}`);
  await page.screenshot({ path: png });
  await page.close();
  execFileSync(ff, ["-v", "error", "-y", "-loop", "1", "-i", png, "-t", "20", "-r", "25", "-c:v", "libvpx-vp9", "-b:v", "600k", "-pix_fmt", "yuv420p", out]);
  clip = readFileSync(out);
  rmSync(dir, { recursive: true, force: true });
  return clip;
}
/** Serves the clip with byte ranges, as Pako's small preview server does. */
function serveClip(route) {
  const body = clip;
  const range = /bytes=(\d+)-(\d*)/.exec(route.request().headers()["range"] ?? "");
  if (!range) return route.fulfill({ status: 200, contentType: "video/webm", headers: { "Accept-Ranges": "bytes" }, body });
  const start = Number(range[1]);
  const end = range[2] ? Math.min(Number(range[2]), body.length - 1) : body.length - 1;
  return route.fulfill({
    status: 206,
    contentType: "video/webm",
    headers: { "Accept-Ranges": "bytes", "Content-Range": `bytes ${start}-${end}/${body.length}` },
    body: body.subarray(start, end + 1),
  });
}

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

/** Videos the full extension caught on a sample page (extension-videos), on neutral sample hosts. */
const CAUGHT = [
  { kind: "hls", height: 1080, host: "video.example.com", url: "https://video.example.com/hls/master.m3u8" },
  { kind: "dash", height: 720, host: "stream.example.net", url: "https://stream.example.net/dash/manifest.mpd" },
  { kind: "file", height: null, host: "media.example.org", url: "https://media.example.org/clips/trailer.mp4" },
];
/** Just enough of `chrome.*` for the popup to say Connected (and list `videos` on the page). */
/** The extension's own version: its popup checks the background script says the same one. */
const EXT_VERSION = JSON.parse(readFileSync(join(root, "extension", "manifest.chromium.json"), "utf8")).version;
const chromeStub = (videos) => `(() => {
  const store = { takeOver: true, playerButton: true, lastError: "", lastDetail: "" };
  const session = { lastError: "", lastDetail: "" };
  const caught = ${JSON.stringify(videos)};
  const version = ${JSON.stringify(EXT_VERSION)};
  window.chrome = {
    runtime: {
      sendMessage: async (m) => (m && m.type === "videos" ? { ok: true, list: caught, playing: caught.length > 0 } : { ok: true, script: version }),
      sendNativeMessage: (_h, _m, cb) => cb && cb({ ok: true }),
      getManifest: () => ({ version }),
      reload: () => {},
      lastError: undefined,
    },
    storage: {
      local: { get: async (d) => ({ ...d, ...store }), set: async (v) => Object.assign(store, v) },
      session: { get: async (d) => ({ ...d, ...session }), set: async (v) => Object.assign(session, v) },
    },
    action: { setBadgeText: () => {} },
    tabs: { query: async () => [{ id: 1, url: "https://example.com/" }], update: async () => {}, create: async () => {} },
  };
})();`;

const TYPES = { ".html": "text/html", ".css": "text/css", ".js": "text/javascript", ".png": "image/png", ".svg": "image/svg+xml" };

async function shootExtension(browser, name, edition, videos = false) {
  const { dir, cleanup } = extensionDir(edition);
  try {
    for (const { scale, dir: outDir } of SCALES) {
      const context = await browser.newContext({ viewport: { width: 360, height: 420 }, deviceScaleFactor: scale, colorScheme: THEME, reducedMotion: "reduce", timezoneId: "UTC" });
      await context.addInitScript(chromeStub(videos ? CAUGHT : []));
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
      if (videos) {
        await page.locator("#videos-list li").first().waitFor();
        // Only the videos and Send this page: the settings lines below stay out of this picture.
        await page.addStyleTag({ content: "#lines { display: none !important; }" });
      }
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
  if (shot.extension) return shootExtension(browser, name, shot.extension, !!shot.videos);
  if (shot.before) await sampleClip(browser);
  for (const { scale, dir } of SCALES) {
    const { context, page } = await open(browser, scale, shot.path, shot.viewport, shot.before, shot.columns);
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
const browser = await chromium.launch({ headless: true, args: ["--disable-notifications"] });
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
