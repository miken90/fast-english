; Fast English Inno Setup Installer
; Requires Inno Setup 6+ (https://jrsoftware.org/isinfo.php)
;
; The version is never written in this file. It is passed in by
; scripts\build-release.ps1 -Installer, which reads it from Cargo.toml, the
; single source of truth. A missing define is a hard error rather than a
; default, because a default here is exactly the drift this removes.

#ifndef AppVersion
  #error AppVersion is not defined. Build with: scripts\build-release.ps1 -Installer
#endif

[Setup]
AppName=Fast English
AppVersion={#AppVersion}
AppPublisher=Fast English
AppPublisherURL=https://github.com/miken90/fast-english
DefaultDirName={autopf}\FastEnglish
DefaultGroupName=Fast English
OutputDir=..\dist
OutputBaseFilename=FastEnglish-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=..\resources\app.ico
UninstallDisplayIcon={app}\FastEnglish.exe

[Files]
Source: "..\dist\FastEnglish.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Fast English"; Filename: "{app}\FastEnglish.exe"
Name: "{group}\Uninstall Fast English"; Filename: "{uninstallexe}"

[Tasks]
Name: autostart; Description: "Start Fast English when Windows starts"; GroupDescription: "Additional options:"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "FastEnglish"; ValueData: """{app}\FastEnglish.exe"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "{app}\FastEnglish.exe"; Description: "Launch Fast English"; Flags: postinstall nowait skipifsilent
