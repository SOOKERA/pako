# Installing and updating

Get Pako, keep it up to date, and remove it if you need to. Pako runs on Windows 10 and 11 (64-bit).

## Pako Setup or Pako Portable

Pako comes two ways. Both are the same program and update themselves the same way.

- **Pako Setup** installs Pako for you, without admin rights, in `%LOCALAPPDATA%\Programs\Pako` (you can pick another folder). It puts Pako in the Start menu (and on the desktop if you like) and in Windows' Apps list, so it can be uninstalled like any app.
- **Pako Portable** is one file, `Pako.exe`. Put it in any folder and run it. It isn't added to the Start menu or Windows' Apps list. Give it a folder of its own that only you change (not your Downloads folder, where web pages can save files next to it).

**Settings › Features & updates** shows which one you have: **Installed** or **Portable**, next to the version.

### Where your settings and lists are kept

- **Pako Setup**: in your Windows user folder, `%LOCALAPPDATA%\Pako`. Reinstalling or updating never touches them.
- **Pako Portable**: in a `data` folder next to `Pako.exe`. If Pako can't write there (in Program Files, for example), it uses `%LOCALAPPDATA%\Pako` instead.

Your downloaded files are never in these folders: they go to your download folder.

Pako Setup won't install Pako into `%LOCALAPPDATA%\Pako` or into a portable Pako's folder (**Next** stays greyed), so your settings and lists can't get mixed up. To move from a portable Pako to an installed one, see [From Portable to Setup](#from-portable-to-setup).

## Terms of use

Pako has terms of use, the same as on [sookera.com/terms](https://sookera.com/terms). In short: Pako is free and comes as it is, with no warranty; you're responsible for what you download, share, send and record with it; use it only for what you have the right to, and follow the rules of the sites you use.

- **Pako Setup** shows them on its **Terms of use** page. Click **I Agree** to go on; **Cancel** stops the install.
- **When Pako first starts** (Setup or Portable), it shows them again before anything else, with **Agree and continue** and **Quit Pako**. Nothing runs before you agree: no downloads, no Direct or My devices, nothing from the browser extension (it says "Pako needs you to accept its terms first"), no update checks, and the tray icon only opens or quits Pako. Closing the window or pressing **Esc** doesn't skip them. As soon as you agree, everything starts, without a restart, and the first-start setup follows.
- **If you used Pako before it had terms** (Pako 1.0.x), it shows them once after updating, as **Pako's terms of use**. Your downloads and lists stay as they are: paused and waiting downloads simply wait until you agree, and nothing is lost. A Pako that updates and restarts in the tray doesn't pop up: the terms show the next time you open Pako, or when the browser extension sends it a download (the browser keeps that download meanwhile). Until then, the tray icon's tooltip says "Pako: open it to continue".

Pako asks once: it notes the date of the terms you agreed to in its settings and doesn't ask again, also after later updates. The newest version is always on [sookera.com/terms](https://sookera.com/terms); **Terms** next to **Privacy** in Help and in **Settings › Features & updates** opens it, and **Help › About Pako › Terms of use** shows the text that came with your Pako.

## "Windows protected your PC"

Pako isn't signed with a paid certificate, so the first time you run it, Windows may show "Windows protected your PC". Click **More info**, then **Run anyway**.

Pako's window needs Microsoft Edge WebView2, which Windows 11 and most Windows 10 PCs already have. If it's missing, Pako Setup downloads it from Microsoft and installs it. Pako Portable doesn't: it says to install it (see [If Pako has to close](help-and-problems.md#if-pako-has-to-close)).

## Free software

Pako is free software under the GNU GPL version 3, with Sookera's additional terms. **Help › About Pako** shows the copyright, the licence and the additional terms, with a link to the source code; **Free software (GPL-3.0)** in Help and in **Settings › Features & updates** opens it too. Every release comes with the same texts as `LICENSE` and `NOTICE.md`, and Pako Setup puts them next to `Pako.exe`, with the terms of use (`TERMS.md`).

Its source code is on GitHub: [github.com/sookera/pako](https://github.com/sookera/pako).

## Updates

Pako looks for a new version once a day, also while it stays open in the tray for days.

- With **Settings › Features & updates › Update Pako and its tools automatically** on (as it is at first), Pako downloads the new version by itself. A pop-up and the bell say "Pako 0.2.0 is ready" (for example), with **Restart now**, also while Pako waits in the tray. When files are downloading, uploading or moving in Direct, **Restart now** asks first, as Quit does. If you don't restart, the new version starts the next time you start Pako. The tools Pako uses (for videos, for split archives) update quietly too. With it off, they're only updated when you click **Update** next to them.
- With it off, Pako only tells you: "Pako 0.2.0 is available (Settings › Features & updates)". Click **Update** there, then **Restart now**.
- **Check for updates** (in **Settings › Features & updates**) looks right away (and, with automatic updates on, downloads the new version).

After an update, Pako shows once what's new in that version (Help has them all).

Updates are safe:

- Pako only accepts a new version signed by Sookera. Anything else is refused, wherever it came from.
- Your settings and lists aren't touched.
- If the new version doesn't start properly, or Windows won't start it (an antivirus, for example), the previous one is put back and keeps working, and **Settings › Features & updates** says so. Pako doesn't download that version again by itself; **Update** tries it again.

A portable Pako in a folder Windows doesn't let you change (Program Files, for example) can't update itself, and Pako says so. Move its folder somewhere you can change files, such as Documents.

### A newer Pako Setup over an installed Pako

You can also run a newer **Pako Setup** over the installed Pako. Keep **Replace the installed version (recommended)** and click **Next**: if Pako is open, Setup offers to close it first. Your settings and lists, the shortcuts, the taskbar pin, starting with Windows and the browser connection all stay, and Setup doesn't ask about deleting anything.

## Uninstalling

### Pako Setup

1. Open Windows **Settings › Apps** (**Installed apps**), find Pako and click **Uninstall**. If Pako is open, the uninstaller offers to close it first.
2. Tick **Also delete Pako's settings and lists** only if you want them gone too. It's unticked at first, so reinstalling later picks up where you left off.

Uninstalling removes Pako itself, its shortcuts, and what it set up in Windows: the connection to your browsers, Pako's links from web pages, and starting with Windows (a portable Pako's own browser connection and start with Windows stay). Your downloaded files stay, and so do your uploads on Gofile or Google Drive. The browser extension stays in your browser until you remove it there.

If a portable Pako is open while you uninstall with the box ticked, the settings and lists are kept (it may use them too) and the uninstaller says so. To delete them anyway, close that Pako, then delete the `Pako` and `com.sookera.pako` folders in `%LOCALAPPDATA%`.

With the box ticked, Pako's folder in Windows' temporary folder (`%TEMP%\Pako`) goes too: recordings not made into a video yet.

Without the box ticked, both folders stay: `Pako` (settings and lists) and `com.sookera.pako` (the window's own data, such as its size and place; a Pako older than 1.0.0 also kept what link pages stored there).

### Pako Portable

1. In Pako, turn off **Settings › Window › Start Pako with Windows** and **Settings › Browser extension › Connect Pako to your browsers**, then click **Save**.
2. Quit Pako (right-click the tray icon › **Quit Pako**).
3. Delete `Pako.exe` and its `data` folder, and the `com.sookera.pako` folder in `%LOCALAPPDATA%` (the window's own data, such as its size and place; a Pako older than 1.0.0 also kept what link pages stored there). If you recorded videos, also delete the `Pako` folder in `%TEMP%`.

A portable Pako's `data` folder holds your lists (the links you download, your contacts): anyone who can open the folder `Pako.exe` is in can read them, so keep it in a folder of your own. Its logins (Google Drive, Gofile, your Pako ID's key) are encrypted for your Windows user and stay unreadable.

## From Portable to Setup

The installed Pako keeps its settings and lists in another place than the portable one, so it starts with empty lists and asks the first-start questions again.

To keep your lists and settings (downloads, uploads, Direct contacts and history, Dashboard numbers):

1. Quit Pako (right-click the tray icon › **Quit Pako**).
2. Open the `data` folder next to your portable `Pako.exe` and copy everything inside it.
3. Type `%LOCALAPPDATA%` in File Explorer's address bar and press **Enter**. Open the `Pako` folder there (make it if it isn't there) and paste the files, replacing the ones already there.
4. Start the installed Pako. It takes over the browser connection and starting with Windows from the portable one.
5. In Chrome, Edge, Brave, Vivaldi or Opera, the extension is still loaded from the portable Pako's folder. Remove it on the browser's extensions page, then add it again from the installed Pako (see [Browser extension](browser-extension.md#add-pako-to-your-browser)). Firefox needs nothing.

If your portable Pako has no `data` folder next to it, it already kept everything in `%LOCALAPPDATA%\Pako`: the installed Pako finds it there by itself.

Do this on the same PC and Windows user. Your Direct contacts know your Pako by its key, which only this Windows user can read: on another PC, Pako makes a new Pako ID.
