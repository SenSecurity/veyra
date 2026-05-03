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
      <div className="veyra-window-shell flex h-screen overflow-hidden font-sans text-foreground">
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <WindowTitleBar setupMode />
        <main className="min-h-0 flex-1 overflow-hidden bg-[radial-gradient(circle_at_20%_0%,rgb(59_130_246_/_0.16),transparent_34%),radial-gradient(circle_at_86%_22%,rgb(14_165_233_/_0.12),transparent_30%),linear-gradient(135deg,oklch(0.992_0.010_248),oklch(0.953_0.030_245)_58%,oklch(0.928_0.040_238))]">
          <Outlet />
        </main>
        </div>
        <Toaster />
      </div>
    );
  }

  return (
    <div className="veyra-window-shell flex h-screen flex-col overflow-hidden font-sans text-foreground">
      <WindowTitleBar />
      <div className="flex min-h-0 flex-1 overflow-hidden bg-app">
        <Sidebar />
        <div className="flex min-w-0 flex-1 flex-col overflow-hidden">
          <main className="min-h-0 flex-1 overflow-hidden bg-[radial-gradient(circle_at_12%_0%,rgb(59_130_246_/_0.13),transparent_32%),radial-gradient(circle_at_94%_8%,rgb(14_165_233_/_0.10),transparent_28%),linear-gradient(135deg,oklch(0.992_0.010_245)_0%,oklch(0.970_0.024_246)_48%,oklch(0.940_0.035_238)_100%)]">
            <Outlet />
          </main>
        </div>
      </div>
      <CommandPalette />
      <Toaster />
    </div>
  );
}
