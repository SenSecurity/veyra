import { Link } from "@tanstack/react-router";
import { getVersion } from "@tauri-apps/api/app";
import type { ComponentType } from "react";
import { useEffect, useState } from "react";
import {
  BookOpen,
  Clock,
  Home,
  Mail,
  Settings,
  SlidersHorizontal,
} from "lucide-react";
import { BrandMark } from "@/components/brand-mark";
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
    return window.localStorage.getItem(STORAGE_KEY) === "1";
  });

  useEffect(() => {
    void getVersion().then((value) => setVersion(value)).catch(() => setVersion(""));
  }, []);

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, collapsed ? "1" : "0");
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

  return (
    <aside
      className={cn(
        "flex shrink-0 flex-col border-r border-sidebar-border/90 bg-[linear-gradient(180deg,color-mix(in_oklab,var(--sidebar)_92%,white),color-mix(in_oklab,var(--sidebar)_78%,#d8efff))] text-sidebar-foreground shadow-[inset_-1px_0_0_rgb(255_255_255_/_0.68)] backdrop-blur transition-[width] duration-150",
        collapsed ? "w-14" : "w-[182px]",
      )}
      style={{ width: collapsed ? 56 : 182 }}
      aria-label="Primary navigation"
    >
      <nav className="min-h-0 flex-1 space-y-1 overflow-auto p-2.5 pt-4">
        {items.map((it) => {
          const Icon = it.icon;
          return (
            <Link
              key={it.to}
              to={it.to}
              className={cn(
                "flex h-9 items-center gap-2 rounded-xl px-2.5 text-sm font-medium text-sidebar-foreground/72 transition-colors hover:bg-white/62 hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-ring/35",
                collapsed && "justify-center",
              )}
              activeProps={{
                className:
                  "bg-sidebar-accent text-sidebar-accent-foreground font-semibold shadow-sm ring-1 ring-white/62",
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
      <div className={cn("mx-2.5 mb-2 rounded-2xl border border-sidebar-border/80 bg-white/64 p-2.5 shadow-sm", collapsed && "hidden")}>
        <div className="flex items-center gap-2">
          <BrandMark className="h-7 w-7 rounded-xl" />
          <div className="min-w-0">
            <p className="truncate text-[0.72rem] font-semibold text-sidebar-foreground">Veyra ready</p>
            <p className="truncate text-[0.68rem] text-sidebar-foreground/55">Local services running</p>
          </div>
        </div>
        <div className="mt-2 space-y-1.5 border-t border-sidebar-border/70 pt-2">
          <StatusLine label="Speech" value={settings?.hotkey ?? "F24"} tone="blue" />
          <StatusLine label="Email" value={settings?.commandHotkey ?? "Pause"} tone="orange" />
          <StatusLine label="Models" value={settings?.emailDraftEngine === "ollama" ? "Local" : "Cloud"} tone="green" />
        </div>
      </div>
      <button
        type="button"
        onClick={() => setCollapsed((c) => !c)}
        className={cn(
          "m-2 mt-0 flex h-8 items-center justify-center rounded-lg border border-sidebar-border/80 bg-white/58 text-xs text-sidebar-foreground/70 shadow-sm transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
          collapsed ? "w-10" : "gap-1 px-2",
        )}
        title="Toggle sidebar (Ctrl+\\)"
        aria-label="Toggle sidebar"
      >
        {collapsed ? (
          <SlidersHorizontal className="h-4 w-4" />
        ) : (
          <>
            <SlidersHorizontal className="h-4 w-4" />
            <span>{version ? `v${version}` : "Collapse"}</span>
          </>
        )}
      </button>
    </aside>
  );
}

function StatusLine({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "blue" | "orange" | "green";
}) {
  const dot = {
    blue: "bg-sky-500 shadow-[0_0_0_3px_rgb(14_165_233_/_0.13)]",
    orange: "bg-orange-400 shadow-[0_0_0_3px_rgb(251_146_60_/_0.13)]",
    green: "bg-emerald-500 shadow-[0_0_0_3px_rgb(16_185_129_/_0.13)]",
  }[tone];

  return (
    <div className="flex items-center justify-between gap-2 text-[0.68rem] leading-4">
      <span className="flex min-w-0 items-center gap-1.5 text-sidebar-foreground/68">
        <span className={cn("h-1.5 w-1.5 shrink-0 rounded-full", dot)} />
        <span className="truncate">{label}</span>
      </span>
      <span className="max-w-[5.5rem] truncate rounded-md bg-white/58 px-1.5 py-0.5 font-medium text-sidebar-foreground/78 ring-1 ring-sidebar-border/70">
        {value}
      </span>
    </div>
  );
}
