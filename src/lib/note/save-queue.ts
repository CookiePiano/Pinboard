/** Serializes saves and retries only the latest edit after a failure.
 * Treat queued values as immutable. The caller owns debounce timing and error display.
 */
export function createSaveQueue<T>(save: (value: T) => Promise<unknown>, report: (error: string | null) => void) {
  let revision = 0;
  let pending: { revision: number; read: () => T } | null = null;
  let active = 0;
  let writes = Promise.resolve();

  return {
    queue(value: T) { pending = { revision: ++revision, read: () => value }; },
    /** Capture an immutable state now and materialize its payload only when saved. */
    queueLazy(read: () => T) { pending = { revision: ++revision, read }; },
    get dirty() { return pending !== null || active > 0; },
    flush(): Promise<void> {
      if (pending === null) return writes;
      const patch = pending;
      pending = null;
      active++;
      writes = writes.then(async () => {
        try { await save(patch.read()); report(null); }
        catch (error) {
          // A newer edit may already be queued or in flight. Never resurrect this one.
          if (patch.revision === revision && pending === null) pending = patch;
          report(String(error));
        } finally { active--; }
      });
      return writes;
    },
  };
}
