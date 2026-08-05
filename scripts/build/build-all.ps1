[CmdletBinding()]
param(
    [switch]$SkipInstall,
    [switch]$SkipCore
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$launcherDir = Join-Path $projectRoot 'apps\launcher'
$associationDir = Join-Path $projectRoot 'apps\association-api'
$tauriManifest = Join-Path $launcherDir 'src-tauri\Cargo.toml'
$coreScript = Join-Path $projectRoot 'minecraft\private-client-core\scripts\build-core.ps1'
$coreJar = Join-Path $projectRoot 'minecraft\private-client-core\build\libs\private-client-core-1.0.0.jar'

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

Push-Location $projectRoot
try {
    if (-not $SkipInstall) {
        Invoke-Checked 'pnpm.cmd' 'install' '--frozen-lockfile'
    }

    Invoke-Checked 'pnpm.cmd' '--dir' $launcherDir 'lint'
    Invoke-Checked 'pnpm.cmd' '--dir' $launcherDir 'typecheck'
    Invoke-Checked 'pnpm.cmd' '--dir' $launcherDir 'test'
    Invoke-Checked 'pnpm.cmd' '--dir' $launcherDir 'build'

    # The association API is a shipped trust boundary, so it is verified in the
    # same pass rather than being left to a separate deploy.
    Invoke-Checked 'pnpm.cmd' '--dir' $associationDir 'typecheck'
    Invoke-Checked 'pnpm.cmd' '--dir' $associationDir 'test'

    # The Rust binary embeds the verified Core JAR at compile time, so the Core
    # must exist before any cargo command invokes tauri-build.
    if (-not $SkipCore) {
        if (-not (Test-Path -LiteralPath $coreScript -PathType Leaf)) {
            throw "Core build script is missing: $coreScript"
        }
        Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' '-File' $coreScript
    }
    elseif (-not (Test-Path -LiteralPath $coreJar -PathType Leaf)) {
        throw "SkipCore requires an existing verified Core JAR: $coreJar"
    }

    Invoke-Checked 'cargo.exe' 'fmt' '--manifest-path' $tauriManifest '--check'
    Invoke-Checked 'cargo.exe' 'clippy' '--manifest-path' $tauriManifest '--all-targets' '--' '-D' 'warnings'
    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' '-File' `
        (Join-Path $projectRoot 'scripts\build\test-rust.ps1')

    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' '-File' `
        (Join-Path $projectRoot 'scripts\build\build-launcher.ps1')
    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' '-File' (Join-Path $projectRoot 'scripts\release\package-release.ps1')
    Invoke-Checked 'powershell.exe' '-NoProfile' '-ExecutionPolicy' 'Bypass' '-File' (Join-Path $projectRoot 'scripts\verification\verify-release.ps1')
}
finally {
    Pop-Location
}
