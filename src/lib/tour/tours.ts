// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The tutorials, one per page, plus the quick tour. Steps point at `data-tour="…"` attributes.
// When a page changes, change its steps in the same commit.
// Keep them short: group what sits together (one step for a whole bar), and order the steps so
// the spotlight travels around the window in one direction instead of jumping back and forth.
import { app } from "../state.svelte";
import type { Step } from "./tour.svelte";
import { devices, targets } from "../devices.svelte";
import { priv } from "../private.svelte";

export type TourId = "downloads" | "found" | "send" | "direct" | "devices" | "dashboard" | "settings" | "private" | "full";

// Steps that don't always apply say when they do (`when`), from what Pako knows, so the count is
// right from the first bubble whatever page the tutorial starts from.
/** Direct is running and has people to show (yours, or the tutorial's examples). */
const hasContacts = () => !!app.directView?.ready && app.directContacts.length > 0;
/** Uploads in the Uploaded list (the ones still uploading are listed above it, not in it). */
const hasUploaded = () => app.sent.some((s) => !app.sends.some((j) => j.id === s.id));

// Centre → left → top → right → bottom.
const downloads: Step[] = [
  { view: "downloads", target: "dl-list", side: "inside", title: "Your downloads", text: "Everything Pako is downloading or has downloaded, with its progress, speed and status. Click a column title to sort, drag its edge to resize it, and right-click the titles to show or hide columns. Right-click a download for more. The bell at the end of a row says how it tells you when it's done: Pop-up, Important (a big pop-up in the middle of the screen) or Off; click it to change. Drag a finished download out of the list to drop it in a folder, an email or another app." },
  { view: "downloads", target: "filters", title: "Show and Sources", text: "Show only what's downloading, waiting, finished or has a problem. Sources shows one kind of link, like Google Drive." },
  { view: "downloads", target: "private-entry", when: () => priv.status.on, optional: true, title: "Private", text: "Downloads from your browser's private windows, encrypted and behind your PIN or password. Click Private (or press Ctrl+Shift+P) to open it." },
  { view: "downloads", target: ["private-back", "private-lock"], when: () => priv.status.unlocked, optional: true, title: "Private mode is open", text: "Private downloads goes back to your private list, Lock locks it. Going to another page, a filter on the left, Settings or Help locks it too." },
  { view: "downloads", target: ["search", "dl-actions", "dl-add"], title: "Search, pause, add", text: "Ctrl+F finds a download by name. Resume and Pause work on the selected downloads, or on all of them when none is selected; Remove needs a selection. + adds links, and so does Ctrl+V anywhere in Pako." },
  { view: "downloads", target: "details", panel: true, title: "Details", text: "Select a download to see its parts filling in, where it's saved, and what to do if something went wrong. Ctrl+I shows or hides this panel; drag its left edge to make it wider or narrower (each page remembers its width)." },
  { view: "downloads", target: ["when-done", "at-once"], title: "When done and At once", text: "When done lets the PC sleep or shut down once everything has finished, after a short countdown you can cancel. At once sets how many downloads run together." },
];

// Centre → left → top → right.
const found: Step[] = [
  { view: "found", target: "found-list", side: "inside", title: "Your links", text: "Links you paste land here first, and Pako checks each one: its name, its size and whether it works. Tick the ones you want. Ctrl+A ticks everything shown, Esc unticks. The same file on several hosts is one row: \"on 3 hosts\" picks the host, the others are backups. Drag a column title's edge to resize it, and right-click the titles to show or hide columns." },
  { view: "found", target: "filters", title: "Show", text: "Only the links that are ready, broken, still being checked, or that only work in the browser." },
  { view: "found", target: ["found-add", "found-dest", "found-download"], title: "Add, Save to, Download", text: "+ pastes more links. The folder picks where they're saved. Download starts the ticked links; when the running ones leave no room it says Add to queue, with Download first next to it. Off, Pop-up and Important (beside it, or in its arrow) say how they tell you when they're done; the bell at the end of a row does the same for one link. With Private mode on, the arrow also offers Download privately: straight into Private mode, encrypted." },
  { view: "found", target: "found-download-on", when: () => targets.list.length > 0, optional: true, title: "Download on", text: "Download on lets one of your other PCs download the ticked links instead, this time only (it's shown while a device that lets this PC control it is online). Their quality and priority go with them." },
  { view: "found", target: "details", panel: true, title: "Details", text: "Select a link to see what Pako found out about it, and give it its own folder if you like. Download on sends it to another of your PCs each time you press Download (a PC mark shows on its row)." },
];

// Top → centre → down → right.
const send: Step[] = [
  { view: "send", target: "send-choose", title: "Upload", text: "Share big files with a link anyone can open. Drag and drop files or folders anywhere on this page, from Explorer or your desktop, or pick them with this button." },
  { view: "send", target: "send-basket", when: () => app.basket.length > 0, optional: true, title: "New upload", text: "Files gather here first: nothing is uploaded until you click Upload. Choose where they go (Gofile needs no account, Google Drive uses your own), when they delete themselves, and how several files are packed. Protect with a password locks them before they leave your PC: people open them with 7-Zip or Pako and that password." },
  { view: "send", target: "send-uploaded", when: hasUploaded, optional: true, title: "Uploaded", text: "Each upload with where it went, when it deletes itself and how often it was downloaded. Copy its link from here. Drag a column title's edge to resize it, and right-click the titles to show or hide columns." },
  { view: "send", target: "details", panel: true, title: "Details", text: "Select an upload to see its files, its link as a QR code and how fast it went." },
];

// Left → right. With no contacts yet, example people are shown during this tutorial.
const direct: Step[] = [
  { view: "direct", target: "filters", title: "Direct", text: "Send files straight to another Pako, PC to PC, nothing uploaded in between. On the left: show who's online, waiting for you or a favourite, and group people with tags like Work. Right-click a tag to rename or delete it." },
  { view: "direct", target: "direct-people", when: hasContacts, optional: true, title: "Your contacts", text: "Everyone you've connected with. A green dot means online, a blue one online on your home network, an amber one that they haven't accepted yet, and a red ! that something waits for your answer. + Add adds someone with their Pako ID. At the top, under You, sending from: the name your contacts see (click it to change it) and your Pako ID to share. Its PC part sends from another of your PCs: pick one to see its contacts and send to them through it. At the bottom: whether Pakos on the same network can see you." },
  { view: "direct", target: "direct-chat", when: hasContacts, optional: true, title: "Your history together", text: "Everything you sent each other, with progress and speed. To send, drag and drop files or folders anywhere on this page (from Explorer or your desktop), or use Choose files. They go straight to this person, encrypted." },
  { view: "direct", target: "direct-details", panel: true, when: () => hasContacts() && app.directShown.length > 0, optional: true, title: "Details", text: "Your speeds with this person, what you exchanged, their tags and where their files go. Check this contact says how often Pako calls them by itself, or often for an hour, today or a week; Mute stops their pop-ups and sounds. Turn on Can send to me without asking for people you trust." },
];

// Left → right. Steps about a device show once one is paired (or asks to be).
const hasDevices = () => (devices.data?.devices.length ?? 0) > 0;
const devicesSteps: Step[] = [
  { view: "devices", target: "nav-devices", title: "My devices", text: "Your own PCs, paired once. Files between them go without anyone accepting, and you can see and start downloads on the other PC. Direct stays for other people." },
  { view: "devices", target: "devices-welcome", when: () => !hasDevices(), optional: true, title: "Pair a PC", text: "Add a device shows this PC's Pako ID. Enter it on your other PC (or that PC's ID here), pick where its files are saved, and confirm on both PCs with the same safety code." },
  { view: "devices", target: "devices-list", when: hasDevices, optional: true, title: "Your devices", text: "A green dot means online, a blue one online on your home network (the fastest, no internet used), grey offline. Drop files on a device to send them to it. + Add pairs another PC. Under This PC, click its name to rename it: your other PCs see the new name." },
  { view: "devices", target: "devices-pane", when: hasDevices, optional: true, title: "Files, Links, Downloads, Settings", text: "Files: everything sent between the two PCs, with progress and speed. Links: paste links for that PC to download, with their quality and priority (a playlist opens here first, to pick its videos), and follow each send. Downloads: the other PC's downloads live, pause, resume, change their priority or remove them, and Get this file. Settings: where its files are saved (only there), whether it can control this Pako, and its history." },
];

// Top → down the page.
const dashboard: Step[] = [
  { view: "dashboard", target: ["dash-period", "dash-more"], title: "Dashboard", text: "Your Pako in numbers. Pick Today, This week, This month or All time: the whole page follows it. The gear at the top right saves your numbers as a file for Excel, or starts them over." },
  { view: "dashboard", target: "dash-empty", when: () => app.numbersEmpty !== false, optional: true, title: "Nothing yet", text: "Your numbers show up here as you download, upload and send." },
  { view: "dashboard", target: ["dash-totals", "dash-activity"], when: () => app.numbersEmpty !== true, optional: true, title: "Totals and Activity", text: "What you downloaded, uploaded, sent and received in that period, and how it spread over the days. Point at a bar for its numbers." },
  { view: "dashboard", target: ["dash-speed", "dash-routes"], when: () => app.numbersEmpty !== true, optional: true, title: "Speed and routes", text: "Your average and top speeds, and which way is fastest for you: Gofile, Google Drive or Direct." },
  { view: "dashboard", target: ["dash-people", "dash-records"], when: () => app.numbersEmpty !== true, optional: true, title: "Your people and Records", text: "Who you exchanged the most with in Direct and your best speeds with them, then your records: fastest download, biggest file and more." },
];

// Private mode, open: the list, then its buttons and the details on the right, then the header's
// Normal list and Lock at the top (left → right, then up).
const privateSteps: Step[] = [
  { view: "downloads", target: "private-list", side: "inside", title: "Private downloads", text: "Downloads from your browser's private windows, links you sent with Download privately, and files you moved here. Their files are encrypted on this PC: they only open from here. A folder download is one folder row. Double-click a file: a video plays in Pako, anything else opens from a temporary copy that Pako deletes afterwards. Drag a column's edge to make it wider or narrower, and right-click the column titles to show or hide columns." },
  { view: "downloads", target: "private-actions", optional: true, when: () => priv.list.length > 0, title: "Open, Send, Decrypt, Delete", text: "On each row: play or open it, send it with Direct or Upload (read decrypted as it goes, no readable copy), Decrypt it into a readable file where you pick (then take it off the private list, move it to Downloads, or leave it here too), or delete it for good. Tick the boxes (or Ctrl-click, Shift-click, Ctrl+A) to Decrypt or Delete several at once, or a whole folder." },
  { view: "downloads", target: "private-details", optional: true, when: () => priv.list.length > 0, title: "Details", text: "The picked download: its progress, where it came from and when it was downloaded; while it downloads, its priority and how it tells you it's done (Off, Pop-up or Important). Its buttons are at the bottom: Open or Play, Decrypt…, Send…, Upload…, Delete…. Ctrl+I hides or shows it." },
  { view: "downloads", target: ["private-normal", "private-lock"], title: "Normal list and Lock", text: "Normal list shows your other downloads and keeps Private mode open (Private downloads, in the same place, brings you back). Lock locks it now (Ctrl+Shift+P too). Going anywhere else locks it too: another page, a filter on the left, Settings or Help. It also locks by itself when you step away, lock Windows, minimise or close Pako. Sending, uploading and Decrypt… show their progress right here." },
];

// The tabs on the left first, then each tab in their order (each step opens its tab).
const settings: Step[] = [
  { view: "settings", target: "settings-nav", title: "Settings", text: "One tab per part of Settings, each in small titled groups: click a tab, or use ↑ ↓ and Enter (Ctrl+Tab from anywhere in Settings). Search finds any setting or group by name, even under Advanced. Save keeps the changes in every tab at once; a dot marks a tab with changes not saved yet." },
  { view: "settings", target: "set-updates", title: "Features & updates", text: "Pako's version and its updates. The tools for features like videos fold under What Pako can do. At the bottom of each tab, Advanced holds what most people never change: here, running the first-start setup again." },
  { view: "settings", target: "set-browser", title: "Browser extension", text: "Connect Pako to your browsers, add the extension (each browser says when it's Installed), and choose whether Pako asks before each browser download." },
  { view: "settings", target: "set-accounts", title: "Accounts", text: "Connect your Google Drive for uploads. Downloads never need a sign-in." },
  { view: "settings", target: "set-window", title: "Window", text: "Light or dark, what minimising and closing do, starting with Windows, and asking before quitting." },
  { view: "settings", target: "set-notices", title: "Notifications", text: "How new downloads tell you they're done (Off, Pop-up or Important) and when they chime, what shows as a pop-up and what the bell keeps, the sounds, and Do not disturb." },
  { view: "settings", target: "set-downloads", title: "Downloads", text: "Speed (how many at once, a speed limit, set hours), saving files (your download folder, split archives), videos, what happens while downloading, and downloads from private windows. Connections per file and other rarely changed ones are under Advanced." },
  { view: "settings", target: "set-private", title: "Private mode", text: "Turn on Private mode to keep downloads from private windows behind a PIN or a password, then choose when it locks, when its list clears, and manage the PIN and recovery code." },
  { view: "settings", target: "set-upload", title: "Upload", text: "Where uploads go and how several files are packed." },
];

// The first look: short, down the sidebar. The details are in each page's Tutorial.
const full: Step[] = [
  { target: "tutorial", title: "Welcome to Pako", text: "A one-minute look around. Every page has a Tutorial button like this one for its details. Use → and ←, and Esc stops." },
  { target: ["nav-downloads", "nav-found"], title: "Downloads and Links", text: "Paste links anywhere with Ctrl+V. Pako checks them in Links, you pick what you want, and they download in Downloads." },
  { target: ["nav-send", "nav-direct"], title: "Upload and Direct", text: "Upload shares big files with a link anyone can open. Direct sends files straight to another Pako, PC to PC." },
  { target: "nav-devices", title: "My devices", text: "Your own PCs, paired once: send files between them without asking, and see or start downloads on the other one." },
  { target: "nav-dashboard", title: "Dashboard", text: "Your Pako in numbers: what you moved, how fast, and with whom." },
  // Only once Private mode is on (Settings › Private mode): it says nothing about it otherwise.
  { view: "downloads", target: "private-entry", when: () => priv.status.on, optional: true, title: "Private", text: "Downloads kept behind your PIN or password, encrypted on this PC. Click Private or press Ctrl+Shift+P." },
  { target: ["help", "settings"], title: "Help and Settings", text: "Guides, What's new, every tutorial and Report a problem are in Help (F1). Settings has your download folder, the browser extension and more." },
  { title: "Next to the clock", text: "Pako also lives in the tray: one click on its icon shows today's downloads, uploads and Direct transfers in a small window. Pin it to keep it on top." },
];

export const TOURS: Record<TourId, Step[]> = { downloads, found, send, direct, devices: devicesSteps, dashboard, settings, private: privateSteps, full };

/** Asked before a tutorial starts, for its steps' `when` (the Dashboard: anything counted yet?). */
export const PREPARE: Partial<Record<TourId, () => Promise<unknown>>> = {
  dashboard: () => app.checkNumbers(),
  // Started while the private list shows: back to the normal list, whose parts the steps show.
  downloads: async () => {
    priv.showing = false;
  },
};

/** The pages with a tutorial, in sidebar order (for Help). */
export const PAGE_TOURS: { id: Exclude<TourId, "full">; label: string }[] = [
  { id: "downloads", label: "Downloads" },
  { id: "found", label: "Links" },
  { id: "send", label: "Upload" },
  { id: "direct", label: "Direct" },
  { id: "devices", label: "My devices" },
  { id: "dashboard", label: "Dashboard" },
  { id: "settings", label: "Settings" },
];
