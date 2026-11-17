// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako's "Download" button on video players (YouTube, Vimeo, Twitch, Facebook, Instagram), shown
// at the top right of the video while the mouse is over it (the pill itself is pill.js, loaded
// just before this file). It reads the page's address and, on Facebook and Instagram (where a
// page shows many videos), the links of the post around the video.
// Sites that change pages without reloading (YouTube) are handled by reading the address at
// each hover and at each click.
(() => {
  if (globalThis.__pakoPlayer) return;
  const kit = globalThis.__pakoPill;
  if (!kit) return;
  // (caught.js steps aside in a frame where this is set.)
  globalThis.__pakoPlayer = true;

  // (pill.js's choice: never a page element named "browser".)
  const api = kit.api;
  const QUALITIES = [
    [1080, "Up to 1080p", "MP4"],
    [720, "Up to 720p", "MP4"],
    [480, "Up to 480p", "MP4"],
    [0, "Audio only", "M4A"],
  ];

  // ---------- which video, as a link Pako knows ----------

  const YT_ID = /^[\w-]{6,20}$/;

  // Facebook and Instagram: a feed shows many videos, so the link comes from the post around the
  // video (its date or "Reels" link), else from the page's address (a post or reel opened alone).
  const IG_POST = /^\/(?:[\w.]+\/)?(p|reel|reels|tv)\/([\w-]{5,})(?:\/|$)/;
  function instagramLink(u) {
    const m = IG_POST.exec(u.pathname);
    return m ? `https://www.instagram.com/${m[1] === "reels" ? "reel" : m[1]}/${m[2]}/` : null;
  }
  function facebookLink(u) {
    const p = u.pathname;
    const v = u.searchParams.get("v") ?? "";
    let m;
    if ((p === "/watch" || p === "/watch/" || p === "/video.php") && /^\d+$/.test(v)) return `https://www.facebook.com/watch/?v=${v}`;
    if ((m = /^\/reels?\/(\d+)/.exec(p))) return `https://www.facebook.com/reel/${m[1]}`;
    if ((m = /^\/[^/]+\/videos\/(?:[^/]+\/)?(\d+)/.exec(p))) return `https://www.facebook.com/watch/?v=${m[1]}`;
    if ((m = /^\/watch\/live\/?$/.exec(p)) && /^\d+$/.test(v)) return `https://www.facebook.com/watch/?v=${v}`;
    return null;
  }
  /** The post around this video: Instagram's <article>, Facebook's post box. */
  function postOf(video, host) {
    if (host === "instagram.com") return video.closest("article");
    return video.closest('[role="article"], [aria-posinset], [data-pagelet^="FeedUnit"]');
  }
  /** The post's own link to its video: links inside a comment or a shared post within it
   *  (their own article) are skipped, and the post's time stamp / permalink comes first. */
  function linkInPost(video, host, read) {
    const post = video && postOf(video, host);
    if (!post) return null;
    const nested = (a) => {
      const inner = a.parentElement?.closest('[role="article"], article');
      return !!inner && inner !== post && post.contains(inner);
    };
    // A time stamp link: holds a <time> / <abbr>, or is labelled with the date.
    const stamp = (a) => !!a.querySelector("time, abbr") || (a.getAttribute("aria-label") || "").length > 0;
    let first = null;
    for (const a of post.querySelectorAll("a[href]")) {
      if (nested(a)) continue;
      let u;
      try {
        u = new URL(a.href, location.href);
      } catch {
        continue;
      }
      if (u.hostname.replace(/^(www|web|m)\./, "") !== host) continue;
      const link = read(u);
      if (!link) continue;
      if (stamp(a)) return link;
      first ??= link;
    }
    return first;
  }

  /** The video as a clean link (no playlist, no time), or null when it isn't one Pako knows.
   *  `video` is the <video> under the mouse (used where a page shows many videos). */
  function videoUrl(video) {
    let u;
    try {
      u = new URL(location.href);
    } catch {
      return null;
    }
    const social = u.hostname.replace(/^(www|web|m)\./, "");
    if (social === "instagram.com") return linkInPost(video, social, instagramLink) ?? instagramLink(u);
    if (social === "facebook.com") return linkInPost(video, social, facebookLink) ?? facebookLink(u);
    const host = u.hostname.replace(/^www\./, "");
    const parts = u.pathname.split("/").filter(Boolean);
    if (host === "youtube.com" || host === "youtube-nocookie.com") {
      if (u.pathname === "/watch") {
        const v = u.searchParams.get("v");
        return v && YT_ID.test(v) ? `https://www.youtube.com/watch?v=${v}` : null;
      }
      const [kind, id] = parts;
      if (!id || !YT_ID.test(id)) return null;
      if (kind === "shorts") return `https://www.youtube.com/shorts/${id}`;
      if (kind === "embed" || kind === "live") return `https://www.youtube.com/watch?v=${id}`;
      return null;
    }
    if (host === "vimeo.com") {
      const id = parts.find((p) => /^\d{3,}$/.test(p));
      return id ? `https://vimeo.com/${id}` : null;
    }
    if (host === "player.vimeo.com") {
      return parts[0] === "video" && /^\d{3,}$/.test(parts[1] ?? "") ? `https://vimeo.com/${parts[1]}` : null;
    }
    if (host === "twitch.tv") {
      if (parts[0] === "videos" && /^\d+$/.test(parts[1] ?? "")) return `https://www.twitch.tv/videos/${parts[1]}`;
      if (parts[1] === "clip" && /^[\w-]+$/.test(parts[0]) && /^[\w-]+$/.test(parts[2] ?? "")) {
        return `https://www.twitch.tv/${parts[0]}/clip/${parts[2]}`;
      }
      return null;
    }
    if (host === "clips.twitch.tv") {
      return parts.length === 1 && /^[\w-]+$/.test(parts[0]) && parts[0] !== "embed" ? `https://clips.twitch.tv/${parts[0]}` : null;
    }
    return null;
  }

  // ---------- the pill (pill.js) ----------

  const pill = kit.make({
    find: (x, y) => {
      const v = kit.videoAt(x, y);
      return v && videoUrl(v) ? v : null;
    },
    items: () => [{ head: "Download with Pako" }, ...QUALITIES.map(([quality, text, side]) => ({ text, side, quality }))],
    pick: async (item, video) => {
      const url = videoUrl(video);
      if (!url) return null;
      return api.runtime.sendMessage({ type: "video", url, quality: item.quality });
    },
  });
  // YouTube changes pages without reloading: a new page starts without the pill.
  document.addEventListener("yt-navigate-start", () => pill.hide());
})();
