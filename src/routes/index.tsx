import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { ArrowRight, CheckCircle2, Copy, Keyboard, Mail, Mic } from "lucide-react";
import { toast } from "sonner";
import { EmptyState } from "@/components/empty-state";
import { PageShell, Panel } from "@/components/page-shell";
import { Button } from "@/components/ui/button";
import { useSettings } from "@/hooks/use-settings";
import { isUsableTranscription } from "@/lib/email-output-quality";
import { ipc } from "@/lib/tauri";
import type { StreakInfo, Totals, Transcription } from "@/types/ipc";

export function HomeRoute() {
  const [totals, setTotals] = useState<Totals | null>(null);
  const [streak, setStreak] = useState<StreakInfo | null>(null);
  const [recent, setRecent] = useState<Transcription[]>([]);
  const { settings } = useSettings();
  const dictationHotkey = settings?.hotkey || "F24";
  const emailHotkey = settings?.commandHotkey || "Pause";
  const operational = (
    <span className="veyra-status-ready inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[0.68rem] font-semibold">
      <CheckCircle2 className="h-3 w-3" />
      Operational
    </span>
  );

  async function copyRecent(row: Transcription) {
    const text = row.finalText || row.rawText;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied");
    } catch (error) {
      toast.error(`Copy failed: ${String(error)}`);
    }
  }

  useEffect(() => {
    void Promise.all([
      ipc.getStatsTotals().catch(() => null),
      ipc.getStatsStreak().catch(() => null),
      ipc.listTranscriptions(12, 0).catch(() => []),
    ]).then(([t, s, r]) => {
      setTotals(t);
      setStreak(s);
      setRecent(r.filter(isUsableTranscription).slice(0, 5));
    });
  }, []);

  return (
    <PageShell title="Home" description="Quick controls, readiness, and recent work." className="max-w-[1080px]">
      <div className="grid shrink-0 gap-4 md:grid-cols-2">
        <Panel className="veyra-command-panel" title="Speech to Text" description="Dictate and transcribe anywhere." action={operational}>
          <button
            type="button"
            className="veyra-glass flex w-full items-center justify-between gap-3 rounded-2xl p-3 text-left transition-colors hover:bg-white focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/35"
            onClick={() => void ipc.toggleRecording()}
          >
            <span className="flex items-center gap-3">
              <span className="flex h-10 w-10 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-[0_10px_24px_rgb(20_121_223_/_0.18)]">
                <Mic className="h-4 w-4" />
              </span>
              <span>
                <span className="block text-sm font-semibold tracking-[-0.015em]">Hold {dictationHotkey} to dictate</span>
                <span className="block text-xs text-muted-foreground">Dictate and transcribe anywhere</span>
              </span>
            </span>
            <Keyboard className="h-4 w-4 text-muted-foreground" />
          </button>
          <div className="veyra-wave mt-4">
            {Array.from({ length: 38 }).map((_, index) => (
              <i
                key={index}
                style={{ height: `${18 + ((index * 13) % 30)}px` }}
              />
            ))}
          </div>
        </Panel>
        <Panel className="veyra-command-panel" title="Email Drafter" description="Draft emails from your voice." action={operational}>
          <Link
            to="/email-drafts"
            className="veyra-glass flex w-full items-center justify-between gap-3 rounded-2xl p-3 text-left transition-colors hover:bg-white focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/35"
          >
            <span className="flex items-center gap-3">
              <span className="flex h-10 w-10 items-center justify-center rounded-xl bg-sky-100 text-primary shadow-[0_10px_24px_rgb(20_121_223_/_0.10)]">
                <Mail className="h-4 w-4" />
              </span>
              <span>
                <span className="block text-sm font-semibold tracking-[-0.015em]">Hold {emailHotkey} to draft</span>
                <span className="block text-xs text-muted-foreground">Draft emails from your voice</span>
              </span>
            </span>
            <ArrowRight className="h-4 w-4 text-muted-foreground" />
          </Link>
          <div className="veyra-wave veyra-wave-orange mt-4">
            {Array.from({ length: 38 }).map((_, index) => (
              <i
                key={index}
                style={{ height: `${16 + ((index * 17) % 32)}px` }}
              />
            ))}
          </div>
        </Panel>
      </div>
      <div className="grid shrink-0 gap-3 text-sm md:grid-cols-4">
        <div className="veyra-glass rounded-2xl p-2.5"><span className="text-muted-foreground">Total words</span><strong className="mt-0.5 block text-lg">{totals?.wordCount ?? 0}</strong></div>
        <div className="veyra-glass rounded-2xl p-2.5"><span className="text-muted-foreground">Sessions</span><strong className="mt-0.5 block text-lg">{totals?.sessionCount ?? 0}</strong></div>
        <div className="veyra-glass rounded-2xl p-2.5"><span className="text-muted-foreground">Current streak</span><strong className="mt-0.5 block text-lg">{streak?.current ?? 0}</strong></div>
        <div className="veyra-glass rounded-2xl p-2.5"><span className="text-muted-foreground">Longest streak</span><strong className="mt-0.5 block text-lg">{streak?.longest ?? 0}</strong></div>
      </div>
      <Panel
        className="min-h-0 flex-1"
        title="Recent activity"
        action={
          <Button variant="ghost" size="sm" asChild>
            <Link to="/history">
              View all
              <ArrowRight className="h-3.5 w-3.5" />
            </Link>
          </Button>
        }
      >
        {recent.length === 0 ? (
          <EmptyState title="No transcriptions yet">Record once and the last captures appear here.</EmptyState>
        ) : (
          <div className="h-full overflow-auto rounded-2xl border border-border/80 bg-white/62">
            {recent.map((row) => (
              <div key={row.id} className="flex items-center gap-3 border-b border-border/70 px-3.5 py-3 last:border-b-0">
                <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-sky-50 text-primary">
                  {row.mode === "command" ? <Mail className="h-3.5 w-3.5" /> : <Mic className="h-3.5 w-3.5" />}
                </span>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm text-foreground">{row.finalText || row.rawText}</p>
                  <p className="mt-0.5 text-xs text-muted-foreground">
                    {row.wordCount} words - {row.engine} - {row.mode}
                  </p>
                </div>
                <button
                  type="button"
                  className="veyra-icon-button"
                  onClick={() => void copyRecent(row)}
                  aria-label="Copy recent transcription"
                  title="Copy"
                >
                  <Copy className="h-4 w-4" />
                </button>
              </div>
            ))}
          </div>
        )}
      </Panel>
    </PageShell>
  );
}
