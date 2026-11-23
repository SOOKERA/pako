// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// English plurals and keys with a note for this area (see ./en.ts).
import type { Entry } from "./core";

export const uploadEn: Record<string, Entry> = {
  // Keys with a note: the same English, other French.
  "Remove from list (button)": "Remove from list",
  "Upload (button)": "Upload",
  "Uploading (list)": "Uploading",
  "Between 50 MB and 10 GB (in a sentence)": "between 50 MB and 10 GB",
  "Google account (in a sentence)": "Google account",
  // Plurals.
  "{n} minutes ago": { one: "{n} minute ago", other: "{n} minutes ago" },
  "Deleted {n} items from Gofile": { one: "Deleted {n} item from Gofile", other: "Deleted {n} items from Gofile" },
  "Deleted {n} items.": { one: "Deleted {n} item.", other: "Deleted {n} items." },
  "{n} items couldn't be deleted: {error}": { one: "{n} item couldn't be deleted: {error}", other: "{n} items couldn't be deleted: {error}" },
  "Delete {n} downloaded items from Gofile?": { one: "Delete {n} downloaded item from Gofile?", other: "Delete {n} downloaded items from Gofile?" },
  // The same English either way; French says "autre" / "autres".
  "{name} and {n} more.zip": { one: "{name} and {n} more.zip", other: "{name} and {n} more.zip" },
  "{n} items can't be read. Remove them to upload the rest.": {
    one: "{n} item can't be read. Remove it to upload the rest.",
    other: "{n} items can't be read. Remove them to upload the rest.",
  },
  "Deletes in {n} hours": { one: "Deletes in {n} hour", other: "Deletes in {n} hours" },
  "Deletes in {n} days": { one: "Deletes in {n} day", other: "Deletes in {n} days" },
  // n: the accounts other than the connected one.
  "Some uploads on your list are in the Google Drive of {accounts}. Connect that account to delete them, or to let Pako delete them on their date.": {
    one: "Some uploads on your list are in the Google Drive of {accounts}. Connect that account to delete them, or to let Pako delete them on their date.",
    other: "Uploads on your list are in the Google Drive of {accounts}. Connect the account they're in to delete them, or to let Pako delete them on their date.",
  },
  "{n} uploads are in another Google account ({account}): connect that account to delete them.": {
    one: "{n} upload is in another Google account ({account}): connect that account to delete it.",
    other: "{n} uploads are in another Google account ({account}): connect that account to delete them.",
  },
  "{n} uploads are in another Google account: connect that account to delete them.": {
    one: "{n} upload is in another Google account: connect that account to delete it.",
    other: "{n} uploads are in another Google account: connect that account to delete them.",
  },
  // The same English either way; French: "coché" / "cochés".
  "{n} ticked · {size} freed": { one: "{n} ticked · {size} freed", other: "{n} ticked · {size} freed" },
  "Deleted {n} uploads from your Google Drive, {size} freed": {
    one: "Deleted {n} upload from your Google Drive, {size} freed",
    other: "Deleted {n} uploads from your Google Drive, {size} freed",
  },
  "Deleted {n} uploads.": { one: "Deleted {n} upload.", other: "Deleted {n} uploads." },
  "{n} uploads couldn't be deleted: {error}": { one: "{n} upload couldn't be deleted: {error}", other: "{n} uploads couldn't be deleted: {error}" },
};
