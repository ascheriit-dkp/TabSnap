param(
    [string]$Tag,
    [string]$Destination
)

$ErrorActionPreference = 'Stop'

if (-not $Tag) {
    $version = (Get-Content (Join-Path $PSScriptRoot '..\package.json') -Raw | ConvertFrom-Json).version
    $Tag = "v$version"
}
if ($Tag -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$') {
    throw "Invalid release tag: $Tag"
}

if (-not $Destination) {
    $Destination = Join-Path $PWD ("tabsnap-qa-" + $Tag)
}
$Destination = [System.IO.Path]::GetFullPath($Destination)

if (Test-Path $Destination) {
    $existing = @(Get-ChildItem -LiteralPath $Destination -Force -ErrorAction SilentlyContinue)
    if ($existing.Count -gt 0) {
        throw "QA destination is not empty: $Destination"
    }
} else {
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
}

$downloads = Join-Path $Destination 'downloads'
$expanded = Join-Path $Destination 'expanded'
New-Item -ItemType Directory -Force -Path $downloads, $expanded | Out-Null

$archives = @(
    "tabsnap-chrome-$Tag.zip",
    "tabsnap-edge-$Tag.zip",
    "tabsnap-firefox-$Tag.zip",
    "tabsnap-companion-windows-x64-$Tag.zip"
)

$baseUrl = "https://github.com/ascheriit-dkp/TabSnap/releases/download/$Tag"
foreach ($archive in $archives) {
    foreach ($name in @($archive, "$archive.sha256")) {
        $target = Join-Path $downloads $name
        Write-Host "Downloading $name"
        Invoke-WebRequest -Uri "$baseUrl/$name" -OutFile $target
    }

    $archivePath = Join-Path $downloads $archive
    $checksumPath = "$archivePath.sha256"
    $checksumText = (Get-Content $checksumPath -Raw).Trim()
    if ($checksumText -notmatch '^([0-9a-f]{64})  (.+)$') {
        throw "Invalid checksum format for $archive"
    }
    if ($Matches[2] -ne $archive) {
        throw "Checksum names $($Matches[2]) instead of $archive"
    }
    $actualHash = (Get-FileHash -Algorithm SHA256 $archivePath).Hash.ToLowerInvariant()
    if ($actualHash -ne $Matches[1]) {
        throw "SHA-256 mismatch for $archive"
    }
}

$targets = @{
    chrome = "tabsnap-chrome-$Tag.zip"
    edge = "tabsnap-edge-$Tag.zip"
    firefox = "tabsnap-firefox-$Tag.zip"
    companion = "tabsnap-companion-windows-x64-$Tag.zip"
}
foreach ($name in $targets.Keys) {
    $target = Join-Path $expanded $name
    New-Item -ItemType Directory -Force -Path $target | Out-Null
    Expand-Archive -LiteralPath (Join-Path $downloads $targets[$name]) -DestinationPath $target
}

$version = $Tag.Substring(1)
foreach ($browser in @('chrome', 'edge', 'firefox')) {
    $manifestPath = Join-Path (Join-Path $expanded $browser) 'manifest.json'
    if (-not (Test-Path $manifestPath -PathType Leaf)) {
        throw "Missing $browser manifest.json"
    }
    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.version_name -ne $version) {
        throw "$browser version_name $($manifest.version_name) does not match $version"
    }
}

$companionRoot = Join-Path $expanded 'companion'
$companionFiles = @(
    Get-ChildItem -LiteralPath $companionRoot -File -Recurse |
        ForEach-Object { [System.IO.Path]::GetRelativePath($companionRoot, $_.FullName).Replace('\', '/') } |
        Sort-Object
)
$expectedCompanionFiles = @('README.txt', 'tabsnap-companion.exe')
if (($companionFiles -join [Environment]::NewLine) -ne ($expectedCompanionFiles -join [Environment]::NewLine)) {
    throw "Unexpected companion package contents: $($companionFiles -join ', ')"
}

$checklistSource = Join-Path $PSScriptRoot '..\docs\security\manual-release-qa.md'
$checklistTarget = Join-Path $Destination 'QA-CHECKLIST.md'
Copy-Item $checklistSource $checklistTarget -Force

Write-Host ''
Write-Host "TabSnap release QA workspace prepared for $Tag"
Write-Host "Chrome:    $(Join-Path $expanded 'chrome')"
Write-Host "Edge:      $(Join-Path $expanded 'edge')"
Write-Host "Firefox:   $(Join-Path $expanded 'firefox')"
Write-Host "Companion: $(Join-Path $expanded 'companion\tabsnap-companion.exe')"
Write-Host "Checklist: $checklistTarget"
Write-Host ''
Write-Host 'Next: open QA-CHECKLIST.md and execute the browser/removable-storage matrix.'
