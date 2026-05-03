import { RecordingPill } from "@/components/recording-pill";
import { Search } from "lucide-react";

// App title left, command-palette trigger + recording pill right.
// The command-palette button is a stub here; full cmdk dialog in T12.
export function TopBar() {
  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-border/80 bg-background/82 px-4 shadow-[inset_0_-1px_0_rgb(255_255_255_/_0.72)] backdrop-blur-xl">
      <div className="hidden min-w-0 sm:block">
        <p className="text-xs font-medium text-muted-foreground">Private, fast, local</p>
      </div>
      <div className="flex-1" />
      <button
        type="button"
        className="inline-flex h-8 min-w-36 items-center gap-2 rounded-lg border border-border bg-white/76 px-2.5 text-xs text-muted-foreground shadow-sm transition-colors hover:bg-white hover:text-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/35"
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
        <Search className="h-3.5 w-3.5" />
        <span className="flex-1 text-left">Command</span>
        <span className="rounded-md bg-muted px-1.5 py-0.5 font-mono text-[0.68rem] text-muted-foreground">
          Ctrl K
        </span>
      </button>
      <RecordingPill />
    </header>
  );
}
