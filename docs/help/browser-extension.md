# Browser extension

The Pako extension sends links, videos and downloads from your browser straight to Pako.

## Add Pako to your browser

Pako's extension works in Chrome, Edge, Brave, Vivaldi, Opera (Opera GX too), Firefox, and the browsers built on Firefox: Zen, LibreWolf, Waterfox and Floorp.

### Chrome, Edge, Brave, Vivaldi, Opera

1. In Pako, open **Settings › Browser extension**, turn on **Connect Pako to your browsers** and click **Save**. (The first-start setup does this for you when **Add Pako to my browsers** stays ticked.)
2. Under **Add Pako to your browser**, click **Add to Chrome** (or **Add to Edge**, **Add to Brave**, **Add to Vivaldi**, **Add to Opera**, **Add to Opera GX**). Its extensions page opens, and the extension's folder is copied for you (when the steps don't say "the folder is already copied", click **Copy** next to the folder).
3. On the extensions page, turn on **Developer mode**.
4. Click **Load unpacked**.
5. Click the address box at the top of the window that opens, press **Ctrl+V** and **Enter**, then **Select Folder**.

If the browser doesn't open its extensions page, Pako opens the folder instead and tells you which address to type in the browser. **Open the extension folder** (**Settings › Browser extension › Advanced**) opens it any time.

### Firefox, Zen, LibreWolf, Waterfox, Floorp

The Firefox version comes with Pako, signed by Mozilla (Firefox only adds signed extensions). Zen, LibreWolf, Waterfox and Floorp take the same one: the steps are the same, with that browser's name instead of Firefox.

1. In Pako, open **Settings › Browser extension**, turn on **Connect Pako to your browsers** and click **Save** (the first-start setup does this for you, as above).
2. Under **Add Pako to your browser**, click **Add to Firefox** (or **Add to Zen**, **Add to LibreWolf**, **Add to Waterfox**, **Add to Floorp**). The browser opens and asks to add Pako: click **Add**.
3. So downloads behind a login work and the **Download** button shows on videos, Pako must be allowed on all websites: in the browser, open `about:addons` › **Pako** › **Permissions** and turn on **Access your data for all websites** if it is off.

When you add it, Firefox 140 and later (and the browsers built on those versions) also lists what the extension passes to Pako on your PC: your login for a download's site, the addresses of downloads and their pages, and the links on a page.

If the browser doesn't open, Pako opens the folder with the file `pako-firefox.xpi` selected: drag it into a window of that browser. A copy of Pako built by someone else from the source code may not include the Firefox version: Pako then says so.

### Another browser

Pako lists Chrome, Edge, Brave, Vivaldi, Opera, Opera GX, Firefox, Zen, LibreWolf, Waterfox and Floorp. Most other browsers are built on one of them, and Pako's extension usually works there too. First, in Pako: **Settings › Browser extension**, turn on **Connect Pako to your browsers**, click **Save**, then click **Open the extension folder** (under **Advanced**, at the bottom of the tab).

**A browser built on Chromium** (for example Arc, Thorium, Yandex or Ungoogled Chromium):

1. In the browser, open its extensions page: type `chrome://extensions` in the address bar (some browsers have their own, like `arc://extensions`).
2. Turn on **Developer mode**, then click **Load unpacked**.
3. Pick the **chromium** folder inside the folder Pako opened, then **Select Folder**.

**Another browser built on Firefox**:

1. Drag the file **pako-firefox.xpi** from the folder Pako opened (it sits next to the **chromium** folder) into a window of the browser. It's also on Pako's download page: open [github.com/sookera/pako-releases/releases/latest/download/pako-firefox.xpi](https://github.com/sookera/pako-releases/releases/latest/download/pako-firefox.xpi) in that browser, and it offers to add it.
2. Click **Add** when it asks.
3. In its add-ons page (`about:addons`) › **Pako** › **Permissions**, turn on **Access your data for all websites** if it is off.

Then click the Pako button in the browser's toolbar: it should say **Connected** (see [Check it's connected](#check-its-connected)). If it says **Pako isn't connected**, that browser looks for Pako somewhere else: tell us which browser it is (**Help › Report a problem**), so Pako can add it. Pako's own updates reach the extension in these browsers too: a Chromium one picks them up by itself, and a Firefox one when you drag in the new **pako-firefox.xpi** (some update it themselves).

### Pako Light

Pako Light is a smaller edition of the extension, made for the Chrome Web Store, with a teal icon. It takes over your downloads and has the right-click items, but it doesn't download videos: no **Download** button on videos, no **Videos on this page**, and **Download with Pako** works on links and pictures only. It also does nothing on video sites: there, **Send this page to Pako** (in its small window and in the right-click menu) and **Send all links on this page to Pako (to pick from)** are greyed out, its small window says "Pako Light doesn't work on this site.", and links to video sites are left out of "all links". Pako itself also refuses videos sent from Pako Light. The full extension, with the **Download** button on videos, comes with Pako (see above).

Only one extension takes over your downloads, so a download goes to Pako only once. If the full extension is on in the same browser, Pako Light steps aside: no right-click items of its own, and its small window says **Not in use** and **The full Pako extension is on in this browser**. You can keep both, or remove Pako Light.

## Check it's connected

Click the Pako icon in the browser's toolbar. The small window says **Connected**. If it says **Not connected**, see [Not connected](#not-connected) below.

The same window lets you:

- See the **Videos on this page** and download them (full extension only, see below).
- **Send this page to Pako**.
- Turn **Let Pako take over downloads** on or off.
- Turn the **Download button on videos** on or off (full extension only).

The small window follows your browser's light or dark mode.

Pako's own window shows it too: in **Settings › Browser extension** (and in the first-start setup), a browser whose extension talked to Pako in the last 30 days says **Connected** in green, with **Pako Light** when it's that edition and when it was last used ("last used today", "3 days ago"). Instead of a button, it says **Installed**. (To add the extension again after removing it, use **Open the extension folder**, under **Advanced**, and load it in the browser's extensions page.) A browser not used for longer says **Not used lately**, with its **Add to …** button. A browser with no line under its name was never connected. The list updates when you come back to Pako's window, so it says **Connected** right after you add the extension.

## Videos on this page

Download and record only what you have the right to, and follow the rules of the sites you use.

With the full extension, Pako notices the video playing on any page, also inside a player embedded from another site. The Pako button in the toolbar shows how many it found on that page. Click it: the small window lists them under **Videos on this page**, each with its kind (**HLS stream**, **DASH stream** or **Video file**), its server, and the best quality when the page says it (**up to 1080p**). Click **Download with Pako** next to one: Pako names the file after the page's title and downloads it like other videos (see [Videos](videos.md)).

- The list is only for that tab, and is emptied when the tab goes to another page or closes.
- If the video is protected, the small window says "This video is protected, so it can't be downloaded." and offers nothing.
- Once Pako has found a video on a page, the **Download** button also appears at the top right of the video when you point at it (see [below](#the-download-button-on-videos)).
- While a video plays or is listed, the small window also offers **Record this video**, for when Pako can't get it (the download says "Pako couldn't get this video.") (see [Record this video](videos.md#record-this-video)).

## The Download button on videos

With the full extension, on YouTube, Vimeo, Twitch, Facebook and Instagram, a **Download** button appears at the top right of a video when you point at it. Click it and pick **Up to 1080p**, **Up to 720p**, **Up to 480p** or **Audio only**. See [Videos](videos.md).

On any other page, the same button appears on the video once Pako has found it there (**Videos on this page**, above), in the frame of the player that plays it. Its menu has **Download this video** (or, when Pako found several, the same list as the small window), and **Record this video** while a video plays or is listed. A protected video gets nothing. Pages where Pako found no video are never touched. **Download button on videos** in the small window turns it off on every page.

## Right-click menu

- On a link, picture, video or sound: **Download with Pako**. It goes straight to **Downloads** (through the **Download with Pako** window, if it's on). A link to a video page downloads the video, in your usual quality. A player that streams in pieces has no address of its own: on a video site, its page's video is downloaded; elsewhere, use **Videos on this page** (below) or **Record this video**. Pako Light has this item on links and pictures only.
- On a page: **Send this page to Pako**. In the full extension, on video sites this item reads **Download video with Pako**: the video downloads in your usual quality, and a playlist goes to **Links** (see [Videos](videos.md)).
- On a page: **Send all links on this page to Pako (to pick from)**. They land in **Links**, where you tick what you want.

The items follow the tab in front, also when you switch to another browser window.

## Downloads from the browser

With **Let Pako take over downloads** on, files you download in the browser go to Pako instead, for example when you click a Download button on Google Drive or any site.

What happens when you click a download:

1. The browser doesn't download it: the extension takes it before the browser saves anything (in Chrome, Edge and similar browsers even before a **Save as** window, and in Vivaldi before its own question, see below), and the **Download with Pako** window opens at once with the file's name and size.
2. When you click **Download**, Pako's own first request to the server is the real download. A download can't be moved from the browser's connection to Pako's (no download manager can), so the server is asked once more, by Pako. A few one-time links refuse that second request.
3. If the server gives Pako the same file, nothing else happens. If not (no file, a web page instead, another size), your browser downloads it after all and Pako follows it: see **Via browser** below.

- Small files (up to 2 MB) stay in the browser.
- A file a page's form sent (with "POST") can't be asked for again: the browser holds it paused while Pako checks, and carries on with it if Pako can't get it. The **Download with Pako** window opens once Pako has checked it.
- A file from a site that asked you to sign in with its own sign-in box (not a page's form) can't be asked for by Pako: the browser holds it paused while Pako checks, as above, and carries on with it if Pako can't get it.
- If your browser couldn't start a download again after Pako couldn't get it, the Pako button shows a red **!** and says "Pako couldn't get … and your browser couldn't start it again. Download it once more from the page." (Pako's row says it too.)
- With Pako Light, which can't see how a page asked for a file, the browser always holds the download paused until Pako has it (the window opens once Pako has checked it, as above).
- With a Pako older than 1.2.0, the extension works as before: the browser holds the download paused until Pako has checked it.
- Files the browser flags as unsafe stay in the browser.
- A video page itself (a YouTube or Vimeo address, for example) is never taken over: for videos, use the **Download** button or the right-click menu. Pako Light leaves every download on a video site to the browser.
- In Firefox, a file type set to **Always ask** still shows Firefox's own question first; set it to **Save File** in Firefox's settings (**Files and Applications**) for a straight handover to Pako.
- Vivaldi asks "Download …?" (Save, Save As, Open, Cancel) before an extension can see a download. So there, Pako's extension stops the file as soon as the server answers: Vivaldi asks nothing and its download panel doesn't open. The tab shows a blank page for a moment and goes straight back to your page (a tab opened only for the file closes), and the **Download with Pako** window opens. Another browser built on Chromium that asks first gets the same once the extension has seen it ask twice in a row.
- The tab stays on Pako's page ("This download went to Pako. Use Back to return to your page.") when going back would send a form again (the page you were on was a form's answer), and when a page asks for the same file again by itself (a "your download starts in 5 seconds" page, even through a new link each time): Pako gets it once, and the page doesn't start it over and over. The same file is the same name and size within half a minute, in the same tab.
- Vivaldi's question still comes first for a file a form sent, a private window, a video site, a small file, while the extension can't reach Pako, and in a browser older than Chrome 128 (2024): click **Save** and Pako takes it from there, or turn on **Save Files to Default Location Without Asking** in Vivaldi's **Settings › Downloads**. A file from a site's own sign-in box goes back to Vivaldi (it asks then): Pako can't sign in there. A private window's tab opened a moment before the download may show Pako's page too: the download then goes on in your browser in that tab (if it can't, the page says so: go back and download it once more).
- A file Pako takes this way never reaches Vivaldi's own download checks; Windows still checks it when you open it.

### Via browser

Some files only the browser can get: a site that refuses download managers, a proxy or VPN that works only in the browser, a link that works once. Then your browser downloads the file, and Pako follows it as one of its own:

- The row in **Downloads** shows **Via browser**, with its progress, speed and time left.
- The browser's download is one connection: Pako's faster download in several parts, **Download first** and the priority don't apply to it (they're greyed out).
- **Pause**, **Resume** and **Remove** in Pako pause, resume or cancel the browser's download (when the browser can). A download the browser stopped can be continued with **Try again** when the browser can continue it.
- When the browser is done, Pako moves the file to the folder you chose (or Pako's usual one), never over another file, and once it's there the browser's download list forgets it (the file stays). If both are the same folder, the file stays where it is, unless another download in your list has that name (an older one whose file was deleted since, or one not finished yet): then it gets a free name, like "file (1).zip". If Pako can't move it (the file is open, the other drive is full), it says so and leaves it where the browser saved it, still in the browser's download list.
- If Pako is closed when the browser finishes, Pako lists the file the next time the extension reaches it (the extension doesn't start Pako for this).

### Private windows

Pako works in private windows only if you allow it there, on the extension's page in your browser: **Allow in Incognito** (Chrome), **Allow in InPrivate** (Edge), or **Run in Private Windows** (Firefox: menu › **Add-ons and themes** › Pako). The extension's small window says whether it's on, with a button to that page (Firefox doesn't let extensions open it, so the window says where to go).

When it's on, everything you send from a private window is private: downloads Pako takes over, the right-click items, **Send all links on this page**, the **Send** button in the extension's window, and the **Download** button and pill on videos. They go to Pako like any other, never with your normal login (a download Pako takes over keeps that window's own login, and so do the right-click items and **Send** in Firefox; links and videos go without one), and Pako keeps no trace of them beyond the file: no line in the bell, nothing in the Dashboard or in the problem list, never saved in Pako's list, and the login is dropped as soon as the download ends. A finished one shows **Done · private** and leaves the list once you open it from Pako or click its **✕** (the file stays), or else 10 minutes after it finished. To keep it longer, pick **1 hour after** or **When you quit Pako** in **Settings › Downloads › Advanced › Remove private downloads from the list**. If Pako closes before one is finished, its unfinished file is deleted the next time Pako starts.

In Chrome, Edge and similar browsers, a private window's download is held paused while Pako checks it (like a file a form sent, above): if Pako can't get it, it carries on in the private window, never in your normal one. A Google Drive file from a private window is downloaded like any other file, with that window's own login.

Links sent from a private window show **private** with a lock in **Links**. They're kept only until you quit Pako (never saved), are never sent to another of your PCs with **Download on** ("Private downloads stay on this PC."), and download as private downloads. Private downloads also never show in **My devices** on your other PCs, so they can't be seen, paused, removed or fetched from there. **Record this video** isn't offered in private windows.

A download from your browser that starts right away shows a short pop-up, **Downloading in Pako**, and "all links on this page" shows **Links added** (a video sent to **Links** shows **Video sent to Pako**, a playlist **Playlist sent to Pako**). Turn these off with **Sent from your browser** in **Settings › Notifications** (see [Notifications](notifications.md)).

## The Download with Pako window

When a download comes from your browser, Pako shows a small **Download with Pako** window first:

- **Name**: change the file's name if you like.
- **Save to**: the folder. Click **Change** to pick another.
- **Quality**: for videos only.
- **Notify me when it's done**: Pako remembers your choice.
- **Don't ask again**: downloads from the browser then start right away. You can turn it back on in **Settings › Browser extension › Ask before each browser download**.

Click **Download** (or press **Enter**) to start, or **Cancel** (or **Esc**). When other downloads are already running, you get **Download now** and **Add to queue** instead. If several downloads arrive together, the window shows them one after the other ("1 of 3").

With [My devices](devices.md), a small arrow next to **Download** lets another of your PCs download it instead: **Download on Home PC**, for example. It's there only while one of your devices that lets this PC control it is online. That PC gets the link and downloads it into its own download folder, as **First** (like **Download now**), and a video in the quality chosen in this window, at once. The download leaves this PC; the name and folder chosen here aren't sent. If it didn't work, the window says why and the download stays here. Only the link is sent, never your browser's login, so a download that needs you to be signed in on this PC may not work there. **Download** itself always downloads on this PC.

With **Show the small window when a browser download starts** on (**Settings › Browser extension**), the tray window opens at the bottom right so you can watch the download.

## After a Pako update

Pako refreshes its extension folder by itself when it starts, and the extension picks up the new version by itself the next time it talks to Pako (a few seconds after, once nothing is on its way to Pako and nothing is being recorded). A browser can also keep running the extension it had before even after it restarts (seen in Vivaldi): opening the Pako button in the toolbar then makes it take the new one (the small window says "Updating Pako's extension…": open it again a moment later). You can also click the reload button on Pako in the browser's extensions page.

Firefox, Zen, LibreWolf, Waterfox and Floorp update Pako's extension by themselves, like their other add-ons (to check now: `about:addons`, the gear button, **Check for Updates**). If you turned off add-on updates in one of them, add it again with **Add to …** after updating Pako.

If the Download button then says "Pako's extension was just updated. Reload this page and try again.", reload the web page. If it says "Pako's extension isn't responding. Open the Pako button in the toolbar for details.", click the Pako button: its message says what the browser reported (also shown when you point at the button's message).

## Not connected

If the extension says **Not connected**, or the Download button says "Pako isn't connected":

1. Pako starts by itself, in the tray, when the extension sends it something. If it doesn't, start it yourself from its folder or the Start menu.
2. In Pako, check **Settings › Browser extension › Connect Pako to your browsers** is on.
3. Click the reload button on Pako in the browser's extensions page, then reload the web page.
4. Click **Open Pako** in the extension's small window: it opens Pako and connects it. Allow it if the browser asks.

Pako Light's small window also says where to get Pako, in case it isn't on this PC yet: **Get it at pako.sookera.com**.

If the small window says **This Pako is too old for this extension**, update Pako (**Settings › Features & updates**), then open the small window again.

A red **!** on the Pako button in the toolbar means something sent to Pako didn't go through: click the button to see what happened.

If it still fails, the Download button's message ends with "(Browser says: …)". That text tells exactly what's wrong; keep it when asking for help.

## Good to know

- The extension talks to Pako directly on your PC. Nothing goes over the internet.
- For **Connected** in Settings, Pako keeps only which browser it was, the edition (full or Pako Light), the extension's version when it says one, and the date, on this PC (`browsers_seen.json` in Pako's data folder).
- In private windows, the extension does nothing unless you allowed it there in your browser (see **Private windows** above); then Pako keeps no trace of anything sent from them beyond the file. Without that, nothing from a private window reaches Pako: the browser doesn't run the extension there.
- For **Videos on this page**, the full extension only looks at the video requests your browser makes (it never changes or blocks them). The list stays in the browser's memory, for that tab only, and nothing goes to Pako until you click. With a video, Pako gets its address, the page it played on, the page's title, and your browser's login for the video's own server only.
- Your browser's login for a site is passed to Pako only for the downloads it handles (ones you send, and browser downloads it takes over), kept encrypted, and deleted when the download is finished or removed.
