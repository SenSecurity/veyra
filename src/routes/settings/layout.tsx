import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { cn } from "@/lib/utils";

const tabs: { to: string; label: string }[] = [
  { to: "/settings/general", label: "General" },
  { to: "/settings/transcription", label: "Transcription" },
  { to: "/settings/hotkeys", label: "Hotkeys" },
];

// Settings parent layout: horizontal tab strip + outlet for active tab.
// Uses a plain link strip rather than the shadcn `<Tabs>` component because
// each tab is its own route; the shadcn Tabs is for in-page state, not
// router-driven state.
export function SettingsLayout() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-border bg-background/95">
        <nav className="flex gap-1 overflow-x-auto px-4 py-2" aria-label="Settings sections">
          {tabs.map((t) => {
            const active = pathname.startsWith(t.to);
            return (
              <Link
                key={t.to}
                to={t.to}
                className={cn(
                  "rounded-md px-3 py-1.5 text-sm whitespace-nowrap transition-colors",
                  active
                    ? "bg-muted text-foreground"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                {t.label}
              </Link>
            );
          })}
        </nav>
      </div>
      <div className="flex-1 overflow-auto">
        <Outlet />
      </div>
    </div>
  );
}
