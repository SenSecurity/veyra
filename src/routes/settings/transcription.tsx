import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, Circle, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ipc } from "@/lib/tauri";
import { useSettings } from "@/hooks/use-settings";
import { SettingsPanel } from "./general";

export function SettingsTranscriptionRoute() {
  const { settings, loading, update } = useSettings();
  const [checking, setChecking] = useState(false);
  const [modelReady, setModelReady] = useState(false);
  const [checkingModel, setCheckingModel] = useState(false);
  const [emailModelReady, setEmailModelReady] = useState(false);
  const [checkingEmailModel, setCheckingEmailModel] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(0);

  const whisperModel =
    settings?.whisperModel === "large-v3-turbo" ||
    settings?.whisperModel === "ggml-large-v3-turbo.bin"
      ? "turbo"
      : settings?.whisperModel ?? "turbo";

  const saveWhisperModel = (value: string) => {
    void update({ whisperModel: value });
  };

  useEffect(() => {
    if (!settings) return;
    setCheckingModel(true);
    void ipc
      .checkModelDownloaded(whisperModel)
      .then(setModelReady)
      .catch(() => setModelReady(false))
      .finally(() => setCheckingModel(false));
  }, [settings, whisperModel]);

  useEffect(() => {
    if (!settings) return;
    if (!settings.groqApiKey.trim()) {
      setEmailModelReady(false);
      return;
    }
    setCheckingEmailModel(true);
    void ipc
      .checkEmailDraftModel(settings.groqApiKey, settings.emailDraftModel)
      .then(() => setEmailModelReady(true))
      .catch(() => setEmailModelReady(false))
      .finally(() => setCheckingEmailModel(false));
  }, [settings?.emailDraftModel, settings?.groqApiKey]);

  useEffect(() => {
    const un = listen<{ modelSize: string; downloaded: number; total: number; percent: number }>(
      "model:download:progress",
      (event) => {
        if (event.payload.modelSize !== whisperModel) return;
        setProgress(Math.max(0, Math.min(100, event.payload.percent)));
      },
    );
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [whisperModel]);

  if (loading || !settings) return <SettingsPanel title="Transcription" muted="Loading settings." />;

  async function downloadCurrentModel() {
    setDownloading(true);
    setProgress(0);
    try {
      await ipc.downloadModel(whisperModel);
      setModelReady(true);
      setProgress(100);
      toast.success("Model downloaded");
    } catch (error) {
      if (String(error).includes("cancelled")) {
        toast.info("Download cancelled");
      } else {
        toast.error(String(error));
      }
      setModelReady(await ipc.checkModelDownloaded(whisperModel).catch(() => false));
    } finally {
      setDownloading(false);
    }
  }

  async function cancelDownload() {
    await ipc.cancelModelDownload();
  }

  return (
    <SettingsPanel title="Transcription" muted="Engine, local model, and Groq credentials.">
      <label className="grid gap-2 text-sm">
        <span className="font-medium">Engine</span>
        <select
          className="h-9 rounded-md border border-border bg-background px-3"
          value={settings.engine}
          onChange={(e) => void update({ engine: e.target.value as "local" | "cloud" })}
        >
          <option value="local">Local whisper.cpp</option>
          <option value="cloud">Groq cloud</option>
        </select>
      </label>
      <label className="grid gap-2 text-sm">
        <div className="flex items-center justify-between gap-3">
          <span className="font-medium">Whisper model</span>
          <span
            className={
              modelReady
                ? "inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2 py-0.5 text-xs font-medium text-emerald-700 ring-1 ring-emerald-200"
                : "inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground ring-1 ring-border"
            }
          >
            {modelReady ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Circle className="h-3.5 w-3.5" />}
            {checkingModel ? "Checking" : modelReady ? "Operational" : "Not installed"}
          </span>
        </div>
        <select
          className="h-9 rounded-md border border-border bg-background px-3"
          value={whisperModel}
          onChange={(e) => saveWhisperModel(e.target.value)}
        >
          <option value="turbo">turbo - recommended</option>
          <option value="base">base - fastest/lightest</option>
          <option value="large-v3">large-v3 - highest accuracy</option>
        </select>
      </label>
      <div className="flex gap-2">
        <Button
          type="button"
          variant="outline"
          disabled={downloading}
          onClick={() => void downloadCurrentModel()}
        >
          {modelReady ? "Re-download model" : "Download model"}
        </Button>
        {downloading && (
          <Button type="button" variant="destructive" onClick={() => void cancelDownload()}>
            <XCircle className="h-4 w-4" />
            Cancel download
          </Button>
        )}
        <Button
          type="button"
          variant="outline"
          disabled={downloading}
          onClick={() =>
            void ipc.checkModelDownloaded(whisperModel).then((ok) =>
              {
                setModelReady(ok);
                toast[ok ? "success" : "warning"](ok ? "Model operational" : "Model missing");
              },
            )
          }
        >
          Check local model
        </Button>
      </div>
      {downloading && (
        <div className="space-y-2 rounded-lg border border-border bg-card p-3">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>Downloading {whisperModel}</span>
            <span>{Math.round(progress)}%</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-muted">
            <div className="h-full rounded-full bg-primary transition-[width]" style={{ width: `${progress}%` }} />
          </div>
        </div>
      )}
      <label className="grid gap-2 text-sm">
        <div className="flex items-center justify-between gap-3">
          <span className="font-medium">Email draft model</span>
          <span
            className={
              emailModelReady
                ? "inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2 py-0.5 text-xs font-medium text-emerald-700 ring-1 ring-emerald-200"
                : "inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground ring-1 ring-border"
            }
          >
            {emailModelReady ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Circle className="h-3.5 w-3.5" />}
            {checkingEmailModel ? "Checking" : emailModelReady ? "Operational" : "Not checked"}
          </span>
        </div>
        <select
          className="h-9 rounded-md border border-border bg-background px-3"
          value={settings.emailDraftModel}
          onChange={(e) => void update({ emailDraftModel: e.target.value })}
        >
          <option value="llama-3.3-70b-versatile">Llama 3.3 70B - recommended</option>
          <option value="llama-3.1-8b-instant">Llama 3.1 8B - fastest</option>
          <option value="openai/gpt-oss-120b">GPT-OSS 120B - stronger</option>
          <option value="openai/gpt-oss-20b">GPT-OSS 20B - fast</option>
        </select>
      </label>
      <Button
        type="button"
        variant="outline"
        disabled={checkingEmailModel}
        onClick={() => {
          setCheckingEmailModel(true);
          void ipc
            .checkEmailDraftModel(settings.groqApiKey, settings.emailDraftModel)
            .then(() => {
              setEmailModelReady(true);
              toast.success("Email model operational");
            })
            .catch((e) => {
              setEmailModelReady(false);
              toast.error(String(e));
            })
            .finally(() => setCheckingEmailModel(false));
        }}
      >
        Check email model
      </Button>
      <label className="grid gap-2 text-sm">
        <span className="font-medium">Groq API key</span>
        <Input
          type="password"
          value={settings.groqApiKey}
          onChange={(e) => void update({ groqApiKey: e.target.value })}
          placeholder="gsk_..."
        />
      </label>
      <Button
        type="button"
        disabled={checking}
        onClick={() => {
          setChecking(true);
          void ipc.testGroqKey(settings.groqApiKey)
            .then(() => toast.success("Groq key works"))
            .catch((e) => toast.error(String(e)))
            .finally(() => setChecking(false));
        }}
      >
        Test Groq key
      </Button>
    </SettingsPanel>
  );
}
