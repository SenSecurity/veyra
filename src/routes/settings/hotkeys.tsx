import { HotkeyInput } from "@/components/hotkey-input";
import { useSettings } from "@/hooks/use-settings";
import { SettingsPanel } from "./general";

export function SettingsHotkeysRoute() {
  const { settings, loading, update } = useSettings();
  if (loading || !settings) return <SettingsPanel title="Hotkeys" muted="Loading settings." />;
  return (
    <SettingsPanel title="Hotkeys" muted="Global shortcuts. Restart Veyra after changing them.">
      <label className="grid gap-2 text-sm">
        <span className="font-medium">Dictation hotkey</span>
        <HotkeyInput value={settings.hotkey} onChange={(hotkey) => void update({ hotkey })} />
      </label>
      <label className="grid gap-2 text-sm">
        <span className="font-medium">Email draft hotkey</span>
        <HotkeyInput
          value={settings.commandHotkey}
          onChange={(commandHotkey) => void update({ commandHotkey })}
        />
      </label>
    </SettingsPanel>
  );
}

