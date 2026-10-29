Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
Name "WebDrop Image Optimizer"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\WebDrop"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetDatablockOptimize on
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "WebDrop Image Optimizer"
VIAddVersionKey "FileDescription" "WebDrop Setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "Jędrzej Czarnecki"
!define MUI_ABORTWARNING
!define MUI_ICON "${PAYLOAD}/WebDrop.ico"
!define MUI_UNICON "${PAYLOAD}/WebDrop.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\WebDrop.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Polish"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "WebDrop requires 64-bit Windows 10 or newer."
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_ICONSTOP "WebDrop requires Windows 10 or newer."
    Abort
  ${EndIf}
  SetShellVarContext current
FunctionEnd

Section "WebDrop"
  SetOutPath "$INSTDIR"
  File /r "${PAYLOAD}/*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateShortcut "$SMPROGRAMS\WebDrop.lnk" "$INSTDIR\WebDrop.exe" "" "$INSTDIR\WebDrop.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "DisplayName" "WebDrop Image Optimizer"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "Publisher" "Jędrzej Czarnecki"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "DisplayIcon" "$INSTDIR\WebDrop.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$SMPROGRAMS\WebDrop.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\WebDrop"
  ; Delete only packaged files. Preserve user images and unknown files, even in /D overrides.
  !include "${REMOVE_FILES}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
