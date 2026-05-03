import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsOverlayRoute } from "./overlay";
import { useSettingsStore } from "@/stores/settings-store";

afterEach(() => cleanup());

const mockUpdate = vi.hoisted(() => vi.fn(() => Promise.resolve()));

vi.mock("@/hooks/use-settings", () => ({
  useSettings: () => ({
    settings: {
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
    },
    update: mockUpdate,
    error: null,
  }),
}));

beforeEach(() => {
  mockUpdate.mockClear();
  useSettingsStore.setState({
    settings: null,
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

  it("calls update with overlayStyle: 'orb' when the Halo Orb card is clicked", () => {
    render(<SettingsOverlayRoute />);
    const orbButton = screen.getByText("Halo Orb").closest("button");
    expect(orbButton).not.toBeNull();
    fireEvent.click(orbButton!);
    expect(mockUpdate).toHaveBeenCalledWith({ overlayStyle: "orb" });
  });

  it("calls update with overlaySize: 'large' when the Large segment is clicked", () => {
    render(<SettingsOverlayRoute />);
    fireEvent.click(screen.getByRole("radio", { name: /large/i }));
    expect(mockUpdate).toHaveBeenCalledWith({ overlaySize: "large" });
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
});
