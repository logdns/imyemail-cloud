# Windows EXE installer

[Install both editions](../../docs/INSTALL.md) · [Self-signed Authenticode](../../docs/SIGNING.md) · [Preserved versions](../../docs/VERSIONS.md). Tagged native releases sign the project binaries and installer; local builds stay unsigned by default. MyGo uses a separate portable ZIP, not this installer. Inno's generated uninstaller is not separately Authenticode-signed in this workflow.

Requires **Inno Setup 6.7.0 or newer**, installed from
https://github.com/jrsoftware/issrc/releases/download/is-6_7_0/innosetup-6.7.0.exe.
The build script never downloads or executes a bootstrapper.

Run the normal Windows verification/publish first, then package each clean publish:

```powershell
.\scripts\build-installer.ps1 -Architecture x64 -PublishDirectory .\artifacts\release\win-x64 -OutputDirectory .\dist
.\scripts\build-installer.ps1 -Architecture ARM64 -PublishDirectory .\artifacts\release\win-arm64 -OutputDirectory .\dist
```

Use `-IsccPath C:\tools\inno\ISCC.exe` for a private compiler installation.
The script checks the app's PE architecture, self-contained runtime files and
absence of mail databases, `.secrets` files and WebView profiles before compiling.
It calls `optimize-publish.ps1 -CheckOnly` to confirm the language policy and
absence of PDB symbols, without changing the publish directory. Run the optimizer
first if packaging reports that the publish is not optimized. The output includes
a SHA-256 file.

Setup UI languages are English, Simplified Chinese, Traditional Chinese,
Japanese, Spanish and French. English/Japanese/Spanish/French use the compiler's
bundled translations. Chinese translations are checked into `Languages/` from
the Inno Setup **is-6_7_0** tag, retaining their author attribution:

- https://github.com/jrsoftware/issrc/blob/is-6_7_0/Files/Languages/Unofficial/ChineseSimplified.isl
- https://github.com/jrsoftware/issrc/blob/is-6_7_0/Files/Languages/Unofficial/ChineseTraditional.isl

Traditional Chinese additionally includes the missing 6.7 message translations and updated placeholders; both Chinese key sets match the 6.7 default messages.
See `Languages/INNO-LICENSE.txt` for the upstream license. Setup translations do
not imply the app's own UI has been translated.

Both architectures use a stable product identity and install per user into
`%LOCALAPPDATA%\Programs\imyemail-cloud`, without administrator privileges. x64 means
x86-64 (Intel/AMD 64-bit), not 32-bit x86. Windows 10 version 1809 or newer is
required. The .NET and Windows App SDK runtimes are included by the application's
self-contained publish; WebView2 Evergreen Runtime must already be installed.
Setup checks Microsoft's documented HKCU/HKLM runtime registration and explains
how to obtain it if absent, without silently downloading anything.
Detection follows https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution
using client ID `{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` in both registry views.

Inno Setup's Restart Manager asks to close processes using application files
during interactive upgrades; it is never configured to force-close them. Exit
the app through its Settings before a silent upgrade. Desktop and Start menu
shortcuts point at the installed executable. Uninstall removes installed program
files and shortcuts, retaining `%LOCALAPPDATA%\imy.email` and saved credentials.
Remove accounts inside the app first if their local data should also be removed.
An upgrade replaces the application-owned `runtimes` dependency folder to avoid
retaining obsolete RID-specific files when switching architecture. It does not
delete any user-profile directory.

Local builds are unsigned development installers until a separate code-signing process
is configured. Validate installation, launch, upgrade and uninstall on Windows;
compilation alone is not an installation test.
