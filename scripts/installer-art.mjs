// Pako Setup's pictures: the five feature slides on the left of the window, the logo, the
// switches, the folder box and the buttons, each in a dark and a light version (Setup follows
// Windows). Drawn once here with Playwright's Chromium at twice the size and saved in
// src-tauri/installer/art/; Setup scales them to the screen (installer/ui.nsh: PakoImage).
//
//   node scripts/installer-art.mjs
//
// Run it again after a screenshot in docs/screenshots changes, or the wording below. The slides
// are JPEG (small enough for Setup), the rest PNG. Every picture is drawn on its own background
// colour: Windows' picture controls don't blend, so nothing is transparent.
//
// [Pako] French (template 1.1.0): every slide also has `fr: { title, desc }`, drawn on the French
// screenshots (docs/screenshots/fr/, made by `node scripts/screenshots.mjs --fr [--light] <name>`),
// as slideN-<look>-fr.jpg ("1 sur 5"), and every button has its French words,
// b-<name>-<state>-<look>-fr.png. Setup shows them when Français is picked (installer/ui.nsh).
// The French is typed with its typography: a no-break space before : and inside « », and the
// curly apostrophe.
import { chromium } from "playwright";
import { mkdirSync, readFileSync, writeFileSync, statSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "src-tauri", "installer", "art");
const shots = join(root, "docs", "screenshots");
mkdirSync(out, { recursive: true });

// Must match installer/ui.nsh (colours, sizes, the slide's click areas).
const looks = {
  d: { bg: "#14171b", pane: "#0f1114", text: "#e9eaee", mut: "#8b919c", ln: "#262a31", off: "#343a44", ra: "#1b1f24", btn: "#20242a", btnLine: "#3a404a", btnHover: "#272c33", shadow: "rgba(0,0,0,.5)" },
  l: { bg: "#ffffff", pane: "#f6f7f9", text: "#17181c", mut: "#6b7280", ln: "#e2e7ee", off: "#cdd4de", ra: "#f3f5f8", btn: "#ffffff", btnLine: "#828b9b", btnHover: "#f1f3f6", shadow: "rgba(23,24,28,.18)" },
};
const ACCENT = "#f96263";
const ACCENT_HOVER = "#fa7576";
const font = `"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`;
const fontDisplay = `"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif`;

// Pictures go in as data: addresses (a page set from a string can't open files).
const dataUrl = (file) =>
  `data:${file.endsWith(".svg") ? "image/svg+xml" : "image/png"};base64,${readFileSync(file).toString("base64")}`;
const asUrl = (f) => dataUrl(join(shots, f));
const shot = (name, look, lang) => asUrl(`${lang === "fr" ? "fr/" : ""}${name}${look === "l" ? "-light" : ""}.png`);
const svgShot = (name, look, lang) =>
  dataUrl(join(shots, ...(lang === "fr" ? ["fr"] : []), "src", `${name}${look === "l" ? "-light" : ""}.svg`));

// The slides: wording approved by the user (2026-11-18). fr: the French set, with the window's and
// the extension's French words (Envoi, Télécharger), as the French screenshots show them.
const slides = [
  {
    title: "Downloads go straight to Pako",
    desc: "Click a download in your browser: Pako takes it at once, before any Save as, and downloads it faster.",
    fr: {
      title: "Les téléchargements vont droit dans Pako",
      desc: "Cliquez sur un téléchargement dans votre navigateur : Pako le prend aussitôt, avant tout « Enregistrer sous », et le télécharge plus vite.",
    },
    main: (k, l) => ({ src: shot("browser-handoff", k, l) }),
  },
  {
    title: "Keep the videos you watch",
    desc: "On any page with a video, click Download and pick the quality, or audio only. Pako saves it to your PC, playlists too.",
    fr: {
      title: "Gardez les vidéos que vous regardez",
      desc: "Sur toute page avec une vidéo, cliquez sur Télécharger et choisissez la qualité, ou l’audio seul. Pako l’enregistre sur votre PC, playlists comprises.",
    },
    main: (k, l) => ({ src: svgShot("pill-on-video", k, l), fit: "contain" }),
    card: (k, l) => ({ src: shot("tray-video", k, l), side: "left", width: 38 }),
  },
  {
    title: "Share files with a link",
    desc: "Drop files in Upload: Pako zips them, uploads them and copies the link. Pick when they're deleted.",
    fr: {
      title: "Partagez des fichiers avec un lien",
      desc: "Déposez des fichiers dans Envoi : Pako les compresse, les met en ligne et copie le lien. Choisissez quand ils sont supprimés.",
    },
    main: (k, l) => ({ src: shot("share-step-link", k, l) }),
  },
  {
    title: "Send files PC to PC with Direct",
    desc: "Straight to a friend's Pako, encrypted, any size. A transfer cut off picks up where it stopped.",
    fr: {
      title: "Envoyez des fichiers de PC à PC avec Direct",
      desc: "Directement dans le Pako d’un ami, chiffrés, quelle que soit leur taille. Un transfert coupé reprend là où il s’était arrêté.",
    },
    main: (k, l) => ({ src: shot("direct", k, l) }),
  },
  {
    title: "Your PCs work together",
    desc: "Send files between your own PCs, and from your laptop make your home PC download for you. Follow it all live.",
    fr: {
      title: "Vos PC travaillent ensemble",
      desc: "Envoyez des fichiers entre vos PC et, depuis votre portable, faites télécharger votre PC de la maison. Suivez tout en direct.",
    },
    main: (k, l) => ({ src: shot("devices-files", k, l) }),
    card: (k, l) => ({ src: shot("devices-downloads", k, l), side: "right", width: 56 }),
  },
];

// The left panel: 530 × 540. Dots start at x 28 (7 wide, the current one 20, 6 apart), centre
// y 514; the arrows are 26 × 26 boxes at x 448 and 476, y 501.
function slideHtml(i, k, lang = "en") {
  const c = looks[k];
  const s = slides[i];
  const words = lang === "fr" ? s.fr : s;
  const m = s.main(k, lang);
  const card = s.card?.(k, lang);
  const dots = slides
    .map((_, j) => `<span style="width:${j === i ? 20 : 7}px;background:${j === i ? ACCENT : c.off}"></span>`)
    .join("");
  const chevron = (d) =>
    `<svg width="26" height="26" viewBox="0 0 26 26"><path d="${d}" fill="none" stroke="${c.mut}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  return `<!doctype html><html lang="${lang}"><head><meta charset="utf-8"><style>
    html,body{margin:0;width:530px;height:540px;overflow:hidden;background:${c.pane};font-family:${font};-webkit-font-smoothing:antialiased}
    .k{position:absolute;left:28px;top:28px;font-size:12px;color:${ACCENT};letter-spacing:.02em;margin:0}
    h2{position:absolute;left:28px;top:46px;right:28px;margin:0;font:600 20px/26px ${fontDisplay};color:${c.text}}
    .d{position:absolute;left:28px;top:78px;right:28px;margin:0;font-size:13px;line-height:20px;color:${c.mut}}
    .shot{position:absolute;left:28px;right:28px;top:130px;bottom:52px;border-radius:10px;overflow:hidden;border:1px solid ${c.ln};background:${c.bg}}
    .shot>img.main{width:100%;height:100%;display:block;object-fit:${m.fit ?? "cover"};object-position:${m.fit ? "center" : "top left"}}
    .card{position:absolute;bottom:14px;${card?.side === "right" ? "right" : "left"}:14px;width:${card?.width ?? 40}%;border-radius:8px;border:1px solid ${c.btnLine};box-shadow:0 10px 26px ${c.shadow};display:block}
    .dots{position:absolute;left:28px;top:511px;display:flex;gap:6px}
    .dots span{height:7px;border-radius:4px;display:block}
    .nav{position:absolute;top:501px;left:448px;display:flex;gap:2px}
  </style></head><body>
    <p class="k">${i + 1} ${lang === "fr" ? "sur" : "of"} ${slides.length}</p>
    <h2>${words.title}</h2>
    <p class="d">${words.desc}</p>
    <div class="shot"><img class="main" src="${m.src}">${card ? `<img class="card" src="${card.src}">` : ""}</div>
    <div class="dots">${dots}</div>
    <div class="nav">${chevron("M15 8 L10 13 L15 18")}${chevron("M11 8 L16 13 L11 18")}</div>
  </body></html>`;
}

// The mark and wordmark, from src/lib/Logo.svelte (box 44 high, word 28 high, 12 apart).
function logoHtml(k) {
  const c = looks[k];
  const logo = readFileSync(join(root, "src", "lib", "Logo.svelte"), "utf8");
  const svgs = [...logo.matchAll(/<svg[\s\S]*?<\/svg>/g)].map((m) => m[0]);
  const fix = (s, h) =>
    s
      .replace(/\{uid\}/g, "p")
      .replace(/var\(--accent\)/g, ACCENT)
      .replace(/currentColor/g, c.text)
      .replace(/width=\{size\}/, `width="${h}"`)
      .replace(/height=\{size\}/, `height="${h}"`)
      .replace(/height=\{wordH\}/, `height="${h}"`)
      .replace(/\s(class|aria-[a-z]+|role)="[^"]*"/g, "")
      .replace(/\{[^}]*\}/g, "");
  return `<!doctype html><html><head><style>html,body{margin:0;background:${c.bg}}
    div{display:flex;align-items:center;gap:12px;width:160px;height:44px;justify-content:center}
    svg.w{width:89px}</style></head><body><div>${fix(svgs[0], 44)}${fix(svgs[1], 28).replace("<svg", '<svg class="w"')}</div></body></html>`;
}

const sw = (k, on) => {
  const c = looks[k];
  return `<!doctype html><html><head><style>html,body{margin:0;background:${c.bg}}
    i{display:block;position:relative;width:32px;height:18px;border-radius:9px;background:${on ? ACCENT : c.off}}
    i:after{content:"";position:absolute;top:2px;${on ? "right" : "left"}:2px;width:14px;height:14px;border-radius:50%;background:#fff;box-shadow:0 1px 2px rgba(0,0,0,.2)}
  </style></head><body><i></i></body></html>`;
};

const field = (k) => {
  const c = looks[k];
  return `<!doctype html><html><head><style>html,body{margin:0;background:${c.bg}}
    div{width:282px;height:32px;border-radius:7px;background:${c.ra};display:flex;align-items:center;padding-left:10px;box-sizing:border-box}
  </style></head><body><div><svg width="15" height="15" viewBox="0 0 16 16"><path d="M1.5 4.5 V12 a1 1 0 0 0 1 1 H13.5 a1 1 0 0 0 1-1 V6 a1 1 0 0 0-1-1 H8 L6.5 3 H2.5 a1 1 0 0 0-1 1 Z" fill="none" stroke="${c.mut}" stroke-width="1.3" stroke-linejoin="round"/></svg></div></body></html>`;
};

// kind: "main" (coral) or "second"; state: n(ormal), h(over), x (can't be clicked yet).
const button = (k, label, width, kind, state) => {
  const c = looks[k];
  const main = kind === "main";
  const bg = main ? (state === "h" ? ACCENT_HOVER : ACCENT) : state === "h" ? c.btnHover : c.btn;
  const edge = main ? bg : c.btnLine;
  const color = main ? "#ffffff" : c.text;
  return `<!doctype html><html><head><style>html,body{margin:0;background:${c.bg}}
    div{width:${width}px;height:40px;box-sizing:border-box;border-radius:8px;background:${bg};border:1px solid ${edge};color:${color};
      display:flex;align-items:center;justify-content:center;font:600 14px ${font};opacity:${state === "x" ? 0.45 : 1}}
  </style></head><body><div>${label}</div></body></html>`;
};

// name, label, width (logical px), kind. The installer's buttons are one wide button; the
// uninstaller's two sit side by side.
const buttons = [
  ["install", "Install", 282, "main"],
  ["update", "Update", 282, "main"],
  ["again", "Install again", 282, "main"],
  ["older", "Install this version", 282, "main"],
  ["open", "Open Pako", 282, "main"],
  ["done", "Close", 282, "second"],
  ["uninstall", "Uninstall", 200, "main"],
  ["cancel", "Cancel", 200, "second"],
  ["close", "Close", 412, "second"],
];
// [Pako] Their French words (installer/French.nsh; the pictures end in -fr).
const buttonsFr = {
  install: "Installer",
  update: "Mettre à jour",
  again: "Réinstaller",
  older: "Installer cette version",
  open: "Ouvrir Pako",
  done: "Fermer",
  uninstall: "Désinstaller",
  cancel: "Annuler",
  close: "Fermer",
};

const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: 2 });
const written = [];
async function render(name, html, w, h, type = "png") {
  await page.setViewportSize({ width: w, height: h });
  await page.setContent(html, { waitUntil: "load" });
  await page.evaluate(() => document.fonts.ready);
  const file = join(out, `${name}.${type === "jpeg" ? "jpg" : "png"}`);
  await page.screenshot({ path: file, type, ...(type === "jpeg" ? { quality: 86 } : {}), clip: { x: 0, y: 0, width: w, height: h } });
  written.push(file);
}

for (const k of ["d", "l"]) {
  for (let i = 0; i < slides.length; i++) await render(`slide${i + 1}-${k}`, slideHtml(i, k), 530, 540, "jpeg");
  for (let i = 0; i < slides.length; i++) await render(`slide${i + 1}-${k}-fr`, slideHtml(i, k, "fr"), 530, 540, "jpeg");
  await render(`logo-${k}`, logoHtml(k), 160, 44);
  await render(`on-${k}`, sw(k, true), 32, 18);
  await render(`off-${k}`, sw(k, false), 32, 18);
  await render(`field-${k}`, field(k), 282, 32);
  for (const [name, label, width, kind] of buttons) {
    for (const st of kind === "main" ? ["n", "h", "x"] : ["n", "h"]) {
      await render(`b-${name}-${st}-${k}`, button(k, label, width, kind, st), width, 40);
      await render(`b-${name}-${st}-${k}-fr`, button(k, buttonsFr[name], width, kind, st), width, 40);
    }
  }
}
await browser.close();

// Old pictures this script no longer makes would still be built into Setup.
const keep = new Set([...written.map((f) => f.split(/[\\/]/).pop()), "README.md"]);
for (const f of readdirSync(out)) if (!keep.has(f)) console.warn(`not made any more (delete it): ${f}`);
const total = written.reduce((n, f) => n + statSync(f).size, 0);
console.log(`${written.length} pictures, ${(total / 1024).toFixed(0)} KB in ${out}`);
writeFileSync(join(out, "README.md"), "Made by `node scripts/installer-art.mjs`: don't edit these by hand.\n");
