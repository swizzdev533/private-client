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
    # Release tag as published on the release host, e.g. `v1.2.0`.
    [Parameter(Mandatory = $true)][string]$Tag,
    [string]$Notes,
    [string]$OutputPath
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$tauriConfPath = Join-Path $projectRoot 'apps\launcher\src-tauri\tauri.conf.json'
$bundleDir = Join-Path $projectRoot 'apps\launcher\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis'

if (-not $OutputPath) {
    $OutputPath = Join-Path $projectRoot 'artifacts\release\latest.json'
}

# The tag must match the version the binary reports, or the updater would
# advertise a version the installed launcher can never satisfy.
if ($Tag -notmatch '^v(?<version>\d+\.\d+\.\d+)$') {
    throw "Tag '$Tag' must be of the form vMAJOR.MINOR.PATCH."
}
$tagVersion = $Matches['version']

$tauriConf = Get-Content -LiteralPath $tauriConfPath -Raw | ConvertFrom-Json
$version = $tauriConf.version
if ($version -ne $tagVersion) {
    throw "Tag '$Tag' does not match tauri.conf.json version '$version'."
}

$endpoint = $tauriConf.plugins.updater.endpoints[0]
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

$installer = Get-ChildItem -LiteralPath $bundleDir -File -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
if ($null -eq $installer) {
    throw "No NSIS installer found in $bundleDir. Run pnpm build first."
}
if ($installer.Name -notlike "*$version*") {
    throw "Installer '$($installer.Name)' does not carry version $version."
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

# The release asset name is the installer file name URL-encoded; GitHub serves
# it verbatim under the tag, so the manifest URL must match byte for byte.
$assetName = [System.Uri]::EscapeDataString($installer.Name)
$downloadUrl = "https://github.com/$repositorySlug/releases/download/$Tag/$assetName"

$hash = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA512).Hash

$manifest = [ordered]@{
    version   = $version
    notes     = if ($Notes) { $Notes } else { "Private Client $version" }
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
