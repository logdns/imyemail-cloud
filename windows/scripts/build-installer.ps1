# Packages an already verified self-contained Windows publish. Does not download tools.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('x64', 'ARM64')][string]$Architecture,
    [Parameter(Mandatory = $true)][string]$PublishDirectory,
    [string]$OutputDirectory = '',
    [string]$IsccPath = '',
    [string]$Version = '',
    [ValidatePattern('^[A-Za-z0-9._-]+$')][string]$BuildLabel = (Get-Date -Format 'yyyyMMdd')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'The installer compiler must run on Windows.'
}
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$PublishDirectory = (Resolve-Path -LiteralPath $PublishDirectory).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) { $OutputDirectory = Join-Path $ProjectRoot 'dist' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
if ([string]::IsNullOrWhiteSpace($Version)) {
    [xml]$Project = Get-Content -LiteralPath (Join-Path $ProjectRoot 'Chck.Mail\Chck.Mail.csproj')
    $VersionNode = $Project.SelectSingleNode('/Project/PropertyGroup/Version')
    if ($null -eq $VersionNode) { throw 'Project does not declare an installer version.' }
    $Version = $VersionNode.InnerText
}
if ($Version -notmatch '^\d+\.\d+\.\d+(\.\d+)?$') { throw "Invalid installer version: $Version" }
if ([string]::IsNullOrWhiteSpace($IsccPath)) {
    $Command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($Command) { $IsccPath = $Command.Source }
    else {
        $Candidates = @(
            (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
            (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe')
        )
        $IsccPath = $Candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    }
}
if ([string]::IsNullOrWhiteSpace($IsccPath) -or -not (Test-Path -LiteralPath $IsccPath)) {
    throw 'Install Inno Setup 6.7.0 or newer and provide -IsccPath. https://github.com/jrsoftware/issrc/releases/download/is-6_7_0/innosetup-6.7.0.exe'
}
foreach ($Required in @('imyemail-cloud.exe', 'imyemail-cloud.dll', 'coreclr.dll', 'Microsoft.UI.Xaml.Controls.dll', 'Microsoft.WindowsAppRuntime.dll', 'Assets\imyemail-cloud.ico', 'THIRD-PARTY-NOTICES.txt', 'MarkItDown\python.exe', 'MarkItDown\convert.py', 'MarkItDown\runtime-lock.json', 'MarkItDown\Licenses\sources.json')) {
    if (-not (Test-Path -LiteralPath (Join-Path $PublishDirectory $Required))) {
        throw "Incomplete self-contained publish: missing $Required"
    }
}
$RuntimeLock = Get-Content (Join-Path $PublishDirectory 'MarkItDown\runtime-lock.json') -Raw | ConvertFrom-Json
if (@($RuntimeLock.wheels | Where-Object { $_.name -eq 'markitdown' -and $_.version -eq '0.1.7' }).Count -ne 1) { throw 'Unexpected MarkItDown runtime version.' }
$Stream = [IO.File]::OpenRead((Join-Path $PublishDirectory 'imyemail-cloud.exe'))
$Reader = New-Object IO.BinaryReader($Stream)
try {
    if ($Reader.ReadUInt16() -ne 0x5A4D) { throw 'The app executable is not a PE file.' }
    $Stream.Position = 0x3C
    $PeOffset = $Reader.ReadInt32()
    $Stream.Position = $PeOffset
    if ($Reader.ReadUInt32() -ne 0x00004550) { throw 'Invalid PE signature.' }
    $Machine = $Reader.ReadUInt16()
} finally { $Reader.Dispose(); $Stream.Dispose() }
$ExpectedMachine = if ($Architecture -eq 'ARM64') { 0xAA64 } else { 0x8664 }
if ($Machine -ne $ExpectedMachine) { throw "Publish architecture does not match $Architecture (PE machine $Machine)." }

# Reject dirty runtime directories instead of silently shipping local account data.
$PrivateFiles = @(Get-ChildItem -LiteralPath $PublishDirectory -Recurse -File | Where-Object {
    $_.Name -match '(?i)(\.(db|sqlite|sqlite3)(-wal|-shm)?$|\.secrets$|^preferences\.json$|^notifications\.log$)'
})
$PrivateDirectories = @(Get-ChildItem -LiteralPath $PublishDirectory -Recurse -Directory | Where-Object {
    $_.Name -match '^(?:.*\.)?WebView2$'
})
if ($PrivateFiles.Count -gt 0 -or $PrivateDirectories.Count -gt 0) { throw 'Publish contains runtime data. Build to a fresh output directory before packaging.' }
# Keep packaging read-only: optimization must happen before build review and hashing.
& (Join-Path $PSScriptRoot 'optimize-publish.ps1') -PublishDirectory $PublishDirectory -CheckOnly
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$OutputName = 'imyemail-cloud-native-windows-' + $Architecture.ToLowerInvariant() + '-' + $BuildLabel + '-setup'
$Installer = Join-Path $OutputDirectory ($OutputName + '.exe')
$CompilerArgs = @(
    "/DPublishDir=$PublishDirectory", "/DOutputDir=$OutputDirectory", "/DArchitecture=$Architecture",
    "/DAppVersion=$Version", "/DOutputName=$OutputName", (Join-Path $ProjectRoot 'installer\imyemail-cloud.iss')
)
& $IsccPath @CompilerArgs
if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed with exit code $LASTEXITCODE." }
if (-not (Test-Path -LiteralPath $Installer)) { throw 'Inno Setup did not produce the expected installer.' }
$Hash = (Get-FileHash -LiteralPath $Installer -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText(($Installer + '.sha256'), "$Hash  $([IO.Path]::GetFileName($Installer))`n", (New-Object Text.UTF8Encoding($false)))
Write-Host "Installer: $Installer"
Write-Host "SHA256: $Hash"
Write-Host 'Development installer is unsigned. Validate install, upgrade, launch and uninstall on Windows.'
