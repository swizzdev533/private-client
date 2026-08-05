<#
.SYNOPSIS
    Builds and verifies the signed `latest.json` update manifest.

.DESCRIPTION
    Reads the NSIS installer and its minisign signature produced by
    `createUpdaterArtifacts`, then emits the exact manifest the launcher's
    updater expects. Every field is derived from a real artifact on disk:
    the version comes from tauri.conf.json, the signature from the `.sig`
    written by the signing key, and the URL from the pinned release host and
    the tag being published.

    This script never signs anything and never contacts the network. It fails
    loudly rather than emitting a manifest that points at a missing or
    unsigned artifact, because a manifest is what the updater trusts.
#>
[CmdletBinding()]
param(
    # Release tag as published on the release host. Stable uses `vMAJOR.MINOR.PATCH`;
    # beta uses the fixed tag `beta`, which is republished in place each build.
    [Parameter(Mandatory = $true)][string]$Tag,
    [ValidateSet('stable', 'beta')]
    [string]$Channel = 'stable',
    [string]$Notes,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$tauriConfPath = Join-Path $projectRoot 'apps\launcher\src-tauri\tauri.conf.json'
$betaConfPath = Join-Path $projectRoot 'apps\launcher\src-tauri\tauri.beta.conf.json'
$bundleDir = Join-Path $projectRoot 'apps\launcher\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis'

if (-not $OutputPath) {
    $manifestName = if ($Channel -eq 'beta') { 'beta.json' } else { 'latest.json' }
    $OutputPath = Join-Path $projectRoot "artifacts\release\$manifestName"
}

$tauriConf = Get-Content -LiteralPath $tauriConfPath -Raw | ConvertFrom-Json
$version = $tauriConf.version

if ($Channel -eq 'beta') {
    # The beta channel lives under one permanent tag so the endpoint URL never
    # changes; each build replaces the assets in place.
    if ($Tag -ne 'beta') {
        throw "The beta channel must publish under the fixed tag 'beta', not '$Tag'."
    }
}
else {
    # The tag must match the version the binary reports, or the updater would
    # advertise a version the installed launcher can never satisfy.
    if ($Tag -notmatch '^v(?<version>\d+\.\d+\.\d+)$') {
        throw "Tag '$Tag' must be of the form vMAJOR.MINOR.PATCH."
    }
    if ($version -ne $Matches['version']) {
        throw "Tag '$Tag' does not match tauri.conf.json version '$version'."
    }
}

# The endpoint is channel-specific: the beta overlay repoints it, and writing a
# beta manifest against the stable endpoint would publish a beta build to every
# stable installation.
$endpoint = if ($Channel -eq 'beta') {
    $betaConf = Get-Content -LiteralPath $betaConfPath -Raw | ConvertFrom-Json
    $betaConf.plugins.updater.endpoints[0]
}
else {
    $tauriConf.plugins.updater.endpoints[0]
}
if ([string]::IsNullOrWhiteSpace($endpoint)) {
    throw 'No updater endpoint is configured; refusing to build a manifest.'
}
if ($endpoint -notmatch '^https://github\.com/(?<slug>[^/]+/[^/]+)/releases/') {
    throw "Updater endpoint '$endpoint' is not a recognized GitHub release endpoint."
}
$repositorySlug = $Matches['slug']

if ([string]::IsNullOrWhiteSpace($tauriConf.plugins.updater.pubkey)) {
    throw 'No updater public key is pinned; refusing to build a manifest.'
}

# Both channels bundle into the same directory, so "newest setup.exe" is not a
# safe selector: building stable and then beta would make the stable manifest
# point at the beta installer. Match the exact product name of this channel.
$productName = if ($Channel -eq 'beta') {
    (Get-Content -LiteralPath $betaConfPath -Raw | ConvertFrom-Json).productName
}
else {
    $tauriConf.productName
}
$expectedName = "${productName}_${version}_x64-setup.exe"

$installer = Get-ChildItem -LiteralPath $bundleDir -File -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -eq $expectedName } |
    Select-Object -First 1
if ($null -eq $installer) {
    $present = (Get-ChildItem -LiteralPath $bundleDir -File -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
        ForEach-Object { $_.Name }) -join ', '
    throw @"
No installer named '$expectedName' in $bundleDir.
Present: $(if ($present) { $present } else { '(none)' })
Build the '$Channel' channel first.
"@
}

$signaturePath = "$($installer.FullName).sig"
if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
    throw @"
No signature found next to $($installer.Name).
Set TAURI_SIGNING_PRIVATE_KEY (and TAURI_SIGNING_PRIVATE_KEY_PASSWORD when the
key is protected) before pnpm build so the updater artifact is signed.
"@
}

$signature = (Get-Content -LiteralPath $signaturePath -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($signature)) {
    throw "Signature file $signaturePath is empty."
}

# GitHub rewrites release asset names on upload: every character outside
# [A-Za-z0-9._-] becomes a dot, so "Private Client_1.0.0_x64-setup.exe" is
# served as "Private.Client_1.0.0_x64-setup.exe". URL-encoding the original
# name instead produces a link that 404s — and it 404s only in production,
# against a published release, which is precisely where it must not.
$assetName = [System.Text.RegularExpressions.Regex]::Replace($installer.Name, '[^A-Za-z0-9._-]', '.')
$downloadUrl = "https://github.com/$repositorySlug/releases/download/$Tag/$assetName"

$hash = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA512).Hash

$manifest = [ordered]@{
    version   = $version
    notes     = if ($Notes) { $Notes } else { "$productName $version" }
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = $signature
            url       = $downloadUrl
        }
    }
}

$outputDir = Split-Path -Parent $OutputPath
if (-not (Test-Path -LiteralPath $outputDir)) {
    New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
}
# Write BOM-less UTF-8 explicitly: Windows PowerShell's `Out-File -Encoding utf8`
# emits a BOM, and the updater's JSON parser rejects a manifest that starts with
# one — which would fail only in production, against a published release.
$json = $manifest | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText($OutputPath, $json, (New-Object System.Text.UTF8Encoding($false)))

Write-Host "Update manifest written to: $OutputPath"
Write-Host "  version:   $version"
Write-Host "  installer: $($installer.Name)"
Write-Host "  sha512:    $hash"
Write-Host "  url:       $downloadUrl"
Write-Host ''
Write-Host 'Publish the installer, its .sig, and latest.json as assets on the same release.'
