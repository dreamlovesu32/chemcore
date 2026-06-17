!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Registering Chemcore Office/OLE integration..."

  IfFileExists "$INSTDIR\chemcore-office.exe" chemcore_office_found_root
  IfFileExists "$INSTDIR\resources\chemcore-office.exe" chemcore_office_found_resources
  DetailPrint "Chemcore Office/OLE registration skipped: chemcore-office.exe was not found."
  MessageBox MB_ICONSTOP "Chemcore Office/OLE registration failed because chemcore-office.exe was not found."
  Abort

  chemcore_office_found_root:
    StrCpy $1 "$INSTDIR\chemcore-office.exe"
    Goto chemcore_office_register_machine

  chemcore_office_found_resources:
    StrCpy $1 "$INSTDIR\resources\chemcore-office.exe"

  chemcore_office_register_machine:
  ClearErrors
  ExecWait '"$1" --register-machine' $0
  IfErrors chemcore_office_register_machine_exec_failed
  StrCmp $0 0 chemcore_office_register_machine_done
  DetailPrint "Chemcore Office/OLE machine registration failed with exit code: $0"
  Goto chemcore_office_register_user

  chemcore_office_register_machine_exec_failed:
  DetailPrint "Chemcore Office/OLE machine registration could not launch."

  chemcore_office_register_user:
  DetailPrint "Registering Chemcore Office/OLE integration for the current user..."
  ClearErrors
  ExecWait '"$1" --register-user' $0
  IfErrors chemcore_office_register_user_exec_failed
  StrCmp $0 0 chemcore_office_register_user_done
  DetailPrint "Chemcore Office/OLE current-user registration failed with exit code: $0"
  MessageBox MB_ICONSTOP "Chemcore Office/OLE registration failed with exit code $0."
  Abort

  chemcore_office_register_user_exec_failed:
  DetailPrint "Chemcore Office/OLE current-user registration could not launch."
  MessageBox MB_ICONSTOP "Chemcore Office/OLE registration failed because chemcore-office.exe could not be launched."
  Abort

  chemcore_office_register_machine_done:
  DetailPrint "Chemcore Office/OLE machine registration succeeded."
  Goto chemcore_office_register_done

  chemcore_office_register_user_done:
  DetailPrint "Chemcore Office/OLE current-user registration succeeded."

  chemcore_office_register_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Unregistering Chemcore Office/OLE integration..."
  IfFileExists "$INSTDIR\chemcore-office.exe" chemcore_office_uninstall_found_root
  IfFileExists "$INSTDIR\resources\chemcore-office.exe" chemcore_office_uninstall_found_resources
  DetailPrint "Chemcore Office/OLE unregistration skipped: chemcore-office.exe was not found."
  Goto chemcore_office_uninstall_done

  chemcore_office_uninstall_found_root:
    StrCpy $1 "$INSTDIR\chemcore-office.exe"
    Goto chemcore_office_unregister

  chemcore_office_uninstall_found_resources:
    StrCpy $1 "$INSTDIR\resources\chemcore-office.exe"

  chemcore_office_unregister:
  ClearErrors
  ExecWait '"$1" --unregister-machine' $0
  IfErrors 0 chemcore_office_unregister_user
  DetailPrint "Chemcore Office/OLE machine unregistration could not launch."

  chemcore_office_unregister_user:
  ClearErrors
  ExecWait '"$1" --unregister-user' $0
  IfErrors 0 chemcore_office_uninstall_done
  DetailPrint "Chemcore Office/OLE current-user unregistration could not launch."

  chemcore_office_uninstall_done:
!macroend
