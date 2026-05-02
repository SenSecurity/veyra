import { RecordingPill } from "@/components/recording-pill";

// App title left, command-palette trigger + recording pill right.
// The command-palette button is a stub here; full cmdk dialog in T12.
export function TopBar() {
  return (
    <header className="h-12 shrink-0 border-b border-border bg-background/95 flex items-center px-4 gap-3">
      <h1 className="text-sm font-semibold tracking-tight">Veyra</h1>
      <div className="flex-1" />
      <button
        type="button"
        className="rounded-md border border-border bg-card px-2 py-1 text-xs text-muted-foreground hover:bg-muted hover:text-foreground"
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
        <span className="font-mono">Ctrl+K</span>
      </button>
      <RecordingPill />
    </header>
  );
}
