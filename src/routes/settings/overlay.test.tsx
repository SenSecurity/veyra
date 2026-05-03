import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsOverlayRoute } from "./overlay";
import { useSettingsStore } from "@/stores/settings-store";
import type { Settings } from "@/types/settings";

afterEach(() => cleanup());

const mockPreviewOverlay = vi.hoisted(() => vi.fn(() => Promise.resolve()));
const mockHideOverlayPreview = vi.hoisted(() => vi.fn(() => Promise.resolve()));
const mockSaveSettings = vi.hoisted(() => vi.fn(() => Promise.resolve()));
const mockSetOverlayLayout = vi.hoisted(() => vi.fn(() => Promise.resolve()));

const defaultSettings: Settings = {
  microphone: "default",
  engine: "local",
  whisperModel: "large-v3-turbo",
  emailDraftEngine: "ollama",
  emailDraftModel: "llama3.2:1b",
  groqApiKey: "",
  recordingMode: "toggle",
  hotkey: "F24",
  commandHotkey: "Pause",
  overlayStyle: "capsule",
  overlaySize: "medium",
};

vi.mock("@/lib/tauri", () => ({
  ipc: {
    getSettings: vi.fn(() => Promise.resolve(defaultSettings)),
    saveSettings: mockSaveSettings,
    setOverlayLayout: mockSetOverlayLayout,
    previewOverlay: mockPreviewOverlay,
    hideOverlayPreview: mockHideOverlayPreview,
  },
}));

beforeEach(() => {
  mockPreviewOverlay.mockClear();
  mockHideOverlayPreview.mockClear();
  mockSaveSettings.mockClear();
  mockSetOverlayLayout.mockClear();
  useSettingsStore.setState({
    settings: defaultSettings,
    loading: false,
    error: null,
  });
});

describe("SettingsOverlayRoute", () => {
  it("renders both style options", () => {
    render(<SettingsOverlayRoute />);
    expect(screen.getByText("Capsule")).toBeInTheDocument();
    expect(screen.getByText("Halo Orb")).toBeInTheDocument();
  });

  it("renders all three size segments", () => {
    render(<SettingsOverlayRoute />);
    expect(screen.getByRole("radio", { name: /small/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /medium/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /large/i })).toBeInTheDocument();
  });

  it("calls update with overlayStyle: 'orb' when the Halo Orb card is clicked", async () => {
    render(<SettingsOverlayRoute />);
    const orbButton = screen.getByText("Halo Orb").closest("button");
    expect(orbButton).not.toBeNull();
    fireEvent.click(orbButton!);
    expect(mockSaveSettings).toHaveBeenCalledWith({
      ...defaultSettings,
      overlayStyle: "orb",
    });
    await waitFor(() => expect(mockSetOverlayLayout).toHaveBeenCalledWith("orb", "medium"));
  });

  it("calls update with overlaySize: 'large' when the Large segment is clicked", async () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("radio", { name: /large/i }));
    expect(mockSaveSettings).toHaveBeenCalledWith({
      ...defaultSettings,
      overlaySize: "large",
    });
    await waitFor(() => expect(mockSetOverlayLayout).toHaveBeenCalledWith("capsule", "large"));
  });

  it("renders the capsule dimension caption when style is capsule", () => {
    render(<SettingsOverlayRoute />);
    expect(screen.getByText("560 × 96")).toBeInTheDocument();
  });

  it("marks the current selection as aria-pressed/aria-checked", () => {
    render(<SettingsOverlayRoute />);
    const capsuleCard = screen.getByText("Capsule").closest("button");
    expect(capsuleCard?.getAttribute("aria-pressed")).toBe("true");
    expect(
      screen.getByRole("radio", { name: /medium/i }).getAttribute("aria-checked"),
    ).toBe("true");
  });

  it("previews STT recording with the current style and size", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("button", { name: /preview stt/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "capsule",
      "medium",
      "dictation",
      "Recording",
    );
  });

  it("previews the newly selected Halo Orb style instead of stale capsule state", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByText("Halo Orb").closest("button")!);
    fireEvent.click(screen.getByRole("button", { name: /preview stt/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "orb",
      "medium",
      "dictation",
      "Recording",
    );
  });

  it("previews the newly selected size instead of stale medium state", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("radio", { name: /small/i }));
    fireEvent.click(screen.getByRole("button", { name: /preview stt/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "capsule",
      "small",
      "dictation",
      "Recording",
    );
  });

  it("previews email drafter recording with the current style and size", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("button", { name: /preview drafter/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "capsule",
      "medium",
      "command",
      "Recording",
    );
  });

  it("previews transcribing state with the current style and size", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("button", { name: /preview transcribing/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "capsule",
      "medium",
      "dictation",
      "Transcribing",
    );
  });

  it("hides the live overlay preview", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("button", { name: /hide preview/i }));
    expect(mockHideOverlayPreview).toHaveBeenCalledTimes(1);
  });
});
