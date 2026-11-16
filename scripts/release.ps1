# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
# Makes a Pako release ready to upload: builds Pako.exe and Pako Setup, signs Pako.exe, writes
# latest.json.
#
#   1. Raise the version in src-tauri/Cargo.toml, src-tauri/tauri.conf.json AND package.json (same number).
#   2. powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "What's new"
#   3. Upload all six files in dist/<version>/ to a GitHub release tagged v<version> in
#      sookera/pako-releases (attached to the release, names unchanged):
#        Pako.exe        the portable Pako, and the file Pako's own updates download (both kinds)
#        Pako-Setup.exe  the installer (per user, no admin): same Pako.exe inside
#        latest.json     what Pako's update check reads (version, SHA-256, signatures)
#        help.zip        the guides (docs/help/*.md without AUDIT.md, flat): pako.sookera.com/help
#                        is built from the latest release's help.zip, so the site matches this Pako
#        LICENSE         the GNU GPL version 3, and NOTICE.md: the copyright and Sookera's
#        NOTICE.md       additional terms (they go with every copy; the installer puts them next to
#                        Pako.exe too, from bundle.resources in tauri.conf.json)
#
# The signing key is in %USERPROFILE%\.pako-signing (made once with: cargo run -- --release keygen
# in src-tauri). Keep a backup of it: without it, Pako can't take updates any more.
param(
    [string]$Notes = ""
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$cargo = Get-Content (Join-Path $root "src-tauri/Cargo.toml") -Raw
$version = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
$conf = Get-Content (Join-Path $root "src-tauri/tauri.conf.json") -Raw | ConvertFrom-Json
if ($conf.version -ne $version) {
    throw "Cargo.toml says $version but tauri.conf.json says $($conf.version): make them the same first."
}
$package = Get-Content (Join-Path $root "package.json") -Raw | ConvertFrom-Json
if ($package.version -ne $version) {
    throw "Cargo.toml says $version but package.json says $($package.version): make them the same first."
}
# What's new: Pako shows the section whose heading is its own version once after updating.
$whatsNew = Get-Content (Join-Path $root "docs/help/whats-new.md") -Raw
if ($whatsNew -match '(?m)^##\s+Next version\s*$') {
    throw "docs/help/whats-new.md still has '## Next version': rename that heading to '## $version' first."
}
if ($whatsNew -notmatch "(?m)^##\s+(Pako\s+)?$([regex]::Escape($version))\s*$") {
    throw "docs/help/whats-new.md has no '## $version' section: Pako would show no What's new after this update."
}
# Pako's Drive key must be built in (it lists public Drive folders): stop before building without it.
$keyFile = Join-Path $root "signing/drive_api_key.txt"
$key = $env:PAKO_DRIVE_API_KEY
if (-not $key) {
    if (-not (Test-Path $keyFile)) { throw "signing/drive_api_key.txt is missing: a release must have the Drive key built in." }
    $key = [System.IO.File]::ReadAllText($keyFile)
}
# A BOM in front (Notepad) isn't part of the key. The key itself is never printed.
$key = $key.TrimStart([char]0xFEFF).Trim()
if ($key -notmatch '^[A-Za-z0-9_-]+$') { throw "signing/drive_api_key.txt doesn't hold a valid key (one line: letters, digits, - and _)." }
Remove-Variable key
# Upload to Google Drive needs Sookera's sign-in built in (every Pako, not only this PC's): a
# Desktop-app client as Google's console saves it ({"installed": {...}}, or the fields at the top).
# Nothing in it is ever printed.
$clientFile = Join-Path $root "signing/google_send_client.json"
if (-not (Test-Path $clientFile)) {
    throw "signing/google_send_client.json is missing: a release must have the Google sign-in for uploads built in."
}
try { $client = [System.IO.File]::ReadAllText($clientFile).TrimStart([char]0xFEFF) | ConvertFrom-Json }
catch { throw "signing/google_send_client.json isn't valid JSON: save it again from Google's console." }
if ($client.installed) { $client = $client.installed }
if ([string]$client.client_id -cnotmatch '^\d+-[a-z0-9]+\.apps\.googleusercontent\.com$') {
    throw "signing/google_send_client.json has no valid client_id (<number>-<code>.apps.googleusercontent.com)."
}
if ([string]$client.client_secret -cnotmatch '^[A-Za-z0-9._-]+$') {
    throw "signing/google_send_client.json has no valid client_secret."
}
Remove-Variable client
# A release always has Google's features: building without them is only for other builders.
if ($env:PAKO_WITHOUT_GOOGLE) {
    throw "PAKO_WITHOUT_GOOGLE is set: a release must have the Google key and sign-in built in. Remove it first."
}
Write-Host "Building Pako $version..."
Push-Location $root
try {
    # Builds Pako.exe and the installer (bundle.targets in tauri.conf.json: NSIS only). The
    # installer holds the very same Pako.exe, so one update file serves both.
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { throw "The build failed." }
    $out = Join-Path $root "dist/$version"
    New-Item -ItemType Directory -Force $out | Out-Null
    $exe = Join-Path $root "src-tauri/target/release/Pako.exe"
    $setup = Join-Path $root "src-tauri/target/release/bundle/nsis/Pako_${version}_x64-setup.exe"
    if (-not (Test-Path $exe)) { throw "The build made no Pako.exe ($exe)." }
    if (-not (Test-Path $setup)) { throw "The build made no installer ($setup)." }
    # No personal paths in the program: Rust keeps source paths for its error messages, and only
    # the local src-tauri/.cargo/config.toml (--remap-path-prefix) keeps this PC's out. A RUSTFLAGS
    # variable replaces it, and another PC or folder has none: so the built exe is checked here
    # for this Windows user's name and folder and the project folder, in plain and UTF-16 text.
    $personal = @($root, $root.Replace([char]92, [char]47), $env:USERPROFILE, $env:USERPROFILE.Replace([char]92, [char]47))
    # (A very short user name would also match ordinary text: the folder checks cover it then.)
    if ($env:USERNAME.Length -ge 4) { $personal += $env:USERNAME }
    $bytes = [System.IO.File]::ReadAllBytes($exe)
    $texts = @(
        [System.Text.Encoding]::GetEncoding(28591).GetString($bytes),
        [System.Text.Encoding]::Unicode.GetString($bytes),
        [System.Text.Encoding]::Unicode.GetString($bytes, 1, $bytes.Length - 1)
    )
    Remove-Variable bytes
    foreach ($p in $personal | Where-Object { $_ }) {
        foreach ($t in $texts) {
            if ($t.IndexOf($p, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
                throw "Pako.exe contains '$p': it must not ship with this PC's paths. Check src-tauri/.cargo/config.toml (--remap-path-prefix for your user folder and the project folder) and that RUSTFLAGS isn't set, then build again."
            }
        }
    }
    Remove-Variable texts
    Copy-Item $exe (Join-Path $out "Pako.exe") -Force
    Copy-Item $setup (Join-Path $out "Pako-Setup.exe") -Force
    # The guides for the website (the same files Pako's Help shows), without the audit log.
    $help = Get-ChildItem (Join-Path $root "docs/help") -Filter *.md | Where-Object { $_.Name -ne "AUDIT.md" }
    if (-not ($help | Where-Object { $_.Name -eq "README.md" })) { throw "docs/help/README.md (the guide index) is missing." }
    $zip = Join-Path $out "help.zip"
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Compress-Archive -Path $help.FullName -DestinationPath $zip
    # The licence and its additional terms go with every copy (the portable Pako is one file).
    foreach ($legal in @("LICENSE", "NOTICE.md")) {
        $from = Join-Path $root $legal
        if (-not (Test-Path $from)) { throw "$legal is missing: a release must carry the licence." }
        Copy-Item $from (Join-Path $out $legal) -Force
    }
    Write-Host "Signing..."
    Push-Location (Join-Path $root "src-tauri")
    try {
        cargo run --quiet -- --release sign (Join-Path $out "Pako.exe") $version $Notes
        if ($LASTEXITCODE -ne 0) { throw "Signing failed." }
    } finally { Pop-Location }
    Write-Host ""
    Write-Host "Ready in $out :"
    Write-Host "  Pako.exe, Pako-Setup.exe, latest.json, help.zip, LICENSE, NOTICE.md"
    Write-Host "Upload all six to a GitHub release tagged v$version in sookera/pako-releases."
} finally { Pop-Location }
