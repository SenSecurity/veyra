import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HaloOrb } from "./halo-orb";
import { useOverlayStore } from "@/stores/overlay-store";

afterEach(() => {
  cleanup();
  useOverlayStore.setState({
    state: "idle",
    mode: "dictation",
    level: 0,
    recordingStartedAt: null,
  });
});

const mockIpc = vi.hoisted(() => ({
  toggleRecording: vi.fn(() => Promise.resolve()),
  cancelRecording: vi.fn(() => Promise.resolve()),
}));

vi.mock("@/lib/tauri", () => ({
  ipc: mockIpc,
}));

vi.mock("@/hooks/use-settings", () => ({
  useSettings: () => ({
    settings: {
      hotkey: "F24",
      commandHotkey: "Pause",
      whisperModel: "large-v3-turbo",
      emailDraftEngine: "ollama",
      emailDraftModel: "llama3.2:1b",
      overlayStyle: "orb",
      overlaySize: "medium",
      overlayPosition: "bottom-center",
    },
  }),
}));

beforeEach(() => {
  mockIpc.toggleRecording.mockClear();
  mockIpc.cancelRecording.mockClear();
});

describe("HaloOrb", () => {
  it("renders three concentric rings while recording", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now(),
    });
    const { container } = render(<HaloOrb state="recording" mode="dictation" />);
    const rings = container.querySelectorAll(".veyra-orb-ring");
    expect(rings.length).toBe(3);
  });

  it("renders the dashed shimmer ring while transcribing and hides the pulse rings", () => {
    const { container } = render(<HaloOrb state="transcribing" mode="dictation" />);
    expect(container.querySelectorAll(".veyra-orb-ring").length).toBe(0);
    expect(container.querySelectorAll(".veyra-orb-shimmer").length).toBe(1);
  });

  it("renders no rings while idle", () => {
    const { container } = render(<HaloOrb state="idle" mode="dictation" />);
    expect(container.querySelectorAll(".veyra-orb-ring").length).toBe(0);
    expect(container.querySelectorAll(".veyra-orb-shimmer").length).toBe(0);
  });

  it("uses data-mode='stt' on dictation mode", () => {
    const { container } = render(<HaloOrb state="recording" mode="dictation" />);
    expect(container.querySelector("[data-mode='stt']")).not.toBeNull();
  });

  it("uses data-mode='drafter' on command mode", () => {
    const { container } = render(<HaloOrb state="recording" mode="command" />);
    expect(container.querySelector("[data-mode='drafter']")).not.toBeNull();
  });

  it("scales the squircle for each size variant", () => {
    const { container: c0 } = render(<HaloOrb state="recording" mode="dictation" size="smaller" />);
    const orb0 = c0.querySelector(".veyra-orb") as HTMLElement;
    expect(orb0.style.width).toBe("34px");

    cleanup();

    const { container: c1 } = render(<HaloOrb state="recording" mode="dictation" size="small" />);
    const orb1 = c1.querySelector(".veyra-orb") as HTMLElement;
    expect(orb1.style.width).toBe("42px");

    cleanup();

    const { container: c2 } = render(<HaloOrb state="recording" mode="dictation" size="medium" />);
    const orb2 = c2.querySelector(".veyra-orb") as HTMLElement;
    expect(orb2.style.width).toBe("52px");

    cleanup();

    const { container: c3 } = render(<HaloOrb state="recording" mode="dictation" size="large" />);
    const orb3 = c3.querySelector(".veyra-orb") as HTMLElement;
    expect(orb3.style.width).toBe("64px");
  });

  it("marks the selected orb size on the root wrapper", () => {
    const { container } = render(<HaloOrb state="recording" mode="dictation" size="large" />);
    expect(container.querySelector("[data-size='large']")).not.toBeNull();
  });

  it("calls toggleRecording when the orb button is clicked while recording", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now(),
    });
    render(<HaloOrb state="recording" mode="dictation" />);
    fireEvent.click(screen.getByRole("button", { name: /stop recording/i }));
    expect(mockIpc.toggleRecording).toHaveBeenCalledTimes(1);
    expect(mockIpc.cancelRecording).not.toHaveBeenCalled();
  });

  it("calls cancelRecording when the orb button is clicked while transcribing", () => {
    render(<HaloOrb state="transcribing" mode="dictation" />);
    fireEvent.click(screen.getByRole("button", { name: /cancel transcription/i }));
    expect(mockIpc.cancelRecording).toHaveBeenCalledTimes(1);
    expect(mockIpc.toggleRecording).not.toHaveBeenCalled();
  });

  it("renders the timer chip with elapsed label and rec caption while recording", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now() - 3_200,
    });
    const { container } = render(<HaloOrb state="recording" mode="dictation" />);
    const text = container.textContent ?? "";
    expect(text).toMatch(/rec/i);
    expect(text).toMatch(/00:0[2-3]\.\d/); // elapsed ~3.2s tolerant of jitter
  });

  it("renders 'transcribing…' in the chip while transcribing", () => {
    const { container } = render(<HaloOrb state="transcribing" mode="dictation" />);
    expect(container.textContent ?? "").toMatch(/transcribing/i);
  });

  it("renders the F24 hotkey hint shortly after recording starts in dictation mode", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now(),
    });
    const { container } = render(<HaloOrb state="recording" mode="dictation" />);
    expect(container.textContent ?? "").toMatch(/F24/);
  });
});
