import { getCurrentWindow } from "@tauri-apps/api/window";
import { AudioLines, Minus, Square, X } from "lucide-react";

const appWindow = getCurrentWindow();

export function WindowTitleBar() {
  return (
    <header
      data-tauri-drag-region
      className="flex h-8 shrink-0 items-center border-b border-border bg-zinc-950 text-zinc-100"
    >
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-2 px-3">
        <AudioLines className="h-4 w-4 text-zinc-300" />
        <span className="text-xs font-medium">Veyra</span>
      </div>
      <div className="flex h-full">
        <button
          type="button"
          className="flex w-11 items-center justify-center text-zinc-300 hover:bg-zinc-800 hover:text-white"
          onClick={() => void appWindow.minimize()}
          aria-label="Minimize"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          className="flex w-11 items-center justify-center text-zinc-300 hover:bg-zinc-800 hover:text-white"
          onClick={() => void appWindow.toggleMaximize()}
          aria-label="Maximize"
        >
          <Square className="h-3 w-3" />
        </button>
        <button
          type="button"
          className="flex w-11 items-center justify-center text-zinc-300 hover:bg-red-600 hover:text-white"
          onClick={() => void appWindow.close()}
          aria-label="Close"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}
