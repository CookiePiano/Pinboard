; $0: requested path. Returns PbPathError (empty, invalid, space) and $5 canonical path.
; Reads volume capacity only; never creates a directory during validation.
Var PbPathError
Function PbValidatePath
  StrCpy $PbPathError invalid
  StrCmp $0 "" done
  StrCpy $1 $0 2
  StrCmp $1 "\\" absolute
  StrCpy $1 $0 2 1
  StrCmp $1 ":\" 0 done
  absolute:
  ; Unlike NSIS GetFullPathName, this supports a destination that does not exist yet.
  System::Call 'kernel32::GetFullPathNameW(w r0, i ${NSIS_MAX_STRLEN}, w .r5, p 0) i .r6'
  StrCmp $6 0 done
  IntCmp $6 ${NSIS_MAX_STRLEN} done 0 done
  ${GetRoot} "$5" $1
  StrCmp $1 "" done
  StrCmp $5 "$1\" done
  StrCmp $5 "$1" done
  StrCpy $1 "$1\"
  System::Call 'kernel32::GetDiskFreeSpaceExW(w r1, *l .r2, p 0, p 0) i .r3'
  StrCmp $3 0 done
  System::Int64Op $2 / 1024
  Pop $2
  System::Int64Op $2 < ${ESTIMATEDSIZE}
  Pop $2
  StrCpy $PbPathError space
  StrCmp $2 1 done
  StrCpy $PbPathError ""
  done:
FunctionEnd
