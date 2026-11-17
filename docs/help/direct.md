# Direct

Send files straight from your PC to the PC of someone you know, like a messaging app for files. Both of you need Pako.

For your own PCs (your desk PC and your laptop, say), use [My devices](devices.md) instead: files go without Accept, and you can see and start downloads on the other PC.

## How it works

- Files go from one Pako to the other, encrypted. Nothing is uploaded and stored anywhere in between.
- Both Pakos must be open at the same time (in the tray counts). Files can't wait for someone who is offline.
- Your contacts and your history stay on your PC.
- Nothing goes through a Sookera server. To find each other, Pakos use a public address directory, and when the two PCs can't reach each other straight, a public relay. Both are run by the makers of the connection library Pako uses. The relay only passes on encrypted data.
- Direct starts the first time you open the **Direct** page (it says "Direct is starting…" for a moment), and with Pako from then on. Until then, Pako makes no Direct connection at all. If you used Direct in an earlier Pako, it starts with Pako as before.
- Once it runs, your Pako connects to a relay and lists its Pako ID in the directory, so others can find it, also before you have contacts: someone you gave your ID to can ask to connect. Your internet address isn't listed there. Pako may also ask your router to open a port, so two PCs can connect straight.
- On the same network (the same Wi-Fi or home network), Pakos find each other without the internet, and files go PC to PC over that network: see [On the same Wi-Fi](#on-the-same-wi-fi).
- Your contacts can tell when your Pako is open: they see you as online, and when they last saw you.

## Your Pako ID and your name

Every Pako has its own **Pako ID**, a long code. It's at the bottom of the contact list in **Direct** (or in the middle of the page before you have contacts). **Copy** copies it: send it to the people you want to connect with.

**Your name** (just above it) is what your contacts see. They can still give you their own name for you. Pako asks for it once, before your first contact: when you add someone, or accept someone who wants to connect.

## Add someone

1. Ask them for their Pako ID, or send them yours.
2. Click **Add** (or **Add contact**), paste their ID under **Their Pako ID**, type a **Name** (what you call them) and click **Add contact**.
3. Pako reaches them. The first time, they're asked to accept. While they're asked, they show "Waiting for them to accept". If they don't answer, the line says "No answer yet. Pako asks again later."

If they decline, their line says "They declined. You can ask again after 7 days." For 7 days their Pako turns you away without asking them; after that, **Connect** asks again.

You only do this once. After that, you connect by yourselves whenever both Pakos are open.

### The first time: Accept and the safety code

When someone adds you, Pako asks: "Rado wants to connect" (for example). It shows a **Safety code**. The person who added you sees the same code above your conversation.

- If you know them, check once that you both see the same code (by voice or chat). A different code means it isn't them.
- The name is the one they chose: Pako can't check it. If you already have a contact with that name, Pako warns you that this is a different Pako.
- **Accept** adds them to your contacts. **Decline** turns them away. Closing the question with its **×** (its tooltip says **Decline**) or **Esc** declines too.

For 7 days, Pako turns away someone you declined or removed without asking you. After that, if they reach you again, you're asked like the first time.

## Your contacts

The contact list shows each person with a dot on their picture:

- **Grey**: offline. Under the name: when you last saw them.
- **Green**: online, and how you're connected (see [How you're connected](#how-youre-connected)).
- **Blue**: online on the same network as you ("Local network").
- **Amber**: waiting for them to accept you.

The dot changes colour by itself, for example when someone joins or leaves your Wi-Fi.

Something waiting for *your* answer is coral, not amber: a **!** on the contact, the dot next to **Waiting for you** on the left, and the count on the **Direct** tab.

Online people come first, then those you're waiting for, then the rest, by name. When there are many contacts, **Search contacts** finds them by name or tag.

Pako tries to reach your contacts by itself every few minutes (not someone who declined you: only **Connect** asks them again). **Connect** (above the conversation) tries now. A contact who restarts their Pako is back online at once.

### Filters and tags on the left

- **All contacts**, **Online**, **Waiting for you** (files waiting for your answer) and **Favourites**.
- **Tags**: your own groups, such as Friends, Family or Work. **New tag** makes one. Right-click a tag to rename or delete it (deleting a tag keeps the contacts).

Tags are only on your PC: your contacts never see them.

### Rename, favourites, tags

Right-click a contact, or click the gear above the conversation, for:

- **Rename…**: your own name for them. If they call themselves something else, it shows next to their name ("calls themselves …"). When they change their name, connected contacts see it at once. A contact you never renamed takes their new name.
- **Tags…**: tick their tags, or type a new one and click **Add**, then **Save**.
- **Add to favourites** / **Remove from favourites**: favourites have a star and their own filter.
- **Can send to me without asking** (see below).
- **Save their files in…**: where their files go.
- **Clear history…** (Pako asks first): empties your history with them on this PC. It can't be brought back.
- **Remove contact** (Pako asks first): you're no longer connected, a transfer with them stops, and for 7 days Pako turns them away without asking you. Files already received stay on your PC.

## Send files

1. Pick the person on the left.
2. Drop files or folders on the page, or click **Choose files** or **Choose folder**. They gather at the bottom and stay with that person, like a draft: another contact shows their own. Remove one with its **×**, or all with **Clear**.
3. Pick **As they are**, or **Compressed ZIP** to pack them into one smaller ZIP file first. **Name (optional)** names it.
4. Click **Send to Rado** (with their name).

They're asked whether they want the files, unless you may send to them without asking. Meanwhile your conversation shows "Waiting for them to accept…". Once they accept, the files go straight across.

Files go only while you're both online: with someone offline, **Send to Rado** stays greyed. You can still pick the files, and send them once they're back.

## Receive files

When a contact sends you files, Pako asks: "Rado wants to send you files". You see the name, how many files, the size and the first few file names.

- **Save to** shows where they'll go. **Change** picks another folder for this time only.
- **Accept** starts the transfer. **Decline** says no, and they're told. Closing the question (**×** or **Esc**) declines too.
- A question takes your answer only after a short moment on screen, so a stray click or **Esc** can't decline it by accident. The same goes for someone who wants to connect.

The question also goes in the bell, and shows as a pop-up when Pako's window isn't in front, both with **Accept** and **Decline** (see [Notifications](notifications.md)).

### Can send to me without asking

For people you trust, turn on **Can send to me without asking** (right-click menu, or in the details panel). Their files then come in without the question. Pako still asks when a send is big for the drive: "Asked because it's big for the drive".

### Where received files go

By default, a contact's files go to a folder named after them inside your download folder, like "From Rado". Change it with **Save their files in…** (right-click), or **Change…** next to **Their files go to** in the details panel.

Pako marks received files as coming from the internet, so Windows warns before starting a program you got this way. **Open** on a received program or script doesn't start it: Pako shows it in its folder, so you can open it there if you trust it.

## Your history with someone

Pick a contact to see everything sent and received with them, newest at the bottom. Each item shows its files, size, date, how it travelled, and how it ended: **Delivered**, **Received**, **Declined**, **Cancelled**, or what went wrong (for example "The connection was lost."). Received items have **Open** and **Folder**.

While files move, their item shows how far they are, the speed and **Cancel**. The contact's row in the list shows it too: "Sending 37% · 12 MB/s" under their name and a thin bar along the row. The **Direct** tab on the left shows how many questions wait for your answer (files, and people who want to connect), or how many transfers are moving.

If the connection drops in the middle, the transfer stops. Send it again once you're connected.

Other things a send can say:

- "Too many sends in a short time. Try again in a minute.": their Pako turned it away because many came at once.
- "Their Pako couldn't save it." or "Their Pako couldn't save it: there isn't enough room on their drive.": the problem is on their PC. Pako never tells you their folder or how much room they have.

While files move, Pako keeps the PC awake (with **Settings › Downloads › Keep the PC awake while downloading** on), **When done** waits for them, and quitting Pako asks first (with **Ask before quitting while downloading or uploading** on): "Files are on their way in Direct".

## The details panel

Press **Ctrl+I**, or click the panel button above the conversation, to see more about the person you picked:

- **Connection**: **Status** (**Online**, **Waiting for them** or **Offline**), the **Route** or when you last saw them, and **Speed now**.
- **Between you**: how much you **Sent** and **Received**, the number of **Transfers** and the **Last exchange**.
- **Speed with** them: the **Fastest** and **Average** speeds both ways, and a bar of how what you exchanged travelled: **Local network** (over your home network, in blue; shown once something went that way), **Direct** (straight between the PCs over the internet) and **Via relay**.
- **About them**: their **Pako ID** (with a copy button), **Their name** (the one they chose), **Tags**, **Favourite**, when they were **Added**, **Can send to me without asking**, and **Their files go to**.

## How you're connected

Next to "Online", Pako says how the two PCs are connected. It shows above the conversation, in the details panel's **Route**, and on each transfer too:

- **Local network**: over your home network (the same Wi-Fi, or a cable on the same router). Nothing goes through the internet, and it's usually the fastest by far. The contact's dot is blue.
- **Direct (IPv6)**: straight between the two PCs, over IPv6. Usually the fastest.
- **Direct**: straight between the two PCs.
- **Via relay**: through a relay server, when the two networks don't allow a straight connection. Still encrypted from end to end, often slower.

You don't choose this: Pako takes the best way it can. A connection can start **Via relay** and switch to a better way a few seconds later.

## On the same Wi-Fi

When you and a contact are on the same network, for example at home on the same Wi-Fi, your two Pakos find each other on that network and connect without the internet. Files go straight from one PC to the other over the network, often much faster than over the internet, and their dot turns blue: "Online · Local network".

- **Without internet:** Direct keeps working on the network. Contacts on the same network connect as usual. Contacts elsewhere show "Not reachable without internet" until the internet is back; then Pako reaches them by itself.
- **What your network sees:** when you're visible (below), your Pako tells the other devices on the same network that a Pako is here: its Pako ID, and its address and port on that network. Nothing else: not your name, not your contacts. This stays on the network: your router doesn't pass it on to the internet. Someone on the same network who sees your Pako ID can ask to connect, like someone you gave it to: you're asked first, as always. Only your contacts connect by themselves.
- **Be visible to Pakos on this network**, under your Pako ID on the **Direct** page:
  - **On home networks** (the default): only while Windows marks the network you're on as private (or as your company's network). On a network Windows marks as public, like a café or hotel Wi-Fi, nobody there sees your Pako ID. The **Direct** page then says "Not visible on this network (Windows marks it, or another network this PC is on, as public)." When the PC is on several networks at once (a VPN, or a virtual network made by Hyper-V, WSL or VirtualBox), each one counts: one marked public is enough to stay hidden. When you change networks, Pako checks again by itself.
  - **Always**: visible on every network, public ones too.
  - **Never**: visible on no network.

  Even when you're not visible, your Pako still finds your contacts on the same network and connects to them (they need to be visible). To make your home Wi-Fi private, see [the Windows firewall question](#the-windows-firewall-question) below.
- **Guest networks** often keep devices apart: two PCs on a guest Wi-Fi may not see each other. They still connect over the internet.

### The Windows firewall question

The first time Direct starts, Windows may ask whether Pako can use the network ("Windows Security Alert"). Pako says so on the **Direct** page while it starts. Allow it on **Private networks**, so Pakos on your home network can reach you.

If you clicked **Cancel**, or your home Wi-Fi is set as a public network, contacts on the same Wi-Fi may connect only over the internet (or not at all without it), and with **On home networks** they don't see you there. To fix it:

- Make your home Wi-Fi private: **Windows Settings › Network & internet › Wi-Fi ›** your network **› Private network**.
- Or allow Pako: **Windows Security › Firewall & network protection › Allow an app through firewall**, then tick **Private** next to Pako.
