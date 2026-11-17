# Links and the tray window

Pick what to download from the links you paste, and keep an eye on your downloads from the taskbar.

## Links

Links you paste or drop, and the ones from **Send all links on this page to Pako (to pick from)** in your browser, first land in **Links**. (A single download from your browser goes straight to **Downloads**.) Pako checks each one (name, size, whether it works), then you pick what to download. Pako switches to **Links** by itself when you paste, so you always see where they went.

### Pick and download

1. Tick the links you want (or **Tick all**, **Ctrl+A**; **Untick all**, **Esc**).
2. Click **Download**.

The button says **Download** when there's room to start now, and **Add to queue** when the downloads already running leave no room (see **At once** in [Downloading](downloading.md)). Then **Download first** also appears: the downloads that started last step aside (nothing is lost) and yours start now. A download that couldn't continue where it stopped (its server doesn't allow it) never steps aside: yours waits for a free spot instead.

The small arrow next to the button has more choices: **Download and notify me**, or, when the button says **Add to queue**, **Add to queue and notify me** and **Download first and notify me**.

**Save to** (at the top) picks the folder the links download to. A link can have its own folder: select it and change **Save to** in its details, or right-click it › **Save to…**.

Right-click a link for **Download**, **Download first**, **Download and notify me**, **Download on …** (your other PCs, see below), **Notify when done**, **Priority: First**, **Normal** or **Later**, **Save to…** and **Remove**.

### Download on another PC

With [My devices](devices.md), another of your PCs can download links instead of this one.

**For one time:** click **Download on** (the button with the PC icon, next to **Download**), then the PC: **Download on Home PC**, for example. In a link's details, the small arrow next to **Download** has the same choice, and so does right-click › **Download on …**. It's shown only while one of your devices that lets this PC control it is online. **Download on this PC** in that menu downloads them here, whatever their own setting says. The choice counts for that one time only.

**For a link, remembered:** select it and set **Download on** in its details: **This PC** (the default), or one of your devices that lets this PC control it (an offline one shows "(offline)"). Its row then shows a small PC mark with the PC's name. **Download**, **Download first** and **Download and notify me** download the links set to this PC here, and send the others to their PC, one send per PC. If that PC is unpaired, or no longer lets this PC control it, the link goes back to **This PC**, with a note on its row. A link from a private browser window (marked **private** with a lock) is never sent to another PC: "Private downloads stay on this PC."

What goes with each link:

- a video's **Quality** as picked in its details (or **Audio only**); when it's on the default, this PC's default (**Settings › Downloads**);
- its **Priority** (**First** with **Download first**);
- for a video of a playlist or a file of a Drive folder, the playlist's or folder's name: it lands in a folder of that name inside that PC's download folder.

That PC downloads them into its own download folder: a video at once in that quality (or the closest lower one), without asking there. A playlist is sent video by video, only the ticked ones; one whose videos aren't listed yet, or a link page Pako hasn't opened yet, isn't sent and keeps a note on its row. A file on several hosts goes as the host it would use here.

Once that PC took them, they leave **Links** here, and a message says so: "Added 2 links to the downloads on Home PC." Links that PC couldn't take, or links for a PC that's offline, stay here with a note on the row saying why. Every send also shows in **My devices** › that PC › **Links**.

### What you see

- **Folders**: a Drive folder or a playlist shows as a block. Click it to open or close it, and tick the box on its header to pick every file in it.
- **Added on**: when each link was added. Click a column title to sort by it.
- **Split archive** sets (.part1.rar, .7z.001, .zip.001…) are grouped, and **Incomplete set** warns when parts are missing.
- **Ready · Google limit today**: a Drive file that can be downloaded, but Google's daily limit for it is reached. See [Google limit](downloading.md#google-limit-too-many-downloads-today).
- Select a link to see its details on the right: type, size, and for videos the **Quality**. You can also set its **Save to** folder, **Priority**, **Notify when done** and (with My devices) **Download on** there, before it downloads.
- A link to a video file (.mp4, .mkv…) also gets a **Preview** in its details: play a short check that it's the right video before downloading it (**Settings › Downloads › Advanced › Video preview quality**). MP4 and WebM play as they are; other types, and the preview quality, need **Preview more video types** in **Settings › Features & updates**.
- A YouTube (or other video site) link gets a **Preview** too: click play for about 20 seconds at 480p, just to check it's the right video (see [Videos](videos.md)).

### Same file on several hosts

Sometimes the same file is offered on several sites. When links in **Links** point to the same file (the same name and the same size, once they're checked), they become one row with **on 3 hosts** (for example) next to its name. A link still being checked joins the row when its check ends. Two links from the same site, Drive folders and video sites are never put together.

- The row shows the host it will download from (in **Source**, and at the top of its details).
- Click **on 3 hosts** (in a small window it shows only the number of hosts), or select the row and look at **Hosts** in its details: each host with **Ready**, **Broken** or **Checking**, and its size. Click a host to download from it. A broken host can't be picked.
- Pako picks a ready host by itself: the one you picked last time for another file, otherwise the link you added first.
- **Download** adds one download, not one per host. The other ready hosts go with it as backups.
- If its host fails for good (the file is gone, the host's daily limit is reached, or it sends a web page instead of the file), Pako goes on from the next ready host by itself, and says so: "Holiday.zip: continuing from …".
- A dropped connection or a busy server is tried again on the same host first (see [Downloading](downloading.md#fast-and-steady)). But when the host gives no answer at all for 5 minutes in a row (the site is down, refuses connections, or stays busy), Pako goes on from the next host: "Holiday.zip: continuing from B (A didn't answer for 5 minutes)". That host goes to the end of the backups, so it's tried again after the others. Pausing and resuming starts the 5 minutes again. A download without backups keeps trying its host, as before.
- On the new host, Pako keeps what it has already downloaded when it can be sure it's the same file:
  - the same size and the same file tag or checksum: it carries on right away ("continuing from B where A stopped");
  - the same size, and the new host can send a file part-way: Pako first downloads a few small pieces from the new host (near the start, the middle and the end of what it has) and compares them with the file so far. All the same: it carries on where A stopped. Any difference: it starts again from zero and says why, for example "B has a different version of the file: starting over". A piece it can't get right now (no connection, a busy server) is asked again a little later, and what it has stays; if B gives up for good meanwhile, or doesn't answer for 5 minutes, Pako goes on to the next host and compares there;
  - another size, or a host that can't send a file part-way: it starts again from zero.

  Bytes from two hosts are never mixed on a guess. A Google Drive file is still checked as a whole at the end (when **Check Drive files when finished** is on, in **Settings › Downloads › Advanced**); if that check fails after changing host, Pako starts that file again from zero on the same host and says so.
- After Google's daily limit, Pako tries again by itself on the same host every few hours; **Try again** goes round all the hosts again.
- A split archive on several hosts stays one set: its parts on several hosts show **on 2 hosts**, and **Hosts** in the set's details picks one host for all of them.

### Filters

On the left:

- **Everything**: all links.
- **Ready**: checked and ready to download.
- **Only in the browser**: Google only gives these to certain accounts. Click **Open** on the link (or select it and click **Open in browser**) to download it there; Pako catches the download.
- **Link broken**: the link doesn't work. **Clear broken** removes them all.
- **Checking**: still being checked.

### Link pages

Some links lead to a web page instead of the file: a page you have to click through, for example. Pako opens any such page in a hidden window of its own and looks for the file behind it, working through common "click to continue" pages by itself. The link shows "Opening link…" meanwhile. When a page needs you (a "I'm not a robot" check, for example), the link shows **Waiting for you** and a **Pako needs you** pop-up appears (see [Notifications](notifications.md)): click **Show page** (or **Show check**), finish it, and Pako carries on.

The page runs in that window as it would in your browser, ads included, but without your browser's logins. It's a private window: its cookies and other site data are gone when it closes, and nothing is kept on your PC. Each link page starts fresh, so a site's check may ask you again for each link.

### Good to know

- Already downloaded? Pako asks before downloading a file twice: **Download again** (a second copy with a new name), **Skip it** (only when some of the links are new: they download, the others don't) or **Cancel**.
- **Remove** takes ticked links off the list (**Delete** removes the one you're on). Nothing is downloaded or deleted. For a few seconds, **Undo** (or **Ctrl+Z**) brings them back.

## The tray window

Pako lives in the taskbar's tray, next to the clock. Its icon gives you a small window to watch your downloads:

- **One click** on the tray icon opens or closes it. **Double-click** opens Pako's main window. **Right-click** gives a short menu: **Open Pako**, **Settings…**, **Pause all**, **Resume all** and **Quit Pako**.
- It shows what's downloading, waiting or paused, the ones that stopped today (failed, **Google limit**, **Only in the browser**), and what finished **today**, with **Open** and **Folder**. Older ones stay in the main window. When there are many, it says "and 5 more in Pako" (for example).
- When nothing is left to do and something finished, it says **All done**. With nothing at all, it says "Nothing downloading."
- Pause or resume one download with its button, or everything with **Pause all** / **Resume all**. **Open Pako** opens the main window, and the gear opens **Settings**.
- At the top, a switch shows **Downloads** or **Uploads**. **Uploads** lists what's uploading and your latest uploads, with a button to copy each one's link (see [Upload](upload.md)).

### Pin, minimise, close

- **Always on top** (the pin) keeps the window open where you put it, even when you click somewhere else. Drag it anywhere; Pako remembers the spot.
- Without the pin, the window closes when you click somewhere else.
- **Minimize** and **Close** hide it. One click on the tray icon brings it back.

While the window is open, a finished download doesn't get a **Download finished** pop-up too: you see it finish here (see [Notifications](notifications.md)).

With **Settings › Browser extension › Show the small window when a browser download starts** on, it opens by itself when a download arrives from your browser (at the bottom right, or where you pinned it), without taking you away from the browser.
