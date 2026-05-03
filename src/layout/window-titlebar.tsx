import { AudioLines, Minus, Search, Square, X } from "lucide-react";
import { RecordingPill } from "@/components/recording-pill";
import { ipc } from "@/lib/tauri";

export function WindowTitleBar() {
  return (
    <header
      className="flex h-9 shrink-0 items-center border-b border-zinc-700/70 bg-[linear-gradient(180deg,#27303a,#151a20)] text-zinc-100"
    >
      <div data-tauri-drag-region className="flex min-w-0 flex-1 items-center gap-2 px-3">
        <span className="flex h-5 w-5 items-center justify-center rounded-md bg-black/45 ring-1 ring-white/10">
          <AudioLines className="h-3.5 w-3.5 text-sky-300" />
        </span>
        <span className="text-xs font-medium">Veyra</span>
      </div>
      <button
        type="button"
        className="mr-2 hidden h-6 items-center gap-1.5 rounded-md border border-white/10 bg-white/8 px-2 text-[0.7rem] text-zinc-300 hover:bg-white/12 hover:text-white sm:inline-flex"
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
        <span className="rounded bg-white/10 px-1 font-mono text-[0.65rem]">Ctrl K</span>
      </button>
      <div className="mr-2 hidden sm:block">
        <RecordingPill />
      </div>
      <div className="flex h-full">
        <button
          type="button"
          className="flex w-11 items-center justify-center text-zinc-300 hover:bg-zinc-800 hover:text-white"
          onClick={() => void ipc.windowMinimize()}
          aria-label="Minimize"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          className="flex w-11 items-center justify-center text-zinc-300 hover:bg-zinc-800 hover:text-white"
          onClick={() => void ipc.windowToggleMaximize()}
          aria-label="Maximize"
        >
          <Square className="h-3 w-3" />
        </button>
        <button
          type="button"
          className="flex w-11 items-center justify-center text-zinc-300 hover:bg-red-600 hover:text-white"
          onClick={() => void ipc.windowClose()}
          aria-label="Close"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </header>
  );
}
