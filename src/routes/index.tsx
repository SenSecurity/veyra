import { useEffect, useState } from "react";
import { Clock, Flame, Mic, Type } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { StatCard } from "@/components/stat-card";
import { TranscriptionRow } from "@/components/transcription-row";
import { ipc } from "@/lib/tauri";
import type { StreakInfo, Totals, Transcription } from "@/types/ipc";

export function HomeRoute() {
  const [totals, setTotals] = useState<Totals | null>(null);
  const [streak, setStreak] = useState<StreakInfo | null>(null);
  const [recent, setRecent] = useState<Transcription[]>([]);

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
    <section className="mx-auto max-w-5xl space-y-6 p-8">
      <div>
        <h1 className="text-3xl font-semibold tracking-tight">Home</h1>
        <p className="text-sm text-muted-foreground">Dictation activity and recent captures.</p>
      </div>
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard label="Total words" value={totals?.wordCount ?? 0} icon={<Type className="h-4 w-4 text-muted-foreground" />} />
        <StatCard label="Sessions" value={totals?.sessionCount ?? 0} icon={<Mic className="h-4 w-4 text-muted-foreground" />} />
        <StatCard label="Current streak" value={streak?.current ?? 0} icon={<Flame className="h-4 w-4 text-muted-foreground" />} />
        <StatCard label="Longest streak" value={streak?.longest ?? 0} icon={<Clock className="h-4 w-4 text-muted-foreground" />} />
      </div>
      <div className="space-y-3">
        <h2 className="text-sm font-semibold">Recent transcriptions</h2>
        {recent.length === 0 ? (
          <EmptyState title="No transcriptions yet">Record once and the last captures appear here.</EmptyState>
        ) : (
          recent.map((row) => <TranscriptionRow key={row.id} row={row} />)
        )}
      </div>
    </section>
  );
}
