[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$reportRoot = Join-Path $projectRoot 'reports\sbom'
New-Item -ItemType Directory -Path $reportRoot -Force | Out-Null

$componentMap = [ordered]@{}

function Add-Component {
    param(
        [string]$Type,
        [string]$Name,
        [string]$Version,
        [string]$License,
        [string]$Purl
    )

    if ([string]::IsNullOrWhiteSpace($Name) -or [string]::IsNullOrWhiteSpace($Version)) {
        return
    }
    $reference = if ($Purl) { $Purl } else { "${Type}:${Name}@${Version}" }
    if ($componentMap.Contains($reference)) {
        return
    }

    $component = [ordered]@{
        type = $Type
        name = $Name
        version = $Version
        'bom-ref' = $reference
    }
    if ($Purl) {
        $component['purl'] = $Purl
    }
    if (-not [string]::IsNullOrWhiteSpace($License)) {
        $component['licenses'] = @([ordered]@{ license = [ordered]@{ name = $License } })
    }
    $componentMap[$reference] = $component
}

function Add-NodeTree {
    param([object]$Dependencies)

    if ($null -eq $Dependencies) {
        return
    }
    foreach ($property in $Dependencies.PSObject.Properties) {
        $dependency = $property.Value
        $name = [string]$property.Name
        $version = [string]$dependency.version
        Add-Component -Type 'library' -Name $name -Version $version -License '' -Purl "pkg:npm/$([Uri]::EscapeDataString($name))@$version"
        Add-NodeTree -Dependencies $dependency.dependencies
    }
}

$nodeJson = & 'pnpm.cmd' '--dir' (Join-Path $projectRoot 'apps\launcher') 'list' '--json' '--depth' '1000'
if ($LASTEXITCODE -ne 0) {
    throw "pnpm dependency listing failed with exit code $LASTEXITCODE"
}
foreach ($project in ($nodeJson | ConvertFrom-Json)) {
    Add-NodeTree -Dependencies $project.dependencies
    Add-NodeTree -Dependencies $project.devDependencies
}

$cargoJson = & 'cargo.exe' 'metadata' '--manifest-path' (Join-Path $projectRoot 'apps\launcher\src-tauri\Cargo.toml') '--format-version' '1'
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed with exit code $LASTEXITCODE"
}
$cargoPackagesJson = $cargoJson | & 'node.exe' '-e' @'
let input = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => { input += chunk; });
process.stdin.on('end', () => {
  const metadata = JSON.parse(input);
  process.stdout.write(JSON.stringify(metadata.packages.map(({ name, version, license }) => ({
    name,
    version,
    license: license || ''
  }))));
});
'@
if ($LASTEXITCODE -ne 0) {
    throw "Cargo metadata normalization failed with exit code $LASTEXITCODE"
}
$cargoPackages = $cargoPackagesJson | ConvertFrom-Json
foreach ($package in $cargoPackages) {
    Add-Component -Type 'library' -Name ([string]$package.name) -Version ([string]$package.version) -License ([string]$package.license) -Purl "pkg:cargo/$($package.name)@$($package.version)"
}

Add-Component -Type 'library' -Name 'Minecraft Forge' -Version '11.15.1.2318' -License 'LGPL-2.1' -Purl 'pkg:maven/net.minecraftforge/forge@1.8.9-11.15.1.2318-1.8.9'
Add-Component -Type 'library' -Name 'JUnit' -Version '4.13.2' -License 'EPL-1.0' -Purl 'pkg:maven/junit/junit@4.13.2'

$bom = [ordered]@{
    bomFormat = 'CycloneDX'
    specVersion = '1.5'
    serialNumber = "urn:uuid:$([Guid]::NewGuid())"
    version = 1
    metadata = [ordered]@{
        timestamp = [DateTime]::UtcNow.ToString('o')
        component = [ordered]@{
            type = 'application'
            name = 'Private Client'
            version = '1.0.0'
            'bom-ref' = 'pkg:generic/private-client@1.0.0'
        }
    }
    components = @($componentMap.Values)
}

$bom | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $reportRoot 'private-client.cdx.json') -Encoding utf8
Write-Host "CycloneDX SBOM written to $reportRoot"
