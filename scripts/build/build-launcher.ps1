[CmdletBinding()]
param(
    [switch]$ForceXwin
)

$ErrorActionPreference = 'Stop'
$targetTriple = 'x86_64-pc-windows-msvc'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$launcherDir = Join-Path $projectRoot 'apps\launcher'

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

function Get-RustHostTriple {
    $versionOutput = & 'rustc.exe' '-vV'
    if ($LASTEXITCODE -ne 0) {
        throw "rustc -vV exited with code $LASTEXITCODE"
    }

    $hostLine = $versionOutput | Where-Object { $_ -like 'host: *' } | Select-Object -First 1
    if (-not $hostLine) {
        throw 'Unable to determine the active Rust host triple.'
    }

    return ($hostLine -replace '^host:\s*', '').Trim()
}

function Import-VisualStudioEnvironment {
    $vsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vsWhere -PathType Leaf)) {
        return $false
    }

    $installationPath = & $vsWhere -latest -products '*' `
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
        -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $installationPath) {
        return $false
    }

    $devCommand = Join-Path ($installationPath | Select-Object -First 1) 'Common7\Tools\VsDevCmd.bat'
    if (-not (Test-Path -LiteralPath $devCommand -PathType Leaf)) {
        return $false
    }

    $environmentLines = & $env:ComSpec /d /s /c "`"$devCommand`" -no_logo -arch=x64 -host_arch=x64 >nul && set"
    if ($LASTEXITCODE -ne 0) {
        return $false
    }

    foreach ($line in $environmentLines) {
        $separator = $line.IndexOf('=')
        if ($separator -le 0) {
            continue
        }

        $name = $line.Substring(0, $separator)
        $value = $line.Substring($separator + 1)
        [Environment]::SetEnvironmentVariable($name, $value, 'Process')
    }

    return $true
}

function Test-NativeMsvcEnvironment {
    if ((Get-RustHostTriple) -ne $targetTriple) {
        return $false
    }

    $requiredCommands = @('cl.exe', 'link.exe', 'rc.exe')
    $missing = $requiredCommands | Where-Object {
        -not (Get-Command $_ -CommandType Application -ErrorAction SilentlyContinue)
    }
    if ($missing.Count -eq 0) {
        return $true
    }

    if (-not (Import-VisualStudioEnvironment)) {
        return $false
    }

    $missing = $requiredCommands | Where-Object {
        -not (Get-Command $_ -CommandType Application -ErrorAction SilentlyContinue)
    }
    return $missing.Count -eq 0
}

function Assert-XwinEnvironment {
    $requiredCommands = @('cargo-xwin.exe', 'clang.exe', 'llvm-lib.exe', 'llvm-rc.exe')
    foreach ($command in $requiredCommands) {
        if (-not (Get-Command $command -CommandType Application -ErrorAction SilentlyContinue)) {
            throw "The cargo-xwin fallback requires $command on PATH."
        }
    }

    $installedTargets = & 'rustup.exe' 'target' 'list' '--installed'
    if ($LASTEXITCODE -ne 0) {
        throw "rustup target list --installed exited with code $LASTEXITCODE"
    }
    if ($installedTargets -notcontains $targetTriple) {
        throw "The cargo-xwin fallback requires the Rust target $targetTriple."
    }

    if (-not (Get-Command 'lld-link.exe' -CommandType Application -ErrorAction SilentlyContinue)) {
        $targetLibOutput = & 'rustc.exe' '--print' 'target-libdir'
        $rustcExitCode = $LASTEXITCODE
        # Trim only after the exit-code check: a failed rustc emits nothing, and
        # calling .Trim() on $null throws before the intended message is reached.
        $targetLibDir = if ($rustcExitCode -eq 0) { ($targetLibOutput | Select-Object -First 1) } else { $null }
        if (-not $targetLibDir) {
            throw 'Unable to locate the Rust LLVM linker for the cargo-xwin fallback.'
        }
        $targetLibDir = $targetLibDir.Trim()
        $rustLld = Join-Path (Split-Path -Parent $targetLibDir) 'bin\rust-lld.exe'
        if (-not (Test-Path -LiteralPath $rustLld -PathType Leaf)) {
            throw "The cargo-xwin fallback requires lld-link.exe or rust-lld.exe: $rustLld"
        }

        # cargo-xwin invokes the LLVM linker by its lld-link multicall name.
        # A hard link avoids copying the large rust-lld binary and requires no
        # administrator or Developer Mode privileges.
        $toolDirectory = Join-Path $projectRoot 'apps\launcher\src-tauri\target\xwin-tools'
        New-Item -ItemType Directory -Path $toolDirectory -Force | Out-Null
        $linkerShim = Join-Path $toolDirectory 'lld-link.exe'
        if (-not (Test-Path -LiteralPath $linkerShim -PathType Leaf)) {
            New-Item -ItemType HardLink -Path $linkerShim -Target $rustLld | Out-Null
        }
        $env:Path = $toolDirectory + [IO.Path]::PathSeparator + $env:Path
    }
}

# Updater artifacts are produced only when the signing key is available.
# Enabling them unconditionally would make every developer and CI build fail
# with "a public key has been found, but no private key", and the alternative —
# shipping the key so builds pass — is exactly what must never happen.
$updaterArguments = @()
if ([string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY) -and
    [string]::IsNullOrWhiteSpace($env:TAURI_SIGNING_PRIVATE_KEY_PATH)) {
    Write-Host 'No updater signing key in the environment; building without updater artifacts.'
}
else {
    $updaterConfig = Join-Path $launcherDir 'src-tauri\tauri.updater.conf.json'
    if (-not (Test-Path -LiteralPath $updaterConfig -PathType Leaf)) {
        throw "Updater config overlay is missing: $updaterConfig"
    }
    Write-Host 'Updater signing key detected; building signed updater artifacts.'
    $updaterArguments = @('--config', $updaterConfig)
}

Push-Location $projectRoot
try {
    $useNativeMsvc = -not $ForceXwin -and (Test-NativeMsvcEnvironment)
    if ($useNativeMsvc) {
        Write-Host "Building the launcher natively for $targetTriple."
        Invoke-Checked 'pnpm.cmd' '--dir' $launcherDir 'tauri' 'build' `
            '--target' $targetTriple '--bundles' 'nsis' @updaterArguments
    }
    else {
        Assert-XwinEnvironment
        $previousCrossCompiler = $env:XWIN_CROSS_COMPILER
        try {
            $env:XWIN_CROSS_COMPILER = 'clang'
            Write-Host "Building the launcher for $targetTriple with cargo-xwin and clang."
            Invoke-Checked 'pnpm.cmd' '--dir' $launcherDir 'tauri' 'build' `
                '--runner' 'cargo-xwin' '--target' $targetTriple '--bundles' 'nsis' @updaterArguments
        }
        finally {
            $env:XWIN_CROSS_COMPILER = $previousCrossCompiler
        }
    }
}
finally {
    Pop-Location
}
