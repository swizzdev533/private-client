<#
.SYNOPSIS
    Builds and publishes the private beta client.

.DESCRIPTION
    Builds the beta channel (its own product name, identifier, data directory,
    and update endpoint), signs the updater artifact, and republishes the fixed
    `beta` pre-release in place.

    The beta lives under one permanent tag so its endpoint URL never changes.
    It is published as a pre-release, never a normal release: GitHub's
    `/releases/latest/` deliberately skips pre-releases, which is what keeps
    every stable installation from ever seeing a beta build.

    Requires TAURI_SIGNING_PRIVATE_KEY (and the passphrase) in the environment.
#>
[CmdletBinding()]
param(
    [string]$Notes
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$tauriConfPath = Join-Path $projectRoot 'apps\launcher\src-tauri\tauri.conf.json'
$bundleDir = Join-Path $projectRoot 'apps\launcher\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis'

function Resolve-GhExecutable {
    $candidate = Join-Path $env:ProgramFiles 'GitHub CLI\gh.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        return $candidate
    }
    $command = Get-Command 'gh' -CommandType Application -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }
    throw 'GitHub CLI (gh) is required to publish a release.'
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments
    )
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Executable exited with code $LASTEXITCODE"
    }
}

if ([string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY) -and
    [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY_PATH)) {
    throw @'
No updater signing key in the environment. An unsigned beta cannot update
itself, because the launcher verifies every artifact against the pinned key.
Set TAURI_SIGNING_PRIVATE_KEY (and TAURI_SIGNING_PRIVATE_KEY_PASSWORD) first.
'@
}

$gh = Resolve-GhExecutable
$version = (Get-Content -LiteralPath $tauriConfPath -Raw | ConvertFrom-Json).version
$productName = (Get-Content -LiteralPath (Join-Path $projectRoot 'apps\launcher\src-tauri\tauri.beta.conf.json') -Raw |
    ConvertFrom-Json).productName

Write-Host "Building $productName $version..."
Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' `
    '-File' (Join-Path $projectRoot 'scripts\build\build-launcher.ps1') '-Channel' 'beta'

$installerPath = Join-Path $bundleDir "${productName}_${version}_x64-setup.exe"
if (-not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
    throw "Beta installer not found: $installerPath"
}
$signaturePath = "$installerPath.sig"
if (-not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
    throw "Beta installer was not signed: $signaturePath is missing."
}

$manifestPath = Join-Path $projectRoot 'artifacts\release\beta.json'
Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' `
    '-File' (Join-Path $PSScriptRoot 'build-update-manifest.ps1') `
    '-Tag' 'beta' '-Channel' 'beta' '-OutputPath' $manifestPath

$releaseNotes = if ($Notes) { $Notes } else { "$productName $version - prywatna wersja testowa." }

# "Release not found" is the expected answer before the first beta, but gh
# reports it on stderr, and Windows PowerShell turns a native command's stderr
# into an ErrorRecord that $ErrorActionPreference='Stop' escalates into a
# terminating error. Suppress every stream and judge purely by the exit code.
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
    & $gh release view beta *> $null
    $exists = ($LASTEXITCODE -eq 0)
}
finally {
    $ErrorActionPreference = $previousPreference
}

if ($exists) {
    Write-Host 'Replacing the assets on the existing beta pre-release...'
    Invoke-Checked $gh 'release' 'upload' 'beta' $installerPath $signaturePath $manifestPath '--clobber'
    Invoke-Checked $gh 'release' 'edit' 'beta' '--prerelease' '--notes' $releaseNotes
}
else {
    Write-Host 'Creating the beta pre-release...'
    Invoke-Checked $gh 'release' 'create' 'beta' $installerPath $signaturePath $manifestPath `
        '--prerelease' '--title' "$productName (test)" '--notes' $releaseNotes
}

Write-Host ''
Write-Host "Published $productName $version to the beta channel."
Write-Host 'Stable installations are unaffected: /releases/latest/ skips pre-releases.'
