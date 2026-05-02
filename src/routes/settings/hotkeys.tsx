import { HotkeyInput } from "@/components/hotkey-input";
import { useSettings } from "@/hooks/use-settings";
import { SettingsPanel } from "./general";

export function SettingsHotkeysRoute() {
  const { settings, loading, update } = useSettings();
  if (loading || !settings) return <SettingsPanel title="Hotkeys" muted="Loading settings." />;
  return (
    <SettingsPanel title="Hotkeys" muted="Global dictation shortcut.">
      <label className="grid gap-2 text-sm">
        <span className="font-medium">Dictation hotkey</span>
        <HotkeyInput value={settings.hotkey} onChange={(hotkey) => void update({ hotkey })} />
      </label>
    </SettingsPanel>
  );
}

