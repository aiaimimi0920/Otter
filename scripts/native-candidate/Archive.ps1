# 内部 ZIP 的内容一致性门；不执行归档内容，不把无签名清单当成可信发布证明。
function New-NativeCandidateArchive {
    param([string]$Directory, [string]$ZipPath)
    if (Test-Path -LiteralPath $ZipPath) { throw 'Native ZIP already exists' }
    Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
    $root = [IO.Path]::GetFullPath($Directory).TrimEnd('\')
    $archive = [IO.Compression.ZipFile]::Open($ZipPath, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in @(Get-OtterSafeReleaseFiles -RootPath $root)) {
            $name = $file.FullName.Substring($root.Length + 1).Replace('\', '/')
            [void](Assert-OtterSafeRelativePath -RelativePath $name)
            [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $file.FullName, $name, [IO.Compression.CompressionLevel]::Optimal)
        }
    } finally { $archive.Dispose() }
}
function Test-NativeCandidateArchive {
    param([Parameter(Mandatory = $true)][string]$ZipPath)
    $entries = @(Get-OtterArchiveEntries -ZipPath $ZipPath -MaxEntries 4096 -MaxUncompressedBytes 128MB)
    # 原生产品包不允许重新夹带已退役的 ANGLE DLL，清单有无登记均拒绝。
    foreach ($name in $entries) {
        if ([IO.Path]::GetFileName($name) -imatch '^lib(EGL|GLESv2)\.dll$') {
            throw 'Retired ANGLE DLL in native package'
        }
    }
    $manifest = (Read-OtterArchiveEntryText -ZipPath $ZipPath -EntryName 'package-manifest.json' -MaxBytes 4MB) | ConvertFrom-Json
    if ($manifest.schemaVersion -ne 1 -or $manifest.kind -ne 'otter-native-internal-package' -or
        $manifest.buildVersion -notmatch '^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)\.[1-9]\d*$' -or
        $manifest.sourceSha256 -notmatch '^[a-f0-9]{64}$' -or $manifest.publishable -isnot [bool] -or
        $manifest.signed -isnot [bool] -or $manifest.runtimeVerified -isnot [bool] -or $manifest.publishable -cne $false -or
        $manifest.signed -cne $false -or $manifest.runtimeVerified -cne $false -or
        @($manifest.files).Count -lt 4 -or @($manifest.files).Count -gt 4095) { throw 'Invalid native package manifest' }
    $expected = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    [void]$expected.Add('package-manifest.json')
    $records = @{}
    foreach ($file in $manifest.files) {
        $name = Assert-OtterSafeRelativePath -RelativePath $file.path
        if (-not $expected.Add($name) -or $file.sha256 -notmatch '^[a-f0-9]{64}$' -or
            $file.bytes -isnot [ValueType] -or [long]$file.bytes -ne $file.bytes -or $file.bytes -lt 0 -or $file.bytes -gt 64MB) {
            throw 'Invalid native package file record'
        }
        $digest = Get-OtterArchiveEntryDigest -ZipPath $ZipPath -EntryName $file.path -MaxBytes 64MB
        if ($digest.bytes -ne $file.bytes -or $digest.sha256 -cne $file.sha256) { throw "Native ZIP content mismatch: $name" }
        $records[$name] = $file
    }
    if (-not $expected.SetEquals([string[]]$entries)) { throw 'Native ZIP has missing or extra files' }
    foreach ($required in @('otter-native.exe', 'README.txt', 'build-provenance.json', 'licenses\inventory.json')) {
        if (-not $records.ContainsKey($required)) { throw "Missing native package component: $required" }
    }
    $receipt = (Read-OtterArchiveEntryText -ZipPath $ZipPath -EntryName 'build-provenance.json' -MaxBytes 4MB) | ConvertFrom-Json
    $exe = $records['otter-native.exe']
    if ($receipt.kind -ne 'otter-native-internal-candidate' -or $receipt.buildVersion -cne $manifest.buildVersion -or
        $receipt.sourceSha256 -cne $manifest.sourceSha256 -or $receipt.publishable -isnot [bool] -or
        $receipt.signed -isnot [bool] -or $receipt.runtimeVerified -isnot [bool] -or $receipt.publishable -cne $false -or
        $receipt.signed -cne $false -or $receipt.runtimeVerified -cne $false -or
        $receipt.files[0].path -cne 'payload/otter-native.exe' -or
        $receipt.files[0].sha256 -cne $exe.sha256 -or $receipt.files[0].bytes -ne $exe.bytes) { throw 'Native ZIP build identity mismatch' }
    $licenses = (Read-OtterArchiveEntryText -ZipPath $ZipPath -EntryName 'licenses/inventory.json' -MaxBytes 4MB) | ConvertFrom-Json
    if ($licenses.schemaVersion -ne 1 -or $licenses.kind -ne 'otter-native-license-materials' -or
        $licenses.legalClearance -isnot [bool] -or $licenses.legalClearance -cne $false -or @($licenses.packages).Count -lt 1) { throw 'Invalid native license inventory' }
    $licensePaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    [void]$licensePaths.Add('licenses\inventory.json')
    foreach ($package in $licenses.packages) {
        if (@($package.files).Count -lt 1) { throw 'Empty native license package' }
        foreach ($file in $package.files) {
            $name = Assert-OtterSafeRelativePath -RelativePath ('licenses/' + $file.path)
            if (-not $licensePaths.Add($name) -or -not $records.ContainsKey($name) -or
                $records[$name].sha256 -cne $file.sha256 -or $records[$name].bytes -ne $file.bytes) { throw 'Native license material mismatch' }
        }
    }
    $actualLicenses = @($records.Keys | Where-Object { $_.StartsWith('licenses\', [StringComparison]::OrdinalIgnoreCase) })
    if (-not $licensePaths.SetEquals([string[]]$actualLicenses)) { throw 'Uninventoried native license material' }
    return [pscustomobject]@{ buildVersion = $manifest.buildVersion; files = $entries.Count; executableSha256 = $exe.sha256; publishable = $false }
}
