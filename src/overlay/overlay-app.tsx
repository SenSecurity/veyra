import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { OverlayPill } from "./pill";
import { ipc } from "@/lib/tauri";
import { useOverlayStore, type OverlayMode, type OverlayState } from "@/stores/overlay-store";
import type { RecordingState } from "@/types/ipc";

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

  useEffect(() => {
    void ipc.getRecordingState().then((recordingState) => {
      setState(mapState(recordingState));
    }).catch(() => {});
  }, [setState]);

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
    }, 180);
    return () => window.clearInterval(timer);
  }, [setState]);

  useEffect(() => {
    const un = listen<{ level: number }>("overlay:level", (e) => setLevel(e.payload.level));
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [setLevel]);

  useEffect(() => {
    if (state !== "recording") {
      setLevel(0);
      return;
    }
    let cancelled = false;
    const timer = window.setInterval(() => {
      void ipc.getRecordingLevel().then((level) => {
        if (!cancelled) setLevel(level);
      }).catch(() => {});
    }, 50);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [setLevel, state]);

  return (
    <main className="flex h-screen w-screen items-center justify-center overflow-hidden bg-transparent">
      <OverlayPill state={state} mode={mode} />
    </main>
  );
}
