import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
  overlayPosition: "bottom-center",
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

  it("renders all four size segments", () => {
    render(<SettingsOverlayRoute />);
    const sizeGroup = screen.getByRole("radiogroup", { name: /overlay size/i });
    expect(within(sizeGroup).getAllByRole("radio")).toHaveLength(4);
    expect(screen.getByText("Smaller")).toBeInTheDocument();
    expect(screen.getByText("Small")).toBeInTheDocument();
    expect(screen.getByText("Medium")).toBeInTheDocument();
    expect(screen.getByText("Large")).toBeInTheDocument();
  });

  it("renders the nine screen position anchors", () => {
    render(<SettingsOverlayRoute />);
    const positionGroup = screen.getByRole("radiogroup", { name: /overlay position/i });
    expect(within(positionGroup).getAllByRole("radio")).toHaveLength(9);
    expect(within(positionGroup).getByRole("radio", { name: /bottom center/i })).toHaveAttribute(
      "aria-checked",
      "true",
    );
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
    const sizeGroup = screen.getByRole("radiogroup", { name: /overlay size/i });
    fireEvent.click(within(sizeGroup).getByRole("radio", { name: /large/i }));
    expect(mockSaveSettings).toHaveBeenCalledWith({
      ...defaultSettings,
      overlaySize: "large",
    });
    await waitFor(() => expect(mockSetOverlayLayout).toHaveBeenCalledWith("capsule", "large"));
  });

  it("calls update with overlayPosition when a position anchor is clicked", async () => {
    render(<SettingsOverlayRoute />);
    const positionGroup = screen.getByRole("radiogroup", { name: /overlay position/i });
    fireEvent.click(within(positionGroup).getByRole("radio", { name: /top right/i }));
    expect(mockSaveSettings).toHaveBeenCalledWith({
      ...defaultSettings,
      overlayPosition: "top-right",
    });
    await waitFor(() => expect(mockSetOverlayLayout).toHaveBeenCalledWith("capsule", "medium"));
  });

  it("renders the capsule dimension caption when style is capsule", () => {
    render(<SettingsOverlayRoute />);
    expect(screen.getByText("292 x 60")).toBeInTheDocument();
  });

  it("marks the current selection as aria-pressed/aria-checked", () => {
    render(<SettingsOverlayRoute />);
    const capsuleCard = screen.getByText("Capsule").closest("button");
    expect(capsuleCard?.getAttribute("aria-pressed")).toBe("true");
    const sizeGroup = screen.getByRole("radiogroup", { name: /overlay size/i });
    expect(
      within(sizeGroup).getByRole("radio", { name: /medium/i }).getAttribute("aria-checked"),
    ).toBe("true");
  });

  it("renders a single preview toggle button", () => {
    render(<SettingsOverlayRoute />);
    expect(screen.getByRole("button", { name: /show preview/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /preview stt/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /preview drafter/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /preview transcribing/i })).not.toBeInTheDocument();
  });

  it("toggles the live overlay preview with the same button", async () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("button", { name: /show preview/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "capsule",
      "medium",
      "dictation",
      "Recording",
    );

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /hide preview/i })).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByRole("button", { name: /hide preview/i }));
    expect(mockHideOverlayPreview).toHaveBeenCalledTimes(1);
  });

  it("previews the newly selected Halo Orb style instead of stale capsule state", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByText("Halo Orb").closest("button")!);
    fireEvent.click(screen.getByRole("button", { name: /show preview/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "orb",
      "medium",
      "dictation",
      "Recording",
    );
  });

  it("previews the newly selected size instead of stale medium state", () => {
    render(<SettingsOverlayRoute />);
    const sizeGroup = screen.getByRole("radiogroup", { name: /overlay size/i });
    fireEvent.click(within(sizeGroup).getByRole("radio", { name: /smaller/i }));
    fireEvent.click(screen.getByRole("button", { name: /show preview/i }));
    expect(mockPreviewOverlay).toHaveBeenCalledWith(
      "capsule",
      "smaller",
      "dictation",
      "Recording",
    );
  });

});
