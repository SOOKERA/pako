# Videos

Download videos from YouTube and many other sites, in the quality you choose, or just the sound.

## Before the first video

Videos need three free tools (yt-dlp, Deno and ffmpeg, about 200 MB). If they're missing, a video link in **Links** shows **Needs the video tools**: click **Install** next to it. You can also install them in **Settings › Features & updates › Download videos from YouTube and other sites**. With **Update Pako and its tools automatically** on (same place), Pako keeps them up to date.

## Download a video

1. Copy the video's address and press **Ctrl+V** in Pako (or use the browser's **Download** button, see below).
2. The video appears in **Links** with its size for the chosen quality.
3. Select it to change the **Quality** on the right if you want, then click **Download**.

A playlist link lists every video in it, so you can tick the ones you want.

## Quality and audio

- The **Quality** list offers the sizes the video really has (1080p, 720p…), plus **Audio only (M4A)** and **Audio only (MP3)**.
- **Default** uses **Settings › Downloads › Video quality**.
- If a video doesn't have the quality you picked, Pako takes the closest lower one.
- Pick several videos at once to set the same quality for all of them.

## The Download button on video players

With the full [browser extension](browser-extension.md) (not Pako Light), a **Download** button appears at the top right of a video when you point at it, on YouTube, Vimeo, Twitch, Facebook and Instagram:

1. Click **Download**.
2. Pick **Up to 1080p**, **Up to 720p**, **Up to 480p** or **Audio only**.
3. The button says **Sent to Pako ✓**, and the video starts downloading in Pako. If **Ask before each browser download** is on, Pako first shows the **Download with Pako** window.

If Pako can't get the video, the button says why. When finding out takes longer, a pop-up says it instead: **Pako couldn't get this video** (see [Notifications](notifications.md)).

On Facebook and Instagram, the button takes the video you're pointing at, even in a feed with many videos.

You can also right-click a video page › **Download video with Pako**, right-click a link to a video › **Download with Pako**, or click **Send this page to Pako** in the extension's small window. The video then downloads in your usual quality (**Settings › Downloads › Video quality**), through the **Download with Pako** window if it's on. A playlist goes to **Links** instead, so you can tick the videos you want (the pop-up says **Playlist sent to Pako**).

## Private and logged-in videos

Videos sent from your browser (the button or the right-click menu) use your browser's login for that site, so videos you can watch while signed in can be downloaded too. The login is kept encrypted, used only for that download, and deleted afterwards. While the video downloads, the login sits in a small file only you can read, in Windows' temporary folder (`%TEMP%\Pako`); that file is deleted as soon as the download stops, and when Pako closes. Videos from private windows never get it.

## Fast downloads

When a site sends a video as one big file (YouTube, Vimeo…), Pako downloads it in parallel parts, then joins picture and sound into one file. When a site sends a video as many small pieces (Twitch and most TV sites), Pako fetches many pieces at once.

## Good to know

- Sites that lock their videos (Netflix, Disney+, Prime Video and similar) can't be downloaded. Pako says "This site protects its videos (DRM), so they can't be downloaded." and never tries to get around it.
- Sites Pako recognises as video sites include YouTube, Vimeo, Dailymotion, Twitch, TikTok, X, Instagram, Facebook, Reddit, Bilibili, Streamable, Rumble, Odysee, BitChute and SoundCloud. Many others work too.
- The details panel of a video download shows its quality under **Quality**.
