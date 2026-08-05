[CmdletBinding()]
param(
    [string]$JavaHome = $env:PRIVATE_CLIENT_JAVA_HOME,
    [switch]$SkipClean
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$gradleUserHome = if ([string]::IsNullOrWhiteSpace($env:GRADLE_USER_HOME)) {
    Join-Path $env:USERPROFILE '.gradle'
} else {
    $env:GRADLE_USER_HOME
}

function Find-Java8Home {
    param([string]$RequestedHome)

    $candidates = @()
    if (-not [string]::IsNullOrWhiteSpace($RequestedHome)) {
        $candidates += $RequestedHome
    }
    if (-not [string]::IsNullOrWhiteSpace($env:JAVA_HOME)) {
        $candidates += $env:JAVA_HOME
    }
    $candidates += 'C:\Program Files\Eclipse Adoptium\jdk-8.0.492.9-hotspot'
    $candidates += 'C:\Program Files\Java\jdk8'

    foreach ($candidate in ($candidates | Select-Object -Unique)) {
        $java = Join-Path $candidate 'bin\java.exe'
        $javac = Join-Path $candidate 'bin\javac.exe'
        if (-not (Test-Path -LiteralPath $java) -or -not (Test-Path -LiteralPath $javac)) {
            continue
        }
        $previousErrorAction = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $versionOutput = (& $java -version 2>&1 | Out-String)
        $ErrorActionPreference = $previousErrorAction
        if ($versionOutput -match 'version "1\.8\.0_') {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    throw 'A 64-bit Java 8 JDK is required. Set PRIVATE_CLIENT_JAVA_HOME to its directory.'
}

function Assert-ArtifactHash {
    param(
        [string[]]$SearchRoots,
        [string]$FileName,
        [string]$ExpectedSha256
    )

    $matches = @()
    foreach ($root in $SearchRoots) {
        if ([string]::IsNullOrWhiteSpace($root) -or -not (Test-Path -LiteralPath $root)) {
            continue
        }
        $matches += @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter $FileName -ErrorAction SilentlyContinue)
    }
    if ($matches.Count -eq 0) {
        throw "Verified build dependency was not found after Gradle completed: $FileName"
    }
    foreach ($match in $matches) {
        $actual = (Get-FileHash -LiteralPath $match.FullName -Algorithm SHA256).Hash
        if ($actual -eq $ExpectedSha256) {
            return
        }
    }
    throw "SHA-256 verification failed for build dependency: $FileName"
}

function Get-GradleModuleCacheRoots {
    $monorepoRoot = (Resolve-Path -LiteralPath (Join-Path $projectRoot '..\..')).Path
    $gradleHomes = @(
        $gradleUserHome,
        (Join-Path $env:USERPROFILE '.gradle'),
        (Join-Path $monorepoRoot '.gradle-local')
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique

    $roots = @()
    foreach ($gradleHome in $gradleHomes) {
        $cache = Join-Path $gradleHome 'caches\modules-2\files-2.1'
        if ((Test-Path -LiteralPath $cache) -and ($roots -notcontains $cache)) {
            $roots += $cache
        }
    }
    return $roots
}

$wrapperJar = Join-Path $projectRoot 'gradle\wrapper\gradle-wrapper.jar'
$expectedWrapperSha256 = '498495120A03B9A6AB5D155F5DE3C8F0D986A449153702FB80FC80E134484F17'
if (-not (Test-Path -LiteralPath $wrapperJar)) {
    throw 'The Gradle wrapper bootstrap JAR is missing.'
}
if ((Get-FileHash -LiteralPath $wrapperJar -Algorithm SHA256).Hash -ne $expectedWrapperSha256) {
    throw 'The Gradle wrapper bootstrap JAR failed SHA-256 verification.'
}

$resolvedJavaHome = Find-Java8Home -RequestedHome $JavaHome
$env:JAVA_HOME = $resolvedJavaHome
$env:Path = (Join-Path $resolvedJavaHome 'bin') + [IO.Path]::PathSeparator + $env:Path

$gradleArgs = @('--no-daemon')
if (-not $SkipClean) {
    $gradleArgs += 'clean'
}
$gradleArgs += @('test', 'build')

Push-Location $projectRoot
try {
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & (Join-Path $projectRoot 'gradlew.bat') @gradleArgs
    $gradleExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorAction
    if ($gradleExitCode -ne 0) {
        throw "Gradle failed with exit code $gradleExitCode"
    }

    $moduleCaches = Get-GradleModuleCacheRoots
    if ($moduleCaches.Count -eq 0) {
        throw 'No Gradle module caches were found to verify pinned build dependencies.'
    }
    Assert-ArtifactHash `
        -SearchRoots ($moduleCaches | ForEach-Object { Join-Path $_ 'net.minecraftforge.gradle\ForgeGradle' }) `
        -FileName 'ForgeGradle-2.1-20211118.174922-42.jar' `
        -ExpectedSha256 '29F4F9A4B7AD917937D6CA761404ED4C56EE2A716CBFDD190B9AA99F25EB4695'
    Assert-ArtifactHash `
        -SearchRoots ($moduleCaches | ForEach-Object { Join-Path $_ 'net.minecraftforge\forge' }) `
        -FileName 'forge-1.8.9-11.15.1.2318-1.8.9-userdev.jar' `
        -ExpectedSha256 '62BE30583A9A3FB1BE844D5B82E9F105FCE8E2EFB8B68CE0BDF7F742025B5AF2'
    Assert-ArtifactHash `
        -SearchRoots ($moduleCaches | ForEach-Object { Join-Path $_ 'de.oceanlabs.mcp\mcp_stable' }) `
        -FileName 'mcp_stable-22-1.8.9.zip' `
        -ExpectedSha256 'AEED0AABA9D159B7CE60A21E2DCC36ADB249FADE65CE2F76C730DD0EC7270763'

    $moduleJars = @(
        'private-client-core-1.0.0.jar'
    )
    foreach ($moduleName in $moduleJars) {
        $jar = Join-Path $projectRoot "build\libs\$moduleName"
        $checksum = $jar + '.sha512'
        if (-not (Test-Path -LiteralPath $jar) -or -not (Test-Path -LiteralPath $checksum)) {
            throw "The module JAR or its SHA-512 sidecar is missing: $moduleName"
        }
        Write-Host "Module JAR: $jar"
        Write-Host "SHA-512:    $((Get-FileHash -LiteralPath $jar -Algorithm SHA512).Hash)"
    }
    Write-Host "Tests:      $projectRoot\build\reports\tests\index.html"
} finally {
    Pop-Location
}
