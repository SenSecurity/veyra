import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { ipc } from "@/lib/tauri";
import { useSettings } from "@/hooks/use-settings";

const steps = ["Welcome", "Microphone", "Engine", "Hotkey", "Language", "Done"];

export function WizardRoute() {
  const navigate = useNavigate();
  const { settings } = useSettings();
  const [step, setStep] = useState(0);
  const [completed, setCompleted] = useState<boolean | null>(null);

  useEffect(() => {
    void ipc.wizardStatus().then((s) => setCompleted(s.completed)).catch(() => setCompleted(false));
  }, []);

  async function finish() {
    await ipc.markWizardComplete();
    await navigate({ to: "/" });
  }

  return (
    <section className="flex min-h-full items-center justify-center p-8">
      <div className="w-full max-w-xl rounded-lg border border-border bg-card p-7 shadow-2">
        <div className="mb-6 flex gap-1.5">
          {steps.map((s, i) => (
            <div key={s} className={i <= step ? "h-1.5 flex-1 rounded bg-primary" : "h-1.5 flex-1 rounded bg-muted"} />
          ))}
        </div>
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{completed ? "Configured" : "First run"}</p>
        <h1 className="mt-2 text-2xl font-semibold tracking-tight">{steps[step]}</h1>
        <p className="mt-3 text-sm leading-6 text-muted-foreground">
          {step === 0 && "Set the basics before first dictation."}
          {step === 1 && `Current microphone: ${settings?.microphone || "system default"}.`}
          {step === 2 && `Engine: ${settings?.engine ?? "local"}. Configure Groq in Settings when needed.`}
          {step === 3 && `Hotkey: ${settings?.hotkey ?? "not loaded"}.`}
          {step === 4 && "Language detection is automatic in this Phase 3 build."}
          {step === 5 && "Setup complete. You can rerun this wizard from About."}
        </p>
        <div className="mt-8 flex justify-between">
          <Button type="button" variant="outline" disabled={step === 0} onClick={() => setStep((s) => Math.max(0, s - 1))}>Back</Button>
          {step < steps.length - 1 ? (
            <Button type="button" onClick={() => setStep((s) => s + 1)}>Next</Button>
          ) : (
            <Button type="button" onClick={() => void finish()}>Finish</Button>
          )}
        </div>
      </div>
    </section>
  );
}
