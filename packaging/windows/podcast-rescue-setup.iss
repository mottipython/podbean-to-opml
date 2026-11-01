; Inno Setup Script for Podcast Rescue
; Produces a fully featured Windows Installer (Setup.exe)

#ifndef AppVersion
#define AppVersion "0.4.1"
#endif

[Setup]
AppId={{E68C6A5E-4B02-4D3C-8A8E-6C48B012F5A1}
AppName=Podcast Rescue
AppVersion={#AppVersion}
AppVerName=Podcast Rescue {#AppVersion}
AppPublisher=Motti Schleider
AppPublisherURL=https://github.com/mottipython/podbean-to-opml
AppSupportURL=https://github.com/mottipython/podbean-to-opml/issues
AppUpdatesURL=https://github.com/mottipython/podbean-to-opml/releases
DefaultDirName={autopf}\Podcast Rescue
DefaultGroupName=Podcast Rescue
AllowNoIcons=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist\packages
OutputBaseFilename=podcast-rescue-windows-x86_64-setup
SetupIconFile=icon.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\icon.ico
UninstallDisplayName=Podcast Rescue
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addtopath"; Description: "Add Podcast Rescue to system PATH (accessible from Command Prompt / PowerShell)"; GroupDescription: "System Integration:"

[Files]
Source: "..\..\target\release\podcast-rescue.exe"; DestDir: "{app}"; Flags: ignoreversion; DestName: "podcast-rescue.exe"
Source: "..\..\target\release\podcast-rescue.exe"; DestDir: "{app}"; Flags: ignoreversion; DestName: "podbean-to-opml.exe"
Source: "icon.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion; DestName: "README.txt"
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion; DestName: "LICENSE.txt"

[Icons]
Name: "{group}\Podcast Rescue"; Filename: "{app}\podcast-rescue.exe"; IconFilename: "{app}\icon.ico"; Comment: "Universal Podcast Subscription Rescue & OPML Exporter"
Name: "{group}\Uninstall Podcast Rescue"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Podcast Rescue"; Filename: "{app}\podcast-rescue.exe"; IconFilename: "{app}\icon.ico"; Tasks: desktopicon

[Registry]
; Register App Path so Windows Run dialog (Win+R) can launch "podcast-rescue" directly
Root: HKLM; Subkey: "SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\podcast-rescue.exe"; ValueType: string; ValueName: ""; ValueData: "{app}\podcast-rescue.exe"; Flags: uninsdeletekey
Root: HKLM; Subkey: "SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\podcast-rescue.exe"; ValueType: string; ValueName: "Path"; ValueData: "{app}"; Flags: uninsdeletekey

; Add to user PATH if task selected
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\podcast-rescue.exe"; Description: "{cm:LaunchProgram,Podcast Rescue}"; Flags: nowait postinstall skipifsilent

[Code]
// Helper function to prevent duplicate PATH entries
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  // Check if already in PATH
  Result := Pos(';' + UpperCase(Param) + ';', ';' + UpperCase(OrigPath) + ';') = 0;
end;
