import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { HaloOrb } from "./halo-orb";
import { OverlayPill } from "./pill";
import { INITIAL_VOICE_ACTIVITY, nextVoiceActivity } from "./voice-activity";
import { useSettings } from "@/hooks/use-settings";
import { ipc } from "@/lib/tauri";
import { useOverlayStore, type OverlayMode, type OverlayState } from "@/stores/overlay-store";
import type { RecordingState } from "@/types/ipc";
import type { OverlaySize, OverlayStyle } from "@/types/settings";

function mapState(state: RecordingState): OverlayState {
  if (state === "Recording") return "recording";
  if (state === "Transcribing") return "transcribing";
  return "idle";
}

export function OverlayApp() {
  const state = useOverlayStore((s) => s.state);
  const mode = useOverlayStore((s) => s.mode);
  const setState = useOverlayStore((s) => s.setState);
  const setMode = useOverlayStore((s) => s.setMode);
  const setLevel = useOverlayStore((s) => s.setLevel);
  const { settings } = useSettings();
  const overlayStyle: OverlayStyle = settings?.overlayStyle ?? "capsule";
  const overlaySize: OverlaySize = settings?.overlaySize ?? "medium";
  const voiceActivity = useRef(INITIAL_VOICE_ACTIVITY);
  const lastLevelAt = useRef<number | null>(null);

  useEffect(() => {
    void ipc.getRecordingState().then((recordingState) => {
      setState(mapState(recordingState));
    }).catch(() => {});
    void ipc.getRecordingMode().then(setMode).catch(() => {});
  }, [setMode, setState]);

  useEffect(() => {
    const un = listen<RecordingState>("overlay:state", (e) => setState(mapState(e.payload)));
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [setState]);

  useEffect(() => {
    const un = listen<{ mode: OverlayMode }>("overlay:mode", (e) => setMode(e.payload.mode));
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [setMode]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void ipc.getRecordingState().then((recordingState) => {
        setState(mapState(recordingState));
      }).catch(() => {});
      void ipc.getRecordingMode().then(setMode).catch(() => {});
    }, 180);
    return () => window.clearInterval(timer);
  }, [setMode, setState]);

  useEffect(() => {
    const un = listen<{ level: number }>("overlay:level", (e) => {
      if (state !== "recording") return;
      const now = performance.now();
      const delta = lastLevelAt.current === null ? 50 : now - lastLevelAt.current;
      lastLevelAt.current = now;
      voiceActivity.current = nextVoiceActivity(voiceActivity.current, e.payload.level, delta);
      setLevel(voiceActivity.current.energy);
    });
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [setLevel, state]);

  useEffect(() => {
    if (state === "recording") return;
    voiceActivity.current = INITIAL_VOICE_ACTIVITY;
    lastLevelAt.current = null;
    setLevel(0);
  }, [setLevel, state]);

  return (
    <main className="flex h-screen w-screen items-center justify-center overflow-hidden bg-transparent">
      {overlayStyle === "orb" ? (
        <HaloOrb state={state} mode={mode} size={overlaySize} />
      ) : (
        <OverlayPill state={state} mode={mode} size={overlaySize} />
      )}
    </main>
  );
}
