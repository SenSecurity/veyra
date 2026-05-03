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
        "flex shrink-0 flex-col border-r border-sidebar-border/90 bg-sidebar/95 text-sidebar-foreground shadow-[inset_-1px_0_0_rgb(255_255_255_/_0.55)] transition-[width] duration-150",
        collapsed ? "w-14" : "w-44",
      )}
      style={{ width: collapsed ? 56 : 176 }}
      aria-label="Primary navigation"
    >
      <nav className="flex-1 space-y-1 overflow-y-auto p-2.5 pt-4">
        {items.map((it) => {
          const Icon = it.icon;
          return (
            <Link
              key={it.to}
              to={it.to}
              className={cn(
                "flex h-9 items-center gap-2 rounded-lg px-2.5 text-sm text-sidebar-foreground/70 transition-colors hover:bg-white/62 hover:text-sidebar-accent-foreground",
                collapsed && "justify-center",
              )}
              activeProps={{
                className:
                  "bg-sidebar-accent text-sidebar-accent-foreground font-medium shadow-sm ring-1 ring-white/58",
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
      <div className={cn("mx-2.5 mb-2 rounded-xl border border-sidebar-border/80 bg-white/60 p-2 shadow-sm", collapsed && "hidden")}>
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-emerald-500 shadow-[0_0_0_3px_rgb(16_185_129_/_0.14)]" />
          <div className="min-w-0">
            <p className="truncate text-[0.72rem] font-medium text-sidebar-foreground">All systems operational</p>
            <p className="truncate text-[0.68rem] text-sidebar-foreground/55">Local services running</p>
          </div>
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
