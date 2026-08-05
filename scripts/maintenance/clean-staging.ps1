[CmdletBinding(SupportsShouldProcess = $true)]
param()

$ErrorActionPreference = 'Stop'
$localData = [Environment]::GetFolderPath([Environment+SpecialFolder]::LocalApplicationData)
$dataRoot = [System.IO.Path]::GetFullPath((Join-Path $localData 'Private Client'))
$staging = [System.IO.Path]::GetFullPath((Join-Path $dataRoot 'staging'))

if (-not $staging.StartsWith($dataRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Unsafe staging path: $staging"
}

if (Test-Path -LiteralPath $staging -PathType Container) {
    Get-ChildItem -LiteralPath $staging -Directory |
        Where-Object { $_.LastWriteTimeUtc -lt [DateTime]::UtcNow.AddDays(-7) } |
        ForEach-Object {
            $resolved = [System.IO.Path]::GetFullPath($_.FullName)
            if ($resolved.StartsWith($staging + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase) -and
                $PSCmdlet.ShouldProcess($resolved, 'Remove stale staging directory')) {
                Remove-Item -LiteralPath $resolved -Recurse -Force
            }
        }
}
