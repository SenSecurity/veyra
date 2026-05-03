import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EngineBadge } from "./engine-badge";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@/lib/tauri", () => ({
  ipc: {
    getRecordingState: () => Promise.resolve("Ready"),
  },
}));

vi.mock("@/hooks/use-settings", () => ({
  useSettings: () => ({
    settings: {
      whisperModel: "large-v3-turbo",
      emailDraftEngine: "ollama",
      emailDraftModel: "llama3.2:1b",
    },
  }),
}));

describe("EngineBadge", () => {
  it("renders both engine segments with role captions", () => {
    render(<EngineBadge />);
    expect(screen.getByText("STT")).toBeInTheDocument();
    expect(screen.getByText("Drafter")).toBeInTheDocument();
  });

  it("formats the Whisper turbo model into the canonical name", () => {
    render(<EngineBadge />);
    expect(screen.getAllByText(/Whisper.*Turbo/i).length).toBeGreaterThan(0);
  });

  it("formats the Llama 3.2 1B Ollama model into a Glacier-style name", () => {
    render(<EngineBadge />);
    expect(screen.getAllByText(/Llama.*3\.2.*1B/).length).toBeGreaterThan(0);
  });
});
