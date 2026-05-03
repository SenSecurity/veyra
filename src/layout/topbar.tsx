import { RecordingPill } from "@/components/recording-pill";

// App title left, command-palette trigger + recording pill right.
// The command-palette button is a stub here; full cmdk dialog in T12.
export function TopBar() {
  return (
    <header className="flex h-12 shrink-0 items-center gap-3 border-b border-border bg-background/85 px-4 backdrop-blur">
      <h1 className="text-sm font-semibold tracking-tight">Veyra</h1>
      <div className="flex-1" />
      <button
        type="button"
        className="rounded-lg border border-border bg-card px-2.5 py-1.5 text-xs text-muted-foreground shadow-sm hover:bg-muted hover:text-foreground"
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
