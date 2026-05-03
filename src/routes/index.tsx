import { useEffect, useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import { ArrowRight, CheckCircle2, Mail, Mic, Plus } from "lucide-react";
import { toast } from "sonner";
import { ActivityRow } from "@/components/activity-row";
import { BrandMark } from "@/components/brand-mark";
import { EmptyState } from "@/components/empty-state";
import { KpiStrip, type KpiCell } from "@/components/kpi-strip";
import { PageShell, Panel } from "@/components/page-shell";
import { Button } from "@/components/ui/button";
import { WaveStage } from "@/components/wave-stage";
import { useSettings } from "@/hooks/use-settings";
import { isUsableTranscription } from "@/lib/email-output-quality";
import { ipc } from "@/lib/tauri";
import type { StreakInfo, Totals, Transcription } from "@/types/ipc";

export function HomeRoute() {
  const [totals, setTotals] = useState<Totals | null>(null);
  const [streak, setStreak] = useState<StreakInfo | null>(null);
  const [recent, setRecent] = useState<Transcription[]>([]);
  const [todayDrafts, setTodayDrafts] = useState<number | null>(null);
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
      ipc.listTranscriptions(50, 0).catch(() => [] as Transcription[]),
    ]).then(([t, s, all]) => {
      setTotals(t);
      setStreak(s);
      const usable = (all as Transcription[]).filter(isUsableTranscription);
      setRecent(usable.slice(0, 5));
      setTodayDrafts(countTodayDrafts(usable));
    });
  }, []);

  const eyebrow = useMemo(() => {
    const date = new Intl.DateTimeFormat(undefined, {
      weekday: "long",
      day: "2-digit",
      month: "long",
    }).format(new Date());
    return `Workspace · Home · ${capitalize(date)}`;
  }, []);

  const greeting = useMemo(getGreeting, []);

  const kpiCells: KpiCell[] = [
    {
      label: "Sessions today",
      value: streak?.current != null ? String(streak.current) : "0",
    },
    {
      label: "Words transcribed",
      value: formatNumber(totals?.wordCount ?? 0),
      unit: "w",
    },
    {
      label: "Drafts composed",
      value: todayDrafts != null ? String(todayDrafts) : "0",
    },
    {
      label: "STT latency · p50",
      value: "—",
    },
  ];

  return (
    <PageShell
      className="max-w-[1080px]"
      eyebrow={eyebrow}
      title={
        <span>
          {greeting}, Bruno.{" "}
          <span className="veyra-italic ml-1 text-[var(--cyan-deep)]">
            — quiet desk, ready when you are.
          </span>
        </span>
      }
      description="Two engines awake on this machine. Press F24 to dictate, Pause to draft an email — both run locally and never leave your disk."
      action={
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm">
            <Plus className="h-3.5 w-3.5" />
            New session
          </Button>
          <Button size="sm" onClick={() => void ipc.toggleRecording()}>
            <Mic className="h-3.5 w-3.5" />
            Start dictation
            <span className="ml-1 rounded-md border border-white/20 bg-white/15 px-1 font-mono text-[0.6rem] tracking-[0.06em]">
              {dictationHotkey}
            </span>
          </Button>
        </div>
      }
    >
      {/* Hero brand mark + KPI strip side by side */}
      <div className="grid shrink-0 gap-4 sm:grid-cols-[88px_1fr] sm:items-center">
        <BrandMark className="h-[88px] w-[88px] rounded-[22px]" />
        <KpiStrip cells={kpiCells} />
      </div>

      {/* Two engine cards */}
      <div className="grid shrink-0 gap-4 md:grid-cols-2">
        <Panel
          className="veyra-command-panel"
          eyebrow="01 · Capture · STT"
          title={
            <span>
              Speech to Text{" "}
              <span className="veyra-italic ml-0.5 text-[var(--cyan-deep)]">— Whisper</span>
            </span>
          }
          description="Dictate into any focused field. Whisper transcribes locally and pastes at the cursor."
          action={operational}
        >
          <button
            type="button"
            className="flex w-full items-center justify-between gap-3 rounded-xl border border-border/70 bg-[var(--paper)] p-3 text-left transition-colors hover:bg-white focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/35"
            onClick={() => void ipc.toggleRecording()}
          >
            <span className="flex items-center gap-3">
              <span className="flex h-9 w-9 items-center justify-center rounded-[11px] border border-[rgba(43,199,255,0.28)] bg-[var(--ice-50)] text-[var(--cyan-deep)]">
                <Mic className="h-4 w-4" strokeWidth={1.6} />
              </span>
              <span>
                <span className="block text-sm font-semibold tracking-[-0.005em]">
                  Hold {dictationHotkey} to dictate
                </span>
                <span className="block text-xs text-muted-foreground">
                  Press once to start, again to stop.
                </span>
              </span>
            </span>
            <span className="rounded-md border border-border/70 bg-white px-2 py-[3px] font-mono text-[0.65rem] tracking-[0.04em] text-foreground/85 shadow-[inset_0_1px_0_rgb(255_255_255_/_0.9),0_1px_0_rgb(12_17_28_/_0.06)]">
              {dictationHotkey}
            </span>
          </button>
          <div className="mt-3">
            <WaveStage variant="stt" />
          </div>
          <SpecRow
            cells={[
              { label: "Model", value: "Turbo", suffix: "1.5 GB" },
              { label: "Latency", value: "112", suffix: "ms" },
              { label: "Language", value: "pt-PT", suffix: "auto" },
            ]}
          />
        </Panel>

        <Panel
          className="veyra-command-panel veyra-command-panel-spark"
          eyebrow="02 · Compose · LLM"
          title={
            <span>
              Email Drafter{" "}
              <span className="veyra-italic ml-0.5 text-[var(--spark-deep)]">— Llama</span>
            </span>
          }
          description="Speak an instruction. The local LLM drafts a polished message, kept on your machine."
          action={operational}
        >
          <Link
            to="/email-drafts"
            className="flex w-full items-center justify-between gap-3 rounded-xl border border-border/70 bg-[var(--paper)] p-3 text-left transition-colors hover:bg-white focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/35"
          >
            <span className="flex items-center gap-3">
              <span className="flex h-9 w-9 items-center justify-center rounded-[11px] border border-[rgba(255,138,31,0.28)] bg-[#fff5e6] text-[var(--spark-deep)]">
                <Mail className="h-4 w-4" strokeWidth={1.6} />
              </span>
              <span>
                <span className="block text-sm font-semibold tracking-[-0.005em]">
                  Hold {emailHotkey} to draft
                </span>
                <span className="block text-xs text-muted-foreground">
                  Voice instruction → finished draft.
                </span>
              </span>
            </span>
            <span className="rounded-md border border-border/70 bg-white px-2 py-[3px] font-mono text-[0.65rem] tracking-[0.04em] text-foreground/85 shadow-[inset_0_1px_0_rgb(255_255_255_/_0.9),0_1px_0_rgb(12_17_28_/_0.06)]">
              {emailHotkey}
            </span>
          </Link>
          <div className="mt-3">
            <WaveStage variant="drafter" />
          </div>
          <SpecRow
            cells={[
              { label: "Model", value: "Llama 3.2", suffix: "1B" },
              { label: "Quant", value: "Q4_K_M", suffix: "0.8 GB" },
              { label: "Runtime", value: "Ollama", suffix: "local" },
            ]}
          />
        </Panel>
      </div>

      {/* Recent activity */}
      <Panel
        className="min-h-0 flex-1"
        title={
          <span>
            Recent activity{" "}
            <span className="veyra-italic ml-0.5 text-[var(--cyan-deep)]">— last 24h</span>
          </span>
        }
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
          <EmptyState title="No transcriptions yet">
            Record once and the last captures appear here.
          </EmptyState>
        ) : (
          <div className="h-full overflow-auto rounded-2xl border border-border/70 bg-white">
            {recent.map((row, i) => (
              <div
                key={row.id}
                className={i < recent.length - 1 ? "border-b border-border/70" : ""}
              >
                <ActivityRow row={row} onCopy={copyRecent} />
              </div>
            ))}
          </div>
        )}
      </Panel>
    </PageShell>
  );
}

function SpecRow({
  cells,
}: {
  cells: { label: string; value: string; suffix?: string }[];
}) {
  return (
    <div className="mt-3 grid grid-cols-3 border-t border-border/70 pt-3">
      {cells.map((cell, i) => (
        <div
          key={cell.label}
          className={
            i < cells.length - 1
              ? "border-r border-border/70 pr-3"
              : "pl-3"
          }
        >
          <div className="font-mono text-[0.6rem] tracking-[0.22em] uppercase text-muted-foreground">
            {cell.label}
          </div>
          <div className="mt-1 text-[0.82rem] font-semibold tracking-[-0.005em] text-foreground tabular-nums">
            {cell.value}
            {cell.suffix ? (
              <span className="ml-1 text-[0.7rem] font-normal text-muted-foreground">
                {cell.suffix}
              </span>
            ) : null}
          </div>
        </div>
      ))}
    </div>
  );
}

function countTodayDrafts(rows: Transcription[]): number {
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const cutoff = today.getTime();
  return rows.filter((r) => {
    if (r.mode !== "command") return false;
    const ms = r.createdAt > 1e12 ? r.createdAt : r.createdAt * 1000;
    return ms >= cutoff;
  }).length;
}

function formatNumber(n: number): string {
  return n.toLocaleString(undefined);
}

function capitalize(s: string): string {
  if (!s) return s;
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function getGreeting(): string {
  const h = new Date().getHours();
  if (h < 5) return "Boa noite";
  if (h < 12) return "Bom dia";
  if (h < 19) return "Boa tarde";
  return "Boa noite";
}
