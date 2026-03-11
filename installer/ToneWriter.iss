; ToneWriter Inno Setup Installer
; Requires Inno Setup 6+ (https://jrsoftware.org/isinfo.php)

[Setup]
AppName=ToneWriter
AppVersion=1.0.0
AppPublisher=ToneWriter
AppPublisherURL=https://github.com/canhnguyen/tone-writer
DefaultDirName={autopf}\ToneWriter
DefaultGroupName=ToneWriter
OutputDir=..\dist
OutputBaseFilename=ToneWriter-Setup-1.0.0
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=..\resources\app.ico
UninstallDisplayIcon={app}\ToneWriter.exe

[Files]
Source: "..\dist\ToneWriter.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\ToneWriter"; Filename: "{app}\ToneWriter.exe"
Name: "{group}\Uninstall ToneWriter"; Filename: "{uninstallexe}"

[Tasks]
Name: autostart; Description: "Start ToneWriter when Windows starts"; GroupDescription: "Additional options:"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ToneWriter"; ValueData: """{app}\ToneWriter.exe"""; Tasks: autostart; Flags: uninsdeletevalue

[Run]
Filename: "{app}\ToneWriter.exe"; Description: "Launch ToneWriter"; Flags: postinstall nowait skipifsilent
