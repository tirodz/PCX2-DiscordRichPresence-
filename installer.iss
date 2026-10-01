#define MyAppName "PCSX2 Discord Rich Presence"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "TIRO"
#define MyAppExeName "pcsx2-discord-rich-presence.exe"

[Setup]
AppId={{A4B4C6A2-5E90-4E74-9B7C-2F4E8A2B8F43}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\PCSX2 Discord Rich Presence
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputBaseFilename=PCSX2-DiscordRichPresence-Setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
Uninstallable=yes
UninstallDisplayIcon={app}\{#MyAppExeName}
SetupIconFile=assets\icon.ico

[Files]
Source: "target\release\pcsx2-discord-rich-presence.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--install"; Flags: runhidden nowait skipifsilent
Filename: "{app}\{#MyAppExeName}"; Description: "Set up PCSX2 Discord Rich Presence now"; Flags: postinstall skipifsilent

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--uninstall"; Flags: runhidden skipifdoesntexist

[UninstallDelete]
Type: files
Name: "{app}\config.toml"
Type: files
Name: "{app}\.stop"
Type: files
Name: "{app}\.pid"
Type: files
Name: "{app}\.pcsx2-discord-rpc-backup.toml"
Type: files
Name: "{app}\helper.log"
