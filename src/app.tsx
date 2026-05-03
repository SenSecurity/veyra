import { Outlet } from "@tanstack/react-router";
import { Sidebar } from "@/layout/sidebar";
import { CommandPalette } from "@/layout/command-palette";
import { WindowTitleBar } from "@/layout/window-titlebar";
import { Toaster } from "@/components/ui/sonner";
import { useLiveEvents } from "@/hooks/use-live-events";
import { useWizardGate } from "@/hooks/use-wizard-gate";

// Root shell. Mounted by the rootRoute in router.tsx; renders the global
// chrome (sidebar + topbar + command-palette stub) around the active route.
export function App() {
  useLiveEvents();
  const { completed: wizardCompleted } = useWizardGate();

  if (wizardCompleted !== true) {
    return (
      <div className="flex min-h-screen flex-col bg-background font-sans text-foreground">
        <WindowTitleBar setupMode />
        <main className="min-h-0 flex-1 overflow-auto bg-[radial-gradient(circle_at_50%_8%,rgb(74_158_255_/_0.18),transparent_34%),linear-gradient(180deg,oklch(0.994_0.006_245),oklch(0.958_0.022_245))]">
          <Outlet />
        </main>
        <Toaster />
      </div>
    );
  }

  return (
    <div className="flex min-h-screen flex-col bg-background font-sans text-foreground">
      <WindowTitleBar />
      <div className="flex min-h-0 flex-1 bg-app">
        <Sidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <main className="flex-1 overflow-auto bg-[linear-gradient(180deg,oklch(0.992_0.008_245)_0%,oklch(0.972_0.016_245)_46%,var(--app)_100%)]">
            <Outlet />
          </main>
        </div>
      </div>
      <CommandPalette />
      <Toaster />
    </div>
  );
}
