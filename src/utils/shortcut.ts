const isMac = () => navigator.userAgent.includes("Mac");

const SHIFT = isMac() ? "⇧" : "Shift+";
const CMD = isMac() ? "⌘" : "Ctrl+";
const CTRL = "Ctrl+";
const ALT = isMac() ? "⌥" : "Alt+";
const ESC = "Esc";

export function formatShortcut(key: string): string {
  const lower = key.trim().toLowerCase();

  // Handle comma-separated combos: take the first one appropriate for the current OS
  const parts = lower.split(",").map((p) => p.trim());

  // Prefer command on Mac, ctrl otherwise
  let chosen = parts[0];
  for (const part of parts) {
    if (isMac() && part.startsWith("command")) {
      chosen = part;
      break;
    }
    if (!isMac() && part.startsWith("ctrl")) {
      chosen = part;
      break;
    }
  }

  return formatCombo(chosen.trim());
}

function formatCombo(combo: string): string {
  const keys = combo.split("+").map((k) => k.trim());

  return keys
    .map((k) => {
      switch (k) {
        case "command":
        case "cmd":
          return CMD.replace("+", "");
        case "ctrl":
        case "control":
          return CTRL.replace("+", "");
        case "shift":
          return SHIFT.replace("+", "");
        case "alt":
        case "option":
          return ALT.replace("+", "");
        case "escape":
        case "esc":
          return ESC;
        case ",":
          return ",";
        default:
          return k.length === 1 ? k.toUpperCase() : k;
      }
    })
    .join("");
}

/** Returns the shortcut label for the "New transaction" action. */
export function newTransactionShortcut(): string {
  return formatShortcut("command+n, ctrl+n");
}

/** Returns the shortcut label for the "Current month" action. */
export function currentMonthShortcut(): string {
  return formatShortcut("command+t, ctrl+t");
}

/** Returns the shortcut label for the spotlight / command palette action. */
export function spotlightShortcut(): string {
  return formatShortcut("command+k, ctrl+k");
}

/** Returns the shortcut label for a navigation item by its index (1-based). */
export function navShortcut(index: number): string {
  return formatShortcut(`command+${index}, ctrl+${index}`);
}


