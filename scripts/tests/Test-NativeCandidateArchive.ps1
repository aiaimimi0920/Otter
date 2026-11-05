param(
    [Parameter(Mandatory = $true)][string]$ZipPath,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $repo 'scripts/file-hash.ps1')
. (Join-Path $repo 'scripts/release/PathSafety.ps1')
. (Join-Path $repo 'scripts/native-candidate/Archive.ps1')
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Test output must be fresh' }
[void][IO.Directory]::CreateDirectory($OutputDirectory)
$baseline = Test-NativeCandidateArchive -ZipPath $ZipPath
if ($baseline.files -lt 5) { throw 'Expected populated archive fixture' }
$passed = 1
foreach ($case in @('tamper', 'missing', 'extra', 'traversal', 'duplicate', 'false-string', 'angle-egl', 'angle-gles')) {
    $copy = Join-Path $OutputDirectory ($case + '.zip')
    Copy-Item -LiteralPath $ZipPath -Destination $copy
    $archive = [IO.Compression.ZipFile]::Open($copy, [IO.Compression.ZipArchiveMode]::Update)
    try {
        switch ($case) {
            'missing' { $archive.GetEntry('README.txt').Delete(); break }
            'tamper' { $name = 'otter-native.exe'; $archive.GetEntry($name).Delete(); $text = 'wrong executable'; break }
            'extra' { $name = 'extra.txt'; $text = 'extra'; break }
            'traversal' { $name = '../escape.txt'; $text = 'escape'; break }
            'duplicate' { $name = 'README.TXT'; $text = 'duplicate'; break }
            'angle-egl' { $name = 'libEGL.dll'; $text = 'retired'; break }
            'angle-gles' { $name = 'nested/LIBGLESv2.DLL'; $text = 'retired'; break }
            'false-string' {
                $name = 'package-manifest.json'
                $entry = $archive.GetEntry($name)
                $reader = [IO.StreamReader]::new($entry.Open())
                try { $manifest = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
                $manifest.publishable = 'False'
                $text = $manifest | ConvertTo-Json -Depth 30
                $entry.Delete()
                break
            }
        }
        if ($case -ne 'missing') {
            $entry = $archive.CreateEntry($name)
            $writer = [IO.StreamWriter]::new($entry.Open(), [Text.UTF8Encoding]::new($false))
            try { $writer.Write($text) } finally { $writer.Dispose() }
        }
    } finally { $archive.Dispose() }
    $rejected = $false
    try { [void](Test-NativeCandidateArchive -ZipPath $copy) } catch {
        if ($case.StartsWith('angle-') -and $_.ToString() -notmatch 'Retired ANGLE DLL') { throw }
        $rejected = $true
        [IO.File]::WriteAllText((Join-Path $OutputDirectory ($case + '.log')), $_.ToString(), [Text.UTF8Encoding]::new($false))
    }
    if (-not $rejected) { throw "Invalid ZIP accepted: $case" }
    $passed++
}
Write-Output "Native archive contracts: $passed passed"
