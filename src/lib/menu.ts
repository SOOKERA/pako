// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
export interface MenuItem {
  label?: string;
  icon?: string;
  hint?: string;
  danger?: boolean;
  disabled?: boolean;
  separator?: boolean;
  action?: () => void;
}
