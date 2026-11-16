# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
#
# The steps after a release is published on GitHub (all files uploaded), for Sookera's own setup:
#   1. checks that the latest release is this version and carries help.zip;
#   2. creates the report label v<version> in sookera/pako-reports (the report service only adds a
#      version label that already exists);
#   3. asks Cloudflare to rebuild pako.sookera.com (the deploy hook URL is in the gitignored
#      signing/deploy-hook.txt), so the site shows this version and its guides;
#   4. publishes the public source copy (scripts/publish-source.ps1).
#
#   $env:GH_TOKEN = (gh auth token --user Zak-SKR)
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/after-publish.ps1
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$cargo = Get-Content -Raw "src-tauri/Cargo.toml"
$version = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if (-not $version) { throw "Couldn't read the version from src-tauri/Cargo.toml." }

# 1. The release is out, with help.zip (the site's build stops without it).
$latest = Invoke-RestMethod "https://github.com/sookera/pako-releases/releases/latest/download/latest.json"
if ($latest.version -ne $version) { throw "The latest release is $($latest.version), not ${version}: publish it first." }
$help = Invoke-WebRequest -Method Head -UseBasicParsing "https://github.com/sookera/pako-releases/releases/latest/download/help.zip"
if ($help.StatusCode -ne 200) { throw "The latest release has no help.zip: upload it first." }
Write-Host "Release $version is out, with help.zip."

# 2. The report label.
$labels = gh label list -R sookera/pako-reports --limit 200 --json name -q ".[].name"
if ($LASTEXITCODE) { throw "Couldn't read the labels of sookera/pako-reports (is GH_TOKEN set?)." }
if ($labels -contains "v$version") {
    Write-Host "Report label v$version is already there."
} else {
    gh label create "v$version" -R sookera/pako-reports --description "Reports from Pako $version" --color "F96263"
    if ($LASTEXITCODE) { throw "Couldn't create the report label v$version." }
    Write-Host "Report label v$version created."
}

# 3. Rebuild the website.
$hookFile = Join-Path $root "signing/deploy-hook.txt"
if (-not (Test-Path $hookFile)) { throw "No signing/deploy-hook.txt: ask for the deploy hook (it's in the vault)." }
$hook = (Get-Content -Raw $hookFile).Trim()
Invoke-RestMethod -Method Post -Uri $hook | Out-Null
Write-Host "pako.sookera.com is rebuilding (a few minutes)."

# 4. The public source copy.
& (Join-Path $PSScriptRoot "publish-source.ps1")
