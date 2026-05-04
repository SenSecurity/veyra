import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OverlayApp } from "./overlay-app";
import { useOverlayStore } from "@/stores/overlay-store";

const listeners = vi.hoisted(() => new Map<string, Array<(event: { payload: unknown }) => void>>());
const getOverlayLayoutMock = vi.hoisted(() =>
  vi.fn(() => Promise.resolve({ style: "capsule", size: "medium", revision: 0 })),
);

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, handler: (event: { payload: unknown }) => void) => {
    const handlers = listeners.get(event) ?? [];
    handlers.push(handler);
    listeners.set(event, handlers);
    return Promise.resolve(() => {
      listeners.set(
        event,
        (listeners.get(event) ?? []).filter((candidate) => candidate !== handler),
      );
    });
  }),
}));

vi.mock("@/lib/tauri", () => ({
  ipc: {
    getRecordingState: vi.fn(() => Promise.resolve("Ready")),
    getRecordingMode: vi.fn(() => Promise.resolve("dictation")),
    getOverlayLayout: getOverlayLayoutMock,
    toggleRecording: vi.fn(() => Promise.resolve()),
    cancelRecording: vi.fn(() => Promise.resolve()),
  },
}));

vi.mock("@/hooks/use-settings", () => ({
  useSettings: () => ({
    settings: {
      hotkey: "F24",
      commandHotkey: "Pause",
      whisperModel: "large-v3-turbo",
      emailDraftEngine: "ollama",
      emailDraftModel: "llama3.2:1b",
    },
  }),
}));

function emit(event: string, payload: unknown) {
  for (const handler of listeners.get(event) ?? []) {
    handler({ payload });
  }
}

afterEach(() => {
  cleanup();
  listeners.clear();
  useOverlayStore.setState({
    state: "idle",
    mode: "dictation",
    level: 0,
    recordingStartedAt: null,
  });
});

beforeEach(() => {
  listeners.clear();
  getOverlayLayoutMock.mockReset();
  getOverlayLayoutMock.mockResolvedValue({ style: "capsule", size: "medium", revision: 0 });
});

describe("OverlayApp", () => {
  it("bootstraps Halo Orb from the backend layout when no event arrives", async () => {
    getOverlayLayoutMock.mockResolvedValue({
      style: "orb",
      size: "smaller",
      revision: 9,
    });

    const { container } = render(<OverlayApp />);

    await waitFor(() => {
      expect(container.querySelector(".veyra-orb")).not.toBeNull();
      expect(container.querySelector(".veyra-capsule")).toBeNull();
      expect(container.querySelector("[data-size='smaller']")).not.toBeNull();
    });
  });

  it("renders Halo Orb from preview payload style even if no layout event arrives", async () => {
    const { container } = render(<OverlayApp />);

    emit("overlay:preview", {
      active: true,
      mode: "dictation",
      state: "Recording",
      style: "orb",
      size: "smaller",
      revision: 42,
    });

    await waitFor(() => {
      expect(container.querySelector(".veyra-orb")).not.toBeNull();
      expect(container.querySelector(".veyra-capsule")).toBeNull();
      expect(container.querySelector("[data-size='smaller']")).not.toBeNull();
    });
  });

  it("applies the smaller Halo Orb size from layout events", async () => {
    const { container } = render(<OverlayApp />);

    emit("overlay:layout", {
      style: "orb",
      size: "smaller",
      revision: 7,
    });
    emit("overlay:state", "Recording");

    await waitFor(() => {
      expect(container.querySelector(".veyra-orb")).not.toBeNull();
      expect(container.querySelector("[data-size='smaller']")).not.toBeNull();
    });
  });
});
