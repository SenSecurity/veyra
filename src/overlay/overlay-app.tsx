import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { HaloOrb } from "./halo-orb";
import { OverlayPill } from "./pill";
import { INITIAL_VOICE_ACTIVITY, nextVoiceActivity } from "./voice-activity";
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
  const setRecordingStartedAt = useOverlayStore((s) => s.setRecordingStartedAt);
  // The overlay webview is hidden and receives live layout from Rust before
  // it is shown. Do not load settings here: a late settings fetch can race
  // with `overlay:layout` and put Capsule content inside an Orb-sized window.
  const [overlayStyle, setOverlayStyle] = useState<OverlayStyle>("capsule");
  const [overlaySize, setOverlaySize] = useState<OverlaySize>("medium");
  const [layoutRevision, setLayoutRevision] = useState(0);
  const [previewActive, setPreviewActive] = useState(false);
  const previewActiveRef = useRef(false);

  useEffect(() => {
    previewActiveRef.current = previewActive;
  }, [previewActive]);

  useEffect(() => {
    const un = listen<{ style: OverlayStyle; size: OverlaySize; revision?: number }>(
      "overlay:layout",
      (e) => {
        if (e.payload.style === "capsule" || e.payload.style === "orb") {
          setOverlayStyle(e.payload.style);
        }
        if (
          e.payload.size === "small" ||
          e.payload.size === "medium" ||
          e.payload.size === "large"
        ) {
          setOverlaySize(e.payload.size);
        }
        if (typeof e.payload.revision === "number") {
          setLayoutRevision(e.payload.revision);
        }
      },
    );
    return () => void un.then((fn) => fn()).catch(() => {});
  }, []);

  const voiceActivity = useRef(INITIAL_VOICE_ACTIVITY);
  const lastLevelAt = useRef<number | null>(null);

  useEffect(() => {
    void ipc.getRecordingState().then((recordingState) => {
      if (previewActiveRef.current) return;
      setState(mapState(recordingState));
    }).catch(() => {});
    void ipc.getRecordingMode().then((recordingMode) => {
      if (!previewActiveRef.current) setMode(recordingMode);
    }).catch(() => {});
  }, [setMode, setState]);

  useEffect(() => {
    const un = listen<RecordingState>("overlay:state", (e) => {
      if (previewActiveRef.current) return;
      setState(mapState(e.payload));
    });
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [setState]);

  useEffect(() => {
    const un = listen<{ mode: OverlayMode }>("overlay:mode", (e) => {
      if (previewActiveRef.current) return;
      setMode(e.payload.mode);
    });
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [setMode]);

  useEffect(() => {
    const un = listen<{
      active: boolean;
      mode?: OverlayMode;
      state?: RecordingState;
    }>("overlay:preview", (e) => {
      if (!e.payload.active) {
        setPreviewActive(false);
        setState("idle");
        setLevel(0);
        setRecordingStartedAt(null);
        return;
      }

      setPreviewActive(true);
      if (e.payload.mode === "dictation" || e.payload.mode === "command") {
        setMode(e.payload.mode);
      }
      if (e.payload.state) {
        setState(mapState(e.payload.state));
      }
    });
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [setLevel, setMode, setRecordingStartedAt, setState]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      void ipc.getRecordingState().then((recordingState) => {
        if (previewActiveRef.current) return;
        setState(mapState(recordingState));
      }).catch(() => {});
      void ipc.getRecordingMode().then((recordingMode) => {
        if (!previewActiveRef.current) setMode(recordingMode);
      }).catch(() => {});
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

  const overlayKey = `${overlayStyle}:${overlaySize}:${layoutRevision}`;

  return (
    <main className="flex h-screen w-screen items-center justify-center overflow-hidden bg-transparent">
      {overlayStyle === "orb" ? (
        <HaloOrb key={overlayKey} state={state} mode={mode} size={overlaySize} />
      ) : (
        <OverlayPill key={overlayKey} state={state} mode={mode} size={overlaySize} />
      )}
    </main>
  );
}
