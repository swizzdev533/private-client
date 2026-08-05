[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$reportRoot = Join-Path $projectRoot 'reports\licenses'
New-Item -ItemType Directory -Path $reportRoot -Force | Out-Null

Push-Location $projectRoot
try {
    & 'pnpm.cmd' '--dir' (Join-Path $projectRoot 'apps\launcher') 'licenses' 'list' '--json' |
        Set-Content -LiteralPath (Join-Path $reportRoot 'node-licenses.json') -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw "pnpm license report failed with exit code $LASTEXITCODE"
    }

    & 'cargo.exe' 'metadata' '--manifest-path' (Join-Path $projectRoot 'apps\launcher\src-tauri\Cargo.toml') '--format-version' '1' |
        Set-Content -LiteralPath (Join-Path $reportRoot 'cargo-metadata.json') -Encoding utf8
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed with exit code $LASTEXITCODE"
    }

    $gradle = Join-Path $projectRoot 'minecraft\private-client-core\gradlew.bat'
    if (Test-Path -LiteralPath $gradle -PathType Leaf) {
        & $gradle '--no-daemon' 'dependencies' |
            Set-Content -LiteralPath (Join-Path $reportRoot 'core-runtime-dependencies.txt') -Encoding utf8
        if ($LASTEXITCODE -ne 0) {
            throw "Core dependency report failed with exit code $LASTEXITCODE"
        }
    }
}
finally {
    Pop-Location
}

Write-Host "License reports written to $reportRoot"
