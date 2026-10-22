import { api } from "./ipc";

/** Share Windows pointer-driven snapping and native GTK dragging across windows. */
export function dragWindow(node: HTMLElement, report: (error: unknown) => void = (error) => console.error("Window drag failed", error)) {
  let pointer: number | null = null;
  let ready = Promise.resolve(false);
  let frame = 0;
  function finish(id: number, notify: boolean) {
    if (pointer !== id) return;
    pointer = null;
    cancelAnimationFrame(frame); frame = 0;
    if (notify) void ready.then(() => api.dragNote("end")).catch(report);
    if (node.hasPointerCapture(id)) node.releasePointerCapture(id);
  }
  function start(event: PointerEvent) {
    if (event.button !== 0 || (event.target as Element).closest("button, input, select")) return;
    event.preventDefault();
    pointer = event.pointerId;
    node.setPointerCapture(pointer);
    ready = api.dragNote("start");
    const id = event.pointerId;
    void ready.then(manual => {
      // GTK can consume pointerup during its native move loop. Retaining DOM
      // capture would redirect later editor/button clicks back to the header.
      if (!manual) finish(id, false);
    }).catch(error => { finish(id, false); report(error); });
  }
  function move(event: PointerEvent) {
    if (event.pointerId !== pointer || frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      void ready.then(manual => { if (manual && pointer === event.pointerId) return api.dragNote("move"); }).catch(report);
    });
  }
  function end(event: PointerEvent) {
    finish(event.pointerId, true);
  }
  node.addEventListener("pointerdown", start);
  node.addEventListener("pointermove", move);
  for (const name of ["pointerup", "pointercancel", "lostpointercapture"] as const) node.addEventListener(name, end);
  return { destroy() {
    if (pointer !== null) finish(pointer, true);
    node.removeEventListener("pointerdown", start);
    node.removeEventListener("pointermove", move);
    for (const name of ["pointerup", "pointercancel", "lostpointercapture"] as const) node.removeEventListener(name, end);
  } };
}
