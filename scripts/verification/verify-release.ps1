[CmdletBinding()]
param(
    [string]$ReleaseDirectory
)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
if ($ReleaseDirectory) {
    $releaseDir = [System.IO.Path]::GetFullPath($ReleaseDirectory)
}
else {
    $releaseDir = Join-Path $projectRoot 'artifacts\release'
}
$manifestPath = Join-Path $releaseDir 'release-manifest.json'

function Assert-FileRange {
    param(
        [Parameter(Mandatory = $true)][long]$FileLength,
        [Parameter(Mandatory = $true)][long]$Offset,
        [Parameter(Mandatory = $true)][long]$Length,
        [Parameter(Mandatory = $true)][string]$Context
    )

    if ($Offset -lt 0 -or $Length -lt 0 -or $Offset -gt $FileLength -or
        $Length -gt ($FileLength - $Offset)) {
        throw "Invalid PE range for $Context."
    }
}

function Read-PeUInt16 {
    param(
        [Parameter(Mandatory = $true)][System.IO.BinaryReader]$Reader,
        [Parameter(Mandatory = $true)][long]$FileLength,
        [Parameter(Mandatory = $true)][long]$Offset,
        [Parameter(Mandatory = $true)][string]$Context
    )

    Assert-FileRange -FileLength $FileLength -Offset $Offset -Length 2 -Context $Context
    $Reader.BaseStream.Position = $Offset
    return $Reader.ReadUInt16()
}

function Read-PeUInt32 {
    param(
        [Parameter(Mandatory = $true)][System.IO.BinaryReader]$Reader,
        [Parameter(Mandatory = $true)][long]$FileLength,
        [Parameter(Mandatory = $true)][long]$Offset,
        [Parameter(Mandatory = $true)][string]$Context
    )

    Assert-FileRange -FileLength $FileLength -Offset $Offset -Length 4 -Context $Context
    $Reader.BaseStream.Position = $Offset
    return $Reader.ReadUInt32()
}

function Convert-PeRvaToFileOffset {
    param(
        [Parameter(Mandatory = $true)][uint32]$Rva,
        [Parameter(Mandatory = $true)][object[]]$Sections,
        [Parameter(Mandatory = $true)][uint32]$SizeOfHeaders,
        [Parameter(Mandatory = $true)][long]$FileLength,
        [Parameter(Mandatory = $true)][string]$Context
    )

    if ([uint64]$Rva -lt [uint64]$SizeOfHeaders) {
        Assert-FileRange -FileLength $FileLength -Offset ([long]$Rva) -Length 1 -Context $Context
        return [long]$Rva
    }

    foreach ($section in $Sections) {
        $extent = [uint64]$section.VirtualSize
        if ([uint64]$section.SizeOfRawData -gt $extent) {
            $extent = [uint64]$section.SizeOfRawData
        }

        $start = [uint64]$section.VirtualAddress
        $end = $start + $extent
        if ([uint64]$Rva -lt $start -or [uint64]$Rva -ge $end) {
            continue
        }

        $delta = [uint64]$Rva - $start
        if ($delta -ge [uint64]$section.SizeOfRawData) {
            throw "PE RVA for $Context points into virtual-only section data."
        }

        $fileOffset = [uint64]$section.PointerToRawData + $delta
        if ($fileOffset -gt [long]::MaxValue) {
            throw "PE RVA for $Context exceeds the supported file size."
        }
        Assert-FileRange -FileLength $FileLength -Offset ([long]$fileOffset) -Length 1 -Context $Context
        return [long]$fileOffset
    }

    throw ('PE RVA 0x{0:X8} for {1} does not map to a file section.' -f $Rva, $Context)
}

function Read-PeAsciiZ {
    param(
        [Parameter(Mandatory = $true)][System.IO.BinaryReader]$Reader,
        [Parameter(Mandatory = $true)][long]$FileLength,
        [Parameter(Mandatory = $true)][long]$Offset,
        [Parameter(Mandatory = $true)][int]$MaximumLength,
        [Parameter(Mandatory = $true)][string]$Context
    )

    Assert-FileRange -FileLength $FileLength -Offset $Offset -Length 1 -Context $Context
    $available = [int][Math]::Min([long]$MaximumLength, $FileLength - $Offset)
    $Reader.BaseStream.Position = $Offset
    $buffer = $Reader.ReadBytes($available)
    for ($length = 0; $length -lt $buffer.Length; $length++) {
        if ($buffer[$length] -eq 0) {
            return [System.Text.Encoding]::ASCII.GetString($buffer, 0, $length)
        }
    }

    throw "Unterminated or overlong ASCII string in $Context."
}

function Get-PeMetadata {
    param(
        [Parameter(Mandatory = $true)][string]$Path
    )

    $stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::Read
    )
    $reader = New-Object System.IO.BinaryReader($stream)
    try {
        $fileLength = $stream.Length
        Assert-FileRange -FileLength $fileLength -Offset 0 -Length 64 -Context 'DOS header'
        if ((Read-PeUInt16 -Reader $reader -FileLength $fileLength -Offset 0 -Context 'DOS signature') -ne 0x5a4d) {
            throw "$Path is not an MZ executable."
        }

        $peOffset = [long](Read-PeUInt32 -Reader $reader -FileLength $fileLength `
            -Offset 0x3c -Context 'PE header offset')
        Assert-FileRange -FileLength $fileLength -Offset $peOffset -Length 24 -Context 'PE and COFF headers'
        if ((Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                -Offset $peOffset -Context 'PE signature') -ne 0x00004550) {
            throw "$Path has an invalid PE signature."
        }

        $machine = Read-PeUInt16 -Reader $reader -FileLength $fileLength `
            -Offset ($peOffset + 4) -Context 'COFF machine'
        $sectionCount = Read-PeUInt16 -Reader $reader -FileLength $fileLength `
            -Offset ($peOffset + 6) -Context 'COFF section count'
        $optionalHeaderSize = Read-PeUInt16 -Reader $reader -FileLength $fileLength `
            -Offset ($peOffset + 20) -Context 'optional-header size'
        if ($sectionCount -lt 1 -or $sectionCount -gt 96) {
            throw "$Path has an unreasonable PE section count."
        }

        $optionalOffset = $peOffset + 24
        Assert-FileRange -FileLength $fileLength -Offset $optionalOffset `
            -Length $optionalHeaderSize -Context 'optional header'
        $optionalMagic = Read-PeUInt16 -Reader $reader -FileLength $fileLength `
            -Offset $optionalOffset -Context 'optional-header magic'
        if ($optionalMagic -eq 0x20b) {
            $directoryCountOffset = 108
            $directoryTableOffset = 112
        }
        elseif ($optionalMagic -eq 0x10b) {
            $directoryCountOffset = 92
            $directoryTableOffset = 96
        }
        else {
            throw ('{0} has unsupported optional-header magic 0x{1:X4}.' -f $Path, $optionalMagic)
        }

        if ($optionalHeaderSize -lt ($directoryCountOffset + 4)) {
            throw "$Path has a truncated optional header."
        }
        $sizeOfHeaders = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
            -Offset ($optionalOffset + 60) -Context 'SizeOfHeaders'
        $directoryCount = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
            -Offset ($optionalOffset + $directoryCountOffset) -Context 'data-directory count'

        $sectionTableOffset = $optionalOffset + $optionalHeaderSize
        Assert-FileRange -FileLength $fileLength -Offset $sectionTableOffset `
            -Length ([long]$sectionCount * 40) -Context 'section table'
        $sections = @()
        for ($index = 0; $index -lt $sectionCount; $index++) {
            $entry = $sectionTableOffset + ([long]$index * 40)
            $sections += [pscustomobject]@{
                VirtualSize = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                    -Offset ($entry + 8) -Context 'section virtual size'
                VirtualAddress = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                    -Offset ($entry + 12) -Context 'section RVA'
                SizeOfRawData = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                    -Offset ($entry + 16) -Context 'section raw size'
                PointerToRawData = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                    -Offset ($entry + 20) -Context 'section raw pointer'
            }
        }

        $imports = @()
        if ($directoryCount -gt 1 -and $optionalHeaderSize -ge ($directoryTableOffset + 16)) {
            $importDirectoryOffset = $optionalOffset + $directoryTableOffset + 8
            $importRva = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                -Offset $importDirectoryOffset -Context 'import-directory RVA'
            $importSize = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                -Offset ($importDirectoryOffset + 4) -Context 'import-directory size'

            if ($importRva -ne 0) {
                $descriptorLimit = 4096
                if ($importSize -ge 20) {
                    $sizeBound = [int][Math]::Ceiling(([double]$importSize) / 20.0)
                    if ($sizeBound -lt $descriptorLimit) {
                        $descriptorLimit = $sizeBound
                    }
                }

                $terminated = $false
                for ($index = 0; $index -lt $descriptorLimit; $index++) {
                    $descriptorRva64 = [uint64]$importRva + ([uint64]$index * 20)
                    if ($descriptorRva64 -gt [uint32]::MaxValue) {
                        throw "$Path has an overflowing import-descriptor RVA."
                    }
                    $descriptorOffset = Convert-PeRvaToFileOffset `
                        -Rva ([uint32]$descriptorRva64) -Sections $sections `
                        -SizeOfHeaders $sizeOfHeaders -FileLength $fileLength `
                        -Context 'import descriptor'
                    Assert-FileRange -FileLength $fileLength -Offset $descriptorOffset `
                        -Length 20 -Context 'import descriptor'

                    $originalFirstThunk = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                        -Offset $descriptorOffset -Context 'OriginalFirstThunk'
                    $timeDateStamp = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                        -Offset ($descriptorOffset + 4) -Context 'import timestamp'
                    $forwarderChain = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                        -Offset ($descriptorOffset + 8) -Context 'forwarder chain'
                    $nameRva = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                        -Offset ($descriptorOffset + 12) -Context 'import name RVA'
                    $firstThunk = Read-PeUInt32 -Reader $reader -FileLength $fileLength `
                        -Offset ($descriptorOffset + 16) -Context 'FirstThunk'

                    if ($originalFirstThunk -eq 0 -and $timeDateStamp -eq 0 -and
                        $forwarderChain -eq 0 -and $nameRva -eq 0 -and $firstThunk -eq 0) {
                        $terminated = $true
                        break
                    }
                    if ($nameRva -eq 0) {
                        throw "$Path has an import descriptor without a DLL name."
                    }

                    $nameOffset = Convert-PeRvaToFileOffset -Rva $nameRva `
                        -Sections $sections -SizeOfHeaders $sizeOfHeaders `
                        -FileLength $fileLength -Context 'import DLL name'
                    $imports += Read-PeAsciiZ -Reader $reader -FileLength $fileLength `
                        -Offset $nameOffset -MaximumLength 512 -Context 'import DLL name'
                }

                if (-not $terminated) {
                    throw "$Path has an unterminated or oversized import descriptor table."
                }
            }
        }

        return [pscustomobject]@{
            Machine = [uint16]$machine
            OptionalMagic = [uint16]$optionalMagic
            Imports = [string[]]$imports
        }
    }
    finally {
        $reader.Dispose()
        $stream.Dispose()
    }
}

if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Release manifest not found: $manifestPath"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
if ($manifest.schemaVersion -ne 1 -or $manifest.version -ne '1.0.0') {
    throw 'Unexpected release manifest schema or version.'
}
if ($manifest.buildTarget -ne 'x86_64-pc-windows-msvc') {
    throw 'Release manifest does not identify the required MSVC x64 build target.'
}
if ($manifest.artifacts.Count -ne 2) {
    throw 'Exactly two Windows release executables are required.'
}

$artifactNames = @($manifest.artifacts | ForEach-Object { [string]$_.name })
if (($artifactNames | Sort-Object -Unique).Count -ne 2 -or
    'PrivateClient.exe' -notin $artifactNames -or 'setup.exe' -notin $artifactNames) {
    throw 'Release artifacts must be exactly PrivateClient.exe and setup.exe.'
}

$peByName = @{}
foreach ($artifact in $manifest.artifacts) {
    if ($artifact.platform -ne 'windows' -or $artifact.arch -ne 'x86_64') {
        throw "Unexpected platform or architecture for $($artifact.name)."
    }

    $path = Join-Path $releaseDir $artifact.name
    $item = Get-Item -LiteralPath $path -ErrorAction Stop
    if ($item.Length -le 1024 -or $item.Length -ne [long]$artifact.size) {
        throw "Invalid artifact size for $($artifact.name)"
    }

    $metadata = Get-PeMetadata -Path $path
    $peByName[$artifact.name] = $metadata

    $actualHash = (Get-FileHash -LiteralPath $path -Algorithm SHA512).Hash.ToLowerInvariant()
    if ($actualHash -cne [string]$artifact.sha512) {
        throw "SHA-512 mismatch for $($artifact.name)"
    }
}

$launcherPe = $peByName['PrivateClient.exe']
if ($launcherPe.Machine -ne 0x8664 -or $launcherPe.OptionalMagic -ne 0x20b) {
    throw 'PrivateClient.exe is not an x86-64 PE32+ executable.'
}
if ($launcherPe.Imports | Where-Object { $_ -ieq 'WebView2Loader.dll' }) {
    throw 'PrivateClient.exe imports WebView2Loader.dll and is not a standalone launcher executable.'
}

$setupPe = $peByName['setup.exe']
if ($setupPe.Machine -notin @(0x014c, 0x8664)) {
    throw ('setup.exe has an unsupported PE machine type 0x{0:X4}.' -f $setupPe.Machine)
}

# Recursive: a DLL smuggled into a subdirectory of the release still breaks the
# standalone guarantee, and a top-level-only scan would miss it.
$runtimeDlls = Get-ChildItem -LiteralPath $releaseDir -File -Filter '*.dll' -Recurse
if ($runtimeDlls) {
    throw "Standalone release unexpectedly contains DLL files: $(($runtimeDlls | ForEach-Object { $_.FullName.Substring($releaseDir.Length).TrimStart('\') }) -join ', ')"
}

$sumPath = Join-Path $releaseDir 'SHA512SUMS.txt'
if (-not (Test-Path -LiteralPath $sumPath -PathType Leaf)) {
    throw 'SHA512SUMS.txt is missing.'
}
$expectedSums = @($manifest.artifacts | ForEach-Object {
    "$([string]$_.sha512) *$([string]$_.name)"
})
$actualSums = @(Get-Content -LiteralPath $sumPath -Encoding ascii)
if (($actualSums -join "`n") -cne ($expectedSums -join "`n")) {
    throw 'SHA512SUMS.txt does not exactly match the release manifest.'
}

Write-Host 'Release verification passed: MSVC x64 launcher, standalone imports, PE files, and SHA-512 hashes are valid.'
