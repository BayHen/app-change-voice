!include "LogicLib.nsh"
!include "WinVer.nsh"
!include "x64.nsh"

!define VOICEMORPH_DRIVER_SERVICE "VoiceMorphVirtualAudio"
!define VOICEMORPH_DRIVER_VERSION_HIGH 1
!define VOICEMORPH_DRIVER_VERSION_LOW 0

!macro NSIS_HOOK_PREINSTALL
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_ICONSTOP|MB_OK "VoiceMorph Virtual Microphone requires Windows 10 or Windows 11 x64."
    Abort
  ${EndIf}

  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP|MB_OK "VoiceMorph Virtual Microphone requires 64-bit Windows."
    Abort
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  StrCpy $0 "$INSTDIR\resources\driver-package\VoiceMorphVirtualAudio.inf"
  StrCpy $1 "$INSTDIR\resources\driver-package\VoiceMorphVirtualAudio.sys"
  StrCpy $2 "$INSTDIR\resources\driver-package\VoiceMorphVirtualAudio.cat"

  ${IfNot} ${FileExists} "$0"
    MessageBox MB_ICONSTOP|MB_OK "The signed VoiceMorph driver package is missing. Installation cannot continue."
    Abort
  ${EndIf}
  ${IfNot} ${FileExists} "$1"
    MessageBox MB_ICONSTOP|MB_OK "The VoiceMorph driver binary is missing. Installation cannot continue."
    Abort
  ${EndIf}
  ${IfNot} ${FileExists} "$2"
    MessageBox MB_ICONSTOP|MB_OK "The VoiceMorph driver catalog is missing. Installation cannot continue."
    Abort
  ${EndIf}

  ClearErrors
  GetDLLVersion "$WINDIR\System32\drivers\VoiceMorphVirtualAudio.sys" $3 $4
  ${IfNot} ${Errors}
    IntCmpU $3 ${VOICEMORPH_DRIVER_VERSION_HIGH} driver_version_low driver_install driver_keep
    driver_version_low:
      IntCmpU $4 ${VOICEMORPH_DRIVER_VERSION_LOW} driver_install driver_keep driver_keep
    driver_keep:
      DetailPrint "VoiceMorph Virtual Microphone is already current or newer; keeping installed driver."
      Goto driver_done
  ${EndIf}

  driver_install:
    DetailPrint "Installing signed VoiceMorph Virtual Microphone driver package..."
    nsExec::ExecToStack '"$SYSDIR\pnputil.exe" /add-driver "$0" /install'
    Pop $5
    Pop $6
    DetailPrint "$6"
    ${If} $5 == 3010
      SetRebootFlag true
      Goto driver_done
    ${EndIf}
    ${If} $5 != 0
      MessageBox MB_ICONSTOP|MB_OK "Windows rejected the VoiceMorph Virtual Microphone driver (PnPUtil exit code $5). No security checks were bypassed."
      Abort
    ${EndIf}

  driver_done:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  MessageBox MB_ICONINFORMATION|MB_OK "VoiceMorph Virtual Microphone is retained by default so other installed VoiceMorph versions are not disrupted. See DRIVER_INSTALLATION.md for the explicit driver removal procedure. Presets and recordings are not removed."
!macroend
