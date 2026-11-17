# What's new

## 1.2.1

### Fixed

- Everything sent from a private window (right-click, links, the video button) now leaves no trace in Pako, like downloads taken over.
- The Download pill on videos no longer says "just updated" for every problem: it says when the extension isn't responding, with the browser's reason.
- A download from a site's own sign-in box is no longer taken from the browser before Pako has it, and a download neither Pako nor the browser could get now says so.
- Downloads in Vivaldi (and browsers like it that ask first) no longer show the browser's own download question before Pako's window.
- A Direct transfer you cancelled, or that expired, no longer comes back as "waiting" after Pako restarts.
- Pako no longer crashes, rarely, when Direct stops or restarts.

## 1.2.0

### Browser extension

- **Downloads go straight to Pako**: its window opens the moment you click, before any **Save as**.
- **Via browser**: files only your browser can get are downloaded there and followed in Pako.
- **Private windows**: allow Pako there in your browser; it keeps no trace beyond the file.
- **Zen**, **LibreWolf**, **Waterfox** and **Floorp** can add Pako too, and Settings shows which browsers have it.

### Direct

- **Transfers resume** by themselves after a drop, a sleep or a restart. Also in **My devices**.
- **Send from another of your PCs**: pick it at the top of the contact list and send to its contacts.
- **Mute** a contact, and choose how often Pako checks on each one.
- Stays light even with hundreds of contacts.

### Upload

- **ZIP in parts**: a dropped connection or a power cut only resends the unfinished part.

### Notifications

- A soft chime when a download marked **Notify when done** finishes.
- No **Download finished** pop-up while the tray window is open.

### Settings

- **Tabs**, a **search** box, and an **Advanced** part on each tab for the rarely used settings.

### Fixed

- Sending links from the browser no longer waits for Pako to check the server.
- In the light look, the background behind a question window no longer looks see-through.
- In **My devices**, a file on its way no longer shows a tall empty strip instead of its progress bar.

## 1.1.0

### Videos

- **Videos on any page**: the full browser extension spots the video playing on almost any page. Click the **Download** button on the video, or the Pako button in the toolbar.
- **Record this video**, when Pako can't download a video: the extension records it while it plays. Protected videos are never downloaded or recorded.
- **Preview**: play about 20 seconds of a video in its details, to check it's the right one.

### Browser extension

- Now in **Firefox**, **Opera** and **Opera GX** too: **Settings › Browser extension**.
- **Settings › Browser extension** shows which browsers are **Connected**.

### Links

- **The same file on several hosts** is now one row: pick the host, and if it fails, Pako carries on from the next one, where it stopped when the file is the same.

### My devices

- A new page for **your own PCs**: send files between them without Accept, see and control each other's downloads, and send links for another PC to download, with their quality and priority.
- **Download on** in Links: pick which of your PCs downloads a link.

### Direct

- **On the same Wi-Fi**, two Pakos send files straight over your home network, much faster, even without internet. The contact's dot turns blue.
- Pako asks for your name once, before your first contact.

### Pako

- A pop-up tells you when a new Pako is ready, with **Restart now**.
- **Continue downloads when Pako starts** can now be turned off, in **Settings › Downloads**.
- Pako now has **terms of use**, which you accept once. Your downloads and lists stay as they are.

### Fixed

- **Download first** no longer restarts a download that can't resume.
- Downloading a split archive again now asks before making a second copy.
- **Restart now** asks first while files are moving in Direct.
- A contact's new name shows at once, instead of staying "Someone".
- With automatic updates off, the video tool no longer updates by itself.
- Uninstalling with **Also delete Pako's settings and lists** also clears Pako's temporary files.
- Safety fixes for Direct, My devices, video playlists and Pako Portable.

## 1.0.1

### Fixed

- Installing ffmpeg (for **Preview more video types** and **Download videos from YouTube and other sites**) failed with "No Windows build in the latest release": its source changed how it names its builds. Pako finds them again.
- With a portable and an installed Pako on the same PC, turning off **Start with Windows** in one (or leaving it off in the first-start setup) no longer stops the other from starting with Windows.

## 1.0.0

### Light or dark

- **Pako has a light look.** Switch with the sun or moon at the top right, or pick **Dark**, **Light** or **Match Windows** in **Settings › Window › Theme**. The first-start setup asks too.
- The tray window, pop-ups and the **Download with Pako** window follow. The browser extension's window follows your browser's light or dark mode.

### Downloads

- Unfinished downloads now end in .pakopart. Downloads left unfinished by an older Pako carry on from where they stopped.
- A link that answers with a web page (a sign-in page, or "link expired") instead of the file now stops and says so, instead of saving the page as the file. A browser download like that stays in the browser.
- Extracting split archives never writes over a file of the same name: the extracted file gets a new name.
- **Start over** on a Google Drive file that was replaced on Google now works.
- Renaming a paused video keeps what was already downloaded.
- **Pause**, **Start over** and the end of the download hours act at once, even with a low speed limit.
- Link pages (a page to click through before the file) now open in a private window: its cookies and site data are gone when it closes. A site's check may ask you again for each link.

### Browser extension

- **Pako Light**: a smaller edition of the extension for the Chrome Web Store, without videos. If the full extension is on in the same browser, Pako Light steps aside, so a download goes to Pako only once.
- Pako Light's small window now says where to get Pako when Pako isn't on this PC.
- A browser download keeps its name after the **Download with Pako** window: no more "name (1)", and no unfinished .pakopart file left behind.
- Right-click › **Download with Pako** now uses the link's own file name instead of "download".
- The right-click items now follow you when you switch to another browser window.
- When a file sent from your browser is on another site than its page, that site now gets only the page's main address (like your browser does), never the full address.

### Upload

- Connecting your Google Drive for uploads now works in every Pako, not only on some PCs.
- If the box for Google Drive isn't ticked on Google's page, Pako now says so instead of connecting a Drive it can't upload to.
- When several files go up **As they are**, **Keep** or a new delete date picked before the upload finishes (or before **Try again**) now stays. It used to go back to 7 days.
- Upload says "upload" everywhere: **Not all uploaded**, **Uploading**, "Pako's uploads" in the Drive bar.
- New uploads to Google Drive go to a folder called "Pako uploads". If you already have a "Pako sends" folder from an earlier Pako, Pako keeps using it.
- The same three choices everywhere: **As they are**, **One ZIP** and **Compressed ZIP**, in **New upload**, in **Settings › Upload** and under **Uploaded as** in the details panel.

### Direct

- Direct now starts the first time you open the **Direct** page, and with Pako from then on. If you never open it, Pako makes no Direct connection. If you already used Direct, nothing changes.
- A contact who restarts their Pako is back online at once. It could take up to 3 minutes.
- When their Pako can't save what you send, yours says so ("Their Pako couldn't save it.", or that their drive is full) without showing their folder or free space. Too many sends at once are turned away with "Too many sends in a short time. Try again in a minute."
- Files still arriving now end in .pakorecv, so they can't mix up with an unfinished download.
- A contact's progress no longer jumps back when one of several files finishes.
- Closing a "wants to connect" or "wants to send you files" question with **×** or **Esc** declines it, and now says so. A question takes an answer only after a short moment, so a stray click can't decline it.
- **Clear history…** asks first.
- Declining or removing someone says that Pako turns them away without asking you for 7 days. If someone declines you, Pako says when you can ask again.
- **Waiting for you** and the count on the **Direct** tab are now coral, like the **!** on a contact: something waits for your answer. Amber stays "waiting for them to accept".
- Files you pick to send stay with that person, like a draft: another contact shows their own, and files can't go to someone else when a contact comes online.
- With an offline contact, Direct says plainly that files go only while you're both online.
- A received folder with deep sub-folders no longer fails at the very end.
- Files moving in Direct now count like downloads: the PC stays awake for them (with **Keep the PC awake while downloading** on), **When done** waits for them, and quitting asks first.

### Help

- **Help › About Pako**: the copyright, the licence and Sookera's additional terms, with a link to Pako's source code. **Free software (GPL-3.0)** in Help and Settings opens it.
- **Help › Report a problem**: say what happened and send it to Sookera in one click with **Send report** (you get a reference back), or with your own email app, or copy it, or save it as a file. You see everything that goes with it first, including the last problems Pako noted (kept on your PC, without links' addresses, file names or your name). Nothing goes until you click.
- New guides: **Installing and updating**, **Upload**, **Direct**, **Notifications**, **Dashboard** and **Help and problems**. Every guide was checked against this version.

### Updates and problems

- An update that Windows won't start (an antivirus, for example) is put back: your Pako keeps working, and **Settings › Features & updates** says the new version didn't start properly.
- If Pako has to close because of a problem, it now says so, and the problem is in **Help › Report a problem**. If its window can't open, it says to install Microsoft Edge WebView2 Runtime.

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
