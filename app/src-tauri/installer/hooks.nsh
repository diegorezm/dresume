; Puts the bundled `dresume` CLI on the user's PATH (and takes it off on uninstall).

!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec 'powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d = \"$INSTDIR\bin\"; $$p = [Environment]::GetEnvironmentVariable(\"Path\", \"User\"); if (-not (($$p -split \";\") -contains $$d)) { [Environment]::SetEnvironmentVariable(\"Path\", (($$p.TrimEnd(\";\") + \";\" + $$d).TrimStart(\";\")), \"User\") }"'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d = \"$INSTDIR\bin\"; $$p = [Environment]::GetEnvironmentVariable(\"Path\", \"User\"); [Environment]::SetEnvironmentVariable(\"Path\", (($$p -split \";\" | Where-Object { $$_ -and $$_ -ne $$d }) -join \";\"), \"User\")"'
!macroend
