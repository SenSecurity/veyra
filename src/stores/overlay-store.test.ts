import { afterEach, describe, expect, it } from "vitest";
import { useOverlayStore } from "./overlay-store";

afterEach(() => {
  useOverlayStore.setState({
    state: "idle",
    mode: "dictation",
    level: 0,
    recordingStartedAt: null,
  });
});

describe("useOverlayStore.setState transitions", () => {
  it("stamps recordingStartedAt when transitioning idle -> recording", () => {
    const before = Date.now();
    useOverlayStore.getState().setState("recording");
    const ts = useOverlayStore.getState().recordingStartedAt;
    expect(ts).not.toBeNull();
    expect(ts!).toBeGreaterThanOrEqual(before);
    expect(ts!).toBeLessThanOrEqual(Date.now());
  });

  it("freezes recordingStartedAt across recording -> transcribing", () => {
    useOverlayStore.getState().setState("recording");
    const stamped = useOverlayStore.getState().recordingStartedAt!;
    useOverlayStore.getState().setState("transcribing");
    expect(useOverlayStore.getState().recordingStartedAt).toBe(stamped);
  });

  it("resets recordingStartedAt when returning to idle", () => {
    useOverlayStore.getState().setState("recording");
    expect(useOverlayStore.getState().recordingStartedAt).not.toBeNull();
    useOverlayStore.getState().setState("transcribing");
    useOverlayStore.getState().setState("idle");
    expect(useOverlayStore.getState().recordingStartedAt).toBeNull();
  });

  it("does not change anything when the same state is dispatched twice", () => {
    useOverlayStore.getState().setState("recording");
    const first = useOverlayStore.getState().recordingStartedAt!;
    useOverlayStore.getState().setState("recording");
    expect(useOverlayStore.getState().recordingStartedAt).toBe(first);
  });

  it("resets when dispatching idle from idle (no spurious timestamp)", () => {
    useOverlayStore.getState().setState("idle");
    expect(useOverlayStore.getState().recordingStartedAt).toBeNull();
  });

  it("setRecordingStartedAt allows manual override (for tests / edge cases)", () => {
    useOverlayStore.getState().setRecordingStartedAt(1234);
    expect(useOverlayStore.getState().recordingStartedAt).toBe(1234);
  });
});
