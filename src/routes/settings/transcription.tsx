import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, Circle, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ipc } from "@/lib/tauri";
import { useSettings } from "@/hooks/use-settings";
import { SettingsPanel } from "./general";

type EmailDraftEngine = "ollama" | "groq";

const OLLAMA_RECOMMENDED_MODEL = "llama3.2";
const GROQ_RECOMMENDED_MODEL = "llama-3.3-70b-versatile";

export function SettingsTranscriptionRoute() {
  const { settings, loading, update } = useSettings();
  const [checking, setChecking] = useState(false);
  const [modelReady, setModelReady] = useState(false);
  const [checkingModel, setCheckingModel] = useState(false);
  const [emailModelReady, setEmailModelReady] = useState(false);
  const [checkingEmailModel, setCheckingEmailModel] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [downloadingEmailModel, setDownloadingEmailModel] = useState(false);
  const [progress, setProgress] = useState(0);
  const [emailProgress, setEmailProgress] = useState(0);
  const [emailDownloadStatus, setEmailDownloadStatus] = useState("");

  const whisperModel =
    settings?.whisperModel === "large-v3-turbo" ||
    settings?.whisperModel === "ggml-large-v3-turbo.bin"
      ? "turbo"
      : settings?.whisperModel ?? "turbo";
  const emailDraftEngine = settings?.emailDraftEngine ?? "ollama";
  const emailDraftModel =
    settings?.emailDraftModel ??
    (emailDraftEngine === "ollama" ? OLLAMA_RECOMMENDED_MODEL : GROQ_RECOMMENDED_MODEL);
  const recommendedEmailModel =
    emailDraftEngine === "ollama" ? OLLAMA_RECOMMENDED_MODEL : GROQ_RECOMMENDED_MODEL;
  const hasGroqKey = Boolean(settings?.groqApiKey.trim());
  const needsGroqKey = settings?.engine === "cloud" || emailDraftEngine === "groq";

  const saveWhisperModel = (value: string) => {
    void update({ whisperModel: value });
  };

  const saveEmailDraftEngine = (engine: EmailDraftEngine) => {
    void update({
      emailDraftEngine: engine,
      emailDraftModel: engine === "ollama" ? OLLAMA_RECOMMENDED_MODEL : GROQ_RECOMMENDED_MODEL,
    });
    setEmailModelReady(false);
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
    if (emailDraftEngine === "groq" && !hasGroqKey) {
      setEmailModelReady(false);
      return;
    }
    setCheckingEmailModel(true);
    void ipc
      .checkEmailDraftModel(settings.groqApiKey, emailDraftEngine, emailDraftModel)
      .then(() => setEmailModelReady(true))
      .catch(() => setEmailModelReady(false))
      .finally(() => setCheckingEmailModel(false));
  }, [emailDraftEngine, emailDraftModel, hasGroqKey, settings?.groqApiKey]);

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

  useEffect(() => {
    const un = listen<{ model: string; downloaded: number; total: number; percent: number; status: string }>(
      "email-model:download:progress",
      (event) => {
        if (event.payload.model !== emailDraftModel) return;
        setEmailProgress(Math.max(0, Math.min(100, event.payload.percent)));
        setEmailDownloadStatus(event.payload.status || "Downloading");
      },
    );
    return () => void un.then((fn) => fn()).catch(() => {});
  }, [emailDraftModel]);

  if (loading || !settings) {
    return <SettingsPanel title="Transcription" muted="Loading settings." />;
  }

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

  async function downloadCurrentEmailModel() {
    setDownloadingEmailModel(true);
    setCheckingEmailModel(false);
    setEmailProgress(0);
    setEmailDownloadStatus("Starting download");
    try {
      await ipc.downloadEmailDraftModel(emailDraftEngine, emailDraftModel);
      setEmailModelReady(true);
      setEmailProgress(100);
      setEmailDownloadStatus("Downloaded");
      toast.success("Email model downloaded");
    } catch (error) {
      setEmailModelReady(false);
      toast.error(String(error));
    } finally {
      setDownloadingEmailModel(false);
    }
  }

  async function checkCurrentEmailModel() {
    if (!settings) return;
    if (emailDraftEngine === "groq" && !hasGroqKey) {
      toast.error("Enter a Groq API key first");
      return;
    }
    setCheckingEmailModel(true);
    try {
      await ipc.checkEmailDraftModel(settings.groqApiKey, emailDraftEngine, emailDraftModel);
      setEmailModelReady(true);
      toast.success("Email model operational");
    } catch (error) {
      setEmailModelReady(false);
      toast.error(String(error));
    } finally {
      setCheckingEmailModel(false);
    }
  }

  return (
    <SettingsPanel title="Transcription" muted="Engine, local models, email drafts, and credentials.">
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
          <div className="flex items-center gap-2">
            <span className="font-medium">Whisper model</span>
            {whisperModel === "turbo" ? (
              <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary ring-1 ring-primary/20">
                Recommended
              </span>
            ) : null}
          </div>
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
          <option value="turbo">turbo - Recommended</option>
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
            void ipc.checkModelDownloaded(whisperModel).then((ok) => {
              setModelReady(ok);
              toast[ok ? "success" : "warning"](ok ? "Model operational" : "Model missing");
            })
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
        <span className="font-medium">Email draft engine</span>
        <select
          className="h-9 rounded-md border border-border bg-background px-3"
          value={emailDraftEngine}
          onChange={(e) => saveEmailDraftEngine(e.target.value as EmailDraftEngine)}
        >
          <option value="ollama">Local Ollama</option>
          <option value="groq">Groq cloud</option>
        </select>
      </label>
      <label className="grid gap-2 text-sm">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2">
            <span className="font-medium">Email draft model</span>
            {emailDraftModel === recommendedEmailModel ? (
              <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary ring-1 ring-primary/20">
                Recommended
              </span>
            ) : null}
          </div>
          <span
            className={
              emailModelReady
                ? "inline-flex items-center gap-1 rounded-full bg-emerald-50 px-2 py-0.5 text-xs font-medium text-emerald-700 ring-1 ring-emerald-200"
                : "inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground ring-1 ring-border"
            }
          >
            {emailModelReady ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Circle className="h-3.5 w-3.5" />}
            {emailDraftEngine === "groq" && !hasGroqKey
              ? "Needs API key"
              : downloadingEmailModel
                ? "Downloading"
              : checkingEmailModel
                ? "Checking"
                : emailModelReady
                  ? "Operational"
                  : emailDraftEngine === "ollama"
                    ? "Not installed"
                    : "Not checked"}
          </span>
        </div>
        <select
          className="h-9 rounded-md border border-border bg-background px-3"
          value={emailDraftModel}
          onChange={(e) => {
            setEmailModelReady(false);
            void update({ emailDraftModel: e.target.value });
          }}
        >
          {emailDraftEngine === "ollama" ? (
            <>
              <option value="llama3.2">Llama 3.2 3B - Recommended</option>
              <option value="llama3.2:1b">Llama 3.2 1B - lightest</option>
              <option value="qwen3:1.7b">Qwen3 1.7B - fast</option>
              <option value="qwen3:4b">Qwen3 4B - stronger</option>
            </>
          ) : (
            <>
              <option value="llama-3.3-70b-versatile">Llama 3.3 70B - Recommended</option>
              <option value="llama-3.1-8b-instant">Llama 3.1 8B - fastest</option>
              <option value="openai/gpt-oss-120b">GPT-OSS 120B - stronger</option>
              <option value="openai/gpt-oss-20b">GPT-OSS 20B - fast</option>
            </>
          )}
        </select>
      </label>
      <div className="flex gap-2">
        {emailDraftEngine === "ollama" ? (
          <Button
            type="button"
            variant="outline"
            disabled={downloadingEmailModel}
            onClick={() => void downloadCurrentEmailModel()}
          >
            {downloadingEmailModel
              ? "Downloading email model..."
              : emailModelReady
                ? "Re-download email model"
                : "Download email model"}
          </Button>
        ) : (
          <Button
            type="button"
            variant="outline"
            disabled
            title="Groq email draft models run in the cloud and do not need a local download."
          >
            Cloud model - no download
          </Button>
        )}
        <Button
          type="button"
          variant="outline"
          disabled={checkingEmailModel || downloadingEmailModel || (emailDraftEngine === "groq" && !hasGroqKey)}
          title={
            emailDraftEngine === "groq" && !hasGroqKey
              ? "Enter a Groq API key first"
              : emailDraftEngine === "ollama"
                ? "Check this Ollama model"
                : "Check this Groq model"
          }
          onClick={() => void checkCurrentEmailModel()}
        >
          Check email model
        </Button>
      </div>
      {downloadingEmailModel && (
        <div className="space-y-2 rounded-lg border border-border bg-card p-3">
          <div className="flex items-center justify-between gap-3 text-xs text-muted-foreground">
            <span className="truncate">{emailDownloadStatus || `Downloading ${emailDraftModel}`}</span>
            <span>{emailProgress > 0 ? `${Math.round(emailProgress)}%` : "Preparing"}</span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-muted">
            <div
              className={
                emailProgress > 0
                  ? "h-full rounded-full bg-primary transition-[width]"
                  : "h-full w-1/3 animate-pulse rounded-full bg-primary/70"
              }
              style={emailProgress > 0 ? { width: `${emailProgress}%` } : undefined}
            />
          </div>
        </div>
      )}

      {needsGroqKey ? (
        <>
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
        </>
      ) : null}
    </SettingsPanel>
  );
}
