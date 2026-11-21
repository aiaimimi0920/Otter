[CmdletBinding()]
param(
    [string]$Tag,
    [switch]$RequireTagAtHead,
    [string]$RequireReachableFromBranch
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
. (Join-Path $PSScriptRoot "version-identity.ps1")
$null = Get-OtterVersionIdentity -RepoRoot $repoRoot
$packageJson = Get-Content -LiteralPath (Join-Path $repoRoot "package.json") -Raw | ConvertFrom-Json
$cargoToml = Get-Content -LiteralPath (Join-Path $repoRoot "native/Cargo.toml") -Raw
$cargoLock = Get-Content -LiteralPath (Join-Path $repoRoot "native/Cargo.lock") -Raw

$cargoTomlMatch = [regex]::Match($cargoToml, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $cargoTomlMatch.Success) {
    throw "Could not read the Otter package version from native/Cargo.toml."
}

$cargoLockMatch = [regex]::Match($cargoLock, '(?ms)\[\[package\]\]\s+name\s*=\s*"otter-native"\s+version\s*=\s*"([^"]+)"')
if (-not $cargoLockMatch.Success) {
    throw "Could not read the Otter package version from native/Cargo.lock."
}

$versions = [ordered]@{
    "package.json" = [string]$packageJson.version
    "native/Cargo.toml" = $cargoTomlMatch.Groups[1].Value
    "native/Cargo.lock" = $cargoLockMatch.Groups[1].Value
}

$metadataVersion = [string]$packageJson.version
if ($metadataVersion -notmatch '^\d+\.\d+\.\d+$') {
    throw "Otter product version must use strict SemVer X.Y.Z. Received: $metadataVersion"
}

$expectedVersion = $metadataVersion
if (-not [string]::IsNullOrWhiteSpace($Tag)) {
    if ($Tag -notmatch '^V(\d+\.\d+\.\d+)$') {
        throw "Release tag must match Vx.x.x. Received: $Tag"
    }
    $expectedVersion = $Matches[1]
}

$mismatches = @($versions.GetEnumerator() | Where-Object { $_.Value -ne $expectedVersion })
if ($mismatches.Count -gt 0) {
    $details = $mismatches | ForEach-Object { "$($_.Key)=$($_.Value)" }
    throw "Release version $expectedVersion does not match all product metadata: $($details -join ', ')"
}

if (($RequireTagAtHead -or -not [string]::IsNullOrWhiteSpace($RequireReachableFromBranch)) -and [string]::IsNullOrWhiteSpace($Tag)) {
    throw "A release tag is required for Git provenance validation."
}

if (-not [string]::IsNullOrWhiteSpace($Tag)) {
    Push-Location -LiteralPath $repoRoot
    try {
        $tagCommit = (& git rev-parse "$Tag^{commit}").Trim()
        if ($LASTEXITCODE -ne 0) {
            throw "Could not resolve release tag $Tag."
        }

        if ($RequireTagAtHead) {
            $headCommit = (& git rev-parse HEAD).Trim()
            if ($LASTEXITCODE -ne 0) {
                throw "Could not resolve HEAD."
            }

            if ($tagCommit -ne $headCommit) {
                throw "Release tag $Tag resolves to $tagCommit but the checked-out HEAD is $headCommit."
            }
        }

        if (-not [string]::IsNullOrWhiteSpace($RequireReachableFromBranch)) {
            $branchCommit = (& git rev-parse "$RequireReachableFromBranch^{commit}").Trim()
            if ($LASTEXITCODE -ne 0) {
                throw "Could not resolve protected release branch $RequireReachableFromBranch."
            }

            & git merge-base --is-ancestor $tagCommit $branchCommit
            $ancestorExitCode = $LASTEXITCODE
            if ($ancestorExitCode -eq 1) {
                throw "Release tag $Tag ($tagCommit) is not reachable from protected release branch $RequireReachableFromBranch ($branchCommit)."
            }
            if ($ancestorExitCode -ne 0) {
                throw "Could not verify release tag ancestry for $Tag against $RequireReachableFromBranch."
            }
        }
    }
    finally {
        Pop-Location
    }
}

$scope = if ([string]::IsNullOrWhiteSpace($Tag)) { "metadata" } else { $Tag }
Write-Host "[otter-release-version] $scope matches all product version fields ($expectedVersion)."
