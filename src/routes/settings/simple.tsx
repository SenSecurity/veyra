import { Button } from "@/components/ui/button";
import { useNavigate } from "@tanstack/react-router";
import { SettingsPanel } from "./general";

export function SettingsSimpleRoute({
  title,
  text,
}: {
  title: string;
  text: string;
}) {
  return (
    <SettingsPanel title={title} muted={text}>
      <div className="rounded-md border border-border bg-card p-4 text-sm text-muted-foreground">
        This area is reserved for the next settings schema expansion. Current Phase 3 UI keeps v1 settings writable.
      </div>
    </SettingsPanel>
  );
}

export function SettingsAboutRoute() {
  const navigate = useNavigate();
  return (
    <SettingsPanel title="About" muted="Veyra 0.1.0">
      <div className="rounded-md border border-border bg-card p-4 text-sm text-muted-foreground">
        Local-first dictation with whisper.cpp and Groq fallback.
      </div>
      <Button type="button" variant="outline" onClick={() => void navigate({ to: "/wizard" })}>
        Run setup wizard
      </Button>
    </SettingsPanel>
  );
}
