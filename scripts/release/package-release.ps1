[CmdletBinding()]
param(
    [ValidateSet('x86_64-pc-windows-msvc')]
    [string]$TargetTriple = 'x86_64-pc-windows-msvc'
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$artifactRoot = Join-Path $projectRoot 'artifacts'
$releaseDir = Join-Path $artifactRoot 'release'

function Resolve-ReleaseTargetDir {
    param([string]$Triple)

    $candidates = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
        $candidates.Add((Join-Path $env:CARGO_TARGET_DIR "$Triple\release"))
    }
    $candidates.Add((Join-Path $projectRoot "apps\launcher\src-tauri\target\$Triple\release"))

    $best = $null
    foreach ($candidate in $candidates) {
        $exe = Join-Path $candidate 'private-client.exe'
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
            continue
        }
        $item = Get-Item -LiteralPath $exe
        if ($null -eq $best -or $item.LastWriteTimeUtc -gt $best.LastWriteTimeUtc) {
            $best = $item
        }
    }

    if ($null -eq $best) {
        throw @"
Could not find private-client.exe for $Triple.
Checked CARGO_TARGET_DIR and apps\launcher\src-tauri\target.
"@
    }

    return $best.Directory.FullName
}

$targetDir = Resolve-ReleaseTargetDir -Triple $TargetTriple
Write-Host "Packaging launcher from: $targetDir"

$rawPath = Join-Path $targetDir 'private-client.exe'
$rawApp = Get-Item -LiteralPath $rawPath -ErrorAction SilentlyContinue

$setupApp = Get-ChildItem -LiteralPath (Join-Path $targetDir 'bundle\nsis') -File -Filter '*-setup.exe' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1

if (-not $rawApp) {
    throw "The exact MSVC launcher executable was not found: $rawPath"
}
if (-not $setupApp) {
    throw "The target-specific NSIS setup executable was not found below: $targetDir"
}

$resolvedArtifactRoot = [System.IO.Path]::GetFullPath($artifactRoot)
$resolvedRelease = [System.IO.Path]::GetFullPath($releaseDir)
if (-not $resolvedRelease.StartsWith($resolvedArtifactRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Unsafe release path: $resolvedRelease"
}

if (Test-Path -LiteralPath $releaseDir) {
    Get-ChildItem -LiteralPath $releaseDir -Recurse -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $releaseDir -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null

$rawDestination = Join-Path $releaseDir 'PrivateClient.exe'
$setupDestination = Join-Path $releaseDir 'setup.exe'
Copy-Item -LiteralPath $rawApp.FullName -Destination $rawDestination
Copy-Item -LiteralPath $setupApp.FullName -Destination $setupDestination

$artifacts = @()
foreach ($file in @($rawDestination, $setupDestination)) {
    $item = Get-Item -LiteralPath $file
    $hash = (Get-FileHash -LiteralPath $file -Algorithm SHA512).Hash.ToLowerInvariant()
    $artifacts += [ordered]@{
        name = $item.Name
        platform = 'windows'
        arch = 'x86_64'
        size = $item.Length
        sha512 = $hash
        signed = $false
    }
}

$manifest = [ordered]@{
    schemaVersion = 1
    version = '1.0.0'
    channel = 'local-unsigned'
    buildTarget = $TargetTriple
    packagedFrom = $targetDir
    publishedAt = [DateTime]::UtcNow.ToString('o')
    artifacts = $artifacts
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $releaseDir 'release-manifest.json') -Encoding utf8

$sumLines = $artifacts | ForEach-Object { "$($_.sha512) *$($_.name)" }
$sumLines | Set-Content -LiteralPath (Join-Path $releaseDir 'SHA512SUMS.txt') -Encoding ascii

$coreJar = Get-ChildItem -LiteralPath (Join-Path $projectRoot 'minecraft\private-client-core\build\libs') -File -Filter 'private-client-core-*.jar' -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -notlike '*sources*' } |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
if ($coreJar) {
    $coreHash = (Get-FileHash -LiteralPath $coreJar.FullName -Algorithm SHA512).Hash.ToLowerInvariant()
    [ordered]@{
        schemaVersion = 1
        fileName = $coreJar.Name
        size = $coreJar.Length
        sha512 = $coreHash
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $releaseDir 'core-artifact.json') -Encoding utf8
}

Write-Host "Release artifacts written to $releaseDir"
