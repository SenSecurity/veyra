import { Minus, Search, Square, X } from "lucide-react";
import { BrandMark } from "@/components/brand-mark";
import { EngineBadge } from "@/components/engine-badge";
import { ipc } from "@/lib/tauri";

export function WindowTitleBar({ setupMode = false }: { setupMode?: boolean }) {
  return (
    <header
      className="flex h-9 shrink-0 items-center border-b border-border/70 bg-[linear-gradient(180deg,rgb(255_255_255_/_0.92),rgb(255_255_255_/_0.78))] text-foreground backdrop-blur-md"
    >
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-2 px-3">
        <BrandMark className="h-[22px] w-[22px] rounded-[7px]" />
        <span className="text-[0.78rem] font-semibold tracking-[-0.005em] text-foreground">Veyra</span>
      </div>
      {!setupMode ? (
        <>
          <div className="mr-2 hidden sm:block">
            <EngineBadge />
          </div>
          <button
            type="button"
            className="mr-2 hidden h-6 items-center gap-1.5 rounded-lg border border-border/70 bg-white/85 px-2 text-[0.7rem] text-muted-foreground shadow-[inset_0_1px_0_rgb(255_255_255_/_0.9),0_1px_2px_rgb(12_17_28_/_0.05)] transition-colors hover:bg-white hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 sm:inline-flex"
            onClick={() => {
              window.dispatchEvent(
                new KeyboardEvent("keydown", {
                  key: "k",
                  ctrlKey: true,
                  bubbles: true,
                }),
              );
            }}
            title="Command palette (Ctrl+K)"
          >
            <Search className="h-3 w-3" />
            <span>Command</span>
            <span className="rounded-md bg-frost px-1 font-mono text-[0.65rem] text-foreground/80 [background:var(--frost)]">Ctrl K</span>
          </button>
        </>
      ) : (
        <div className="mr-2 inline-flex h-6 items-center gap-2 rounded-md border border-border/70 bg-white/85 px-2 text-[0.7rem] font-medium text-foreground">
          <span className="h-1.5 w-1.5 rounded-full bg-cyan-500 shadow-[0_0_0_3px_rgb(43_199_255_/_0.18)]" />
          first boot
        </div>
      )}
      <div className="flex h-full">
        <button
          type="button"
          className="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50"
          onClick={() => void ipc.windowMinimize()}
          aria-label="Minimize"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          className="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring/50"
          onClick={() => void ipc.windowToggleMaximize()}
          aria-label="Maximize"
        >
          <Square className="h-3 w-3" />
        </button>
        <button
          type="button"
          className="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-red-600 hover:text-white focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-red-300/70"
          onClick={() => void ipc.windowClose()}
          aria-label="Close"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}
