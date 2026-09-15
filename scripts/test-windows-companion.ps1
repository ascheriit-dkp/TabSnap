$ErrorActionPreference = 'Stop'

$source = Join-Path $PSScriptRoot '..\apps\windows-companion\target\release\tabsnap-companion.exe'
if (-not (Test-Path $source)) {
    throw "Companion executable not found: $source"
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("tabsnap-smoke-" + [guid]::NewGuid().ToString('N'))
$portable = Join-Path $root 'portable'
$custom = Join-Path $root 'custom-library'
$export = Join-Path $root 'exported'
New-Item -ItemType Directory -Force -Path $portable, $custom, $export | Out-Null

try {
    $exe = Join-Path $portable 'tabsnap-companion.exe'
    Copy-Item $source $exe

    $info = & $exe info
    if ($LASTEXITCODE -ne 0) { throw 'info command failed' }
    if (($info -join "`n") -notmatch 'mode: portable') { throw 'portable mode is not the default' }
    if (($info -join "`n") -notmatch [regex]::Escape((Join-Path $portable 'snapshots'))) {
        throw 'portable snapshots directory does not resolve next to the executable'
    }

    & $exe init | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'init command failed' }
    if (-not (Test-Path (Join-Path $portable 'snapshots') -PathType Container)) {
        throw 'portable snapshots directory was not created'
    }

    & $exe storage set custom $custom | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'custom storage command failed' }

    $opaque = Join-Path $root 'smoke.tabsnap'
    [System.IO.File]::WriteAllBytes($opaque, [byte[]](0x54,0x41,0x42,0x53,0x4e,0x41,0x50,0x00,0xff))
    $importOutput = & $exe library import $opaque
    if ($LASTEXITCODE -ne 0) { throw 'library import failed' }
    if (($importOutput -join "`n") -notmatch 'smoke.tabsnap') { throw 'import did not report the snapshot name' }

    $list = & $exe library list
    if ($LASTEXITCODE -ne 0) { throw 'library list failed' }
    if (($list -join "`n") -notmatch 'smoke.tabsnap') { throw 'imported snapshot is absent from library list' }

    & $exe library export 'smoke.tabsnap' $export | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'library export failed' }
    $exported = Join-Path $export 'smoke.tabsnap'
    if (-not (Test-Path $exported -PathType Leaf)) { throw 'exported snapshot is missing' }

    $sourceBytes = [System.IO.File]::ReadAllBytes($opaque)
    $exportBytes = [System.IO.File]::ReadAllBytes($exported)
    if (-not [System.Linq.Enumerable]::SequenceEqual[byte]($sourceBytes, $exportBytes)) {
        throw 'opaque snapshot bytes changed during import/export'
    }

    $config = Join-Path $portable 'tabsnap-companion.conf'
    if (-not (Test-Path $config -PathType Leaf)) { throw 'storage configuration was not persisted next to the executable' }
    $configText = Get-Content $config -Raw
    if ($configText -notmatch 'mode=custom') { throw 'custom storage mode was not persisted' }

    Write-Host 'Windows companion smoke test passed.'
}
finally {
    Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
}
