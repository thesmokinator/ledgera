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

    for (const { keys, disabled } of shortcutsRef.current) {
      if (!disabled) {
        hotkeys(keys, (event, handler) => {
          const current = shortcutsRef.current.find((s) => {
            const comboKeys = s.keys.split(",").map((k) => k.trim());
            return !s.disabled && comboKeys.includes(handler.key);
          });
          if (current) {
            event.preventDefault();
            current.action(event, handler);
          }
        });
      }
    }

    return () => {
      hotkeys.unbind();
    };
  }, [shortcuts]);
}
