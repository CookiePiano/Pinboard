// Run under xvfb-run + dbus-run-session with PINBOARD_TEST_PRIVATE_BUS=1.
// The fixture host drives the application's real D-Bus tray menu, not mocked IPC.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, execFileSync } from "node:child_process";

assert.equal(process.env.PINBOARD_TEST_PRIVATE_BUS, "1", "Use a private dbus-run-session test bus");
const binary = path.resolve(process.argv[2]);
const output = path.resolve(process.argv[3]);
const reproduce = process.argv[4] === "--reproduce-before";
fs.mkdirSync(output, { recursive: true });
const data = fs.mkdtempSync(path.join(output, "data-"));
const port = Number(process.env.PINBOARD_TEST_PORT || 4496);
const log = fs.openSync(path.join(output, "runtime.log"), "w");
const watcherLog = path.join(output, "watcher.log");
const watcherFd = fs.openSync(watcherLog, "w");
const source = path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures/tray-watcher.c");
const flags = execFileSync("pkg-config", ["--cflags", "--libs", "gio-2.0"], {encoding:"utf8"}).trim().split(/\s+/);
const watcherBinary = path.join(output, "watcher");
execFileSync("gcc", [source, "-o", watcherBinary, ...flags]);
const watcher = spawn(watcherBinary, [], {stdio:["ignore",watcherFd,watcherFd]});
const wm = spawn("openbox", [], {stdio:["ignore",log,log]});
// Use an external URI handler in this process environment only. Exercise the
// actual opener/xdg-open chain without opening user browser tabs or networking.
const openedUrls = path.join(output,"opened-urls.jsonl");
const browserHandler = path.join(output,"record-uri.cjs");
fs.writeFileSync(browserHandler,`#!${process.execPath}\nrequire('node:fs').appendFileSync(${JSON.stringify(openedUrls)},JSON.stringify(process.argv.slice(2))+'\\n');\n`,{mode:0o700});
const xdgConfig=path.join(output,"xdg-config"),xdgData=path.join(output,"xdg-data");
fs.mkdirSync(xdgConfig,{recursive:true});fs.mkdirSync(path.join(xdgData,"applications"),{recursive:true});
fs.writeFileSync(path.join(xdgData,"applications/pinboard-verification.desktop"),`[Desktop Entry]\nType=Application\nName=Pinboard test URI handler\nExec=${browserHandler} %u\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\nNoDisplay=true\n`);
fs.writeFileSync(path.join(xdgConfig,"mimeapps.list"),"[Default Applications]\nx-scheme-handler/http=pinboard-verification.desktop\nx-scheme-handler/https=pinboard-verification.desktop\n");
const driverEnv={...process.env,PINBOARD_DATA_DIR:data,GDK_BACKEND:"x11",
  PATH:process.env.PATH.split(":").filter(p=>!p.startsWith("/mnt/")).join(":"),
  BROWSER:browserHandler,XDG_CURRENT_DESKTOP:"X-Generic",DESKTOP_SESSION:"",
  XDG_CONFIG_HOME:xdgConfig,XDG_DATA_HOME:xdgData};
const driver = spawn("tauri-driver", ["--port", String(port), "--native-port", String(port+1)], {
  stdio:["ignore",log,log], env:driverEnv,
});
let session, blocked, original, target, menu, appPid;
const checks = [];
const sleep = ms => new Promise(resolve=>setTimeout(resolve,ms));
async function wait(check, label) {
  const deadline=Date.now()+20000;
  while(Date.now()<deadline) {try {const result=await check(); if(result)return result;} catch {} await sleep(100);}
  throw Error(`Timed out: ${label}`);
}
async function request(route, body, method=body===undefined?"GET":"POST") {
  const response=await fetch(`http://127.0.0.1:${port}${route}`,{method,headers:{"content-type":"application/json"},body:body===undefined?undefined:JSON.stringify(body),signal:AbortSignal.timeout(25000)});
  const value=await response.json();
  if(!response.ok||value.value?.error)throw Error(JSON.stringify(value));
  return value.value;
}
const api=(route,body,method)=>request(`/session/${session}${route}`,body,method);
const execute=(script,args=[])=>api("/execute/sync",{script,args});
const xdo=(...args)=>execFileSync("xdotool",args,{encoding:"utf8",timeout:5000});
async function click(selector) {
  const element=(await api("/element",{using:"css selector",value:selector}))["element-6066-11e4-a52e-4f735466cecf"];
  await api(`/element/${element}/click`,{});
}
async function select(selector) {
  return wait(async()=>{
    for(const handle of await api("/window/handles")) {
      await api("/window",{handle});
      if(await execute("return !!document.querySelector(arguments[0])",[selector]))return handle;
    }
  },selector);
}
async function selectNote(id) {
  return wait(async()=>{
    for(const handle of await api("/window/handles")) {
      await api("/window",{handle});
      if(await execute("return window.__TAURI_INTERNALS__?.metadata.currentWindow.label === arguments[0] && !!document.querySelector('.cm-content')",[`note-${id}`]))return handle;
    }
  },`editor ${id}`);
}
const notes=()=>fs.readdirSync(path.join(data,"notes")).filter(n=>n.endsWith(".json")).map(n=>JSON.parse(fs.readFileSync(path.join(data,"notes",n),"utf8")));
const bus=(destination,object,method,...args)=>execFileSync("gdbus",["call","--session","--timeout","8","--dest",destination,"--object-path",object,"--method",method,...args],{encoding:"utf8",timeout:10000});
function menuItem(label) {
  const layout=bus(target,menu,"com.canonical.dbusmenu.GetLayout","0","2","[]");
  fs.writeFileSync(path.join(output,"menu-layout.txt"),layout);
  const items=[...layout.matchAll(/\((\d+), \{[^}]*'label': <'([^']*)'>[^}]*\}/g)];
  const item=items.find(item=>item[2]===label);
  assert.ok(item,`Tray menu item: ${label}`);
  bus(target,menu,"com.canonical.dbusmenu.Event",item[1],"clicked","<0>","0");
}
function alive(){try{process.kill(appPid,0);return true;}catch{return false;}}
function visibleWindows(){try{return execFileSync("xdotool",["search","--onlyvisible","--pid",String(appPid)],{encoding:"utf8",stdio:["ignore","pipe","ignore"]}).trim().split("\n").filter(Boolean);}catch{return [];}}
async function capture(name){fs.writeFileSync(path.join(output,`${name}.png`),Buffer.from(await api("/screenshot"),"base64"));}
function unblock(){if(blocked&&fs.existsSync(blocked)&&fs.statSync(blocked).isDirectory())fs.rmdirSync(blocked);if(original&&fs.existsSync(original))fs.renameSync(original,blocked);original=undefined;blocked=undefined;}
try {
  await wait(()=>fs.readFileSync(watcherLog,"utf8").includes("WATCHER_READY"),"tray host ready");
  await wait(()=>request("/status"),"driver ready");
  session=(await request("/session",{capabilities:{alwaysMatch:{"tauri:options":{application:binary}}}})).sessionId;
  await select(".cm-content");
  const registration=await wait(()=>fs.readFileSync(watcherLog,"utf8").split("\n").find(line=>line.startsWith("RegisterStatusNotifierItem\t")),"tray registered");
  const [,sender,service]=registration.split("\t");
  target=service.startsWith("/")?sender:service;
  const object=service.startsWith("/")?service:"/StatusNotifierItem";
  menu=bus(target,object,"org.freedesktop.DBus.Properties.Get","org.kde.StatusNotifierItem","Menu").match(/objectpath '([^']+)'/)[1];
  appPid=Number(bus("org.freedesktop.DBus","/org/freedesktop/DBus","org.freedesktop.DBus.GetConnectionUnixProcessID",target).match(/uint32 (\d+)/)[1]);
  await wait(()=>visibleWindows().length>0,"welcome native window mapped");
  const initialWindows = new Set(visibleWindows());
  menuItem("새 메모");
  await wait(()=>notes().length===2,"tray creates note");
  const blank=notes().find(n=>!n.body);
  await selectNote(blank.id);
  const noteWindow=await wait(()=>visibleWindows().find(id=>!initialWindows.has(id)
    && execFileSync("xdotool",["getwindowname",id],{encoding:"utf8"}).trim()==="Pinboard"),"new native note window");
  checks.push("tray creates one note through the native menu callback");
  if(!reproduce) {
    const local=()=>JSON.parse(fs.readFileSync(path.join(data,"local-state.json"),"utf8"));
    await api("/window/rect",{x:100,y:130,width:420,height:360});
    await wait(()=>local().windows[blank.id]?.width===420 && local().windows[blank.id]?.height===360,"native resize persisted");
    const oldIds=new Set(notes().map(n=>n.id));
    const button=(await api("/element",{using:"css selector",value:'button[aria-label="새 메모"]'}))["element-6066-11e4-a52e-4f735466cecf"];
    await api(`/element/${button}/click`,{});
    const near=await wait(()=>notes().find(n=>!oldIds.has(n.id)),"source note child");
    await selectNote(near.id);
    const nearRect=await api("/window/rect");
    assert.equal(nearRect.width,420);assert.equal(nearRect.height,360);
    assert.equal(nearRect.x,128);assert.equal(nearRect.y,158);
    oldIds.add(near.id);
    menuItem("새 메모");
    const latest=await wait(()=>notes().find(n=>!oldIds.has(n.id)),"latest note child");
    await selectNote(latest.id);
    const latestRect=await api("/window/rect");
    assert.equal(latestRect.width,420);assert.equal(latestRect.height,360);
    assert.equal(near.body,"");assert.equal(latest.body,"");
    fs.writeFileSync(path.join(output,"window-inheritance.json"),JSON.stringify({sourceId:blank.id,nearId:near.id,latestId:latest.id,nearRect,latestRect},null,2));
    checks.push("native resize and source/latest size inheritance, including cascade position");
    await selectNote(blank.id);
    const wmState=()=>execFileSync("xprop",["-id",noteWindow,"_NET_WM_STATE"],{encoding:"utf8"});
    const otherWindow=[...initialWindows][0];
    const modes=[];
    for(const [index,mode] of [[2,"top"],[3,"desktop"],[1,"normal"]]) {
      xdo("windowactivate","--sync",noteWindow);
      await click('button[aria-label^="위치:"]');
      await click(`[role="radiogroup"] [role="radio"]:nth-child(${index})`);
      await wait(()=>local().windows[blank.id]?.pin===mode,`pin ${mode} persisted`);
      xdo("windowactivate","--sync",otherWindow);
      await wait(()=>{
        const state=wmState();
        return mode==="top" ? state.includes("_NET_WM_STATE_ABOVE")
          : mode==="desktop" ? state.includes("_NET_WM_STATE_BELOW")
          : !/_(ABOVE|BELOW)/.test(state);
      },`native ${mode} stacking after blur`);
      modes.push({mode,blur:wmState()});
      xdo("windowactivate","--sync",noteWindow);
      await wait(()=>wmState().includes("_NET_WM_STATE_ABOVE"),"focused note raised");
      modes.at(-1).focus=wmState();
    }
    fs.writeFileSync(path.join(output,"native-pin-modes.json"),JSON.stringify(modes,null,2));
    checks.push("X11 top/desktop/normal stacking hints and focus/blur restoration");
    const beforeDrag=await api("/window/rect");
    const dragPoint=await execute("const r=document.querySelector('header.bar').getBoundingClientRect(); return {x:Math.round(r.x+80),y:Math.round(r.y+r.height/2)}");
    xdo("mousemove","--window",noteWindow,String(dragPoint.x),String(dragPoint.y));
    xdo("mousedown","1");
    await sleep(100);
    xdo("mousemove_relative","--sync","--","55","45");
    await sleep(100);
    xdo("mouseup","1");
    const afterDrag=await wait(async()=>{const r=await api("/window/rect");return r.x!==beforeDrag.x||r.y!==beforeDrag.y?r:false;},"header drag moved native window");
    assert.equal(afterDrag.width,beforeDrag.width);assert.equal(afterDrag.height,beforeDrag.height);
    await wait(()=>local().windows[blank.id]?.x===afterDrag.x && local().windows[blank.id]?.y===afterDrag.y,"drag position persisted");
    fs.writeFileSync(path.join(output,"native-drag.json"),JSON.stringify({beforeDrag,afterDrag},null,2));
    const capturedPointers=await execute("return Array.from({length:10},(_,id)=>id).filter(id=>document.querySelector('header.bar').hasPointerCapture(id))");
    fs.writeFileSync(path.join(output,"drag-pointer-capture.json"),JSON.stringify({capturedPointers},null,2));
    assert.deepEqual(capturedPointers,[],"Native drag must release DOM pointer capture before later clicks");
    checks.push("header drag moves the native window and persists position without resizing");
    const listEditor=(await api("/element",{using:"css selector",value:".cm-content"}))["element-6066-11e4-a52e-4f735466cecf"];
    await api(`/element/${listEditor}/value`,{text:"- first\uE007second\uE007\uE007end"});
    const listBody="- first\n- second\nend";
    await wait(()=>notes().find(n=>n.id===blank.id)?.body===listBody,"bullet continuation and empty-item exit");
    checks.push("Enter continues a bullet list and an empty item exits it");
    await api(`/element/${listEditor}/value`,{text:"\uE009a\uE000\uE003"});
    await wait(()=>notes().find(n=>n.id===blank.id)?.body==="","temporary list text cleared");
    const urls=["http://example.com/pinboard-http","https://example.com/pinboard-https","https://www.example.com/pinboard-www","https://example.com/pinboard-markdown"];
    const linkText="http://example.com/pinboard-http\nhttps://example.com/pinboard-https\nwww.example.com/pinboard-www\n[Guide](https://example.com/pinboard-markdown)";
    await api(`/element/${listEditor}/value`,{text:linkText.replaceAll("\n","\uE007")});
    await wait(()=>execute("return document.querySelectorAll('.cm-note-link').length === 4"),"all link forms decorated");
    const opened=()=>fs.existsSync(openedUrls)?fs.readFileSync(openedUrls,"utf8").trim().split("\n").filter(Boolean).map(line=>JSON.parse(line)[0]):[];
    // A selected URL must remain selectable without opening the external handler.
    await api(`/element/${listEditor}/value`,{text:"\uE009a\uE000"});
    const firstLink=await execute("const r=document.querySelector('.cm-note-link').getClientRects()[0];return {x:Math.round(r.x),y:Math.round(r.y+r.height/2)}");
    xdo("mousemove","--window",noteWindow,String(firstLink.x+80),String(firstLink.y));
    xdo("keydown","Shift_L");
    try{xdo("click","1");}finally{xdo("keyup","Shift_L");}
    await sleep(250);
    assert.deepEqual(opened(),[]);
    assert.ok(await execute("return getSelection().toString().length>0"));
    xdo("mousemove","--window",noteWindow,String(firstLink.x+4),String(firstLink.y));
    xdo("mousedown","1");
    await sleep(100);
    xdo("mousemove","--window",noteWindow,String(firstLink.x+150),String(firstLink.y));
    await sleep(100);
    xdo("mouseup","1");
    await wait(()=>execute("return getSelection().toString().length>5"),"drag selects link text");
    assert.deepEqual(opened(),[]);
    xdo("key","--clearmodifiers","Left");
    await wait(()=>execute("return getSelection().toString().length===0"),"native arrow key collapses selection");
    await sleep(250);
    fs.writeFileSync(path.join(output,"link-selection-before-click.json"),JSON.stringify(await execute("return {selection:getSelection().toString(),text:document.querySelector('.cm-content').innerText,links:[...document.querySelectorAll('.cm-note-link')].map(e=>({text:e.textContent,href:e.dataset.href}))}"),null,2));
    await execute("window.__linkClicks=[];document.addEventListener('click',e=>window.__linkClicks.push({target:e.target.outerHTML?.slice(0,200),shift:e.shiftKey,alt:e.altKey,button:e.button,x:e.clientX,y:e.clientY,screenX:e.screenX,screenY:e.screenY,selection:getSelection().toString()}),true)");
    for(const [i,url] of urls.entries()) {
      fs.writeFileSync(path.join(output,"link-layout.json"),JSON.stringify(await execute("return {scrollY,window:{width:innerWidth,height:innerHeight},rects:['.note','header.bar','.body','.cm-scroller','.cm-content','.cm-note-link'].map(s=>{const e=document.querySelector(s);return {selector:s,rect:e.getBoundingClientRect().toJSON(),scroll:e.scrollTop,position:getComputedStyle(e).position,inert:e.inert}})}"),null,2));
      const point=await execute("const r=[...document.querySelectorAll('.cm-note-link')].find(e=>e.dataset.href===arguments[0]).getClientRects()[0];return {x:Math.round(r.x+Math.min(25,r.width/2)),y:Math.round(r.y+r.height/2)}",[url]);
      xdo("windowactivate","--sync",noteWindow);
      xdo("mousemove","--window",noteWindow,String(point.x),String(point.y));
      xdo("click","1");
      await sleep(500);
      fs.writeFileSync(path.join(output,"opener-diagnostic.json"),JSON.stringify({point,geometry:xdo("getwindowgeometry","--shell",noteWindow),state:await execute("return {clicks:window.__linkClicks,selection:getSelection().toString()}" )},null,2));
      await wait(()=>opened().length===i+1,`external URI handler ${url}`);
      assert.equal(opened()[i],url);
      assert.match(await execute("return location.href"),/^tauri:\/\/localhost/);
    }
    fs.writeFileSync(path.join(output,"link-interactions.json"),JSON.stringify({passed:true,selectionPreserved:true,opened:opened(),handler:"isolated BROWSER executable through real native opener and xdg-open"},null,2));
    checks.push("HTTP/HTTPS/www/Markdown URL dispatch, native drag selection and Shift-click protection");
    await api(`/element/${listEditor}/value`,{text:"\uE009a\uE000\uE003"});
    await wait(()=>notes().find(n=>n.id===blank.id)?.body==="","temporary links cleared");
  }
  blocked=path.join(data,"notes",`${blank.id}.json`); original=`${blocked}.original`;
  fs.renameSync(blocked,original); fs.mkdirSync(blocked);
  const editor=(await api("/element",{using:"css selector",value:".cm-content"}))["element-6066-11e4-a52e-4f735466cecf"];
  const pending="Preserve failed tray edit 한글";
  await api(`/element/${editor}/value`,{text:pending});
  assert.equal(await execute("return document.querySelector('.cm-content').textContent"),pending);
  await wait(()=>execute("return !!document.querySelector('.save-error')"),"save failure displayed");
  await capture("save-error");
  if(!reproduce) {
    execFileSync("xdotool",["windowactivate","--sync",noteWindow],{timeout:5000});
    execFileSync("xdotool",["key","--clearmodifiers","alt+F4"]);
    await sleep(400);
    fs.writeFileSync(path.join(output,"native-close.json"),JSON.stringify({noteWindow,initialWindows:[...initialWindows],visibleAfterClose:visibleWindows(),label:await execute("return window.__TAURI_INTERNALS__.metadata.currentWindow.label")},null,2));
    assert.ok(visibleWindows().includes(noteWindow),"Native close retains a failed save");
    checks.push("Alt+F4 keeps an unsaved note visible");
  }
  menuItem("모든 메모 숨기기");
  await sleep(500);
  const hidden=visibleWindows().length===0;
  // WebKitGTK cannot capture an unmapped native window; visibility is authoritative.
  if(!hidden) await capture("after-hide-all");
  menuItem("Pinboard 종료");
  if(reproduce){
    await wait(()=>!alive(),"old app exits despite failed save");
    const oldBody=JSON.parse(fs.readFileSync(original,"utf8")).body;
    assert.equal(hidden,true); assert.notEqual(oldBody,pending);
    fs.writeFileSync(path.join(output,"result.json"),JSON.stringify({reproduced:true,binary,data,hiddenAfterSaveFailure:hidden,exitedWithUnsavedText:true,savedBody:oldBody,pendingBody:pending},null,2)+"\n");
  }else{
    await sleep(700);
    assert.equal(hidden,false,"A failed save must keep notes visible");
    assert.equal(alive(),true,"Quit must abort when an editor cannot save");
    assert.ok(visibleWindows().length>0,"Show the failure after a rejected quit");
    assert.equal(await execute("return document.querySelector('.note').inert"),false);
    checks.push("hide-all and quit retain unsaved text and visible retry UI");
    unblock();
    menuItem("모든 메모 숨기기");
    await wait(()=>visibleWindows().length===0,"successful hide-all");
    assert.equal(notes().find(n=>n.id===blank.id).body,pending);
    menuItem("모든 메모 보이기");
    await wait(()=>visibleWindows().length>=2,"show-all restores native windows");
    checks.push("retry saves text, hides notes, and show-all restores windows");
    await selectNote(blank.id);
    const beforeCalendar=new Set(visibleWindows());
    await click('button[aria-label="메모 달력"]');
    await select("main.calendar");
    const calendarWindow=await wait(()=>visibleWindows().find(id=>!beforeCalendar.has(id)
      && xdo("getwindowname",id).trim()==="메모 달력"),"native calendar window");
    const calendarBefore=await api("/window/rect");
    xdo("windowactivate","--sync",calendarWindow);
    xdo("mousemove","--window",calendarWindow,"200","20");xdo("mousedown","1");
    await sleep(100);xdo("mousemove_relative","--sync","--","45","35");
    await sleep(100);xdo("mouseup","1");
    const calendarAfter=await wait(async()=>{const r=await api("/window/rect");return r.x!==calendarBefore.x||r.y!==calendarBefore.y?r:false;},"calendar dragged");
    assert.equal(await execute("return document.querySelector('header').hasPointerCapture(1)"),false);
    const monthBefore=await execute("return document.querySelector('.month h1').textContent");
    await click('button[aria-label="다음 달"]');
    await wait(async()=>await execute("return document.querySelector('.month h1').textContent")!==monthBefore,"calendar click works after drag");
    await click(".pin-toggle");await click('[role="radio"]:nth-child(2)');
    await wait(()=>JSON.parse(fs.readFileSync(path.join(data,"local-state.json"),"utf8")).calendar_pin==="top","calendar top pin saved");
    xdo("windowactivate","--sync",noteWindow);
    const calendarState=()=>execFileSync("xprop",["-id",calendarWindow,"_NET_WM_STATE"],{encoding:"utf8"});
    await wait(()=>calendarState().includes("_NET_WM_STATE_ABOVE"),"calendar stays above after blur");
    fs.writeFileSync(path.join(output,"calendar-window.json"),JSON.stringify({calendarBefore,calendarAfter,monthBefore,monthAfter:await execute("return document.querySelector('.month h1').textContent"),blurState:calendarState(),captureReleased:true},null,2));
    await click('button[aria-label="달력 닫기"]');
    await wait(()=>!visibleWindows().includes(calendarWindow),"calendar closes");
    checks.push("calendar drag releases capture, month controls work, and top pin survives blur");
    menuItem("설정");
    await select(".shell nav.side");
    menuItem("지금 동기화");
    checks.push("tray settings and sync actions remain responsive");
    menuItem("Pinboard 종료");
    await wait(()=>!alive(),"successful quit");
    checks.push("quit succeeds after all editors save");
    fs.writeFileSync(path.join(output,"result.json"),JSON.stringify({passed:true,binary,data,checks},null,2)+"\n");
  }
  console.log(fs.readFileSync(path.join(output,"result.json"),"utf8"));
} catch(error) {
  if(session&&(!appPid||alive())) {await capture("failure").catch(()=>{});}
  fs.writeFileSync(path.join(output,"failure.json"),JSON.stringify({error:String(error),checks,data},null,2));
  throw error;
} finally {
  unblock();
  if(session)await api("",undefined,"DELETE").catch(()=>{});
  driver.kill();wm.kill();watcher.kill();
  fs.closeSync(log);fs.closeSync(watcherFd);
}
