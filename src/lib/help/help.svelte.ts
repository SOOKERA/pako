// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Help panel state: open it on a guide (and a section), or on What's new.

export type HelpPage = { kind: "home" } | { kind: "guide"; file: string; anchor: string | null } | { kind: "whatsNew" } | { kind: "about" };

class Help {
  open = $state(false);
  page = $state<HelpPage>({ kind: "home" });
  /** Bumped on every show(), so the panel scrolls to the section again even if it's the same one. */
  nonce = $state(0);

  /** `target`: "downloading.md", "downloading.md#google-limit…", "whats-new", "about" (copyright and
   *  licence), or nothing for the start page. */
  show(target?: string) {
    if (!target) this.page = { kind: "home" };
    else if (target === "whats-new") this.page = { kind: "whatsNew" };
    else if (target === "about") this.page = { kind: "about" };
    else {
      const [file, anchor] = target.split("#");
      this.page = { kind: "guide", file, anchor: anchor || null };
    }
    this.nonce++;
    this.open = true;
  }

  close() {
    this.open = false;
  }

  /** Report a problem (opened from Help, over it). */
  report = $state(false);
  /** What you wrote there: kept while Pako runs, until the report went to your email app. */
  reportDraft = $state("");
}

export const help = new Help();
