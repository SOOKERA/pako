# Downloading

Add links, follow your downloads, and get files from Google Drive, even big or busy ones.

## Add links

You can add links in several ways:

- Press **Ctrl+V** anywhere in Pako. Whatever you copied is read for links.
- Paste them in the search box at the top.
- Drag links, or a text file full of links, onto the window.
- Click **Add links**, paste the links, and click **Add link** (or **Add 3 links**, and so on).
- With **Settings › Downloads › Catch copied links** on, links you copy in another app go to Links by themselves (not while Pako's window is in front, and not links already in Pako).

New links first go to **Links**, where Pako checks them and you pick what to download (see [Links and the tray window](links-and-tray.md)). Click **Download** there and they move to **Downloads**.

## Follow your downloads

The **Downloads** tab lists everything with its **Source**, **Progress**, **Speed**, **Status** and **Date**. Click a column title to sort by it; the newest downloads are at the top by default.

The left side filters the list:

- **All downloads**, **Downloading**, **Waiting**, **Finished** and **Problems**.
- **Sources**: only Google Drive, only web links, and so on.

Select a download to see its details on the right (**Ctrl+I** shows or hides that panel). You see its parts, where it's saved, whether it can continue after a pause, a **Preview** for an unfinished video file (play a short check before it's all there; types other than MP4 and WebM need **Preview more video types** in **Settings › Features & updates**; a video from YouTube or another site plays a short clip from the site instead), and buttons to **Pause**, **Resume** or **Try again** (**Open** once it's finished), open its **Folder**, or **Remove** it.

**Resume** and **Pause** at the top work on the downloads you selected, or on every one shown when nothing is selected. **Remove** works on the selected ones.

A row marked **Via browser** is a file only your browser could get: the browser downloads it, in one piece (none of Pako's faster download in parts), and Pako follows it and moves it to your folder when it's done. Its **Pause**, **Resume** and **Remove** act on the browser's download. See [Browser extension](browser-extension.md#via-browser).

## Fast and steady

- Pako splits a big file into several parts and downloads them at the same time. The details panel shows the parts filling in.
- **At once** (bottom bar) sets how many downloads run together, up to 20. Use **−** and **+** to change it. The rest wait their turn.
- **Settings › Downloads** also has a **Speed limit**, and under **Advanced** at the bottom of the tab, **Connections per file**. With **Auto** on (the default), a file starts with 4 connections and Pako adds more while it makes the file faster, up to 16 (8 for Google Drive files). Turn **Auto** off to set a fixed number from 1 to 32.
- If the connection drops, Pako waits and carries on where it stopped. Status shows **Reconnecting** or **No internet** meanwhile.
- **Pause** keeps what's done. **Resume** continues from there, even after closing Pako or restarting the PC, when the site allows it. The details panel says so under **After a pause**.
- If the file on the server was replaced since the download started, Pako doesn't mix the two: it stops and says "The file on the server changed since this download started". Use **Start over**.
- **Start over** (right-click a download) throws away what's done and starts again from zero.
- If a link answers with a web page (a sign-in page, or "link expired") instead of the file, Pako stops and says "The server sent a web page instead of the file". Open the link in your browser and download it there: Pako catches the download. A browser download that turns out to be a web page stays in the browser.
- A file you picked in **Links** from several hosts takes the other hosts along as backups. If its host fails for good, or gives no answer for 5 minutes in a row, Pako goes on from the next one by itself: the row shows **from** and the new host, and the details show the **Backups** left and the hosts **Tried**. With the same size on the new host, Pako compares a few small pieces from it with what it already has, and carries on from there when they match. When every host fails, the download stops and its message ends with "All 3 hosts were tried" and their names; **Try again** goes round them once more. See [Same file on several hosts](links-and-tray.md#same-file-on-several-hosts).

## When everything is done

**When done** (bottom bar) tells Pako what to do once every download has finished (it also waits for uploads and Direct transfers): **Do nothing**, **Sleep** or **Shut down**. Before sleeping or shutting down, Pako shows a short countdown you can cancel. The choice goes back to **Do nothing** after it happens, and each time Pako starts.

Downloads that were running when Pako closed carry on by themselves the next time it starts. To have them wait for **Resume** instead, turn off **Settings › Downloads › Advanced › Continue downloads when Pako starts**.

With **Settings › Downloads › Keep the PC awake while downloading** on, Windows doesn't go to sleep in the middle of a download, an upload or a Direct transfer.

Downloads can also be limited to set hours: **Settings › Downloads › Only download at set hours**. Waiting downloads then show **Scheduled · 22:00** (for example), and the bottom bar shows when they start.

## Google Drive

You don't need to sign in to anything.

- **A shared file** ("anyone with the link"): paste its link. Pako downloads it straight from Google, in parallel parts.
- **A shared folder** ("anyone with the link"): paste its link. Pako lists every file inside, sub-folders included, in **Links**. Tick what you want and click **Download**. There's no waiting for Google to make a zip.
- After a Drive download, Pako compares the file with Google's own record (**Settings › Downloads › Advanced › Check Drive files when finished**). The details panel's **File check** says "Passed: the file is complete" when it matches.

### "Google limit": too many downloads today

When a shared file has been downloaded too many times in one day, Google blocks it for a while. The download shows **Google limit**, and its details show what to do:

1. Click **Open in Drive** (details panel), or right-click the download › **Open in Drive (make a copy)**.
2. Click **Add shortcut to Drive**, then in your Drive right-click it and choose **Make a copy**.
3. Download your copy: Pako catches it (with the browser extension). Delete the copy afterwards to free the space.

You can also wait: Pako tries the original again by itself every few hours, or click **Try again**.

### "Only in the browser": files shared with certain people

Some files are shared only with certain accounts, not with "anyone with the link". Pako shows **Only in the browser**:

1. Click **Open in browser** (details panel), or right-click the download › **Open in browser to download**.
2. Sign in to the right Google account in your browser if needed, and click Download there.
3. Pako catches the download and finishes it, still in parallel parts.

The same goes for a folder Google won't list: open it in your browser and download from there. If Google refuses Pako itself for a while (not the folder), Pako says "Pako can't list Drive folders right now" and offers the same way through your browser.

## Before a download starts

- **Already downloaded?** If a file is already in your downloads or in its folder, Pako asks first. **Download again** keeps the old file and saves a second copy with a new name. When only some of the links are already downloaded, **Skip it** (or **Skip them**) downloads just the others. **Cancel** downloads nothing.
- **Disk space.** If the drive is too full, the download stops with a message saying how much it needs and how much is free. Free some space, then click **Try again**.

## Right-click a download

- **Download first** puts it ahead of the others, even outside the set hours.
- **Pause**, **Resume**, or **Try again** for one that stopped.
- **Priority: First**, **Normal** or **Later**, and **Move to top of queue** / **Move to bottom of queue**.
- **Notify when done** shows a pop-up at the bottom right of the screen when it finishes, or if it stops, and plays a short chime when it finishes (see [Notifications](notifications.md)).
- **Open file**, **Show in folder**, **Open link in browser**, **Copy link**, **Rename…**
- **Remove**. For a finished download Pako asks whether to **Remove from list** only, or **Delete file too** (to the Recycle Bin). Unfinished parts are always deleted.

Right-click an empty part of the list for **Paste links**, **Add links…**, **Select all** and **Clear finished**.

## Keyboard shortcuts

- **Ctrl+V**: paste links (anywhere).
- **Ctrl+F** or **Ctrl+K**: search.
- **Ctrl+I**: show or hide the details panel.
- **Ctrl+Tab**: switch between Downloads and Links.
- **↑ ↓**, **Home**, **End**: move through the list; hold **Shift** to select several.
- **Space**: select or unselect the row you're on (tick or untick it, in Links).
- **Ctrl+A**: select all. **Esc**: clear the selection.
- **Delete**: remove. **F2**: rename. **Enter**: open a finished file.
- **Ctrl+Z**: undo the last removal, for a few seconds after it and in the same list (links, or finished downloads taken off the list without deleting the file). The message that shows also has **Undo**.
- **F1**: open Help.

## Light or dark

You pick the look in the first-start setup (dark unless you pick another). Click the sun at the top right, next to the bell, to switch to light; click the moon to go back. The choice is kept, and the small tray window, the pop-ups and the **Download with Pako** window follow it.

In **Settings › Window › Theme**, pick **Dark**, **Light** or **Match Windows**. **Match Windows** follows the app mode chosen in Windows (**Settings › Personalization › Colors**), even while Pako is open. The browser extension's window follows your browser's light or dark mode.

## Good to know

- Downloads go to the folder you picked when Pako first started. Change it in **Settings › Downloads › Download folder**.
- A download's folder can be changed until it starts (details panel, **Save to**).
- While a file downloads, it sits in its folder with `.pakopart` added to its name (`film.mp4.pakopart`), and gets its real name once it's complete. Downloads left unfinished by an older Pako carry on from what they have.
- Drag finished downloads out of the list into any folder.
- Split archives (.part1.rar, .7z.001, .zip.001…) are extracted when all parts are done (**Settings › Downloads › Extract split archives**, or **Extract when all parts are done** in the set's details). This needs 7-Zip: install it in **Settings › Features & updates**. A file of the same name already in the folder is kept: the extracted one gets a new name.
- Everything Pako knows about your downloads stays on your PC.
