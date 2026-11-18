# My devices

Pair your own PCs once, like your desk PC and your laptop. Then, from the **My devices** page:

- send files between them without anyone pressing Accept;
- see the other PC's downloads live, pause, resume, change their priority or remove them, and get a finished one;
- send links from one PC for the other to download, with the video quality and priority you pick, and follow how they're doing.

**Direct** stays for other people. **My devices** is only for your own PCs.

## How it works

- My devices uses the same connection as [Direct](direct.md): files and messages go PC to PC, encrypted, and nothing goes through a Sookera server.
- Both Pakos must be open at the same time (in the tray counts). Orders and files can't wait for a PC that is off.
- On the same Wi-Fi or home network, your PCs find each other without the internet and files go over that network, usually much faster. The device's dot turns blue and it says **Local network**.
- Opening **My devices** starts Direct if it wasn't running yet, like opening the Direct page.
- A device doesn't show among Direct's contacts.

## Pair a PC

Do this once for each pair of PCs.

1. On the first PC, open **My devices** and click **Add a device** (or **+ Add**). It shows **This PC's Pako ID**: click **Copy**, and get the ID to your other PC (an email to yourself, a chat, a USB stick).
2. On the other PC, open **My devices** › **Add a device**, paste it under **The other PC's Pako ID**, give it a name (like **Desk PC**) and check **Files from it are saved in** (see below). Click **Pair this device**.
3. The first PC shows "… wants to pair with this PC as one of your devices", with a **safety code**. The other PC shows the same code under its name. If the codes are the same, check where its files go and click **This is my device**. If they aren't the same, click **Decline**.

   The card also says what pairing allows, and warns you if one of your devices or contacts already has that name: if you didn't just add this PC on your other one, decline. If this PC has no name yet, the card asks for it.

You can also enter the IDs the other way round: whichever PC enters the other's ID, the other one asks you to confirm. Pako asks for a name for this PC first if it doesn't have one yet (your other PC sees it).

## This PC's name

At the bottom of the device list, under **This PC**, is the name your other PCs see for this one, like **Desk PC**. It's only for your devices: the name your contacts see in Direct (your name under **You, sending from**) stays as it is.

- To rename this PC, click its name, type the new one and press **Enter** (or click elsewhere). **Esc** keeps the old name.
- Your other PCs see the new name at once when they're connected, otherwise the next time they connect.
- If you renamed a device yourself on a PC (**Rename** in its **Settings**), that PC keeps your name for it.
- Until this PC has a name, it says **Name this PC**, and pairing asks for one.

If one of your Direct contacts is really your own PC, add its ID under **Add a device** on both PCs: it moves to **My devices**.

## Where files from a device are saved

Each device has its own folder. It's chosen when you pair: by default `<your download folder>\From <device name>`, and **Change…** picks another one.

**Files from this device are saved here, and only here.** The other PC never chooses a place on this PC: it can't write anywhere else, can't see your folders, and names that try to climb out of the folder are refused, as in Direct. You can change the folder later in the device's **Settings**.

## Send files between your devices

Pick the device on the left. Its page has four tabs: **Files**, **Links**, **Downloads** and **Settings**.

Its **Files** tab shows everything sent between the two PCs, with progress, speed, and the way it travels (**Local network**, **Direct** or **Via relay**).

- **Drop files or folders** anywhere on the device's page, or on the device in the list, or click **Choose files** / **Choose folder**. They go at once.
- The other PC doesn't ask: files from your own devices come in by themselves, into that device's folder. (If a send is very big for the drive there, it still asks.)
- **Cancel** stops a file on its way. **Open** and **Folder** show what you received. Programs and scripts are never started from Pako: they're shown in their folder.
- The arrow on a card of several files or folders unfolds what was in it, each with its size (and **Open** / **Folder** on each one you received).
- If the connection drops (a PC sleeps, the Wi-Fi goes, Pako is closed), the file waits: "Waiting for Home PC to continue · 62% done". It continues by itself from where it stopped when both PCs are connected again, as in [Direct](direct.md#your-history-with-someone). **Cancel** deletes what had arrived; after 7 days it's cleared.

## See and control the other PC's downloads

Open the device's **Downloads** tab. While it's open and the device is online, you see its downloads live: name, size, progress, speed and status.

- **Pause** and **Resume** work on that PC's download.
- The priority button (an arrow, or a dash for Normal) changes its priority there: **Priority: First**, **Normal** or **Later**, as in Downloads. A download set to First or Later says so on its line. If that PC has an older Pako, it answers that it doesn't know this order: update Pako there.
- **×** removes it from that PC's list. A finished file stays on that PC.
- **Get this file** (finished downloads only) sends the file to this PC, like a normal send, into that device's folder here.

## Send links to download there

The device's **Links** tab shows the links this PC sent it to download, one card per send, newest at the bottom. Sends from the **Links** page and from the **Download with Pako** window get a card here too.

- Each card says how many links, their priority and quality, when they were sent and where they go: "3 links · Normal · Best up to 1080p", "Sent today 14:02 · into Downloads".
- While that PC is online, the next line says how they're doing there, from its download list: "2 done · 1 downloading 64%". A PC with an older Pako doesn't say which downloads it made: its cards just say **Sent**.
- **Show the links** lists them, with the same buttons as the **Downloads** tab (pause, resume, priority, remove, **Get this file**).
- This PC keeps the last 200 sends for each device.

To send more, use the box at the bottom:

1. Paste one or more links in **Paste links for Home PC to download** (for example).
2. Pick the **Quality** for videos (**Best up to 1080p** by default, as in **Settings › Downloads**, a height, or **Audio only**) and the **Priority** (**First**, **Normal** or **Later**).
3. **Into folder** is optional: a folder name, made inside that PC's own download folder (for example **Course**, or **Course/Week 3**; `\` works like `/`). A name that can't be used there (with `..`, a drive or `:`, or more than two levels) isn't sent: the line under the box says why.
4. Click **Send** (or press **Enter**).

Files and single videos go at once. The line under the box says what happened there ("Added 2 links to the downloads on Home PC"), or why not.

A **playlist**, a **Google Drive folder** or a **link page** Pako must click through is never sent as it is: this PC opens it first, in a card in the tab ("Reading the playlist…"). Then the card lists its videos or files with tick boxes, each video with its own quality (the box's quality to start with). Untick what you don't want and click **Send 5 to Home PC** (for example), or **Cancel**. Each ticked video or file goes as its own link, into a folder named after the playlist or Drive folder there (or **Into folder**, if you filled it in). If it can't be opened, the card says why. If that PC doesn't take some of them, those stay in the card with why, to send again.

Only web links (http and https) can be sent. Everything else is refused. Links to the PC itself (localhost) are refused too. Only the link goes, with its quality, priority and folder name: never your browser's login or cookies.

## What the other PC does with them

- **A video** from YouTube or another video site downloads at once, in the quality picked, or the closest lower one that video has, like a normal video download. The **Download with Pako** window doesn't open there, and it doesn't wait in **Links**.
- **A file** goes straight to its downloads, with its priority.
- Everything goes into that PC's own download folder, or into the folder named for it **inside** that folder. **The other PC can only add a folder inside its download folder**: never anywhere else. A folder name is cleaned like a file name (a reserved Windows name like `CON` gets a `_` in front). One that isn't safe (like `..`, `C:\`, a network path, or more than two levels) is refused by the PC that sends it; if one still arrives, it isn't used: those files go into the download folder itself, and its history says so. A file with the same name as one already there gets a new name, as usual; nothing is overwritten.
- A video whose site is slow to answer may still be read after that PC replied: it counts as taken. If it then can't be downloaded, it stays there as a failed download (the card here says **failed**; **Resume** tries again).
- Many links at once (a large Drive folder): what that PC couldn't add within about 40 seconds comes back as not taken ("Pako there was still busy adding the others: send it again"), so nothing is added twice.
- A link from a PC with an older Pako works as before: a video's page or a link page goes to **Links** on that PC first.

## Download on another PC from Links

Links you have on this PC can be downloaded by another of your PCs. Two ways:

- **For one time**, with **Download on** next to **Download**: in **Links**, the button with the PC icon in the toolbar, the small arrow next to **Download** in a link's details, or right-click › **Download on …**; in the **Download with Pako** window, the small arrow next to **Download** (see [Browser extension](browser-extension.md#the-download-with-pako-window)). It lists **This PC** and each of your devices that is online and lets this PC control it; when none is, it isn't shown. The choice counts for that one time only, whatever the link's own setting says.
- **For a link, remembered**: select it in **Links** and set **Download on** in its details (see [Links and the tray window](links-and-tray.md#download-on-another-pc)). Then **Download** sends it to that PC.

Each link goes with:

- its **quality**, for a video: the one picked on its row in **Links** (or **Audio only**), the one chosen in the **Download with Pako** window, or else this PC's own default (**Settings › Downloads**);
- its **priority**: the one set on its row, or **First** with **Download first** (and from the **Download with Pako** window, like its **Download now**);
- for a video of a playlist or a file of a Drive folder opened in **Links**: the playlist's or folder's name, so it lands in a folder of that name there.

A playlist opened in **Links** is sent video by video (only the ticked ones), and a Drive folder file by file. A playlist whose videos aren't listed yet, or a link page Pako hasn't opened yet, isn't sent: it stays in **Links** with a note on its row ("A playlist: open it in Links first to pick the videos."). A file on several hosts is sent as the host it would use here.

Once that PC took them, they leave **Links** here, and a message says so ("Added 2 links to the downloads on Home PC"). A link it couldn't take, or links for a PC that's offline, stay here with a note on the row saying why. A link that only works while you're signed in on this PC may not work there: only the link is sent, never your browser's login.

## This device can control my Pako

In the device's **Settings** tab, **… can control my Pako** says whether that PC may see this PC's downloads and give the orders above. It's on when you pair, and you can turn it off for each device. Turned off, its very next order is refused and it stops seeing your downloads. Files between the two PCs still go without asking.

What a device you allow can do here, and nothing else:

- see this PC's download list (never its private downloads from a browser's private window: a device can't see, change or get them, and a link it sends never joins one, even for the same video);
- add web links to the downloads, with a video quality and a priority, into this PC's own download folder or a folder it names inside it;
- pause, resume, change the priority of, or remove a download from the list;
- get a finished download from that list.

It can never open or run anything on this PC, pick a folder, or reach a file that isn't a finished download of your list. If the other PC doesn't allow this PC, its **Downloads** tab says so and how to turn it on there.

The same permission lets the other PC send to your Direct contacts through this one (see [Send from another of your PCs](direct.md#send-from-another-of-your-pcs)): it may see this PC's Direct contacts and your history with them, and send files that this PC then offers to one of them as a send of its own. Only the files it sends in that send are passed on, kept meanwhile in Pako's own data here (never your folders), and deleted once delivered (or kept in this device's folder, if it asked), declined, or after 7 days. Turning this permission off (or removing the device) stops at once what it asked this PC to pass on, and deletes those files.

## History

The device's **Settings** tab lists what the two PCs did with each other: links added, downloads paused, removed or given another priority, files asked for, orders refused, permission changes, and files sent to a Direct contact through the other PC ("Relayed 3 files to Rado · delivered"), with who did it and when. It stays on this PC. **Clear** deletes it from this PC.

While files go to a Direct contact through a device, that device's line in the list says so: "Relaying 3 files to Rado". In its **Files** tab, that send says who it's for ("for Rado").

## Rename or remove a device

In the device's **Settings** tab: **Rename** changes the name you see on this PC. **Remove this device** ends the pairing: it can no longer send without asking or control this Pako, and the connection closes. Files already received stay. To pair again later, add it again under **Add a device** on this PC; the other PC confirms. If the other PC still shows it as one of its devices, remove it there too first, then add it again there.

## Questions

**The device says "Confirm on the other PC".**
Pairing needs a yes on both PCs. Open **My devices** on the other PC: it asks there. Both Pakos must be open.

**The Downloads tab says the other PC doesn't let this PC see its downloads.**
On the other PC, open **My devices**, pick this PC, then **Settings**, and turn on **… can control my Pako**.

**Can a contact in Direct control my Pako?**
No. Only a PC that both sides paired as "my device", and that you allow, can see your downloads or give orders. Contacts can only offer you files, which you accept or decline.

**One of my Direct contacts says this PC is one of its devices.**
Only click **This is my device** if that contact is really your own PC: once paired, it can send files here without asking, see and add to this PC's downloads, and get finished ones. **Not now** hides the question until Pako restarts.

**The device stays "Connected · not confirmed on it yet".**
The other PC has an older Pako that doesn't know My devices: update Pako there, then add this PC there under **Add a device**.
