import assert from "node:assert/strict";
import { ChangeSet, EditorState } from "@codemirror/state";
import { equalFormats, mapFormats, normalizeFormats } from "../src/lib/note/formatting.ts";
import { createSaveQueue } from "../src/lib/note/save-queue.ts";

// Compare the ordered mapper with the previous normalize-and-sort behavior.
let seed = 42;
const random = (n) => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed % n; };
{
  const runs = [{ from: 1, to: 4, bold: true }];
  assert.equal(mapFormats(runs, ChangeSet.of({ from: 8, insert: "😀" }, 10), 12), runs);
  assert.equal(mapFormats(runs, ChangeSet.of({ from: 2, to: 3, insert: "a" }, 10), 10), runs);
  assert.notEqual(mapFormats(runs, ChangeSet.of({ from: 0, insert: "😀" }, 10), 12), runs);
}
for (let trial = 0; trial < 3000; trial++) {
  const length = 20 + random(300);
  const runs = normalizeFormats(Array.from({ length: 30 }, () => {
    const from = random(length);
    return { from, to: from + random(length - from + 1), bold: true, size: random(2) ? 12 : 18 };
  }), length);
  const from = random(length), to = from + random(length - from + 1);
  const changes = ChangeSet.of({ from, to, insert: "한😀글".repeat(random(5)) }, length);
  const expected = normalizeFormats(runs.map((f) => ({ ...f, from: changes.mapPos(f.from, -1), to: changes.mapPos(f.to, 1) })), changes.newLength);
  const result = mapFormats(runs, changes, changes.newLength);
  assert.deepEqual(result, expected);
  assert.ok(equalFormats(result, structuredClone(result)));
  if (result.length) assert.ok(!equalFormats(result, result.map((f, i) => i ? f : { ...f, italic: true })));
}

// Lazy snapshots must protect the latest editor state across a failed in-flight save.
{
  let unblock;
  const gate = new Promise((resolve) => { unblock = resolve; });
  const saved = [];
  let attempts = 0, reads = 0;
  const queue = createSaveQueue(async (body) => {
    if (++attempts === 1) { await gate; throw new Error("disk blocked"); }
    saved.push(body);
  }, () => {});
  let state = EditorState.create({ doc: "older" });
  const older = state;
  queue.queueLazy(() => { reads++; return older.doc.toString(); });
  const first = queue.flush();
  await Promise.resolve();
  state = state.update({ changes: { from: 0, to: state.doc.length, insert: "한😀글 newer" } }).state;
  const latest = state;
  queue.queueLazy(() => { reads++; return latest.doc.toString(); });
  unblock();
  await first;
  assert.ok(queue.dirty);
  await queue.flush();
  assert.deepEqual(saved, ["한😀글 newer"]);
  assert.equal(reads, 2);
  assert.equal(queue.dirty, false);
}
console.log("Performance regressions passed: 3,000 differential format mappings and lazy immutable editor saves.");
