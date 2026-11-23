; Pako Setup's own window (user-approved design, 2026-11-18): one wide window that follows
; Windows' light or dark look. Left: five feature slides that slide on by themselves every 4.5 s
; (they wait while the mouse is on them; the dots and arrows flip them, and a click stops them).
; Right: Pako's logo and every option at once (folder, desktop shortcut, start with Windows, the
; browser connection), one Install button and the terms line. Then the same window shows the
; progress, then "Open Pako". Over an installed Pako it is "Update Pako" (folder locked). The
; uninstaller has the matching look, smaller.
;
; Included by hooks.nsh; the functions are put in by installer.nsi ("!insertmacro
; PAKO_UI_FUNCTIONS", where Tauri's defines exist). The pictures are made by
; scripts/installer-art.mjs (art/), drawn at twice the size and scaled here to the screen.
; Silent (/S) and passive (/P) installs never show any of this.

!define PAKO_ART "${__FILEDIR__}\art"

!ifndef SS_NOTIFY
  !define SS_NOTIFY 0x100
!endif
!define PAKO_SS_CENTER 0x1
!define PAKO_SS_RIGHT 0x2
!define PAKO_SS_BITMAP 0xE
!define PAKO_SS_NOPREFIX 0x80
!define PAKO_SS_PATHELLIPSIS 0x8000
!define PAKO_SS_ENDELLIPSIS 0x4000
!define PAKO_LABEL 0x50000080 ; WS_CHILD | WS_VISIBLE | SS_NOPREFIX
!define PAKO_PIC 0x5000010E ; WS_CHILD | WS_VISIBLE | SS_BITMAP | SS_NOTIFY
; A picture the mouse goes through (no SS_NOTIFY), hidden at first: the progress page's Close,
; over Windows' own Cancel button.
!define PAKO_PIC_PASS 0x4000000E
; The keyboard's and screen readers' stand-ins (PakoProxy): WS_CHILD | WS_VISIBLE |
; WS_CLIPSIBLINGS | WS_TABSTOP, a push button or a check box.
!define PAKO_PUSH 0x54010000
!define PAKO_CHECK 0x54010002
!define PAKO_HIDDEN_LABEL 0x40000080

; The slides' pictures are read before anything else (solid compression).
ReserveFile "${PAKO_ART}\*.jpg"
ReserveFile "${PAKO_ART}\*.png"

; The switches' steps between off and on (template 1.1.5: they slide), drawn by the art script.
!if ! /FileExists "${PAKO_ART}\sw9-l.png"
  !error "art\sw1-d.png ... sw9-l.png are missing: template 1.1.5's switches slide between off and on. Run scripts/installer-art.mjs again."
!endif

; [Pako] Pako's Setup always has the browser switch: the template's setting for it (app.nsh), so
; the switch code taken from the template (1.1.5) reads the same.
!ifndef PAKO_BROWSER_SWITCH
  !define PAKO_BROWSER_SWITCH 1
!endif

; [Pako] French needs the art script's French pictures too (a slide's `fr` in
; scripts/installer-art.mjs makes slideN-d-fr.jpg, the buttons' b-<name>-<state>-d-fr.png...).
!if ! /FileExists "${PAKO_ART}\slide1-d-fr.jpg"
  !error "Pako Setup speaks French but art\slide1-d-fr.jpg is missing: give every slide its fr: { title, desc } in scripts/installer-art.mjs and run it again."
!endif
!if ! /FileExists "${PAKO_ART}\b-install-n-d-fr.png"
  !error "Pako Setup speaks French but art\b-install-n-d-fr.png is missing: run scripts/installer-art.mjs again (with fr in the slides)."
!endif

; ---------- the words, in English or French ----------
; [Pako] English.nsh and French.nsh give each text twice: as Tauri's LangString (what $(name) reads,
; always in the language Setup started in: NSIS fixes it once .onInit is over) and as a define,
; PakoEn_<name> / PakoFr_<name>. Setup's own code reads the define for the language shown ($PakoLang)
; through PakoT, so the window can switch languages while it is open. The uninstaller and silent or
; passive runs know their language from the start, so both ways give the same words there.
Var PakoLang          ; en or fr: the language Setup's window speaks
Var PakoTx            ; PakoT's text, for the line that uses it next
Var PakoTxExtract     ; NSIS's own "Copying " (^Extract) in the language shown (English.nsh)
Var PakoArtSfx        ; "" or "-fr": the pictures with words in them, in the language shown
!define PAKO_LANG_EN_NAME "English"
!define PAKO_LANG_FR_NAME "Français"

!macro PakoEn KEY TEXT
  !define PakoEn_${KEY} "${TEXT}"
  LangString ${KEY} ${LANG_ENGLISH} "${TEXT}"
!macroend
!macro PakoFr KEY TEXT
  !define PakoFr_${KEY} "${TEXT}"
  LangString ${KEY} ${LANG_FRENCH} "${TEXT}"
!macroend

; OUT = the text KEY in the language shown. Only after English.nsh and French.nsh (installer.nsi
; includes them after the pages): code before that calls a function that uses it (PakoRunTexts).
!macro PakoT OUT KEY
  !ifndef PakoEn_${KEY}
    !error "No English text ${KEY} (English.nsh), or PakoT is used before the language files are included."
  !endif
  !ifndef PakoFr_${KEY}
    !error "No French text ${KEY} in French.nsh."
  !endif
  ${If} $PakoLang == "fr"
    StrCpy ${OUT} "${PakoFr_${KEY}}"
  ${Else}
    StrCpy ${OUT} "${PakoEn_${KEY}}"
  ${EndIf}
!macroend

; .onInit (installer.nsi): Setup speaks English until the window's choice says otherwise; it never
; follows Windows' language. Over an installed Pako the window opens in the language saved for
; it (PakoGuiInit). A silent or passive install (no window, no choice) keeps the language saved
; before, so that the uninstaller keeps it too.
!macro PAKO_LANG_INIT
  StrCpy $PakoLang "en"
  StrCpy $PakoArtSfx ""
  !if "${DISPLAYLANGUAGESELECTOR}" == "true"
    !error "Pako Setup's window asks the language itself: set displayLanguageSelector to false in tauri.conf.json."
  !endif
  StrCpy $LANGUAGE ${LANG_ENGLISH}
  ${If} ${Silent}
  ${OrIf} $PassiveMode = 1
    ReadRegStr $PakoTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
    ${If} $PakoTx == ${LANG_FRENCH}
      StrCpy $PakoLang "fr"
      StrCpy $LANGUAGE ${LANG_FRENCH}
      StrCpy $PakoArtSfx "-fr"
    ${EndIf}
  ${EndIf}
  Call PakoLangTexts
!macroend

; un.onInit (installer.nsi): the uninstaller speaks the language Setup ran in (saved by Setup as
; the app's "Installer Language", 1033 or 1036). Missing (installed by an older Pako Setup) or
; anything else: English. Never Windows' language dialog.
!macro PAKO_UN_LANG_INIT
  StrCpy $PakoLang "en"
  StrCpy $PakoArtSfx ""
  StrCpy $LANGUAGE ${LANG_ENGLISH}
  ReadRegStr $PakoTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
  ${If} $PakoTx == ${LANG_FRENCH}
    StrCpy $PakoLang "fr"
    StrCpy $LANGUAGE ${LANG_FRENCH}
    StrCpy $PakoArtSfx "-fr"
  ${EndIf}
!macroend

Var PakoDpi
Var PakoLook          ; d(ark) or l(ight)
Var PakoCBg
Var PakoCText
Var PakoCRa
Var PakoFBody         ; 13 px
Var PakoFSmall        ; 12 px
Var PakoFTiny         ; 11 px
Var PakoFTitle        ; 17 px semibold
Var PakoX
Var PakoY
Var PakoW
Var PakoH
Var PakoDialog
Var PakoGdip
; slides
Var PakoS1
Var PakoS2
Var PakoS3
Var PakoS4
Var PakoS5
; [Pako] PakoS1..5 are the slides of the language shown: the English ones (PakoSE) are read before
; the window shows, the French ones (PakoSF) one by one while the window is idle (PakoTick), or at
; once when French is picked before that.
Var PakoSE1
Var PakoSE2
Var PakoSE3
Var PakoSE4
Var PakoSE5
Var PakoSF1
Var PakoSF2
Var PakoSF3
Var PakoSF4
Var PakoSF5
Var PakoSFReady       ; how many French slides are read
Var PakoSlide         ; 1..5
Var PakoAuto          ; 1 = moves on by itself
Var PakoIdleT0        ; when the slide last changed or the mouse was last on it (ms)
Var PakoCur0          ; where the mouse was when the slides showed ("x,y")
Var PakoMouseSeen     ; 1 = the mouse has moved since then (only then does it hold the slides)
Var PakoSlideCtl
Var PakoFailName      ; the file PakoCanWrite couldn't write
; [Pako] The slide change: the new slide comes in sideways while it fades in over the one that
; showed, which moves off the other way (PakoSlideDraw). Each step is drawn into $PakoOut and put
; on the screen in one go (never through the picture control's own painting, which showed a white
; flash while Windows was busy); $PakoFrame keeps the last step for the control's own repaints.
Var PakoFrame
Var PakoFrom
Var PakoTo
Var PakoOut
Var PakoMix
Var PakoDcF           ; picks up a picture shared with a control, for one copy at a time
Var PakoDcFrom
Var PakoDcTo
Var PakoDcOut
Var PakoDcMix
Var PakoDir           ; 1 = the next slide (comes from the right), -1 = the previous one
Var PakoIntro         ; 1 = the first slide still has to come in (once, when Setup's window shows)
Var PakoFadeT0
Var PakoSW            ; the slide's size, the moving part's height and how far it moves, on screen
Var PakoSH
Var PakoSTop
Var PakoShift
; pictures and buttons
Var PakoBmpLogo
Var PakoBmpOn
Var PakoBmpOff
Var PakoBmpField
Var PakoB1Ctl
Var PakoB1N
Var PakoB1H
Var PakoB1X
Var PakoB1Now         ; n, h or x
Var PakoB2Ctl
Var PakoB2N
Var PakoB2H
Var PakoB2Now
; the options
Var PakoMode          ; install, update (older installed), same, older (newer installed)
Var PakoOldVersion
Var PakoCmp
Var PakoR6
Var PakoUiChoice      ; read by PageLeaveReinstall instead of its old radio buttons
Var PakoDesktop
Var PakoStart
Var PakoBrowsers
Var PakoSwDesk        ; the switches' pictures (the uninstaller's two use Desk and Start)
Var PakoSwStart
Var PakoSwBrow
; The switches' slide (template 1.1.5, PakoSwTick). Each switch shows its own picture (PakoSwFace,
; 24-bit: the control draws it as it is), into which the step to show is copied. The steps:
; off (0), PakoSwB1..9 in between, on (10), drawn by the art script (sw1..sw9). Per switch (1 to 3):
; P where it is now, G where it goes, A where it was when it set off (0 to 1000), T when (ms),
; K the step shown.
Var PakoSwB1
Var PakoSwB2
Var PakoSwB3
Var PakoSwB4
Var PakoSwB5
Var PakoSwB6
Var PakoSwB7
Var PakoSwB8
Var PakoSwB9
Var PakoSwFace1
Var PakoSwFace2
Var PakoSwFace3
Var PakoSwDcA
Var PakoSwDcB
Var PakoSwW
Var PakoSwH
Var PakoSwP1
Var PakoSwP2
Var PakoSwP3
Var PakoSwG1
Var PakoSwG2
Var PakoSwG3
Var PakoSwA1
Var PakoSwA2
Var PakoSwA3
Var PakoSwT1
Var PakoSwT2
Var PakoSwT3
Var PakoSwK1
Var PakoSwK2
Var PakoSwK3
Var PakoPathCtl
Var PakoNoteCtl
Var PakoHeld          ; 1 = Install can't be clicked (the folder won't do)
Var PakoRunning       ; 1 = Pako is open (said in the window)
Var PakoAgreedClose   ; 1 = Pako may be closed without asking again (it was said)
Var PakoPassiveKeep
Var PakoOwnCheck      ; 1 = PakoCheckRunning uses its own words (see there)
Var PakoPassiveOrig
Var PakoOpen          ; done page: 1 = start Pako on the way out
Var PakoFolderMsg
Var PakoOldDir        ; where the installed Pako is (not "install")
Var PakoCanMove       ; 1 = the installed Pako's folder won't do: Change moves Pako
Var PakoMoved         ; 1 = Pako goes to another folder than the installed one's
Var PakoChangeCtl
; The keyboard's and screen readers' stand-ins (PakoProxy) for what is drawn as pictures.
Var PakoPxChange
Var PakoPxB1
Var PakoPxB2
Var PakoPxDesk
Var PakoPxStart
Var PakoPxBrow
Var PakoPxTerms
Var PakoPxPrev        ; the slides' ‹ and ›, and the line beside them (they stay on a language switch)
Var PakoPxNext
Var PakoSepCtl
; [Pako] The language choice (top right of the main page): its two words and their stand-ins
; (radio buttons: Tab reaches the one chosen, the arrows move between them, Narrator reads
; "English, radio button, selected").
Var PakoLangEnCtl
Var PakoLangFrCtl
Var PakoPxEn
Var PakoPxFr
Var PakoLangNext      ; the language just clicked (applied a moment later: PakoLangApply)
Var PakoLangFocus     ; which control had the keyboard when it was clicked
Var PakoLangPicked    ; 1 = a language was picked in the window (even the one shown)
; [Pako] Uninstaller: 1 = "Also delete Private mode's files" (never in a silent or passive run).
Var PakoDeletePrivate
; The focus ring (4 thin lines), shown around what the keyboard is on.
Var PakoRing1
Var PakoRing2
Var PakoRing3
Var PakoRing4
Var PakoRingOn
; The progress page: 1 = Pako's look shows it; its Close (shown only when something went wrong).
Var PakoInFiles
Var PakoFailBtn
Var PakoFailPic
Var PakoFailBmp
Var PakoChoicesOk     ; 0 = Pako.exe couldn't take the window's choices (said when done)
; Arguments and result of the helper functions below.
Var PakoA1
Var PakoA2
Var PakoA3
Var PakoRet

; ---------- scaled sizes ----------
; Every size in this file is in pixels at 100 % (96 dpi); these make them the screen's.
!macro PakoScale OUT VAL
  IntOp ${OUT} ${VAL} * $PakoDpi
  IntOp ${OUT} ${OUT} + 48
  IntOp ${OUT} ${OUT} / 96
!macroend

; Draws the slide in SRC into DC, moved sideways by X (the strip it leaves gets the background),
; and the dots and arrows under it where they are.
!macro PakoShifted DC SRC X
  System::Call "gdi32::BitBlt(p ${DC}, i ${X}, i 0, i $PakoSW, i $PakoSTop, p ${SRC}, i 0, i 0, i 0x00CC0020)"
  ${If} ${X} < 0
    IntOp $6 $PakoSW + ${X}
    IntOp $5 0 - ${X}
    System::Call "gdi32::PatBlt(p ${DC}, i r6, i 0, i r5, i $PakoSTop, i 0x00F00021)"
  ${ElseIf} ${X} > 0
    System::Call "gdi32::PatBlt(p ${DC}, i 0, i 0, i ${X}, i $PakoSTop, i 0x00F00021)"
  ${EndIf}
  IntOp $5 $PakoSH - $PakoSTop
  System::Call "gdi32::BitBlt(p ${DC}, i 0, i $PakoSTop, i $PakoSW, i r5, p ${SRC}, i 0, i $PakoSTop, i 0x00CC0020)"
!macroend

; A control on the current nsDialogs page: VAR = its handle.
!macro PakoCtl VAR CLASS STYLE X Y W H TEXT
  !insertmacro PakoScale $PakoX ${X}
  !insertmacro PakoScale $PakoY ${Y}
  !insertmacro PakoScale $PakoW ${W}
  !insertmacro PakoScale $PakoH ${H}
  nsDialogs::CreateControl ${CLASS} ${STYLE} 0 $PakoX $PakoY $PakoW $PakoH "${TEXT}"
  Pop ${VAR}
!macroend

; A control on a page that isn't nsDialogs' (the progress pages): PARENT is that page.
!macro PakoRaw VAR PARENT CLASS STYLE X Y W H TEXT
  !insertmacro PakoScale $PakoX ${X}
  !insertmacro PakoScale $PakoY ${Y}
  !insertmacro PakoScale $PakoW ${W}
  !insertmacro PakoScale $PakoH ${H}
  ; (Texts go to System::Call in a register: a comma in one would split its arguments.)
  Push $R9
  StrCpy $R9 "${TEXT}"
  System::Call 'user32::CreateWindowExW(i 0, w "${CLASS}", w R9, i ${STYLE}, i $PakoX, i $PakoY, i $PakoW, i $PakoH, p ${PARENT}, p 0, p 0, p 0) p .s'
  Exch
  Pop $R9
  Pop ${VAR}
!macroend

; A label: FONT, text colour, background colour.
!macro PakoLabel VAR X Y W H STYLE FONT FG BG TEXT
  !insertmacro PakoCtl ${VAR} STATIC ${STYLE} ${X} ${Y} ${W} ${H} "${TEXT}"
  SendMessage ${VAR} ${WM_SETFONT} ${FONT} 1
  !insertmacro PakoColors ${VAR} ${FG} ${BG}
!macroend

; Pako's colours, by name, in the look Setup uses. SetCtlColors takes its colours when Setup is
; built, so each look's are written out here (the same as scripts/installer-art.mjs).
!define PAKO_D_BG 0x14171b
!define PAKO_D_PANE 0x0f1114
!define PAKO_D_TEXT 0xe9eaee
!define PAKO_D_MUT 0x8b919c
!define PAKO_D_LN 0x262a31
!define PAKO_D_RA 0x1b1f24
!define PAKO_D_WARN 0xff7a7b
!define PAKO_D_ACCENT 0xf96263
!define PAKO_L_BG 0xffffff
!define PAKO_L_PANE 0xf6f7f9
!define PAKO_L_TEXT 0x17181c
!define PAKO_L_MUT 0x6b7280
!define PAKO_L_LN 0xe2e7ee
!define PAKO_L_RA 0xf3f5f8
!define PAKO_L_WARN 0xd33a3d
!define PAKO_L_ACCENT 0xf96263
!macro PakoColors HWND FG BG
  ${If} $PakoLook == "d"
    SetCtlColors ${HWND} ${PAKO_D_${FG}} ${PAKO_D_${BG}}
  ${Else}
    SetCtlColors ${HWND} ${PAKO_L_${FG}} ${PAKO_L_${BG}}
  ${EndIf}
!macroend

!macro PakoPicture VAR X Y W H BMP
  !insertmacro PakoCtl ${VAR} STATIC ${PAKO_PIC} ${X} ${Y} ${W} ${H} ""
  SendMessage ${VAR} ${STM_SETIMAGE} 0 ${BMP}
!macroend

; What is drawn as a picture (buttons, switches, links, the slides' arrows) gets a real button
; under it, the same size, for the keyboard and screen readers: Tab reaches it, Narrator reads
; TEXT (and a switch's on/off), Space or Enter clicks it. It is never seen (the picture covers it
; and WS_CLIPSIBLINGS keeps it from painting over it); the focus ring shows where Tab is. The
; mouse still clicks the picture. Tab goes in the order they are made (each goes to the bottom
; of the window's order, under the one before).
!macro PakoProxy VAR STYLE X Y W H TEXT
  !insertmacro PakoCtl ${VAR} BUTTON ${STYLE} ${X} ${Y} ${W} ${H} "${TEXT}"
  SendMessage ${VAR} ${WM_SETFONT} $PakoFSmall 1
  System::Call "user32::SetWindowPos(p ${VAR}, p 1, i 0, i 0, i 0, i 0, i 0x13)"
!macroend

; The focus ring's 4 lines, hidden until Tab is used (PakoFocusTick places them).
!macro PakoRingMake
  !insertmacro PakoLabel $PakoRing1 0 0 1 1 ${PAKO_HIDDEN_LABEL} $PakoFBody TEXT TEXT ""
  !insertmacro PakoLabel $PakoRing2 0 0 1 1 ${PAKO_HIDDEN_LABEL} $PakoFBody TEXT TEXT ""
  !insertmacro PakoLabel $PakoRing3 0 0 1 1 ${PAKO_HIDDEN_LABEL} $PakoFBody TEXT TEXT ""
  !insertmacro PakoLabel $PakoRing4 0 0 1 1 ${PAKO_HIDDEN_LABEL} $PakoFBody TEXT TEXT ""
  StrCpy $PakoRingOn ""
!macroend

!macro PakoRingNone
  StrCpy $PakoRing1 0
  StrCpy $PakoRing2 0
  StrCpy $PakoRing3 0
  StrCpy $PakoRing4 0
  StrCpy $PakoRingOn ""
!macroend

; Something went wrong while installing or uninstalling: the line under the bar says what (in
; the warning colour) and Close shows, so the window can be left (Windows' Cancel under Pako's
; picture; Esc and the window's ✕ close it too). Only with Pako's look on the progress page.
!macro PakoFail TEXT
  ${If} $PakoInFiles = 1
    Push $0
    Push $1
    FindWindow $0 "#32770" "" $HWNDPARENT
    GetDlgItem $1 $0 1006
    ; Windows' line under the bar doesn't wrap (SS_LEFTNOWORDWRAP): what went wrong may need two
    ; lines, so here it wraps (SS_LEFT) in a box tall enough for three (same place and width).
    Push $2
    Push $3
    System::Call "user32::GetWindowLongW(p r1, i -16) i .r2"
    IntOp $2 $2 & 0xFFFFFFE0
    System::Call "user32::SetWindowLongW(p r1, i -16, i r2)"
    System::Call "*(i 0, i 0, i 0, i 0) p .r3"
    System::Call "user32::GetClientRect(p r1, p r3)"
    System::Call "*$3(i, i, i .r2, i)"
    System::Free $3
    !insertmacro PakoScale $3 52
    System::Call "user32::SetWindowPos(p r1, p 0, i 0, i 0, i r2, i r3, i 0x16)" ; NOMOVE NOZORDER NOACTIVATE
    Pop $3
    Pop $2
    SendMessage $1 ${WM_SETTEXT} 0 "STR:${TEXT}"
    !insertmacro PakoColors $1 WARN BG
    System::Call "user32::InvalidateRect(p r1, p 0, i 1)"
    ShowWindow $PakoFailBtn ${SW_SHOW}
    ShowWindow $PakoFailPic ${SW_SHOW}
    Pop $1
    Pop $0
  ${EndIf}
!macroend

; [Pako] Before a file is copied over: can it be written? A file another program holds (an
; antivirus scan, a window left open) is tried again for 5 s; still held, the install stops
; there with what happened under the bar and Close (PakoFail), instead of Windows' own
; Abort / Retry / Ignore box over a window with no button. NAME: the file's name, to say it.
!macro PakoCanWrite FILE NAME
  Push $0
  Push $1
  StrCpy $1 0
  ${Do}
    ${IfNot} ${FileExists} "${FILE}"
      ${Break}
    ${EndIf}
    ; GENERIC_WRITE, no sharing, OPEN_EXISTING: what copying over it needs.
    System::Call 'kernel32::CreateFileW(w "${FILE}", i 0x40000000, i 0, p 0, i 3, i 0x80, p 0) i .r0'
    ${If} $0 <> -1
      System::Call 'kernel32::CloseHandle(i r0)'
      ${Break}
    ${EndIf}
    IntOp $1 $1 + 1
    ${If} $1 >= 10
      StrCpy $PakoFailName "${NAME}"
      Pop $1
      Pop $0
      !insertmacro PakoT $PakoTx pakoFailFile
      !insertmacro PakoFail $PakoTx
      Abort $PakoTx
    ${EndIf}
    Sleep 500
  ${Loop}
  Pop $1
  Pop $0
!macroend

; Pako's own Check: the same as Tauri's (asks before closing a Pako that is open), without the
; question when the window already said Pako will be closed. On the progress page in Pako's look
; it is Tauri's check written out, so that a Pako that couldn't be closed (or "Cancel" to the
; question) says so under the bar, with Close (PakoFail), instead of a frozen window.
!macro PakoCheckRunning EXE
  StrCpy $PakoPassiveKeep $PassiveMode
  ${If} $PakoAgreedClose = 1
    StrCpy $PassiveMode 1
  ${EndIf}
  ; [Pako] With French on, also when the window's main page is left (PageLeaveReinstall), so that
  ; its question is in the language shown (Tauri's check reads its LangStrings: the language
  ; Setup started in). Silent and passive runs keep Tauri's.
  StrCpy $PakoOwnCheck $PakoInFiles
  ${IfNot} ${Silent}
  ${AndIf} $PakoPassiveOrig <> 1
    StrCpy $PakoOwnCheck 1
  ${EndIf}
  ${If} $PakoOwnCheck = 1
    ; (A function: this macro is also used before the language files are included.)
    !ifdef __UNINSTALL__
      Call un.PakoRunTexts
    !else
      Call PakoRunTexts
    !endif
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 != ""
      !insertmacro RestartManager_RegisterFile $R0 "${EXE}"
      ${If} $0 = 0
        System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
        ${If} $0 = ${ERROR_MORE_DATA}
          StrCpy $1 1
          ${If} $PassiveMode != 1
            MessageBox MB_OKCANCEL $R2 IDOK +2
            StrCpy $1 0
          ${EndIf}
          ${If} $1 = 0
            !insertmacro RestartManager_EndSession $R0
            StrCpy $PassiveMode $PakoPassiveKeep
            !insertmacro PakoFail $R1
            Abort $R1
          ${EndIf}
          System::Call 'RSTRTMGR::RmShutdown(p R0, i ${RmForceShutdown}, p 0) i .r0'
          ${If} $0 <> 0
            !insertmacro RestartManager_EndSession $R0
            StrCpy $PassiveMode $PakoPassiveKeep
            !insertmacro PakoFail $R3
            Abort $R3
          ${EndIf}
        ${EndIf}
      ${EndIf}
      !insertmacro RestartManager_EndSession $R0
    ${EndIf}
  ${Else}
    !insertmacro CheckIfAppIsRunning "${EXE}" "${PRODUCTNAME}"
  ${EndIf}
  StrCpy $PassiveMode $PakoPassiveKeep
!macroend

; ---------- the functions (installer and uninstaller each get their own copy) ----------
!macro PAKO_UI_COMMON UN

; [Pako] PakoCheckRunning's three messages, in the language shown -> $R1, $R2, $R3.
Function ${UN}PakoRunTexts
  !insertmacro PakoT $PakoTx appRunning
  nsis_tauri_utils::StrReplace "$PakoTx" "{{product_name}}" "${PRODUCTNAME}"
  Pop $R1
  !insertmacro PakoT $PakoTx appRunningOkKill
  nsis_tauri_utils::StrReplace "$PakoTx" "{{product_name}}" "${PRODUCTNAME}"
  Pop $R2
  !insertmacro PakoT $PakoTx failedToKillApp
  nsis_tauri_utils::StrReplace "$PakoTx" "{{product_name}}" "${PRODUCTNAME}"
  Pop $R3
FunctionEnd

; Colours, fonts and the screen's scale; GDI+ for the pictures.
Function ${UN}PakoLookInit
  Push $0
  Push $1
  Push $2
  StrCpy $PakoDpi 96
  System::Call "user32::GetDpiForWindow(p $HWNDPARENT) i .r0"
  ${If} $0 > 0
    StrCpy $PakoDpi $0
  ${EndIf}
  ; Windows' "app mode": AppsUseLightTheme 0 = dark (missing = light, Windows' default).
  StrCpy $PakoLook "l"
  ClearErrors
  ReadRegDWORD $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Themes\Personalize" "AppsUseLightTheme"
  ${IfNot} ${Errors}
  ${AndIf} $0 = 0
    StrCpy $PakoLook "d"
  ${EndIf}
  ; For trying both looks (tests, screenshots).
  ReadEnvStr $0 PAKO_SETUP_LOOK
  ${If} $0 == "dark"
    StrCpy $PakoLook "d"
  ${ElseIf} $0 == "light"
    StrCpy $PakoLook "l"
  ${EndIf}
  ; Pako's colours as numbers, for the title bar and the progress bar.
  ${If} $PakoLook == "d"
    StrCpy $PakoCBg ${PAKO_D_BG}
    StrCpy $PakoCText ${PAKO_D_TEXT}
    StrCpy $PakoCRa ${PAKO_D_RA}
  ${Else}
    StrCpy $PakoCBg ${PAKO_L_BG}
    StrCpy $PakoCText ${PAKO_L_TEXT}
    StrCpy $PakoCRa ${PAKO_L_RA}
  ${EndIf}
  ; Fonts in pixels for this screen (negative = the letters' own height).
  !insertmacro PakoScale $0 13
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $PakoFBody $1
  !insertmacro PakoScale $0 12
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $PakoFSmall $1
  !insertmacro PakoScale $0 11
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $PakoFTiny $1
  !insertmacro PakoScale $0 17
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 600, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $PakoFTitle $1
  ; GDI+ reads the PNG and JPEG pictures and scales them smoothly.
  System::Call "*(i 1, p 0, i 0, i 0) p .r0"
  System::Call "gdiplus::GdiplusStartup(*p .r1, p r0, p 0) i .r2"
  System::Free $0
  StrCpy $PakoGdip $1
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $PakoA1 = file name in art (with -d/-l and the extension), $PakoA2 x $PakoA3 = its size at
; 100 % -> $PakoRet = a bitmap of that size for this screen, or 0.
Function ${UN}PakoImage
  Push $0
  Push $1
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  Push $9
  !insertmacro PakoScale $1 $PakoA2
  !insertmacro PakoScale $0 $PakoA3
  StrCpy $9 0
  StrCpy $8 "$PLUGINSDIR\art\$PakoA1"
  System::Call "gdiplus::GdipCreateBitmapFromFile(w r8, *p .r3) i .r8"
  ${If} $8 = 0
    System::Call "gdiplus::GdipGetImageWidth(p r3, *i .r6)"
    System::Call "gdiplus::GdipGetImageHeight(p r3, *i .r7)"
    System::Call "gdiplus::GdipCreateBitmapFromScan0(i r1, i r0, i 0, i 0x26200A, p 0, *p .r4) i .r8"
    System::Call "gdiplus::GdipGetImageGraphicsContext(p r4, *p .r5) i .r8"
    System::Call "gdiplus::GdipSetInterpolationMode(p r5, i 7)"   ; high quality bicubic
    System::Call "gdiplus::GdipSetPixelOffsetMode(p r5, i 2)"     ; high quality
    System::Call "gdiplus::GdipSetCompositingQuality(p r5, i 2)"
    ; Edges: the picture's own edge pixels, not a darker blend with nothing.
    System::Call "gdiplus::GdipCreateImageAttributes(*p .r8)"
    System::Call "gdiplus::GdipSetImageAttributesWrapMode(p r8, i 3, i 0, i 0)"
    System::Call "gdiplus::GdipDrawImageRectRectI(p r5, p r3, i 0, i 0, i r1, i r0, i 0, i 0, i r6, i r7, i 2, p r8, p 0, p 0)"
    System::Call "gdiplus::GdipDisposeImageAttributes(p r8)"
    System::Call "gdiplus::GdipDeleteGraphics(p r5)"
    System::Call "gdiplus::GdipCreateHBITMAPFromBitmap(p r4, *p .r9, i 0xff000000)"
    System::Call "gdiplus::GdipDisposeImage(p r4)"
    System::Call "gdiplus::GdipDisposeImage(p r3)"
    ; Made 24-bit: a picture control (comctl32 6) keeps its own copy of every 32-bit picture it
    ; is given, which would never be freed (the slides change every 4.5 s, the buttons on hover).
    ${If} $9 != 0
      Push $2
      System::Call "user32::GetDC(p 0) p .r2"
      System::Call "gdi32::CreateCompatibleDC(p r2) p .r5"
      System::Call "gdi32::CreateCompatibleDC(p r2) p .r6"
      System::Call "user32::ReleaseDC(p 0, p r2)"
      IntOp $7 0 - $0
      System::Call "*(i 40, i r1, i r7, &i2 1, &i2 24, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r8"
      System::Call "gdi32::CreateDIBSection(p r6, p r8, i 0, *p .r3, p 0, i 0) p .r4"
      System::Free $8
      ${If} $4 != 0
        System::Call "gdi32::SelectObject(p r5, p r9) p .r7"
        System::Call "gdi32::SelectObject(p r6, p r4) p .r8"
        System::Call "gdi32::BitBlt(p r6, i 0, i 0, i r1, i r0, p r5, i 0, i 0, i 0x00CC0020)"
        System::Call "gdi32::SelectObject(p r5, p r7)"
        System::Call "gdi32::SelectObject(p r6, p r8)"
        System::Call "gdi32::DeleteObject(p r9)"
        StrCpy $9 $4
      ${EndIf}
      System::Call "gdi32::DeleteDC(p r5)"
      System::Call "gdi32::DeleteDC(p r6)"
      Pop $2
    ${EndIf}
  ${EndIf}
  StrCpy $PakoRet $9
  Pop $9
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $1
  Pop $0
FunctionEnd

; $PakoA1 x $PakoA2 (at 100 %): makes the window's inside that size, with only Pako's pages
; in it (Tauri's header, buttons and lines hidden; Next and Cancel still work from the keyboard
; and from Pako's own buttons), centred on its screen, with a title bar in Pako's colours.
Function ${UN}PakoWindow
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  !insertmacro PakoScale $0 $PakoA1
  !insertmacro PakoScale $1 $PakoA2
  ; The whole window for that inside.
  System::Call "user32::GetWindowLongW(p $HWNDPARENT, i -16) i .r6"
  System::Call "user32::GetWindowLongW(p $HWNDPARENT, i -20) i .r7"
  System::Call "*(i 0, i 0, i r0, i r1) p .r2"
  System::Call "user32::AdjustWindowRectExForDpi(p r2, i r6, i 0, i r7, i $PakoDpi) i .r8"
  ${If} $8 = 0
    System::Call "user32::AdjustWindowRectEx(p r2, i r6, i 0, i r7)"
  ${EndIf}
  System::Call "*$2(i .r3, i .r4, i .r5, i .r6)"
  System::Free $2
  IntOp $5 $5 - $3
  IntOp $6 $6 - $4
  ; Centred in the work area of the screen the window is on.
  System::Call "user32::MonitorFromWindow(p $HWNDPARENT, i 2) p .r7"
  System::Call "*(i 40, i 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r2"
  System::Call "user32::GetMonitorInfoW(p r7, p r2)"
  System::Call "*$2(i, i, i, i, i, i .r3, i .r4, i .r7, i .r8)"
  System::Free $2
  IntOp $7 $7 - $3
  IntOp $7 $7 - $5
  IntOp $7 $7 / 2
  ; Bigger than the screen (a small screen at a high scale): its top left corner stays on it.
  ${IfThen} $7 < 0 ${|} StrCpy $7 0 ${|}
  IntOp $3 $3 + $7
  IntOp $8 $8 - $4
  IntOp $8 $8 - $6
  IntOp $8 $8 / 2
  ${IfThen} $8 < 0 ${|} StrCpy $8 0 ${|}
  IntOp $4 $4 + $8
  System::Call "user32::SetWindowPos(p $HWNDPARENT, p 0, i r3, i r4, i r5, i r6, i 0x14)"
  ; Tauri's own parts of the window, hidden (the buttons keep working).
  !insertmacro PakoHideItem 1
  !insertmacro PakoHideItem 2
  !insertmacro PakoHideItem 3
  !insertmacro PakoHideItem 1028
  !insertmacro PakoHideItem 1256
  !insertmacro PakoHideItem 1034
  !insertmacro PakoHideItem 1035
  !insertmacro PakoHideItem 1036
  !insertmacro PakoHideItem 1037
  !insertmacro PakoHideItem 1038
  !insertmacro PakoHideItem 1039
  !insertmacro PakoHideItem 1044
  !insertmacro PakoHideItem 1045
  ; Pages fill the whole inside.
  GetDlgItem $2 $HWNDPARENT 1018
  System::Call "user32::SetWindowPos(p r2, p 0, i 0, i 0, i r0, i r1, i 0x14)"
  ; The title bar in Pako's colours (Windows 11; Windows 10 shows its own).
  ${If} $PakoLook == "d"
    System::Call "*(i 1) p .r2"
  ${Else}
    System::Call "*(i 0) p .r2"
  ${EndIf}
  System::Call "dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 20, p r2, i 4)"
  System::Free $2
  ; COLORREF is 0x00BBGGRR.
  StrCpy $PakoA1 $PakoCBg
  Call ${UN}PakoColorRef
  System::Call "*(i $PakoRet) p .r2"
  System::Call "dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 35, p r2, i 4)"
  System::Free $2
  StrCpy $PakoA1 $PakoCText
  Call ${UN}PakoColorRef
  System::Call "*(i $PakoRet) p .r2"
  System::Call "dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 36, p r2, i 4)"
  System::Free $2
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $PakoA1 = 0xRRGGBB -> $PakoRet = 0x00BBGGRR (a Windows COLORREF).
Function ${UN}PakoColorRef
  Push $1
  Push $2
  IntOp $1 $PakoA1 & 0xff
  IntOp $1 $1 << 16
  IntOp $2 $PakoA1 & 0xff00
  IntOp $1 $1 | $2
  IntOp $2 $PakoA1 >> 16
  IntOp $2 $2 & 0xff
  IntOp $PakoRet $1 | $2
  Pop $2
  Pop $1
FunctionEnd

; $PakoA1 = a control -> $PakoRet = 1 if the mouse is on it, else 0.
Function ${UN}PakoMouseOn
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  StrCpy $PakoRet 0
  ${If} $PakoA1 <> 0
    System::Call "*(i 0, i 0) p .r1"
    System::Call "user32::GetCursorPos(p r1)"
    System::Call "*$1(i .r2, i .r3)"
    System::Free $1
    System::Call "*(i 0, i 0, i 0, i 0) p .r1"
    System::Call "user32::GetWindowRect(p $PakoA1, p r1)"
    System::Call "*$1(i .r4, i .r5, i .r6, i .r7)"
    System::Free $1
    ${If} $2 >= $4
    ${AndIf} $2 < $6
    ${AndIf} $3 >= $5
    ${AndIf} $3 < $7
      ; [Pako] And it's really Setup's window under the mouse: with another window over Setup
      ; (or Setup behind it), the mouse resting at that place on the screen doesn't count. It
      ; once kept the slides from ever moving on.
      System::Int64Op $3 << 32
      Pop $1
      System::Int64Op $2 & 0xFFFFFFFF
      Pop $4
      System::Int64Op $1 | $4
      Pop $1
      System::Call "user32::WindowFromPoint(l r1) p .r4"
      System::Call "user32::GetAncestor(p r4, i 2) p .r4" ; GA_ROOT
      ${If} $4 = $HWNDPARENT
        StrCpy $PakoRet 1
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
FunctionEnd

; The page's buttons: the picture for their state (n, h(over), x = can't be clicked).
Function ${UN}PakoButtonsTick
  Push $0
  Push $1
  ${If} $PakoB1Ctl <> 0
    StrCpy $1 "n"
    ${If} $PakoHeld = 1
    ${AndIf} $PakoB1X <> 0
      StrCpy $1 "x"
    ${Else}
      StrCpy $PakoA1 $PakoB1Ctl
      Call ${UN}PakoMouseOn
      ${IfThen} $PakoRet = 1 ${|} StrCpy $1 "h" ${|}
    ${EndIf}
    ${If} $1 != $PakoB1Now
      StrCpy $PakoB1Now $1
      ${If} $1 == "h"
        SendMessage $PakoB1Ctl ${STM_SETIMAGE} 0 $PakoB1H
      ${ElseIf} $1 == "x"
        SendMessage $PakoB1Ctl ${STM_SETIMAGE} 0 $PakoB1X
      ${Else}
        SendMessage $PakoB1Ctl ${STM_SETIMAGE} 0 $PakoB1N
      ${EndIf}
    ${EndIf}
  ${EndIf}
  ${If} $PakoB2Ctl <> 0
    StrCpy $PakoA1 $PakoB2Ctl
    Call ${UN}PakoMouseOn
    StrCpy $1 "n"
    ${IfThen} $PakoRet = 1 ${|} StrCpy $1 "h" ${|}
    ${If} $1 != $PakoB2Now
      StrCpy $PakoB2Now $1
      ${If} $1 == "h"
        SendMessage $PakoB2Ctl ${STM_SETIMAGE} 0 $PakoB2H
      ${Else}
        SendMessage $PakoB2Ctl ${STM_SETIMAGE} 0 $PakoB2N
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Call ${UN}PakoFocusTick
  Pop $1
  Pop $0
FunctionEnd

; The focus ring: around the stand-in (PakoProxy) the keyboard is on, 3 px out, 2 px thick.
; Only once a key was used (Windows hides focus until then, as in its own windows).
Function ${UN}PakoFocusTick
  ${If} $PakoRing1 = 0
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  System::Call "user32::GetFocus() p .r0"
  System::Call "user32::GetParent(p r0) p .r1"
  ${If} $1 != $PakoDialog
    StrCpy $0 0
  ${EndIf}
  SendMessage $PakoDialog 0x129 0 0 $1 ; WM_QUERYUISTATE
  IntOp $1 $1 & 1                      ; UISF_HIDEFOCUS
  ${IfThen} $1 <> 0 ${|} StrCpy $0 0 ${|}
  ${If} $0 != $PakoRingOn
    StrCpy $PakoRingOn $0
    ${If} $0 = 0
      ShowWindow $PakoRing1 ${SW_HIDE}
      ShowWindow $PakoRing2 ${SW_HIDE}
      ShowWindow $PakoRing3 ${SW_HIDE}
      ShowWindow $PakoRing4 ${SW_HIDE}
    ${Else}
      System::Call "*(i 0, i 0, i 0, i 0) p .r1"
      System::Call "user32::GetWindowRect(p r0, p r1)"
      System::Call "user32::MapWindowPoints(p 0, p $PakoDialog, p r1, i 2)"
      System::Call "*$1(i .r2, i .r3, i .r4, i .r5)"
      System::Free $1
      !insertmacro PakoScale $6 3 ; gap
      !insertmacro PakoScale $7 2 ; thickness
      ; Outer box: left $2, top $3, right $4, bottom $5.
      IntOp $2 $2 - $6
      IntOp $2 $2 - $7
      IntOp $3 $3 - $6
      IntOp $3 $3 - $7
      IntOp $4 $4 + $6
      IntOp $4 $4 + $7
      IntOp $5 $5 + $6
      IntOp $5 $5 + $7
      IntOp $8 $4 - $2 ; width
      IntOp $1 $5 - $3 ; height
      System::Call "user32::SetWindowPos(p $PakoRing1, p 0, i r2, i r3, i r8, i r7, i 0x50)"
      IntOp $6 $5 - $7
      System::Call "user32::SetWindowPos(p $PakoRing2, p 0, i r2, i r6, i r8, i r7, i 0x50)"
      System::Call "user32::SetWindowPos(p $PakoRing3, p 0, i r2, i r3, i r7, i r1, i 0x50)"
      IntOp $6 $4 - $7
      System::Call "user32::SetWindowPos(p $PakoRing4, p 0, i r6, i r3, i r7, i r1, i 0x50)"
    ${EndIf}
  ${EndIf}
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $PakoA1 = a button's name (art/b-<name>-<state>-<look>[-fr].png: its words are in the picture,
; in the language shown), $PakoA2 = its width at 100 % -> its pictures in $PakoB1N/H/X
; (PakoLoadButton2: $PakoB2N/H).
Function ${UN}PakoLoadButton1
  Push $0
  Push $1
  StrCpy $0 $PakoA1
  StrCpy $1 $PakoA2
  StrCpy $PakoA3 40
  StrCpy $PakoA1 "b-$0-n-$PakoLook$PakoArtSfx.png"
  Call ${UN}PakoImage
  StrCpy $PakoB1N $PakoRet
  StrCpy $PakoA1 "b-$0-h-$PakoLook$PakoArtSfx.png"
  StrCpy $PakoA2 $1
  Call ${UN}PakoImage
  StrCpy $PakoB1H $PakoRet
  StrCpy $PakoB1X 0
  ${If} ${FileExists} "$PLUGINSDIR\art\b-$0-x-$PakoLook$PakoArtSfx.png"
    StrCpy $PakoA1 "b-$0-x-$PakoLook$PakoArtSfx.png"
    StrCpy $PakoA2 $1
    Call ${UN}PakoImage
    StrCpy $PakoB1X $PakoRet
  ${EndIf}
  StrCpy $PakoB1Now ""
  Pop $1
  Pop $0
FunctionEnd

Function ${UN}PakoLoadButton2
  Push $0
  Push $1
  StrCpy $0 $PakoA1
  StrCpy $1 $PakoA2
  StrCpy $PakoA3 40
  StrCpy $PakoA1 "b-$0-n-$PakoLook$PakoArtSfx.png"
  Call ${UN}PakoImage
  StrCpy $PakoB2N $PakoRet
  StrCpy $PakoA1 "b-$0-h-$PakoLook$PakoArtSfx.png"
  StrCpy $PakoA2 $1
  Call ${UN}PakoImage
  StrCpy $PakoB2H $PakoRet
  StrCpy $PakoB2Now ""
  Pop $1
  Pop $0
FunctionEnd

; $PakoA1 = a font, $PakoA2 = a text -> $PakoRet = its width in pixels.
Function ${UN}PakoTextWidth
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  ; (The text goes to System::Call in a register: a comma in it would split its arguments.)
  StrCpy $1 $PakoA2
  System::Call "user32::GetDC(p 0) p .r2"
  System::Call "gdi32::SelectObject(p r2, p $PakoA1) p .r3"
  StrLen $4 $1
  System::Call "*(i 0, i 0) p .r0"
  System::Call "gdi32::GetTextExtentPoint32W(p r2, w r1, i r4, p r0)"
  System::Call "*$0(i .r4, i)"
  System::Free $0
  System::Call "gdi32::SelectObject(p r2, p r3)"
  System::Call "user32::ReleaseDC(p 0, p r2)"
  StrCpy $PakoRet $4
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; The look of a progress page (Tauri's, Windows' own): its bar and line of text moved and
; coloured, its list hidden. $0 = the page. X, W (at 100 %): where the bar goes; Y: its top.
Function ${UN}PakoStyleProgress
  ; $0 page, $1 x, $2 y, $3 w (at 100 %)
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  GetDlgItem $4 $0 1016
  ShowWindow $4 ${SW_HIDE}
  GetDlgItem $4 $0 1027
  ShowWindow $4 ${SW_HIDE}
  ; The bar: flat, 6 px, Pako's coral on the field colour.
  GetDlgItem $4 $0 1004
  System::Call 'uxtheme::SetWindowTheme(p r4, w " ", w " ")'
  System::Call "user32::GetWindowLongW(p r4, i -16) i .r5"
  IntOp $5 $5 & 0xFF7FFFFF ; no WS_BORDER
  IntOp $5 $5 | 0x1        ; PBS_SMOOTH
  System::Call "user32::SetWindowLongW(p r4, i -16, i r5)"
  System::Call "user32::SetWindowLongW(p r4, i -20, i 0)"
  StrCpy $PakoA1 $PakoCRa
  Call ${UN}PakoColorRef
  SendMessage $4 0x2001 0 $PakoRet ; PBM_SETBKCOLOR
  StrCpy $PakoA1 0xf96263
  Call ${UN}PakoColorRef
  SendMessage $4 0x409 0 $PakoRet ; PBM_SETBARCOLOR
  !insertmacro PakoScale $5 $1
  !insertmacro PakoScale $6 $2
  !insertmacro PakoScale $7 $3
  !insertmacro PakoScale $8 6
  System::Call "user32::SetWindowPos(p r4, p 0, i r5, i r6, i r7, i r8, i 0x34)"
  ; The line under it ("Copying Pako.exe... 40%", or what went wrong).
  GetDlgItem $4 $0 1006
  IntOp $6 $2 + 16
  !insertmacro PakoScale $6 $6
  !insertmacro PakoScale $8 36
  System::Call "user32::SetWindowPos(p r4, p 0, i r5, i r6, i r7, i r8, i 0x14)"
  SendMessage $4 ${WM_SETFONT} $PakoFSmall 1
  !insertmacro PakoColors $4 MUT BG
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
FunctionEnd

; $0 = a progress page (Windows' own): it fills the window ($PakoA1 x $PakoA2 at 100 %) and gets
; Pako's background (a label under everything: that page doesn't take colours itself).
Function ${UN}PakoPageFill
  Push $1
  Push $2
  !insertmacro PakoScale $1 $PakoA1
  !insertmacro PakoScale $2 $PakoA2
  System::Call "user32::SetWindowPos(p r0, p 0, i 0, i 0, i r1, i r2, i 0x16)"
  ; (WS_CLIPSIBLINGS: it never paints over the bar and the words on it.)
  !insertmacro PakoRaw $1 $0 STATIC 0x54000080 0 0 $PakoA1 $PakoA2 ""
  !insertmacro PakoColors $1 BG BG
  System::Call "user32::SetWindowPos(p r1, p 1, i 0, i 0, i 0, i 0, i 0x13)" ; to the bottom
  Pop $2
  Pop $1
FunctionEnd

; The progress page's Close, ready and hidden ($0 = the page; $1, $2, $3 = x, y, w at 100 %;
; $PakoFailBmp its picture): Windows' own Cancel, moved there, under Pako's picture of Close,
; which the mouse goes through. Shown by PakoFail when something goes wrong (Windows enables
; Cancel then; clicking it ends Setup, as do Esc and the window's ✕).
Function ${UN}PakoFailClose
  Push $4
  Push $5
  ; The page paints around them, never over them.
  System::Call "user32::GetWindowLongW(p r0, i -16) i .r4"
  IntOp $4 $4 | 0x04000000 ; WS_CLIPSIBLINGS
  System::Call "user32::SetWindowLongW(p r0, i -16, i r4)"
  GetDlgItem $PakoFailBtn $HWNDPARENT 2
  System::Call "user32::GetWindowLongW(p $PakoFailBtn, i -16) i .r4"
  IntOp $4 $4 | 0x04000000
  System::Call "user32::SetWindowLongW(p $PakoFailBtn, i -16, i r4)"
  !insertmacro PakoT $PakoTx pakoClose
  SendMessage $PakoFailBtn ${WM_SETTEXT} 0 "STR:$PakoTx"
  !insertmacro PakoScale $4 $1
  !insertmacro PakoScale $5 $2
  !insertmacro PakoScale $PakoW $3
  !insertmacro PakoScale $PakoH 40
  System::Call "user32::SetWindowPos(p $PakoFailBtn, p 0, i r4, i r5, i $PakoW, i $PakoH, i 0x10)"
  !insertmacro PakoRaw $PakoFailPic $HWNDPARENT STATIC ${PAKO_PIC_PASS} $1 $2 $3 40 ""
  SendMessage $PakoFailPic ${STM_SETIMAGE} 0 $PakoFailBmp
  System::Call "user32::SetWindowPos(p $PakoFailPic, p 0, i 0, i 0, i 0, i 0, i 0x13)"
  ; Windows shows its Back button again when something went wrong: none to see (no size).
  GetDlgItem $4 $HWNDPARENT 3
  System::Call "user32::SetWindowPos(p r4, p 0, i 0, i 0, i 0, i 0, i 0x16)"
  Pop $5
  Pop $4
FunctionEnd

; Tauri's Back, Next and Cancel stay hidden on every page (Windows shows Back again on each one);
; they keep working from the keyboard and from Pako's own buttons.
Function ${UN}PakoHideNav
  Push $2
  !insertmacro PakoHideItem 1
  !insertmacro PakoHideItem 2
  !insertmacro PakoHideItem 3
  ; The focus ring waits for the keyboard on each page (Tab shows it, as in Windows' own
  ; windows): WM_CHANGEUISTATE, UIS_SET UISF_HIDEFOCUS.
  SendMessage $HWNDPARENT 0x127 0x10001 0
  Pop $2
FunctionEnd

; ---------- the switches slide between off and on (template 1.1.5) ----------
; The knob moves and the colour eases, both ways, in 180 ms (eased out); a switch flipped again
; on the way turns back from where it is. With Windows' "Animation effects" off it changes at
; once. Each step is drawn into the switch's own picture and straight onto the screen (as the
; slides are), never through the control's own painting: nothing flashes.

; Once, when the look is known: the steps' pictures, each switch's own picture, two drawings.
Function ${UN}PakoSwLoad
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  StrCpy $PakoA2 32
  StrCpy $PakoA3 18
  StrCpy $PakoA1 "on-$PakoLook.png"
  Call ${UN}PakoImage
  StrCpy $PakoBmpOn $PakoRet
  StrCpy $PakoA1 "off-$PakoLook.png"
  Call ${UN}PakoImage
  StrCpy $PakoBmpOff $PakoRet
  !insertmacro PakoSwRead "${UN}" 1
  !insertmacro PakoSwRead "${UN}" 2
  !insertmacro PakoSwRead "${UN}" 3
  !insertmacro PakoSwRead "${UN}" 4
  !insertmacro PakoSwRead "${UN}" 5
  !insertmacro PakoSwRead "${UN}" 6
  !insertmacro PakoSwRead "${UN}" 7
  !insertmacro PakoSwRead "${UN}" 8
  !insertmacro PakoSwRead "${UN}" 9
  !insertmacro PakoScale $PakoSwW 32
  !insertmacro PakoScale $PakoSwH 18
  System::Call "user32::GetDC(p 0) p .r0"
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $PakoSwDcA $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $PakoSwDcB $1
  System::Call "user32::ReleaseDC(p 0, p r0)"
  ; Each switch's own picture: 24-bit, the switch's size on screen (the picture's handle in $3,
  ; its pixels' address in $4).
  IntOp $1 0 - $PakoSwH
  System::Call "*(i 40, i $PakoSwW, i r1, &i2 1, &i2 24, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r2"
  System::Call "gdi32::CreateDIBSection(p 0, p r2, i 0, *p .r4, p 0, i 0) p .r3"
  StrCpy $PakoSwFace1 $3
  System::Call "gdi32::CreateDIBSection(p 0, p r2, i 0, *p .r4, p 0, i 0) p .r3"
  StrCpy $PakoSwFace2 $3
  System::Call "gdi32::CreateDIBSection(p 0, p r2, i 0, *p .r4, p 0, i 0) p .r3"
  StrCpy $PakoSwFace3 $3
  System::Free $2
  StrCpy $PakoSwDesk 0
  StrCpy $PakoSwStart 0
  StrCpy $PakoSwBrow 0
  ; All off until a page says otherwise (PakoSwSet).
  StrCpy $PakoA2 0
  StrCpy $PakoA1 1
  Call ${UN}PakoSwSet
  StrCpy $PakoA1 2
  Call ${UN}PakoSwSet
  StrCpy $PakoA1 3
  Call ${UN}PakoSwSet
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $PakoA3 = a step (0 = off to 10 = on) -> $PakoRet = its picture.
Function ${UN}PakoSwFrame
  ${If} $PakoA3 <= 0
    StrCpy $PakoRet $PakoBmpOff
  ${ElseIf} $PakoA3 >= 10
    StrCpy $PakoRet $PakoBmpOn
  ${ElseIf} $PakoA3 = 1
    StrCpy $PakoRet $PakoSwB1
  ${ElseIf} $PakoA3 = 2
    StrCpy $PakoRet $PakoSwB2
  ${ElseIf} $PakoA3 = 3
    StrCpy $PakoRet $PakoSwB3
  ${ElseIf} $PakoA3 = 4
    StrCpy $PakoRet $PakoSwB4
  ${ElseIf} $PakoA3 = 5
    StrCpy $PakoRet $PakoSwB5
  ${ElseIf} $PakoA3 = 6
    StrCpy $PakoRet $PakoSwB6
  ${ElseIf} $PakoA3 = 7
    StrCpy $PakoRet $PakoSwB7
  ${ElseIf} $PakoA3 = 8
    StrCpy $PakoRet $PakoSwB8
  ${Else}
    StrCpy $PakoRet $PakoSwB9
  ${EndIf}
  ; A step that couldn't be read: the nearer end.
  ${If} $PakoRet = 0
    ${If} $PakoA3 < 5
      StrCpy $PakoRet $PakoBmpOff
    ${Else}
      StrCpy $PakoRet $PakoBmpOn
    ${EndIf}
  ${EndIf}
FunctionEnd

; $PakoA1 = a switch (1 to 3): it shows the step nearest to where it is ($PakoSwP), unless that step
; shows already; $PakoA2 = 1: even then (a switch just made).
Function ${UN}PakoSwDraw
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  ${If} $PakoA1 = 1
    StrCpy $0 $PakoSwP1
    StrCpy $1 $PakoSwK1
    StrCpy $2 $PakoSwDesk
    StrCpy $3 $PakoSwFace1
  ${ElseIf} $PakoA1 = 2
    StrCpy $0 $PakoSwP2
    StrCpy $1 $PakoSwK2
    StrCpy $2 $PakoSwStart
    StrCpy $3 $PakoSwFace2
  ${Else}
    StrCpy $0 $PakoSwP3
    StrCpy $1 $PakoSwK3
    StrCpy $2 $PakoSwBrow
    StrCpy $3 $PakoSwFace3
  ${EndIf}
  IntOp $0 $0 * 10
  IntOp $0 $0 + 500
  IntOp $0 $0 / 1000
  ${If} "$0" != "$1"
  ${OrIf} $PakoA2 = 1
    ${If} $PakoA1 = 1
      StrCpy $PakoSwK1 $0
    ${ElseIf} $PakoA1 = 2
      StrCpy $PakoSwK2 $0
    ${Else}
      StrCpy $PakoSwK3 $0
    ${EndIf}
    StrCpy $PakoA3 $0
    Call ${UN}PakoSwFrame
    StrCpy $1 1
    ; A picture that couldn't be made: nothing is drawn (never a black switch).
    ${If} $PakoRet = 0
    ${OrIf} $3 = 0
      StrCpy $1 0
    ${EndIf}
  ${Else}
    StrCpy $1 0
  ${EndIf}
  ${If} $1 = 1
    ; Into the switch's own picture, which is held only for the copy (a picture control can't
    ; draw one that is held somewhere else)...
    System::Call "gdi32::SelectObject(p $PakoSwDcA, p $PakoRet) p .r4"
    System::Call "gdi32::SelectObject(p $PakoSwDcB, p r3) p .r5"
    System::Call "gdi32::BitBlt(p $PakoSwDcB, i 0, i 0, i $PakoSwW, i $PakoSwH, p $PakoSwDcA, i 0, i 0, i 0x00CC0020)"
    ; ...and straight onto the screen in one copy, when it shows: the control isn't asked to
    ; paint it again (DCX_CACHE | DCX_CLIPSIBLINGS: the focus ring stays).
    ${If} $2 != ""
    ${AndIf} $2 <> 0
      System::Call "user32::IsWindowVisible(p r2) i .r0"
      ${If} $0 <> 0
        System::Call "user32::GetDCEx(p r2, p 0, i 0x12) p .r0"
        System::Call "gdi32::BitBlt(p r0, i 0, i 0, i $PakoSwW, i $PakoSwH, p $PakoSwDcB, i 0, i 0, i 0x00CC0020)"
        System::Call "user32::ReleaseDC(p r2, p r0)"
        System::Call "user32::ValidateRect(p r2, p 0)"
      ${EndIf}
    ${EndIf}
    System::Call "gdi32::SelectObject(p $PakoSwDcA, p r4)"
    System::Call "gdi32::SelectObject(p $PakoSwDcB, p r5)"
  ${EndIf}
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $PakoA1 = a switch (1 to 3), $PakoA2 = on (1) or off (0): it shows that at once (a new page).
Function ${UN}PakoSwSet
  Push $0
  IntOp $0 $PakoA2 * 1000
  ${If} $PakoA1 = 1
    StrCpy $PakoSwP1 $0
    StrCpy $PakoSwG1 $0
    StrCpy $PakoSwA1 $0
  ${ElseIf} $PakoA1 = 2
    StrCpy $PakoSwP2 $0
    StrCpy $PakoSwG2 $0
    StrCpy $PakoSwA2 $0
  ${Else}
    StrCpy $PakoSwP3 $0
    StrCpy $PakoSwG3 $0
    StrCpy $PakoSwA3 $0
  ${EndIf}
  Push $PakoA2
  StrCpy $PakoA2 1
  Call ${UN}PakoSwDraw
  Pop $PakoA2
  Pop $0
FunctionEnd

; $PakoA1 = a switch (1 to 3), $PakoA2 = on (1) or off (0): it slides there from where it is (so
; one flipped again on the way turns back), or is there at once with Windows' "Animation
; effects" off.
Function ${UN}PakoSwTo
  Push $0
  Push $1
  IntOp $0 $PakoA2 * 1000
  System::Call "kernel32::GetTickCount() i .r1"
  ${If} $PakoA1 = 1
    StrCpy $PakoSwG1 $0
    StrCpy $PakoSwA1 $PakoSwP1
    StrCpy $PakoSwT1 $1
  ${ElseIf} $PakoA1 = 2
    StrCpy $PakoSwG2 $0
    StrCpy $PakoSwA2 $PakoSwP2
    StrCpy $PakoSwT2 $1
  ${Else}
    StrCpy $PakoSwG3 $0
    StrCpy $PakoSwA3 $PakoSwP3
    StrCpy $PakoSwT3 $1
  ${EndIf}
  StrCpy $0 1
  System::Call "user32::SystemParametersInfoW(i 0x1042, i 0, *i .r0, i 0)" ; SPI_GETCLIENTAREAANIMATION
  ${If} $0 = 0
    Call ${UN}PakoSwSet
  ${Else}
    ${NSD_CreateTimer} ${UN}PakoSwTick 10
  ${EndIf}
  Pop $1
  Pop $0
FunctionEnd

; One step of the switches on the move (as often as Windows' timer allows), timed by the clock:
; a busy PC skips steps instead of slowing them down. Stops when all have arrived.
Function ${UN}PakoSwTick
  Push $PakoA1
  Push $PakoA2
  Push $PakoA3
  Push $PakoRet
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  System::Call "kernel32::GetTickCount() i .r0"
  !insertmacro PakoSwStep "${UN}" 1
  !insertmacro PakoSwStep "${UN}" 2
  !insertmacro PakoSwStep "${UN}" 3
  ${If} $PakoSwP1 = $PakoSwG1
  ${AndIf} $PakoSwP2 = $PakoSwG2
  ${AndIf} $PakoSwP3 = $PakoSwG3
    ${NSD_KillTimer} ${UN}PakoSwTick
  ${EndIf}
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
  Pop $PakoRet
  Pop $PakoA3
  Pop $PakoA2
  Pop $PakoA1
FunctionEnd

; The page with the switches is left: they stop where they're going, and their controls are gone.
Function ${UN}PakoSwLeave
  ${NSD_KillTimer} ${UN}PakoSwTick
  StrCpy $PakoSwDesk 0
  StrCpy $PakoSwStart 0
  StrCpy $PakoSwBrow 0
  StrCpy $PakoSwP1 $PakoSwG1
  StrCpy $PakoSwP2 $PakoSwG2
  StrCpy $PakoSwP3 $PakoSwG3
FunctionEnd

!macroend

!macro PakoHideItem ID
  GetDlgItem $2 $HWNDPARENT ${ID}
  ShowWindow $2 ${SW_HIDE}
!macroend

; The switches' slide: a whole one takes this long (ms), part of one (turning back) its share.
!define PAKO_SW_MS 180

; One in-between step of the switches (art/sw<N>-<look>.png) -> $PakoSwB<N>.
!macro PakoSwRead UN N
  StrCpy $PakoA1 "sw${N}-$PakoLook.png"
  Call ${UN}PakoImage
  StrCpy $PakoSwB${N} $PakoRet
!macroend

; PakoSwTick for switch N ($0 = now, ms): where it is by now, eased out (cubic).
!macro PakoSwStep UN N
  ${If} $PakoSwP${N} <> $PakoSwG${N}
    IntOp $1 $0 - $PakoSwT${N}
    IntOp $2 $PakoSwG${N} - $PakoSwA${N}
    ; The time for this way: its share of the whole slide's, at least 60 ms.
    IntOp $3 $2 * ${PAKO_SW_MS}
    ${IfThen} $3 < 0 ${|} IntOp $3 0 - $3 ${|}
    IntOp $3 $3 / 1000
    ${IfThen} $3 < 60 ${|} StrCpy $3 60 ${|}
    ${If} $1 >= $3
    ${OrIf} $1 < 0
      StrCpy $PakoSwP${N} $PakoSwG${N}
    ${Else}
      ; A + (G - A) × (1 - (1 - t)³), t = $1 / $3, in thousandths.
      IntOp $4 $1 * 1000
      IntOp $4 $4 / $3
      IntOp $4 1000 - $4
      IntOp $5 $4 * $4
      IntOp $5 $5 / 1000
      IntOp $5 $5 * $4
      IntOp $5 $5 / 1000
      IntOp $5 1000 - $5
      IntOp $5 $5 * $2
      IntOp $5 $5 / 1000
      IntOp $PakoSwP${N} $PakoSwA${N} + $5
    ${EndIf}
    StrCpy $PakoA1 ${N}
    StrCpy $PakoA2 0
    Call ${UN}PakoSwDraw
  ${EndIf}
!macroend

; One slide's picture: N (1..5), SFX ("" or "-fr") -> VAR.
!macro PakoSlideRead N SFX VAR
  StrCpy $PakoA1 "slide${N}-$PakoLook${SFX}.jpg"
  StrCpy $PakoA2 530
  StrCpy $PakoA3 540
  Call PakoImage
  StrCpy ${VAR} $PakoRet
!macroend

; ---------- the installer's pages ----------
!macro PAKO_UI_FUNCTIONS
!ifndef PakoEn_pakoInstallIn
  !error 'English.nsh is from template 1.0.x (plain LangStrings): write each line as !insertmacro PakoEn <name> "<text>" (see the installer template's CHANGELOG, Template 1.1.0: Updating an app from 1.0.x).'
!endif
!insertmacro PAKO_UI_COMMON ""
!insertmacro PAKO_UI_COMMON "un."

!ifndef PAKO_FRENCH_TEXTS
  !error 'Pako Setup speaks French but French.nsh is not included: tauri.conf.json needs "languages": ["English", "French"] and "customLanguageFiles": { "English": "installer/English.nsh", "French": "installer/French.nsh" }.'
!endif

; [Pako] NSIS's own words that only read the language Setup started in (English.nsh: ^Extract),
; in the language shown.
Function PakoLangTexts
  !insertmacro PakoT $PakoTxExtract pakoExtract
FunctionEnd

; [Pako] Reads the next French slide (PakoTick: one at a time while the window is idle, so the
; first switch to French shows its slides at once).
Function PakoReadFrSlide
  ${If} $PakoSFReady == ""
    StrCpy $PakoSFReady 0
  ${EndIf}
  ${If} $PakoSFReady >= 5
    Return
  ${EndIf}
  IntOp $PakoSFReady $PakoSFReady + 1
  ${If} $PakoSFReady = 1
    !insertmacro PakoSlideRead 1 "-fr" $PakoSF1
  ${ElseIf} $PakoSFReady = 2
    !insertmacro PakoSlideRead 2 "-fr" $PakoSF2
  ${ElseIf} $PakoSFReady = 3
    !insertmacro PakoSlideRead 3 "-fr" $PakoSF3
  ${ElseIf} $PakoSFReady = 4
    !insertmacro PakoSlideRead 4 "-fr" $PakoSF4
  ${Else}
    !insertmacro PakoSlideRead 5 "-fr" $PakoSF5
  ${EndIf}
FunctionEnd

; [Pako] installer.nsi (PageLeaveReinstall): the installed version's uninstaller failed.
Function PakoUnableToUninstall
  !insertmacro PakoT $PakoTx unableToUninstall
  MessageBox MB_ICONEXCLAMATION $PakoTx
FunctionEnd

; [Pako] $PakoS1..5 = the slides of the language shown.
Function PakoUseSlides
  ${If} $PakoLang == "fr"
    ${Do}
      ${IfThen} $PakoSFReady >= 5 ${|} ${Break} ${|}
      Call PakoReadFrSlide
    ${Loop}
    StrCpy $PakoS1 $PakoSF1
    StrCpy $PakoS2 $PakoSF2
    StrCpy $PakoS3 $PakoSF3
    StrCpy $PakoS4 $PakoSF4
    StrCpy $PakoS5 $PakoSF5
    Return
  ${EndIf}
  StrCpy $PakoS1 $PakoSE1
  StrCpy $PakoS2 $PakoSE2
  StrCpy $PakoS3 $PakoSE3
  StrCpy $PakoS4 $PakoSE4
  StrCpy $PakoS5 $PakoSE5
FunctionEnd

; MUI_CUSTOMFUNCTION_GUIINIT (hooks.nsh): the window, before it shows.
Function PakoGuiInit
  Call PakoInstallDir
  StrCpy $PakoPassiveOrig $PassiveMode
  ${If} $PassiveMode = 1
    Return
  ${EndIf}
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\art"
  File "${PAKO_ART}\*.jpg"
  File "${PAKO_ART}\*.png"
  ; [Pako] Out of the pictures' folder again: SetOutPath also made it Setup's working folder, and
  ; Windows can't delete a working folder, so NSIS's own clean-up at exit left it behind, empty, in %TEMP%.
  SetOutPath "$TEMP"
  Call PakoLookInit
  StrCpy $PakoA1 860
  StrCpy $PakoA2 540
  Call PakoWindow
  Call PakoDetect
  ; [Pako] Over an installed Pako (update, reinstall, an older version): the window opens in the
  ; language saved for it ("Installer Language": the one Setup last ran in, which the app
  ; updates when its own language changes, app-side/i18n.rs tell_setup). A first install opens
  ; in English. Either way the choice in the window still works, and a pick counts as a pick.
  ${If} $PakoMode != "install"
    ReadRegStr $PakoTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
    ${If} $PakoTx == ${LANG_FRENCH}
      StrCpy $PakoLang "fr"
      StrCpy $LANGUAGE ${LANG_FRENCH}
      StrCpy $PakoArtSfx "-fr"
      Call PakoLangTexts
    ${EndIf}
  ${EndIf}
  ; The slides, once for every page (English ones; the French ones are read while the window is
  ; idle, PakoTick, or at once when it opens in French: PakoUseSlides).
  !insertmacro PakoSlideRead 1 "" $PakoSE1
  !insertmacro PakoSlideRead 2 "" $PakoSE2
  !insertmacro PakoSlideRead 3 "" $PakoSE3
  !insertmacro PakoSlideRead 4 "" $PakoSE4
  !insertmacro PakoSlideRead 5 "" $PakoSE5
  Call PakoUseSlides
  StrCpy $PakoA1 "logo-$PakoLook.png"
  StrCpy $PakoA2 160
  StrCpy $PakoA3 44
  Call PakoImage
  StrCpy $PakoBmpLogo $PakoRet
  Call PakoSwLoad
  StrCpy $PakoA1 "field-$PakoLook.png"
  StrCpy $PakoA2 282
  StrCpy $PakoA3 32
  Call PakoImage
  StrCpy $PakoBmpField $PakoRet
  StrCpy $PakoSlide 1
  StrCpy $PakoAuto 1
  StrCpy $PakoDir 1
  StrCpy $PakoIntro 1
FunctionEnd

; The window's title on every page (Windows sets "Pako Setup" again on each one).
Function PakoSetTitle
  ${If} $PakoMode == "update"
    !insertmacro PakoT $PakoTx pakoUpdateTitle
    SendMessage $HWNDPARENT ${WM_SETTEXT} 0 "STR:$PakoTx"
  ${Else}
    ; [Pako] NSIS's own title ("<App> Setup") is in the language Setup started in.
    !insertmacro PakoT $PakoTx pakoSetupTitle
    SendMessage $HWNDPARENT ${WM_SETTEXT} 0 "STR:$PakoTx"
  ${EndIf}
FunctionEnd

; The line under the logo for what this Setup does -> $PakoRet.
Function PakoModeTag
  ${If} $PakoMode == "update"
    !insertmacro PakoT $PakoRet pakoTagUpdate
  ${ElseIf} $PakoMode == "same"
    !insertmacro PakoT $PakoRet pakoTagSame
  ${ElseIf} $PakoMode == "older"
    !insertmacro PakoT $PakoRet pakoTagOlder
  ${Else}
    !insertmacro PakoT $PakoRet pakoTag
  ${EndIf}
FunctionEnd

; What is installed already (the same as Tauri's reinstall page worked out): $PakoMode,
; $PakoOldVersion, $PakoCmp, and the switches as they are for it.
Function PakoDetect
  Push $0
  Push $R0
  Push $R1
  Push $R6
  StrCpy $PakoMode "install"
  StrCpy $PakoOldVersion ""
  StrCpy $PakoDesktop 1
  ; [Pako] A new install starts with Windows unless it's switched off (user, 2026-11-21); over an
  ; installed Pako the switch shows what it has now (below).
  StrCpy $PakoStart 1
  StrCpy $PakoBrowsers 1
  Call FindWixInstall
  StrCpy $PakoR6 $R6
  ${If} $R6 != ""
    StrCpy $WixMode 1
    ReadRegStr $PakoOldVersion HKLM "$R6" "DisplayVersion"
  ${Else}
    ReadRegStr $R0 SHCTX "${UNINSTKEY}" ""
    ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
    ${If} "$R0$R1" == ""
      ; [Pako] A portable Pako that already starts with Windows keeps doing so: Windows has one
      ; "Pako" start-up entry, and a new install switched on would take it over. Its entry is
      ; "<exe in quotes>" [--tray] (an unquoted exe then args is read too, as in the template);
      ; one whose Pako is gone doesn't count.
      ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
      StrCpy $R1 $0 1
      ${If} $R1 == '"'
        StrCpy $R0 1
        ${Do}
          StrCpy $R1 $0 1 $R0
          ${IfThen} $R1 == '"' ${|} ${Break} ${|}
          ${IfThen} $R1 == "" ${|} ${Break} ${|}
          IntOp $R0 $R0 + 1
        ${Loop}
        IntOp $R0 $R0 - 1
        StrCpy $R1 $0 $R0 1
        ${If} ${FileExists} "$R1"
          StrCpy $PakoStart 0
        ${EndIf}
      ${ElseIf} $0 != ""
        ; Unquoted: the exe is everything up to the first ".exe".
        StrCpy $R0 0
        ${Do}
          StrCpy $R1 $0 4 $R0
          ${IfThen} $R1 == ".exe" ${|} ${Break} ${|}
          ${IfThen} $R1 == "" ${|} ${Break} ${|}
          IntOp $R0 $R0 + 1
        ${Loop}
        ${If} $R1 == ".exe"
          IntOp $R0 $R0 + 4
          StrCpy $R1 $0 $R0
          ${If} ${FileExists} "$R1"
            StrCpy $PakoStart 0
          ${EndIf}
        ${EndIf}
      ${EndIf}
      Goto pako_detect_done
    ${EndIf}
    ReadRegStr $PakoOldVersion SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${EndIf}
  nsis_tauri_utils::SemverCompare "${VERSION}" $PakoOldVersion
  Pop $PakoCmp
  ${If} $PakoCmp = 1
    StrCpy $PakoMode "update"
  ${ElseIf} $PakoCmp = 0
    StrCpy $PakoMode "same"
  ${Else}
    StrCpy $PakoMode "older"
  ${EndIf}
  ; Its folder (as Tauri's RestorePreviousInstallLocation reads it; $INSTDIR is another one when
  ; Setup was started with /D=).
  ReadRegStr $PakoOldDir SHCTX "${MANUPRODUCTKEY}" ""
  ${IfThen} $PakoOldDir == "" ${|} StrCpy $PakoOldDir $INSTDIR ${|}
  ; The switches show what this Pako has now.
  !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $PakoDesktop
  StrCpy $PakoStart 0
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${StrCase} $0 $0 "L"
  ${StrCase} $R0 "$INSTDIR\${MAINBINARYNAME}.exe" "L"
  ${StrLoc} $R1 $0 $R0 ">"
  ${If} $R1 != ""
    StrCpy $PakoStart 1
  ${EndIf}
  StrCpy $PakoBrowsers 0
  ClearErrors
  ReadRegStr $0 HKCU "Software\Google\Chrome\NativeMessagingHosts\app.pako.host" ""
  ${IfNot} ${Errors}
    StrCpy $PakoBrowsers 1
  ${EndIf}
  ClearErrors
  ReadRegStr $0 HKCU "Software\Microsoft\Edge\NativeMessagingHosts\app.pako.host" ""
  ${IfNot} ${Errors}
    StrCpy $PakoBrowsers 1
  ${EndIf}
  pako_detect_done:
  Pop $R6
  Pop $R1
  Pop $R0
  Pop $0
FunctionEnd

; The left side: the slide, and the line between the two sides.
Function PakoSlidePanel
  Push $0
  !insertmacro PakoCtl $PakoSlideCtl STATIC ${PAKO_PIC} 0 0 530 540 ""
  Call PakoSlideInit
  ${NSD_OnClick} $PakoSlideCtl PakoSlideClick
  ; ‹ and › for the keyboard (the mouse clicks them on the slide).
  !insertmacro PakoT $PakoTx pakoPrevSlide
  !insertmacro PakoProxy $PakoPxPrev ${PAKO_PUSH} 444 492 30 40 $PakoTx
  ${NSD_OnClick} $PakoPxPrev PakoPrevSlide
  !insertmacro PakoT $PakoTx pakoNextSlide
  !insertmacro PakoProxy $PakoPxNext ${PAKO_PUSH} 474 492 34 40 $PakoTx
  ${NSD_OnClick} $PakoPxNext PakoNextSlide
  !insertmacro PakoLabel $PakoSepCtl 530 0 1 540 ${PAKO_LABEL} $PakoFBody LN LN ""
  Pop $0
FunctionEnd

Function PakoPrevSlide
  Pop $0
  StrCpy $PakoAuto 0
  StrCpy $PakoDir -1
  IntOp $PakoSlide $PakoSlide - 1
  ${IfThen} $PakoSlide < 1 ${|} StrCpy $PakoSlide 5 ${|}
  Call PakoShowSlide
FunctionEnd

Function PakoNextSlide
  Pop $0
  StrCpy $PakoAuto 0
  StrCpy $PakoDir 1
  IntOp $PakoSlide $PakoSlide + 1
  ${IfThen} $PakoSlide > 5 ${|} StrCpy $PakoSlide 1 ${|}
  Call PakoShowSlide
FunctionEnd

; $PakoRet = the current slide's picture.
Function PakoSlideBmp
  ${If} $PakoSlide = 1
    StrCpy $PakoRet $PakoS1
  ${ElseIf} $PakoSlide = 2
    StrCpy $PakoRet $PakoS2
  ${ElseIf} $PakoSlide = 3
    StrCpy $PakoRet $PakoS3
  ${ElseIf} $PakoSlide = 4
    StrCpy $PakoRet $PakoS4
  ${Else}
    StrCpy $PakoRet $PakoS5
  ${EndIf}
FunctionEnd

; $PakoRet = a new 24-bit picture the size of a slide (24-bit: the picture control uses it as it
; is, never a copy of its own, so drawing into it shows at once and nothing piles up).
Function PakoNewFrame
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  !insertmacro PakoScale $0 530
  !insertmacro PakoScale $1 540
  IntOp $1 0 - $1
  System::Call "*(i 40, i r0, i r1, &i2 1, &i2 24, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r2"
  ; [Pako] The picture's handle in $3, its pixels' address in $4 (never the same register).
  System::Call "gdi32::CreateDIBSection(p 0, p r2, i 0, *p .r4, p 0, i 0) p .r3"
  System::Free $2
  StrCpy $PakoRet $3
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; Copies the picture $PakoA1 (a slide, or $PakoFrame) into the drawing $PakoA2 (one of the
; memory DCs). The picture is held only for the copy: a picture control can't draw one that is
; held somewhere else.
Function PakoCopyIn
  Push $0
  System::Call "gdi32::SelectObject(p $PakoDcF, p $PakoA1) p .r0"
  System::Call "gdi32::BitBlt(p $PakoA2, i 0, i 0, i $PakoSW, i $PakoSH, p $PakoDcF, i 0, i 0, i 0x00CC0020)"
  System::Call "gdi32::SelectObject(p $PakoDcF, p r0)"
  Pop $0
FunctionEnd

; $PakoOut -> $PakoFrame (what the control shows when Windows asks it to repaint).
Function PakoFrameSave
  Push $0
  System::Call "gdi32::SelectObject(p $PakoDcF, p $PakoFrame) p .r0"
  System::Call "gdi32::BitBlt(p $PakoDcF, i 0, i 0, i $PakoSW, i $PakoSH, p $PakoDcOut, i 0, i 0, i 0x00CC0020)"
  System::Call "gdi32::SelectObject(p $PakoDcF, p r0)"
  Pop $0
FunctionEnd

; Once: the pictures and drawings the slide changes use (kept until Setup closes).
Function PakoSlideSetup
  Push $0
  Push $1
  !insertmacro PakoScale $PakoSW 530
  !insertmacro PakoScale $PakoSH 540
  ; The picture and the words move; the dots and arrows under them only fade.
  !insertmacro PakoScale $PakoSTop 496
  !insertmacro PakoScale $PakoShift 80
  Call PakoNewFrame
  StrCpy $PakoFrame $PakoRet
  Call PakoNewFrame
  StrCpy $PakoFrom $PakoRet
  Call PakoNewFrame
  StrCpy $PakoTo $PakoRet
  Call PakoNewFrame
  StrCpy $PakoOut $PakoRet
  Call PakoNewFrame
  StrCpy $PakoMix $PakoRet
  System::Call "user32::GetDC(p 0) p .r0"
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $PakoDcF $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $PakoDcFrom $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $PakoDcTo $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $PakoDcOut $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $PakoDcMix $1
  System::Call "user32::ReleaseDC(p 0, p r0)"
  System::Call "gdi32::SelectObject(p $PakoDcFrom, p $PakoFrom)"
  System::Call "gdi32::SelectObject(p $PakoDcTo, p $PakoTo)"
  System::Call "gdi32::SelectObject(p $PakoDcOut, p $PakoOut)"
  System::Call "gdi32::SelectObject(p $PakoDcMix, p $PakoMix)"
  ; What shows beside a moving slide: the slides' own background (read from the first one, so it
  ; matches the picture exactly).
  StrCpy $PakoA1 $PakoS1
  StrCpy $PakoA2 $PakoDcTo
  Call PakoCopyIn
  System::Call "gdi32::GetPixel(p $PakoDcTo, i 2, i 2) i .r0"
  System::Call "gdi32::CreateSolidBrush(i r0) p .r1"
  System::Call "gdi32::SelectObject(p $PakoDcOut, p r1)"
  System::Call "gdi32::SelectObject(p $PakoDcMix, p r1)"
  Pop $1
  Pop $0
FunctionEnd

; One step of the change, $PakoA1 = how far it is (0 to 1000, eased): what showed ($PakoFrom)
; moves off one way and fades out while the new slide ($PakoTo) comes in from the other side.
Function PakoSlideDraw
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  IntOp $1 $PakoA1 * $PakoShift
  IntOp $1 $1 / 1000
  IntOp $2 $PakoShift - $1
  ${If} $PakoDir < 0
    StrCpy $3 $1
    IntOp $4 0 - $2
  ${Else}
    IntOp $3 0 - $1
    StrCpy $4 $2
  ${EndIf}
  ${If} $PakoA1 >= 1000
    System::Call "gdi32::BitBlt(p $PakoDcOut, i 0, i 0, i $PakoSW, i $PakoSH, p $PakoDcTo, i 0, i 0, i 0x00CC0020)"
  ${Else}
    !insertmacro PakoShifted $PakoDcOut $PakoDcFrom $3
    !insertmacro PakoShifted $PakoDcMix $PakoDcTo $4
    ; BLENDFUNCTION { AC_SRC_OVER, 0, alpha, 0 } as one 32-bit value.
    IntOp $0 $PakoA1 * 255
    IntOp $0 $0 / 1000
    ${If} $0 > 0
      IntOp $0 $0 << 16
      System::Call "msimg32::AlphaBlend(p $PakoDcOut, i 0, i 0, i $PakoSW, i $PakoSH, p $PakoDcMix, i 0, i 0, i $PakoSW, i $PakoSH, i r0)"
    ${EndIf}
  ${EndIf}
  ; Straight onto the screen in one copy, and the control isn't asked to paint it again.
  ; DCX_CACHE | DCX_CLIPSIBLINGS: the focus ring over the arrows stays.
  System::Call "user32::GetDCEx(p $PakoSlideCtl, p 0, i 0x12) p .r1"
  System::Call "gdi32::BitBlt(p r1, i 0, i 0, i $PakoSW, i $PakoSH, p $PakoDcOut, i 0, i 0, i 0x00CC0020)"
  System::Call "user32::ReleaseDC(p $PakoSlideCtl, p r1)"
  System::Call "user32::ValidateRect(p $PakoSlideCtl, p 0)"
  Call PakoFrameSave
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; A new slide control: it shows the current slide (no move). The slides wait 4.5 s from now.
Function PakoSlideInit
  Push $0
  Push $1
  Push $2
  ${If} $PakoFrame == ""
  ${OrIf} $PakoFrame = 0
    Call PakoSlideSetup
  ${EndIf}
  ${NSD_KillTimer} PakoFade
  ; With Windows' "Animation effects" off there's nothing to come in: the slide shows at once.
  StrCpy $0 1
  System::Call "user32::SystemParametersInfoW(i 0x1042, i 0, *i .r0, i 0)" ; SPI_GETCLIENTAREAANIMATION
  ${IfThen} $0 = 0 ${|} StrCpy $PakoIntro 0 ${|}
  ${If} $PakoIntro = 1
    ; The first time: only the slides' background, and the first slide comes in once the
    ; window shows (PakoTick).
    System::Call "gdi32::PatBlt(p $PakoDcOut, i 0, i 0, i $PakoSW, i $PakoSH, i 0x00F00021)"
  ${Else}
    Call PakoSlideBmp
    StrCpy $PakoA1 $PakoRet
    StrCpy $PakoA2 $PakoDcOut
    Call PakoCopyIn
  ${EndIf}
  Call PakoFrameSave
  SendMessage $PakoSlideCtl ${STM_SETIMAGE} 0 $PakoFrame
  System::Call "kernel32::GetTickCount() i .r0"
  StrCpy $PakoIdleT0 $0
  StrCpy $PakoMouseSeen 0
  System::Call "*(i 0, i 0) p .r1"
  System::Call "user32::GetCursorPos(p r1)"
  System::Call "*$1(i .r0, i .r2)"
  System::Free $1
  StrCpy $PakoCur0 "$0,$2"
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; The slide changed ($PakoSlide, in the direction $PakoDir): it slides in (also from the middle
; of a change), or appears at once when Windows' "Animation effects" are off.
Function PakoShowSlide
  Push $0
  ; A slide flipped before the first one came in replaces that coming in (it never runs after).
  StrCpy $PakoIntro 0
  ; What shows now (maybe half-way through a change) is what moves off.
  StrCpy $PakoA1 $PakoFrame
  StrCpy $PakoA2 $PakoDcFrom
  Call PakoCopyIn
  Call PakoSlideBmp
  StrCpy $PakoA1 $PakoRet
  StrCpy $PakoA2 $PakoDcTo
  Call PakoCopyIn
  System::Call "kernel32::GetTickCount() i .r0"
  StrCpy $PakoIdleT0 $0
  StrCpy $PakoFadeT0 $0
  StrCpy $0 1
  System::Call "user32::SystemParametersInfoW(i 0x1042, i 0, *i .r0, i 0)" ; SPI_GETCLIENTAREAANIMATION
  ${If} $0 = 0
    ${NSD_KillTimer} PakoFade
    StrCpy $PakoA1 1000
    Call PakoSlideDraw
  ${Else}
    StrCpy $PakoA1 0
    Call PakoSlideDraw
    ${NSD_CreateTimer} PakoFade 10
  ${EndIf}
  Pop $0
FunctionEnd

; One step of the change (as often as Windows' timer allows): eased out (cubic), done after 650 ms.
; Timed by the clock, so a busy PC skips steps instead of slowing the change down.
Function PakoFade
  Push $0
  Push $1
  System::Call "kernel32::GetTickCount() i .r0"
  IntOp $0 $0 - $PakoFadeT0
  ${If} $0 >= 650
  ${OrIf} $0 < 0
    ${NSD_KillTimer} PakoFade
    StrCpy $PakoA1 1000
  ${Else}
    ; 1000 × (1 - (1 - t)³), t = $0 / 650, in thousandths.
    IntOp $1 $0 * 1000
    IntOp $1 $1 / 650
    IntOp $1 1000 - $1
    IntOp $0 $1 * $1
    IntOp $0 $0 / 1000
    IntOp $0 $0 * $1
    IntOp $0 $0 / 1000
    IntOp $PakoA1 1000 - $0
  ${EndIf}
  Call PakoSlideDraw
  Pop $1
  Pop $0
FunctionEnd

; A click on the slides: ‹ › or a dot flips them, and they stop moving by themselves. A click
; anywhere else on the picture (to bring the window to the front, say) changes nothing.
Function PakoSlideClick
  Pop $0
  Push $1
  Push $2
  Push $3
  Push $4
  System::Call "*(i 0, i 0) p .r1"
  System::Call "user32::GetCursorPos(p r1)"
  System::Call "user32::ScreenToClient(p $PakoSlideCtl, p r1)"
  System::Call "*$1(i .r2, i .r3)"
  System::Free $1
  ; Back to 100 %.
  IntOp $2 $2 * 96
  IntOp $2 $2 / $PakoDpi
  IntOp $3 $3 * 96
  IntOp $3 $3 / $PakoDpi
  ${If} $3 >= 492
  ${AndIf} $3 < 532
    ${If} $2 >= 444
    ${AndIf} $2 < 474
      StrCpy $PakoAuto 0
      StrCpy $PakoDir -1
      IntOp $PakoSlide $PakoSlide - 1
      ${IfThen} $PakoSlide < 1 ${|} StrCpy $PakoSlide 5 ${|}
      Call PakoShowSlide
    ${ElseIf} $2 >= 474
    ${AndIf} $2 < 508
      StrCpy $PakoAuto 0
      StrCpy $PakoDir 1
      IntOp $PakoSlide $PakoSlide + 1
      ${IfThen} $PakoSlide > 5 ${|} StrCpy $PakoSlide 1 ${|}
      Call PakoShowSlide
    ${ElseIf} $2 < 140
      ; The dots: 7 wide (the current one 20), 6 apart, from x 28.
      StrCpy $1 28
      StrCpy $4 1
      ${Do}
        StrCpy $0 7
        ${IfThen} $4 = $PakoSlide ${|} StrCpy $0 20 ${|}
        IntOp $0 $0 + $1
        IntOp $0 $0 + 3
        ${If} $2 < $0
          StrCpy $PakoAuto 0
          ; A dot further on comes from the right, an earlier one from the left.
          ${If} $4 <> $PakoSlide
            StrCpy $PakoDir 1
            ${IfThen} $4 < $PakoSlide ${|} StrCpy $PakoDir -1 ${|}
            StrCpy $PakoSlide $4
            Call PakoShowSlide
          ${EndIf}
          ${Break}
        ${EndIf}
        IntOp $1 $0 + 3
        IntOp $4 $4 + 1
      ${LoopUntil} $4 > 5
    ${EndIf}
  ${EndIf}
  Pop $4
  Pop $3
  Pop $2
  Pop $1
FunctionEnd

; Every tenth of a second on the pages with slides: the next slide after 4.5 s without the mouse
; on them, and the buttons' hover pictures. The 4.5 s are read from the clock (a busy PC, Windows
; checking the new Setup say, sends these ticks late, and counting them kept the first slide far
; too long). The mouse holds the slides only once it has moved since they showed: resting where
; Setup was double-clicked, it may just happen to be over them.
Function PakoTick
  Push $0
  Push $1
  Push $2
  ; The first slide comes in once Setup's window is on the screen.
  ${If} $PakoIntro = 1
    System::Call "user32::IsWindowVisible(p $HWNDPARENT) i .r0"
    ${If} $0 <> 0
      StrCpy $PakoIntro 0
      StrCpy $PakoDir 1
      Call PakoShowSlide
    ${EndIf}
  ${EndIf}
  ${If} $PakoMouseSeen <> 1
    System::Call "*(i 0, i 0) p .r1"
    System::Call "user32::GetCursorPos(p r1)"
    System::Call "*$1(i .r0, i .r2)"
    System::Free $1
    ${IfThen} "$0,$2" != $PakoCur0 ${|} StrCpy $PakoMouseSeen 1 ${|}
  ${EndIf}
  System::Call "kernel32::GetTickCount() i .r0"
  ${If} $PakoMouseSeen = 1
    StrCpy $PakoA1 $PakoSlideCtl
    Call PakoMouseOn
    ${IfThen} $PakoRet = 1 ${|} StrCpy $PakoIdleT0 $0 ${|}
  ${EndIf}
  IntOp $1 $0 - $PakoIdleT0
  ${If} $PakoAuto = 1
  ${AndIf} $1 >= 4500
    StrCpy $PakoDir 1
    IntOp $PakoSlide $PakoSlide + 1
    ${IfThen} $PakoSlide > 5 ${|} StrCpy $PakoSlide 1 ${|}
    Call PakoShowSlide
  ${EndIf}
  ; [Pako] The French slides are read one per tick, never while a slide moves (a picture takes a
  ; few hundredths of a second), so the first switch to French is instant. Nor while a switch
  ; slides (template 1.1.5: it would stop for that moment).
  ${If} $PakoSFReady < 5
  ${AndIf} $PakoIntro <> 1
  ${AndIf} $PakoSwP1 = $PakoSwG1
  ${AndIf} $PakoSwP2 = $PakoSwG2
  ${AndIf} $PakoSwP3 = $PakoSwG3
    IntOp $1 $0 - $PakoFadeT0
    ${If} $1 > 800
    ${OrIf} $1 < 0
      Call PakoReadFrSlide
    ${EndIf}
  ${EndIf}
  Call PakoButtonsTick
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; Logo, the version under it (user, 2026-11-21) and the line under that, at the top of the right
; side. $PakoA1 = the line under the logo.
Function PakoHeader
  Push $0
  Push $1
  StrCpy $1 $PakoA1
  !insertmacro PakoPicture $0 615 34 160 44 $PakoBmpLogo
  !insertmacro PakoT $PakoTx pakoVersion
  !insertmacro PakoLabel $0 554 80 282 16 0x50000081 $PakoFTiny MUT BG $PakoTx
  !insertmacro PakoLabel $0 554 104 282 18 0x50000081 $PakoFSmall MUT BG $1
  Pop $1
  Pop $0
FunctionEnd

; ---------- page 1: everything at once ----------
Function PakoMainPage
  StrCpy $PassiveMode $PakoPassiveOrig
  ${If} $PassiveMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $PakoDialog
  ${If} $PakoDialog == error
    Abort
  ${EndIf}
  Call PakoHideNav
  Call PakoSetTitle
  !insertmacro PakoColors $PakoDialog TEXT BG
  StrCpy $PakoB1Ctl 0
  StrCpy $PakoB2Ctl 0
  Call PakoSlidePanel
  StrCpy $PakoCanMove 0
  ; The switches show what is set, at once (PakoSwSet: the pictures they are made with).
  Call PakoSwLeave
  StrCpy $PakoA1 1
  StrCpy $PakoA2 $PakoDesktop
  Call PakoSwSet
  StrCpy $PakoA1 2
  StrCpy $PakoA2 $PakoStart
  Call PakoSwSet
  !if ${PAKO_BROWSER_SWITCH} == 1
  StrCpy $PakoA1 3
  StrCpy $PakoA2 $PakoBrowsers
  Call PakoSwSet
  !endif
  Call PakoMainBuild
  Call PakoCheckRunning
  ${NSD_CreateTimer} PakoTick 100
  ${NSD_SetFocus} $PakoPxB1
  nsDialogs::Show
FunctionEnd

; [Pako] The right side of the main page, in the language shown (made again in the other one when
; the language changes: PakoLangApply). Everything it shows comes from the state ($INSTDIR, the
; switches, $PakoMode, $PakoRunning...), so nothing the user set is lost.
Function PakoMainBuild
  Call PakoLangChoice
  ; What this Setup does, under the logo.
  Call PakoModeTag
  StrCpy $PakoA1 $PakoRet
  Call PakoHeader
  !insertmacro PakoLabel $0 554 132 282 1 ${PAKO_LABEL} $PakoFBody LN LN ""
  ; Install in: the folder, and Change. Over an installed Pako it stays where it is (no Change),
  ; unless that folder won't do any more: then Change moves it (PakoCheckFolder).
  !insertmacro PakoT $PakoTx pakoInstallIn
  !insertmacro PakoLabel $0 554 144 282 18 ${PAKO_LABEL} $PakoFBody TEXT BG $PakoTx
  !insertmacro PakoPicture $0 554 168 282 32 $PakoBmpField
  !insertmacro PakoLabel $PakoPathCtl 584 176 190 16 0x50008080 $PakoFSmall TEXT RA $INSTDIR
  !insertmacro PakoT $PakoTx pakoChange
  !insertmacro PakoLabel $PakoChangeCtl 776 176 50 16 0x50000182 $PakoFSmall ACCENT RA $PakoTx
  ${NSD_OnClick} $PakoChangeCtl PakoChangeFolder
  !insertmacro PakoT $PakoTx pakoChangeName
  !insertmacro PakoProxy $PakoPxChange ${PAKO_PUSH} 776 176 50 16 $PakoTx
  ${NSD_OnClick} $PakoPxChange PakoChangeFolder
  ; [Pako] The language choice's radio buttons are a group of their own (the arrows move between
  ; English and Français only): the next stand-in starts the next group (WS_GROUP).
  System::Call "user32::GetWindowLongW(p $PakoPxChange, i -16) i .r1"
  IntOp $1 $1 | 0x00020000
  System::Call "user32::SetWindowLongW(p $PakoPxChange, i -16, i r1)"
  !insertmacro PakoLabel $0 554 214 282 1 ${PAKO_LABEL} $PakoFBody LN LN ""
  ; The switches (the words can be clicked too).
  !insertmacro PakoT $PakoTx pakoDesktop
  !insertmacro PakoLabel $0 554 225 240 18 0x50000180 $PakoFBody TEXT BG $PakoTx
  ${NSD_OnClick} $0 PakoToggleDesk
  !insertmacro PakoPicture $PakoSwDesk 804 225 32 18 $PakoSwFace1
  ${NSD_OnClick} $PakoSwDesk PakoToggleDesk
  !insertmacro PakoProxy $PakoPxDesk ${PAKO_CHECK} 804 225 32 18 $PakoTx
  ${NSD_OnClick} $PakoPxDesk PakoToggleDesk
  !insertmacro PakoLabel $0 554 254 282 1 ${PAKO_LABEL} $PakoFBody LN LN ""
  !insertmacro PakoT $PakoTx pakoStartWithWindows
  !insertmacro PakoLabel $0 554 265 240 18 0x50000180 $PakoFBody TEXT BG $PakoTx
  ${NSD_OnClick} $0 PakoToggleStart
  !insertmacro PakoPicture $PakoSwStart 804 265 32 18 $PakoSwFace2
  ${NSD_OnClick} $PakoSwStart PakoToggleStart
  !insertmacro PakoProxy $PakoPxStart ${PAKO_CHECK} 804 265 32 18 $PakoTx
  ${NSD_OnClick} $PakoPxStart PakoToggleStart
  !insertmacro PakoLabel $0 554 294 282 1 ${PAKO_LABEL} $PakoFBody LN LN ""
  !insertmacro PakoT $PakoTx pakoBrowsers
  !insertmacro PakoLabel $0 554 304 240 18 0x50000180 $PakoFBody TEXT BG $PakoTx
  ${NSD_OnClick} $0 PakoToggleBrowsers
  StrCpy $1 $PakoTx
  !insertmacro PakoT $PakoTx pakoBrowsersSub
  !insertmacro PakoLabel $0 554 323 240 16 0x50000180 $PakoFTiny MUT BG $PakoTx
  ${NSD_OnClick} $0 PakoToggleBrowsers
  !insertmacro PakoPicture $PakoSwBrow 804 313 32 18 $PakoSwFace3
  ${NSD_OnClick} $PakoSwBrow PakoToggleBrowsers
  !insertmacro PakoProxy $PakoPxBrow ${PAKO_CHECK} 804 313 32 18 $1
  ${NSD_OnClick} $PakoPxBrow PakoToggleBrowsers
  Call PakoShowSwitches
  ; Said here when something stands in the way (the folder) or will happen (Pako is open).
  ; [Pako] 84 high (up to five lines, also at 150 %: French is longer), ending 4 px over Install.
  !insertmacro PakoLabel $PakoNoteCtl 554 356 282 84 ${PAKO_LABEL} $PakoFSmall WARN BG ""
  ; Install (Update, Install again, Install this version).
  ${If} $PakoMode == "update"
    StrCpy $PakoA1 "update"
    !insertmacro PakoT $1 pakoBtnUpdate
  ${ElseIf} $PakoMode == "same"
    StrCpy $PakoA1 "again"
    !insertmacro PakoT $1 pakoBtnAgain
  ${ElseIf} $PakoMode == "older"
    StrCpy $PakoA1 "older"
    !insertmacro PakoT $1 pakoBtnOlder
  ${Else}
    StrCpy $PakoA1 "install"
    !insertmacro PakoT $1 pakoBtnInstall
  ${EndIf}
  StrCpy $PakoA2 282
  Call PakoLoadButton1
  !insertmacro PakoPicture $PakoB1Ctl 554 444 282 40 $PakoB1N
  ${NSD_OnClick} $PakoB1Ctl PakoInstallClick
  !insertmacro PakoProxy $PakoPxB1 ${PAKO_PUSH} 554 444 282 40 $1
  ${NSD_OnClick} $PakoPxB1 PakoInstallClick
  ; By installing, you accept the terms of use (a link). No admin rights needed.
  Call PakoTermsLine
  !insertmacro PakoRingMake
  Call PakoCheckFolder
  ; Over an installed Pako whose folder will do: it stays there (no Change).
  ${If} $PakoMode != "install"
  ${AndIf} $PakoCanMove <> 1
    ShowWindow $PakoChangeCtl ${SW_HIDE}
    ShowWindow $PakoPxChange ${SW_HIDE}
    !insertmacro PakoScale $1 242
    !insertmacro PakoScale $2 16
    System::Call "user32::SetWindowPos(p $PakoPathCtl, p 0, i 0, i 0, i r1, i r2, i 0x16)"
    !insertmacro PakoColors $PakoPathCtl MUT RA
  ${EndIf}
FunctionEnd

; [Pako] The language choice, top right: "English | Français", right-aligned with the switches. The
; one shown in the text colour, the other in the accent like the window's links (the mouse clicks
; the word). Under each word its radio button for the keyboard and screen readers: Tab reaches the
; one chosen, the arrows or Space pick, the focus ring shows around the word.
Function PakoLangChoice
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  StrCpy $PakoA1 $PakoFTiny
  StrCpy $PakoA2 "${PAKO_LANG_EN_NAME}"
  Call PakoTextWidth
  IntOp $1 $PakoRet + 1
  StrCpy $PakoA2 "${PAKO_LANG_FR_NAME}"
  Call PakoTextWidth
  IntOp $2 $PakoRet + 1
  ; Right to left: Français ends at 836, 8 px, a 1 px line, 8 px, English.
  !insertmacro PakoScale $3 836
  IntOp $3 $3 - $2            ; Français' left
  !insertmacro PakoScale $4 9
  IntOp $4 $3 - $4            ; the line's left
  !insertmacro PakoScale $5 8
  IntOp $5 $4 - $5
  IntOp $5 $5 - $1            ; English's left
  !insertmacro PakoScale $6 10 ; top
  !insertmacro PakoScale $0 16 ; height
  ; (pixels already: nsDialogs takes them as they are)
  nsDialogs::CreateControl STATIC 0x50000180 0 $5 $6 $1 $0 "${PAKO_LANG_EN_NAME}"
  Pop $PakoLangEnCtl
  SendMessage $PakoLangEnCtl ${WM_SETFONT} $PakoFTiny 1
  ${NSD_OnClick} $PakoLangEnCtl PakoLangClickEn
  nsDialogs::CreateControl STATIC 0x50000180 0 $3 $6 $2 $0 "${PAKO_LANG_FR_NAME}"
  Pop $PakoLangFrCtl
  SendMessage $PakoLangFrCtl ${WM_SETFONT} $PakoFTiny 1
  ${NSD_OnClick} $PakoLangFrCtl PakoLangClickFr
  ${If} $PakoLang == "fr"
    !insertmacro PakoColors $PakoLangEnCtl ACCENT BG
    !insertmacro PakoColors $PakoLangFrCtl TEXT BG
  ${Else}
    !insertmacro PakoColors $PakoLangEnCtl TEXT BG
    !insertmacro PakoColors $PakoLangFrCtl ACCENT BG
  ${EndIf}
  ; The stand-ins (BS_AUTORADIOBUTTON; the first starts the group; only the chosen one is a Tab
  ; stop, as in Windows' own radio groups), under the words, the same size.
  ${If} $PakoLang == "fr"
    StrCpy $PakoA2 0x54020009    ; WS_GROUP
    StrCpy $PakoA3 0x54010009    ; WS_TABSTOP
  ${Else}
    StrCpy $PakoA2 0x54030009    ; WS_GROUP | WS_TABSTOP
    StrCpy $PakoA3 0x54000009
  ${EndIf}
  nsDialogs::CreateControl BUTTON $PakoA2 0 $5 $6 $1 $0 "${PAKO_LANG_EN_NAME}"
  Pop $PakoPxEn
  SendMessage $PakoPxEn ${WM_SETFONT} $PakoFSmall 1
  System::Call "user32::SetWindowPos(p $PakoPxEn, p 1, i 0, i 0, i 0, i 0, i 0x13)"
  ${NSD_OnClick} $PakoPxEn PakoLangClickEn
  nsDialogs::CreateControl BUTTON $PakoA3 0 $3 $6 $2 $0 "${PAKO_LANG_FR_NAME}"
  Pop $PakoPxFr
  SendMessage $PakoPxFr ${WM_SETFONT} $PakoFSmall 1
  System::Call "user32::SetWindowPos(p $PakoPxFr, p 1, i 0, i 0, i 0, i 0, i 0x13)"
  ${NSD_OnClick} $PakoPxFr PakoLangClickFr
  ${If} $PakoLang == "fr"
    SendMessage $PakoPxFr ${BM_SETCHECK} ${BST_CHECKED} 0
  ${Else}
    SendMessage $PakoPxEn ${BM_SETCHECK} ${BST_CHECKED} 0
  ${EndIf}
  ; The line between the words, 10 px high, centred on them.
  !insertmacro PakoScale $6 13
  !insertmacro PakoScale $0 10
  !insertmacro PakoScale $1 1
  nsDialogs::CreateControl STATIC ${PAKO_LABEL} 0 $4 $6 $1 $0 ""
  Pop $0
  !insertmacro PakoColors $0 LN LN
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; A word or a radio button: the language changes a moment later, once Windows is done with the
; click (the control it came from is made again).
Function PakoLangClickEn
  Pop $0
  ; A pick, even of the language shown (English over a saved French): it is the user's choice.
  StrCpy $PakoLangPicked 1
  StrCpy $PakoLangNext "en"
  ${NSD_CreateTimer} PakoLangApply 1
FunctionEnd
Function PakoLangClickFr
  Pop $0
  StrCpy $PakoLangPicked 1
  StrCpy $PakoLangNext "fr"
  ${NSD_CreateTimer} PakoLangApply 1
FunctionEnd

; The window in the other language, at once: every word of Setup's own, the pictures with words in
; them (the slide shown, the button) and the window's title. The right side is made again by the
; same builder (PakoMainBuild), hidden until it is whole (no flash, no jump); the slides, their
; arrows and their timers go on, and the keyboard stays where it was.
Function PakoLangApply
  ${NSD_KillTimer} PakoLangApply
  ${If} $PakoLangNext == $PakoLang
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  StrCpy $PakoLang $PakoLangNext
  ; $LANGUAGE is what Setup saves for the uninstaller once it is done (MUI_LANGDLL_SAVELANGUAGE).
  ${If} $PakoLang == "fr"
    StrCpy $LANGUAGE ${LANG_FRENCH}
    StrCpy $PakoArtSfx "-fr"
  ${Else}
    StrCpy $LANGUAGE ${LANG_ENGLISH}
    StrCpy $PakoArtSfx ""
  ${EndIf}
  Call PakoLangTexts
  Call PakoUseSlides
  ; Where the keyboard is (that control is made again).
  ; (In a variable: PakoMainBuild uses $0 to $2.)
  System::Call "user32::GetFocus() p .r0"
  StrCpy $PakoLangFocus ""
  ${If} $0 = $PakoPxEn
  ${OrIf} $0 = $PakoPxFr
    StrCpy $PakoLangFocus "lang"
  ${ElseIf} $0 = $PakoPxChange
    StrCpy $PakoLangFocus "change"
  ${ElseIf} $0 = $PakoPxDesk
    StrCpy $PakoLangFocus "desk"
  ${ElseIf} $0 = $PakoPxStart
    StrCpy $PakoLangFocus "start"
  ${ElseIf} $0 = $PakoPxB1
    StrCpy $PakoLangFocus "b1"
  ${ElseIf} $0 = $PakoPxTerms
    StrCpy $PakoLangFocus "terms"
  ${ElseIf} $0 = $PakoPxBrow
    StrCpy $PakoLangFocus "brow"
  ${EndIf}
  ; Kept on a control that stays while the others go (a destroyed control's focus is lost).
  ${If} $PakoLangFocus != ""
    System::Call "user32::SetFocus(p $PakoPxNext)"
  ${EndIf}
  SendMessage $PakoDialog ${WM_SETREDRAW} 0 0
  ; Every control of the right side goes; the slide, its arrows and the line beside it stay.
  System::Call "user32::GetWindow(p $PakoDialog, i 5) p .r2" ; GW_CHILD
  ${DoWhile} $2 <> 0
    System::Call "user32::GetWindow(p r2, i 2) p .r3"      ; GW_HWNDNEXT
    ${If} $2 <> $PakoSlideCtl
    ${AndIf} $2 <> $PakoPxPrev
    ${AndIf} $2 <> $PakoPxNext
    ${AndIf} $2 <> $PakoSepCtl
      System::Call "user32::DestroyWindow(p r2)"
    ${EndIf}
    StrCpy $2 $3
  ${Loop}
  ; Whether the focus ring shows (Windows hides it again while controls are made).
  SendMessage $PakoDialog 0x129 0 0 $3 ; WM_QUERYUISTATE
  ; The big button's pictures (PakoMainBuild reads the new language's).
  System::Call "gdi32::DeleteObject(p $PakoB1N)"
  System::Call "gdi32::DeleteObject(p $PakoB1H)"
  ${If} $PakoB1X <> 0
    System::Call "gdi32::DeleteObject(p $PakoB1X)"
  ${EndIf}
  Call PakoMainBuild
  !insertmacro PakoT $PakoTx pakoPrevSlide
  SendMessage $PakoPxPrev ${WM_SETTEXT} 0 "STR:$PakoTx"
  !insertmacro PakoT $PakoTx pakoNextSlide
  SendMessage $PakoPxNext ${WM_SETTEXT} 0 "STR:$PakoTx"
  Call PakoSetTitle
  ; The slide shown, in the new language: one coming in goes on (and ends on it), one standing is
  ; swapped at once. (The first one, still to come in, comes in in the new language.)
  ${If} $PakoIntro <> 1
    Call PakoSlideBmp
    StrCpy $PakoA1 $PakoRet
    StrCpy $PakoA2 $PakoDcTo
    Call PakoCopyIn
    System::Call "kernel32::GetTickCount() i .r0"
    IntOp $0 $0 - $PakoFadeT0
    ${If} $0 >= 650
    ${OrIf} $0 < 0
      StrCpy $PakoA1 1000
      Call PakoSlideDraw
    ${EndIf}
  ${EndIf}
  SendMessage $PakoDialog ${WM_SETREDRAW} 1 0
  ; RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW | RDW_FRAME
  System::Call "user32::RedrawWindow(p $PakoDialog, p 0, p 0, i 0x585)"
  ${If} $PakoLangFocus == "lang"
    ${If} $PakoLang == "fr"
      SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxFr 1
    ${Else}
      SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxEn 1
    ${EndIf}
  ${ElseIf} $PakoLangFocus == "change"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxChange 1
  ${ElseIf} $PakoLangFocus == "desk"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxDesk 1
  ${ElseIf} $PakoLangFocus == "start"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxStart 1
  ${ElseIf} $PakoLangFocus == "b1"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxB1 1
  ${ElseIf} $PakoLangFocus == "terms"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxTerms 1
  ${ElseIf} $PakoLangFocus == "brow"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $PakoPxBrow 1
  ${EndIf}
  IntOp $3 $3 & 1 ; UISF_HIDEFOCUS
  ${If} $3 = 0
    SendMessage $HWNDPARENT 0x127 0x10002 0 ; WM_CHANGEUISTATE, UIS_CLEAR UISF_HIDEFOCUS
  ${EndIf}
  Call PakoButtonsTick
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; The switches as they are now (a switch on its way goes on from there): each is drawn into its
; new control (PakoMainBuild makes them again when the language changes).
Function PakoShowSwitches
  StrCpy $PakoA2 1
  StrCpy $PakoA1 1
  Call PakoSwDraw
  StrCpy $PakoA1 2
  Call PakoSwDraw
  StrCpy $PakoA1 3
  Call PakoSwDraw
  ; Screen readers read each switch's on/off from its stand-in.
  SendMessage $PakoPxDesk ${BM_SETCHECK} $PakoDesktop 0
  SendMessage $PakoPxStart ${BM_SETCHECK} $PakoStart 0
  SendMessage $PakoPxBrow ${BM_SETCHECK} $PakoBrowsers 0
FunctionEnd

; A click (the switch, its words, or Space on its stand-in): the setting flips at once, the
; switch slides there (PakoSwTo).
Function PakoToggleDesk
  Pop $0
  IntOp $PakoDesktop 1 - $PakoDesktop
  StrCpy $PakoA1 1
  StrCpy $PakoA2 $PakoDesktop
  Call PakoSwTo
  SendMessage $PakoPxDesk ${BM_SETCHECK} $PakoDesktop 0
FunctionEnd
Function PakoToggleStart
  Pop $0
  IntOp $PakoStart 1 - $PakoStart
  StrCpy $PakoA1 2
  StrCpy $PakoA2 $PakoStart
  Call PakoSwTo
  SendMessage $PakoPxStart ${BM_SETCHECK} $PakoStart 0
FunctionEnd
Function PakoToggleBrowsers
  Pop $0
  IntOp $PakoBrowsers 1 - $PakoBrowsers
  StrCpy $PakoA1 3
  StrCpy $PakoA2 $PakoBrowsers
  Call PakoSwTo
  SendMessage $PakoPxBrow ${BM_SETCHECK} $PakoBrowsers 0
FunctionEnd

; "By installing, you accept the terms of use." (the last words a link), centred, and under it
; "No admin rights needed."
Function PakoTermsLine
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  ${If} $PakoMode == "update"
    !insertmacro PakoT $4 pakoTermsAUpdate
  ${Else}
    !insertmacro PakoT $4 pakoTermsA
  ${EndIf}
  StrCpy $PakoA1 $PakoFTiny
  StrCpy $PakoA2 $4
  Call PakoTextWidth
  StrCpy $1 $PakoRet
  !insertmacro PakoT $PakoTx pakoTermsLink
  StrCpy $PakoA2 $PakoTx
  Call PakoTextWidth
  StrCpy $2 $PakoRet
  !insertmacro PakoT $PakoTx pakoTermsB
  StrCpy $PakoA2 $PakoTx
  Call PakoTextWidth
  StrCpy $3 $PakoRet
  ; Left edge of the line: the middle of the right side, less half the line.
  IntOp $0 $1 + $2
  IntOp $0 $0 + $3
  !insertmacro PakoScale $5 695
  IntOp $6 $0 / 2
  IntOp $5 $5 - $6
  !insertmacro PakoScale $6 494
  !insertmacro PakoScale $0 16
  ; (pixels already: nsDialogs takes them as they are)
  IntOp $1 $1 + 1
  nsDialogs::CreateControl STATIC ${PAKO_LABEL} 0 $5 $6 $1 $0 $4
  Pop $4
  SendMessage $4 ${WM_SETFONT} $PakoFTiny 1
  !insertmacro PakoColors $4 MUT BG
  IntOp $5 $5 + $1
  IntOp $5 $5 - 1
  IntOp $2 $2 + 1
  !insertmacro PakoT $PakoTx pakoTermsLink
  nsDialogs::CreateControl STATIC 0x50000180 0 $5 $6 $2 $0 $PakoTx
  Pop $4
  SendMessage $4 ${WM_SETFONT} $PakoFTiny 1
  !insertmacro PakoColors $4 ACCENT BG
  ${NSD_OnClick} $4 PakoOpenTerms
  ; Its stand-in for the keyboard (see PakoProxy).
  nsDialogs::CreateControl BUTTON ${PAKO_PUSH} 0 $5 $6 $2 $0 $PakoTx
  Pop $PakoPxTerms
  System::Call "user32::SetWindowPos(p $PakoPxTerms, p 1, i 0, i 0, i 0, i 0, i 0x13)"
  ${NSD_OnClick} $PakoPxTerms PakoOpenTerms
  IntOp $5 $5 + $2
  IntOp $5 $5 - 1
  IntOp $3 $3 + 1
  !insertmacro PakoT $PakoTx pakoTermsB
  nsDialogs::CreateControl STATIC ${PAKO_LABEL} 0 $5 $6 $3 $0 $PakoTx
  Pop $4
  SendMessage $4 ${WM_SETFONT} $PakoFTiny 1
  !insertmacro PakoColors $4 MUT BG
  ${If} $PakoMode == "install"
    !insertmacro PakoT $PakoTx pakoNoAdmin
    !insertmacro PakoLabel $4 554 512 282 16 0x50000081 $PakoFTiny MUT BG $PakoTx
  ${EndIf}
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function PakoOpenTerms
  Pop $0
  ; [Pako] The terms in the language shown (sookera.com/fr/terms in French).
  StrCpy $0 "https://sookera.com/terms"
  ${If} $PakoLang == "fr"
    StrCpy $0 "https://sookera.com/fr/terms"
  ${EndIf}
  ; As the Windows user, never with Setup's rights (Setup may run as administrator).
  nsis_tauri_utils::RunAsUser $0 ""
FunctionEnd

Function PakoChangeFolder
  Pop $0
  !insertmacro PakoT $PakoTx pakoPickFolder
  nsDialogs::SelectFolderDialog $PakoTx $INSTDIR
  Pop $0
  ${If} $0 == error
  ${OrIf} $0 == ""
    Return
  ${EndIf}
  ; Like Windows' own folder pages: Pako gets a folder of its own in the one picked.
  ${GetFileName} $0 $1
  ${If} $1 != "${PRODUCTNAME}"
    StrCpy $1 $0 1 -1
    ${If} $1 == "\"
      StrCpy $0 "$0${PRODUCTNAME}"
    ${Else}
      StrCpy $0 "$0\${PRODUCTNAME}"
    ${EndIf}
  ${EndIf}
  StrCpy $INSTDIR $0
  SendMessage $PakoPathCtl ${WM_SETTEXT} 0 "STR:$INSTDIR"
  Call PakoCheckFolder
  Call PakoCheckRunning
FunctionEnd

; Whether Pako can go in $INSTDIR: sets $PakoHeld and says why not under the options.
Function PakoCheckFolder
  Push $0
  StrCpy $PakoHeld 0
  StrCpy $PakoFolderMsg ""
  ${If} $INSTDIR == ""
    !insertmacro PakoT $PakoFolderMsg pakoFolderPick
  ${Else}
    ; Where the installed Pako keeps its settings and lists (unless it is installed there).
    ${If} $INSTDIR == "$LOCALAPPDATA\${PRODUCTNAME}"
      ReadRegStr $0 SHCTX "${MANUPRODUCTKEY}" ""
      ${If} $0 != $INSTDIR
        !insertmacro PakoT $PakoFolderMsg pakoFolderData
      ${EndIf}
    ${EndIf}
    ; A portable Pako's folder: installing there would leave its lists behind.
    ${If} $PakoFolderMsg == ""
    ${AndIfNot} ${FileExists} "$INSTDIR\.pako-installed"
      ${If} ${FileExists} "$INSTDIR\data\settings.json"
      ${OrIf} ${FileExists} "$INSTDIR\data\downloads.json"
        !insertmacro PakoT $PakoFolderMsg pakoFolderPortable
      ${EndIf}
    ${EndIf}
    ; A folder this Windows user can't change (or only administrators can), a drive or network
    ; folder that isn't there, a path too long for Windows.
    ${If} $PakoFolderMsg == ""
      Call PakoCheckDir
      ${If} $PakoDirProblem == "admin"
        !insertmacro PakoT $PakoFolderMsg pakoFolderAdmin
      ${ElseIf} $PakoDirProblem != ""
        ${If} $PakoDirWhy == "missing"
          !insertmacro PakoT $PakoFolderMsg pakoFolderMissing
        ${ElseIf} $PakoDirWhy == "long"
          !insertmacro PakoT $PakoFolderMsg pakoFolderLong
        ${Else}
          Push $INSTDIR
          Call PakoIsAdminFolder
          Pop $0
          ${If} $0 = 1
            !insertmacro PakoT $PakoFolderMsg pakoFolderAdmin
          ${Else}
            !insertmacro PakoT $PakoFolderMsg pakoFolderWrite
          ${EndIf}
        ${EndIf}
      ${EndIf}
    ${EndIf}
  ${EndIf}
  ; Over an installed Pako: in another folder than its own, Pako moves there (its uninstaller
  ; removes the old files, PageLeaveReinstall); its own folder that won't do any more can only be
  ; left with Change, said in the note.
  StrCpy $PakoMoved 0
  ${If} $PakoMode != "install"
    ${If} $INSTDIR != $PakoOldDir
      StrCpy $PakoMoved 1
    ${ElseIf} $PakoFolderMsg != ""
      StrCpy $PakoCanMove 1
      !insertmacro PakoT $PakoTx pakoMoveHint
      StrCpy $PakoFolderMsg "$PakoFolderMsg $PakoTx"
    ${EndIf}
  ${EndIf}
  ${If} $PakoFolderMsg != ""
    StrCpy $PakoHeld 1
  ${EndIf}
  Call PakoShowNote
  Call PakoButtonsTick
  Pop $0
FunctionEnd

; Whether Pako is open there now (it will be closed first: said in the window, not asked).
Function PakoCheckRunning
  Push $0
  Push $1
  Push $2
  Push $3
  Push $R0
  StrCpy $PakoRunning 0
  ; The installed Pako (where it is, also when it moves), else one in the folder picked.
  StrCpy $3 $INSTDIR
  ${IfThen} $PakoMode != "install" ${|} StrCpy $3 $PakoOldDir ${|}
  ${If} ${FileExists} "$3\${MAINBINARYNAME}.exe"
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 != ""
      !insertmacro RestartManager_RegisterFile $R0 "$3\${MAINBINARYNAME}.exe"
      ${If} $0 = 0
        System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
        ${If} $0 = ${ERROR_MORE_DATA}
          StrCpy $PakoRunning 1
        ${EndIf}
      ${EndIf}
      !insertmacro RestartManager_EndSession $R0
    ${EndIf}
  ${EndIf}
  StrCpy $PakoAgreedClose $PakoRunning
  Call PakoShowNote
  Pop $R0
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function PakoShowNote
  ${If} $PakoFolderMsg != ""
    !insertmacro PakoColors $PakoNoteCtl WARN BG
    SendMessage $PakoNoteCtl ${WM_SETTEXT} 0 "STR:$PakoFolderMsg"
  ${ElseIf} $PakoMoved = 1
    !insertmacro PakoColors $PakoNoteCtl MUT BG
    !insertmacro PakoT $PakoTx pakoMoveNote
    ${If} $PakoRunning = 1
      !insertmacro PakoT $PakoRet pakoIsOpen
      StrCpy $PakoTx "$PakoTx$\r$\n$PakoRet"
    ${EndIf}
    SendMessage $PakoNoteCtl ${WM_SETTEXT} 0 "STR:$PakoTx"
  ${ElseIf} $PakoRunning = 1
    !insertmacro PakoColors $PakoNoteCtl MUT BG
    !insertmacro PakoT $PakoTx pakoIsOpen
    SendMessage $PakoNoteCtl ${WM_SETTEXT} 0 "STR:$PakoTx"
  ${Else}
    SendMessage $PakoNoteCtl ${WM_SETTEXT} 0 "STR:"
  ${EndIf}
  ; The label repaints with its new colour.
  System::Call "user32::InvalidateRect(p $PakoNoteCtl, p 0, i 1)"
FunctionEnd

Function PakoInstallClick
  Pop $0
  ${If} $PakoHeld = 1
    Return
  ${EndIf}
  ; Next: the page's leave function, then the installing.
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd

Function PakoMainLeave
  StrCpy $PassiveMode $PakoPassiveOrig
  Call PakoCheckFolder
  ${If} $PakoHeld = 1
    Abort
  ${EndIf}
  ; [Pako] With the language untouched (the window starts in English on a first install, in
  ; the saved language over an installed Pako: PakoGuiInit), a saved French stays, also after
  ; an uninstall that kept it or over a portable copy: $LANGUAGE is what Setup saves once the
  ; install is done (MUI_LANGDLL_SAVELANGUAGE), and the app gets no --lang (hooks.nsh).
  ${If} $PakoLangPicked <> 1
    ReadRegStr $PakoTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
    ${If} $PakoTx == ${LANG_FRENCH}
      StrCpy $LANGUAGE ${LANG_FRENCH}
    ${EndIf}
  ${EndIf}
  ; Over an installed Pako: what Tauri's reinstall page did with its first (recommended)
  ; choice: the same version is installed again over itself; another one is replaced (its
  ; uninstaller removes only its files, see PageLeaveReinstall).
  ${If} $PakoMode != "install"
    StrCpy $R0 $PakoCmp
    StrCpy $R6 $PakoR6
    StrCpy $PakoUiChoice 1
    Call PageLeaveReinstall
  ${EndIf}
  ${NSD_KillTimer} PakoTick
  ${NSD_KillTimer} PakoFade
  Call PakoSwLeave
  ; Installed before the first slide came in: the last page shows it at once.
  StrCpy $PakoIntro 0
  !insertmacro PakoRingNone
FunctionEnd

; ---------- page 2: installing (Tauri's progress page, in Pako's look) ----------
Function PakoInstShow
  ; A passive run (/P) keeps Windows' plain progress window (nothing of Pako's look was set up).
  ${If} $PakoPassiveOrig = 1
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  Call PakoHideNav
  Call PakoSetTitle
  FindWindow $0 "#32770" "" $HWNDPARENT
  StrCpy $PakoA1 860
  StrCpy $PakoA2 540
  Call PakoPageFill
  ; The slide stays where it was (still: nothing moves while Pako installs).
  !insertmacro PakoRaw $1 $0 STATIC ${PAKO_PIC} 0 0 530 540 ""
  ${If} $PakoSlide = 1
    SendMessage $1 ${STM_SETIMAGE} 0 $PakoS1
  ${ElseIf} $PakoSlide = 2
    SendMessage $1 ${STM_SETIMAGE} 0 $PakoS2
  ${ElseIf} $PakoSlide = 3
    SendMessage $1 ${STM_SETIMAGE} 0 $PakoS3
  ${ElseIf} $PakoSlide = 4
    SendMessage $1 ${STM_SETIMAGE} 0 $PakoS4
  ${Else}
    SendMessage $1 ${STM_SETIMAGE} 0 $PakoS5
  ${EndIf}
  !insertmacro PakoRaw $1 $0 STATIC ${PAKO_LABEL} 530 0 1 540 ""
  !insertmacro PakoColors $1 LN LN
  !insertmacro PakoRaw $1 $0 STATIC ${PAKO_PIC} 615 34 160 44 ""
  SendMessage $1 ${STM_SETIMAGE} 0 $PakoBmpLogo
  !insertmacro PakoT $PakoTx pakoVersion
  !insertmacro PakoRaw $1 $0 STATIC 0x50000081 554 80 282 16 $PakoTx
  SendMessage $1 ${WM_SETFONT} $PakoFTiny 1
  !insertmacro PakoColors $1 MUT BG
  Call PakoModeTag
  !insertmacro PakoRaw $1 $0 STATIC 0x50000081 554 104 282 18 $PakoRet
  SendMessage $1 ${WM_SETFONT} $PakoFSmall 1
  !insertmacro PakoColors $1 MUT BG
  ${If} $PakoMode == "update"
    !insertmacro PakoT $2 pakoUpdating
  ${ElseIf} $PakoMode == "same"
    !insertmacro PakoT $2 pakoInstallingAgain
  ${ElseIf} $PakoMode == "older"
    !insertmacro PakoT $2 pakoInstallingOlder
  ${Else}
    !insertmacro PakoT $2 pakoInstalling
  ${EndIf}
  !insertmacro PakoRaw $1 $0 STATIC 0x50000080 554 210 282 24 $2
  SendMessage $1 ${WM_SETFONT} $PakoFTitle 1
  !insertmacro PakoColors $1 TEXT BG
  StrCpy $1 554
  StrCpy $2 248
  StrCpy $3 282
  Call PakoStyleProgress
  ; Close, for when something goes wrong (PakoFail).
  StrCpy $PakoA1 "b-done-n-$PakoLook$PakoArtSfx.png"
  StrCpy $PakoA2 282
  StrCpy $PakoA3 40
  Call PakoImage
  StrCpy $PakoFailBmp $PakoRet
  StrCpy $1 554
  StrCpy $2 444
  StrCpy $3 282
  Call PakoFailClose
  StrCpy $PakoInFiles 1
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; ---------- page 3: done ----------
Function PakoDonePage
  StrCpy $PassiveMode $PakoPassiveOrig
  ${If} $PassiveMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $PakoDialog
  ${If} $PakoDialog == error
    Abort
  ${EndIf}
  Call PakoHideNav
  Call PakoSetTitle
  !insertmacro PakoColors $PakoDialog TEXT BG
  StrCpy $PakoB1Ctl 0
  StrCpy $PakoB2Ctl 0
  StrCpy $PakoHeld 0
  StrCpy $PakoOpen 1
  StrCpy $PakoInFiles 0
  Call PakoSlidePanel
  !insertmacro PakoT $PakoA1 pakoTag
  Call PakoHeader
  ${If} $PakoMode == "update"
    !insertmacro PakoT $1 pakoDoneUpdate
  ${ElseIf} $PakoMode == "same"
    !insertmacro PakoT $1 pakoDoneAgain
  ${ElseIf} $PakoMode == "older"
    !insertmacro PakoT $1 pakoDoneOlder
  ${Else}
    !insertmacro PakoT $1 pakoDone
  ${EndIf}
  !insertmacro PakoLabel $0 554 210 282 24 ${PAKO_LABEL} $PakoFTitle TEXT BG $1
  ${If} $PakoDesktop = 1
    !insertmacro PakoT $1 pakoDoneWhereDesktop
  ${Else}
    !insertmacro PakoT $1 pakoDoneWhere
  ${EndIf}
  !insertmacro PakoLabel $0 554 242 282 40 ${PAKO_LABEL} $PakoFBody MUT BG $1
  ; Pako.exe couldn't take the window's choices (start with Windows, the browser connection).
  ${If} $PakoChoicesOk == "0"
    ${If} $PakoStart = 1
    ${OrIf} $PakoBrowsers = 1
      !insertmacro PakoT $PakoTx pakoChoicesFailed
      !insertmacro PakoLabel $0 554 292 282 72 ${PAKO_LABEL} $PakoFSmall WARN BG $PakoTx
    ${EndIf}
  ${EndIf}
  StrCpy $PakoA1 "open"
  StrCpy $PakoA2 282
  Call PakoLoadButton1
  !insertmacro PakoPicture $PakoB1Ctl 554 444 282 40 $PakoB1N
  ${NSD_OnClick} $PakoB1Ctl PakoOpenClick
  !insertmacro PakoT $PakoTx pakoOpenName
  !insertmacro PakoProxy $PakoPxB1 ${PAKO_PUSH} 554 444 282 40 $PakoTx
  ${NSD_OnClick} $PakoPxB1 PakoOpenClick
  !insertmacro PakoT $PakoTx pakoClose
  !insertmacro PakoLabel $0 665 496 60 18 0x50000181 $PakoFSmall MUT BG $PakoTx
  ${NSD_OnClick} $0 PakoCloseClick
  !insertmacro PakoT $PakoTx pakoClose
  !insertmacro PakoProxy $PakoPxB2 ${PAKO_PUSH} 665 496 60 18 $PakoTx
  ${NSD_OnClick} $PakoPxB2 PakoCloseClick
  !insertmacro PakoRingMake
  ; The window's ✕ and Esc close without opening Pako: with Cancel off (as Windows leaves it
  ; after installing), Windows would take ✕ as Next, which opens it.
  GetDlgItem $0 $HWNDPARENT 2
  EnableWindow $0 1
  ${NSD_CreateTimer} PakoTick 100
  ${NSD_SetFocus} $PakoPxB1
  nsDialogs::Show
FunctionEnd

Function PakoOpenClick
  Pop $0
  StrCpy $PakoOpen 1
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd
Function PakoCloseClick
  Pop $0
  StrCpy $PakoOpen 0
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd

; Enter (or Open Pako) opens Pako; Close and the window's ✕ don't.
Function PakoDoneLeave
  ${NSD_KillTimer} PakoTick
  ${NSD_KillTimer} PakoFade
  ${If} $PakoOpen = 1
    Call RunMainBinary
  ${EndIf}
FunctionEnd

; ---------- the uninstaller ----------
Function un.PakoGuiInit
  StrCpy $PakoPassiveOrig $PassiveMode
  ${If} $PassiveMode = 1
    Return
  ${EndIf}
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\art"
  File "${PAKO_ART}\logo-*.png"
  File "${PAKO_ART}\on-*.png"
  File "${PAKO_ART}\off-*.png"
  File "${PAKO_ART}\sw?-*.png"
  File "${PAKO_ART}\b-uninstall-*.png"
  File "${PAKO_ART}\b-cancel-*.png"
  File "${PAKO_ART}\b-close-*.png"
  ; [Pako] Out of the pictures' folder again: SetOutPath also made it Setup's working folder, and
  ; Windows can't delete a working folder, so NSIS's own clean-up at exit left it behind, empty, in %TEMP%.
  SetOutPath "$TEMP"
  Call un.PakoLookInit
  StrCpy $PakoA1 460
  StrCpy $PakoA2 320
  Call un.PakoWindow
  StrCpy $PakoA1 "logo-$PakoLook.png"
  StrCpy $PakoA2 160
  StrCpy $PakoA3 44
  Call un.PakoImage
  StrCpy $PakoBmpLogo $PakoRet
  Call un.PakoSwLoad
FunctionEnd

Function un.PakoSetTitle
  !insertmacro PakoT $PakoTx pakoUnTitle
  SendMessage $HWNDPARENT ${WM_SETTEXT} 0 "STR:$PakoTx"
FunctionEnd

Function un.PakoTick
  Call un.PakoButtonsTick
FunctionEnd

Function un.PakoUnPage
  StrCpy $PassiveMode $PakoPassiveOrig
  ${If} $PassiveMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $PakoDialog
  ${If} $PakoDialog == error
    Abort
  ${EndIf}
  Call un.PakoHideNav
  Call un.PakoSetTitle
  !insertmacro PakoColors $PakoDialog TEXT BG
  StrCpy $PakoHeld 0
  StrCpy $DeleteAppDataCheckboxState 0
  StrCpy $PakoDeletePrivate 0
  ; The switches start off, at once (PakoSwSet: the pictures the switches are made with).
  Call un.PakoSwLeave
  StrCpy $PakoA2 0
  StrCpy $PakoA1 1
  Call un.PakoSwSet
  StrCpy $PakoA1 2
  Call un.PakoSwSet
  !insertmacro PakoPicture $0 150 18 160 44 $PakoBmpLogo
  !insertmacro PakoT $PakoTx pakoVersion
  !insertmacro PakoLabel $0 24 64 412 16 0x50000081 $PakoFTiny MUT BG $PakoTx
  !insertmacro PakoT $PakoTx pakoUnTitle
  !insertmacro PakoLabel $0 24 84 412 24 0x50000081 $PakoFTitle TEXT BG $PakoTx
  !insertmacro PakoT $PakoTx pakoUnText
  !insertmacro PakoLabel $0 24 112 412 39 0x50000081 $PakoFBody MUT BG $PakoTx
  !insertmacro PakoLabel $0 24 152 412 1 ${PAKO_LABEL} $PakoFBody LN LN ""
  ; [Pako] Two switches: the settings and lists, and Private mode's files apart (they can't be
  ; downloaded again, and the first switch never deletes them).
  !insertmacro PakoT $PakoTx deleteAppData
  !insertmacro PakoLabel $0 24 164 350 18 0x50000180 $PakoFBody TEXT BG $PakoTx
  ${NSD_OnClick} $0 un.PakoToggleData
  !insertmacro PakoT $PakoTx pakoUnDataSub
  !insertmacro PakoLabel $0 24 183 350 16 0x50000180 $PakoFTiny MUT BG $PakoTx
  ${NSD_OnClick} $0 un.PakoToggleData
  !insertmacro PakoPicture $PakoSwDesk 404 172 32 18 $PakoSwFace1
  ${NSD_OnClick} $PakoSwDesk un.PakoToggleData
  !insertmacro PakoT $PakoTx deleteAppData
  !insertmacro PakoProxy $PakoPxDesk ${PAKO_CHECK} 404 172 32 18 $PakoTx
  ${NSD_OnClick} $PakoPxDesk un.PakoToggleData
  !insertmacro PakoT $PakoTx pakoUnPrivate
  !insertmacro PakoLabel $0 24 208 350 18 0x50000180 $PakoFBody TEXT BG $PakoTx
  ${NSD_OnClick} $0 un.PakoTogglePrivate
  !insertmacro PakoT $PakoTx pakoUnPrivateSub
  !insertmacro PakoLabel $0 24 227 350 16 0x50000180 $PakoFTiny MUT BG $PakoTx
  ${NSD_OnClick} $0 un.PakoTogglePrivate
  !insertmacro PakoPicture $PakoSwStart 404 216 32 18 $PakoSwFace2
  ${NSD_OnClick} $PakoSwStart un.PakoTogglePrivate
  !insertmacro PakoT $PakoTx pakoUnPrivate
  !insertmacro PakoProxy $PakoPxStart ${PAKO_CHECK} 404 216 32 18 $PakoTx
  ${NSD_OnClick} $PakoPxStart un.PakoTogglePrivate
  StrCpy $PakoA1 "cancel"
  StrCpy $PakoA2 200
  Call un.PakoLoadButton2
  !insertmacro PakoPicture $PakoB2Ctl 24 262 200 40 $PakoB2N
  ${NSD_OnClick} $PakoB2Ctl un.PakoCancelClick
  !insertmacro PakoT $PakoTx pakoCancel
  !insertmacro PakoProxy $PakoPxB2 ${PAKO_PUSH} 24 262 200 40 $PakoTx
  ${NSD_OnClick} $PakoPxB2 un.PakoCancelClick
  StrCpy $PakoA1 "uninstall"
  StrCpy $PakoA2 200
  Call un.PakoLoadButton1
  !insertmacro PakoPicture $PakoB1Ctl 236 262 200 40 $PakoB1N
  ${NSD_OnClick} $PakoB1Ctl un.PakoGoClick
  !insertmacro PakoT $PakoTx pakoUninstall
  !insertmacro PakoProxy $PakoPxB1 ${PAKO_PUSH} 236 262 200 40 $PakoTx
  ${NSD_OnClick} $PakoPxB1 un.PakoGoClick
  !insertmacro PakoRingMake
  ${NSD_CreateTimer} un.PakoTick 100
  ${NSD_SetFocus} $PakoPxB1
  nsDialogs::Show
FunctionEnd

Function un.PakoToggleData
  Pop $0
  IntOp $DeleteAppDataCheckboxState 1 - $DeleteAppDataCheckboxState
  StrCpy $PakoA1 1
  StrCpy $PakoA2 $DeleteAppDataCheckboxState
  Call un.PakoSwTo
  SendMessage $PakoPxDesk ${BM_SETCHECK} $DeleteAppDataCheckboxState 0
FunctionEnd

Function un.PakoTogglePrivate
  Pop $0
  IntOp $PakoDeletePrivate 1 - $PakoDeletePrivate
  StrCpy $PakoA1 2
  StrCpy $PakoA2 $PakoDeletePrivate
  Call un.PakoSwTo
  SendMessage $PakoPxStart ${BM_SETCHECK} $PakoDeletePrivate 0
FunctionEnd

Function un.PakoGoClick
  Pop $0
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd
Function un.PakoCancelClick
  Pop $0
  SendMessage $HWNDPARENT ${WM_COMMAND} 2 0
FunctionEnd

Function un.PakoUnLeave
  ${NSD_KillTimer} un.PakoTick
  Call un.PakoSwLeave
  StrCpy $PakoB1Ctl 0
  StrCpy $PakoB2Ctl 0
  !insertmacro PakoRingNone
FunctionEnd

Function un.PakoInstShow
  ; A passive run (/P) keeps Windows' plain progress window (nothing of Pako's look was set up).
  ${If} $PakoPassiveOrig = 1
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  Call un.PakoHideNav
  Call un.PakoSetTitle
  FindWindow $0 "#32770" "" $HWNDPARENT
  StrCpy $PakoA1 460
  StrCpy $PakoA2 320
  Call un.PakoPageFill
  !insertmacro PakoRaw $1 $0 STATIC ${PAKO_PIC} 150 32 160 44 ""
  SendMessage $1 ${STM_SETIMAGE} 0 $PakoBmpLogo
  !insertmacro PakoT $PakoTx pakoVersion
  !insertmacro PakoRaw $1 $0 STATIC 0x50000081 24 78 412 16 $PakoTx
  SendMessage $1 ${WM_SETFONT} $PakoFTiny 1
  !insertmacro PakoColors $1 MUT BG
  !insertmacro PakoT $PakoTx pakoRemoving
  !insertmacro PakoRaw $1 $0 STATIC 0x50000081 24 120 412 24 $PakoTx
  SendMessage $1 ${WM_SETFONT} $PakoFTitle 1
  !insertmacro PakoColors $1 TEXT BG
  StrCpy $1 24
  StrCpy $2 160
  StrCpy $3 412
  Call un.PakoStyleProgress
  ; Close, for when something goes wrong (PakoFail).
  StrCpy $PakoA1 "b-close-n-$PakoLook$PakoArtSfx.png"
  StrCpy $PakoA2 412
  StrCpy $PakoA3 40
  Call un.PakoImage
  StrCpy $PakoFailBmp $PakoRet
  StrCpy $1 24
  StrCpy $2 256
  StrCpy $3 412
  Call un.PakoFailClose
  StrCpy $PakoInFiles 1
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function un.PakoDonePage
  StrCpy $PassiveMode $PakoPassiveOrig
  ${If} $PassiveMode = 1
  ${OrIf} $UpdateMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $PakoDialog
  ${If} $PakoDialog == error
    Abort
  ${EndIf}
  Call un.PakoHideNav
  Call un.PakoSetTitle
  !insertmacro PakoColors $PakoDialog TEXT BG
  StrCpy $PakoB2Ctl 0
  !insertmacro PakoPicture $0 150 32 160 44 $PakoBmpLogo
  !insertmacro PakoT $PakoTx pakoVersion
  !insertmacro PakoLabel $0 24 78 412 16 0x50000081 $PakoFTiny MUT BG $PakoTx
  !insertmacro PakoT $PakoTx pakoUnDone
  !insertmacro PakoLabel $0 24 100 412 24 0x50000081 $PakoFTitle TEXT BG $PakoTx
  !insertmacro PakoT $PakoTx pakoUnDoneText
  !insertmacro PakoLabel $0 24 130 412 39 0x50000081 $PakoFBody MUT BG $PakoTx
  StrCpy $PakoA1 "close"
  StrCpy $PakoA2 412
  Call un.PakoLoadButton1
  !insertmacro PakoPicture $PakoB1Ctl 24 256 412 40 $PakoB1N
  ${NSD_OnClick} $PakoB1Ctl un.PakoGoClick
  !insertmacro PakoT $PakoTx pakoClose
  !insertmacro PakoProxy $PakoPxB1 ${PAKO_PUSH} 24 256 412 40 $PakoTx
  ${NSD_OnClick} $PakoPxB1 un.PakoGoClick
  !insertmacro PakoRingMake
  StrCpy $PakoInFiles 0
  ; The window's ✕ and Esc close it too (with Cancel off, as Windows leaves it after
  ; uninstalling, they did nothing).
  GetDlgItem $0 $HWNDPARENT 2
  EnableWindow $0 1
  ${NSD_CreateTimer} un.PakoTick 100
  ${NSD_SetFocus} $PakoPxB1
  nsDialogs::Show
FunctionEnd

Function un.PakoDoneLeave
  ${NSD_KillTimer} un.PakoTick
FunctionEnd
!macroend
