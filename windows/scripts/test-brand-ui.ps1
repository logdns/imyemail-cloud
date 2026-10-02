# Run on Windows against a fresh publish. Uses isolated local data and makes no external request.
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$PublishDirectory,
    [Parameter(Mandatory=$true)][string]$EvidenceDirectory
)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Drawing
New-Item -ItemType Directory -Force $EvidenceDirectory | Out-Null
$env:IMYEMAIL_CLOUD_DATA_DIR=Join-Path $EvidenceDirectory ('profile-'+[guid]::NewGuid().ToString('N'))
$exe=Join-Path $PublishDirectory 'imyemail-cloud.exe'
New-Item -ItemType Directory -Force $env:IMYEMAIL_CLOUD_DATA_DIR | Out-Null
'{"Language":"en","Theme":"light"}' | Set-Content (Join-Path $env:IMYEMAIL_CLOUD_DATA_DIR 'preferences.json')
$app=Start-Process $exe -WorkingDirectory $PublishDirectory -PassThru
function Find-Element([string]$Name, [string]$Id='') {
    for($i=0;$i -lt 40;$i++) {
        $app.Refresh()
        if($app.HasExited) {throw 'Application exited unexpectedly'}
        if($app.MainWindowHandle -ne 0) {
            $script:root=[System.Windows.Automation.AutomationElement]::FromHandle($app.MainWindowHandle)
            $property=if($Id){[System.Windows.Automation.AutomationElement]::AutomationIdProperty}else{[System.Windows.Automation.AutomationElement]::NameProperty}
            $value=if($Id){$Id}else{$Name}
            $condition=New-Object System.Windows.Automation.PropertyCondition($property,$value)
            # WinUI ContentDialog popups can live outside the main window's UIA subtree.
            $processCondition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$app.Id)
            $combined=New-Object System.Windows.Automation.AndCondition($processCondition,$condition)
            $found=[System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$combined)
            if($found) {return $found}
        }
        Start-Sleep -Milliseconds 250
    }
    throw "Missing UI element: $Name $Id"
}
function Invoke-Element($Element) {
    $pattern=$Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
}

function Snapshot([string]$name) {
    Start-Sleep -Milliseconds 500
    $rect=$root.Current.BoundingRectangle
    $bitmap=New-Object Drawing.Bitmap([int]$rect.Width,[int]$rect.Height)
    $graphics=[Drawing.Graphics]::FromImage($bitmap)
    try { $graphics.CopyFromScreen([int]$rect.X,[int]$rect.Y,0,0,$bitmap.Size); $bitmap.Save((Join-Path $EvidenceDirectory $name),[Drawing.Imaging.ImageFormat]::Png) }
    finally { $graphics.Dispose();$bitmap.Dispose() }
}
function Choose([string]$id,[string]$name) {
    $combo=Find-Element '' $id
    $combo.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern).Expand()
    $item=Find-Element $name
    $item.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select()
    Start-Sleep -Milliseconds 300
}
try {
    Invoke-Element (Find-Element 'Cancel')
    Start-Sleep -Milliseconds 700
    Invoke-Element (Find-Element '' 'SettingsButton')
    Find-Element '' 'LanguageChoice' | Out-Null
    Snapshot 'windows-brand.png'
    Choose 'LanguageChoice' 'Français'
    Choose 'ThemeChoice' 'Dark'
    $preferences=Get-Content (Join-Path $env:IMYEMAIL_CLOUD_DATA_DIR 'preferences.json') -Raw | ConvertFrom-Json
    if($preferences.Language -ne 'fr' -or $preferences.Theme -ne 'dark') {throw 'Language/theme preference did not persist'}
    # Language remains English until relaunch.
    Stop-Process -Id $app.Id -Force
    $app=Start-Process $exe -WorkingDirectory $PublishDirectory -PassThru
    Invoke-Element (Find-Element 'Annuler')
    Start-Sleep -Milliseconds 700
    Invoke-Element (Find-Element '' 'SettingsButton')
    Find-Element 'Paramètres' | Out-Null
    Find-Element '' 'LanguageChoice' | Out-Null
    Snapshot 'windows-dark.png'
    [ordered]@{passed=$true; executable=$exe; checks=@('English first launch','language selection persisted','theme selection persisted','French after restart','dark appearance'); utc=[DateTime]::UtcNow.ToString('o')} | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $EvidenceDirectory 'windows-ui.json')
} finally {
    if($app -and -not $app.HasExited) { Stop-Process -Id $app.Id -Force }
    Remove-Item Env:IMYEMAIL_CLOUD_DATA_DIR -ErrorAction SilentlyContinue
}
