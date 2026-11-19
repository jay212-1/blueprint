#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{152A3A39-AF82-47BD-A7FF-4376CC77D835}
AppName=Blueprint
AppVersion={#AppVersion}
AppPublisher=Cheter
DefaultDirName={autopf}\Showtime Suite\Blueprint
DefaultGroupName=Blueprint
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename=Blueprint-Setup-{#AppVersion}
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\blueprint.exe
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\..\stage\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{autoprograms}\Blueprint"; Filename: "{app}\blueprint.exe"
Name: "{autodesktop}\Blueprint"; Filename: "{app}\blueprint.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\blueprint.exe"; Description: "{cm:LaunchProgram,Blueprint}"; Flags: nowait postinstall
