import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import {
  BookOpen,
  ChevronLeft,
  ChevronRight,
  Clock,
  Home,
  Mail,
  Settings,
} from "lucide-react";
import { cn } from "@/lib/utils";

const STORAGE_KEY = "typr.sidebar.collapsed";

interface NavItem {
  to: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}

const items: NavItem[] = [
  { to: "/", label: "Home", icon: Home },
  { to: "/history", label: "History", icon: Clock },
  { to: "/email-drafts", label: "Email Drafter", icon: Mail },
  { to: "/dictionary", label: "Dictionary", icon: BookOpen },
  { to: "/settings/general", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const [collapsed, setCollapsed] = useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    return window.localStorage.getItem(STORAGE_KEY) === "1";
  });

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
        "flex flex-col shrink-0 border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-[width] duration-150",
        collapsed ? "w-14" : "w-[220px]",
      )}
      style={{ width: collapsed ? 56 : 220 }}
      aria-label="Primary navigation"
    >
      <div
        className={cn(
          "h-12 flex items-center border-b border-border",
          collapsed ? "justify-center" : "px-4",
        )}
      >
        <span className="text-sm font-semibold tracking-tight">
          {collapsed ? "V" : "Veyra"}
        </span>
      </div>
      <nav className="flex-1 overflow-y-auto p-2 space-y-1">
        {items.map((it) => {
          const Icon = it.icon;
          return (
            <Link
              key={it.to}
              to={it.to}
              className={cn(
                "flex items-center gap-2 rounded-md px-2 py-2 text-sm text-sidebar-foreground/75 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground transition-colors",
                collapsed && "justify-center",
              )}
              activeProps={{ className: "bg-sidebar-accent text-sidebar-accent-foreground font-medium" }}
              activeOptions={{ exact: false }}
              title={collapsed ? it.label : undefined}
            >
              <Icon className="h-4 w-4 shrink-0" />
              {!collapsed && <span>{it.label}</span>}
            </Link>
          );
        })}
      </nav>
      <button
        type="button"
        onClick={() => setCollapsed((c) => !c)}
        className={cn(
          "m-2 flex h-8 items-center justify-center rounded-md border border-sidebar-border text-xs text-sidebar-foreground/75 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
          collapsed ? "w-10" : "gap-1 px-2",
        )}
        title="Toggle sidebar (Ctrl+\\)"
        aria-label="Toggle sidebar"
      >
        {collapsed ? (
          <ChevronRight className="h-4 w-4" />
        ) : (
          <>
            <ChevronLeft className="h-4 w-4" />
            <span>Collapse</span>
          </>
        )}
      </button>
    </aside>
  );
}
