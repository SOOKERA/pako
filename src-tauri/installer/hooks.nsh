; Pako Setup: run by Tauri's NSIS installer at fixed points (tauri.conf.json:
; bundle.windows.nsis.installerHooks). This file is included near the top of Tauri's script, before
; its own definitions: code outside the macros can't use ${PRODUCTNAME} and friends (so "Pako" is
; written out), the macros can (they are expanded where Tauri inserts them).

; Pako Setup's own window (the slides, the options, the progress, the uninstaller's look).
!include "${__FILEDIR__}\ui.nsh"
; [Pako] Both set up the window before it shows (installer/ui.nsh).
!define MUI_CUSTOMFUNCTION_GUIINIT PakoGuiInit
!define MUI_CUSTOMFUNCTION_UNGUIINIT un.PakoGuiInit

; ---------- where Pako is installed ----------
; Tauri's per-user default is %LOCALAPPDATA%\Pako, which is where Pako keeps its settings and lists.
; Pako goes to %LOCALAPPDATA%\Programs\Pako instead (where Windows puts per-user programs), unless
; an earlier install is elsewhere (Tauri remembers it) or the folder was chosen with /D=.
; (Called by PakoGuiInit, and for silent and passive installs.)

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

; The window's folder (installer/ui.nsh: PakoCheckFolder) also refuses a folder Pako keeps data
; in: %LOCALAPPDATA%\Pako, where the installed Pako keeps its settings and lists (unless an
; earlier install is there already), and a portable Pako's folder (it has its own data folder:
; installing there would switch that copy to %LOCALAPPDATA%\Pako, and its lists would look gone).

; ---------- a folder Pako can update itself in ----------
; Pako Setup installs without admin rights, and Pako updates itself by writing next to Pako.exe,
; without admin rights too: the folder must be one this Windows user can change. Windows only
; refuses a folder like Program Files once files are being copied ("Error opening file for
; writing"), so the window checks it as soon as a folder is shown or picked, and holds Install
; with the reason under the options (installer/ui.nsh: PakoCheckFolder), and a silent or passive
; install checks it before anything is done (installer.nsi: .onInit).

; "" = fine; "write" = this user can't change the folder; "admin" = Setup runs as administrator
; and the folder is one only administrators can change (Pako, run without, couldn't update there).
Var PakoDirProblem
; With "write", why, when it's known (for the window's wording): "missing" = not even the drive
; or network folder is there, "long" = the path is too long for Windows; else "".
Var PakoDirWhy

; Sets $PakoDirProblem for $INSTDIR. Tried for real, with this Setup's rights (none asked for): a
; file of a unique name is made in $INSTDIR and deleted again or, when $INSTDIR isn't there yet,
; a folder with a file in it, in the nearest folder that is there (where Setup will make it).
Function PakoCheckDir
  Push $0
  Push $1
  Push $2
  Push $3
  StrCpy $PakoDirProblem ""
  StrCpy $PakoDirWhy ""
  ; The folder in full ("..", "." and "/" resolved), without a "\" at the end (but "D:\").
  StrCpy $0 $INSTDIR
  ; A drive's own folder: NSIS keeps "D:\" as "D:", which Windows reads as D:'s current folder.
  StrLen $1 $0
  StrCpy $2 $0 1 1
  ${If} $1 = 2
  ${AndIf} $2 == ":"
    StrCpy $0 "$0\"
  ${EndIf}
  System::Call 'kernel32::GetFullPathNameW(w r0, i ${NSIS_MAX_STRLEN}, w .r3, p 0)i.r1'
  ; 0 = failed; NSIS_MAX_STRLEN or more = didn't fit (nothing was written).
  ${If} $1 = 0
  ${OrIf} $1 >= ${NSIS_MAX_STRLEN}
    StrCpy $3 $INSTDIR
  ${EndIf}
  StrLen $1 $3
  ${If} $1 > 3
    StrCpy $2 $3 1 -1
    ${IfThen} $2 == "\" ${|} StrCpy $3 $3 -1 ${|}
  ${EndIf}
  StrCpy $0 $3
  ; Asked by its attributes: a folder this user can't open is still there (FileExists says no).
  ${Do}
    System::Call 'kernel32::GetFileAttributesW(w r0)i.r1'
    ${If} $1 <> -1
      IntOp $1 $1 & 0x10 ; FILE_ATTRIBUTE_DIRECTORY
      ${IfThen} $1 = 0 ${|} StrCpy $PakoDirProblem "write" ${|} ; a file
      ${Break}
    ${EndIf}
    ${GetParent} $0 $1
    ; Not even the drive or the network share is there. Above "\\server\share" GetParent gives
    ; "\\server", then "\": Windows reads "\" as the root of Setup's current drive, so the walk
    ; must stop there (the test folder was made, and left, in C:\ for an unreachable share).
    ${If} $1 == ""
    ${OrIf} $1 == $0
    ${OrIf} $1 == "\"
      StrCpy $PakoDirProblem "write"
      StrCpy $PakoDirWhy "missing"
      ${Break}
    ${EndIf}
    ; "D:" alone is D:'s current folder to Windows: its root is "D:\".
    StrLen $2 $1
    ${If} $2 = 2
      StrCpy $2 $1 1 1
      ${IfThen} $2 == ":" ${|} StrCpy $1 "$1\" ${|}
    ${EndIf}
    StrCpy $0 $1
  ${Loop}
  ${IfThen} $PakoDirProblem != "" ${|} Goto pako_check_done ${|}
  ${IfThen} $0 == "$3" ${|} StrCpy $0 "" ${|}
  ; "" = $INSTDIR is there; else the folder Setup will make it in.
  System::Call 'kernel32::GetCurrentProcessId()i.r1'
  System::Call 'kernel32::GetTickCount()i.r2'
  StrCpy $1 ".pako-setup-check-$1-$2"
  ${If} $0 == ""
    StrCpy $2 $3 1 -1
    ${If} $2 == "\" ; a drive: "D:\"
      StrCpy $0 "$3$1.tmp"
    ${Else}
      StrCpy $0 "$3\$1.tmp"
    ${EndIf}
    ClearErrors
    FileOpen $2 $0 w
    ${If} ${Errors}
      StrCpy $PakoDirProblem "write"
    ${Else}
      FileClose $2
      System::Call 'kernel32::DeleteFileW(w r0)i'
    ${EndIf}
  ${Else}
    ; One folder, made by Windows itself (NSIS' CreateDirectory would make every missing one).
    StrCpy $2 $0 1 -1
    ${If} $2 == "\" ; a drive: "D:\"
      StrCpy $0 "$0$1"
    ${Else}
      StrCpy $0 "$0\$1"
    ${EndIf}
    System::Call 'kernel32::CreateDirectoryW(w r0, p 0)i.r2'
    ${If} $2 = 0
      StrCpy $PakoDirProblem "write"
    ${Else}
      ClearErrors
      FileOpen $2 "$0\check.tmp" w
      ${If} ${Errors}
        StrCpy $PakoDirProblem "write"
      ${Else}
        FileClose $2
        StrCpy $2 "$0\check.tmp"
        System::Call 'kernel32::DeleteFileW(w r2)i'
      ${EndIf}
      ; Removed by Windows too: NSIS' RMDir refuses some paths CreateDirectoryW takes.
      System::Call 'kernel32::RemoveDirectoryW(w r0)i'
    ${EndIf}
  ${EndIf}
  ; Couldn't write there, with a path near Windows' limit (260, with the files Pako puts there).
  ${If} $PakoDirProblem == "write"
    StrLen $1 $3
    ${IfThen} $1 > 200 ${|} StrCpy $PakoDirWhy "long" ${|}
  ${EndIf}
  ; Run as administrator, Setup can write almost anywhere: what matters is whether Pako can,
  ; later, without. That can't be tried from here, so the folders Windows keeps for
  ; administrators are recognised by name.
  ${If} $PakoDirProblem == ""
    System::Call 'shell32::IsUserAnAdmin()i.r1'
    ${If} $1 <> 0
      Push $3
      Call PakoIsAdminFolder
      Pop $1
      ${IfThen} $1 = 1 ${|} StrCpy $PakoDirProblem "admin" ${|}
    ${EndIf}
  ${EndIf}
  pako_check_done:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

!macro PakoIfUnder BASE
  StrLen $2 "${BASE}\"
  StrCpy $1 "$0\" $2
  ${IfThen} $1 == "${BASE}\" ${|} StrCpy $3 1 ${|}
!macroend

; Stack: a folder -> 1 if it is in a folder only administrators can change (a folder named
; "Program Files" or "Program Files (x86)" on any drive, or Windows' own Program Files or Windows
; folder), else 0. Windows ignores case in paths, and so does ==.
Function PakoIsAdminFolder
  Exch $0
  Push $1
  Push $2
  Push $3
  StrCpy $3 0
  StrCpy $1 $0
  ${Do}
    ${GetFileName} $1 $2
    ${If} $2 == "Program Files"
    ${OrIf} $2 == "Program Files (x86)"
      StrCpy $3 1
      ${Break}
    ${EndIf}
    ${GetParent} $1 $2
    ${If} $2 == ""
    ${OrIf} $2 == $1
      ${Break}
    ${EndIf}
    StrCpy $1 $2
  ${Loop}
  !insertmacro PakoIfUnder $WINDIR
  !insertmacro PakoIfUnder $PROGRAMFILES
  !insertmacro PakoIfUnder $PROGRAMFILES64
  StrCpy $0 $3
  Pop $3
  Pop $2
  Pop $1
  Exch $0
FunctionEnd

; A silent (/S) or passive (/P) install shows no folder page: the same check, at the end of
; .onInit, before anything is changed (an older version isn't removed either). A folder this user
; can't change ends Setup with exit code 2 and the message (in the console that started a silent
; Setup). One only administrators can change, with Setup run as administrator, is accepted
; there: no one can be asked.
!macro PAKO_CHECK_DIR_UNATTENDED
  ${If} ${Silent}
  ${OrIf} $PassiveMode = 1
    Call PakoInstallDir
    Call PakoCheckDir
    ${If} $PakoDirProblem == "write"
      ${If} ${Silent}
        System::Call 'kernel32::AttachConsole(i -1)i.r0'
        ${If} $0 <> 0
          System::Call 'kernel32::GetStdHandle(i -11)i.r0'
          FileWrite $0 "$(pakoDirNotWritableSilent)"
        ${EndIf}
      ${Else}
        MessageBox MB_ICONSTOP|MB_OK "$(pakoDirNotWritable)"
      ${EndIf}
      SetErrorLevel 2
      Abort
    ${EndIf}
  ${EndIf}
!macroend

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
    ; Said under the bar, with Close (installer/ui.nsh: PakoFail).
    !insertmacro PakoFail "$(pakoFailInstall)"
    Abort "$(pakoFailInstall)"
  ${EndIf}
  FileWrite $0 "Installed by Pako Setup. Pako keeps its settings and lists in %LOCALAPPDATA%\Pako.$\r$\n"
  FileClose $0
  ; The window's choices (never in a silent or passive install: Pako asks at its first start):
  ; Pako.exe saves them in its settings and does them now (start with Windows, the browser
  ; connection), without a window (src-tauri/src/install.rs). Its first-start setup shows them.
  ; Waited for 15 s at most (it takes well under one); if it fails or doesn't end, "Pako is
  ; installed" says so ($PakoChoicesOk 0, installer/ui.nsh: PakoDonePage).
  ${IfNot} ${Silent}
  ${AndIf} $PakoPassiveOrig <> 1
    Push $1
    Push $2
    Push $3
    Push $4
    Push $5
    Push $6
    StrCpy $PakoChoicesOk 0
    ; [Pako] --terms-accepted=1: Install / Update was clicked under "By installing, you accept the
    ; terms of use", so Pako doesn't ask again at its first start (silent and passive installs
    ; never come here: nobody saw that line).
    StrCpy $1 '"$INSTDIR\${MAINBINARYNAME}.exe" --install-choices --start-with-windows=$PakoStart --browsers=$PakoBrowsers --terms-accepted=1'
    ; STARTUPINFOW (68 bytes in a 32-bit Setup, 104 in a 64-bit one), PROCESS_INFORMATION.
    System::Alloc 104
    Pop $2
    !if ${NSIS_PTR_SIZE} > 4
      System::Call "*$2(i 104)"
    !else
      System::Call "*$2(i 68)"
    !endif
    System::Alloc 24
    Pop $3
    System::Call 'kernel32::CreateProcessW(p 0, w r1, p 0, p 0, i 0, i 0x08000000, p 0, p 0, p r2, p r3) i .r4'
    ${If} $4 <> 0
      System::Call "*$3(p .r5, p .r6)"
      System::Call 'kernel32::WaitForSingleObject(p r5, i 15000) i .r4'
      ${If} $4 = 0
        System::Call 'kernel32::GetExitCodeProcess(p r5, *i .r4)'
        ${IfThen} $4 = 0 ${|} StrCpy $PakoChoicesOk 1 ${|}
      ${EndIf}
      System::Call 'kernel32::CloseHandle(p r5)'
      System::Call 'kernel32::CloseHandle(p r6)'
    ${EndIf}
    System::Free $2
    System::Free $3
    Pop $6
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
  ${EndIf}
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
; The uninstall-cleanup switches ("--delete-data", "--delete-private").
Var PakoUnArgs

!macro NSIS_HOOK_PREUNINSTALL
  ; Pako must be closed first (asks, unless the window said so already; a silent uninstall
  ; closes it).
  !insertmacro PakoCheckRunning "$INSTDIR\${MAINBINARYNAME}.exe"
  StrCpy $PakoOtherRun ""
  StrCpy $PakoDeleteHere 0
  ; Never when Setup replaces this version (/UPDATE): only the files go then.
  ${If} $UpdateMode <> 1
    ; Removes the browser connection, pako:// links and start with Windows where they point at
    ; this copy. With "Also delete Pako's settings and lists" ticked (never in a silent or passive
    ; uninstall: no page, so no tick), it deletes them too, unless another Pako is open.
    ; "Also delete Private mode's files" is its own switch: never from the first one.
    ClearErrors
    StrCpy $PakoUnArgs ""
    ${If} $DeleteAppDataCheckboxState = 1
      StrCpy $PakoUnArgs " --delete-data"
    ${EndIf}
    ${If} $PakoDeletePrivate = 1
      StrCpy $PakoUnArgs "$PakoUnArgs --delete-private"
    ${EndIf}
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --uninstall-cleanup$PakoUnArgs' $0
    ${If} ${Errors}
      StrCpy $0 1
    ${EndIf}
    ${If} $DeleteAppDataCheckboxState = 1
    ${OrIf} $PakoDeletePrivate = 1
      ${If} $0 = 2
        ; Another Pako is open (a portable copy): it may use the same folders. Nothing is
        ; deleted, also not by Tauri below.
        StrCpy $DeleteAppDataCheckboxState 0
        StrCpy $PakoDeletePrivate 0
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

; [Pako] Deletes everything in the folder on the stack except Private mode's folders ("private",
; "private-open"): "Also delete Pako's settings and lists" alone never deletes them.
Function un.PakoDeleteButPrivate
  Exch $R0
  Push $R1
  Push $R2
  FindFirst $R1 $R2 "$R0\*"
  ${DoWhile} $R2 != ""
    ${If} $R2 != "."
    ${AndIf} $R2 != ".."
    ${AndIf} $R2 != "private"
    ${AndIf} $R2 != "private-open"
      ${If} ${FileExists} "$R0\$R2\*"
        RMDir /r "$R0\$R2"
      ${Else}
        Delete "$R0\$R2"
      ${EndIf}
    ${EndIf}
    FindNext $R1 $R2
  ${Loop}
  FindClose $R1
  RMDir "$R0"
  Pop $R2
  Pop $R1
  Pop $R0
FunctionEnd

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
      ${If} $DeleteAppDataCheckboxState = 1
      ${AndIf} $PakoDeletePrivate = 1
        RMDir /r "$LOCALAPPDATA\Pako"
        RMDir /r "$LOCALAPPDATA\app.zdownloader"
        ; Recordings not joined yet and browser logins left by a Pako that didn't close normally.
        RMDir /r "$TEMP\Pako"
      ${ElseIf} $DeleteAppDataCheckboxState = 1
        ; Everything but Private mode's folders.
        Push "$LOCALAPPDATA\Pako"
        Call un.PakoDeleteButPrivate
        RMDir /r "$LOCALAPPDATA\app.zdownloader"
        Push "$TEMP\Pako"
        Call un.PakoDeleteButPrivate
      ${ElseIf} $PakoDeletePrivate = 1
        RMDir /r "$LOCALAPPDATA\Pako\private"
        RMDir /r "$TEMP\Pako\private-open"
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend
