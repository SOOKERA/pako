# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
# Makes a Pako release ready to upload: builds Pako.exe and Pako Setup, signs Pako.exe, writes
# latest.json.
#
#   1. Raise the version in src-tauri/Cargo.toml, src-tauri/tauri.conf.json AND package.json (same number).
#   2. powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Notes "What's new"
#   3. Upload all the files in dist/<version>/ to a DRAFT GitHub release tagged v<version> in
#      sookera/pako-releases (names unchanged), check it has every file, THEN publish it: the repo
#      has immutable releases (since 2026-11-18), so no file can be added or changed once published:
#        Pako.exe        the portable Pako, and the file Pako's own updates download (both kinds)
#        Pako-Setup.exe  the installer (per user, no admin): same Pako.exe inside
#        latest.json     what Pako's update check reads (version, SHA-256, signatures)
#        help.zip        the guides (docs/help/*.md without AUDIT.md, at the top; their French,
#                        docs/help/fr/*.md without TRADUCTION.md, in fr/): pako.sookera.com/help
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
#
# Before building, the libraries Pako is made of are checked (a release stops on a problem):
#   cargo audit   no known vulnerability in src-tauri/Cargo.lock (its "unmaintained" / "unsound"
#                 warnings are shown, never a stop)
#   cargo deny    what is built for Windows: no known vulnerability, only licences that may ship
#                 in a GPL-3.0 program, no OpenSSL / native-tls, no "any version" dependency,
#                 crates from crates.io only (src-tauri/deny.toml; cargo-deny 0.19 or newer)
#   npm audit     no known vulnerability (moderate or worse) in the npm packages, except the
#                 ones checked by hand in scripts/check-npm-audit.mjs
# Install the two cargo tools once with: cargo install cargo-audit cargo-deny --locked
# All three need the internet (the advisory lists). -SkipSupplyChecks builds without them, loudly,
# for an emergency only.
param(
    [string]$Notes = "",
    [switch]$WithoutFirefox,
    [switch]$SkipSupplyChecks
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
# French What's new: the same '## ' headings as the English (else the version shows in English).
# Checked here, before anything is built.
$frNew = Join-Path $root "docs/help/fr/whats-new.md"
if (Test-Path $frNew) {
    $headsOf = { param($path) @([regex]::Matches([IO.File]::ReadAllText($path), '(?m)^##\s+(.+?)\s*$') | ForEach-Object { $_.Groups[1].Value }) -join " | " }
    $enHeads = & $headsOf (Join-Path $root "docs/help/whats-new.md")
    $frHeads = & $headsOf $frNew
    if ($frHeads -ne $enHeads) { throw "docs/help/fr/whats-new.md must have the same '## ' headings as docs/help/whats-new.md (rename '## Next version' in both). English: $enHeads. French: $frHeads." }
}
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
# The libraries Pako is made of (see the top of this file).
if ($SkipSupplyChecks) {
    Write-Host ""
    Write-Host "  -SkipSupplyChecks: the libraries were NOT checked (cargo audit, cargo deny, npm audit)." -ForegroundColor Yellow
    Write-Host ""
} else {
    if (-not (Get-Command cargo-audit -ErrorAction SilentlyContinue)) {
        throw "cargo audit isn't installed, so the Rust libraries can't be checked for known vulnerabilities. Install it once with: cargo install cargo-audit --locked"
    }
    $denyVersion = $null
    if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
        # (Windows PowerShell stops on any stderr line under "2>" when errors stop the script.)
        $was = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try { $said = (cargo deny --version 2>$null) | Out-String } finally { $ErrorActionPreference = $was }
        $m = [regex]::Match($said, '(\d+)\.(\d+)\.(\d+)')
        if ($m.Success) { $denyVersion = [version]"$($m.Groups[1].Value).$($m.Groups[2].Value).$($m.Groups[3].Value)" }
    }
    if (-not $denyVersion -or $denyVersion -lt [version]"0.19.0") {
        throw "cargo deny 0.19 or newer is needed to check the Rust libraries' licences and sources (src-tauri/deny.toml). Install or update it with: cargo install cargo-deny --locked"
    }
    Push-Location (Join-Path $root "src-tauri")
    try {
        Write-Host "Checking the Rust libraries for known vulnerabilities (cargo audit)..."
        # It reads all of Cargo.lock, Linux-only crates too (their warnings show); --target-os /
        # --target-arch only leave out advisories that are about another system.
        cargo audit --target-os windows --target-arch x86_64
        if ($LASTEXITCODE -ne 0) { throw "cargo audit found a known vulnerability in a Rust library (above), or couldn't read the advisories (offline?). Update that library (cargo update -p <crate>, or a newer version in Cargo.toml) or replace it. Pako wasn't built." }
        Write-Host "Checking the Rust libraries' licences, vulnerabilities, bans and sources (cargo deny $denyVersion)..."
        cargo deny check
        if ($LASTEXITCODE -ne 0) { throw "cargo deny found a Rust library whose licence can't ship in a GPL-3.0 program, a known vulnerability, a banned crate or one from somewhere other than crates.io (above; src-tauri/deny.toml). Pako wasn't built." }
    } finally { Pop-Location }
    Push-Location $root
    try {
        Write-Host "Checking the npm packages for known vulnerabilities (npm audit)..."
        node scripts/check-npm-audit.mjs
        if ($LASTEXITCODE -ne 0) { throw "npm audit found a known vulnerability in an npm package (above), or couldn't run (offline?). Update the package, or, only after reading the advisory against Pako's code, add it with the reason to ALLOWED in scripts/check-npm-audit.mjs. Pako wasn't built." }
    } finally { Pop-Location }
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
    # The guides for the website (the same files Pako's Help shows), without the audit log, and
    # their French under fr/ (docs/help/fr/*.md, without TRADUCTION.md: the translators' note).
    # help.zip: README.md, downloading.md... at the top, fr/README.md, fr/downloading.md... below.
    # Zipped by hand so the names inside use "/" (Windows PowerShell's Compress-Archive writes "fr\").
    $helpDir = Join-Path $root "docs/help"
    $help = @(Get-ChildItem $helpDir -Filter *.md -File | Where-Object { $_.Name -ne "AUDIT.md" })
    if (-not ($help | Where-Object { $_.Name -eq "README.md" })) { throw "docs/help/README.md (the guide index) is missing." }
    $frDir = Join-Path $helpDir "fr"
    $helpFr = @(if (Test-Path $frDir) { Get-ChildItem $frDir -Filter *.md -File | Where-Object { $_.Name -ne "TRADUCTION.md" } })
    $zip = Join-Path $out "help.zip"
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::Open($zip, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($f in $help) { [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $f.FullName, $f.Name, [IO.Compression.CompressionLevel]::Optimal) }
        foreach ($f in $helpFr) { [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $f.FullName, "fr/$($f.Name)", [IO.Compression.CompressionLevel]::Optimal) }
    } finally {
        $archive.Dispose()
    }
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
    Write-Host "Upload all $($files.Count) to a DRAFT release tagged v$version in sookera/pako-releases, check them, then publish it."
    Write-Host "(Immutable releases: once published, no file can be added or changed.)"
    if (-not (Test-Path $signedXpi)) { Write-Host "(No Firefox extension in this release: Firefox users get no extension update from it.)" }
} finally { Pop-Location }
