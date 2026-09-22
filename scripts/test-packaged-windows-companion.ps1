param(
    [Parameter(Mandatory = $true)]
    [string]$Archive,

    [Parameter(Mandatory = $true)]
    [string]$Checksum
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $Archive -PathType Leaf)) {
    throw "Companion archive not found: $Archive"
}
if (-not (Test-Path $Checksum -PathType Leaf)) {
    throw "Companion checksum not found: $Checksum"
}

$archiveLeaf = Split-Path $Archive -Leaf
$checksumText = (Get-Content $Checksum -Raw).Trim()
if ($checksumText -notmatch '^([0-9a-f]{64})  (.+)$') {
    throw 'Companion checksum file has an invalid format.'
}
if ($Matches[2] -ne $archiveLeaf) {
    throw "Companion checksum names $($Matches[2]) instead of $archiveLeaf"
}

$actualHash = (Get-FileHash -Algorithm SHA256 $Archive).Hash.ToLowerInvariant()
if ($actualHash -ne $Matches[1]) {
    throw "Companion SHA-256 mismatch: expected $($Matches[1]), got $actualHash"
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("tabsnap-published-qa-" + [guid]::NewGuid().ToString('N'))
$package = Join-Path $root 'package'
$custom = Join-Path $root 'custom'
$export = Join-Path $root 'export'

try {
    New-Item -ItemType Directory -Force -Path $package, $custom, $export | Out-Null
    Expand-Archive -LiteralPath $Archive -DestinationPath $package

    $relativeFiles = @(
        Get-ChildItem -LiteralPath $package -File -Recurse |
            ForEach-Object { [System.IO.Path]::GetRelativePath($package, $_.FullName).Replace('\', '/') } |
            Sort-Object
    )
    $expectedFiles = @('README.txt', 'tabsnap-companion.exe')
    if (($relativeFiles -join [Environment]::NewLine) -ne ($expectedFiles -join [Environment]::NewLine)) {
        throw "Unexpected companion package contents: $($relativeFiles -join ', ')"
    }

    $exe = Join-Path $package 'tabsnap-companion.exe'
    $info = & $exe info
    if ($LASTEXITCODE -ne 0) { throw 'packaged companion info command failed' }
    if (($info -join [Environment]::NewLine) -notmatch 'mode: portable') {
        throw 'packaged companion did not default to portable mode'
    }
    if (($info -join [Environment]::NewLine) -notmatch [regex]::Escape((Join-Path $package 'snapshots'))) {
        throw 'packaged companion portable storage is not beside the executable'
    }

    & $exe init | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'packaged companion init command failed' }

    & $exe storage set custom $custom | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'packaged companion custom storage command failed' }

    $opaque = Join-Path $root 'published.tabsnap'
    [System.IO.File]::WriteAllBytes($opaque, [byte[]](0x54,0x41,0x42,0x53,0x4e,0x41,0x50,0x00,0xff))

    & $exe library import $opaque | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'packaged companion library import failed' }

    $list = & $exe library list
    if ($LASTEXITCODE -ne 0) { throw 'packaged companion library list failed' }
    if (($list -join [Environment]::NewLine) -notmatch 'published.tabsnap') {
        throw 'packaged companion imported snapshot is absent from library list'
    }

    & $exe library export 'published.tabsnap' $export | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'packaged companion library export failed' }

    $sourceBytes = [System.IO.File]::ReadAllBytes($opaque)
    $exportBytes = [System.IO.File]::ReadAllBytes((Join-Path $export 'published.tabsnap'))
    if (-not [System.Linq.Enumerable]::SequenceEqual[byte]($sourceBytes, $exportBytes)) {
        throw 'packaged companion changed opaque snapshot bytes during import/export'
    }

    Write-Host 'Published Windows companion package verified.'
}
finally {
    Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
}
