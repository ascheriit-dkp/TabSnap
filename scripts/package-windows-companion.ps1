param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\dist')
)

$ErrorActionPreference = 'Stop'

$exe = Join-Path $PSScriptRoot '..\apps\windows-companion\target\release\tabsnap-companion.exe'
if (-not (Test-Path $exe -PathType Leaf)) {
    throw "Companion executable not found: $exe"
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$stage = Join-Path ([System.IO.Path]::GetTempPath()) ("tabsnap-package-" + [guid]::NewGuid().ToString('N'))
$archiveName = "tabsnap-companion-windows-x64-$Version.zip"
$archive = Join-Path $OutputDirectory $archiveName
$checksum = "$archive.sha256"

try {
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    Copy-Item $exe (Join-Path $stage 'tabsnap-companion.exe')

    @'
TabSnap Companion

Portable Windows companion for TabSnap.

Run:
  tabsnap-companion.exe ui

Portable mode stores snapshots in a snapshots folder next to this executable.
No installer, account or cloud service is required.

Documentation:
https://github.com/ascheriit-dkp/TabSnap/tree/main/docs/guide
'@ | Set-Content -Path (Join-Path $stage 'README.txt') -Encoding utf8NoBOM

    if (Test-Path $archive) { Remove-Item $archive -Force }
    if (Test-Path $checksum) { Remove-Item $checksum -Force }

    # Compress-Archive preserves only the staged files we intentionally ship.
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $archive -CompressionLevel Optimal

    $hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
    "$hash  $archiveName" | Set-Content -Path $checksum -Encoding ascii -NoNewline

    Write-Host "archive=$archive"
    Write-Host "checksum=$checksum"
}
finally {
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
}
