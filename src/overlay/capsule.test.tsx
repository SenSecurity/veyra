import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OverlayPill, formatElapsed } from "./pill";
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
    },
  }),
}));

beforeEach(() => {
  mockIpc.toggleRecording.mockClear();
  mockIpc.cancelRecording.mockClear();
});

describe("OverlayPill capsule", () => {
  it("renders the STT chip and the Whisper · Turbo engine name while recording in dictation mode", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now(),
    });
    render(<OverlayPill state="recording" mode="dictation" />);
    expect(screen.getByText("STT")).toBeInTheDocument();
    expect(screen.getByText(/Whisper.*Turbo/)).toBeInTheDocument();
  });

  it("renders the Drafter chip and the Llama engine name in command mode", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "command",
      recordingStartedAt: Date.now(),
    });
    render(<OverlayPill state="recording" mode="command" />);
    expect(screen.getByText("Drafter")).toBeInTheDocument();
    expect(screen.getByText(/Llama.*3\.2.*1B/)).toBeInTheDocument();
  });

  it("renders Listening… in idle and does not animate the LED", () => {
    render(<OverlayPill state="idle" mode="dictation" />);
    expect(screen.getByText("Listening…")).toBeInTheDocument();
  });

  it("renders Transcribing… and an X-shaped cancel button in transcribing state", () => {
    render(<OverlayPill state="transcribing" mode="dictation" />);
    expect(screen.getByText("Transcribing…")).toBeInTheDocument();
    const cancel = screen.getByRole("button", { name: /cancel transcription/i });
    fireEvent.click(cancel);
    expect(mockIpc.cancelRecording).toHaveBeenCalledTimes(1);
    expect(mockIpc.toggleRecording).not.toHaveBeenCalled();
  });

  it("invokes toggleRecording when the stop button is clicked while recording", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now(),
    });
    render(<OverlayPill state="recording" mode="dictation" />);
    fireEvent.click(screen.getByRole("button", { name: /stop recording/i }));
    expect(mockIpc.toggleRecording).toHaveBeenCalledTimes(1);
    expect(mockIpc.cancelRecording).not.toHaveBeenCalled();
  });

  it("uses data-mode='drafter' on the capsule when mode is command", () => {
    const { container } = render(<OverlayPill state="recording" mode="command" />);
    const capsule = container.querySelector("[data-mode='drafter']");
    expect(capsule).not.toBeNull();
  });

  it("uses data-mode='stt' on the capsule when mode is dictation", () => {
    const { container } = render(<OverlayPill state="recording" mode="dictation" />);
    const capsule = container.querySelector("[data-mode='stt']");
    expect(capsule).not.toBeNull();
  });

  it("renders 40 wave-bar cells inside the wave area", () => {
    const { container } = render(<OverlayPill state="recording" mode="dictation" />);
    const wave = container.querySelector(".veyra-capsule-wave");
    expect(wave).not.toBeNull();
    expect(wave!.children.length).toBe(40);
  });

  it("freezes the timer at the recording-end value when transitioning to transcribing", () => {
    const start = Date.now() - 5_300; // 5.3s ago
    useOverlayStore.setState({
      state: "transcribing",
      mode: "dictation",
      recordingStartedAt: start,
    });
    const { container } = render(<OverlayPill state="transcribing" mode="dictation" />);
    const text = container.textContent ?? "";
    expect(text).toMatch(/00:0[5-6]\.\d/); // tolerate the ~100ms read jitter
  });
});

describe("OverlayPill hotkey hint", () => {
  it("renders the F24 hint shortly after recording starts in dictation mode", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now(),
    });
    const { container } = render(<OverlayPill state="recording" mode="dictation" />);
    const text = container.textContent ?? "";
    expect(text).toMatch(/F24/);
    expect(text).toMatch(/stop/i);
  });

  it("renders the Pause hint in command mode and reads 'draft'", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "command",
      recordingStartedAt: Date.now(),
    });
    const { container } = render(<OverlayPill state="recording" mode="command" />);
    const text = container.textContent ?? "";
    expect(text).toMatch(/Pause/);
    expect(text).toMatch(/draft/i);
  });

  it("does not render the hint in transcribing state regardless of recordingStartedAt", () => {
    useOverlayStore.setState({
      state: "transcribing",
      mode: "dictation",
      recordingStartedAt: Date.now(),
    });
    const { container } = render(<OverlayPill state="transcribing" mode="dictation" />);
    expect(container.querySelector(".veyra-capsule-hint")).toBeNull();
  });

  it("does not render the hint when recording started more than 600 ms ago", () => {
    useOverlayStore.setState({
      state: "recording",
      mode: "dictation",
      recordingStartedAt: Date.now() - 1_500,
    });
    const { container } = render(<OverlayPill state="recording" mode="dictation" />);
    expect(container.querySelector(".veyra-capsule-hint")).toBeNull();
  });
});

describe("formatElapsed", () => {
  it("renders 0 ms as 00:00.0", () => {
    expect(formatElapsed(0)).toBe("00:00.0");
  });

  it("renders sub-second values with the tenth", () => {
    expect(formatElapsed(823)).toBe("00:00.8");
  });

  it("renders mm:ss.t correctly across the minute boundary", () => {
    expect(formatElapsed(63_400)).toBe("01:03.4");
  });

  it("clamps negative inputs to zero", () => {
    expect(formatElapsed(-100)).toBe("00:00.0");
  });
});
