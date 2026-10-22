; Shared native presentation. Installation/removal remain owned by NSIS.
!ifndef PINBOARD_BRANDING_INCLUDED
!define PINBOARD_BRANDING_INCLUDED
!include LogicLib.nsh
!include nsDialogs.nsh
!define PB_UI_PLUGIN_ENV "$%PINBOARD_UI_PLUGIN_DIR%"
!if "${PB_UI_PLUGIN_ENV}" != ""
  !addplugindir "${PB_UI_PLUGIN_ENV}"
!else
  !addplugindir "${PINBOARD_INSTALLER_DIR}\build"
!endif
!define MUI_BGCOLOR "15191E"
!define MUI_TEXTCOLOR "F4F7FA"
!define MUI_INSTFILESPAGE_COLORS "F4F7FA 15191E"
!define MUI_INSTFILESPAGE_PROGRESSBAR "smooth"
!define MUI_CUSTOMFUNCTION_GUIINIT PinboardGuiInit
!ifdef PINBOARD_WITH_UNINSTALLER
  !define MUI_CUSTOMFUNCTION_UNGUIINIT un.PinboardGuiInit
!endif
Var PbDialog
Var PbControl
Var PbError
!ifndef PINBOARD_STANDALONE
Var PbPath
Var PbShortcut
Var PbLaunch
Var PbShortcutState
Var PbLaunchState
Var PbRequestedLanguage
!endif
!macro PB_LABEL X Y W H ROLE TEXT
  ${NSD_CreateLabel} 0 0 100% 12u "${TEXT}"
  Pop $PbControl
  PinboardUI::Place /NOUNLOAD $PbControl ${X} ${Y} ${W} ${H} "${ROLE}"
!macroend
!macro PB_BUTTON_TEXT TEXT
  GetDlgItem $PbControl $HWNDPARENT 1
  SendMessage $PbControl ${WM_SETTEXT} 0 "STR:${TEXT}"
!macroend
!macro PB_BEGIN STAGE TITLE SUBTITLE
  nsDialogs::Create 1018
  Pop $PbDialog
  ${If} $PbDialog == error
    Abort
  ${EndIf}
  PinboardUI::Begin /NOUNLOAD $PbDialog ${STAGE}
  !insertmacro PB_LABEL 0 0 -1 52 heading "${TITLE}"
  !insertmacro PB_LABEL 0 60 -1 44 muted "${SUBTITLE}"
!macroend
!macro PB_GUIINIT PREFIX MODE
Function ${PREFIX}PinboardGuiInit
  InitPluginsDir
  File "/oname=$PLUGINSDIR\pinboard.ico" "${PINBOARD_INSTALLER_DIR}\..\src-tauri\icons\icon.ico"
  PinboardUI::Init /NOUNLOAD "${MODE}" "$(pbUiVersion)" "$PLUGINSDIR\pinboard.ico" "$LANGUAGE"
  SetAutoClose true
FunctionEnd
!macroend
!ifdef PINBOARD_STANDALONE
  !insertmacro PB_GUIINIT "" remove
!else
  !insertmacro PB_GUIINIT "" install
!endif
!ifdef PINBOARD_WITH_UNINSTALLER
  !insertmacro PB_GUIINIT un. remove
!endif
!endif
