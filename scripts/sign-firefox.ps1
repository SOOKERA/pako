# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
# Gets the Firefox version of Pako's extension signed by Mozilla ("unlisted": nobody else sees it
# on addons.mozilla.org; Pako ships the signed file itself). Run it whenever the extension changed,
# after raising "version" in extension/manifest.chromium.json AND extension/manifest.firefox.json:
#
#   powershell -ExecutionPolicy Bypass -File scripts/sign-firefox.ps1
#
# It builds the extension, sends dist/extension/firefox/ to Mozilla and waits for the signed file
# (usually a few minutes), then leaves it at:
#   dist/extension/signed/pako-firefox-<version>.xpi   kept, one per version: Pako's build puts the
#                                                      current one inside Pako.exe (src-tauri/build.rs)
#                                                      and scripts/release.ps1 adds it to the release
#   dist/extension/pako-firefox.xpi                    the latest one, to try by hand
#
# Mozilla signs each version only once, so it stops when this version is already signed.
# Mozilla's API login is read from the gitignored signing/amo-api.txt (two lines: issuer=… and
# secret=…). It is handed to web-ext only through this process's environment and never printed.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $versionOf = { param($f) (Get-Content (Join-Path $root "extension/$f") -Raw | ConvertFrom-Json).version }
    $version = & $versionOf "manifest.firefox.json"
    $chromium = & $versionOf "manifest.chromium.json"
    if ($version -ne $chromium) {
        throw "manifest.firefox.json says $version but manifest.chromium.json says ${chromium}: make them the same first."
    }
    $signedDir = Join-Path $root "dist/extension/signed"
    $signed = Join-Path $signedDir "pako-firefox-$version.xpi"
    if (Test-Path $signed) {
        Write-Host "Version $version is already signed ($signed)."
        Write-Host "Mozilla signs each version only once: if the extension changed, raise ""version"" in extension/manifest.chromium.json and extension/manifest.firefox.json, then run this again."
        exit 1
    }

    $keyFile = Join-Path $root "signing/amo-api.txt"
    if (-not (Test-Path $keyFile)) { throw "signing/amo-api.txt is missing (two lines: issuer=... and secret=..., from addons.mozilla.org › Developer Hub › Manage API Keys)." }
    $issuer = $null; $secret = $null
    foreach ($line in [System.IO.File]::ReadAllLines($keyFile)) {
        $l = $line.TrimStart([char]0xFEFF).Trim()
        if ($l -match '^issuer\s*=\s*(.+)$') { $issuer = $Matches[1].Trim() }
        elseif ($l -match '^secret\s*=\s*(.+)$') { $secret = $Matches[1].Trim() }
    }
    $Matches = $null
    if (-not $issuer -or -not $secret) { throw "signing/amo-api.txt needs two lines: issuer=... and secret=... (nothing else is read)." }

    Write-Host "Building the extension..."
    node extension/build.mjs
    if ($LASTEXITCODE -ne 0) { throw "The extension build failed." }
    $source = Join-Path $root "dist/extension/firefox"

    Write-Host "Checking it the way Mozilla does..."
    npx --no-install web-ext lint --self-hosted --source-dir $source
    if ($LASTEXITCODE -ne 0) { throw "Mozilla's checker found a problem (above): fix it first." }

    Write-Host "Sending Pako $version to Mozilla for signing (unlisted). This usually takes a few minutes..."
    $work = Join-Path $root ("dist/extension/signing-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force $work | Out-Null
    # Only this process (and web-ext, started from it) sees the login; cleared right after.
    $env:WEB_EXT_API_KEY = $issuer
    $env:WEB_EXT_API_SECRET = $secret
    Remove-Variable issuer, secret
    try {
        # (web-ext writes its progress to stderr: shown as it comes, and kept to explain a refusal.)
        $ErrorActionPreference = "Continue"
        $said = npx --no-install web-ext sign --channel unlisted --source-dir $source --artifacts-dir $work --no-input 2>&1 |
            ForEach-Object { $text = "$_"; Write-Host $text; $text }
        $code = $LASTEXITCODE
        $ErrorActionPreference = "Stop"
    } finally {
        Remove-Item Env:WEB_EXT_API_KEY, Env:WEB_EXT_API_SECRET -ErrorAction SilentlyContinue
    }
    $xpi = Get-ChildItem $work -Filter *.xpi -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($code -ne 0 -or -not $xpi) {
        Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
        $said = $said -join "`n"
        if ($said -match "Duplicate add-on ID") {
            throw "Mozilla refused: the extension's ID (extension@pako.sookera.com) belongs to another addons.mozilla.org account, or to an add-on that was deleted. Use the API key of the account that owns it, or give the extension a new ID (manifest.firefox.json and FIREFOX_ID in src-tauri/src/extension.rs)."
        }
        if ($said -match "already exists|Version .* already") {
            throw "Mozilla already has version ${version}: raise ""version"" in both extension manifests and run this again (or download that signed file from the add-on's page on addons.mozilla.org and save it as $signed)."
        }
        throw "Mozilla didn't send back a signed file (see above). If it was still waiting for Mozilla's check, download the signed file later from the add-on's page on addons.mozilla.org and save it as $signed."
    }
    New-Item -ItemType Directory -Force $signedDir | Out-Null
    Move-Item $xpi.FullName $signed
    Remove-Item $work -Recurse -Force
    Copy-Item $signed (Join-Path $root "dist/extension/pako-firefox.xpi") -Force
    Write-Host ""
    Write-Host "Signed: $signed"
    Write-Host "Next Pako build puts it inside Pako.exe; scripts/release.ps1 adds it (and firefox-updates.json) to the release."
} finally { Pop-Location }
