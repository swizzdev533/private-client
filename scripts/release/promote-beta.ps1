<#
.SYNOPSIS
    Promotes the current beta to the public stable release and opens the next
    beta cycle.

.DESCRIPTION
    The beta installer is NOT reused as the stable artifact. It carries its own
    product name, identifier, and data directory, so shipping it as stable would
    install a second app called "Private Client Beta" that writes to the beta
    instance. Promotion therefore rebuilds the same commit on the stable
    channel, which is what makes the tested code and the published code the
    same source.

    Steps:
      1. verify the tree is clean and the version is not already released;
      2. build, sign, and verify the stable channel;
      3. publish the public release and its manifest;
      4. bump to the next version and reopen the beta channel there.

    Requires TAURI_SIGNING_PRIVATE_KEY (and the passphrase) in the environment.
#>
[CmdletBinding()]
param(
    # Version for the next beta cycle. Defaults to bumping the minor component.
    [string]$NextVersion,
    [string]$Notes,
    [switch]$SkipPush
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$tauriConfPath = Join-Path $projectRoot 'apps\launcher\src-tauri\tauri.conf.json'
$bundleDir = Join-Path $projectRoot 'apps\launcher\src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis'

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

function Set-ProjectVersion {
    param([Parameter(Mandatory = $true)][string]$Version)

    $targets = @(
        @{ Path = $tauriConfPath; Pattern = '(?<=^\s*"version":\s*")\d+\.\d+\.\d+(?=")' },
        @{ Path = (Join-Path $projectRoot 'package.json'); Pattern = '(?<=^\s*"version":\s*")\d+\.\d+\.\d+(?=")' },
        @{ Path = (Join-Path $projectRoot 'apps\launcher\package.json'); Pattern = '(?<=^\s*"version":\s*")\d+\.\d+\.\d+(?=")' }
    )
    foreach ($target in $targets) {
        $content = Get-Content -LiteralPath $target.Path -Raw
        $updated = [regex]::Replace($content, $target.Pattern, $Version, 'Multiline')
        if ($updated -eq $content) {
            throw "Could not update the version in $($target.Path)."
        }
        [System.IO.File]::WriteAllText($target.Path, $updated, (New-Object System.Text.UTF8Encoding($false)))
    }

    # Cargo.toml: only the [package] version, never a dependency pin.
    $cargoPath = Join-Path $projectRoot 'apps\launcher\src-tauri\Cargo.toml'
    $cargo = Get-Content -LiteralPath $cargoPath -Raw
    $updatedCargo = [regex]::Replace(
        $cargo,
        '(?<=^\[package\][\s\S]*?^version\s*=\s*")\d+\.\d+\.\d+(?=")',
        $Version,
        'Multiline')
    if ($updatedCargo -eq $cargo) {
        throw "Could not update the package version in $cargoPath."
    }
    [System.IO.File]::WriteAllText($cargoPath, $updatedCargo, (New-Object System.Text.UTF8Encoding($false)))
}

if ([string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY) -and
    [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY_PATH)) {
    throw 'No updater signing key in the environment; refusing to promote an unsignable release.'
}

$gh = Resolve-GhExecutable
Push-Location $projectRoot
try {
    # A dirty tree means the stable build would not match what was tested as
    # beta, which defeats the entire point of promoting rather than rebuilding.
    $status = & git status --porcelain
    if ($LASTEXITCODE -ne 0) { throw 'git status failed.' }
    if ($status) {
        throw @"
The working tree has uncommitted changes. Commit or stash them first so the
promoted build matches the commit that was tested as beta.
$($status -join "`n")
"@
    }

    $version = (Get-Content -LiteralPath $tauriConfPath -Raw | ConvertFrom-Json).version
    $tag = "v$version"

    & $gh release view $tag 2>$null | Out-Null
    if ($LASTEXITCODE -eq 0) {
        throw "Release $tag already exists. Bump the version before promoting again."
    }

    if (-not $NextVersion) {
        $parts = $version.Split('.')
        $NextVersion = "$($parts[0]).$([int]$parts[1] + 1).0"
    }
    if ($NextVersion -notmatch '^\d+\.\d+\.\d+$') {
        throw "NextVersion '$NextVersion' must be MAJOR.MINOR.PATCH."
    }

    Write-Host "Promoting $version to stable; next beta cycle will be $NextVersion."
    Write-Host ''

    Write-Host '=== 1/4 building the stable channel ==='
    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' `
        '-File' (Join-Path $projectRoot 'scripts\build\build-launcher.ps1') '-Channel' 'stable'

    $productName = (Get-Content -LiteralPath $tauriConfPath -Raw | ConvertFrom-Json).productName
    $installerPath = Join-Path $bundleDir "${productName}_${version}_x64-setup.exe"
    $signaturePath = "$installerPath.sig"
    foreach ($required in @($installerPath, $signaturePath)) {
        if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
            throw "Missing stable artifact: $required"
        }
    }

    Write-Host '=== 2/4 verifying the release artifacts ==='
    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' `
        '-File' (Join-Path $projectRoot 'scripts\release\package-release.ps1')
    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' `
        '-File' (Join-Path $projectRoot 'scripts\verification\verify-release.ps1')

    $manifestPath = Join-Path $projectRoot 'artifacts\release\latest.json'
    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' `
        '-File' (Join-Path $PSScriptRoot 'build-update-manifest.ps1') `
        '-Tag' $tag '-Channel' 'stable' '-OutputPath' $manifestPath

    Write-Host '=== 3/4 publishing the public release ==='
    $releaseNotes = if ($Notes) { $Notes } else { "$productName $version" }
    Invoke-Checked 'git' 'tag' '-a' $tag '-m' "$productName $version"
    if (-not $SkipPush) {
        Invoke-Checked 'git' 'push' 'origin' $tag
    }
    Invoke-Checked $gh 'release' 'create' $tag $installerPath $signaturePath $manifestPath `
        '--title' "$productName $version" '--notes' $releaseNotes

    Write-Host '=== 4/4 opening the next beta cycle ==='
    Set-ProjectVersion -Version $NextVersion
    # Keep Cargo.lock in step with the bumped package version.
    Invoke-Checked 'cargo' 'update' '--manifest-path' `
        (Join-Path $projectRoot 'apps\launcher\src-tauri\Cargo.toml') '-p' 'private-client-launcher'

    Invoke-Checked 'git' 'add' '-A'
    Invoke-Checked 'git' 'commit' '-m' "Open the $NextVersion beta cycle after promoting $version"
    if (-not $SkipPush) {
        Invoke-Checked 'git' 'push' 'origin' 'HEAD'
    }

    Write-Host ''
    Write-Host "Promoted: $productName $version is now the public release."
    Write-Host "Next beta cycle: $NextVersion. Run 'pnpm release:beta' to publish it."
}
finally {
    Pop-Location
}
