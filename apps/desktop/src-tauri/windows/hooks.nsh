; Installs before 0.6.0 were named "Mangas". The upgrade reuses their uninstall key and folder
; (Windows matches both case-insensitively), but rewriting a shortcut keeps the old file name, so
; the Start menu would still say "Mangas". Rename the shortcuts to the current product name.
!macro MANGAS_RENAME_SHORTCUT LNK
  ${If} ${FileExists} "${LNK}"
    Rename "${LNK}" "${LNK}.rename"
    Rename "${LNK}.rename" "${LNK}"
  ${EndIf}
  ClearErrors
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro MANGAS_RENAME_SHORTCUT "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  !insertmacro MANGAS_RENAME_SHORTCUT "$DESKTOP\${PRODUCTNAME}.lnk"
!macroend
