; Pako Setup: run by Tauri's NSIS installer at fixed points (tauri.conf.json:
; bundle.windows.nsis.installerHooks). This file is included near the top of Tauri's script, before
; its own definitions: code outside the macros can't use ${PRODUCTNAME} and friends (so "Pako" is
; written out), the macros can (they are expanded where Tauri inserts them).

; ---------- where Pako is installed ----------
; Tauri's per-user default is %LOCALAPPDATA%\Pako, which is where Pako keeps its settings and lists.
; Pako goes to %LOCALAPPDATA%\Programs\Pako instead (where Windows puts per-user programs), unless
; an earlier install is elsewhere (Tauri remembers it) or the folder was chosen with /D=.
!define MUI_CUSTOMFUNCTION_GUIINIT PakoInstallDir

Function PakoInstallDir
  Push $0
  ${If} $INSTDIR == "$LOCALAPPDATA\Pako"
    ReadRegStr $0 HKCU "Software\Sookera\Pako" ""
    ${If} $0 == ""
      StrCpy $INSTDIR "$LOCALAPPDATA\Programs\Pako"
    ${EndIf}
  ${EndIf}
  Pop $0
FunctionEnd

; The folder page refuses (Next stays greyed) a folder Pako keeps data in: %LOCALAPPDATA%\Pako,
; where the installed Pako keeps its settings and lists (unless an earlier install is there
; already), and a portable Pako's folder (it has its own data folder: installing there would
; switch that copy to %LOCALAPPDATA%\Pako, and its lists would look gone).
Function .onVerifyInstDir
  Push $0
  ${If} $INSTDIR == "$LOCALAPPDATA\Pako"
    ReadRegStr $0 HKCU "Software\Sookera\Pako" ""
    ${If} $0 != $INSTDIR
      Pop $0
      Abort
    ${EndIf}
  ${EndIf}
  Pop $0
  ${IfNot} ${FileExists} "$INSTDIR\.pako-installed"
    ${If} ${FileExists} "$INSTDIR\data\settings.json"
    ${OrIf} ${FileExists} "$INSTDIR\data\downloads.json"
      Abort
    ${EndIf}
  ${EndIf}
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  ; A silent install (/S) shows no window, so the function above hasn't run yet.
  ${If} ${Silent}
    Call PakoInstallDir
    SetOutPath $INSTDIR
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; This copy is installed: Pako keeps its settings and lists in %LOCALAPPDATA%\Pako, never in the
  ; install folder (see src-tauri/src/paths.rs). Without this file Pako would act as the portable one.
  ClearErrors
  FileOpen $0 "$INSTDIR\.pako-installed" w
  ${If} ${Errors}
    Abort "Couldn't finish installing: Windows didn't allow writing in $INSTDIR."
  ${EndIf}
  FileWrite $0 "Installed by Pako Setup. Pako keeps its settings and lists in %LOCALAPPDATA%\Pako.$\r$\n"
  FileClose $0
  ; Left by an update Pako made itself: this install replaces them (the new Pako.exe must not be
  ; put "on trial" against an older copy, or told an update was put back).
  Delete "$INSTDIR\${MAINBINARYNAME}.old.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.bad.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.trial.json"
  Delete "$INSTDIR\${MAINBINARYNAME}.rolled-back.txt"
!macroend

; ---------- uninstalling ----------
; Most of the clean-up is done by Pako.exe --uninstall-cleanup (src-tauri/src/install.rs): a Pako
; that updated itself keeps this uninstaller from the version first installed, but its Pako.exe is
; always the current one, so what changes between versions belongs there.

; A "start with Windows" entry left by another Pako (a portable copy): Tauri deletes the "Pako"
; value after PREUNINSTALL whatever it starts, so it's put back in POSTUNINSTALL.
Var PakoOtherRun
; 1 = Pako.exe couldn't delete the data: this uninstaller deletes what it knows itself.
Var PakoDeleteHere

!macro NSIS_HOOK_PREUNINSTALL
  ; Pako must be closed first (asks; a silent uninstall closes it).
  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  StrCpy $PakoOtherRun ""
  StrCpy $PakoDeleteHere 0
  ; Never when Setup replaces this version (/UPDATE): only the files go then.
  ${If} $UpdateMode <> 1
    ; Removes the browser connection, pako:// links and start with Windows where they point at
    ; this copy. With "Also delete Pako's settings and lists" ticked (never in a silent or passive
    ; uninstall: no page, so no tick), it deletes them too, unless another Pako is open.
    ClearErrors
    ${If} $DeleteAppDataCheckboxState = 1
      ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --uninstall-cleanup --delete-data' $0
    ${Else}
      ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --uninstall-cleanup' $0
    ${EndIf}
    ${If} ${Errors}
      StrCpy $0 1
    ${EndIf}
    ${If} $DeleteAppDataCheckboxState = 1
      ${If} $0 = 2
        ; Another Pako is open (a portable copy): it may use the same folders. Nothing is
        ; deleted, also not by Tauri below.
        StrCpy $DeleteAppDataCheckboxState 0
        MessageBox MB_ICONINFORMATION|MB_OK "Another Pako is open, so Pako's settings and lists were kept. To delete them, close that Pako, then delete the Pako folder in %LOCALAPPDATA%." /SD IDOK
      ${ElseIf} $0 <> 0
        StrCpy $PakoDeleteHere 1
      ${EndIf}
    ${EndIf}
    ; This copy's entry is gone by now: one still there is another Pako's (unless Pako.exe
    ; couldn't run).
    ReadRegStr $PakoOtherRun HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Pako"
    ${If} $PakoOtherRun == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
    ${OrIf} $PakoOtherRun == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --tray"
      StrCpy $PakoOtherRun ""
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Files Pako writes next to itself (its own updates), so the folder can be removed. Only now:
  ; until Tauri's last check that Pako is closed, this copy must still be the installed one.
  Delete "$INSTDIR\.pako-installed"
  Delete "$INSTDIR\${MAINBINARYNAME}.new.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.new.*.part"
  Delete "$INSTDIR\${MAINBINARYNAME}.old.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.bad.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.update.json"
  Delete "$INSTDIR\${MAINBINARYNAME}.trial.json"
  Delete "$INSTDIR\${MAINBINARYNAME}.rolled-back.txt"
  RMDir "$INSTDIR"
  ${If} $UpdateMode <> 1
    ${If} $PakoOtherRun != ""
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Pako" $PakoOtherRun
    ${EndIf}
    ; Pako.exe couldn't delete the data (Tauri itself deleted %LOCALAPPDATA%\com.sookera.pako).
    ${If} $PakoDeleteHere = 1
    ${AndIf} $DeleteAppDataCheckboxState = 1
      RMDir /r "$LOCALAPPDATA\Pako"
      RMDir /r "$LOCALAPPDATA\app.zdownloader"
    ${EndIf}
  ${EndIf}
!macroend
