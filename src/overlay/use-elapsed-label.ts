import { useEffect, useState } from "react";
import type { OverlayState } from "@/stores/overlay-store";

/** Format a millisecond duration as `mm:ss.t`. Negative inputs clamp to zero. */
export function formatElapsed(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) ms = 0;
  const total = Math.floor(ms);
  const minutes = Math.floor(total / 60_000);
  const seconds = Math.floor((total % 60_000) / 1000);
  const tenths = Math.floor((total % 1000) / 100);
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${tenths}`;
}

/**
 * Render-time elapsed label for overlay surfaces. Polls every 100 ms while
 * the user is recording; freezes at the recording-end value during
 * `transcribing`; shows `00:00.0` when idle / no session.
 */
export function useElapsedLabel(
  startedAt: number | null,
  state: OverlayState,
): string {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (state !== "recording" || startedAt == null) return;
    const id = window.setInterval(() => setNow(Date.now()), 100);
    return () => window.clearInterval(id);
  }, [startedAt, state]);

  if (startedAt == null) return "00:00.0";
  if (state === "transcribing") return formatElapsed(Math.max(0, now - startedAt));
  if (state !== "recording") return "00:00.0";
  return formatElapsed(now - startedAt);
}
