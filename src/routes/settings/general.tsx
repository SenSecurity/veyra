import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/tauri";
import { useSettings } from "@/hooks/use-settings";
import type { MicDevice } from "@/types/ipc";

export function SettingsGeneralRoute() {
  const { settings, loading, error, update } = useSettings();
  const [mics, setMics] = useState<MicDevice[]>([]);

  useEffect(() => {
    void ipc.listMicrophones().then(setMics).catch(() => setMics([]));
  }, []);

  if (loading || !settings) return <SettingsPanel title="General" muted="Loading settings." />;

  return (
    <SettingsPanel title="General" muted={error ?? "Microphone and recording mode."}>
      <label className="grid gap-2 text-sm">
        <span className="font-medium">Microphone</span>
        <select
          className="h-9 rounded-md border border-border bg-background px-3"
          value={settings.microphone}
          onChange={(e) => void update({ microphone: e.target.value })}
        >
          <option value="default">System default</option>
          {mics.map((mic) => (
            <option key={mic.name} value={mic.name}>
              {mic.name}{mic.is_default ? " (default)" : ""}
            </option>
          ))}
        </select>
      </label>
      <div className="flex items-center justify-between rounded-lg border border-border bg-card p-3">
        <div>
          <p className="text-sm font-medium">Push to talk</p>
          <p className="text-xs text-muted-foreground">Hold hotkey while recording.</p>
        </div>
        <Switch
          checked={settings.recordingMode === "push-to-talk"}
          onCheckedChange={(checked) =>
            void update({ recordingMode: checked ? "push-to-talk" : "toggle" })
          }
        />
      </div>
      <Button
        type="button"
        variant="outline"
        onClick={() =>
          void ipc.toggleRecording().then((r) => toast.success(r)).catch((e) => toast.error(String(e)))
        }
      >
        Toggle recording
      </Button>
    </SettingsPanel>
  );
}

export function SettingsPanel({
  title,
  muted,
  children,
}: {
  title: string;
  muted?: string;
  children?: React.ReactNode;
}) {
  return (
    <section className="mx-auto flex w-full max-w-3xl flex-col gap-5 p-8">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        {muted && <p className="mt-1 text-sm text-muted-foreground">{muted}</p>}
      </div>
      {children}
    </section>
  );
}
