#define MyAppName "TBX Translator"
#define MyAppVersion "0.0.1-alpha"
#define MyAppPublisher "samwns"
#define MyAppExeName "TBX-Translator.exe"

[Setup]
AppId={{D9B5B82C-9E89-43BF-881D-43D90E5EE250}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
UninstallDisplayIcon={app}\assets\com.tbx.translator.ico
DefaultDirName={autopf}\TBX Translator
DefaultGroupName=TBX Translator
OutputDir=..\..\release
OutputBaseFilename=TBX-Translator-Setup
SetupIconFile=..\..\assets\com.tbx.translator.ico
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64

[Files]
Source: "..\..\release\TBX-Translator-Windows-x64\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{autoprograms}\TBX Translator"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\com.tbx.translator.ico"
Name: "{autodesktop}\TBX Translator"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\assets\com.tbx.translator.ico"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch TBX Translator"; Flags: nowait postinstall skipifsilent
