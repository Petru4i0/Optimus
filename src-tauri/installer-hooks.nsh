!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Stopping running Optimus instance..."
  nsExec::ExecToLog 'taskkill /F /IM "Optimus.exe"'
  Sleep 1000
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Stopping running Optimus instance..."
  nsExec::ExecToLog 'taskkill /F /IM "Optimus.exe"'
  Sleep 1000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "Removing elevated autostart task..."
  nsExec::ExecToLog 'schtasks /Delete /TN "OptimusAutoStart" /F'

  DetailPrint "Removing Optimus registry keys..."
  DeleteRegKey HKCU "Software\com.petruchio.optimus"
  DeleteRegKey HKLM "Software\com.petruchio.optimus"
  DeleteRegKey /ifempty SHCTX "Software\PetruchiO\Optimus"
  DeleteRegKey /ifempty SHCTX "Software\PetruchiO"
  DeleteRegValue HKCU "Software\PetruchiO\Optimus" "Installer Language"
  DeleteRegKey /ifempty HKCU "Software\PetruchiO\Optimus"
  DeleteRegKey /ifempty HKCU "Software\PetruchiO"
!macroend
