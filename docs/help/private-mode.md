# Private mode

Private mode keeps the downloads from your browser's private windows behind a PIN or a password. Their files are encrypted on this PC: they can't be opened from File Explorer, only from Pako, once you've opened Private mode.

It's off until you turn it on. Without it, a private window's download works as described in [Browser extension › Private windows](browser-extension.md#private-windows): **Done · private**, then it leaves the list.

## Turn it on

1. Go to **Settings › Private mode** and click **Turn on…** next to **Private mode: keep private downloads behind a PIN**.
2. Read what it does, then click **Continue**.
3. Pick **4-digit PIN** or **Password**, type it twice, and click **Turn on Private mode**.
   - A PIN slows down someone at this PC: Pako waits longer after each wrong try. Someone who gets Pako's files and can use your Windows account could try all 10,000 PINs in minutes. A password is the real protection (at least 10 characters; a few words in a row, with a digit or a sign, is hard to guess).
4. Pako offers a **recovery code**: 8 groups of 4 letters and digits. Keep it with **Copy**, **Save as a file** or **Print**, then type its last group and click **I kept it**. Or click **Skip**.

Pako shows the recovery code only then and never keeps it. With it, you can open Private mode if you forget your PIN or password, and choose a new one. Keep it away from this PC: anyone with it and this PC can open your private files.

**If you forget your PIN or password and don't have your recovery code, nobody can open the private list or its files, not even us.** The only way back is **Reset Private mode**, which deletes them.

## What goes in

Once it's on:

- Downloads from your browser's private windows go to Private mode, not to your normal list. They're encrypted while they download (they can be paused and resumed as usual) and stay until you delete them. The **Download with Pako** window isn't shown for them.
- Videos from a private window are encrypted the moment they're done. Until then their pieces wait, readable, in Pako's own folder, which only your Windows account can open (on a drive formatted FAT32 or exFAT, like some USB sticks, Windows can't limit a folder to one account). They're deleted when the video is done, removed, or at the next start.
- A file your browser downloads for Pako (**Via browser**) is readable, under its real name, in your browser's download folder until it finishes; then Pako encrypts it into Private mode and deletes it there. If it can't be deleted, Pako says where it is: delete it yourself.
- **Download privately** sends any link straight into Private mode, also from a normal window: in **Links** (the arrow next to the main button, the right-click menu, or the arrow next to **Download** in the details), and in the **Download with Pako** window (the arrow next to **Download**). From **Links** it's encrypted from its first byte. From the **Download with Pako** window, the few seconds Pako had already fetched to check it are overwritten and deleted, and it starts again, encrypted. Either way it leaves your normal lists at once, and nothing about it is kept there. It's offered while Private mode is on, locked or open.
- A folder download (a Google Drive folder, a split archive, a playlist) stays together: one folder row in the private list, its files inside in their own subfolders.
- You can move a finished download in: right-click it in **Downloads**, then **Move to Private**. Its readable file is deleted (if it can't be, Pako says where it is). What was counted in the Dashboard before stays counted, and copies made earlier (backups, OneDrive or other synced folders, the Recycle Bin) aren't touched.

Private downloads never show in your normal list, the tray window, the Dashboard, the problem list or **My devices**. The bell only says "A private download finished" (or stopped), without its name.

## Open it

In **Downloads**, the last item under **Show** is **Private**, with a lock. While Private mode is locked it looks like the others: nothing hints at what's in it. Click it (or press **Ctrl+Shift+P** anywhere in Pako, or click **Open Private mode…** in **Settings › Private mode**) and type your PIN or password.

- The first 3 wrong tries cost nothing. After the 4th, Pako makes you wait before the next one: 5 seconds, then twice as long each time, up to 15 minutes, even if you restart Pako. Every 30 minutes without a try forgives one wrong try.
- **Forgot your PIN?** (or password) takes your recovery code instead. Pako then asks you to choose a new PIN or password.

While Private mode is open, the **Private** entry turns violet and shows how many files are in it, and the list's header has two violet buttons at the top right:

- on the private list, **Normal list** (your other downloads; Private mode stays open) and **Lock**;
- on the normal list, **Private downloads** (back to the private list, no PIN) and **Lock**.

Those two lists are the only places Private mode stays open. Going anywhere else locks it: another page (**Links**, **Upload**, **Direct**, **My devices**, **Dashboard**), a filter on the left (**All downloads**, **Downloading**, **Waiting**, **Finished**, **Problems**, a source), **Settings** or **Help**, however you get there (the left side, a pop-up, the bell, a shortcut). Private mode's own windows (Decrypt, Send with Direct, Upload, Delete, the player, its right-click menus), the bell itself and the search box don't lock it.

If something is still running in Private mode (sending, uploading, decrypting, or a video playing), Pako asks first, for example "Leaving locks Private mode and stops sending Holiday.mp4. Leave anyway?": **Leave and lock** goes on (and stops it), **Stay** keeps you where you were. With nothing running, it locks at once without asking. Locking by itself (see [It locks itself](#it-locks-itself)) never asks.

## The private list

**Private downloads** lists what's in Private mode, newest first. A finished file shows when it was downloaded: the same time its temporary copies are named after. A folder download is one folder row (with how many files it holds); click its arrow to close or open it. Its **Size** and **Downloaded** columns can be made wider or narrower, or hidden by right-clicking the column titles, as in every list (see [Downloading](downloading.md#follow-your-downloads)). With **Downloaded** hidden, how each file is doing shows under its name. Pako remembers the widths and which columns you hid, nothing about the files.

- **Click** a file or folder to see it in the details panel on the right: its progress and speed, where it came from, when it was added and downloaded, its **Priority** and how it tells you it's done (**Off**, **Pop-up**, **Important**), with its buttons at the bottom: **Open** (or **Play**), **Decrypt…**, **Send…**, **Upload…** and **Delete…** (or **Pause**, **Resume** or **Try again**, and **Delete…**, while it downloads). **Ctrl+I**, or the details panel button in the title bar (the same one as on every page), hides or shows the panel; drag its left edge to make it wider or narrower.
- **Pick several**: tick their boxes, or **Ctrl**-click and **Shift**-click, or press **Ctrl+A** for all (**Esc** picks none). The top shows "3 selected" with **Decrypt 3 files…** (when they're all finished), **Pause 3** / **Resume 3** and **Delete 3 files…**, and the details panel sets their priority and notification together. Right-click does the same.
- **Keys**: **↑ ↓**, **Home**, **End** move through the list, **Space** ticks the row you're on, **Shift+F10** or the **Menu** key opens its right-click menu, **Ctrl+F** searches the private downloads' names (the search box at the top), and **Ctrl+I** shows or hides the details panel.

- **Double-click** a file, or click its **▶** / open button:
  - A video plays in Pako's own player. It's decrypted as it plays: no readable copy is written anywhere, and Pako notes nothing about it. Kinds other than MP4 and WebM need **Preview more video types** in **Settings › Features & updates**.
  - Anything else opens in its usual app from a temporary copy, named like `PrivateFile_2026-11-18_14h32.pdf` (when it was downloaded), so its real name doesn't show in that app's title or in Windows' recent files. Turn on **Show real names when opening private files** in **Settings › Private mode** to keep the real name. Temporary copies and decrypted files carry Windows' "from the internet" mark, so Windows and Office check them before they run or open for editing.
- **Decrypt…**: saves a readable file in the folder you pick (your download folder at first; **Change…** to pick another). A file already there is never replaced: the new one gets " (2)". In the same step, pick what happens next:
  - **Take it off the private list**: the readable file stays where you saved it, Pako keeps no record of it, and the encrypted copy is deleted.
  - **Move it to Downloads**: it shows in your Downloads list like any finished download (**Open file**, **Show in folder**…). The Dashboard doesn't count it after the fact. The encrypted copy is deleted.
  - **Leave it as is**: it stays in Private mode, still encrypted, and you also have the readable file.

  Its window closes and it decrypts right there: the row and the details panel show how far it is ("Decrypting · 40%"), and a short note says when it's done. If decrypting stops partway (a full drive, Private mode locking), the half-made file is deleted and the private file stays as it was. Anyone who can open that folder can read the file.
- **Decrypt folder…** (on a folder row, or right-click it): the same for every file in it, into a folder of the same name in the place you pick, each file back in its own subfolder. Several picked files are decrypted the same way, side by side.
- **Send with Direct…** and **Upload…** (the send button on its row, its details, right-click, or for several picked files): see [Send private files](#send-private-files).
- **Delete…**: deletes it from Private mode for good (there's no copy in the Recycle Bin).
- **Normal list** (top right) goes back to your normal list; Private mode stays open. **Lock**, next to it, locks it.

### Send private files

A finished private file (or a folder, or several picked files) can go to someone without decrypting it first:

- **Send with Direct…**: pick one of your contacts or your own PCs. Pako reads the files decrypted as they go: no readable copy is written on this PC. The other side gets normal files, under their real names (a folder keeps its layout).
- **Upload…**: a window with the same choices as Upload's **New upload** (where it goes, **Delete after**, how several files go, **Protect with a password**), right here in Private mode. The files go up under their real names, and Pako reads them decrypted as they go up. **Protect with a password** still works: 7-Zip needs files on disk for that, so Pako writes readable copies in its own private folder (only your Windows account can open it) and deletes them as soon as the archive is made, or if making it fails or you cancel (and at the next start, if Pako stopped meanwhile). **ZIP in parts** works too: Pako makes one part at a time, in that same folder, and deletes each part (overwriting it first) the moment it's up, so one part at a time is briefly readable in a folder only your Windows account can open. After a cut, a pause or a restart, the next part is made again from the private files.

You stay in Private mode while it goes: the files' rows and the details panel show it ("Sending to Rado · 35%", "Uploading · 60%"). Sending only works while Private mode is open: if it locks, or you leave it (Pako asks first), the send stops (send it again once it's open). Like any send, it shows with the file's name in Direct's history (or **My devices**) or in **Uploaded**, and in its pop-ups (**Files delivered**, the upload finishing), the bell and the tray window: the file is yours to share; what Pako keeps private is its own record of your downloads. If Private mode locks while its Upload window is open, the window closes and nothing goes up.

### Temporary copies

Pako deletes a temporary copy (overwriting it first) once the app that opened it lets go of it, when Private mode locks (or as soon as the app lets go, if it still has it open), when Pako closes, and any left over the next time Pako starts.

If you changed the copy in its app, Pako asks **Save your changes into Private mode?**: **Save changes** puts the new version in Private mode, **Don't save** keeps the old one. While Private mode is locked, Pako doesn't ask: your changes are saved into it. If Pako closes first (also from the tray), or stops unexpectedly, your changes are saved into Private mode then, or at its next start. If saving doesn't work, the copy is kept and Pako tries again: it's never deleted with your changes in it.

Good to know: an app's own recent-files list and its jump list (Word's, say, or the taskbar's right-click list) may still show the temporary name; Pako can't clean those. On an SSD, deleting can't promise every old copy of the bytes is gone from the drive.

## It locks itself

Private mode locks:

- after **5 minutes** with no keyboard or mouse use anywhere on the PC (change it in **Settings › Private mode › Lock Private mode after**: 1, 5, 15 or 30 minutes, or **Never for being idle**, which still locks for every reason below). A private video playing in Pako's player, with Pako's window in front, keeps it open while it plays;
- at once when Windows locks (**Win+L**) or the PC is about to sleep, when the screen saver starts, and when Pako is minimised, hidden in the tray or closed;
- when you click **Lock** (or press **Ctrl+Shift+P** while the private list shows);
- when you leave its two lists: see [Open it](#open-it) above.

Locking goes back to your normal list and closes Pako's player. Private downloads still running carry on, hidden, into Private mode.

A private download that wasn't finished when Pako closed doesn't continue by itself at the next start: Pako needs your PIN or password to go on writing it. It continues as soon as you open Private mode again.

## Clear the private list

In **Settings › Private mode › Clear the private list**, pick **3 hours after**, **12 hours after**, **3 days after**, or **After a number of hours…**: files are then deleted from Private mode that long after they finished, whether it's open or locked. **Never** (the default) keeps them until you delete them. Downloads still running stay.

## Change your PIN, password or recovery code

In **Settings › Private mode**:

- **Change PIN or password…**: type your current one, then the new one (you can switch between a PIN and a password). Your files aren't touched. The old one stops working on this PC, but a copy of Pako's files made before the change still opens with it.
- **New recovery code…**: type your PIN or password first (Private mode stays locked), then Pako makes a new code; once you've typed its last group, the old code stops working on this PC (a copy of Pako's files made before still opens with it). **Cancel** forgets the new code.
- **Reset…** (next to **Reset Private mode**): deletes the private list and every file in it, for good, and turns Private mode off.

## How it works

Private mode has one key of its own, made at random when you turn it on. Your PIN or password (and your recovery code) only lock that key, through a deliberately slow step (about half a second per try on a recent PC), and that locked key is also sealed by Windows for your account, so a copy of Pako's folder can't be tried on another PC or account. Changing your PIN or password locks the same key again: nothing else changes. Each file has its own key too, tied to its line in the private list, and every file and every line of the list is encrypted (XChaCha20-Poly1305) and checked when it's read, so a changed, swapped or damaged file is refused rather than shown. Pako can add downloads while Private mode is locked without being able to read anything in it.

Everything stays on this PC, in the `private` folder of Pako's data folder. Uninstalling Pako keeps it, even with **Also delete Pako's settings and lists** on, unless you also turn on **Also delete Private mode's files** (see [Installing and updating](install.md#uninstalling)).
