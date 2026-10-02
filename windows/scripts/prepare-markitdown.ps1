param(
    [Parameter(Mandatory=$true)][string]$PublishDirectory,
    [string]$WheelCache = (Join-Path $env:LOCALAPPDATA 'imy.email-build-cache\markitdown-win-x64'),
    [string]$WorkerPath = (Join-Path $PSScriptRoot '..\tools\markitdown\convert.py'),
    [switch]$Offline
)
$ErrorActionPreference = 'Stop'
$metadata = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\tools\markitdown-runtime'))
$lock = Get-Content (Join-Path $metadata 'runtime-lock.json') -Raw | ConvertFrom-Json
if ($lock.schemaVersion -ne 1 -or $lock.python.architecture -ne 'x64') { throw 'Unsupported MarkItDown runtime lock' }
if (!(Test-Path -LiteralPath $WorkerPath -PathType Leaf)) { throw "Missing conversion worker: $WorkerPath" }
$publish = [IO.Path]::GetFullPath($PublishDirectory)
$cache = [IO.Path]::GetFullPath($WheelCache)
$destination = Join-Path $publish 'MarkItDown'
New-Item -ItemType Directory -Force -Path $publish, $cache | Out-Null
# Build atomically beside the target. Never leave a partially refreshed runtime.
$staging = Join-Path $publish ('.markitdown-build-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $staging | Out-Null
Add-Type -AssemblyName System.IO.Compression.FileSystem
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Get-VerifiedArtifact($artifact) {
    $uri = [Uri]$artifact.url
    if ($uri.Scheme -ne 'https' -or $uri.Host -notin @('www.python.org', 'files.pythonhosted.org')) { throw 'Untrusted artifact source' }
    if ($artifact.filename -ne [IO.Path]::GetFileName($artifact.filename)) { throw 'Invalid cached artifact filename' }
    $path = Join-Path $cache $artifact.filename
    if ((Test-Path -LiteralPath $path) -and (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -eq $artifact.sha256) { return $path }
    if ($Offline) { throw "Missing or invalid offline artifact: $($artifact.filename)" }
    $temporary = $path + '.' + [Guid]::NewGuid().ToString('N') + '.download'
    try {
        Invoke-WebRequest -Uri $uri.AbsoluteUri -OutFile $temporary -UseBasicParsing
        if ((Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash -ne $artifact.sha256) { throw "Artifact SHA256 mismatch: $($artifact.filename)" }
        Move-Item -LiteralPath $temporary -Destination $path -Force
    }
    finally { if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force } }
    return $path
}

function Expand-VerifiedZip([string]$archivePath, [string]$target, [bool]$wheel) {
    $archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
    try {
        foreach ($entry in $archive.Entries) {
            $relative = $entry.FullName.Replace('\', '/')
            $entryTarget = $target
            if ($relative.EndsWith('/')) { continue }
            if ($wheel -and $relative -match '^[^/]+\.data/(.+)$') {
                $dataPath = $Matches[1]
                if ($dataPath -match '^(purelib|platlib)/(.+)$') { $relative = $Matches[2] }
                elseif ($dataPath -match '^(scripts|headers)/') { continue } # CLI launchers / C headers are not runtime imports.
                elseif ($dataPath -match '^data/(.+)$') { $relative = $Matches[1]; $entryTarget = $staging }
                else { throw "Unmapped wheel data payload: $relative" }
            }
            $file = [IO.Path]::GetFullPath((Join-Path $entryTarget $relative))
            $prefix = [IO.Path]::GetFullPath($entryTarget).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
            if (!$file.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Archive entry escapes destination' }
            New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($file)) | Out-Null
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $file, $true)
        }
    }
    finally { $archive.Dispose() }
}

try {
    Expand-VerifiedZip (Get-VerifiedArtifact $lock.python) $staging $false
    $sitePackages = Join-Path $staging 'Lib\site-packages'
    New-Item -ItemType Directory -Force -Path $sitePackages | Out-Null
    foreach ($package in $lock.wheels) {
        Write-Host ('Bundling ' + $package.name + ' ' + $package.version)
        Expand-VerifiedZip (Get-VerifiedArtifact $package) $sitePackages $true
    }
    # Explicit isolated paths: no host Python, user-site, PYTHONPATH, registry or pip.
    @('python313.zip', '.', 'Lib\site-packages') | Set-Content (Join-Path $staging 'python313._pth') -Encoding ascii
    Copy-Item -LiteralPath $WorkerPath -Destination (Join-Path $staging 'convert.py')
    Copy-Item -LiteralPath (Join-Path $metadata 'runtime-lock.json') -Destination $staging
    Copy-Item -LiteralPath (Join-Path $metadata 'THIRD-PARTY-NOTICES.md') -Destination $staging
    Copy-Item -LiteralPath (Join-Path $metadata 'python-embed.sigstore.json') -Destination $staging
    $licenseSource = Join-Path $metadata 'licenses'
    foreach ($license in (Get-Content (Join-Path $licenseSource 'sources.json') -Raw | ConvertFrom-Json)) {
        if ((Get-FileHash -LiteralPath (Join-Path $licenseSource $license.filename) -Algorithm SHA256).Hash -ne $license.sha256) { throw 'Supplemental dependency license hash mismatch' }
    }
    Copy-Item -LiteralPath $licenseSource -Destination (Join-Path $staging 'Licenses') -Recurse
    # A child command validates the actual bundled native DLLs under x64 or ARM64
    # Windows compatibility mode. It does not contact any network endpoint.
    $check = @'
import importlib.metadata as m, io, json, struct, sys
import markitdown, mammoth, pdfminer, pdfplumber, pptx, openpyxl, pandas, pypdfium2, onnxruntime
from lxml import etree
from markitdown.converters import HtmlConverter
assert sys.version_info[:3] == (3, 13, 15), sys.version
assert struct.calcsize('P') == 8
assert m.version('markitdown') == '0.1.7'
assert 'offline import check' in HtmlConverter().convert_string('<p>offline import check</p>').markdown
print(json.dumps({'python': sys.version.split()[0], 'markitdown': m.version('markitdown'), 'architecture': 'x64', 'packages': len(list(m.distributions())), 'offlineImports': True}))
'@
    & (Join-Path $staging 'python.exe') -I -B -c $check
    if ($LASTEXITCODE -ne 0) { throw 'Embedded MarkItDown runtime import validation failed' }
    $probeFile = Join-Path $staging 'offline-conversion-check.html'
    try {
        '<h1>Offline worker check</h1>' | Set-Content -LiteralPath $probeFile -Encoding UTF8
        $response = & (Join-Path $staging 'python.exe') -I -B (Join-Path $staging 'convert.py') $probeFile
        if ($LASTEXITCODE -ne 0) { throw 'Embedded MarkItDown worker conversion failed' }
        $converted = $response | ConvertFrom-Json
        if ($converted.version -ne '0.1.7' -or $converted.markdown -notmatch 'Offline worker check') { throw 'Unexpected worker conversion result' }
    }
    finally { if (Test-Path -LiteralPath $probeFile) { Remove-Item -LiteralPath $probeFile -Force } }
    # Preserve upstream package modules, DLLs, models, data, dist-info and licenses.
    # Only caches and debug symbols are optional runtime-unrelated payloads.
    Get-ChildItem -LiteralPath $staging -Recurse -File -Filter '*.pdb' | Remove-Item -Force
    Get-ChildItem -LiteralPath $staging -Recurse -Directory -Filter '__pycache__' | Sort-Object FullName -Descending | Remove-Item -Recurse -Force
    if (Test-Path -LiteralPath $destination) {
        if (!(Test-Path -LiteralPath (Join-Path $destination 'runtime-lock.json'))) { throw 'Refusing to replace an unrecognized MarkItDown directory' }
        Remove-Item -LiteralPath $destination -Recurse -Force
    }
    Move-Item -LiteralPath $staging -Destination $destination
    Write-Host ('MarkItDown offline runtime ready: ' + $destination)
}
finally { if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force } }
