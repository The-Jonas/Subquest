; Extra steps for the Windows installer (see bundle.windows.nsis.installerHooks).

; Subquest keeps its settings, game profiles and fonts in %APPDATA%\Subquest,
; not in the folder named after the identifier that the uninstaller already
; clears. Deletes it too when the user ticks "Delete the application data",
; and never during an update.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    RmDir /r "$APPDATA\Subquest"
  ${EndIf}
!macroend
