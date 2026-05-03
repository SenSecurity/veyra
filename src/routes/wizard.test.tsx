import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

afterEach(() => {
  cleanup();
});

const mockIpc = vi.hoisted(() => ({
  listMicrophones: vi.fn(() => Promise.resolve([])),
  markWizardComplete: vi.fn(() => Promise.resolve()),
}));

const mockUpdate = vi.hoisted(() => vi.fn(() => Promise.resolve()));
const mockReload = vi.hoisted(() => vi.fn(() => Promise.resolve()));
const mockNavigate = vi.hoisted(() => vi.fn());
const orchestratorMock = vi.hoisted(() => ({
  whisper: { status: "idle" as const, progress: 0 },
  ollama: { status: "idle" as const, progress: 0 },
  drafter: { status: "idle" as const, progress: 0 },
  allDone: false,
  anyFailed: false,
  anyRunning: false,
  refresh: vi.fn(),
  runAll: vi.fn(() => Promise.resolve()),
  retry: vi.fn(() => Promise.resolve()),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock("@/lib/tauri", () => ({
  ipc: mockIpc,
}));

vi.mock("@/hooks/use-settings", () => ({
  useSettings: () => ({
    settings: {
      microphone: "default",
      engine: "local",
      whisperModel: "turbo",
      emailDraftEngine: "ollama",
      emailDraftModel: "llama3.2:1b",
      groqApiKey: "",
      recordingMode: "toggle",
      hotkey: "F24",
      commandHotkey: "Pause",
      overlayStyle: "capsule",
      overlaySize: "medium",
    },
    update: mockUpdate,
    reload: mockReload,
    loading: false,
    error: null,
  }),
}));

vi.mock("@/hooks/use-install-orchestrator", () => ({
  useInstallOrchestrator: () => orchestratorMock,
}));

vi.mock("@/components/hotkey-input", () => ({
  HotkeyInput: ({ value }: { value: string }) => <input value={value} readOnly />,
}));

import { WizardRoute } from "./wizard";

beforeEach(() => {
  for (const v of Object.values(mockIpc)) {
    if (typeof v === "function" && "mockReset" in v) v.mockReset();
  }
  mockIpc.listMicrophones.mockResolvedValue([]);
  mockIpc.markWizardComplete.mockResolvedValue(undefined);
  mockUpdate.mockReset();
  mockReload.mockReset();
  mockNavigate.mockReset();
  orchestratorMock.whisper = { status: "idle", progress: 0 };
  orchestratorMock.ollama = { status: "idle", progress: 0 };
  orchestratorMock.drafter = { status: "idle", progress: 0 };
  orchestratorMock.allDone = false;
  orchestratorMock.anyFailed = false;
  orchestratorMock.anyRunning = false;
  orchestratorMock.runAll = vi.fn(() => Promise.resolve());
  orchestratorMock.retry = vi.fn(() => Promise.resolve());
});

describe("WizardRoute", () => {
  it("starts on the Welcome step with both Speech to Text and Email Drafter cards", () => {
    render(<WizardRoute />);
    expect(screen.getByText("Set up Veyra")).toBeInTheDocument();
    expect(screen.getByText("Speech to Text")).toBeInTheDocument();
    expect(screen.getByText("Email Drafter")).toBeInTheDocument();
  });

  it("advances to the Install step and renders three install cards", async () => {
    render(<WizardRoute />);
    fireEvent.click(screen.getByRole("button", { name: /next/i }));
    expect(screen.getByRole("heading", { level: 1 }).textContent).toMatch(/Install everything/);
    expect(screen.getByText("Whisper")).toBeInTheDocument();
    expect(screen.getByText("Ollama runtime")).toBeInTheDocument();
    expect(screen.getByText("Email model")).toBeInTheDocument();
  });

  it("clicking Install everything calls orchestrator.runAll exactly once", async () => {
    render(<WizardRoute />);
    fireEvent.click(screen.getByRole("button", { name: /next/i }));
    fireEvent.click(screen.getByRole("button", { name: /^install everything$/i }));
    await waitFor(() => expect(orchestratorMock.runAll).toHaveBeenCalledTimes(1));
  });

  it("renders a Retry button on a failed step and clicking it calls retry('whisper')", async () => {
    orchestratorMock.whisper = { status: "failed", progress: 0, error: "boom" };
    orchestratorMock.anyFailed = true;
    render(<WizardRoute />);
    fireEvent.click(screen.getByRole("button", { name: /next/i }));
    const retry = screen.getByRole("button", { name: /retry/i });
    fireEvent.click(retry);
    await waitFor(() => expect(orchestratorMock.retry).toHaveBeenCalledWith("whisper"));
  });

  it("Start using Veyra is disabled until orchestrator.allDone is true", async () => {
    render(<WizardRoute />);
    // step through Welcome -> Install -> Microphone -> Hotkeys -> Ready
    for (let i = 0; i < 4; i++) {
      fireEvent.click(screen.getByRole("button", { name: /next/i }));
    }
    const start = screen.getByRole("button", { name: /start using veyra/i });
    expect(start).toBeDisabled();
  });

  it("Start using Veyra calls markWizardComplete and navigate when allDone is true", async () => {
    orchestratorMock.whisper = { status: "done", progress: 100 };
    orchestratorMock.ollama = { status: "done", progress: 100 };
    orchestratorMock.drafter = { status: "done", progress: 100 };
    orchestratorMock.allDone = true;
    render(<WizardRoute />);
    for (let i = 0; i < 4; i++) {
      fireEvent.click(screen.getByRole("button", { name: /next/i }));
    }
    const start = screen.getByRole("button", { name: /start using veyra/i });
    expect(start).not.toBeDisabled();
    fireEvent.click(start);
    await waitFor(() => expect(mockIpc.markWizardComplete).toHaveBeenCalledTimes(1));
    expect(mockNavigate).toHaveBeenCalledWith({ to: "/" });
  });
});
