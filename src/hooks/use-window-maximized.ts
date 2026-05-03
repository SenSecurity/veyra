import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Tracks whether the current Tauri window is maximized.
 *
 * Used by `src/app.tsx` to toggle a `data-maximized` attribute on the
 * outermost `.veyra-window-shell` so rounded corners + drop shadow are
 * disabled while the window fills the work area, then restored when
 * the user un-maximizes.
 *
 * Subscribes to `tauri://resize` (fires for any resize, including
 * snap / maximize / restore). Re-queries `isMaximized()` on every
 * event so we stay in sync with whatever the OS decided.
 */
export function useWindowMaximized(): boolean {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let active = true;

    const refresh = async () => {
      try {
        const result = await getCurrentWindow().isMaximized();
        if (active) setMaximized(result);
      } catch {
        if (active) setMaximized(false);
      }
    };

    void refresh();

    let unlisten: (() => void) | null = null;
    void getCurrentWindow()
      .onResized(() => {
        void refresh();
      })
      .then((fn) => {
        if (active) unlisten = fn;
        else fn();
      })
      .catch(() => {});

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  return maximized;
}
