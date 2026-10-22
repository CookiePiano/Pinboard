import assert from "node:assert/strict";
import { createSaveQueue } from "../src/lib/note/save-queue.ts";

const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};

// Superseded immutable snapshots must not be materialized before a save is needed.
{
  let reads = 0;
  const saved = [];
  const queue = createSaveQueue(async value => { saved.push(value); }, () => {});
  queue.queueLazy(() => { throw new Error("obsolete snapshot was materialized"); });
  queue.queueLazy(() => { reads++; return "한😀글 latest"; });
  assert.equal(queue.dirty, true);
  assert.equal(reads, 0);
  await queue.flush();
  assert.equal(reads, 1);
  assert.deepEqual(saved, ["한😀글 latest"]);
  assert.equal(queue.dirty, false);
}

// A failed older write must not be retried after a newer queued write succeeds.
{
  const first = deferred();
  const requests = [];
  let error = null;
  const queue = createSaveQueue(async value => {
    requests.push(value);
    if (requests.length === 1) await first.promise;
  }, value => { error = value; });
  queue.queue("older");
  const older = queue.flush();
  await Promise.resolve();
  queue.queue("newer");
  const newer = queue.flush();
  assert.equal(queue.dirty, true);
  first.reject(new Error("disk unavailable"));
  await Promise.all([older, newer]);
  await queue.flush();
  assert.deepEqual(requests, ["older", "newer"]);
  assert.equal(queue.dirty, false);
  assert.equal(error, null);
}

// An edit that is still being debounced also supersedes a failed in-flight write.
{
  const first = deferred();
  const requests = [];
  const queue = createSaveQueue(async value => {
    requests.push(value);
    if (requests.length === 1) await first.promise;
  }, () => {});
  queue.queue("older");
  const saving = queue.flush();
  await Promise.resolve();
  queue.queue("newer");
  first.reject(new Error("disk unavailable"));
  await saving;
  assert.equal(queue.dirty, true);
  await queue.flush();
  assert.deepEqual(requests, ["older", "newer"]);
}

// Retain the latest failed edit for a later blur/export/manual retry.
{
  let attempts = 0;
  let error;
  const saved = [];
  const queue = createSaveQueue(async value => {
    if (++attempts === 1) throw new Error("disk unavailable");
    saved.push(value);
  }, value => { error = value; });
  const patch = { body: "한😀", formats: [{ from: 0, to: 3, bold: true }] };
  queue.queue(patch);
  await queue.flush();
  assert.equal(queue.dirty, true);
  assert.match(error, /disk unavailable/);
  await queue.flush();
  assert.deepEqual(saved, [patch]);
  assert.equal(error, null);
  assert.equal(queue.dirty, false);
}

// Coalesce edits until flush, serialize IPC, and await in-flight writes on repeated flush.
{
  const first = deferred();
  const requests = [];
  const queue = createSaveQueue(async value => {
    requests.push(value);
    if (requests.length === 1) await first.promise;
  }, () => {});
  queue.queue("discarded draft");
  queue.queue("first saved");
  const saving = queue.flush();
  assert.equal(queue.flush(), saving);
  await Promise.resolve();
  queue.queue("last saved");
  const last = queue.flush();
  assert.deepEqual(requests, ["first saved"]);
  first.resolve();
  await last;
  assert.deepEqual(requests, ["first saved", "last saved"]);
  assert.equal(queue.dirty, false);
}
console.log("Save queue checks passed: stale failure, pending edit, retry, Unicode formatting, coalescing and serialized flush.");
