import { Outlet } from "@tanstack/react-router";
import { Sidebar } from "@/layout/sidebar";
import { TopBar } from "@/layout/topbar";
import { CommandPalette } from "@/layout/command-palette";
import { WindowTitleBar } from "@/layout/window-titlebar";
import { Toaster } from "@/components/ui/sonner";
import { useLiveEvents } from "@/hooks/use-live-events";
import { useWizardGate } from "@/hooks/use-wizard-gate";

// Root shell. Mounted by the rootRoute in router.tsx; renders the global
// chrome (sidebar + topbar + command-palette stub) around the active route.
export function App() {
  useLiveEvents();
  useWizardGate();

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground font-sans">
      <WindowTitleBar />
      <div className="flex min-h-0 flex-1 bg-app">
        <Sidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <TopBar />
          <main className="flex-1 overflow-auto bg-[linear-gradient(180deg,color-mix(in_oklab,var(--app)_78%,white)_0%,var(--app)_220px)]">
            <Outlet />
          </main>
        </div>
      </div>
      <CommandPalette />
      <Toaster />
    </div>
  );
}
