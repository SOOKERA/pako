# Notifications

Pako tells you things in two ways: pop-ups at the bottom right of the screen, and the bell at the top of Pako's window, which keeps the important ones. A download you mark **Important** gets a bigger pop-up in the middle of the screen.

## When a download finishes: Off, Pop-up, Important

Each download has its own choice, **Notify when done**:

- **Off**: no pop-up and no sound when it finishes.
- **Pop-up** (the usual): a pop-up at the bottom right and a short chime.
- **Important**: a big pop-up in the middle of the screen you're using, with a chime. It stays until you click it, so you can't miss it, and never takes your keyboard away from what you're typing.

Change it in one click wherever you see it:

- the bell at the end of the download's row: each click goes one step on (**Pop-up**, **Important**, **Off**). For **Pop-up** it only shows while the mouse is on the row; **Important** (filled, in Pako's colour) and **Off** (crossed out) always show;
- the download's details, under **Settings**;
- right-click: **Notify: Off**, **Notify: Pop-up**, **Notify: Important**;
- in **Links**: **Off**, **Pop-up** or **Important** beside **Download** (in a narrow window, in the arrow next to it), the bell at the end of each link's row, or **Download as Important** in a link's right-click menu;
- the **Download with Pako** window (see [Browser extension](browser-extension.md)).

New downloads start with **New downloads** in **Settings › Notifications** (**Pop-up** at first).

The **Important** pop-up gets through Do not disturb, Pako's or Windows'. It never shows over something full screen (a game, a full-screen video, a presentation): it waits until that ends, then shows, and it steps aside if something goes full screen while it's up. Several finishing together make one card ("3 downloads finished", with **Show in folder** when they're in one folder). Cards that come later wait their turn ("1 more after this", **Close all**). Click it to open Pako on that download, or use **Open**, **Show in folder** or **×**. One that never got to show (Pako closed first) is kept in the bell.

A folder or a split archive tells you once, as its loudest file is set. A split archive Pako extracts by itself tells you when it's done extracting: **Downloaded and extracted**.

**Off** is only about finishing: a download that stops (an error, Google's limit…) gets the **Download stopped** pop-up as Settings say, whatever its choice; an **Important** one always gets it.

## Pop-ups

Pop-ups are small cards at the bottom right of the screen, just above the taskbar. They never take you away from what you're doing: the app you're in stays in front. They follow Pako's light or dark look (see [Light or dark](downloading.md#light-or-dark)).

Click a pop-up to open Pako on the right page (the download, the upload, the person in Direct…). The **×** closes it. Some have buttons:

- A finished download: **Open** and **Show in folder** (also on the **Important** one in the middle of the screen).
- A finished upload: **Show in Upload**.
- Files someone sends you: **Decline** and **Accept**.
- Someone who wants to connect: **Review**. It opens Pako, where you compare the safety code (see [Direct](direct.md)). For a PC that wants to pair as one of your devices, it opens **My devices**.

Up to three show at once, newest at the bottom. When more are waiting, the stack says "2 more waiting" (for example).

### How long they stay

- Most leave by themselves after about six seconds. While the mouse is on them, they wait, and stay at least a few more seconds after it leaves.
- Questions stay until you answer or close them: files waiting for your answer, someone who wants to connect, and **Pako needs you** (a link page that needs you, such as an "I'm not a robot" check). They also leave by themselves once the question is settled somewhere else: answered in Pako, the link page finished or closed, or the other person gave up.
- The same link asking again replaces its earlier pop-up.

Some news doesn't pop up when Pako's window shows it already: an upload that finishes or fails while the window is open, and the news of [Direct](direct.md) and [My devices](devices.md) while Pako's window is in front on that page. On any other page, or with Pako behind another app, it pops up: files coming in from someone who can send without asking (or from your other PC), files received, and **Files delivered** when your send reaches them (also when it went through your other PC). Files offered to you and someone asking to connect are asked inside Pako's window on any page, so they pop up only while Pako is behind another app or hidden. A finished download set to **Pop-up** doesn't pop up while the tray window (the small window from the tray icon) is open on your screen: you see the download finish there. **Important** always shows. Failed downloads and other news still pop up, and the bell still gets the line if its **Bell** switch is on.

A download in [Private mode](private-mode.md) never shows its name: its pop-up (also the **Important** one) and its line in the bell only say "A private download finished" (or stopped).

## The bell

The bell at the top of Pako's window keeps the important things for 30 days. A count on it shows how many are unread, and the tray icon gets a dot too.

By default, it keeps:

- **Someone wants to connect**
- **Files waiting for your answer**
- **Pako needs you**
- **New Pako ready**: a new version is downloaded, with **Restart now**
- **Couldn't get a video**: a video sent from the player's **Download** button that Pako couldn't get (the button had already said **Sent to Pako ✓**)

Other kinds can go in too: turn on their **Bell** switch in **Settings › Notifications**.

Click the bell to see the list, grouped by **Today** and **Earlier**:

- Click a line to open Pako on the right page. It then counts as read.
- Files waiting for your answer have **Decline** and **Accept** right there (the files go to that contact's usual folder). Someone who wants to connect has **Review**.
- Once a question is settled, its line says how: **Accepted**, **Declined**, **Solved**, **Done**, **Not finished** or **No longer waiting**.
- **Mark all as read** clears the count and the tray dot. **Clear** empties the list.
- The gear opens **Settings › Notifications**.

Lines older than 30 days go by themselves, and the bell keeps the newest 300 at most.

## Settings › Notifications

At the top, under **When a download finishes**, two choices:

- **New downloads**: **Off**, **Pop-up** or **Important**, where every new download starts. Each download can still change it. If you had turned off the **Download finished** pop-up in an older Pako, this starts at **Off**.
- **Chime when a download finishes**: **Pop-up and Important** (at first), **Important only**, or **Never**.

Under **Each kind of news**, each kind of notification has two switches: **Pop-up** and **Bell**. **Download finished** has only **Bell**: its pop-up is chosen per download (it says **Per download**). They're grouped:

- **Downloads**: **Download finished**, **Download stopped**, **Sent from your browser**, **Couldn't get a video**.
- **Upload**: **Upload finished**, **Couldn't upload**, **Upload deleted on its date**.
- **Direct**: **Someone wants to connect**, **Files waiting for your answer**, **Files coming in**, **Files received**, **Files delivered**. They cover [My devices](devices.md) too.
- **Pako**: **Pako needs you**, **Sleep and shut down**, **New Pako ready**, **Other messages**.

Every kind pops up at first. Click **Save** to keep your changes.

### Sounds: files arriving, files sent

Pako has two sounds, one for each direction:

- A finished download chimes as **Chime when a download finishes** says (see above): with **Pop-up and Important**, every download set to **Pop-up** or **Important**; with **Important only**, only those; with **Never**, none. **Off** never chimes. The **Important** one chimes when its pop-up appears.
- **Play a sound when files arrive** (on at first): the same soft chime when files arrive in [Direct](direct.md) or [My devices](devices.md), and when someone offers you files and Pako waits for your answer. A send you have to accept chimes twice (when it's offered, then when it's here); one from someone who can send without asking chimes once, when it's here. A muted contact makes no sound.
- **Play a sound when files are sent** (on at first): a brighter sound when an [upload](upload.md) finishes (once for the whole upload, also a ZIP in parts), and when a Direct or My devices send is delivered. A send through another of your PCs sounds once it reaches the contact. A file another of your PCs asks for (**Get this file** in [My devices](devices.md)) makes no sent sound here: that PC chimes when it arrives.

Both also play while the tray window is open. When the same sound would play again within about two seconds (several downloads finishing together), it plays once; a folder chimes once, when its last file is done. When both sounds come at the same moment, one plays after the other. They stay silent during Do not disturb, and while Windows says not now when **Follow Windows' Do not disturb** is on, except the chime of an **Important** download, which comes with its pop-up.

### Do not disturb

Under **Quiet times**:

- **Do not disturb**: no pop-ups and no sound, except an **Important** download's (see above). The bell still collects (the kinds with their **Bell** switch on).
- **Follow Windows' Do not disturb** (on at first, under **Advanced** at the bottom of the tab): no pop-ups or sounds during full-screen games and videos, presentations, or Windows' own Do not disturb. The bell still collects. (**Important** never shows over something full screen, whatever this says: it waits.)

Pop-ups that would have shown during that time are skipped, not saved for later (the bell has the kinds with their **Bell** switch on). Pop-ups already on screen hide, and come back when it's over with the rest of their time.
