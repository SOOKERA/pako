// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// The tutorials, one per page, plus the quick tour. Steps point at `data-tour="…"` attributes.
// When a page changes, change its steps in the same commit.
// Keep them short: group what sits together (one step for a whole bar), and order the steps so
// the spotlight travels around the window in one direction instead of jumping back and forth.
import { app } from "../state.svelte";
import type { Step } from "./tour.svelte";

export type TourId = "downloads" | "found" | "send" | "direct" | "dashboard" | "settings" | "full";

// Steps that don't always apply say when they do (`when`), from what Pako knows, so the count is
// right from the first bubble whatever page the tutorial starts from.
/** Direct is running and has people to show (yours, or the tutorial's examples). */
const hasContacts = () => !!app.directView?.ready && app.directContacts.length > 0;
/** Uploads in the Uploaded list (the ones still uploading are listed above it, not in it). */
const hasUploaded = () => app.sent.some((s) => !app.sends.some((j) => j.id === s.id));

// Centre → left → top → right → bottom.
const downloads: Step[] = [
  { view: "downloads", target: "dl-list", side: "inside", title: "Your downloads", text: "Everything Pako is downloading or has downloaded, with its progress, speed and status. Click a column title to sort, and right-click a download for more. Drag a finished download out of the list to drop it in a folder, an email or another app." },
  { view: "downloads", target: "filters", title: "Show and Sources", text: "Show only what's downloading, waiting, finished or has a problem. Sources shows one kind of link, like Google Drive." },
  { view: "downloads", target: ["search", "dl-actions", "dl-add"], title: "Search, pause, add", text: "Ctrl+F finds a download by name. Resume and Pause work on the selected downloads, or on all of them when none is selected; Remove needs a selection. + adds links, and so does Ctrl+V anywhere in Pako." },
  { view: "downloads", target: "details", panel: true, title: "Details", text: "Select a download to see its parts filling in, where it's saved, and what to do if something went wrong. Ctrl+I shows or hides this panel." },
  { view: "downloads", target: ["when-done", "at-once"], title: "When done and At once", text: "When done lets the PC sleep or shut down once everything has finished, after a short countdown you can cancel. At once sets how many downloads run together." },
];

// Centre → left → top → right.
const found: Step[] = [
  { view: "found", target: "found-list", side: "inside", title: "Your links", text: "Links you paste land here first, and Pako checks each one: its name, its size and whether it works. Tick the ones you want. Ctrl+A ticks everything shown, Esc unticks." },
  { view: "found", target: "filters", title: "Show", text: "Only the links that are ready, broken, still being checked, or that only work in the browser." },
  { view: "found", target: ["found-add", "found-dest", "found-download"], title: "Add, Save to, Download", text: "+ pastes more links. The folder picks where they're saved. Download starts the ticked links; when the running ones leave no room it says Add to queue, with Download first next to it." },
  { view: "found", target: "details", panel: true, title: "Details", text: "Select a link to see what Pako found out about it, and give it its own folder if you like." },
];

// Top → centre → down → right.
const send: Step[] = [
  { view: "send", target: "send-choose", title: "Upload", text: "Share big files with a link anyone can open. Drag and drop files or folders anywhere on this page, from Explorer or your desktop, or pick them with this button." },
  { view: "send", target: "send-basket", when: () => app.basket.length > 0, optional: true, title: "New upload", text: "Files gather here first: nothing is uploaded until you click Upload. Choose where they go (Gofile needs no account, Google Drive uses your own), when they delete themselves, and how several files are packed." },
  { view: "send", target: "send-uploaded", when: hasUploaded, optional: true, title: "Uploaded", text: "Each upload with where it went, when it deletes itself and how often it was downloaded. Copy its link from here." },
  { view: "send", target: "details", panel: true, title: "Details", text: "Select an upload to see its files, its link as a QR code and how fast it went." },
];

// Left → right. With no contacts yet, example people are shown during this tutorial.
const direct: Step[] = [
  { view: "direct", target: "filters", title: "Direct", text: "Send files straight to another Pako, PC to PC, nothing uploaded in between. On the left: show who's online, waiting for you or a favourite, and group people with tags like Work. Right-click a tag to rename or delete it." },
  { view: "direct", target: "direct-people", when: hasContacts, optional: true, title: "Your contacts", text: "Everyone you've connected with. A green dot means online, an amber one that they haven't accepted yet, and a red ! that something waits for your answer. + Add adds someone with their Pako ID. At the bottom: the name your contacts see, and your Pako ID to share." },
  { view: "direct", target: "direct-chat", when: hasContacts, optional: true, title: "Your history together", text: "Everything you sent each other, with progress and speed. To send, drag and drop files or folders anywhere on this page (from Explorer or your desktop), or use Choose files. They go straight to this person, encrypted." },
  { view: "direct", target: "direct-details", panel: true, when: () => hasContacts() && app.directShown.length > 0, optional: true, title: "Details", text: "Your speeds with this person, what you exchanged, their tags and where their files go. Turn on Can send to me without asking for people you trust." },
];

// Top → down the page.
const dashboard: Step[] = [
  { view: "dashboard", target: ["dash-period", "dash-more"], title: "Dashboard", text: "Your Pako in numbers. Pick Today, This week, This month or All time: the whole page follows it. The … button saves your numbers as a file for Excel, or starts them over." },
  { view: "dashboard", target: "dash-empty", when: () => app.numbersEmpty !== false, optional: true, title: "Nothing yet", text: "Your numbers show up here as you download, upload and send." },
  { view: "dashboard", target: ["dash-totals", "dash-activity"], when: () => app.numbersEmpty !== true, optional: true, title: "Totals and Activity", text: "What you downloaded, uploaded, sent and received in that period, and how it spread over the days. Point at a bar for its numbers." },
  { view: "dashboard", target: ["dash-speed", "dash-routes"], when: () => app.numbersEmpty !== true, optional: true, title: "Speed and routes", text: "Your average and top speeds, and which way is fastest for you: Gofile, Google Drive or Direct." },
  { view: "dashboard", target: ["dash-people", "dash-records"], when: () => app.numbersEmpty !== true, optional: true, title: "Your people and Records", text: "Who you exchanged the most with in Direct and your best speeds with them, then your records: fastest download, biggest file and more." },
];

// Left, then down the sections.
const settings: Step[] = [
  { view: "settings", target: "settings-nav", title: "Settings", text: "Click a section to jump to it. Changes apply when you click Save; Cancel leaves everything as it was." },
  { view: "settings", target: "set-accounts", title: "Accounts", text: "Connect your Google Drive for uploads. Downloads never need a sign-in." },
  { view: "settings", target: ["set-window", "set-notices"], title: "Window and Notifications", text: "Light or dark, what minimising and closing do, starting with Windows, and asking before quitting. Then what shows as a pop-up and what the bell keeps, and Do not disturb." },
  { view: "settings", target: "set-downloads", title: "Downloads", text: "Your download folder, connections per file, a speed limit, video quality, and set hours for downloading." },
  { view: "settings", target: ["set-upload", "set-browser"], title: "Upload and Browser extension", text: "Where uploads go and how several files are packed. Then connect Pako to your browsers, add the extension, and choose whether Pako asks before each browser download." },
  { view: "settings", target: "set-updates", title: "Features & updates", text: "Pako's version, the video tools and updates. Run the first-start setup again from here too." },
];

// The first look: short, down the sidebar. The details are in each page's Tutorial.
const full: Step[] = [
  { target: "tutorial", title: "Welcome to Pako", text: "A one-minute look around. Every page has a Tutorial button like this one for its details. Use → and ←, and Esc stops." },
  { target: ["nav-downloads", "nav-found"], title: "Downloads and Links", text: "Paste links anywhere with Ctrl+V. Pako checks them in Links, you pick what you want, and they download in Downloads." },
  { target: ["nav-send", "nav-direct"], title: "Upload and Direct", text: "Upload shares big files with a link anyone can open. Direct sends files straight to another Pako, PC to PC." },
  { target: "nav-dashboard", title: "Dashboard", text: "Your Pako in numbers: what you moved, how fast, and with whom." },
  { target: ["help", "settings"], title: "Help and Settings", text: "Guides, What's new, every tutorial and Report a problem are in Help (F1). Settings has your download folder, the browser extension and more." },
  { title: "Next to the clock", text: "Pako also lives in the tray: one click on its icon shows today's downloads in a small window. Pin it to keep it on top." },
];

export const TOURS: Record<TourId, Step[]> = { downloads, found, send, direct, dashboard, settings, full };

/** Asked before a tutorial starts, for its steps' `when` (the Dashboard: anything counted yet?). */
export const PREPARE: Partial<Record<TourId, () => Promise<unknown>>> = {
  dashboard: () => app.checkNumbers(),
};

/** The pages with a tutorial, in sidebar order (for Help). */
export const PAGE_TOURS: { id: Exclude<TourId, "full">; label: string }[] = [
  { id: "downloads", label: "Downloads" },
  { id: "found", label: "Links" },
  { id: "send", label: "Upload" },
  { id: "direct", label: "Direct" },
  { id: "dashboard", label: "Dashboard" },
  { id: "settings", label: "Settings" },
];
