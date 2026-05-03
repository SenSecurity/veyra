import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useInstallOrchestrator } from "./use-install-orchestrator";

const mocks = vi.hoisted(() => ({
  checkModelDownloaded: vi.fn(),
  isOllamaInstalled: vi.fn(),
  checkEmailDraftModel: vi.fn(),
  downloadModel: vi.fn(),
  installOllamaRuntime: vi.fn(),
  listen: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  ipc: {
    checkModelDownloaded: mocks.checkModelDownloaded,
    isOllamaInstalled: mocks.isOllamaInstalled,
    checkEmailDraftModel: mocks.checkEmailDraftModel,
    downloadModel: mocks.downloadModel,
    installOllamaRuntime: mocks.installOllamaRuntime,
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (event: string, handler: (e: { payload: unknown }) => void) => {
    mocks.listen(event, handler);
    return Promise.resolve(mocks.unlisten);
  },
}));

const INPUT = {
  whisperModel: "turbo",
  emailDraftEngine: "ollama" as const,
  emailDraftModel: "llama3.2:1b",
  groqApiKey: "",
};

beforeEach(() => {
  for (const m of Object.values(mocks)) {
    if (typeof m === "function" && "mockReset" in m) m.mockReset();
  }
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useInstallOrchestrator", () => {
  it("seeds all three steps to 'done' when initial probes succeed", async () => {
    mocks.checkModelDownloaded.mockResolvedValue(true);
    mocks.isOllamaInstalled.mockResolvedValue(true);
    mocks.checkEmailDraftModel.mockResolvedValue(undefined);

    const { result } = renderHook(() => useInstallOrchestrator(INPUT));
    await waitFor(() => {
      expect(result.current.allDone).toBe(true);
    });
    expect(result.current.whisper.status).toBe("done");
    expect(result.current.ollama.status).toBe("done");
    expect(result.current.drafter.status).toBe("done");
  });

  it("seeds whisper as idle when checkModelDownloaded returns false", async () => {
    mocks.checkModelDownloaded.mockResolvedValue(false);
    mocks.isOllamaInstalled.mockResolvedValue(true);
    mocks.checkEmailDraftModel.mockResolvedValue(undefined);

    const { result } = renderHook(() => useInstallOrchestrator(INPUT));
    await waitFor(() => {
      expect(result.current.whisper.status).toBe("idle");
    });
    expect(result.current.allDone).toBe(false);
  });

  it("transitions whisper to 'done' after runAll() resolves", async () => {
    mocks.checkModelDownloaded.mockResolvedValue(false);
    mocks.isOllamaInstalled.mockResolvedValue(true);
    mocks.checkEmailDraftModel.mockResolvedValue(undefined);
    mocks.downloadModel.mockResolvedValue(undefined);

    const { result } = renderHook(() => useInstallOrchestrator(INPUT));
    await waitFor(() => expect(result.current.whisper.status).toBe("idle"));

    await act(async () => {
      await result.current.runAll();
    });

    expect(result.current.whisper.status).toBe("done");
    expect(mocks.downloadModel).toHaveBeenCalledTimes(1);
  });

  it("flips whisper to 'failed' when downloadModel rejects, and retry re-invokes it", async () => {
    mocks.checkModelDownloaded.mockResolvedValue(false);
    mocks.isOllamaInstalled.mockResolvedValue(true);
    mocks.checkEmailDraftModel.mockResolvedValue(undefined);
    mocks.downloadModel.mockRejectedValueOnce(new Error("network"));

    const { result } = renderHook(() => useInstallOrchestrator(INPUT));
    await waitFor(() => expect(result.current.whisper.status).toBe("idle"));

    await act(async () => {
      await result.current.runAll();
    });

    expect(result.current.whisper.status).toBe("failed");
    expect(result.current.whisper.error).toMatch(/network/);
    expect(result.current.anyFailed).toBe(true);

    mocks.downloadModel.mockResolvedValueOnce(undefined);
    await act(async () => {
      await result.current.retry("whisper");
    });

    expect(result.current.whisper.status).toBe("done");
    expect(mocks.downloadModel).toHaveBeenCalledTimes(2);
  });

  it("auto-runs drafter once Ollama is detected as installed", async () => {
    mocks.checkModelDownloaded.mockResolvedValue(true);
    mocks.isOllamaInstalled.mockResolvedValue(true);
    mocks.checkEmailDraftModel.mockResolvedValueOnce(undefined);

    const { result } = renderHook(() => useInstallOrchestrator(INPUT));
    await waitFor(() => expect(result.current.drafter.status).toBe("done"));
    expect(mocks.checkEmailDraftModel).toHaveBeenCalledTimes(1);
  });

  it("does not start a duplicate run while a step is already running", async () => {
    mocks.checkModelDownloaded.mockResolvedValue(false);
    mocks.isOllamaInstalled.mockResolvedValue(true);
    mocks.checkEmailDraftModel.mockResolvedValue(undefined);

    let resolveDownload: (() => void) | null = null;
    mocks.downloadModel.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveDownload = resolve;
        }),
    );

    const { result } = renderHook(() => useInstallOrchestrator(INPUT));
    await waitFor(() => expect(result.current.whisper.status).toBe("idle"));

    await act(async () => {
      void result.current.runAll();
      void result.current.retry("whisper");
    });

    expect(mocks.downloadModel).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveDownload?.();
      await Promise.resolve();
    });
  });

  it("runs the Ollama installer from first boot when Ollama is missing", async () => {
    mocks.checkModelDownloaded.mockResolvedValue(true);
    mocks.isOllamaInstalled
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(false)
      .mockResolvedValueOnce(true);
    mocks.checkEmailDraftModel.mockResolvedValue(undefined);
    mocks.installOllamaRuntime.mockResolvedValue(undefined);

    const { result } = renderHook(() => useInstallOrchestrator(INPUT));
    await waitFor(() => expect(result.current.ollama.status).toBe("idle"));

    vi.useFakeTimers();
    try {
      let run: Promise<void> | undefined;
      await act(async () => {
        run = result.current.runAll();
        await Promise.resolve();
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(OLLAMA_POLL_STEP_MS);
        await run;
      });

      expect(mocks.installOllamaRuntime).toHaveBeenCalledTimes(1);
      expect(result.current.ollama.status).toBe("done");
    } finally {
      vi.useRealTimers();
    }
  });
});

const OLLAMA_POLL_STEP_MS = 2_000;
