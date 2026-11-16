# What's new

## Next version

### Light or dark

- **Pako has a light look.** Switch with the sun or moon at the top right, or pick **Dark**, **Light** or **Match Windows** in **Settings › Window › Theme**. The first-start setup asks too.
- The tray window, pop-ups and the **Download with Pako** window follow. The browser extension's window follows your browser's light or dark mode.

### Downloads

- Unfinished downloads now end in .pakopart. Downloads left unfinished by an older Pako carry on.
- If an unfinished download was left twice, once by this version and once by an older one, it carries on from the file changed last.

### Browser extension

- **Pako Light**: a smaller edition of the extension for the Chrome Web Store, without videos. If the full extension is on in the same browser, Pako Light steps aside, so a download goes to Pako only once.
- The right-click items now follow you when you switch to another browser window.

### Upload

- Connecting your Google Drive for uploads now works in every Pako, not only on some PCs.
- Upload says "upload" everywhere: **Not all uploaded**, **Uploading**, "Pako's uploads" in the Drive bar.
- New uploads to Google Drive go to a folder called "Pako uploads". If you already have a "Pako sends" folder from an earlier Pako, Pako keeps using it.
- The same three choices everywhere: **As they are**, **One ZIP** and **Compressed ZIP**, in **New upload**, in **Settings › Upload** and under **Uploaded as** in the details panel.

### Direct

- Files still arriving now end in .pakorecv, so they can't mix up with an unfinished download.
- A contact's progress no longer jumps back when one of several files finishes.
- Closing a "wants to connect" or "wants to send you files" question with **×** or **Esc** declines it, and now says so.
- Declining or removing someone says that Pako turns them away without asking you for 7 days. If someone declines you, Pako says when you can ask again.
- **Waiting for you** is now coral, like the **!** on a contact: something waits for your answer. Amber stays "waiting for them to accept".
- With an offline contact, Direct says plainly that files go only while you're both online.

### Help

- **Help › About Pako**: the copyright, the licence and Sookera's additional terms. **Free software (GPL-3.0)** in Help and Settings opens it.
- **Help › Report a problem**: say what happened and send it to Sookera in one click with **Send report** (you get a reference back), or with your own email app, or copy it, or save it as a file. You see everything that goes with it first, including the last problems Pako noted (kept on your PC, without links' addresses, file names or your name). Nothing goes until you click.
- New guides: **Installing and updating**, **Upload**, **Direct**, **Notifications** and **Dashboard**. Every guide was checked against this version.

### Installer

- Installing a newer Pako Setup over an installed Pako keeps its shortcuts, its taskbar pin, starting with Windows and the browser connection, and no longer asks about deleting your settings in the middle.
- Uninstalling (a Pako installed with this version's Setup or a later one) no longer turns off a portable Pako's start with Windows, and keeps the settings and lists while another Pako is open.
- Pako Setup won't install into the folder Pako keeps its settings in, or into a portable Pako's folder.

## 0.1.2

### Downloads

- **Videos**: paste a link from YouTube or another video site and Pako downloads it. Pick the quality, or **Audio only**.
- Videos download in several parts at once, so they finish much faster.
- Videos sent from your browser use your login for that site, so videos you can only watch signed in work too.
- Google Drive downloads no longer need a Google sign-in. A shared folder lists every file inside: no waiting for Google to make a zip.
- The temporary copies older versions made in your Google Drive are removed, and the old Google sign-in for downloads is signed out.
- **Google limit** and **Only in the browser** downloads now say what to do next.
- At first start, Pako asks where your downloads go.
- Drag finished files out of Pako into any folder.
- Pako keeps its video tools up to date even when it stays open in the tray for days (**Update Pako and its tools automatically**).

### Browser extension

- New: the Pako extension for Chrome, Edge, Brave, Vivaldi and Firefox (**Settings › Browser extension › Add to**).
- A **Download** button on YouTube, Vimeo, Twitch, Facebook and Instagram videos.
- Right-click a link › **Download with Pako**, or a video page › **Download video with Pako**.
- The **Download with Pako** window: check the name, the folder and the quality before a browser download starts.
- Not connected? **Open Pako** in the extension's small window opens Pako and connects it.
- Pako refreshes its extension folder by itself when it starts.

### Upload

- New **Upload** tab: share big files with a link. Drop files or folders, pick **Upload to** (Gofile, or your Google Drive) and click **Upload**.
- **Delete after**: uploads can delete themselves after 3, 7 or 30 days, or **Never**.
- Once you connect your Google Drive, uploads go there by default (you can still pick Gofile). Disconnecting it goes back to Gofile.
- Several files go up as they are, with one link, or packed into one ZIP (**Settings › Upload › When you upload several files**). A single file can be compressed.
- The **Uploaded** list shows where each item went and how often it was downloaded. Its details panel has the files, the link as a QR code, when it deletes itself and how fast it went.

### Direct

- New **Direct** tab: send files straight to another Pako, nothing uploaded in between. Each person has their own history.
- Tags and filters on the left, your own name for each person, and **Can send to me without asking** for people you trust.
- Files people send you go to a folder inside your download folder, like "From Rado".
- A person you're sending to or receiving from shows it in the list: "Sending 37% · 12 MB/s" under their name and a thin bar along their row.

### Dashboard

- New **Dashboard**: your Pako in numbers. What you downloaded, uploaded, sent and received, your speeds, **Your people** and your records. Save them as a file that opens in Excel.

### Notifications

- Pako's own pop-ups, at the bottom right of the screen: **Open** and **Show in folder** for a finished download, **Accept** and **Decline** for files someone sends you, **Review** when someone wants to connect. Click one to open Pako on the right page. They leave after a few seconds (not while the mouse is on them); questions stay until you answer or close them, and leave by themselves once settled elsewhere (answered in Pako, the link page finished or closed, the other person gave up). The same link asking again replaces its earlier pop-up.
- The bell at the top of Pako keeps what's important for 30 days, with a red count while something is unread: people asking to connect, files waiting for your answer, and **Pako needs you**. **Mark all as read** and **Clear** are in its panel. The tray icon gets a dot too. Once a question is settled, its line says how: **Accepted**, **Declined**, **Done**, **Not finished** or **No longer waiting**.
- **Settings › Notifications**: a **Pop-up** and a **Bell** switch for each kind. **Notify when a download finishes** is now **Download finished › Pop-up**; a download with **Notify when done** still gets its pop-up.
- **Do not disturb** turns pop-ups off (the bell still collects). **Follow Windows' Do not disturb** (on at first) holds them during full-screen games and videos, presentations and Windows' own Do not disturb. Pop-ups already on screen hide then too, and come back when it's over.
- A download Pako tries again by itself (Google's daily limit, every few hours) doesn't pop up again when it stops the same way.

### Window

- **Help** (or **F1**): guides, What's new, a one-minute **Quick tour**, and a **Tutorial** button on every page for its details. A Tutorial button stays red, with a dot, until you've seen that page's tutorial.
- Help and **Settings › Features & updates** now say **Pako by Sookera**, with links to the licence (Pako is free software under the GPL-3.0), the privacy page and the help online.
- The tray window can be pinned on top, minimised or closed like the main window. It shows what's running or waiting and what finished today; finished items have **Open** and **Folder**.
- **Settings** are grouped in cards, with the sections on the left.
- Smoother motion everywhere. Pako follows Windows' "Animation effects" setting.

### Installer

- Pako now also comes as an installer, **Pako Setup**, next to the portable Pako.exe. It puts Pako in the Start menu (and on the desktop if you like) and in Windows' Apps list, with no admin rights needed. Your settings and lists are kept in your Windows user folder, so reinstalling never loses them, and uninstalling asks before deleting them. Both update themselves the same way. **Settings › Features & updates** shows which one you have: **Installed** or **Portable**.
