# Help and problems

## Help in Pako

Press **F1** anywhere in Pako, or click **Help** at the bottom left. Help has these guides (with a search box), **What's new**, **About Pako**, and every tutorial: the **Quick tour** shows what each page is for, and each page's own tutorial shows its details.

## Report a problem

When something goes wrong, tell Sookera in one click with **Send report**, or with your own email app. Nothing goes until you click: you see everything that goes with your words first.

1. Open **Help › Report a problem**.
2. Write what you did and what you saw in **What happened?**.
3. Check **Sent with your report**. It shows everything that goes with your words: Pako's version, Setup or Portable, your Windows version, the theme, your PC's time zone (the times in the list are your PC's) and, while **Include recent problems** is on, the last 20 problems Pako noted (click **Show all** to see each one). Turn **Include recent problems** off to leave them out.
4. Pick how to send it:
   - **Send report** (or **Ctrl+Enter**): sends what's shown to Sookera's report service (the time zone goes only inside the problems' times: with **Include recent problems** off, it isn't sent). Reports are kept private and used only to fix Pako. When it's sent, Pako shows the report's reference (like **#42**): mention it if you write to support@sookera.com about the same problem. If it can't go (no connection, too many reports at once, or the service isn't answering), Pako says so and keeps your words: try again later, or use one of the ways below. You can close the window while it's sending: a message then says whether it went.
   - **Send by email**: your email app opens a new message to support@sookera.com with everything filled in. Check it, then send it. The email, **Copy details** and **Save as a file…** all carry the same things as **Sent with your report**, with the time zone on its own line. If the report is too long for one email link, Pako puts the details on your clipboard first, and the email says where to paste them.
   - **Copy details**: for email in your browser (Gmail, Outlook on the web). Paste them into a new email to support@sookera.com.
   - **Save as a file…**: a text file you can attach to an email.

### The list of problems

Pako keeps a short private list of the last 300 problems it ran into: downloads and videos that stopped, link checks, uploads, Direct transfers, update checks and the browser extension. Each line says when, where, the message Pako showed you, and Pako's version. It's the file `problems.log` in Pako's data folder (see [Where your settings and lists are kept](install.md#where-your-settings-and-lists-are-kept)).

Before a line is written, Pako takes out what could say who you are or what you download:

- Links keep only their site (`https://example.com`), never the rest of the address, also when written without `https://`.
- Folder and file names in paths read `…`: a file keeps only its type (`%USERPROFILE%\Downloads\….zip`). Windows' own folders (Downloads, Documents, Program Files…) stay. A file name in quotes reads the same way (`"….zip"`).
- Cookies, passwords, tokens and other keys, long IDs (Pako IDs, Google Drive IDs, video IDs too), Direct safety codes and email addresses are removed.
- Your Windows user folder reads `%USERPROFILE%`, and other people's user names are left out.

When many problems come at once, Pako notes the same problem only once a minute, and at most 30 problems a minute.

### If Pako has to close

If Pako runs into a problem it can't get past, it says so before it closes, and that problem is in the list. Start Pako again: your downloads carry on. If it happens again, use **Help › Report a problem**. If Pako's window can't open at all, the message says to install Microsoft Edge WebView2 Runtime (from microsoft.com) and start Pako again.

The list stays on your PC. It leaves only in a report you send yourself, and only while **Include recent problems** is on.
