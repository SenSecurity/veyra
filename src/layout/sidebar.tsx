import { Link } from "@tanstack/react-router";
import { getVersion } from "@tauri-apps/api/app";
import type { ComponentType } from "react";
import { useEffect, useState } from "react";
import {
  BookOpen,
  Clock,
  Home,
  Mail,
  PanelLeft,
  Settings,
} from "lucide-react";
import { EngineCard } from "@/components/engine-card";
import { useSettings } from "@/hooks/use-settings";
import { cn } from "@/lib/utils";

const STORAGE_KEY = "typr.sidebar.collapsed";

interface NavItem {
  to: string;
  label: string;
  icon: ComponentType<{ className?: string }>;
}

const items: NavItem[] = [
  { to: "/", label: "Home", icon: Home },
  { to: "/history", label: "History", icon: Clock },
  { to: "/email-drafts", label: "Email Drafter", icon: Mail },
  { to: "/dictionary", label: "Dictionary", icon: BookOpen },
  { to: "/settings/general", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const [version, setVersion] = useState("");
  const { settings } = useSettings();
  const [collapsed, setCollapsed] = useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    try {
      return window.localStorage?.getItem?.(STORAGE_KEY) === "1";
    } catch {
      return false;
    }
  });

  useEffect(() => {
    void getVersion().then((value) => setVersion(value)).catch(() => setVersion(""));
  }, []);

  useEffect(() => {
    try {
      window.localStorage?.setItem?.(STORAGE_KEY, collapsed ? "1" : "0");
    } catch {
      /* ignore — non-DOM test env */
    }
  }, [collapsed]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Ctrl+\ toggles sidebar collapse. `\` arrives as `Backslash` code on
      // most layouts; check both `e.key` and `e.code` for portability.
      if ((e.ctrlKey || e.metaKey) && (e.key === "\\" || e.code === "Backslash")) {
        e.preventDefault();
        setCollapsed((c) => !c);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const sttName = formatWhisperShort(settings?.whisperModel);
  const sttItalic = formatWhisperItalic(settings?.whisperModel);
  const drafterName = formatDrafterShort(
    settings?.emailDraftEngine,
    settings?.emailDraftModel,
  );
  const drafterItalic = formatDrafterItalic(
    settings?.emailDraftEngine,
    settings?.emailDraftModel,
  );
  const drafterRuntime = settings?.emailDraftEngine === "groq" ? "Groq" : "Ollama";

  return (
    <aside
      className={cn(
        "flex shrink-0 flex-col border-r border-border/70 bg-[linear-gradient(180deg,var(--ice-50)_0%,#ffffff_100%)] text-foreground transition-[width] duration-150",
        collapsed ? "w-14" : "w-[244px]",
      )}
      style={{ width: collapsed ? 56 : 244 }}
      aria-label="Primary navigation"
    >
      {!collapsed ? (
        <div className="px-4 pt-4 pb-1.5 font-mono text-[0.6rem] tracking-[0.24em] uppercase text-muted-foreground">
          Workspace
        </div>
      ) : null}
      <nav className="space-y-0.5 px-2 pt-2">
        {items.map((it) => {
          const Icon = it.icon;
          return (
            <Link
              key={it.to}
              to={it.to}
              className={cn(
                "group relative flex h-9 items-center gap-3 rounded-lg px-3 text-[0.85rem] font-medium text-foreground/65 transition-colors hover:bg-white hover:text-foreground hover:shadow-[0_1px_0_rgb(12_17_28_/_0.04)] focus-visible:ring-2 focus-visible:ring-ring/50",
                collapsed && "justify-center px-0",
              )}
              activeProps={{
                className:
                  "bg-white text-foreground shadow-[0_1px_0_rgb(12_17_28_/_0.04),inset_0_0_0_1px_var(--hairline)] before:pointer-events-none before:absolute before:left-[-1px] before:top-2 before:bottom-2 before:w-[2px] before:rounded-[2px] before:bg-[linear-gradient(180deg,var(--cyan),var(--cyan-deep))] before:shadow-[0_0_8px_var(--halo)]",
              }}
              activeOptions={{ exact: it.to === "/" }}
              title={collapsed ? it.label : undefined}
            >
              <Icon className="h-4 w-4 shrink-0" />
              {!collapsed && <span>{it.label}</span>}
            </Link>
          );
        })}
      </nav>

      {!collapsed ? (
        <>
          <div className="mt-5 px-4 pb-1.5 font-mono text-[0.6rem] tracking-[0.24em] uppercase text-muted-foreground">
            Engines · 02
          </div>
          <div className="mx-3 overflow-hidden rounded-xl border border-border/70 bg-white shadow-[inset_0_1px_0_rgb(255_255_255_/_0.9),0_1px_0_rgb(12_17_28_/_0.025)]">
            <EngineCard
              role="stt"
              index={1}
              name={sttName}
              italic={sttItalic}
              meta={[
                { value: "1.5 GB", bold: true },
                { value: "16 kHz" },
              ]}
            />
            <div className="h-px bg-border/70" aria-hidden="true" />
            <EngineCard
              role="drafter"
              index={2}
              name={drafterName}
              italic={drafterItalic}
              meta={[
                { value: drafterRuntime, bold: true },
                { value: "Local" },
              ]}
            />
          </div>

          <div className="mx-3 mt-3 rounded-xl border border-border/70 bg-white/60 px-3 py-2.5">
            <StatusRow label="Storage" value="2.3 GB" />
            <StatusRow label="Models" value="4 installed" />
            <StatusRow label="Status" value="All systems nominal" tone="cyan" />
          </div>
        </>
      ) : null}

      <button
        type="button"
        onClick={() => setCollapsed((c) => !c)}
        className={cn(
          "mx-2 mt-auto mb-2 flex h-8 items-center justify-center rounded-lg border border-border/70 bg-white/85 text-[0.7rem] text-muted-foreground shadow-[0_1px_0_rgb(255_255_255_/_0.9)] transition-colors hover:bg-white hover:text-foreground",
          collapsed ? "w-10" : "gap-1.5 px-2.5",
        )}
        title="Toggle sidebar (Ctrl+\\)"
        aria-label="Toggle sidebar"
      >
        <PanelLeft className="h-3.5 w-3.5" />
        {!collapsed && <span>{version ? `v${version}` : "Collapse"}</span>}
      </button>
    </aside>
  );
}

function StatusRow({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "cyan";
}) {
  return (
    <div className="flex items-center justify-between gap-2 py-1 text-[0.7rem]">
      <span className="font-mono text-[0.6rem] tracking-[0.22em] uppercase text-muted-foreground">
        {label}
      </span>
      <span
        className={cn(
          "inline-flex items-center gap-1.5 font-medium",
          tone === "cyan" ? "text-[var(--cyan-deep)]" : "text-foreground",
        )}
      >
        {tone === "cyan" ? (
          <span
            className="h-1.5 w-1.5 rounded-full bg-[var(--cyan)] shadow-[0_0_5px_var(--halo)]"
            aria-hidden="true"
          />
        ) : null}
        {value}
      </span>
    </div>
  );
}

// ---------- model name formatters ----------

function formatWhisperShort(raw: string | undefined): string {
  if (!raw) return "Whisper";
  return "Whisper";
}

function formatWhisperItalic(raw: string | undefined): string {
  if (!raw) return "turbo";
  const lower = raw.toLowerCase();
  if (lower.includes("large-v3-turbo") || lower === "turbo") return "turbo";
  if (lower.includes("medium")) return "medium";
  if (lower.includes("small")) return "small";
  if (lower.includes("base")) return "base";
  return raw.replace(/^ggml-|\.bin$/g, "");
}

function formatDrafterShort(
  engine: "ollama" | "groq" | undefined,
  model: string | undefined,
): string {
  if (engine === "groq") return model ? "Groq" : "Groq";
  if (!model) return "Llama";
  const stem = model.split(":")[0]?.split(/[-/]/)[0] ?? "";
  if (!stem) return "Llama";
  if (/^llama/i.test(stem)) return "Llama";
  if (/^qwen/i.test(stem)) return "Qwen";
  if (/^bonsai/i.test(stem)) return "Bonsai";
  return stem.charAt(0).toUpperCase() + stem.slice(1);
}

function formatDrafterItalic(
  engine: "ollama" | "groq" | undefined,
  model: string | undefined,
): string {
  if (!model) return "3.2 · 1B";
  const tag = model.split(":")[1] ?? "";
  const stem = model.split(":")[0] ?? model;
  // try to surface a version + size for Llama-style names
  const versionMatch = stem.match(/(\d+\.?\d*)/);
  const version = versionMatch ? versionMatch[1] : "";
  const size = tag ? tag.toUpperCase() : "";
  return [version, size].filter(Boolean).join(" · ") || (engine === "groq" ? "cloud" : "local");
}
