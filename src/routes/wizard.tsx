import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useNavigate } from "@tanstack/react-router";
import { CheckCircle2, Keyboard, Mail, Mic } from "lucide-react";
import { BrandMark } from "@/components/brand-mark";
import { HotkeyInput } from "@/components/hotkey-input";
import { Panel } from "@/components/page-shell";
import { Button } from "@/components/ui/button";
import { ipc } from "@/lib/tauri";
import { useSettings } from "@/hooks/use-settings";
import { cn } from "@/lib/utils";
import type { MicDevice } from "@/types/ipc";

const steps = ["Welcome", "Models", "Microphone", "Hotkeys", "Ready"] as const;

export function WizardRoute() {
  const navigate = useNavigate();
  const { settings, update, loading, error, reload } = useSettings();
  const [step, setStep] = useState(0);
  const [mics, setMics] = useState<MicDevice[]>([]);
  const [modelReady, setModelReady] = useState<boolean | null>(null);
  const [emailReady, setEmailReady] = useState<boolean | null>(null);

  useEffect(() => {
    void ipc.listMicrophones().then(setMics).catch(() => setMics([]));
  }, []);

  useEffect(() => {
    if (!settings) return;
    void ipc
      .checkModelDownloaded(settings.whisperModel)
      .then(setModelReady)
      .catch(() => setModelReady(false));
    void ipc
      .checkEmailDraftModel(settings.groqApiKey, settings.emailDraftEngine, settings.emailDraftModel)
      .then(() => setEmailReady(true))
      .catch(() => setEmailReady(false));
  }, [settings]);

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

  return (
    <OnboardingShell step={step}>
      {step === 0 ? (
        <>
          <SetupHero
            eyebrow={`Step 01 · ${steps[0]}`}
            title="Set up Veyra"
            accent="— quiet desk, ready when you are."
            description="Choose the essentials before the app opens. Menus stay locked until this setup is complete."
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
            title="Models"
            accent="— Whisper + Llama"
            description="Use fast local defaults now. You can change or re-download them later."
          />
          <div className="grid gap-3 md:grid-cols-2">
            <label className="grid gap-2 text-sm">
              <span className="font-semibold tracking-[-0.005em]">Speech model</span>
              <select
                className="veyra-select w-full"
                value={settings.whisperModel}
                onChange={(event) => void update({ whisperModel: event.target.value })}
              >
                <option value="turbo">turbo · recommended</option>
                <option value="base">base · fastest / lightest</option>
                <option value="large-v3">large-v3 · highest accuracy</option>
              </select>
            </label>
            <label className="grid gap-2 text-sm">
              <span className="font-semibold tracking-[-0.005em]">Email model</span>
              <select
                className="veyra-select w-full"
                value={settings.emailDraftModel}
                onChange={(event) =>
                  void update({ emailDraftEngine: "ollama", emailDraftModel: event.target.value })
                }
              >
                <option value="llama3.2:1b">Llama 3.2 1B · recommended</option>
                <option value="llama3.2">Llama 3.2 3B · stronger</option>
                <option value="qwen3:1.7b">Qwen3 1.7B · fast</option>
                <option value="qwen3:4b">Qwen3 4B · stronger</option>
              </select>
            </label>
          </div>
          <div className="grid gap-3 md:grid-cols-2">
            <SetupChoice
              icon={<CheckCircle2 className="h-4 w-4" />}
              tone="stt"
              title="Speech model"
              detail={settings.whisperModel}
              status={modelReady ? "Operational" : "Download later"}
            />
            <SetupChoice
              icon={<CheckCircle2 className="h-4 w-4" />}
              tone="drafter"
              title="Email model"
              detail={settings.emailDraftModel}
              status={emailReady ? "Operational" : "Install / check later"}
            />
          </div>
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
            description="Veyra is configured. The main app opens after this step."
          />
          <div className="grid gap-3 md:grid-cols-2">
            <SetupChoice
              icon={<Mic className="h-4 w-4" />}
              tone="stt"
              title="Speech to Text"
              detail={`${settings.whisperModel} via ${settings.engine}`}
              status="Ready"
            />
            <SetupChoice
              icon={<Mail className="h-4 w-4" />}
              tone="drafter"
              title="Email Drafter"
              detail={`${settings.emailDraftModel} via ${settings.emailDraftEngine}`}
              status="Ready"
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
          <Button type="button" onClick={() => void finish()}>
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
