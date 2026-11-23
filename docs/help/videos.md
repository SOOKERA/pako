# Videos

Download videos from YouTube and many other sites, in the quality you choose, or just the sound.

## Before the first video

Videos need three free tools (yt-dlp, Deno and ffmpeg, about 600 MB on disk). If they're missing, a video link in **Links** shows a small download button in its **Link** column (**Needs the video tools**): click it. You can also install them in **Settings › Features & updates › Download videos from YouTube and other sites**. With **Update Pako and its tools automatically** on (same place), Pako keeps them up to date.

Pako installs the edition of yt-dlp that starts fastest (a folder with yt-dlp.exe inside, not the single file). A yt-dlp installed by an older Pako keeps working and switches at its next update: by itself with automatic updates on, else with **Update** under **What Pako can do**.

## Download a video

1. Copy the video's address and press **Ctrl+V** in Pako (or use the browser's **Download** button, see below).
2. The video appears in **Links** with its size for the chosen quality.
3. Select it to change the **Quality** on the right if you want, then click **Download**.

A playlist link lists every video in it, so you can tick the ones you want.

## Check it's the right video

Before downloading (or while it downloads), you can play a few seconds of the video to make sure it's the one you want:

1. Select the video in **Links** (or the unfinished download in **Downloads**).
2. Under **Preview** in the details on the right, click the play button.
3. After a moment, about 20 seconds play at 480p or lower, a few seconds into the video.

Nothing is fetched from the site until you click play. The clip stops when you pick another item or hide the details. It needs the video tools. Live streams can't be previewed, and a private, age-restricted or protected (DRM) video says **This video can't be previewed** and why.

## Quality and audio

- The **Quality** list offers the sizes the video really has (1080p, 720p…), plus **Audio only (M4A)** and **Audio only (MP3)**.
- **Default** uses **Settings › Downloads › Video quality**.
- If a video doesn't have the quality you picked, Pako takes the closest lower one.
- Click a playlist's header in **Links** to set the same quality for all its videos.

## The Download button on video players

Download and record only what you have the right to, and follow the rules of the sites you use.

With the full [browser extension](browser-extension.md) (not Pako Light), a **Download** button appears at the top right of a video when you point at it, on YouTube, Vimeo, Twitch, Facebook and Instagram:

1. Click **Download**. Pako starts reading the video right away, while you pick.
2. Pick **Up to 1080p**, **Up to 720p**, **Up to 480p** or **Audio only**.
3. The button says **Sent to Pako ✓**, and the video starts downloading in Pako. If **Ask before each browser download** is on, Pako first shows the **Download with Pako** window.

Pako starts reading the video's title and qualities as soon as you click **Download** (never when you only point at the video), so once you've picked a quality the video often starts at once. If you close the menu without picking, Pako stops and forgets that read as soon as you close it (or after two minutes at most), and nothing appears in Pako. The button answers in about a second. If Pako can tell at once that it can't get the video (the video tools aren't installed, or it's already in Pako), the button says why. Otherwise a pop-up says it once Pako knows, and the bell keeps it: **Pako couldn't get this video** (see [Notifications](notifications.md)). Clicking the button again while Pako is still reading the video says **Pako is already getting this video.**

On Facebook and Instagram, the button takes the video you're pointing at, even in a feed with many videos.

You can also right-click a video page › **Download video with Pako**, right-click a link to a video › **Download link with Pako**, or click **Send this page to Pako** in the extension's small window. The video then downloads in your usual quality (**Settings › Downloads › Video quality**), through the **Download with Pako** window if it's on. A playlist goes to **Links** instead, so you can tick the videos you want (the pop-up says **Playlist sent to Pako**).

## Private and logged-in videos

Videos sent from your browser (the button or the right-click menu) use your browser's login for that site, so videos you can watch while signed in can be downloaded too. The login is kept encrypted, used only for that download, and deleted afterwards. While the video downloads, the login sits in a small file only you can read, in Windows' temporary folder (`%TEMP%\Pako`); that file is deleted as soon as the download stops, and when Pako closes. Videos from private windows never get it.

## The video playing on any page

With the full [browser extension](browser-extension.md#videos-on-this-page), you can download the video playing on almost any page, not only on the sites above:

1. Play the video for a moment.
2. Click the Pako button in the browser's toolbar.
3. Under **Videos on this page**, click **Download with Pako** next to the video.

Or point at the video itself: once Pako has found it, the same **Download** button appears at its top right, as on the sites above. Click it, then **Download this video** (when Pako found several videos on the page, it lists them as the small window does: pick one). Its menu also has **Record this video** (below), and it offers nothing for a protected video. The button appears only on pages where Pako found a video; **Download button on videos** in the extension's small window turns it off too.

The video goes to **Downloads**, named after the page's title, in your usual quality (**Settings › Downloads › Video quality**), through the **Download with Pako** window if it's on. Pako gets it the same way the page does: from the same page, with your browser's login for the video's server. When its video tool can't get a video made of small pieces (an HLS stream), Pako tries again by itself, fetching the pieces itself; it continues from the pieces it already has after a pause or a restart.

If it doesn't work, the download says "Pako couldn't get this video." Then try **Record this video**.

## Record this video

The last resort, offered in the extension's small window (full extension), and in the **Download** button's menu on the video, while a video plays or is listed under **Videos on this page**, for when Pako couldn't get it:

1. Click **Record this video**. The page reloads.
2. Press play if the video doesn't start by itself. Pako records it as it plays, sped up and muted when the page allows it. If the video doesn't start within 5 minutes, recording stops: "The video didn't start playing, so nothing was recorded."
3. Keep the tab open: **the video has to play to the end**. The small window shows how much is recorded; **Stop recording** stops it and nothing is kept.
4. At the end, the small window says "Sent to Pako: it makes the video in Downloads." The video appears in **Downloads**, named after the page's title. If the video changed quality while it played, Pako makes it again at the best quality it had, which takes a little longer.

One video is recorded at a time, never in private windows, and never a protected video.

## Fast downloads

When a site sends a video as one big file (YouTube, Vimeo…), Pako downloads it in parallel parts, then joins picture and sound into one file. When a site sends a video as many small pieces (Twitch and most TV sites), Pako fetches many pieces at once.

## Good to know

- Sites that lock their videos (Netflix, Disney+, Prime Video and similar) can't be downloaded. Pako says "This site protects its videos (DRM), so they can't be downloaded." and never tries to get around it. For the video playing on a page, it says "This video is protected, so it can't be downloaded."
- Sites Pako recognises as video sites include YouTube, Vimeo, Dailymotion, Twitch, TikTok, X, Instagram, Facebook, Reddit, Bilibili, Streamable, Rumble, Odysee, BitChute and SoundCloud. Many others work too.
- The details panel of a video download shows its quality under **Quality**.
