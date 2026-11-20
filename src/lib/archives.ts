// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Which finished files offer "Extract…": a whole archive (.zip, .7z, .rar), or the first part of
// a split one (.part1.rar, .7z.001). Its other parts (.part2.rar, .002, .z01, .r00) don't: the
// first part extracts the whole set.

/** Another part of a split archive than its first. */
export function laterPart(name: string): boolean {
  return /\.part0*(?:[2-9]|[1-9]\d+)\.rar$/i.test(name) || /\.(?!001$)\d{3}$/i.test(name) || /\.[zr]\d{2}$/i.test(name);
}

/** "Extract…" makes sense for it. */
export function extractable(name: string): boolean {
  return !laterPart(name) && /\.(zip|7z|rar|001)$/i.test(name);
}
