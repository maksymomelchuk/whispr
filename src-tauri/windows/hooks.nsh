; The VC++ 2015–2022 runtime (msvcp140.dll and friends) is a load-time dependency
; of both whispr.exe (whisper.cpp) and the bundled onnxruntime.dll. A clean Windows
; install lacks it, so the app fails to launch with "MSVCP140.dll was not found".
!macro NSIS_HOOK_POSTINSTALL
  ; NSIS runs as a 32-bit process and would otherwise read the WOW6432Node view,
  ; where the x64 runtime key is absent — re-running the installer (and prompting
  ; for UAC) on every silent auto-update. Read the native 64-bit view instead.
  SetRegView 64
  ReadRegDWORD $0 HKLM "SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64" "Installed"
  SetRegView lastused

  ${If} $0 == 1
    DetailPrint "Visual C++ runtime already present — skipping."
    Goto vc_redist_done
  ${EndIf}

  ${IfNot} ${FileExists} "$INSTDIR\resources\vc_redist.x64.exe"
    DetailPrint "Visual C++ runtime installer not bundled — skipping."
    Goto vc_redist_done
  ${EndIf}

  DetailPrint "Installing Visual C++ runtime..."
  ExecWait '"$INSTDIR\resources\vc_redist.x64.exe" /install /quiet /norestart' $0

  ; 0 = installed, 3010 = installed (reboot pending), 1638 = newer already present.
  ${If} $0 == 0
  ${OrIf} $0 == 3010
  ${OrIf} $0 == 1638
    DetailPrint "Visual C++ runtime installed."
  ${ElseIf} ${Silent}
    DetailPrint "Visual C++ runtime install failed (code $0)."
  ${Else}
    MessageBox MB_ICONEXCLAMATION "Could not install the Visual C++ runtime (code $0). Whispr may not start until you install it from https://aka.ms/vs/17/release/vc_redist.x64.exe."
  ${EndIf}

  vc_redist_done:
!macroend
