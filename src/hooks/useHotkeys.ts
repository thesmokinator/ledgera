import { useEffect, useRef } from "react";
import hotkeys from "hotkeys-js";

type Shortcut = {
  keys: string;
  action: (event: KeyboardEvent, handler: { key: string }) => void;
  disabled?: boolean;
};

export function useHotkeys(shortcuts: Shortcut[]) {
  const shortcutsRef = useRef(shortcuts);
  shortcutsRef.current = shortcuts;

  useEffect(() => {
    // Allow hotkeys everywhere, including inside form fields
    hotkeys.filter = () => true;

    const boundKeys: string[] = [];
    for (const shortcut of shortcutsRef.current) {
      if (!shortcut.disabled) {
        boundKeys.push(shortcut.keys);
        hotkeys(shortcut.keys, (event, handler) => {
          event.preventDefault();
          shortcut.action(event, handler);
        });
      }
    }

    return () => {
      for (const keys of boundKeys) {
        hotkeys.unbind(keys);
      }
    };
  }, [shortcuts]);
}
