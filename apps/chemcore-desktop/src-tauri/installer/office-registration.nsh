!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Registering Chemcore Office/OLE integration..."
  ExecWait '"$INSTDIR\resources\chemcore-office.exe" --register-machine' $0
  StrCmp $0 0 chemcore_office_register_done
  DetailPrint "Chemcore Office/OLE registration failed with exit code: $0"
  MessageBox MB_ICONSTOP "Chemcore Office/OLE registration failed with exit code $0."
  Abort
  chemcore_office_register_done:
  DetailPrint "Chemcore Office/OLE registration exit code: $0"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Unregistering Chemcore Office/OLE integration..."
  ExecWait '"$INSTDIR\resources\chemcore-office.exe" --unregister-machine' $0
  DetailPrint "Chemcore Office/OLE unregistration exit code: $0"
!macroend
