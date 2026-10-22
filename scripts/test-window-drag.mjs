import assert from "node:assert/strict";
import { createServer } from "vite";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Exercise the shared Svelte action and its native IPC contract. Real GTK
// pointer ownership is covered separately by test-linux-tray.mjs.
const server=await createServer({configFile:false,cacheDir:join(tmpdir(),`pinboard-drag-${process.pid}`),
  optimizeDeps:{noDiscovery:true,include:[]},server:{middlewareMode:true,hmr:false,ws:false},appType:"custom"});
const originals={window:globalThis.window,raf:globalThis.requestAnimationFrame,caf:globalThis.cancelAnimationFrame};
let frames=new Map(),nextFrame=0,manual=true,calls=[],errors=[],rejectStart=false;
globalThis.requestAnimationFrame=callback=>{frames.set(++nextFrame,callback);return nextFrame;};
globalThis.cancelAnimationFrame=id=>frames.delete(id);
globalThis.window={__TAURI_INTERNALS__:{invoke:async(command,{phase})=>{
  assert.equal(command,"drag_note");calls.push(phase);
  if(rejectStart&&phase==="start")throw Error("native start failed");
  return manual;
}}};
class Header extends EventTarget {
  captured=new Set();blocked=false;
  closest(){return this.blocked?this:null;}
  setPointerCapture(id){this.captured.add(id);}
  hasPointerCapture(id){return this.captured.has(id);}
  releasePointerCapture(id){this.captured.delete(id);this.send("lostpointercapture",id);}
  send(type,id=1,button=0){const event=new Event(type,{cancelable:true});Object.assign(event,{pointerId:id,button});this.dispatchEvent(event);}
}
const settle=()=>new Promise(resolve=>setImmediate(resolve));
async function animationFrame(){const current=[...frames.values()];frames.clear();for(const callback of current)callback();await settle();}
try {
  const {dragWindow}=await server.ssrLoadModule("/src/lib/window-drag.ts");
  const node=new Header(),action=dragWindow(node,error=>errors.push(String(error)));
  node.send("pointerdown");await settle();assert.equal(node.hasPointerCapture(1),true);
  node.send("pointermove");node.send("pointermove");await animationFrame();
  assert.deepEqual(calls,["start","move"],"Windows coalesces pointer moves into one native update");
  node.send("pointermove");node.send("pointerup");await animationFrame();
  assert.deepEqual(calls,["start","move","end"]);assert.equal(node.captured.size,0);
  calls=[];manual=false;
  node.send("pointerdown");node.send("pointermove");await settle();await animationFrame();
  assert.equal(node.captured.size,0,"GTK owns the drag, so DOM capture must end even without pointerup");
  node.send("pointermove");node.send("pointerup");await animationFrame();assert.deepEqual(calls,["start"]);
  calls=[];manual=true;rejectStart=true;
  node.send("pointerdown");await settle();assert.equal(node.captured.size,0);assert.equal(errors.length,1);
  rejectStart=false;calls=[];node.blocked=true;node.send("pointerdown");node.blocked=false;
  node.send("pointerdown",1,2);await settle();assert.deepEqual(calls,[],"Controls and secondary buttons do not start drags");
  node.send("pointerdown");await settle();action.destroy();await settle();
  assert.equal(node.captured.size,0);assert.deepEqual(calls,["start","end"]);
  node.send("pointerdown");await settle();assert.deepEqual(calls,["start","end"],"Destroyed action leaves no listeners");
  console.log("Window drag checks passed: Windows movement/coalescing, GTK ownership, errors, controls and cleanup.");
} finally {
  globalThis.window=originals.window;globalThis.requestAnimationFrame=originals.raf;globalThis.cancelAnimationFrame=originals.caf;
  await server.close();
}
