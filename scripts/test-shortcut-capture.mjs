import assert from "node:assert/strict";
import { captureShortcut } from "../src/lib/settings/shortcut-capture.ts";

const event = (patch) => ({ key: "n", code: "KeyN", ctrlKey: true, altKey: false,
  shiftKey: false, metaKey: false, repeat: false, isComposing: false, ...patch });
assert.deepEqual(captureShortcut(event({ altKey: true })), { kind: "shortcut", value: "Ctrl+Alt+KeyN" });
// Observed Windows SendInput sequence: all three events have an empty code.
for (const key of ["Control", "Alt"]) {
  assert.deepEqual(captureShortcut(event({ key, code: "", altKey: key === "Alt" })), { kind: "ignore" });
}
assert.deepEqual(captureShortcut(event({ key: "F8", code: "", altKey: true })), { kind: "shortcut", value: "Ctrl+Alt+F8" });
assert.deepEqual(captureShortcut(event({ key: "N", code: "", shiftKey: true })), { kind: "shortcut", value: "Ctrl+Shift+KeyN" });
assert.deepEqual(captureShortcut(event({ key: "4", code: "Unidentified" })), { kind: "shortcut", value: "Ctrl+Digit4" });
assert.deepEqual(captureShortcut(event({ key: "ArrowLeft", code: "" })), { kind: "shortcut", value: "Ctrl+ArrowLeft" });
assert.deepEqual(captureShortcut(event({ key: "한", code: "KeyG" })), { kind: "shortcut", value: "Ctrl+KeyG" });
for (const patch of [{ key: "한", code: "" }, { key: "+", code: "" }, { key: "F25", code: "" },
  { isComposing: true }, { ctrlKey: false, shiftKey: true }]) {
  assert.deepEqual(captureShortcut(event(patch)), { kind: "invalid" });
}
assert.deepEqual(captureShortcut(event({ repeat: true })), { kind: "ignore" });
assert.deepEqual(captureShortcut(event({ key: "Escape" })), { kind: "cancel" });
console.log("Shortcut capture checks passed (physical keys, Windows empty-code input, modifiers, IME, invalid input).");
