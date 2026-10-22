import { writable } from "svelte/store";
import { api, type NoteBackground } from "../ipc";
import { createSaveQueue } from "../note/save-queue";

// Survive panel switches so failed image edits still participate in archive/exit flushes.
const queues = new Map<string, ReturnType<typeof createSaveQueue<NoteBackground>>>();
const errors = new Map<string, string>();
export const designSaveError = writable<string | null>(null);

export function queueDesignBackground(id: string, background: NoteBackground) {
  let queue = queues.get(id);
  if (!queue) {
    queue = createSaveQueue<NoteBackground>(
      (value) => api.updateNote(id, { background: value }),
      (error) => {
        if (error) errors.set(id, error); else errors.delete(id);
        designSaveError.set([...errors.values()].join("\n") || null);
      },
    );
    queues.set(id, queue);
  }
  queue.queue({ ...background });
}

export function designDirty(id: string) { return queues.get(id)?.dirty ?? false; }

export async function flushDesignSaves(): Promise<string | null> {
  do {
    await Promise.all([...queues.values()].map((queue) => queue.flush()));
    for (const [id, queue] of queues) if (!queue.dirty) queues.delete(id);
  } while (queues.size && !errors.size);
  return [...errors.values()].join("\n") || null;
}
