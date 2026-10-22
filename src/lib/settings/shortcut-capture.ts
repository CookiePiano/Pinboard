type ShortcutKeyEvent = Pick<KeyboardEvent,
  "key" | "code" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "repeat" | "isComposing">;

type Capture = { kind: "ignore" | "cancel" | "invalid" } | { kind: "shortcut"; value: string };

// Accessibility and remote input can provide a key but no physical code.
// Only infer unambiguous keys; punctuation and composed text need a real code.
function fallbackCode(key: string): string | null {
  if (/^[a-z]$/i.test(key)) return `Key${key.toUpperCase()}`;
  if (/^[0-9]$/.test(key)) return `Digit${key}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) return key;
  if (["Enter", "Tab", "Backspace", "Delete", "Insert", "Home", "End", "PageUp", "PageDown",
    "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(key)) return key;
  return key === " " ? "Space" : null;
}

export function captureShortcut(event: ShortcutKeyEvent): Capture {
  if (event.key === "Escape") return { kind: "cancel" };
  if (event.repeat || /^(Control|Alt|Shift|Meta)/.test(event.code)
    || ["Control", "Alt", "AltGraph", "Shift", "Meta", "OS"].includes(event.key)) return { kind: "ignore" };
  if (!(event.ctrlKey || event.altKey || event.metaKey) || event.isComposing) return { kind: "invalid" };
  const code = event.code && event.code !== "Unidentified" ? event.code : fallbackCode(event.key);
  if (!code) return { kind: "invalid" };
  const keys = [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Super", code];
  return { kind: "shortcut", value: keys.filter(Boolean).join("+") };
}
