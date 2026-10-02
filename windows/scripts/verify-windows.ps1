# Run from a Windows PowerShell / PowerShell 7 terminal in the checked-out project.
[CmdletBinding()]
param(
    [ValidateSet('x64', 'ARM64')][string[]]$Architectures = @('x64', 'ARM64'),
    [string]$OutputDirectory = '',
    [string]$MarkItDownCache = (Join-Path $env:LOCALAPPDATA 'imy.email-build-cache\markitdown-win-x64'),
    [switch]$OfflineMarkItDown
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'This verification must run inside Windows; macOS cannot execute the WinUI XAML compiler.'
}
$ProjectRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $ProjectRoot ('artifacts\windows-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
}
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$Transcript = Join-Path $OutputDirectory 'verification.log'
Start-Transcript -Path $Transcript | Out-Null
Push-Location $ProjectRoot
try {
    if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
        throw 'Install .NET 9 SDK: winget install --id Microsoft.DotNet.SDK.9 --exact ; then reopen this terminal.'
    }
    & dotnet --info
    if ($LASTEXITCODE -ne 0) { throw '.NET SDK inspection failed.' }
    # Let the SDK resolver apply global.json, including latestFeature roll-forward.
    $ResolvedSdk = & dotnet --version
    if ($LASTEXITCODE -ne 0 -or $ResolvedSdk -notmatch '^9\.0\.') {
        throw 'Install .NET SDK 9.0.318 or a compatible 9.0 feature band (global.json): https://dotnet.microsoft.com/download/dotnet/9.0'
    }
    Write-Host 'If WinUI/XAML tooling is missing, install Visual Studio 2022 17.12+ with WinUI application development/.NET desktop development and Windows 11 SDK.'
    Write-Host 'WebView2 Runtime is required to run the app: winget install --id Microsoft.EdgeWebView2Runtime --exact'
    & dotnet test Chck.Mail.Tests/Chck.Mail.Tests.csproj -c Release --logger 'trx;LogFileName=core-tests.trx' --results-directory $OutputDirectory
    if ($LASTEXITCODE -ne 0) { throw 'Core tests failed; application packaging stopped.' }
    foreach ($Architecture in $Architectures) {
        $Rid = 'win-' + $Architecture.ToLowerInvariant()
        $PublishDirectory = Join-Path $OutputDirectory $Rid
        if (Test-Path $PublishDirectory) { throw "Publish directory already exists; choose a new OutputDirectory: $PublishDirectory" }
        & dotnet build Chck.Mail/Chck.Mail.csproj -c Release "-p:Platform=$Architecture" "-r:$Rid"
        if ($LASTEXITCODE -ne 0) { throw "WinUI build failed for $Architecture." }
        & dotnet publish Chck.Mail/Chck.Mail.csproj -c Release "-p:Platform=$Architecture" "-r:$Rid" --self-contained true '-p:WindowsAppSDKSelfContained=true' '-p:WindowsPackageType=None' '-p:PublishSingleFile=false' -o $PublishDirectory
        if ($LASTEXITCODE -ne 0) { throw "Unpackaged publish failed for $Architecture." }
        $Executable = Join-Path $PublishDirectory 'imyemail-cloud.exe'
        if (-not (Test-Path $Executable)) { throw "Missing expected executable: $Executable" }
        & (Join-Path $PSScriptRoot 'prepare-markitdown.ps1') -PublishDirectory $PublishDirectory -WheelCache $MarkItDownCache -Offline:$OfflineMarkItDown
        & (Join-Path $PSScriptRoot 'optimize-publish.ps1') -PublishDirectory $PublishDirectory -ReportPath (Join-Path $OutputDirectory "$Rid-optimization.json")
        Get-ChildItem -Path $PublishDirectory -Recurse -File | Get-FileHash -Algorithm SHA256 |
            Select-Object Hash, Path | Export-Csv -NoTypeInformation -Path (Join-Path $OutputDirectory "$Rid-sha256.csv")
        Write-Host "Built $Executable"
    }
    Write-Host 'Verification succeeded. These are unpackaged development artifacts, not signed MSIX or Store submissions.'
    Write-Host 'Run the executable matching this Windows architecture for UI, notification-permission and WebView2 smoke verification.'
} finally {
    Pop-Location
    Stop-Transcript | Out-Null
}
