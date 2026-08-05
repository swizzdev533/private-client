[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$targetTriple = 'x86_64-pc-windows-msvc'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$tauriDirectory = Join-Path $projectRoot 'apps\launcher\src-tauri'
$manifest = Join-Path $tauriDirectory 'Cargo.toml'

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
    $hostLine = & 'rustc.exe' '-vV' |
        Where-Object { $_ -like 'host: *' } |
        Select-Object -First 1
    if ($LASTEXITCODE -ne 0 -or -not $hostLine) {
        throw 'Unable to determine the active Rust host triple.'
    }
    return ($hostLine -replace '^host:\s*', '').Trim()
}

function Add-LldLinkShim {
    if (Get-Command 'lld-link.exe' -CommandType Application -ErrorAction SilentlyContinue) {
        return
    }

    $targetLibOutput = & 'rustc.exe' '--print' 'target-libdir'
    $rustcExitCode = $LASTEXITCODE
    # Trim only after the exit-code check: a failed rustc emits nothing, and
    # calling .Trim() on $null throws before the intended message is reached.
    $targetLibDir = if ($rustcExitCode -eq 0) { ($targetLibOutput | Select-Object -First 1) } else { $null }
    if (-not $targetLibDir) {
        throw 'Unable to locate the Rust LLVM linker.'
    }
    $targetLibDir = $targetLibDir.Trim()
    $rustLld = Join-Path (Split-Path -Parent $targetLibDir) 'bin\rust-lld.exe'
    if (-not (Test-Path -LiteralPath $rustLld -PathType Leaf)) {
        throw "The MSVC cross-test requires lld-link.exe or rust-lld.exe: $rustLld"
    }

    $toolDirectory = Join-Path $tauriDirectory 'target\xwin-tools'
    New-Item -ItemType Directory -Path $toolDirectory -Force | Out-Null
    $linkerShim = Join-Path $toolDirectory 'lld-link.exe'
    if (-not (Test-Path -LiteralPath $linkerShim -PathType Leaf)) {
        New-Item -ItemType HardLink -Path $linkerShim -Target $rustLld | Out-Null
    }
    $env:Path = $toolDirectory + [IO.Path]::PathSeparator + $env:Path
}

if ((Get-RustHostTriple) -eq $targetTriple) {
    Invoke-Checked 'cargo.exe' 'test' '--manifest-path' $manifest '--all-targets'
    exit 0
}

foreach ($command in @('cargo-xwin.exe', 'clang.exe', 'llvm-lib.exe', 'llvm-rc.exe')) {
    if (-not (Get-Command $command -CommandType Application -ErrorAction SilentlyContinue)) {
        throw "MSVC Rust tests from a non-MSVC host require $command on PATH."
    }
}
$installedTargets = & 'rustup.exe' 'target' 'list' '--installed'
if ($LASTEXITCODE -ne 0 -or $installedTargets -notcontains $targetTriple) {
    throw "MSVC Rust tests require the installed Rust target $targetTriple."
}

$previousCompiler = $env:XWIN_CROSS_COMPILER
$previousRunner = $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER
$previousPath = $env:Path
try {
    Add-LldLinkShim
    $env:XWIN_CROSS_COMPILER = 'clang'
    # cargo-xwin defaults to Wine for cross-host tests. On Windows the target
    # executable can run directly through cmd while preserving quoted paths.
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER = 'cmd.exe /d /c'
    Push-Location $tauriDirectory
    try {
        Invoke-Checked 'cargo.exe' 'xwin' 'test' '--target' $targetTriple '--all-targets'
    }
    finally {
        Pop-Location
    }
}
finally {
    $env:XWIN_CROSS_COMPILER = $previousCompiler
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUNNER = $previousRunner
    $env:Path = $previousPath
}
