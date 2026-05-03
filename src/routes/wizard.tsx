import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  AlertTriangle,
  CheckCircle2,
  Download,
  Keyboard,
  Loader2,
  Mail,
  Mic,
  RefreshCw,
} from "lucide-react";
import { BrandMark } from "@/components/brand-mark";
import { HotkeyInput } from "@/components/hotkey-input";
import { Panel } from "@/components/page-shell";
import { Button } from "@/components/ui/button";
import { useInstallOrchestrator, type StepState } from "@/hooks/use-install-orchestrator";
import { useSettings } from "@/hooks/use-settings";
import { ipc } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import type { MicDevice } from "@/types/ipc";

const steps = ["Welcome", "Install", "Microphone", "Hotkeys", "Ready"] as const;

export function WizardRoute() {
  const navigate = useNavigate();
  const { settings, update, loading, error, reload } = useSettings();
  const [step, setStep] = useState(0);
  const [mics, setMics] = useState<MicDevice[]>([]);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  useEffect(() => {
    void ipc.listMicrophones().then(setMics).catch(() => setMics([]));
  }, []);

  const orchestrator = useInstallOrchestrator({
    whisperModel: settings?.whisperModel ?? "turbo",
    emailDraftEngine: settings?.emailDraftEngine ?? "ollama",
    emailDraftModel: settings?.emailDraftModel ?? "llama3.2:1b",
    groqApiKey: settings?.groqApiKey ?? "",
  });

  const selectedMic = useMemo(() => {
    if (!settings) return "System default";
    if (settings.microphone === "default") return "System default";
    return settings.microphone;
  }, [settings]);

  async function finish() {
    await ipc.markWizardComplete();
    window.dispatchEvent(new Event("veyra:wizard-complete"));
    await navigate({ to: "/" });
  }

  if (loading || !settings) {
    return (
      <OnboardingShell step={step}>
        <SetupHero
          eyebrow="First boot"
          title="Preparing Veyra"
          accent="— loading"
          description={error ?? "Loading local settings."}
        />
        {error ? (
          <Button type="button" onClick={() => void reload()}>
            Retry
          </Button>
        ) : null}
      </OnboardingShell>
    );
  }

  const installLocked = orchestrator.anyRunning;

  return (
    <OnboardingShell step={step}>
      {step === 0 ? (
        <>
          <SetupHero
            eyebrow={`Step 01 · ${steps[0]}`}
            title="Set up Veyra"
            accent="— quiet desk, ready when you are."
            description="Three quick steps. The next screen installs everything Veyra needs in one click; the rest is just preferences."
          />
          <div className="grid gap-3 md:grid-cols-2">
            <SetupChoice
              icon={<Mic className="h-4 w-4" />}
              tone="stt"
              title="Speech to Text"
              detail="Local whisper.cpp, ready for dictation."
            />
            <SetupChoice
              icon={<Mail className="h-4 w-4" />}
              tone="drafter"
              title="Email Drafter"
              detail="Voice intent becomes a polished draft."
            />
          </div>
        </>
      ) : null}

      {step === 1 ? (
        <>
          <SetupHero
            eyebrow={`Step 02 · ${steps[1]}`}
            title="Install everything"
            accent="— Whisper, Ollama, Llama"
            description="One click downloads the speech model, installs Ollama if missing, and pulls the local email-draft model. You can keep going through the wizard while these finish in the background."
          />

          <div className="grid gap-3 md:grid-cols-3">
            <InstallCard
              icon={<Mic className="h-4 w-4" />}
              tone="stt"
              title="Whisper"
              subtitle={`${settings.whisperModel} · 1.5 GB`}
              state={orchestrator.whisper}
              onRetry={() => void orchestrator.retry("whisper")}
            />
            <InstallCard
              icon={<Download className="h-4 w-4" />}
              tone="stt"
              title="Ollama runtime"
              subtitle="Local LLM host"
              state={orchestrator.ollama}
              onRetry={() => void orchestrator.retry("ollama")}
            />
            <InstallCard
              icon={<Mail className="h-4 w-4" />}
              tone="drafter"
              title="Email model"
              subtitle={settings.emailDraftModel}
              state={orchestrator.drafter}
              onRetry={() => void orchestrator.retry("drafter")}
            />
          </div>

          <div className="flex items-center justify-between gap-3">
            <Button
              type="button"
              size="lg"
              onClick={() => void orchestrator.runAll()}
              disabled={orchestrator.allDone || orchestrator.anyRunning}
            >
              {orchestrator.allDone
                ? "All set"
                : orchestrator.anyRunning
                  ? "Installing…"
                  : orchestrator.anyFailed
                    ? "Resume install"
                    : "Install everything"}
            </Button>

            <button
              type="button"
              className="text-xs font-medium text-muted-foreground transition-colors hover:text-foreground"
              onClick={() => setAdvancedOpen((o) => !o)}
            >
              {advancedOpen ? "Hide advanced" : "Advanced…"}
            </button>
          </div>

          {advancedOpen ? (
            <div className="grid gap-3 rounded-xl border border-border bg-frost/50 p-3 md:grid-cols-2">
              <label className="grid gap-1.5 text-xs">
                <span className="font-mono uppercase tracking-[0.2em] text-muted-foreground">
                  Speech model
                </span>
                <select
                  className="veyra-select w-full disabled:cursor-not-allowed disabled:opacity-60"
                  value={settings.whisperModel}
                  disabled={installLocked}
                  onChange={(event) => void update({ whisperModel: event.target.value })}
                >
                  <option value="turbo">turbo · recommended</option>
                  <option value="base">base · fastest / lightest</option>
                  <option value="large-v3">large-v3 · highest accuracy</option>
                </select>
              </label>
              <label className="grid gap-1.5 text-xs">
                <span className="font-mono uppercase tracking-[0.2em] text-muted-foreground">
                  Email model
                </span>
                <select
                  className="veyra-select w-full disabled:cursor-not-allowed disabled:opacity-60"
                  value={settings.emailDraftModel}
                  disabled={installLocked}
                  onChange={(event) =>
                    void update({
                      emailDraftEngine: "ollama",
                      emailDraftModel: event.target.value,
                    })
                  }
                >
                  <option value="llama3.2:1b">Llama 3.2 1B · recommended</option>
                  <option value="llama3.2">Llama 3.2 3B · stronger</option>
                  <option value="qwen3:1.7b">Qwen3 1.7B · fast</option>
                  <option value="qwen3:4b">Qwen3 4B · stronger</option>
                </select>
              </label>
            </div>
          ) : null}
        </>
      ) : null}

      {step === 2 ? (
        <>
          <SetupHero
            eyebrow={`Step 03 · ${steps[2]}`}
            title="Microphone"
            accent="— pick your input"
            description="Choose the input Veyra should use for dictation and email drafting."
          />
          <label className="grid gap-2 text-sm">
            <span className="font-semibold tracking-[-0.005em]">Input device</span>
            <select
              className="veyra-select w-full"
              value={settings.microphone}
              onChange={(event) => void update({ microphone: event.target.value })}
            >
              <option value="default">System default</option>
              {mics.map((mic) => (
                <option key={mic.name} value={mic.name}>
                  {mic.name}
                  {mic.is_default ? " (default)" : ""}
                </option>
              ))}
            </select>
          </label>
          <SetupChoice
            icon={<Mic className="h-4 w-4" />}
            tone="stt"
            title="Selected microphone"
            detail={selectedMic}
            status="Ready"
          />
        </>
      ) : null}

      {step === 3 ? (
        <>
          <SetupHero
            eyebrow={`Step 04 · ${steps[3]}`}
            title="Hotkeys"
            accent="— the only contract"
            description="Global shortcuts can be changed now or later in Settings."
          />
          <div className="grid gap-3 md:grid-cols-2">
            <label className="grid gap-2 text-sm">
              <span className="font-semibold tracking-[-0.005em]">Dictation hotkey</span>
              <HotkeyInput value={settings.hotkey} onChange={(hotkey) => void update({ hotkey })} />
            </label>
            <label className="grid gap-2 text-sm">
              <span className="font-semibold tracking-[-0.005em]">Email draft hotkey</span>
              <HotkeyInput
                value={settings.commandHotkey}
                onChange={(commandHotkey) => void update({ commandHotkey })}
              />
            </label>
          </div>
          <SetupChoice
            icon={<Keyboard className="h-4 w-4" />}
            tone="stt"
            title="Defaults"
            detail="F24 for dictation, Pause for email draft."
          />
        </>
      ) : null}

      {step === 4 ? (
        <>
          <SetupHero
            eyebrow={`Step 05 · ${steps[4]}`}
            title="Ready"
            accent="— quiet desk, ready when you are."
            description={
              orchestrator.allDone
                ? "All components installed. Veyra opens after this step."
                : "Install is still finishing in the background. The button below unlocks once everything is ready."
            }
          />
          <div className="grid gap-3 md:grid-cols-3">
            <InstallCard
              icon={<Mic className="h-4 w-4" />}
              tone="stt"
              title="Whisper"
              subtitle={settings.whisperModel}
              state={orchestrator.whisper}
              onRetry={() => void orchestrator.retry("whisper")}
            />
            <InstallCard
              icon={<Download className="h-4 w-4" />}
              tone="stt"
              title="Ollama runtime"
              subtitle="Local LLM host"
              state={orchestrator.ollama}
              onRetry={() => void orchestrator.retry("ollama")}
            />
            <InstallCard
              icon={<Mail className="h-4 w-4" />}
              tone="drafter"
              title="Email model"
              subtitle={settings.emailDraftModel}
              state={orchestrator.drafter}
              onRetry={() => void orchestrator.retry("drafter")}
            />
          </div>
        </>
      ) : null}

      <div className="mt-7 flex items-center justify-between">
        <Button
          type="button"
          variant="outline"
          disabled={step === 0}
          onClick={() => setStep((s) => Math.max(0, s - 1))}
        >
          Back
        </Button>
        {step < steps.length - 1 ? (
          <Button type="button" onClick={() => setStep((s) => Math.min(steps.length - 1, s + 1))}>
            Next
          </Button>
        ) : (
          <Button
            type="button"
            disabled={!orchestrator.allDone}
            onClick={() => void finish()}
            title={
              orchestrator.allDone
                ? undefined
                : "Waiting for installs to finish before opening Veyra."
            }
          >
            Start using Veyra
          </Button>
        )}
      </div>
    </OnboardingShell>
  );
}

function OnboardingShell({ step, children }: { step: number; children: ReactNode }) {
  return (
    <section className="flex h-full min-h-0 items-center justify-center overflow-auto p-8">
      <Panel className="w-full max-w-3xl">
        <div className="mb-7 flex gap-1.5" aria-label={`Step ${step + 1} of ${steps.length}`}>
          {steps.map((name, index) => (
            <div
              key={name}
              className={cn(
                "h-1 flex-1 rounded-full transition-colors",
                index <= step
                  ? "bg-[linear-gradient(90deg,var(--cyan),var(--cyan-deep))] shadow-[0_0_8px_var(--halo)]"
                  : "bg-border/70",
              )}
              title={name}
            />
          ))}
        </div>
        <div className="flex flex-col gap-4">{children}</div>
      </Panel>
    </section>
  );
}

function SetupHero({
  eyebrow,
  title,
  accent,
  description,
}: {
  eyebrow: string;
  title: string;
  accent?: string;
  description: string;
}) {
  return (
    <div className="mb-2 flex items-start gap-4">
      <BrandMark className="h-12 w-12 rounded-2xl" />
      <div className="min-w-0">
        <div className="veyra-eyebrow mb-1.5">{eyebrow}</div>
        <h1 className="text-[1.75rem] font-medium leading-[1.05] tracking-[-0.03em] text-foreground">
          {title}
          {accent ? (
            <span className="veyra-italic ml-2 text-[var(--cyan-deep)] text-[1.4rem]">{accent}</span>
          ) : null}
        </h1>
        <p className="mt-1.5 max-w-xl text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

function SetupChoice({
  icon,
  title,
  detail,
  status,
  tone = "stt",
}: {
  icon: ReactNode;
  title: string;
  detail: string;
  status?: string;
  tone?: "stt" | "drafter";
}) {
  const accent =
    tone === "drafter"
      ? "text-[var(--spark-deep)] bg-[#fff5e6] border-[rgba(255,138,31,0.22)]"
      : "text-[var(--cyan-deep)] bg-[var(--ice-50)] border-[rgba(43,199,255,0.25)]";
  return (
    <div className="relative flex items-center justify-between gap-4 rounded-xl border border-border bg-white p-3.5 shadow-[0_1px_0_rgb(12_17_28_/_0.025)]">
      <span
        className={cn(
          "pointer-events-none absolute left-0 top-3 bottom-3 w-[2px] rounded-[2px]",
          tone === "drafter"
            ? "bg-[linear-gradient(180deg,var(--spark),var(--spark-deep))] shadow-[0_0_8px_var(--spark-glow)]"
            : "bg-[linear-gradient(180deg,var(--cyan),var(--cyan-deep))] shadow-[0_0_8px_var(--halo)]",
        )}
        aria-hidden="true"
      />
      <div className="flex min-w-0 items-center gap-3 pl-2">
        <span className={cn("flex h-9 w-9 items-center justify-center rounded-xl border", accent)}>
          {icon}
        </span>
        <div className="min-w-0">
          <p className="font-semibold tracking-[-0.005em] text-foreground">{title}</p>
          <p className="truncate text-sm text-muted-foreground">{detail}</p>
        </div>
      </div>
      {status ? (
        <span className="veyra-status-ready shrink-0 rounded-full border px-2 py-0.5 text-xs font-semibold">
          {status}
        </span>
      ) : null}
    </div>
  );
}

function InstallCard({
  icon,
  title,
  subtitle,
  tone,
  state,
  onRetry,
}: {
  icon: ReactNode;
  title: string;
  subtitle: string;
  tone: "stt" | "drafter";
  state: StepState;
  onRetry: () => void;
}) {
  const isFailed = state.status === "failed";
  const isRunning = state.status === "running";
  const isDone = state.status === "done";

  const iconAccent =
    tone === "drafter"
      ? "text-[var(--spark-deep)] bg-[#fff5e6] border-[rgba(255,138,31,0.22)]"
      : "text-[var(--cyan-deep)] bg-[var(--ice-50)] border-[rgba(43,199,255,0.25)]";

  return (
    <div className="relative flex flex-col gap-3 rounded-xl border border-border bg-white p-4 shadow-[0_1px_0_rgb(12_17_28_/_0.025)]">
      <span
        className={cn(
          "pointer-events-none absolute left-0 top-4 bottom-4 w-[2px] rounded-[2px]",
          tone === "drafter"
            ? "bg-[linear-gradient(180deg,var(--spark),var(--spark-deep))] shadow-[0_0_8px_var(--spark-glow)]"
            : "bg-[linear-gradient(180deg,var(--cyan),var(--cyan-deep))] shadow-[0_0_8px_var(--halo)]",
        )}
        aria-hidden="true"
      />
      <div className="flex items-center gap-3 pl-2">
        <span className={cn("flex h-9 w-9 items-center justify-center rounded-xl border", iconAccent)}>
          {icon}
        </span>
        <div className="min-w-0">
          <p className="font-semibold tracking-[-0.005em] text-foreground">{title}</p>
          <p className="truncate text-xs text-muted-foreground">{subtitle}</p>
        </div>
      </div>

      <StatusLine state={state} />

      {isRunning ? (
        <div className="h-1.5 overflow-hidden rounded-full bg-border/60">
          <div
            className={cn(
              "h-full rounded-full transition-[width]",
              tone === "drafter"
                ? "bg-[linear-gradient(90deg,var(--spark),var(--spark-deep))]"
                : "bg-[linear-gradient(90deg,var(--cyan),var(--cyan-deep))]",
            )}
            style={{ width: `${Math.max(2, Math.min(100, state.progress))}%` }}
          />
        </div>
      ) : null}

      {isFailed && state.error ? (
        <p className="text-[0.75rem] leading-snug text-amber-700">{state.error}</p>
      ) : null}

      {isFailed ? (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="self-start"
          onClick={onRetry}
        >
          <RefreshCw className="h-3.5 w-3.5" />
          Retry
        </Button>
      ) : null}

      {isDone ? null : null}
    </div>
  );
}

function StatusLine({ state }: { state: StepState }) {
  if (state.status === "done") {
    return (
      <div className="inline-flex items-center gap-1.5 text-xs font-semibold text-emerald-600">
        <CheckCircle2 className="h-3.5 w-3.5" />
        Ready
      </div>
    );
  }
  if (state.status === "running") {
    return (
      <div className="inline-flex items-center gap-1.5 text-xs font-medium text-[var(--cyan-deep)]">
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
        {state.detail ?? "Working"}
      </div>
    );
  }
  if (state.status === "failed") {
    return (
      <div className="inline-flex items-center gap-1.5 text-xs font-semibold text-amber-700">
        <AlertTriangle className="h-3.5 w-3.5" />
        Needs attention
      </div>
    );
  }
  return (
    <div className="inline-flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
      Idle
    </div>
  );
}
