import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { ArrowRight, Clock, Flame, Keyboard, Mail, Mic, Type } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { PageShell, Panel } from "@/components/page-shell";
import { StatCard } from "@/components/stat-card";
import { TranscriptionRow } from "@/components/transcription-row";
import { Button } from "@/components/ui/button";
import { useSettings } from "@/hooks/use-settings";
import { ipc } from "@/lib/tauri";
import type { StreakInfo, Totals, Transcription } from "@/types/ipc";

export function HomeRoute() {
  const [totals, setTotals] = useState<Totals | null>(null);
  const [streak, setStreak] = useState<StreakInfo | null>(null);
  const [recent, setRecent] = useState<Transcription[]>([]);
  const { settings } = useSettings();
  const dictationHotkey = settings?.hotkey || "F24";
  const emailHotkey = settings?.commandHotkey || "Pause";

  useEffect(() => {
    void Promise.all([
      ipc.getStatsTotals().catch(() => null),
      ipc.getStatsStreak().catch(() => null),
      ipc.listTranscriptions(5, 0).catch(() => []),
    ]).then(([t, s, r]) => {
      setTotals(t);
      setStreak(s);
      setRecent(r);
    });
  }, []);

  return (
    <PageShell title="Home" description="Quick overview and controls.">
      <div className="grid gap-4 lg:grid-cols-2">
        <Panel title="Speech to Text" description="Dictate and transcribe anywhere.">
          <button
            type="button"
            className="flex w-full items-center justify-between gap-3 rounded-xl border border-border bg-white/72 p-3 text-left shadow-sm transition-colors hover:bg-white focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/35"
            onClick={() => void ipc.toggleRecording()}
          >
            <span className="flex items-center gap-3">
              <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
                <Mic className="h-4 w-4" />
              </span>
              <span>
                <span className="block text-sm font-medium">Toggle dictation</span>
                <span className="block text-xs text-muted-foreground">{dictationHotkey} hotkey works anywhere</span>
              </span>
            </span>
            <Keyboard className="h-4 w-4 text-muted-foreground" />
          </button>
          <div className="mt-4 flex h-14 items-center gap-1 overflow-hidden rounded-xl bg-accent/45 px-3">
            {Array.from({ length: 42 }).map((_, index) => (
              <span
                key={index}
                className="w-1 rounded-full bg-primary/70"
                style={{ height: `${18 + ((index * 13) % 30)}px` }}
              />
            ))}
          </div>
        </Panel>
        <Panel title="Email Drafter" description="Draft emails from your voice.">
          <Link
            to="/email-drafts"
            className="flex w-full items-center justify-between gap-3 rounded-xl border border-border bg-white/72 p-3 text-left shadow-sm transition-colors hover:bg-white focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/35"
          >
            <span className="flex items-center gap-3">
              <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-sky-100 text-primary">
                <Mail className="h-4 w-4" />
              </span>
              <span>
                <span className="block text-sm font-medium">Open email drafts</span>
                <span className="block text-xs text-muted-foreground">{emailHotkey} hotkey drafts from voice</span>
              </span>
            </span>
            <ArrowRight className="h-4 w-4 text-muted-foreground" />
          </Link>
          <div className="mt-4 flex h-14 items-center gap-1 overflow-hidden rounded-xl bg-sky-50 px-3">
            {Array.from({ length: 42 }).map((_, index) => (
              <span
                key={index}
                className="w-1 rounded-full bg-sky-400/75"
                style={{ height: `${16 + ((index * 17) % 32)}px` }}
              />
            ))}
          </div>
        </Panel>
      </div>
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard label="Total words" value={totals?.wordCount ?? 0} icon={<Type className="h-4 w-4 text-muted-foreground" />} />
        <StatCard label="Sessions" value={totals?.sessionCount ?? 0} icon={<Mic className="h-4 w-4 text-muted-foreground" />} />
        <StatCard label="Current streak" value={streak?.current ?? 0} icon={<Flame className="h-4 w-4 text-muted-foreground" />} />
        <StatCard label="Longest streak" value={streak?.longest ?? 0} icon={<Clock className="h-4 w-4 text-muted-foreground" />} />
      </div>
      <Panel
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
          <div className="space-y-3">
            {recent.map((row) => <TranscriptionRow key={row.id} row={row} />)}
          </div>
        )}
      </Panel>
    </PageShell>
  );
}
