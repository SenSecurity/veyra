import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HomeRoute } from "./index";
import type { Transcription } from "@/types/ipc";

afterEach(() => cleanup());

const mockIpc = vi.hoisted(() => ({
  getStatsTotals: vi.fn(),
  getStatsStreak: vi.fn(),
  listTranscriptions: vi.fn(),
  toggleRecording: vi.fn(),
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

vi.mock("@tanstack/react-router", () => ({
  Link: ({
    children,
    className,
    to,
  }: {
    children?: React.ReactNode;
    className?: string;
    to?: string;
  }) => (
    <a className={className} href={to}>
      {children}
    </a>
  ),
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

function row(overrides: Partial<Transcription> = {}): Transcription {
  return {
    id: 1,
    createdAt: Date.now(),
    rawText: "raw",
    finalText: "Olá, escrevo para informar.",
    wordCount: 36,
    durationMs: 1200,
    language: "pt-PT",
    engine: "whisper",
    model: "turbo",
    appContext: null,
    mode: "command",
    enhanced: true,
    ...overrides,
  };
}

describe("HomeRoute", () => {
  it("calls all three stats IPC methods on mount", async () => {
    mockIpc.getStatsTotals.mockResolvedValue({
      wordCount: 3482,
      sessionCount: 12,
      totalDurationMs: 0,
    });
    mockIpc.getStatsStreak.mockResolvedValue({ current: 4, longest: 9 });
    mockIpc.listTranscriptions.mockResolvedValue([] as Transcription[]);

    render(<HomeRoute />);

    await waitFor(() => {
      expect(mockIpc.getStatsTotals).toHaveBeenCalledTimes(1);
      expect(mockIpc.getStatsStreak).toHaveBeenCalledTimes(1);
      expect(mockIpc.listTranscriptions).toHaveBeenCalledTimes(1);
    });
  });

  it("renders the four KPI cells with the expected labels", async () => {
    mockIpc.getStatsTotals.mockResolvedValue({
      wordCount: 3482,
      sessionCount: 12,
      totalDurationMs: 0,
    });
    mockIpc.getStatsStreak.mockResolvedValue({ current: 4, longest: 9 });
    mockIpc.listTranscriptions.mockResolvedValue([] as Transcription[]);

    render(<HomeRoute />);

    await waitFor(() => {
      expect(screen.getByText("Sessions today")).toBeInTheDocument();
    });
    expect(screen.getByText("Words transcribed")).toBeInTheDocument();
    expect(screen.getByText("Drafts composed")).toBeInTheDocument();
    expect(screen.getByText("STT latency · p50")).toBeInTheDocument();
  });

  it("renders Draft and Dictation tags from recent activity", async () => {
    mockIpc.getStatsTotals.mockResolvedValue(null);
    mockIpc.getStatsStreak.mockResolvedValue(null);
    mockIpc.listTranscriptions.mockResolvedValue([
      row({ id: 1, mode: "command", finalText: "Olá Bruno, hoje passo aí." }),
      row({
        id: 2,
        mode: "dictation",
        finalText: "faz-me um email a dizer.",
        wordCount: 18,
      }),
    ]);

    render(<HomeRoute />);

    await waitFor(() => {
      expect(screen.getByText("Draft")).toBeInTheDocument();
      expect(screen.getByText("Dictation")).toBeInTheDocument();
    });
  });

  it("renders the EmptyState when there is no recent activity", async () => {
    mockIpc.getStatsTotals.mockResolvedValue(null);
    mockIpc.getStatsStreak.mockResolvedValue(null);
    mockIpc.listTranscriptions.mockResolvedValue([] as Transcription[]);

    render(<HomeRoute />);

    await waitFor(() => {
      expect(screen.getByText(/No transcriptions yet/i)).toBeInTheDocument();
    });
  });

  it("does not render any ⌘ glyph in the Home DOM", async () => {
    mockIpc.getStatsTotals.mockResolvedValue(null);
    mockIpc.getStatsStreak.mockResolvedValue(null);
    mockIpc.listTranscriptions.mockResolvedValue([] as Transcription[]);

    const { container } = render(<HomeRoute />);
    await waitFor(() => {
      expect(screen.getByText("Sessions today")).toBeInTheDocument();
    });
    expect(container.textContent ?? "").not.toMatch(/⌘/);
  });
});
