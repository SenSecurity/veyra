import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { PageShell } from "@/components/page-shell";
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
    <PageShell title="Settings" description="Configure transcription, models, and hotkeys.">
      <div className="rounded-2xl border border-border bg-white/66 p-1 shadow-sm">
        <nav className="flex gap-1 overflow-x-auto" aria-label="Settings sections">
          {tabs.map((t) => {
            const active = pathname.startsWith(t.to);
            return (
              <Link
                key={t.to}
                to={t.to}
                className={cn(
                  "whitespace-nowrap rounded-xl px-3 py-1.5 text-sm font-medium transition-colors focus-visible:ring-2 focus-visible:ring-ring/35",
                  active
                    ? "bg-white text-foreground shadow-sm ring-1 ring-border/60"
                    : "text-muted-foreground hover:bg-white/70 hover:text-foreground",
                )}
              >
                {t.label}
              </Link>
            );
          })}
        </nav>
      </div>
      <Outlet />
    </PageShell>
  );
}
