[Setup]
AppName="reSVG Explorer Extension"
AppVersion="0.11.0"
VersionInfoVersion="0.0.11.0"
AppVerName="reSVG Explorer Extension 0.11.0"
AppPublisher="Evgeniy Reizner"
AppPublisherURL=https://github.com/RazrFalcon/resvg
DefaultDirName="{pf}\reSVG Explorer Extension"
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
Source: "..\release\vc_redist.x64.exe"; DestDir: "{app}"; AfterInstall: InstallVcredist
Source: "..\..\..\target\release\server.dll"; DestDir: "{app}"; Flags: regserver
Source: "..\LICENSE.txt"; DestDir: "{app}";

[Code]
procedure InstallVcredist;
var
    ResultCode: Integer;
begin
    Exec(ExpandConstant('{app}\vc_redist.x64.exe'), '/install /passive /norestart', '', SW_SHOWNORMAL, ewWaitUntilTerminated, ResultCode)
end;
