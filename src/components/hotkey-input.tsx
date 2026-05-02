import { Input } from "@/components/ui/input";

export function HotkeyInput({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <Input
      value={value}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => {
        if (!e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) return;
        e.preventDefault();
        const parts = [
          e.ctrlKey ? "Ctrl" : "",
          e.metaKey ? "Meta" : "",
          e.altKey ? "Alt" : "",
          e.shiftKey ? "Shift" : "",
          e.key.length === 1 ? e.key.toUpperCase() : e.key,
        ].filter(Boolean);
        onChange(parts.join("+"));
      }}
      placeholder="Ctrl+Shift+Space"
    />
  );
}

