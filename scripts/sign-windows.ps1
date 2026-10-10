[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Directory)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:GITHUB_EVENT_NAME -ne 'workflow_dispatch' -or $env:GITHUB_REF -ne 'refs/heads/main') {
    throw 'Signing is restricted to manually requested main-branch releases.'
}
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw 'Signing requires an ephemeral GitHub-hosted runner.' }
$PublicPath = Join-Path $env:GITHUB_WORKSPACE 'signing/release-cert.der'
$Expected = New-Object Security.Cryptography.X509Certificates.X509Certificate2($PublicPath)
$PfxPath = Join-Path $env:RUNNER_TEMP 'release.p12'
[IO.File]::WriteAllBytes($PfxPath, [Convert]::FromBase64String($env:RELEASE_P12_BASE64))
$Password = ConvertTo-SecureString $env:RELEASE_P12_PASSWORD -AsPlainText -Force
Write-Host 'Importing private identity into the disposable CurrentUser My store'
$Certificate = Import-PfxCertificate -FilePath $PfxPath -CertStoreLocation Cert:\CurrentUser\My -Password $Password
if ($Certificate.Thumbprint -ne $Expected.Thumbprint) { throw 'Unexpected signing identity.' }
$Imported = @()
try {
    foreach ($Store in @('Root')) {
        Write-Host "Importing pinned public certificate into hosted LocalMachine $Store store"
        $Imported += Import-Certificate -FilePath $PublicPath -CertStoreLocation "Cert:\LocalMachine\$Store"
    }
    $Files = @(Get-ChildItem -LiteralPath $Directory -Recurse -File | Where-Object { $_.Name -match '^(imyemail-cloud.*|Chck\.Mail.*)\.(exe|dll)$' })
    if ($Files.Count -eq 0) { throw 'No Windows binaries found to sign.' }
    foreach ($File in $Files) {
        $Existing = Get-AuthenticodeSignature -LiteralPath $File.FullName
        if ($Existing.Status -eq 'NotSigned') {
            $Signature = Set-AuthenticodeSignature -LiteralPath $File.FullName -Certificate $Certificate -HashAlgorithm SHA256
            if ($Signature.Status -ne 'Valid' -or $Signature.SignerCertificate.Thumbprint -ne $Expected.Thumbprint) {
                throw "Signature verification failed: $($File.Name) ($($Signature.Status))"
            }
        } elseif ($Existing.Status -ne 'Valid' -or $Existing.SignerCertificate.Thumbprint -ne $Expected.Thumbprint) {
            throw "Invalid pre-existing signature: $($File.Name) ($($Existing.Status))"
        }
    }
    Write-Host "Verified Authenticode signatures on $($Files.Count) binaries. Self-signed publisher: $($Expected.Thumbprint)"
} finally {
    foreach ($Store in @('Root')) {
        Remove-Item "Cert:\LocalMachine\$Store\$($Expected.Thumbprint)" -ErrorAction SilentlyContinue
    }
    Remove-Item "Cert:\CurrentUser\My\$($Expected.Thumbprint)" -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $PfxPath -ErrorAction SilentlyContinue
}
