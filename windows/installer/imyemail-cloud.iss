; Compile through scripts/build-installer.ps1 with Inno Setup 6.7.0 or newer.
#ifndef PublishDir
  #error PublishDir is required
#endif
#ifndef OutputDir
  #error OutputDir is required
#endif
#ifndef Architecture
  #error Architecture is required (x64 or ARM64)
#endif
#ifndef AppVersion
  #define AppVersion "0.2.0"
#endif
#ifndef OutputName
  #define OutputName "imyemail-cloud-windows-" + LowerCase(Architecture) + "-setup"
#endif

[Setup]
; One product identity across x64 and ARM64; an architecture switch is an upgrade.
AppId={{39D3BD46-178F-4D6E-9D44-D42A1EDE76E7}
AppName=imyemail-cloud
AppVersion={#AppVersion}
AppPublisher=imyemail-cloud
AppPublisherURL=https://imy.email
AppSupportURL=https://imy.email
DefaultDirName={localappdata}\Programs\imyemail-cloud
DefaultGroupName=imyemail-cloud
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
MinVersion=10.0.17763
#if Architecture == "ARM64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#elif Architecture == "x64"
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#else
  #error Unsupported architecture
#endif
OutputDir={#OutputDir}
OutputBaseFilename={#OutputName}
SetupIconFile={#PublishDir}\Assets\imyemail-cloud.ico
UninstallDisplayIcon={app}\imyemail-cloud.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
; Keep first-run installer UI in English on every host; app language remains configurable after install.
LanguageDetectionMethod=none
CloseApplications=yes
CloseApplicationsFilter=*.exe,*.dll
RestartApplications=no
; Never force-close a process that may contain an unsent draft.
AllowCancelDuringInstall=no
UninstallDisplayName=imyemail-cloud
VersionInfoVersion={#AppVersion}
VersionInfoDescription=imyemail-cloud ({#Architecture}) Setup

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "zh_CN"; MessagesFile: "Languages\ChineseSimplified.isl"
Name: "zh_TW"; MessagesFile: "Languages\ChineseTraditional.isl"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"

[CustomMessages]
en.WebViewRequired=Microsoft Edge WebView2 Runtime is required to display emails. Install the Evergreen Runtime from https://developer.microsoft.com/microsoft-edge/webview2/ and run this installer again. No download or installation will start automatically.
zh_CN.WebViewRequired=显示邮件需要 Microsoft Edge WebView2 Runtime。请从 https://developer.microsoft.com/microsoft-edge/webview2/ 安装 Evergreen Runtime，然后重新运行本安装程序。本程序不会自动下载或安装它。
zh_TW.WebViewRequired=顯示郵件需要 Microsoft Edge WebView2 Runtime。請從 https://developer.microsoft.com/microsoft-edge/webview2/ 安裝 Evergreen Runtime，然後重新執行本安裝程式。本程式不會自動下載或安裝它。
ja.WebViewRequired=メールの表示には Microsoft Edge WebView2 Runtime が必要です。https://developer.microsoft.com/microsoft-edge/webview2/ から Evergreen Runtime をインストールしてから、このインストーラーを再実行してください。自動的なダウンロードやインストールは行いません。
es.WebViewRequired=Para mostrar los correos se necesita Microsoft Edge WebView2 Runtime. Instale Evergreen Runtime desde https://developer.microsoft.com/microsoft-edge/webview2/ y vuelva a ejecutar este instalador. No se iniciará ninguna descarga ni instalación automática.
fr.WebViewRequired=Microsoft Edge WebView2 Runtime est nécessaire pour afficher les messages. Installez Evergreen Runtime depuis https://developer.microsoft.com/microsoft-edge/webview2/ puis relancez cet installateur. Aucun téléchargement ni aucune installation ne démarrera automatiquement.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce

[Files]
Source: "{#PublishDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs; Excludes: "*.pdb,*.db,*.db-shm,*.db-wal,*.sqlite,*.sqlite3,*.secrets,preferences.json,notifications.log,WebView2\*,*.WebView2\*"

[InstallDelete]
; Retire only this product's previous shortcut names during a brand upgrade.
Type: files; Name: "{userprograms}\imyemail-cloud.lnk"
Type: files; Name: "{userdesktop}\imyemail-cloud.lnk"
; This is an application-owned dependency directory, never the mail/profile path.
; Remove obsolete RID-specific dependencies when switching x64 <-> ARM64.
Type: filesandordirs; Name: "{app}\runtimes"
Type: filesandordirs; Name: "{app}\MarkItDown"

[Icons]
Name: "{userprograms}\imyemail-cloud"; Filename: "{app}\imyemail-cloud.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\imyemail-cloud"; Filename: "{app}\imyemail-cloud.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\imyemail-cloud.exe"; Description: "{cm:LaunchProgram,imyemail-cloud}"; Flags: nowait postinstall skipifsilent

; No UninstallDelete section: account databases and credentials stay in the
; user's profile. Only files owned by this installer are removed.
[Code]
function HasWebViewRuntime(RootKey: Integer): Boolean;
var
  Version: String;
begin
  Result := RegQueryStringValue(RootKey,
    'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'pv', Version) and (Version <> '') and (Version <> '0.0.0.0');
end;

function InitializeSetup(): Boolean;
begin
  Result := HasWebViewRuntime(HKCU32) or HasWebViewRuntime(HKCU64) or
    HasWebViewRuntime(HKLM32) or HasWebViewRuntime(HKLM64);
  if not Result then
    SuppressibleMsgBox(CustomMessage('WebViewRequired'), mbError, MB_OK, IDOK);
end;
