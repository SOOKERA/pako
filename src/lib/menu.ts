// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
export interface MenuItem {
  label?: string;
  icon?: string;
  hint?: string;
  /** A few plain words after the label ("No room now"); `hint` is for shortcuts. A function is
   *  read live while the menu is open. */
  note?: string | (() => string | undefined);
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  /** A choice that's on or off (✓ when on): shown as a checkbox item. A function is read live
   *  while the menu is open (with `keepOpen`). */
  checked?: boolean | (() => boolean);
  /** Choosing it leaves the menu open (ticking several columns in a row). */
  keepOpen?: boolean;
  action?: () => void;
}
