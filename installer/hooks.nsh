; Supported Tauri NSIS hooks: keep upstream installation/update logic intact.
!define PINBOARD_WITH_UNINSTALLER
!define PINBOARD_INSTALLER_DIR "${__FILEDIR__}"
!include "${PINBOARD_INSTALLER_DIR}\branding.nsh"

!macro NSIS_HOOK_POSTINSTALL
  ; Give installed users a direct Start menu removal entry as well.
  CreateShortcut "$SMPROGRAMS\$(pbUninstallShortcut).lnk" "$INSTDIR\uninstall.exe"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Check the link target so an unrelated user-created shortcut is preserved.
  !insertmacro IsShortcutTarget "$SMPROGRAMS\$(pbUninstallShortcut).lnk" "$INSTDIR\uninstall.exe"
  Pop $0
  ${If} $0 = 1
    Delete "$SMPROGRAMS\$(pbUninstallShortcut).lnk"
  ${EndIf}
!macroend
