import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useWindowMaximized } from "./use-window-maximized";

const mocks = vi.hoisted(() => ({
  isMaximized: vi.fn(),
  onResized: vi.fn(),
  unlisten: vi.fn(),
  resizeHandler: null as null | (() => void),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    isMaximized: mocks.isMaximized,
    onResized: mocks.onResized,
  }),
}));

beforeEach(() => {
  mocks.isMaximized.mockReset();
  mocks.onResized.mockReset();
  mocks.unlisten.mockReset();
  mocks.resizeHandler = null;
  mocks.onResized.mockImplementation((handler: () => void) => {
    mocks.resizeHandler = handler;
    return Promise.resolve(mocks.unlisten);
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useWindowMaximized", () => {
  it("returns false initially when isMaximized resolves false", async () => {
    mocks.isMaximized.mockResolvedValue(false);
    const { result } = renderHook(() => useWindowMaximized());
    await waitFor(() => {
      expect(mocks.isMaximized).toHaveBeenCalled();
    });
    expect(result.current).toBe(false);
  });

  it("flips to true after a tauri://resize event when isMaximized resolves true", async () => {
    mocks.isMaximized.mockResolvedValue(false);
    const { result } = renderHook(() => useWindowMaximized());
    await waitFor(() => expect(mocks.onResized).toHaveBeenCalled());

    mocks.isMaximized.mockResolvedValue(true);
    mocks.resizeHandler?.();
    await waitFor(() => expect(result.current).toBe(true));
  });

  it("flips back to false on a subsequent resize when isMaximized resolves false", async () => {
    mocks.isMaximized.mockResolvedValue(true);
    const { result } = renderHook(() => useWindowMaximized());
    await waitFor(() => expect(result.current).toBe(true));

    mocks.isMaximized.mockResolvedValue(false);
    mocks.resizeHandler?.();
    await waitFor(() => expect(result.current).toBe(false));
  });

  it("treats isMaximized rejection as not-maximized rather than throwing", async () => {
    mocks.isMaximized.mockRejectedValue(new Error("nope"));
    const { result } = renderHook(() => useWindowMaximized());
    await waitFor(() => expect(mocks.isMaximized).toHaveBeenCalled());
    expect(result.current).toBe(false);
  });

  it("unsubscribes the resize listener on unmount", async () => {
    mocks.isMaximized.mockResolvedValue(false);
    const { unmount } = renderHook(() => useWindowMaximized());
    await waitFor(() => expect(mocks.onResized).toHaveBeenCalled());
    unmount();
    expect(mocks.unlisten).toHaveBeenCalledTimes(1);
  });
});
