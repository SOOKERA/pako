<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms. -->
<script lang="ts">
  // Line icons on a 24px grid.
  const paths: Record<string, string> = {
    plus: "M12 5v14M5 12h14",
    pause: "M9 5v14M15 5v14",
    play: "M7 5l12 7-12 7z",
    bolt: "M13 3L5 13h6l-1 8 8-10h-6z",
    folder: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    x: "M6 6l12 12M18 6L6 18",
    file: "M6 3h8l4 4v14H6z",
    bell: "M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15zM10 20a2 2 0 0 0 4 0",
    "bell-off": "M8.6 6.3A6 6 0 0 1 18 11v4.2M16.5 18H4.5L6 16v-5a6 6 0 0 1 .4-2.1M10 20a2 2 0 0 0 4 0M4 4l16 16",
    chart: "M4 20V4M4 20h16M8 16v-5M12 16V8M16 16v-3",
    queue: "M4 6h10M4 12h10M4 18h6M18 13v8M14 17h8",
    settings: "M4 6h16M4 12h16M4 18h16M8 4v4M16 10v4M10 16v4",
    retry: "M4 12a8 8 0 0 1 14-5.3L20 9M20 4v5h-5M20 12a8 8 0 0 1-14 5.3L4 15M4 20v-5h5",
    restart: "M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5",
    check: "M5 12.5l4.5 4.5L19 7",
    alert: "M12 8v5M12 16.5v.5M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z",
    open: "M14 4h6v6M20 4l-9 9M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4",
    copy: "M9 9h11v11H9zM5 15H4V4h11v1",
    link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    globe: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18",
    pencil: "M4 20h4L19 9l-4-4L4 16zM14 6l4 4",
    top: "M5 4h14M12 20V8M7 13l5-5 5 5",
    up: "M12 19V5M6 11l6-6 6 6",
    down: "M12 5v14M6 13l6 6 6-6",
    bottom: "M5 20h14M12 4v12M7 11l5 5 5-5",
    trash: "M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3",
    download: "M12 4v11M7 10l5 5 5-5M5 20h14",
    upload: "M12 20V9M7 14l5-5 5 5M5 4h14",
    send: "M21 3L10 14M21 3l-7 18-4-7-7-4z",
    devices: "M3 5h12v9H3zM6.5 18h5M9 14v4M18 8h3v11h-3z",
    lock: "M6 11h12v9H6zM8.5 11V8a3.5 3.5 0 0 1 7 0v3",
    mail: "M4 6h16v12H4zM4 7l8 6 8-6",
    chat: "M5 5h14a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H10l-5 4v-4H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z",
    drive: "M8 4h8l6 10-4 6H6l-4-6zM8 4l8 16M16 4L8 20M2 14h20",
    user: "M15 7a3 3 0 1 1-6 0 3 3 0 0 1 6 0zM5 20a7 7 0 0 1 14 0",
    power: "M12 3v9M6.3 7.2a8 8 0 1 0 11.4 0",
    list: "M4 6h16M4 12h16M4 18h16",
    clock: "M12 7v5l3 2M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
    search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4",
    chevron: "M6 9l6 6 6-6",
    chevronsRight: "M7 6l6 6-6 6M13 6l6 6-6 6",
    minus: "M5 12h14",
    star: "M12 3.5l2.6 5.3 5.9.9-4.25 4.1 1 5.8L12 16.9l-5.25 2.7 1-5.8L3.5 9.7l5.9-.9z",
    help: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM9.6 9.4a2.5 2.5 0 1 1 3.4 2.4c-.6.3-1 .8-1 1.5v.4M12 16.9v.2",
    tour: "M5 4.5v15M5 5h11l-2.5 4L16 13H5",
    tag: "M3.5 12.2V4.5a1 1 0 0 1 1-1h7.7l8.3 8.3a1 1 0 0 1 0 1.4l-7.3 7.3a1 1 0 0 1-1.4 0zM8 8h.01",
  };
  let { name, size = 16, stroke = 1.8 }: { name: string; size?: number; stroke?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name] ?? name} />
</svg>
