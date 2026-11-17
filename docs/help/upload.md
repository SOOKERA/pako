# Upload

Share big files with a link. Pako uploads them to Gofile or to your own Google Drive, copies the link, and anyone can download it in their browser.

## How sharing works

1. **You drop the files** on the Upload page.
2. **Pako puts them online**, on Gofile or in your Google Drive. Nothing goes through our servers.
3. **They open the link** in any browser, or paste it into **Links** in their own Pako.

## Upload files

1. Open **Upload** on the left.
2. Click **Choose files** or **Choose folder**, or drop files and folders on the page. They gather in **New upload**: nothing is uploaded yet.
3. Pick where they go and how (below), then click **Upload**.

Add more with **Add files** and **Add folder**, take one out with its **×**, or empty the box with **Clear**. An item that can't be read says why under its name: remove it to upload the rest.

When the upload finishes, Pako copies the link for you: paste it to anyone.

## Where it goes: Upload to

- **Gofile**: free, no account needed. Gofile keeps files while people download them. After 10 days without a download, they may be removed.
- **Google Drive**: your own Drive, shared with anyone who has the link. It stays there until you delete it. Pako puts everything it uploads in a folder of your Drive called "Pako uploads". If an earlier Pako made a folder called "Pako sends", Pako keeps using that one, so what you uploaded before stays where it is.

Your choice is remembered for the next upload. You can also set it in **Settings › Upload › Upload to**.

## Delete after

Uploads can delete themselves after **3 days**, **7 days** or **30 days**, or **Never**. **7 days** is the usual choice, and Pako remembers the one you pick.

- On Gofile, Gofile deletes it itself on that day when it accepts the date, even with your PC off. Otherwise Pako deletes it the first time it runs after that day.
- On Google Drive, Pako deletes it the first time it runs after that day.

If Pako can't delete one yet, it shows **Couldn't delete yet** and tries again every hour.

## Several files, or one compressed file

**Several files** (or a folder) can go up four ways:

- **As they are**: one link to a folder with the files as they are. The person sees them straight away.
- **One ZIP**: packed into one ZIP file, not compressed (fastest).
- **Compressed ZIP**: packed into one smaller ZIP file. It takes longer, and helps documents and text, not videos or photos.
- **ZIP in parts**: one ZIP file, not compressed, cut into numbered parts ("Photos.zip.001", "Photos.zip.002"…) that go into one folder, with one link. Best for big uploads on a shaky connection (see below).

The usual choice is in **Settings › Upload › When you upload several files**, with the same names. The choice in **New upload** is for that upload only.

**Name** sets the folder's or the ZIP file's name. It's optional: the grey text shows the name Pako uses otherwise.

A **single file** goes up as it is. Under **Compression**, pick **Compressed ZIP** to pack it into a smaller ZIP file instead, or **ZIP in parts**, with a **Name** if you like.

### ZIP in parts

Each part uploads on its own: if the connection drops, only the part in progress starts over, and the parts already up stay up.

- **Part size**: **500 MB** (the usual), **1 GB**, **2 GB**, or **Custom** to type a size in MB, between 50 MB and 10 GB. Smaller parts start over faster; bigger ones make fewer files. The usual size is in **Settings › Upload › Advanced › Size of each part**.
- Pako makes the parts on your PC first (**Making the parts**), in its own data folder, so it needs as much free space there as the ZIP. They're deleted once every part is up, or when you cancel or remove the upload.
- The upload shows how many parts are up ("3 of 7 parts up"). **Try again** uploads only the parts not up yet, into the same folder, so the link stays the same. After Pako was closed or the PC lost power, the upload is back under **Uploading** when Pako starts, and carries on by itself with the parts not up yet once Gofile or your Google Drive can be reached (it tries again quietly while the connection is down). Turn off **Settings › Upload › Advanced › Continue unfinished uploads when Pako starts** to have it wait for **Try again** instead.

**For the people downloading**: they need every part, in one folder.

- With 7-Zip: open the first part ("Photos.zip.001"). 7-Zip puts the parts back together and shows what's inside.
- With Pako, from a Google Drive link: paste the link in **Links**. Pako downloads all the parts into one folder and, with **Extract split archives** on (or **Extract when all parts are done** in the set's details), puts the ZIP back together and unpacks it.
- From a Gofile link: download every part from Gofile's page, then open the first one with 7-Zip.

## While it uploads

Uploads in progress show under **Uploading**, with how far they are, the speed and the time left. **Cancel** stops one: what already went up is deleted too. After a **Try again**, **Cancel** keeps what was already up, and it stays on the list as **Not all uploaded**.

If some files (or parts) didn't make it, the upload says so. **Try again** uploads only the missing ones, into the same folder, so the link stays the same. **Copy link** already works for the files that are up. The **×** takes the message away.

The tray window shows uploads too: switch to **Uploads** at its top.

## The Uploaded list

Everything you uploaded is listed under **Uploaded**, newest first:

- **Name**, how many files, the size and the date, and **ZIP in 7 parts** (for example) for a ZIP in parts.
- **Uploaded to**: **Gofile** or **Google Drive**, and when it deletes itself (**Deletes in 3 days**, **Deletes tomorrow**, **Kept**…). Next to **Gofile**: how often people downloaded it (**Not downloaded yet**, **Downloaded once**, **Downloaded 4 times**). Only Gofile shares this; Google Drive doesn't, so Drive uploads show none. Gofile counts downloads, not who downloaded. For a ZIP in parts it counts whole downloads (someone who takes all the parts counts once). In a narrow window the count is in the upload's details.
- **Link**: click it to open the download page. **Copy link** copies it.

The counts are checked when you open **Upload**, at most every few minutes. The round arrow next to **Uploaded** checks again.

What you may see on a Gofile upload:

- **May be removed by Gofile soon**: an upload with no delete date that nobody downloaded for a while. Gofile may remove it after 10 days without a download.
- **Gone from Gofile**: it was deleted or expired, and the link doesn't work any more. **Remove from list** takes it off.
- **Not all uploaded**: only some files are up. The link works for them. While Pako is still trying, it says **Uploading**. For a ZIP in parts, people need every part to open it.

**Delete downloaded ones** (next to **Uploaded**) deletes from Gofile everything that was downloaded at least once, after asking.

### Right-click an upload

Right-click it, or click the arrow at the end of its row, for:

- **Copy link**, **Open the download page**, **Show details**.
- **Keep 7 more days** and **Keep (don't delete)**, or **Delete in 7 days** for one that's kept.
- **Remove from the list**: only from Pako's list. The file stays on Gofile or in your Drive.
- **Delete now from Gofile…** or **Delete now from Google Drive…**: the link stops working for everyone and the file can't be brought back (on Drive, it doesn't go to the bin). Your own copy on this PC isn't touched.

## The details panel

Select an upload and press **Ctrl+I** (or double-click it) to see its details on the right:

- **Size**, **Files** and **Downloads**.
- **Link**: the link as a QR code (point a phone's camera at it to open the link), and **Copy link**. The code is made on your PC.
- **Delete after**: when it deletes itself and who does it, with **Keep 7 more days**, **Keep** or **Delete in 7 days**, and **Delete now**.
- **Files inside**: the files in it, with their sizes.
- **How it went**: when it was uploaded, how long it **Took**, the **Average speed** and **Top speed**, how it was **Uploaded as** (**As they are**, **One ZIP**, **Compressed ZIP**, **ZIP in 7 parts**, or **As it is** for a single file), and where (**Uploaded to**).
- **Downloads by others** (Gofile): a small chart of the downloads over time, and roughly when the last one was.
- **Note**: who it's for, what's inside. Only on this PC.

## Your Google Drive

To upload to your own Drive, connect it once: click **Connect Google Drive** in **Settings › Accounts** (under **Google Drive for uploads**), or on the bar that shows in **Upload** when **Google Drive** is picked. Google's page opens in your browser: sign in and allow it. If Google's page shows a box for Google Drive, tick it before you continue. Without it, Pako doesn't connect and says "Google didn't give Pako the permission to add files to your Google Drive. Connect again and, on Google's page, tick the box for Google Drive before you continue."

- Pako only sees the files it uploads, nothing else in your Drive.
- Connecting it makes **Google Drive** the usual choice for uploads. You can still pick **Gofile** for any upload.
- This sign-in is only for uploads. Downloads from Drive never need one.
- **Disconnect** (in **Settings › Accounts**) goes back to Gofile. What you uploaded stays in your Drive.

### The Drive bar

With **Google Drive** picked, a bar above the list shows how much room is left: "120 GB free in your Google Drive" (for example), how much is used, and how much Pako's uploads take ("Pako's uploads: 3 · 2.4 GB"). The round arrow checks again.

Before an upload, **New upload** says how much it needs and how much is free. When it doesn't fit, Pako asks first:

- **Make room…** lists your uploads in your Drive, oldest first, with those older than a week ticked. **Delete from Google Drive** deletes the ticked ones for good. Your own copies on this PC aren't touched.
- **Upload to Gofile instead** switches to Gofile and uploads there.

If uploads on your list are in another Google account's Drive, the bar says so: connect that account to delete them, or to let Pako delete them on their date.

## Good to know

- With Pako's window open, a finished upload shows a message at the bottom of the window. Otherwise it's a pop-up (see [Notifications](notifications.md)).
- If Pako closes while several files go up **As they are**, what was already up stays on the list as **Not all uploaded**. A ZIP in parts comes back under **Uploading** too and carries on with the parts left (on Gofile, once its first part is up), or waits for **Try again** with **Continue unfinished uploads when Pako starts** off. A ZIP or a single file that didn't finish isn't kept.
- Your list of uploads and your notes stay on your PC.
- You don't make a Gofile account, but Gofile makes a guest one at your first upload. Pako keeps its key on your PC (encrypted), to read the download counts and to delete your uploads.
- For Google Drive, Pako keeps your sign-in on your PC (encrypted) and reads your account's email address, to show which account is connected.
