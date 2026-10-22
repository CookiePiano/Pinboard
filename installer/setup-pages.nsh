; Product pages; all file/registry operations remain in the installer template.
Var PbHasExisting
Var PbVersionCompare
Var PbFirstText
Var PbSecondText
Var PbWixKey
Var PbMaintenanceChoice
Var PbMaintenanceFirst
Var PbMaintenanceSecond
Var PbOnlyRemoval
Var PbInteractiveReady
Var PbY
Var PbErrorY
Var PbPreviousVersion
Var PbBrowseButton
!include "${PINBOARD_INSTALLER_DIR}\path-validation.nsh"

Page custom PbSetupPage PbSetupLeave
; Preserve Tauri's Start menu storage machinery (no visible page with current config).
Var AppStartMenuFolder
!if "${STARTMENUFOLDER}" != ""
  !define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
  !define MUI_STARTMENUPAGE_DEFAULTFOLDER "${STARTMENUFOLDER}"
!else
  !define MUI_PAGE_CUSTOMFUNCTION_PRE Skip
!endif
!insertmacro MUI_PAGE_STARTMENU Application $AppStartMenuFolder
!define MUI_PAGE_CUSTOMFUNCTION_SHOW PbInstallProgress
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE PbProgressLeave
!insertmacro MUI_PAGE_INSTFILES
Page custom PbSetupFinish PbSetupFinishLeave

Var DeleteAppDataCheckbox
Var DeleteAppDataCheckboxState
UninstPage custom un.PbConfirm un.PbConfirmLeave
!define MUI_PAGE_CUSTOMFUNCTION_SHOW un.PbProgress
!define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.PbProgressLeave
!insertmacro MUI_UNPAGE_INSTFILES
UninstPage custom un.PbFinish

Function PbSetupPage
  Call PbDetectPrevious
  StrCpy $PbInteractiveReady 1
  StrCpy $PbMaintenanceChoice 1
  StrCpy $PbOnlyRemoval 0
  Call SkipIfPassive
  ${If} $PbHasExisting = 1
    !insertmacro PB_BEGIN 0 "$(pbUiExisting)" "$(pbUiManageIntro)"
  ${Else}
    !insertmacro PB_BEGIN 0 "$(pbUiSetup)" "$(pbUiIntro)"
  ${EndIf}
  StrCpy $PbY 152
  ${If} $PbHasExisting = 1
    ReadRegStr $PbPreviousVersion SHCTX "${UNINSTKEY}" "DisplayVersion"
    !insertmacro PB_LABEL 0 110 -1 24 small "$(pbUiInstalledVersion): $PbPreviousVersion"
    ${NSD_CreateRadioButton} 0 0 100% 12u "$PbFirstText"
    Pop $PbMaintenanceFirst
    PinboardUI::Place /NOUNLOAD $PbMaintenanceFirst 0 146 -1 28 radio
    ${NSD_CreateRadioButton} 0 0 100% 12u "$PbSecondText"
    Pop $PbMaintenanceSecond
    PinboardUI::Place /NOUNLOAD $PbMaintenanceSecond 0 180 -1 28 radio
    ${NSD_Check} $PbMaintenanceFirst
    ${NSD_OnClick} $PbMaintenanceFirst PbMaintenanceChanged
    ${NSD_OnClick} $PbMaintenanceSecond PbMaintenanceChanged
    StrCpy $PbY 226
  ${Else}
    !insertmacro PB_LABEL 0 110 -1 24 small "$(pbUiMetadata)"
  ${EndIf}
  !insertmacro PB_LABEL 0 $PbY -1 24 label "$(pbUiPath)"
  IntOp $PbY $PbY + 32
  ${NSD_CreateText} 0 0 100% 12u "$INSTDIR"
  Pop $PbPath
  PinboardUI::Place /NOUNLOAD $PbPath 0 $PbY -126 40 edit
  ${NSD_CreateButton} 0 0 100% 12u "$(pbUiBrowse)"
  Pop $PbBrowseButton
  PinboardUI::Place /NOUNLOAD $PbBrowseButton -114 $PbY 114 40 button
  ${NSD_OnClick} $PbBrowseButton PbBrowse
  IntOp $PbY $PbY + 60
  ${NSD_CreateCheckbox} 0 0 100% 12u "$(pbUiShortcut)"
  Pop $PbShortcut
  PinboardUI::Place /NOUNLOAD $PbShortcut 0 $PbY -1 28 check
  ${NSD_SetState} $PbShortcut $PbShortcutState
  IntOp $PbY $PbY + 36
  ${NSD_CreateCheckbox} 0 0 100% 12u "$(pbUiLaunch)"
  Pop $PbLaunch
  PinboardUI::Place /NOUNLOAD $PbLaunch 0 $PbY -1 28 check
  ${NSD_SetState} $PbLaunch $PbLaunchState
  IntOp $PbY $PbY + 48
  !insertmacro PB_LABEL 0 $PbY -1 28 muted "$(pbUiKeep)"
  IntOp $PbErrorY $PbY + 42
  ${NSD_CreateLabel} 0 0 100% 12u ""
  Pop $PbError
  PinboardUI::Place /NOUNLOAD $PbError 0 $PbErrorY -1 52 error
  ShowWindow $PbError ${SW_HIDE}
  Call PbMaintenanceChanged
  nsDialogs::Show
FunctionEnd

Function PbMaintenanceChanged
  !insertmacro PB_BUTTON_TEXT "$(pbUiInstall)"
  StrCpy $PbOnlyRemoval 0
  ${If} $PbHasExisting = 1
    !insertmacro PB_BUTTON_TEXT "$(pbUiReinstall)"
    ${NSD_GetState} $PbMaintenanceFirst $PbMaintenanceChoice
    ${If} $PbVersionCompare = 0
    ${AndIf} $PbMaintenanceChoice = 0
      StrCpy $PbOnlyRemoval 1
      !insertmacro PB_BUTTON_TEXT "$(pbUiRemoveAction)"
    ${EndIf}
  ${EndIf}
  ${If} $PbOnlyRemoval = 1
    EnableWindow $PbPath 0
    EnableWindow $PbBrowseButton 0
    EnableWindow $PbShortcut 0
    EnableWindow $PbLaunch 0
  ${Else}
    EnableWindow $PbPath 1
    EnableWindow $PbBrowseButton 1
    EnableWindow $PbShortcut 1
    EnableWindow $PbLaunch 1
  ${EndIf}
FunctionEnd

Function PbBrowse
  nsDialogs::SelectFolderDialog "$(pbUiPath)" "$INSTDIR"
  Pop $PbControl
  ${If} $PbControl != error
    ${NSD_SetText} $PbPath "$PbControl"
  ${EndIf}
FunctionEnd

Function PbSetupLeave
  ${NSD_GetText} $PbPath $0
  ${If} $PbOnlyRemoval <> 1
    Call PbValidatePath
    StrCmp $PbPathError invalid invalid_path
    StrCmp $PbPathError space insufficient_space
    StrCpy $INSTDIR $5
    Goto enough_space
    insufficient_space:
      ${NSD_SetText} $PbError "$(pbUiSpace)"
      ShowWindow $PbError ${SW_SHOW}
      PinboardUI::Reveal /NOUNLOAD $PbError
      Abort
    enough_space:
  ${EndIf}
  ${NSD_GetState} $PbShortcut $PbShortcutState
  ${NSD_GetState} $PbLaunch $PbLaunchState
  Return
  invalid_path:
    ${NSD_SetText} $PbError "$(pbUiInvalidPath)"
    ShowWindow $PbError ${SW_SHOW}
    PinboardUI::Reveal /NOUNLOAD $PbError
    Abort
FunctionEnd

Function PbInstallProgress
  ${If} $PbOnlyRemoval = 1
    PinboardUI::Mode /NOUNLOAD remove
    PinboardUI::Progress /NOUNLOAD "$(pbUiRemoveProgress)" "$(pbUiRemoving)"
  ${Else}
    PinboardUI::Progress /NOUNLOAD "$(pbUiInstallProgress)" "$(pbUiWorking)"
  ${EndIf}
FunctionEnd

Function PbSetupFinish
  Call SkipIfPassive
  ${If} $PbOnlyRemoval = 1
    !insertmacro PB_BEGIN 2 "$(pbUiRemoveDone)" "$(pbUiRemoveDoneText)"
    !insertmacro PB_LABEL 0 150 -1 72 text "$(pbUiKeepDetail)"
  ${Else}
    !insertmacro PB_BEGIN 2 "$(pbUiSetupDone)" "$(pbUiSetupDoneText)"
    !insertmacro PB_LABEL 0 142 -1 28 label "$(pbUiPath)"
    !insertmacro PB_LABEL 0 180 -1 58 muted "$INSTDIR"
    !insertmacro PB_LABEL 0 274 -1 50 text "$(pbUiKeep)"
  ${EndIf}
  !insertmacro PB_BUTTON_TEXT "$(pbUiDone)"
  GetDlgItem $PbControl $HWNDPARENT 2
  ShowWindow $PbControl ${SW_HIDE}
  nsDialogs::Show
FunctionEnd

Function PbSetupFinishLeave
  ${If} $PbOnlyRemoval <> 1
  ${AndIf} $PbLaunchState = 1
    Call RunMainBinary
  ${EndIf}
FunctionEnd

Function RunMainBinary
  nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
FunctionEnd

Function .onInstFailed
  PinboardUI::Failure /NOUNLOAD "$(pbUiFailed)" "$(pbUiFailedHelp)"
FunctionEnd

Function PbProgressLeave
  IfAbort 0 done
  PinboardUI::Failure /NOUNLOAD "$(pbUiFailed)" "$(pbUiFailedHelp)"
  done:
FunctionEnd

Function un.PbProgressLeave
  IfAbort 0 done
  PinboardUI::Failure /NOUNLOAD "$(pbUiFailed)" "$(pbUiFailedHelp)"
  done:
FunctionEnd

Function un.PbConfirm
  Call un.SkipIfPassive
  !insertmacro PB_BEGIN 0 "$(pbUiRemove)" "$(pbUiRemoveIntro)"
  !insertmacro PB_LABEL 0 118 -1 28 small "$(pbUiMetadata)"
  !insertmacro PB_LABEL 0 162 -1 26 label "$(pbUiPath)"
  !insertmacro PB_LABEL 0 198 -1 52 muted "$INSTDIR"
  !insertmacro PB_LABEL 0 264 -1 28 text "$(pbUiRemoveItems)"
  !insertmacro PB_LABEL 0 306 -1 50 muted "$(pbUiDataWarning)"
  ${NSD_CreateCheckbox} 0 0 100% 12u "$(deleteAppData)"
  Pop $DeleteAppDataCheckbox
  PinboardUI::Place /NOUNLOAD $DeleteAppDataCheckbox 0 368 -1 30 check
  !insertmacro PB_BUTTON_TEXT "$(pbUiRemoveAction)"
  nsDialogs::Show
FunctionEnd

Function un.PbConfirmLeave
  ${NSD_GetState} $DeleteAppDataCheckbox $DeleteAppDataCheckboxState
FunctionEnd

Function un.PbProgress
  PinboardUI::Progress /NOUNLOAD "$(pbUiRemoveProgress)" "$(pbUiRemoving)"
FunctionEnd

Function un.PbFinish
  Call un.SkipIfPassive
  ${If} $UpdateMode = 1
    Abort
  ${EndIf}
  !insertmacro PB_BEGIN 2 "$(pbUiRemoveDone)" "$(pbUiRemoveDoneText)"
  ${If} $DeleteAppDataCheckboxState = 1
    !insertmacro PB_LABEL 0 150 -1 86 text "$(pbUiDataDeleted)"
  ${Else}
    !insertmacro PB_LABEL 0 150 -1 86 text "$(pbUiKeepDetail)"
  ${EndIf}
  !insertmacro PB_BUTTON_TEXT "$(pbUiClose)"
  GetDlgItem $PbControl $HWNDPARENT 2
  ShowWindow $PbControl ${SW_HIDE}
  nsDialogs::Show
FunctionEnd

Function un.onUninstFailed
  PinboardUI::Failure /NOUNLOAD "$(pbUiFailed)" "$(pbUiFailedHelp)"
FunctionEnd

Function PbDetectPrevious
  StrCpy $PbHasExisting 0
  StrCpy $WixMode 0
  ; Uninstall previous WiX installation if exists.
  ;
  ; A WiX installer stores the installation info in registry
  ; using a UUID and so we have to loop through all keys under
  ; `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`
  ; and check if `DisplayName` and `Publisher` keys match ${PRODUCTNAME} and ${MANUFACTURER}
  ;
  ; This has a potential issue that there maybe another installation that matches
  ; our ${PRODUCTNAME} and ${MANUFACTURER} but wasn't installed by our WiX installer,
  ; however, this should be fine since the user will have to confirm the uninstallation
  ; and they can chose to abort it if doesn't make sense.
  StrCpy $0 0
  wix_loop:
    EnumRegKey $1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" $0
    StrCmp $1 "" wix_loop_done ; Exit loop if there is no more keys to loop on
    IntOp $0 $0 + 1
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "DisplayName"
    ReadRegStr $R1 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "Publisher"
    StrCmp "$R0$R1" "${PRODUCTNAME}${MANUFACTURER}" 0 wix_loop
    ReadRegStr $R0 HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1" "UninstallString"
    ${StrCase} $R1 $R0 "L"
    ${StrLoc} $R0 $R1 "msiexec" ">"
    StrCmp $R0 0 0 wix_loop_done
    StrCpy $WixMode 1
    StrCpy $R6 "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\$1"
    Goto compare_version
  wix_loop_done:

  ; Check if there is an existing installation, if not, abort the reinstall page
  ReadRegStr $R0 SHCTX "${UNINSTKEY}" ""
  ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
  ${IfThen} "$R0$R1" == "" ${|} Return ${|}

  ; Compare this installar version with the existing installation
  ; and modify the messages presented to the user accordingly
  compare_version:
  StrCpy $R4 "$(older)"
  ${If} $WixMode = 1
    ReadRegStr $R0 HKLM "$R6" "DisplayVersion"
  ${Else}
    ReadRegStr $R0 SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${EndIf}
  ${IfThen} $R0 == "" ${|} StrCpy $R4 "$(unknown)" ${|}

  nsis_tauri_utils::SemverCompare "${VERSION}" $R0
  Pop $R0
  ; Reinstalling the same version
  ${If} $R0 = 0
    StrCpy $R1 "$(alreadyInstalledLong)"
    StrCpy $R2 "$(addOrReinstall)"
    StrCpy $R3 "$(uninstallApp)"

  ; Upgrading
  ${ElseIf} $R0 = 1
    StrCpy $R1 "$(olderOrUnknownVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    StrCpy $R3 "$(dontUninstall)"

  ; Downgrading
  ${ElseIf} $R0 = -1
    StrCpy $R1 "$(newerVersionInstalled)"
    StrCpy $R2 "$(uninstallBeforeInstalling)"
    !if "${ALLOWDOWNGRADES}" == "true"
      StrCpy $R3 "$(dontUninstall)"
    !else
      StrCpy $R3 "$(dontUninstallDowngrade)"
    !endif

  ${Else}
    Return
  ${EndIf}

  StrCpy $PbHasExisting 1
  StrCpy $PbVersionCompare $R0
  StrCpy $PbFirstText $R2
  StrCpy $PbSecondText $R3
  StrCpy $PbWixKey $R6
FunctionEnd

Function PageLeaveReinstall
  StrCpy $R0 $PbVersionCompare
  StrCpy $R1 $PbMaintenanceChoice
  StrCpy $R6 $PbWixKey

  ; If migrating from Wix, always uninstall
  ${If} $WixMode = 1
    Goto reinst_uninstall
  ${EndIf}

  ; In update mode, always proceeds without uninstalling
  ${If} $UpdateMode = 1
    Goto reinst_done
  ${EndIf}

  ; $R0 holds whether same(0)/upgrading(1)/downgrading(-1) version
  ; $R1 holds the radio buttons state:
  ;   1 => first choice was selected
  ;   0 => second choice was selected
  ${If} $R0 = 0 ; Same version, proceed
    ${If} $R1 = 1              ; User chose to add/reinstall
      Goto reinst_done
    ${Else}                    ; User chose to uninstall
      Goto reinst_uninstall
    ${EndIf}
  ${ElseIf} $R0 = 1 ; Upgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${ElseIf} $R0 = -1 ; Downgrading
    ${If} $R1 = 1              ; User chose to uninstall
      Goto reinst_uninstall
    ${Else}
      Goto reinst_done         ; User chose NOT to uninstall
    ${EndIf}
  ${EndIf}

  reinst_uninstall:
    ClearErrors

    ${If} $WixMode = 1
      ReadRegStr $R1 HKLM "$R6" "UninstallString"
      ExecWait '$R1' $0
    ${Else}
      ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
      ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
      StrCpy $R1 "$R1 /S"
      ${IfThen} $UpdateMode = 1 ${|} StrCpy $R1 "$R1 /UPDATE" ${|} ; append /UPDATE
      ${IfThen} $PassiveMode = 1 ${|} StrCpy $R1 "$R1 /P" ${|} ; append /P
      StrCpy $R1 "$R1 _?=$4" ; append uninstall directory
      ExecWait '$R1' $0
    ${EndIf}


    ${IfThen} ${Errors} ${|} StrCpy $0 2 ${|} ; ExecWait failed, set fake exit code

    ${If} $0 <> 0
    ${OrIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
      ; User cancelled wix uninstaller? return to select un/reinstall page
      ${If} $WixMode = 1
      ${AndIf} $0 = 1602
        Abort
      ${EndIf}

      ; User cancelled NSIS uninstaller? return to select un/reinstall page
      ${If} $0 = 1
        Abort
      ${EndIf}

      ; Other errors? show generic error message and return to select un/reinstall page
      DetailPrint "$(unableToUninstall) ($0)"
      SetErrorLevel 1
      Abort
    ${EndIf}

    ; The NSIS uninstaller was run in place (`_?=`), so it cannot delete itself
    ; and the install directory, clean them up here once the uninstall succeeded
    ; https://nsis.sourceforge.io/Docs/Chapter3.html#:~:text=path%20contains%20spaces.-,3.2.3%20Examples,-installer.exe%20/NCRC
    ${If} $WixMode = 0
    ${AndIf} $4 != ""
      Delete "$4\uninstall.exe"
      RMDir "$4"
    ${EndIf}
  reinst_done:
FunctionEnd
