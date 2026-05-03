!macro NSIS_HOOK_POSTINSTALL
  ; Do not install Ollama from the NSIS installer.
  ;
  ; First boot owns dependency setup. The wizard's "Install everything"
  ; step installs Whisper, starts the Ollama installer when missing, and
  ; pulls the email-draft model with visible per-step status. Keeping the
  ; NSIS installer focused on Veyra prevents hidden dependency failures
  ; before the user even reaches first boot.
  DetailPrint "Skipping Ollama dependency install; first boot wizard handles it"
!macroend
