import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc } from "@/lib/tauri";

export type StepStatus = "idle" | "running" | "done" | "failed";

export interface StepState {
  status: StepStatus;
  progress: number; // 0..100
  detail?: string; // free-text, e.g. "Downloading 64%"
  error?: string;
}

export interface InstallOrchestratorState {
  whisper: StepState;
  ollama: StepState;
  drafter: StepState;
  /** True when every step is in `done`. Wizard's Ready button reads this. */
  allDone: boolean;
  /** True when at least one step is in `failed`. */
  anyFailed: boolean;
  /** True while any step is `running`. */
  anyRunning: boolean;
}

export interface InstallOrchestratorApi extends InstallOrchestratorState {
  /** Probe each install's current state from Rust without starting work. */
  refresh: () => Promise<void>;
  /** Run installs in dependency order: Ollama, email model, then Whisper. */
  runAll: () => Promise<void>;
  /** Re-run a specific step; safe to call regardless of current status. */
  retry: (step: "whisper" | "ollama" | "drafter") => Promise<void>;
}

export interface InstallOrchestratorInput {
  whisperModel: string;
  emailDraftEngine: "ollama" | "groq";
  emailDraftModel: string;
  groqApiKey: string;
}

const OLLAMA_POLL_INTERVAL_MS = 2_000;
const OLLAMA_POLL_TIMEOUT_MS = 5 * 60_000;

const INITIAL: StepState = { status: "idle", progress: 0 };

export function useInstallOrchestrator(
  input: InstallOrchestratorInput,
): InstallOrchestratorApi {
  const [whisper, setWhisper] = useState<StepState>(INITIAL);
  const [ollama, setOllama] = useState<StepState>(INITIAL);
  const [drafter, setDrafter] = useState<StepState>(INITIAL);

  // Refs let async callbacks read the freshest values without going stale.
  const whisperRef = useRef(whisper);
  const ollamaRef = useRef(ollama);
  const drafterRef = useRef(drafter);
  const inputRef = useRef(input);
  whisperRef.current = whisper;
  ollamaRef.current = ollama;
  drafterRef.current = drafter;
  inputRef.current = input;

  // ---------- Initial probes ----------

  const probeWhisper = useCallback(async () => {
    try {
      const ready = await ipc.checkModelDownloaded(inputRef.current.whisperModel);
      setWhisper((s) =>
        ready ? { ...s, status: "done", progress: 100 } : { ...INITIAL },
      );
    } catch {
      setWhisper(INITIAL);
    }
  }, []);

  const probeOllama = useCallback(async () => {
    try {
      const installed = await ipc.isOllamaInstalled();
      setOllama((s) =>
        installed ? { ...s, status: "done", progress: 100 } : { ...INITIAL },
      );
    } catch {
      setOllama(INITIAL);
    }
  }, []);

  const probeDrafter = useCallback(async () => {
    try {
      await ipc.checkEmailDraftModel(
        inputRef.current.groqApiKey,
        inputRef.current.emailDraftEngine,
        inputRef.current.emailDraftModel,
      );
      setDrafter((s) => ({ ...s, status: "done", progress: 100 }));
    } catch {
      setDrafter(INITIAL);
    }
  }, []);

  const refresh = useCallback(async () => {
    await Promise.allSettled([probeWhisper(), probeOllama(), probeDrafter()]);
  }, [probeWhisper, probeOllama, probeDrafter]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // ---------- Live progress subscriptions ----------

  useEffect(() => {
    const un = listen<{
      modelSize: string;
      downloaded: number;
      total: number;
      percent: number;
    }>("model:download:progress", (event) => {
      if (event.payload.modelSize !== inputRef.current.whisperModel) return;
      const pct = Math.max(0, Math.min(100, event.payload.percent));
      setWhisper((s) =>
        s.status === "running"
          ? { ...s, progress: pct, detail: `Downloading ${Math.round(pct)}%` }
          : s,
      );
    });
    return () => void un.then((fn) => fn()).catch(() => {});
  }, []);

  useEffect(() => {
    const un = listen<{
      model: string;
      downloaded: number;
      total: number;
      percent: number;
      status: string;
    }>("email-model:download:progress", (event) => {
      if (event.payload.model !== inputRef.current.emailDraftModel) return;
      const pct = Math.max(0, Math.min(100, event.payload.percent));
      setDrafter((s) =>
        s.status === "running"
          ? {
              ...s,
              progress: pct,
              detail: event.payload.status || `Pulling ${Math.round(pct)}%`,
            }
          : s,
      );
    });
    return () => void un.then((fn) => fn()).catch(() => {});
  }, []);

  // ---------- Step runners ----------

  const runWhisper = useCallback(async () => {
    if (whisperRef.current.status === "running") return;
    // Update the ref synchronously so a second call within the same tick
    // (before React flushes setState) sees `running` and bails.
    whisperRef.current = { status: "running", progress: 0, detail: "Starting" };
    setWhisper(whisperRef.current);
    try {
      await ipc.downloadModel(inputRef.current.whisperModel);
      whisperRef.current = { status: "done", progress: 100, detail: "Ready" };
      setWhisper(whisperRef.current);
    } catch (e) {
      whisperRef.current = {
        status: "failed",
        progress: 0,
        error: String(e),
      };
      setWhisper(whisperRef.current);
    }
  }, []);

  const runOllama = useCallback(async () => {
    if (ollamaRef.current.status === "running") return;
    ollamaRef.current = {
      status: "running",
      progress: 0,
      detail: "Downloading Ollama installer",
    };
    setOllama(ollamaRef.current);
    try {
      const alreadyInstalled = await ipc.isOllamaInstalled();
      if (alreadyInstalled) {
        ollamaRef.current = { status: "done", progress: 100, detail: "Installed" };
        setOllama(ollamaRef.current);
        return;
      }
      await ipc.installOllamaRuntime();
      setOllama((s) =>
        s.status === "running"
          ? { ...s, progress: 40, detail: "Checking Ollama" }
          : s,
      );

      const start = Date.now();
      while (Date.now() - start < OLLAMA_POLL_TIMEOUT_MS) {
        await new Promise((r) => setTimeout(r, OLLAMA_POLL_INTERVAL_MS));
        try {
          const ok = await ipc.isOllamaInstalled();
          if (ok) {
            ollamaRef.current = {
              status: "done",
              progress: 100,
              detail: "Installed",
            };
            setOllama(ollamaRef.current);
            return;
          }
        } catch {
          /* keep polling */
        }
      }
      ollamaRef.current = {
        status: "failed",
        progress: 0,
        error:
          "Ollama installation didn't complete within 5 minutes. Install manually then click Retry.",
      };
      setOllama(ollamaRef.current);
    } catch (e) {
      ollamaRef.current = { status: "failed", progress: 0, error: String(e) };
      setOllama(ollamaRef.current);
    }
  }, []);

  const runDrafter = useCallback(async () => {
    if (drafterRef.current.status === "running") return;
    if (ollamaRef.current.status !== "done") {
      drafterRef.current = {
        status: "failed",
        progress: 0,
        error: "Install Ollama first.",
      };
      setDrafter(drafterRef.current);
      return;
    }
    drafterRef.current = { status: "running", progress: 0, detail: "Pulling model" };
    setDrafter(drafterRef.current);
    try {
      await ipc.downloadEmailDraftModel(
        inputRef.current.emailDraftEngine,
        inputRef.current.emailDraftModel,
      );
      drafterRef.current = { status: "done", progress: 100, detail: "Ready" };
      setDrafter(drafterRef.current);
    } catch (e) {
      drafterRef.current = { status: "failed", progress: 0, error: String(e) };
      setDrafter(drafterRef.current);
    }
  }, []);

  const runAll = useCallback(async () => {
    if (ollamaRef.current.status !== "done") await runOllama();
    if (ollamaRef.current.status !== "done") return;

    if (drafterRef.current.status !== "done") await runDrafter();
    if (drafterRef.current.status !== "done") return;

    if (whisperRef.current.status !== "done") await runWhisper();
  }, [runWhisper, runOllama, runDrafter]);

  const retry = useCallback(
    async (step: "whisper" | "ollama" | "drafter") => {
      if (step === "whisper") return runWhisper();
      if (step === "ollama") {
        await runOllama();
        if (ollamaRef.current.status === "done" && drafterRef.current.status !== "done") {
          await runDrafter();
        }
        return;
      }
      return runDrafter();
    },
    [runWhisper, runOllama, runDrafter],
  );

  // ---------- Aggregated flags ----------

  const aggregated = useMemo(() => {
    const allDone =
      whisper.status === "done" &&
      ollama.status === "done" &&
      drafter.status === "done";
    const anyFailed =
      whisper.status === "failed" ||
      ollama.status === "failed" ||
      drafter.status === "failed";
    const anyRunning =
      whisper.status === "running" ||
      ollama.status === "running" ||
      drafter.status === "running";
    return { allDone, anyFailed, anyRunning };
  }, [whisper, ollama, drafter]);

  return {
    whisper,
    ollama,
    drafter,
    ...aggregated,
    refresh,
    runAll,
    retry,
  };
}
