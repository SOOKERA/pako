// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
// Pako Setup's folder check (src-tauri/installer/hooks.nsh: PakoCheckDir, PakoIsAdminFolder),
// run for real: a small NSIS program made with Tauri's makensis calls the functions on folders in
// a temporary folder (and a few that can't be changed), and nothing may be left behind. Skipped
// where Tauri's NSIS isn't there (it is after one `npm run tauri build` on Windows).
// Run: node --test test/*.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const hooks = path.resolve(here, "../src-tauri/installer/hooks.nsh");
const makensis =
  process.platform === "win32" && process.env.LOCALAPPDATA ? path.join(process.env.LOCALAPPDATA, "tauri", "NSIS", "makensis.exe") : "";
const skip = !makensis || !fs.existsSync(makensis) ? "Tauri's NSIS (makensis) isn't installed" : false;

const harness = `Unicode true
SilentInstall silent
RequestExecutionLevel user
OutFile "harness.exe"
Name "PakoCheckDirTest"
!include LogicLib.nsh
!include FileFunc.nsh
!include TextFunc.nsh
LoadLanguageFile "\${NSISDIR}\\Contrib\\Language files\\English.nlf"
LangString pakoDirNotWritable \${LANG_ENGLISH} "x"
LangString pakoDirAdminOnly \${LANG_ENGLISH} "x"
LangString pakoDirNotWritableSilent \${LANG_ENGLISH} "x"
Var PassiveMode
Var UpdateMode
!include "${hooks}"
Section
  SetOutPath "$EXEDIR\\cwd"
  FileOpen $8 "$EXEDIR\\paths.txt" r
  FileOpen $9 "$EXEDIR\\results.txt" w
  System::Call 'kernel32::GetCurrentProcessId()i.r7'
  FileWriteUTF16LE $9 "pid|$7$\\n"
  System::Call 'shell32::IsUserAnAdmin()i.r7'
  FileWriteUTF16LE $9 "admin|$7$\\n"
  next:
    ClearErrors
    FileReadUTF16LE $8 $6
    IfErrors done
    \${TrimNewLines} $6 $6
    StrCmp $6 "" next
    StrCpy $5 $6 4
    \${If} $5 == "adm|"
      StrCpy $6 $6 "" 4
      Push $6
      Call PakoIsAdminFolder
      Pop $5
      FileWriteUTF16LE $9 "adm|$6|$5$\\n"
    \${Else}
      StrCpy $INSTDIR $6
      Call PakoCheckDir
      FileWriteUTF16LE $9 "dir|$6|$PakoDirProblem$\\n"
    \${EndIf}
    Goto next
  done:
  FileClose $8
  FileClose $9
SectionEnd
`;

/** A drive letter with no drive. */
function freeDrive() {
  for (const l of "QRSTUVWXYZ") if (!fs.existsSync(`${l}:\\`)) return l;
  return null;
}

test("Setup's folder check: refuses what can't be changed, accepts the rest, leaves nothing behind", { skip }, (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "pako-dircheck-"));
  t.after(() => {
    spawnSync("icacls", [path.join(dir, "locked"), "/remove:d", os.userInfo().username], { stdio: "ignore" });
    fs.rmSync(dir, { recursive: true, force: true });
  });
  fs.writeFileSync(path.join(dir, "harness.nsi"), harness, "utf8");
  const made = spawnSync(makensis, ["-V1", "harness.nsi"], { cwd: dir, encoding: "utf8" });
  assert.equal(made.status, 0, made.stdout + made.stderr);

  fs.mkdirSync(path.join(dir, "exists"));
  fs.mkdirSync(path.join(dir, "locked"));
  fs.writeFileSync(path.join(dir, "afile"), "x");
  // A folder this user can list but not change.
  const locked =
    spawnSync("icacls", [path.join(dir, "locked"), "/deny", `${os.userInfo().username}:(WD,AD,WEA,WA)`], { stdio: "ignore" }).status === 0;
  const base = path.basename(dir);
  const drive = freeDrive();
  // [folder, "" (accepted) or "write" (refused)]; null = depends on admin rights.
  const cases = [
    [`${dir}\\exists`, ""],
    [`${dir}\\exists\\`, ""],
    [`${dir}\\exists\\..\\exists`, ""],
    [`${dir}\\missing\\deeper\\Pako`, ""],
    [`${dir}\\missing\\..\\..\\${base}\\new\\Pako`, ""],
    [`${dir}/forward/slashes`, ""],
    [`rel\\Pako`, ""],
    [`${dir}\\afile`, "write"],
    [`${dir}\\afile\\Pako`, "write"],
    // A network share that isn't there: never tried in the root of Setup's own drive instead.
    [`\\\\127.0.0.1\\pako-no-such-share\\Pako`, "write"],
    [`\\\\127.0.0.1\\pako-no-such-share`, "write"],
    [`\\\\127.0.0.1`, "write"],
    [`\\\\`, "write"],
  ];
  if (drive) cases.push([`${drive}:\\Pako`, "write"], [`${drive}:\\..\\Pako`, "write"], [`${drive}:`, "write"]);
  if (locked) cases.push([`${dir}\\locked`, "write"], [`${dir}\\locked\\Pako`, "write"]);
  cases.push([`${process.env.ProgramFiles || "C:\\Program Files"}\\Pako`, null]);
  const admin = [
    [String.raw`C:\Program Files\Pako`, "1"],
    [String.raw`D:\Program Files (x86)\Pako`, "1"],
    [`${process.env.windir || "C:\\Windows"}\\Pako`, "1"],
    [String.raw`C:\WindowsApps2\Pako`, "0"],
    [String.raw`C:\Program Filesx\Pako`, "0"],
    [`${process.env.LOCALAPPDATA}\\Programs\\Pako`, "0"],
  ];
  const lines = [...cases.map(([p]) => p), ...admin.map(([p]) => `adm|${p}`)];
  fs.writeFileSync(path.join(dir, "paths.txt"), Buffer.from(lines.join("\r\n") + "\r\n", "utf16le"));

  const run = spawnSync(path.join(dir, "harness.exe"), [], { cwd: dir, timeout: 60_000 });
  assert.equal(run.status, 0, "the test program ran");
  const out = fs.readFileSync(path.join(dir, "results.txt")).toString("utf16le").split(/\r?\n/).filter(Boolean);
  const pid = out.find((l) => l.startsWith("pid|"))?.slice(4);
  const isAdmin = out.find((l) => l.startsWith("admin|"))?.slice(6) !== "0";
  const got = new Map(out.filter((l) => l.startsWith("dir|")).map((l) => l.slice(4).split("|")));
  for (const [p, want] of cases) {
    const expected = want ?? (isAdmin ? "" : "write");
    assert.equal(got.get(p), expected, p);
  }
  const adm = new Map(out.filter((l) => l.startsWith("adm|")).map((l) => l.slice(4).split("|")));
  for (const [p, want] of admin) assert.equal(adm.get(p), want, `admin folder: ${p}`);

  // Nothing left: no test file or folder anywhere, no folder made for a folder that isn't there.
  assert.ok(pid, "the test program said its process ID");
  const leftover = (folder) => {
    try {
      return fs.readdirSync(folder).filter((n) => n.startsWith(`.pako-setup-check-${pid}-`));
    } catch {
      return [];
    }
  };
  const roots = new Set([path.parse(dir).root, "C:\\"]);
  for (const r of roots) assert.deepEqual(leftover(r), [], `nothing left in ${r}`);
  const walk = (d) =>
    fs.readdirSync(d, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? [path.join(d, e.name), ...walk(path.join(d, e.name))] : [path.join(d, e.name)]));
  const all = walk(dir).filter((p) => !p.startsWith(path.join(dir, "locked")));
  assert.deepEqual(all.filter((p) => p.includes(".pako-setup-check-")), [], "no test file left");
  for (const gone of ["missing", "new", "forward", path.join("cwd", "rel")]) assert.ok(!fs.existsSync(path.join(dir, gone)), `${gone} not made`);
});
