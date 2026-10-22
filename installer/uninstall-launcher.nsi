; A standalone, branded removal wizard. It never deletes note or settings folders.
Unicode true
ManifestDPIAware true
ManifestDPIAwareness PerMonitorV2
RequestExecutionLevel user
SetCompressor /SOLID lzma
!include MUI2.nsh
!include LogicLib.nsh
!include FileFunc.nsh
!include x64.nsh
!include "Win\RestartManager.nsh"

!ifndef APP_VERSION
  !define APP_VERSION "0.1.0"
!endif
!ifndef PB_PRODUCT_NAME
  !define PB_PRODUCT_NAME "Pinboard"
!endif
!ifndef OUTPUT_FILE
  !define OUTPUT_FILE "Pinboard_Uninstall.exe"
!endif
!define VERSION "${APP_VERSION}"
!define PRODUCTNAME "${PB_PRODUCT_NAME}"
!define REGKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define PINBOARD_INSTALLER_DIR "${__FILEDIR__}"
!define PINBOARD_STANDALONE
!include "${PINBOARD_INSTALLER_DIR}\branding.nsh"
!define MUI_ICON "${PINBOARD_INSTALLER_DIR}\..\src-tauri\icons\icon.ico"

Name "Pinboard"
Caption "$(pbRemoveTitle)"
BrandingText "Pinboard ${VERSION}"
OutFile "${OUTPUT_FILE}"
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Pinboard"
VIAddVersionKey "FileDescription" "Pinboard Uninstall"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" ""
VIAddVersionKey "OriginalFilename" "Pinboard_Uninstall.exe"

Var InstallFolder
Var InstalledVersion
Var InstalledUninstaller
Var MissingInstallation
Var RemovalExitCode
Var RequestedLanguage

Page custom RemovalConfirm RemovalPreflight
!define MUI_PAGE_CUSTOMFUNCTION_SHOW RemovalProgress
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE RemovalProgressLeave
!insertmacro MUI_PAGE_INSTFILES
Page custom RemovalFinish

!insertmacro MUI_LANGUAGE "Korean"
!insertmacro MUI_LANGUAGE "English"
!include "${PINBOARD_INSTALLER_DIR}\languages\Korean-ui.nsh"
!include "${PINBOARD_INSTALLER_DIR}\languages\English-ui.nsh"
!include "${PINBOARD_INSTALLER_DIR}\languages\ui-strings.nsh"

Function .onInit
  SetShellVarContext current
  ${GetOptions} $CMDLINE "/LANG=" $RequestedLanguage
  ${If} $RequestedLanguage == "1033"
  ${OrIf} $RequestedLanguage == "1042"
    StrCpy $LANGUAGE $RequestedLanguage
  ${EndIf}
  SetRegView 64
  Call ResolveInstallation
  ${If} $MissingInstallation = 1
    SetRegView 32
    Call ResolveInstallation
  ${EndIf}
  ${If} $MissingInstallation = 1
  ${AndIf} ${Silent}
    SetErrorLevel 2
    Abort
  ${EndIf}
FunctionEnd

Function RemovalConfirm
  ${If} $MissingInstallation = 1
    !insertmacro PB_BEGIN 0 "$(pbUiMissing)" "$(pbUiMissingText)"
    !insertmacro PB_LABEL 0 158 -1 100 text "$(pbUiPortable)"
    !insertmacro PB_BUTTON_TEXT "$(pbUiRemoveAction)"
    GetDlgItem $PbControl $HWNDPARENT 1
    EnableWindow $PbControl 0
    GetDlgItem $PbControl $HWNDPARENT 2
    SendMessage $PbControl ${WM_SETTEXT} 0 "STR:$(pbUiClose)"
  ${Else}
    !insertmacro PB_BEGIN 0 "$(pbUiRemove)" "$(pbUiRemoveIntro)"
    !insertmacro PB_LABEL 0 118 -1 26 small "$(pbUiInstalledVersion): $InstalledVersion"
    !insertmacro PB_LABEL 0 166 -1 26 label "$(pbUiPath)"
    !insertmacro PB_LABEL 0 204 -1 54 muted "$InstallFolder"
    !insertmacro PB_LABEL 0 278 -1 28 label "$(pbUiRemoveItems)"
    !insertmacro PB_LABEL 0 318 -1 62 muted "$(pbUiKeepDetail)"
    ${NSD_CreateLabel} 0 0 100% 12u ""
    Pop $PbError
    PinboardUI::Place /NOUNLOAD $PbError 0 390 -1 54 error
    ShowWindow $PbError ${SW_HIDE}
    !insertmacro PB_BUTTON_TEXT "$(pbUiRemoveAction)"
  ${EndIf}
  nsDialogs::Show
FunctionEnd

Function RemovalProgress
  PinboardUI::Progress /NOUNLOAD "$(pbUiRemoveProgress)" "$(pbUiRemoving)"
  PinboardUI::Marquee /NOUNLOAD
FunctionEnd

Function RemovalFinish
  !insertmacro PB_BEGIN 2 "$(pbUiRemoveDone)" "$(pbUiRemoveDoneText)"
  !insertmacro PB_LABEL 0 152 -1 96 text "$(pbUiKeepDetail)"
  !insertmacro PB_BUTTON_TEXT "$(pbUiClose)"
  GetDlgItem $PbControl $HWNDPARENT 2
  ShowWindow $PbControl ${SW_HIDE}
  nsDialogs::Show
FunctionEnd

Function .onInstFailed
  PinboardUI::Failure /NOUNLOAD "$(pbUiFailed)" "$(pbUiFailedHelp)"
FunctionEnd

Function RemovalProgressLeave
  IfAbort 0 done
  PinboardUI::Failure /NOUNLOAD "$(pbUiFailed)" "$(pbUiFailedHelp)"
  done:
FunctionEnd

Function ResolveInstallation
  StrCpy $MissingInstallation 1
  ReadRegStr $0 HKCU "${REGKEY}" "DisplayName"
  StrCmp $0 "${PRODUCTNAME}" 0 resolve_done
  ReadRegStr $InstallFolder HKCU "${REGKEY}" "InstallLocation"
  StrCmp $InstallFolder "" resolve_done
  ; Tauri stores InstallLocation with quotes. Never execute the registry string.
  StrCpy $0 $InstallFolder 1
  StrCmp $0 '$\"' 0 canonicalize
  StrCpy $InstallFolder $InstallFolder -1 1
  canonicalize:
  GetFullPathName $InstallFolder "$InstallFolder"
  ${GetRoot} "$InstallFolder" $0
  StrCmp $InstallFolder "$0\" resolve_done
  StrCmp $InstallFolder "$0" resolve_done
  IfFileExists "$InstallFolder\pinboard.exe" 0 resolve_done
  StrCpy $InstalledUninstaller "$InstallFolder\uninstall.exe"
  IfFileExists "$InstalledUninstaller" 0 resolve_done
  ReadRegStr $0 HKCU "${REGKEY}" "UninstallString"
  StrCmp $0 '$\"$InstalledUninstaller$\"' valid_registration
  StrCmp $0 "$InstalledUninstaller" 0 resolve_done
  valid_registration:
  ReadRegStr $InstalledVersion HKCU "${REGKEY}" "DisplayVersion"
  StrCpy $MissingInstallation 0
  resolve_done:
  ClearErrors
FunctionEnd

Function RemovalPreflight
  ${If} $MissingInstallation = 1
    Abort
  ${EndIf}
  ; Query the exact installed binary, leaving portable/other instances alone.
  !insertmacro RestartManager_StartSession $R0
  StrCmp $R0 "" preflight_failed
  !insertmacro RestartManager_RegisterFile $R0 "$InstallFolder\pinboard.exe"
  ${If} $0 != 0
    !insertmacro RestartManager_EndSession $R0
    Goto preflight_failed
  ${EndIf}
  StrCpy $1 0
  StrCpy $2 0
  StrCpy $3 0
  System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
  !insertmacro RestartManager_EndSession $R0
  ${If} $0 = 234
    IfSilent running_silent
    ${NSD_SetText} $PbError "$(pbUiRunning)"
    ShowWindow $PbError ${SW_SHOW}
    Abort
    running_silent:
    SetErrorLevel 3
    Abort
  ${ElseIf} $0 != 0
    Goto preflight_failed
  ${EndIf}
  Return
  preflight_failed:
  IfSilent preflight_failed_silent
  ${NSD_SetText} $PbError "$(pbUiCheckFailed)"
  ShowWindow $PbError ${SW_SHOW}
  Abort
  preflight_failed_silent:
  SetErrorLevel 3
  Abort
FunctionEnd

Section "RemovePinboard"
  Call RemovalPreflight
  DetailPrint "$(pbRemoveDetail)"
  ClearErrors
  ; _?= keeps the registered uninstaller in place so ExecWait covers real completion.
  ; /S leaves its delete-data checkbox unchecked, including older Pinboard installs.
  ExecWait '$\"$InstalledUninstaller$\" /S _?=$InstallFolder' $RemovalExitCode
  IfErrors removal_failed
  ${If} $RemovalExitCode != 0
  ${AndIf} $RemovalExitCode != 3010
    Goto removal_failed
  ${EndIf}
  IfFileExists "$InstallFolder\pinboard.exe" removal_failed
  ReadRegStr $0 HKCU "${REGKEY}" "UninstallString"
  StrCmp $0 "" 0 removal_failed
  ; The in-place uninstaller cannot delete itself. Clean only its known file.
  Delete "$InstalledUninstaller"
  RMDir "$InstallFolder"
  ClearErrors
  DetailPrint "$(pbRemovePreserved)"
  SetErrorLevel 0
  Goto removal_done
  removal_failed:
  SetErrorLevel 1
  Abort "$(pbRemoveFailed)"
  removal_done:
SectionEnd
