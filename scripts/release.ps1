# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
# Makes a Pako release ready to upload: builds Pako.exe and Pako Setup, signs Pako.exe, writes
# latest.json.
#
#   1. Raise the version in src-tauri/Cargo.toml, src-tauri/tauri.conf.json AND package.json (same number).
#   2. powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "What's new"
#   3. Upload all the files in dist/<version>/ to a GitHub release tagged v<version> in
#      sookera/pako-releases (attached to the release, names unchanged):
#        Pako.exe        the portable Pako, and the file Pako's own updates download (both kinds)
#        Pako-Setup.exe  the installer (per user, no admin): same Pako.exe inside
#        latest.json     what Pako's update check reads (version, SHA-256, signatures)
#        help.zip        the guides (docs/help/*.md without AUDIT.md, flat): pako.sookera.com/help
#                        is built from the latest release's help.zip, so the site matches this Pako
#        LICENSE         the GNU GPL version 3, and NOTICE.md: the copyright and Sookera's
#        NOTICE.md       additional terms (they go with every copy; the installer puts them next to
#                        Pako.exe too, from bundle.resources in tauri.conf.json)
#        pako-firefox-<extension version>.xpi
#                        the Firefox extension signed by Mozilla (scripts/sign-firefox.ps1; the
#                        same file is built into Pako.exe)
#        pako-firefox.xpi
#                        the same file under a fixed name, for the website's one-click link
#                        (releases/latest/download/pako-firefox.xpi)
#        firefox-updates.json
#                        where Firefox looks for the extension's updates (update_url in
#                        extension/manifest.firefox.json: releases/latest/download/…), pointing at
#                        that .xpi in this release
#
# The extension changed since the last release? Raise its version (both extension manifests) and
# run scripts/sign-firefox.ps1 BEFORE this script: a release stops without the signed file for the
# extension's current version (-WithoutFirefox builds one without, saying Firefox isn't available,
# and with no Firefox updates).
#
# The signing key is in %USERPROFILE%\.pako-signing (made once with: cargo run -- --release keygen
# in src-tauri). Keep a backup of it: without it, Pako can't take updates any more.
param(
    [string]$Notes = "",
    [switch]$WithoutFirefox
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
# Fixes a user would notice go in one "### Fixed" group, the last of the version's section.
$sectionAt = '(?ms)^##[ \t]+(Pako[ \t]+)?' + [regex]::Escape($version) + '[ \t]*\r?$(.*?)(?=^##[ \t]|\z)'
$section = [regex]::Match($whatsNew, $sectionAt).Groups[2].Value
$groups = @([regex]::Matches($section, '(?m)^###[ \t]+(.+?)[ \t]*\r?$') | ForEach-Object { $_.Groups[1].Value })
if (-not $groups.Count) { throw "docs/help/whats-new.md: the '## $version' section has no '### ...' groups." }
$fixed = @(for ($i = 0; $i -lt $groups.Count; $i++) { if ($groups[$i] -eq "Fixed") { $i } })
if ($fixed.Count -gt 1) { throw "docs/help/whats-new.md: the '## $version' section has more than one '### Fixed' group." }
if ($fixed.Count -eq 1 -and $fixed[0] -ne $groups.Count - 1) { throw "docs/help/whats-new.md: '### Fixed' must be the last group of '## $version'." }
# The Firefox extension signed by Mozilla, for the extension's current version (src-tauri/build.rs
# builds the same file into Pako.exe).
$extManifest = Get-Content (Join-Path $root "extension/manifest.firefox.json") -Raw | ConvertFrom-Json
$extVersion = $extManifest.version
$extChromium = (Get-Content (Join-Path $root "extension/manifest.chromium.json") -Raw | ConvertFrom-Json).version
if ($extVersion -ne $extChromium) {
    throw "extension/manifest.firefox.json says $extVersion but manifest.chromium.json says ${extChromium}: make them the same first."
}
$xpiName = "pako-firefox-$extVersion.xpi"
$signedXpi = Join-Path $root "dist/extension/signed/$xpiName"
if (-not (Test-Path $signedXpi) -and -not $WithoutFirefox) {
    throw "No Firefox extension signed for version $extVersion ($signedXpi): run scripts/sign-firefox.ps1 first (or -WithoutFirefox for a release without Firefox)."
}
# The signed file must be the extension as it is now: one signed before the extension changed
# (same version) would ship the old extension to Firefox, inside Pako.exe and in the release.
if (Test-Path $signedXpi) {
    Push-Location $root
    try {
        node extension/build.mjs | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "The extension build failed." }
        node scripts/check-firefox-xpi.mjs $signedXpi (Join-Path $root "dist/extension/firefox")
        if ($LASTEXITCODE -ne 0) {
            throw "The signed Firefox extension ($xpiName) isn't the extension as it is now (above). Raise ""version"" in both extension manifests and run scripts/sign-firefox.ps1 again."
        }
    } finally { Pop-Location }
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
    # Pako.exe takes Windows' DLLs from System32 only (src-tauri/build.rs, system_dlls_only): a
    # file named like one of them next to a portable Pako.exe must never be loaded. Read from the
    # exe itself (its "load configuration", DependentLoadFlags = 0x800), so a build that lost the
    # linker setting isn't released.
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    $opt = $pe + 24
    $cfgRva = [BitConverter]::ToUInt32($bytes, $opt + 112 + 80)
    $sec = $opt + [BitConverter]::ToUInt16($bytes, $pe + 20)
    $cfgAt = -1
    for ($i = 0; $i -lt [BitConverter]::ToUInt16($bytes, $pe + 6); $i++) {
        $va = [BitConverter]::ToUInt32($bytes, $sec + 12)
        if ($cfgRva -ge $va -and $cfgRva -lt $va + [BitConverter]::ToUInt32($bytes, $sec + 8)) {
            $cfgAt = $cfgRva - $va + [BitConverter]::ToUInt32($bytes, $sec + 20)
        }
        $sec += 40
    }
    if ([BitConverter]::ToUInt16($bytes, $opt) -ne 0x20B -or $cfgAt -lt 0 -or [BitConverter]::ToUInt16($bytes, $cfgAt + 0x4E) -ne 0x800) {
        throw "Pako.exe would load DLLs from its own folder (DependentLoadFlags isn't 0x800): check system_dlls_only in src-tauri/build.rs, then build again."
    }
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
    # Firefox: the signed extension and the update file Firefox reads (Mozilla's update manifest:
    # https://extensionworkshop.com/documentation/manage/updating-your-extension/), pointing at the
    # .xpi in this very release, so it keeps working after later releases. (No "applications"
    # entry: Firefox then checks the extension's own strict_min_version.)
    if (Test-Path $signedXpi) {
        Copy-Item $signedXpi (Join-Path $out $xpiName) -Force
        Copy-Item $signedXpi (Join-Path $out "pako-firefox.xpi") -Force
        $hash = (Get-FileHash $signedXpi -Algorithm SHA256).Hash.ToLower()
        $id = $extManifest.browser_specific_settings.gecko.id
        $update = [ordered]@{
            version = $extVersion
            update_link = "https://github.com/sookera/pako-releases/releases/download/v$version/$xpiName"
            update_hash = "sha256:$hash"
        }
        $json = [ordered]@{ addons = [ordered]@{ $id = [ordered]@{ updates = @($update) } } } | ConvertTo-Json -Depth 8
        [System.IO.File]::WriteAllText((Join-Path $out "firefox-updates.json"), $json + "`n", (New-Object System.Text.UTF8Encoding $false))
    } else {
        Remove-Item (Join-Path $out "firefox-updates.json") -ErrorAction SilentlyContinue
    }
    Write-Host "Signing..."
    Push-Location (Join-Path $root "src-tauri")
    try {
        cargo run --quiet -- --release sign (Join-Path $out "Pako.exe") $version $Notes
        if ($LASTEXITCODE -ne 0) { throw "Signing failed." }
    } finally { Pop-Location }
    Write-Host ""
    Write-Host "Ready in $out :"
    $files = @("Pako.exe", "Pako-Setup.exe", "latest.json", "help.zip", "LICENSE", "NOTICE.md")
    if (Test-Path $signedXpi) { $files += @($xpiName, "pako-firefox.xpi", "firefox-updates.json") }
    Write-Host ("  " + ($files -join ", "))
    Write-Host "Upload all $($files.Count) to a GitHub release tagged v$version in sookera/pako-releases."
    if (-not (Test-Path $signedXpi)) { Write-Host "(No Firefox extension in this release: Firefox users get no extension update from it.)" }
} finally { Pop-Location }
