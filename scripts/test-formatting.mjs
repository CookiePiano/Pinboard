import assert from "node:assert/strict";
import { ChangeSet } from "@codemirror/state";
import { formatRange, fullyFormatted, mapFormats, normalizeFormats } from "../src/lib/note/formatting.ts";

const options = { step: 1, color: "#ffe066", baseSize: () => 12 };
const unordered = [
  { from: 4, to: 6, bold: true, italic: false },
  { from: -2, to: 4, bold: true },
  { from: 6, to: 8, bold: true, highlight: "#ffe066" },
  { from: 8, to: 20, bold: true, highlight: "#ffe066" },
];
const original = structuredClone(unordered);
assert.deepEqual(normalizeFormats(unordered, 10), [
  { from: 0, to: 6, bold: true },
  { from: 6, to: 10, bold: true, highlight: "#ffe066" },
]);
assert.deepEqual(unordered, original, "normalization must not mutate stored formats");
assert.deepEqual(normalizeFormats([{ from: 0, to: 3, bold: false }, { from: 3, to: 3, italic: true }], 3), []);
assert.deepEqual(formatRange([{ from: 0, to: 2, bold: true }, { from: 4, to: 6, bold: true }], 2, 4, "bold", options, 6), [{ from: 0, to: 6, bold: true }]);
let runs = formatRange([], 2, 8, "bold", options, 10);
assert.deepEqual(runs, [{ from: 2, to: 8, bold: true }]);
runs = formatRange(runs, 4, 6, "italic", options, 10);
assert.deepEqual(runs, [{ from: 2, to: 4, bold: true }, { from: 4, to: 6, bold: true, italic: true }, { from: 6, to: 8, bold: true }]);
runs = formatRange(runs, 2, 8, "bold", options, 10);
assert.deepEqual(runs, [{ from: 4, to: 6, italic: true }]);
runs = formatRange(runs, 0, 10, "highlight", options, 10);
assert.equal(fullyFormatted(runs, 0, 10, "highlight", "#ffe066"), true);
runs = formatRange(runs, 0, 10, "highlight", options, 10);
assert.deepEqual(runs, [{ from: 4, to: 6, italic: true }]);
const sized = formatRange([{ from: 0, to: 4, size: 20 }], 0, 8, "larger", options, 8);
assert.deepEqual(sized, [{ from: 0, to: 4, size: 21 }, { from: 4, to: 8, size: 13 }]);
assert.deepEqual(formatRange([], 0, 6, "larger", { ...options, baseBreaks: [3], baseSize: (pos) => pos < 3 ? 13 : 12 }, 6), [{ from: 0, to: 3, size: 14 }, { from: 3, to: 6, size: 13 }]);
assert.deepEqual(formatRange([{ from: 0, to: 4, size: 6 }], 0, 4, "smaller", options, 4), [{ from: 0, to: 4, size: 6 }]);
const changes = ChangeSet.of({ from: 0, insert: "한😀" }, 10);
assert.deepEqual(mapFormats(runs, changes, 13), [{ from: 7, to: 9, italic: true }]);
const deletion = ChangeSet.of({ from: 3, to: 7 }, 10);
assert.deepEqual(mapFormats(runs, deletion, 6), []);
console.log("Formatting checks passed: mixed styles, toggles, 1pt increments, bounds, Unicode offsets, edits and deletions.");
