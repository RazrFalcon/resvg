[Setup]
AppName="resvg Explorer Extension"
AppVersion="0.43.0"
VersionInfoVersion="0.0.43.0"
AppVerName="resvg Explorer Extension 0.43.0"
AppPublisher="Yevhenii Reizner"
AppPublisherURL=https://github.com/RazrFalcon/resvg
DefaultDirName="{pf}\resvg Explorer Extension"
Compression=lzma
SolidCompression=yes
ChangesAssociations=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputBaseFilename="resvg-explorer-extension"
OutputDir=.

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"; LicenseFile: "..\LICENSE.txt"

[Files]
Source: "..\target\release\server.dll"; DestDir: "{app}"
Source: "..\LICENSE.txt"; DestDir: "{app}";

[Registry]
Root: HKCR; Subkey: "CLSID\{{4432C229-DFD0-4B18-8C4D-F58932AF6105}"; Flags: uninsdeletekey
Root: HKCR; Subkey: "CLSID\{{4432C229-DFD0-4B18-8C4D-F58932AF6105}"; ValueType: string; ValueData: "ThumbnailProvider"
Root: HKCR; Subkey: "CLSID\{{4432C229-DFD0-4B18-8C4D-F58932AF6105}\InprocServer32"; ValueType: string; ValueData: "{app}\server.dll"
Root: HKCR; Subkey: ".svg\shellex"; Flags: uninsdeletekeyifempty
Root: HKCR; Subkey: ".svg\shellex\{{E357FCCD-A995-4576-B01F-234630154E96}"; Flags: uninsdeletekey
Root: HKCR; Subkey: ".svg\shellex\{{E357FCCD-A995-4576-B01F-234630154E96}"; ValueType: string; ValueData: "{{4432C229-DFD0-4B18-8C4D-F58932AF6105}"
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Services\EventLog\Application\resvg Thumbnailer"; Flags: uninsdeletekey
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Services\EventLog\Application\resvg Thumbnailer"; ValueType: string; ValueName: "EventMessageFile"; ValueData: "{app}\server.dll"
