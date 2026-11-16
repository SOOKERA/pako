# Browser extension

The Pako extension sends links, videos and downloads from your browser straight to Pako.

## Add Pako to your browser

Pako's extension works in Chrome, Edge, Brave, Vivaldi and Firefox.

### Chrome, Edge, Brave, Vivaldi

1. In Pako, open **Settings › Browser extension** and turn on **Connect Pako to your browsers**.
2. Under **Add Pako to your browser**, click **Add to Chrome** (or **Add to Edge**, **Add to Brave**, **Add to Vivaldi**). Its extensions page opens, and the extension's folder is copied for you (if Pako says it isn't, click **Copy** next to the folder).
3. On the extensions page, turn on **Developer mode**.
4. Click **Load unpacked**.
5. Click the address box at the top of the window that opens, press **Ctrl+V** and **Enter**, then **Select Folder**.

If the browser doesn't open its extensions page, Pako opens the folder instead and tells you which address to type in the browser. **Open the extension folder** opens it any time.

### Firefox

Firefox only accepts the extension once it's signed by Mozilla (free, done once). With the signed file (`pako-firefox.xpi`) in the extension folder, click **Add to Firefox**, then **Add** in Firefox. Without it, Pako says so and **Show the steps** explains how to get it signed. So that downloads behind a login work, allow it on all websites: **about:addons › Pako › Permissions**.

### Pako Light

Pako Light is a smaller edition of the extension, made for the Chrome Web Store, with a teal icon. It takes over your downloads and has the right-click items, but it doesn't download videos: no **Download** button on videos, and **Download with Pako** works on links and pictures only. It also does nothing on video sites: there, **Send this page to Pako** (in its small window and in the right-click menu) and **Send all links on this page to Pako** are greyed out, its small window says "Pako Light doesn't work on this site.", and links to video sites are left out of "all links". Pako itself also refuses videos sent from Pako Light. The full extension, with the **Download** button on videos, comes with Pako (see above).

Only one extension takes over your downloads, so a download goes to Pako only once. If the full extension is on in the same browser, Pako Light steps aside: no right-click items of its own, and its small window says **Not in use** and **The full Pako extension is on in this browser**. You can keep both, or remove Pako Light.

## Check it's connected

Click the Pako icon in the browser's toolbar. The small window says **Connected**. If it says **Not connected**, see [Not connected](#not-connected) below.

The same window lets you:

- **Send this page to Pako**.
- Turn **Let Pako take over downloads** on or off.
- Turn the **Download button on videos** on or off (full extension only).

The small window follows your browser's light or dark mode.

## The Download button on videos

With the full extension, on YouTube, Vimeo, Twitch, Facebook and Instagram, a **Download** button appears at the top right of a video when you point at it. Click it and pick **Up to 1080p**, **Up to 720p**, **Up to 480p** or **Audio only**. See [Videos](videos.md).

## Right-click menu

- On a link, picture, video or sound: **Download with Pako**. It goes straight to **Downloads** (through the **Download with Pako** window, if it's on). A link to a video page downloads the video, in your usual quality. Pako Light has this item on links and pictures only.
- On a page: **Send this page to Pako**. In the full extension, on video sites this item reads **Download video with Pako**: the video downloads in your usual quality, and a playlist goes to **Links** (see [Videos](videos.md)).
- On a page: **Send all links on this page to Pako (to pick from)**. They land in **Links**, where you tick what you want.

The items follow the tab in front, also when you switch to another browser window.

## Downloads from the browser

With **Let Pako take over downloads** on, files you download in the browser go to Pako instead, for example when you click a Download button on Google Drive or any site.

- Small files (up to 2 MB) stay in the browser.
- Pako only takes a download over once it has checked it gets the same file. Otherwise the browser keeps it.
- Downloads from private windows, and files the browser flags as unsafe, stay in the browser.
- Downloads from video sites stay in the browser too. For those videos, use the **Download** button or the right-click menu.

A download from your browser that starts right away shows a short pop-up, **Downloading in Pako**, and "all links on this page" shows **Links added**. Turn these off with **Sent from your browser** in **Settings › Notifications** (see [Notifications](notifications.md)).

## The Download with Pako window

When a download comes from your browser, Pako shows a small **Download with Pako** window first:

- **Name**: change the file's name if you like.
- **Save to**: the folder. Click **Change** to pick another.
- **Quality**: for videos only.
- **Notify me when it's done**: Pako remembers your choice.
- **Don't ask again**: downloads from the browser then start right away. You can turn it back on in **Settings › Browser extension › Ask before each browser download**.

Click **Download** (or press **Enter**) to start, or **Cancel** (or **Esc**). When other downloads are already running, you get **Download now** and **Add to queue** instead. If several downloads arrive together, the window shows them one after the other ("1 of 3").

With **Show the small window when a browser download starts** on (**Settings › Browser extension**), the tray window opens at the bottom right so you can watch the download.

## After a Pako update

Pako refreshes its extension folder by itself when it starts. Your browser picks up the new version the next time it starts, or right away if you click the reload button on Pako in the browser's extensions page.

If the Download button then says "Pako's extension was just updated. Reload this page and try again.", reload the web page.

## Not connected

If the extension says **Not connected**, or the Download button says "Pako isn't connected":

1. Make sure Pako is running. Start it yourself from its folder or the Start menu.
2. In Pako, check **Settings › Browser extension › Connect Pako to your browsers** is on.
3. Click the reload button on Pako in the browser's extensions page, then reload the web page.
4. Click **Open Pako** in the extension's small window: it opens Pako and connects it. Allow it if the browser asks.

If the small window says **This Pako is too old for this extension**, update Pako (**Settings › Features & updates**), then open the small window again.

A red **!** on the Pako button in the toolbar means something sent to Pako didn't go through: click the button to see what happened.

If it still fails, the Download button's message ends with "(Browser says: …)". That text tells exactly what's wrong; keep it when asking for help.

## Good to know

- The extension talks to Pako directly on your PC. Nothing goes over the internet.
- Your browser's login for a site is passed to Pako only for downloads you send, kept encrypted, and deleted when the download is finished or removed.
