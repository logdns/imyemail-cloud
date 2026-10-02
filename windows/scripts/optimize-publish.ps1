# Limit separate language resources without altering WinUI's shared PRI indexes.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PublishDirectory,
    [string]$ReportPath = '',
    [switch]$CheckOnly
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = [IO.Path]::GetFullPath($PublishDirectory).TrimEnd([IO.Path]::DirectorySeparatorChar)
if (-not (Test-Path -LiteralPath (Join-Path $root 'imyemail-cloud.exe') -PathType Leaf) -or
    -not (Test-Path -LiteralPath (Join-Path $root 'imyemail-cloud.runtimeconfig.json') -PathType Leaf)) {
    throw 'Expected a published imyemail-cloud directory; no files were removed.'
}
if ((Get-Item -LiteralPath $root).Attributes -band [IO.FileAttributes]::ReparsePoint) {
    throw 'Publish directory must not be a symbolic link or junction.'
}
$files = @(Get-ChildItem -LiteralPath $root -Recurse -File)
if (Get-ChildItem -LiteralPath $root -Recurse -Directory | Where-Object { $_.Name -match '^(?:.*\.)?WebView2$' }) {
    throw 'Publish directory contains a WebView2 profile. Use a clean build output.'
}
if ($files | Where-Object { $_.Name -match '^(mail\.db(?:-.*)?|preferences\.json|notifications\.log)$' -or $_.Extension -eq '.secrets' }) {
    throw 'Publish directory contains application data. Use a clean build output.'
}
$before = ($files | Measure-Object Length -Sum).Sum
# Keep one regional resource set for each requested language plus its neutral
# .NET satellite fallback. Do not guess that every top-level directory is a locale.
$allowed = @('en', 'en-US', 'zh', 'zh-Hans', 'zh-CN', 'zh-Hant', 'zh-TW', 'ja', 'ja-JP', 'es', 'es-ES', 'fr', 'fr-FR')
$removed = @()
$retained = @()
foreach ($directory in Get-ChildItem -LiteralPath $root -Directory) {
    $name = $directory.Name
    $isLocale = $name -match '^[a-z]{2,3}(?:-[a-z0-9]{2,8})*$'
    if (-not $isLocale) { continue }
    # Every removed directory must contain only native/.NET localization files.
    $contents = @(Get-ChildItem -LiteralPath $directory.FullName -Recurse -File)
    if (-not $contents.Count -or ($contents | Where-Object { $_.Name -notmatch '\.(mui|resources\.dll)$' })) { continue }
    if ($allowed -contains $name) { $retained += $name; continue }
    if ($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing linked locale: $name" }
    if (-not $CheckOnly) { Remove-Item -LiteralPath $directory.FullName -Recurse -Force }
    $removed += $name
}
$pdbs = @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.pdb')
if ($CheckOnly) {
    if ($removed.Count -gt 0 -or $pdbs.Count -gt 0) {
        throw ('Publish is not optimized: {0} disallowed locale directories and {1} PDBs. Run scripts/optimize-publish.ps1 before packaging.' -f $removed.Count, $pdbs.Count)
    }
    Write-Host ('Publish language check passed: ' + (($retained | Sort-Object) -join ', '))
    return
}
foreach ($file in $pdbs) { Remove-Item -LiteralPath $file.FullName -Force }
$after = (Get-ChildItem -LiteralPath $root -Recurse -File | Measure-Object Length -Sum).Sum
$report = [ordered]@{
    languages = @('English', 'Simplified Chinese', 'Traditional Chinese', 'Japanese', 'Spanish', 'French')
    retainedLocaleDirectories = @($retained | Sort-Object)
    removedLocaleDirectories = @($removed | Sort-Object)
    removedDebugFiles = $pdbs.Count
    beforeBytes = $before
    afterBytes = $after
    savedBytes = $before - $after
    sharedPriIndexesPreserved = $true
}
if ($ReportPath) { $report | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $ReportPath -Encoding UTF8 }
Write-Host ("Publish optimized: {0} locale directories removed; {1} PDBs removed; {2:N0} bytes saved." -f $removed.Count,$pdbs.Count,($before-$after))
Write-Host ('Retained locale directories: ' + (($retained | Sort-Object) -join ', '))
