Set-StrictMode -Version Latest

# Formal release paths are untrusted until containment and reparse-point checks pass.
function Assert-OtterSafeRelativePath {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    if ([string]::IsNullOrWhiteSpace($RelativePath) -or
        $RelativePath.IndexOf([char]0) -ge 0 -or
        [regex]::IsMatch($RelativePath, '[\x00-\x1f:]') -or
        [System.IO.Path]::IsPathRooted($RelativePath)) {
        throw "Invalid Otter release-relative path: $RelativePath"
    }
    $normalized = $RelativePath.Replace("/", "\")
    foreach ($segment in $normalized.Split(@("\"), [System.StringSplitOptions]::None)) {
        $baseName = $segment.Split('.')[0]
        if ([string]::IsNullOrWhiteSpace($segment) -or $segment -in @(".", "..") -or
            $segment.EndsWith(" ") -or $segment.EndsWith(".") -or
            $baseName -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$') {
            throw "Invalid Otter release-relative path: $RelativePath"
        }
    }
    return $normalized
}

function Resolve-OtterReleasePath {
    param(
        [Parameter(Mandatory = $true)][string]$RootPath,
        [Parameter(Mandatory = $true)][string]$RelativePath
    )
    $root = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/")
    $relative = Assert-OtterSafeRelativePath -RelativePath $RelativePath
    $candidate = [System.IO.Path]::GetFullPath((Join-Path $root $relative))
    if (-not $candidate.StartsWith($root + "\", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Otter release path escapes its root: $RelativePath"
    }
    return $candidate
}

function Assert-OtterNoReparsePoints {
    param(
        [Parameter(Mandatory = $true)][string]$RootPath,
        [Parameter(Mandatory = $true)][string]$Path
    )
    $root = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/")
    $candidate = [System.IO.Path]::GetFullPath($Path)
    if ($candidate -ne $root -and -not $candidate.StartsWith($root + "\", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Otter release path is outside its trusted root: $candidate"
    }
    $current = $root
    $paths = @($root)
    $relative = $candidate.Substring($root.Length).TrimStart("\", "/")
    foreach ($segment in $relative.Split(@("\"), [System.StringSplitOptions]::RemoveEmptyEntries)) {
        $current = Join-Path $current $segment
        $paths += $current
    }
    foreach ($entry in $paths) {
        if (-not (Test-Path -LiteralPath $entry)) { break }
        if (((Get-Item -LiteralPath $entry -Force).Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Otter release paths must not contain reparse points: $entry"
        }
    }
}

function Assert-OtterAbsolutePathNoReparsePoints {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string]$TrustedRootPath = ""
    )
    $candidate = [System.IO.Path]::GetFullPath($Path)
    $trustedRoot = if ([string]::IsNullOrWhiteSpace($TrustedRootPath)) {
        Split-Path -Parent $candidate
    } else {
        [System.IO.Path]::GetFullPath($TrustedRootPath)
    }
    Assert-OtterNoReparsePoints -RootPath $trustedRoot -Path $candidate
    return $candidate
}

function Get-OtterSafeReleaseFiles {
    param([Parameter(Mandatory = $true)][string]$RootPath)
    $root = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/")
    Assert-OtterNoReparsePoints -RootPath $root -Path $root
    $pending = [System.Collections.Generic.Stack[string]]::new()
    $files = [System.Collections.Generic.List[System.IO.FileInfo]]::new()
    $pending.Push($root)
    while ($pending.Count -gt 0) {
        foreach ($item in @(Get-ChildItem -LiteralPath $pending.Pop() -Force)) {
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Otter release paths must not contain reparse points: $($item.FullName)"
            }
            if ($item.PSIsContainer) { $pending.Push($item.FullName) } else { $files.Add($item) }
        }
    }
    return @($files)
}

function Get-OtterReleaseDigest {
    param([Parameter(Mandatory = $true)][string]$Path)
    $item = Get-Item -LiteralPath $Path
    return [pscustomobject]@{
        bytes = [int64]$item.Length
        sha256 = Get-OtterFileSha256 -Path $item.FullName
    }
}

function Read-OtterBoundedText {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [int64]$MaxBytes = 4MB
    )
    $item = Get-Item -LiteralPath $Path
    if ($item.Length -gt $MaxBytes) { throw "Otter release file exceeds the $MaxBytes-byte limit: $Path" }
    $encoding = [System.Text.UTF8Encoding]::new($false, $true)
    return [System.IO.File]::ReadAllText($item.FullName, $encoding)
}

function Get-OtterArchiveEntryIndex {
    param(
        [Parameter(Mandatory = $true)]$Archive,
        [int]$MaxEntries = 64,
        [int64]$MaxUncompressedBytes = 1GB
    )
    $entries = $Archive.Entries
    if ($entries.Count -gt $MaxEntries) { throw 'Otter archive exceeds its entry limit' }
    $index = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $total = 0L
    foreach ($entry in $entries) {
        $raw = $entry.FullName.TrimEnd("/", "\")
        if ([string]::IsNullOrWhiteSpace($raw)) { continue }
        $relative = Assert-OtterSafeRelativePath -RelativePath $raw
        if (-not $seen.Add($relative)) { throw "Duplicate Otter archive entry: $relative" }
        if ($entry.Name.Length -eq 0) { continue }
        if ($entry.Length -lt 0 -or $entry.Length -gt ($MaxUncompressedBytes - $total)) {
            throw 'Otter archive exceeds its uncompressed limit'
        }
        $total += $entry.Length
        $index.Add($entry.FullName, $entry)
    }
    return ,$index
}

function Read-OtterExactArchiveEntry {
    param($Entry, [byte[]]$TextBytes, [Security.Cryptography.IncrementalHash]$Hash)
    $remaining = [long]$Entry.Length
    $buffer = $TextBytes
    if ($null -ne $Hash) {
        $buffer = [byte[]]::new([int][Math]::Min(64KB, $remaining))
    }
    $offset = 0
    $stream = $Entry.Open()
    try {
        # The index budgets declared lengths. Read at most that many bytes plus
        # one EOF probe so actual expansion cannot bypass those budgets.
        while ($remaining -gt 0) {
            $count = $stream.Read($buffer, $offset, [int][Math]::Min(64KB, $remaining))
            if ($count -eq 0) { throw "Otter archive entry ended before its declared length: $($Entry.FullName)" }
            if ($null -ne $Hash) { $Hash.AppendData($buffer, 0, $count) }
            else { $offset += $count }
            $remaining -= $count
        }
        if ($stream.ReadByte() -ne -1) { throw "Otter archive entry exceeds its declared length: $($Entry.FullName)" }
    } finally {
        $stream.Dispose()
    }
}

function Get-OtterArchiveEntryDigest {
    param(
        [Parameter(Mandatory = $true)]$Index,
        [Parameter(Mandatory = $true)][string]$EntryName,
        [int64]$MaxBytes = 512MB
    )
    $entry = $null
    if (-not $Index.TryGetValue($EntryName.Replace("\", "/"), [ref]$entry) -or
        $entry.Length -lt 0 -or $entry.Length -gt $MaxBytes) {
        throw "Missing or oversized Otter archive entry: $EntryName"
    }
    $algorithm = [Security.Cryptography.IncrementalHash]::CreateHash([Security.Cryptography.HashAlgorithmName]::SHA256)
    try {
        Read-OtterExactArchiveEntry -Entry $entry -Hash $algorithm
        $hash = ([BitConverter]::ToString($algorithm.GetHashAndReset())).Replace("-", "").ToLowerInvariant()
        return [pscustomobject]@{ bytes = [int64]$entry.Length; sha256 = $hash }
    } finally {
        $algorithm.Dispose()
    }
}

function Read-OtterArchiveEntryText {
    param(
        [Parameter(Mandatory = $true)]$Index,
        [Parameter(Mandatory = $true)][string]$EntryName,
        [int]$MaxBytes = 1MB
    )
    $entry = $null
    if (-not $Index.TryGetValue($EntryName.Replace("\", "/"), [ref]$entry) -or
        $entry.Length -lt 0 -or $entry.Length -gt $MaxBytes) {
        throw "Missing or oversized Otter archive text entry: $EntryName"
    }
    $bytes = [byte[]]::new([int]$entry.Length)
    Read-OtterExactArchiveEntry -Entry $entry -TextBytes $bytes
    $encoding = [System.Text.UTF8Encoding]::new($false, $true)
    return $encoding.GetString($bytes)
}
