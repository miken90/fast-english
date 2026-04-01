; Fast English Inno Setup Installer
; Requires Inno Setup 6+ (https://jrsoftware.org/isinfo.php)

[Setup]
AppName=Fast English
AppVersion=0.3.1
AppPublisher=Fast English
AppPublisherURL=https://github.com/miken90/fast-english
DefaultDirName={autopf}\FastEnglish
DefaultGroupName=Fast English
OutputDir=..\dist
OutputBaseFilename=FastEnglish-Setup-0.3.1
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
