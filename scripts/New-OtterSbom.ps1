[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][string]$Version
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

function Write-Utf8NoBom {
    param([string]$Path, [string]$Value)
    [System.IO.File]::WriteAllText($Path, $Value, [System.Text.UTF8Encoding]::new($false))
}
function Add-Component {
    param([System.Collections.Specialized.OrderedDictionary]$Map, [string]$Purl, [string]$Name, [string]$PackageVersion)
    # 使用经源码完整性校验的身份，不能重建并丢弃本地补丁 checksum qualifier。
    if (-not $Map.Contains($purl)) {
        $Map[$purl] = [ordered]@{ type = "library"; name = $Name; version = $PackageVersion; purl = $purl; "bom-ref" = $purl }
    }
}

$componentsByRef = [ordered]@{}
$nodeOutput = & node (Join-Path $PSScriptRoot "release\sbom-inventory.cjs") inventory $repoRoot
if ($LASTEXITCODE -ne 0) { throw "Failed to read authoritative lockfile inventory." }
$inventory = ($nodeOutput -join "`n") | ConvertFrom-Json
foreach ($entry in $inventory) {
    Add-Component -Map $componentsByRef -Purl ([string]$entry.purl) -Name ([string]$entry.name) -PackageVersion ([string]$entry.version)
}

$components = @($componentsByRef.Values | Sort-Object { [string]$_['bom-ref'] })
$timestamp = [DateTimeOffset]::UtcNow.ToString("o")
$productRef = "pkg:generic/hook@$([Uri]::EscapeDataString($Version))"
$cycloneDx = [ordered]@{
    bomFormat = "CycloneDX"; specVersion = "1.6"; serialNumber = "urn:uuid:$([Guid]::NewGuid())"; version = 1
    metadata = [ordered]@{
        timestamp = $timestamp
        tools = @([ordered]@{ vendor = "Neuro"; name = "New-OtterSbom.ps1"; version = "1" })
        component = [ordered]@{ type = "application"; name = "Otter"; version = $Version; "bom-ref" = $productRef }
    }
    components = $components
}

$spdxPackages = @()
$index = 0
foreach ($component in $components) {
    $index++
    $spdxPackages += [ordered]@{
        SPDXID = "SPDXRef-Package-$index"; name = $component.name; versionInfo = $component.version
        downloadLocation = "NOASSERTION"; filesAnalyzed = $false; licenseConcluded = "NOASSERTION"
        licenseDeclared = "NOASSERTION"; copyrightText = "NOASSERTION"
        externalRefs = @([ordered]@{ referenceCategory = "PACKAGE-MANAGER"; referenceType = "purl"; referenceLocator = $component.purl })
    }
}
$spdx = [ordered]@{
    spdxVersion = "SPDX-2.3"; dataLicense = "CC0-1.0"; SPDXID = "SPDXRef-DOCUMENT"; name = "Otter-$Version"
    documentNamespace = "https://github.com/aiaimimi0920/Otter/sbom/$([Guid]::NewGuid())"
    creationInfo = [ordered]@{ created = $timestamp; creators = @("Tool: New-OtterSbom.ps1-1") }
    packages = $spdxPackages
}

$cyclonePath = Join-Path $outputRoot "Otter-$Version.cdx.json"
$spdxPath = Join-Path $outputRoot "Otter-$Version.spdx.json"
Write-Utf8NoBom -Path $cyclonePath -Value (($cycloneDx | ConvertTo-Json -Depth 20) + "`n")
Write-Utf8NoBom -Path $spdxPath -Value (($spdx | ConvertTo-Json -Depth 20) + "`n")
[ordered]@{ schemaVersion = 1; componentCount = $components.Count; cycloneDx = $cyclonePath; spdx = $spdxPath } | ConvertTo-Json
