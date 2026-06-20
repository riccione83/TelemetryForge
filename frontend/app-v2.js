const tauri = window.__TAURI__;
if (!tauri?.core?.invoke) throw new Error("The Tauri bridge is unavailable");
const invoke = tauri.core.invoke;
const $ = id => document.getElementById(id);
const t = (key, vars) => window.TurzxI18n.t(key, vars);
let config, currentScreen = "", selectedWidget = -1;
let selectedWidgets = new Set();
let previewTimer;
let brightnessTimer;

const nameKeys = {
  cpu_temperature:"widgetCpuTemp",cpu_usage:"widgetCpuUsage",gpu_temperature:"widgetGpuTemp",
  gpu_usage:"widgetGpuUsage",gpu_clock:"widgetGpuClock",ram_usage:"widgetRam",vram_usage:"widgetVram",
  disk_usage:"widgetDisk",network_upload:"widgetUpload",network_download:"widgetDownload",
  fan_speed:"widgetFan",clock:"widgetClock",date:"widgetDate",fps:"widgetFps",text:"widgetText"
};
const widgetName = kind => t(nameKeys[kind] || kind);
const defaults = {
  cpu_temperature: "{value}", cpu_usage: "{value}",
  gpu_temperature: "{value}", gpu_usage: "{value}",
  gpu_clock: "{value}", ram_usage: "{value}", vram_usage: "{value}",
  disk_usage: "{value}", network_upload: "{value}", network_download: "{value}",
  fan_speed: "{value}", clock: "{value}", date: "{value}", fps: "{value}",
  text: "Testo libero"
};
const fonts = ["Segoe UI","Segoe UI Bold","Arial","Arial Bold","Bahnschrift","Calibri","Calibri Bold","Consolas","Consolas Bold","Courier New","Impact","Tahoma","Trebuchet MS","Verdana"];
const modeKeys = {text:"modeText",bar:"modeBar",circle:"modeCircle",graph:"modeGraph"};
const modeName = mode => t(modeKeys[mode] || mode);

function refreshTranslatedUi() {
  window.TurzxI18n.apply();
  $("new-widget-kind").innerHTML = Object.keys(nameKeys)
    .map(kind => `<option value="${kind}">${widgetName(kind)}</option>`).join("");
  if (config) {
    renderWidgets();
    renderOverlay();
    loadScreens();
    loadPorts();
  }
}

async function boot() {
  config = await invoke("get_config");
  refreshTranslatedUi();
  bindConfig();
  $("autostart").checked = await invoke("get_autostart").catch(() => false);
  await Promise.all([loadPorts(), loadScreens()]);
  await refreshPreview();
  setInterval(refreshStatus, 1200);
}

function bindConfig() {
  const values = {
    orientation: config.display.orientation, width: config.display.width, height: config.display.height,
    brightness: config.display.brightness, frameInterval: config.frame_interval_ms,
    cpuTemperatureSource: config.cpu_temperature_source || "core",
    backgroundPath: config.background.path || "", backgroundMode: config.background.mode,
    backgroundSource: config.background.source || "file",
    backgroundFolder: config.background.folder || "",
    slideshowInterval: config.background.slideshow_interval_minutes || 5,
    backgroundColour: config.background.colour, foreground: config.theme.foreground,
    accent: config.theme.accent
  };
  Object.entries(values).forEach(([id, value]) => $(id).value = value);
  $("screen").style.aspectRatio = `${config.display.width} / ${config.display.height}`;
  renderWidgets();
  requestAnimationFrame(renderOverlay);
}

async function loadPorts() {
  const ports = await invoke("list_displays").catch(() => []);
  $("port-list").innerHTML = `<option value="AUTO">${t("automatic")}</option>` + ports.map(p =>
    `<option value="${p.port}">${p.likely_turzx ? "TURZX — " : ""}${p.name}</option>`).join("");
  $("port").value = config.display.port;
}

async function loadScreens(selected = currentScreen) {
  const screens = await invoke("list_screens");
  $("screen-list").innerHTML = `<option value="">${t("selectScreen")}</option>` +
    screens.map(n => `<option value="${esc(n)}">${esc(n)}</option>`).join("");
  if (screens.includes(selected)) $("screen-list").value = selected;
  $("screen-state").textContent = currentScreen || t("screenCurrent");
}

function renderWidgets() {
  $("widgets").innerHTML = config.widgets.map((w, i) => `
    <div class="widget-card ${selectedWidgets.has(i) || selectedWidget === i ? "selected" : ""}" data-card="${i}">
      <div class="widget-title">
        <label class="toggle"><input data-i="${i}" data-k="enabled" type="checkbox" ${w.enabled ? "checked" : ""}><strong>${esc(widgetName(w.kind))} · ${modeName(w.render_mode || "text")}</strong></label>
        <div class="actions">
          <button class="secondary compact" data-add-bar="${i}">${t("addBar")}</button>
          <button class="secondary compact" data-add-circle="${i}">${t("addCircle")}</button>
          <button class="secondary compact" data-add-graph="${i}">${t("addGraph")}</button>
          <button class="danger compact" data-delete="${i}">${t("remove")}</button>
        </div>
      </div>
      <div class="widget-fields">
        <label>${t("type")}<select data-i="${i}" data-k="kind">${Object.keys(nameKeys).map(v => `<option value="${v}" ${w.kind === v ? "selected" : ""}>${widgetName(v)}</option>`).join("")}</select></label>
        <label>${t("visualisation")}<select data-i="${i}" data-k="render_mode">${Object.keys(modeKeys).map(v => `<option value="${v}" ${(w.render_mode || "text") === v ? "selected" : ""}>${modeName(v)}</option>`).join("")}</select></label>
        <label class="wide">${t("textFormat")}<input data-i="${i}" data-k="label_format" value="${esc(w.label_format)}" placeholder="GPU {value} MHz"></label>
        <label>${t("leftText")}<input data-i="${i}" data-k="left_text" value="${esc(w.left_text || "")}" placeholder="GPU "></label>
        <label>${t("rightText")}<input data-i="${i}" data-k="right_text" value="${esc(w.right_text || "")}" placeholder=" °C"></label>
        <label>X<input data-i="${i}" data-k="x" type="number" value="${w.x}"></label>
        <label>Y<input data-i="${i}" data-k="y" type="number" value="${w.y}"></label>
        <label>${t("width")}<input data-i="${i}" data-k="width" type="number" min="1" value="${w.width}"></label>
        <label>${t("height")}<input data-i="${i}" data-k="height" type="number" min="1" value="${w.height}"></label>
        <label>${t("font")}<select data-i="${i}" data-k="font">${fonts.map(font => `<option value="${font}" ${(w.font || "Segoe UI") === font ? "selected" : ""}>${font}</option>`).join("")}</select></label>
        <label>${t("fontSize")}<input data-i="${i}" data-k="font_size" type="number" min="6" value="${w.font_size}"></label>
        <label>${t("interval")}<input data-i="${i}" data-k="refresh_interval_ms" type="number" min="100" value="${w.refresh_interval_ms}"></label>
        <label>${t("colour")}<input data-i="${i}" data-k="colour" type="color" value="${w.colour}"></label>
        <label>${t("gradient")}<input data-i="${i}" data-k="secondary_colour" type="color" value="${w.secondary_colour || w.colour}"></label>
        <label>${t("opacity")}<input data-i="${i}" data-k="opacity" type="range" min="0.1" max="1" step="0.05" value="${w.opacity ?? 1}"></label>
        <label>${t("glow")}<input data-i="${i}" data-k="glow" type="range" min="0" max="16" value="${w.glow || 0}"></label>
        <label>${t("shadow")}<input data-i="${i}" data-k="shadow" type="range" min="0" max="16" value="${w.shadow || 0}"></label>
        <label class="toggle">${t("thresholds")}<input data-i="${i}" data-k="use_thresholds" type="checkbox" ${w.use_thresholds ? "checked" : ""}></label>
        <label>${t("warning")}<input data-i="${i}" data-k="warning_threshold" type="number" value="${w.warning_threshold ?? 70}"></label>
        <label>${t("critical")}<input data-i="${i}" data-k="critical_threshold" type="number" value="${w.critical_threshold ?? 90}"></label>
        <label>${t("warningColour")}<input data-i="${i}" data-k="warning_colour" type="color" value="${w.warning_colour || "#ffd166"}"></label>
        <label>${t("criticalColour")}<input data-i="${i}" data-k="critical_colour" type="color" value="${w.critical_colour || "#ff4d6d"}"></label>
        <label>${t("circleThickness")}<input data-i="${i}" data-k="circle_thickness" type="number" min="1" value="${w.circle_thickness ?? 16}"></label>
        <label>${t("startAngle")}<input data-i="${i}" data-k="circle_start_angle" type="number" value="${w.circle_start_angle ?? -90}"></label>
        <label>${t("circleSweep")}<input data-i="${i}" data-k="circle_sweep_angle" type="number" min="1" max="360" value="${w.circle_sweep_angle ?? 360}"></label>
      </div>
    </div>`).join("");
  document.querySelectorAll("[data-card]").forEach(card => card.onclick = e => {
    if (e.target.closest("button,input,select")) return;
    selectedWidget = +card.dataset.card;
    selectedWidgets = new Set([selectedWidget]);
    renderWidgets(); renderOverlay();
  });
  document.querySelectorAll("[data-delete]").forEach(button => button.onclick = () => {
    config.widgets.splice(+button.dataset.delete, 1); selectedWidget = -1; selectedWidgets.clear();
    renderWidgets(); renderOverlay();
  });
  document.querySelectorAll("[data-add-bar]").forEach(button => button.onclick = () => addVisualWidget(+button.dataset.addBar, "bar"));
  document.querySelectorAll("[data-add-circle]").forEach(button => button.onclick = () => addVisualWidget(+button.dataset.addCircle, "circle"));
  document.querySelectorAll("[data-add-graph]").forEach(button => button.onclick = () => addVisualWidget(+button.dataset.addGraph, "graph"));
  document.querySelectorAll("[data-i]").forEach(input => {
    const update = () => {
      readWidgetInput(input);
      renderOverlay();
      scheduleLivePreview();
    };
    input.oninput = update;
    input.onchange = update;
  });
}

function addVisualWidget(sourceIndex, renderMode) {
  const source = config.widgets[sourceIndex];
  const visual = {
    ...source,
    render_mode: renderMode,
    left_text: "",
    right_text: "",
    label_format: "{value}",
    x: Math.min(config.display.width - 20, source.x + 20),
    y: Math.min(config.display.height - 20, source.y + 20),
    width: renderMode === "circle" ? 80 : 150,
    height: renderMode === "circle" ? 80 : renderMode === "graph" ? 70 : 20
  };
  config.widgets.push(visual);
  selectedWidget = config.widgets.length - 1;
  selectedWidgets = new Set([selectedWidget]);
  renderWidgets();
  renderOverlay();
  scheduleLivePreview();
}

function readWidgetInput(input) {
  const w = config.widgets[+input.dataset.i];
  w[input.dataset.k] = input.type === "checkbox" ? input.checked :
    (input.type === "number" || input.type === "range") ? +input.value : input.value;
}

function scheduleLivePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(async () => {
    try {
      $("preview").src = await invoke("preview_config", {config});
      $("error").textContent = "";
    } catch (error) {
      $("error").textContent = `${t("previewError")}: ${error}`;
    }
  }, 160);
}

function renderOverlay() {
  const layer = $("widget-overlay");
  if (!layer?.clientWidth) return;
  layer.innerHTML = "";
  layer.onpointerdown = startMarquee;
  layer.oncontextmenu = event => {
    if (selectedWidgets.size < 2) return;
    event.preventDefault();
    showObjectMenu(event.clientX, event.clientY);
  };
  const sx = layer.clientWidth / config.display.width, sy = layer.clientHeight / config.display.height;
  config.widgets.forEach((w, i) => {
    if (!w.enabled) return;
    const el = document.createElement("div");
    el.className = `widget-handle ${selectedWidgets.has(i) || selectedWidget === i ? "selected" : ""}`;
    el.dataset.index = i;
    Object.assign(el.style, {
      left: `${w.x*sx}px`, top: `${w.y*sy}px`,
      width: `${Math.max(w.width*sx,30)}px`, height: `${Math.max(w.height*sy,18)}px`
    });
    el.innerHTML = `<span>${esc(widgetName(w.kind))} · ${modeName(w.render_mode || "text")}</span><i class="resize-handle"></i>`;
    el.onpointerdown = event => {
      if (event.target.classList.contains("resize-handle")) return;
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        event.stopPropagation();
        if (selectedWidgets.has(i)) selectedWidgets.delete(i); else selectedWidgets.add(i);
        selectedWidget = selectedWidgets.has(i) ? i : [...selectedWidgets][0] ?? -1;
        renderWidgets();
        renderOverlay();
        return;
      }
      if (!selectedWidgets.has(i)) {
        selectedWidgets = new Set([i]);
        selectedWidget = i;
      }
      startDrag(event);
    };
    el.oncontextmenu = event => {
      event.preventDefault();
      event.stopPropagation();
      if (!selectedWidgets.has(i)) {
        selectedWidgets = new Set([i]);
        selectedWidget = i;
        renderWidgets();
        renderOverlay();
      }
      if (selectedWidgets.size >= 2) showObjectMenu(event.clientX, event.clientY);
    };
    el.querySelector(".resize-handle").onpointerdown = event => startResize(event, i, el);
    layer.appendChild(el);
  });
}

function startMarquee(event) {
  if (event.target !== event.currentTarget || event.button !== 0) return;
  event.preventDefault();
  hideObjectMenu();
  const layer = event.currentTarget;
  const rect = layer.getBoundingClientRect();
  const startX = event.clientX - rect.left;
  const startY = event.clientY - rect.top;
  const previous = (event.ctrlKey || event.metaKey) ? new Set(selectedWidgets) : new Set();
  const box = document.createElement("div");
  box.className = "selection-box";
  layer.appendChild(box);
  layer.setPointerCapture(event.pointerId);

  const move = e => {
    const x = Math.max(0, Math.min(rect.width, e.clientX - rect.left));
    const y = Math.max(0, Math.min(rect.height, e.clientY - rect.top));
    const left = Math.min(startX, x), top = Math.min(startY, y);
    const right = Math.max(startX, x), bottom = Math.max(startY, y);
    Object.assign(box.style, {
      left: `${left}px`, top: `${top}px`,
      width: `${right-left}px`, height: `${bottom-top}px`
    });
    selectedWidgets = new Set(previous);
    layer.querySelectorAll(".widget-handle").forEach(handle => {
      const h = handle.getBoundingClientRect();
      const intersects = h.left < rect.left+right && h.right > rect.left+left &&
        h.top < rect.top+bottom && h.bottom > rect.top+top;
      if (intersects) selectedWidgets.add(+handle.dataset.index);
    });
    selectedWidget = [...selectedWidgets][0] ?? -1;
    layer.querySelectorAll(".widget-handle").forEach(handle =>
      handle.classList.toggle("selected", selectedWidgets.has(+handle.dataset.index)));
  };
  const end = () => {
    layer.onpointermove = layer.onpointerup = layer.onpointercancel = null;
    box.remove();
    renderWidgets();
    renderOverlay();
  };
  layer.onpointermove = move;
  layer.onpointerup = layer.onpointercancel = end;
}

function showObjectMenu(x, y) {
  const menu = $("object-menu");
  menu.style.left = `${Math.min(x, window.innerWidth-240)}px`;
  menu.style.top = `${Math.min(y, window.innerHeight-150)}px`;
  menu.classList.add("open");
}

function hideObjectMenu() {
  $("object-menu").classList.remove("open");
}

async function applyLayout(action) {
  const indexes = [...selectedWidgets].filter(i => config.widgets[i]?.enabled);
  if (indexes.length < 2) return;
  const widgets = indexes.map(i => config.widgets[i]);
  if (action === "align" || action === "align-distribute") {
    const centre = widgets.reduce((sum, w) => sum + w.x + w.width/2, 0) / widgets.length;
    widgets.forEach(w => w.x = Math.round(Math.max(0, Math.min(config.display.width-w.width, centre-w.width/2))));
  }
  if (action === "distribute" || action === "align-distribute") {
    const sorted = widgets.sort((a,b) => a.y-b.y);
    const top = sorted[0].y;
    const bottom = Math.max(...sorted.map(w => w.y+w.height));
    const totalHeight = sorted.reduce((sum,w) => sum+w.height, 0);
    const gap = Math.max(0, (bottom-top-totalHeight)/(sorted.length-1));
    let cursor = top;
    sorted.forEach(w => {
      w.y = Math.round(cursor);
      cursor += w.height + gap;
    });
  }
  hideObjectMenu();
  renderWidgets();
  renderOverlay();
  await invoke("save_config", {config});
  await refreshPreview();
}

function startResize(e, index, el) {
  e.preventDefault();
  e.stopPropagation();
  const w = config.widgets[index];
  const rect = $("widget-overlay").getBoundingClientRect();
  const startX = e.clientX, startY = e.clientY, startWidth = w.width, startHeight = w.height;
  selectedWidget = index;
  el.setPointerCapture(e.pointerId);
  el.onpointermove = m => {
    const dx = (m.clientX-startX)*config.display.width/rect.width;
    const dy = (m.clientY-startY)*config.display.height/rect.height;
    w.width = Math.round(Math.max(8, Math.min(config.display.width-w.x, startWidth+dx)));
    w.height = Math.round(Math.max(8, Math.min(config.display.height-w.y, startHeight+dy)));
    el.style.width = `${w.width*rect.width/config.display.width}px`;
    el.style.height = `${w.height*rect.height/config.display.height}px`;
  };
  const end = async () => {
    el.onpointermove = el.onpointerup = el.onpointercancel = null;
    renderWidgets();
    await invoke("save_config", {config});
    await refreshPreview();
  };
  el.onpointerup = el.onpointercancel = end;
}

function startDrag(e) {
  e.preventDefault();
  const el = e.currentTarget, i = +el.dataset.index, w = config.widgets[i];
  const rect = $("widget-overlay").getBoundingClientRect();
  const sx = e.clientX, sy = e.clientY, ox = w.x, oy = w.y;
  selectedWidget = i;
  const moving = selectedWidgets.has(i) && selectedWidgets.size > 1
    ? [...selectedWidgets].map(index => ({index, x:config.widgets[index].x, y:config.widgets[index].y}))
    : [{index:i, x:ox, y:oy}];
  el.setPointerCapture(e.pointerId);
  el.onpointermove = m => {
    const dx=(m.clientX-sx)*config.display.width/rect.width;
    const dy=(m.clientY-sy)*config.display.height/rect.height;
    moving.forEach(item => {
      const target=config.widgets[item.index];
      target.x=Math.round(Math.max(0,Math.min(config.display.width-target.width,item.x+dx)));
      target.y=Math.round(Math.max(0,Math.min(config.display.height-target.height,item.y+dy)));
      const handle=$(`widget-overlay`).querySelector(`[data-index="${item.index}"]`);
      if(handle){handle.style.left=`${target.x*rect.width/config.display.width}px`;handle.style.top=`${target.y*rect.height/config.display.height}px`;}
    });
  };
  const end = async () => {
    el.onpointermove = el.onpointerup = el.onpointercancel = null;
    renderWidgets(); await invoke("save_config", {config}); await refreshPreview();
  };
  el.onpointerup = el.onpointercancel = end;
}

function readForm() {
  Object.assign(config.display, {
    port: $("port").value, orientation: $("orientation").value,
    width: +$("width").value, height: +$("height").value, brightness: +$("brightness").value
  });
  config.frame_interval_ms = +$("frameInterval").value;
  config.cpu_temperature_source = $("cpuTemperatureSource").value;
  config.background.mode = $("backgroundMode").value;
  config.background.source = $("backgroundSource").value;
  config.background.folder = $("backgroundFolder").value || null;
  config.background.slideshow_interval_minutes = Math.max(1,+$("slideshowInterval").value||5);
  config.background.colour = $("backgroundColour").value;
  config.theme.foreground = $("foreground").value;
  config.theme.accent = $("accent").value;
}

async function save() {
  readForm(); await invoke("save_config", {config});
  try { await invoke("set_autostart", {enabled:$("autostart").checked}); } catch {}
  bindConfig(); await refreshPreview(); $("status").textContent = t("configurationSaved");
}
async function refreshPreview() {
  try { $("error").textContent=""; $("preview").src=await invoke("get_preview"); requestAnimationFrame(renderOverlay); }
  catch(e) { $("error").textContent=String(e); }
}
async function refreshStatus() {
  const s=await invoke("get_status");
  const known={"Stopped":"statusStopped","Rendering active":"statusActive","Frame sent":"statusFrameSent"};
  $("status").textContent=known[s.message]?t(known[s.message]):s.message;
  if(s.running) refreshPreview();
}
const askName=(message,initial="")=>window.prompt(message,initial)?.trim()||"";

$("choose-bg").onclick=async()=>{const p=await invoke("select_background");if(p){config.background.path=p;config.background.source="file";$("backgroundPath").value=p;$("backgroundSource").value="file";await save();}};
$("choose-bg-folder").onclick=async()=>{const p=await invoke("select_background_folder");if(p){config.background.folder=p;config.background.source="folder";$("backgroundFolder").value=p;$("backgroundSource").value="folder";await save();}};
$("backgroundSource").onchange=()=>{readForm();scheduleLivePreview();};
$("slideshowInterval").oninput=()=>{readForm();scheduleLivePreview();};
$("apply-neon-sample").onclick=async()=>{config=await invoke("load_neon_sample");currentScreen="";bindConfig();await refreshPreview();};
$("refresh-ports").onclick=loadPorts;
$("brightness").oninput=()=>{
  config.display.brightness=+$("brightness").value;
  clearTimeout(brightnessTimer);
  brightnessTimer=setTimeout(async()=>{
    try{
      await invoke("set_display_brightness",{brightness:config.display.brightness});
      $("error").textContent="";
    }catch(e){
      $("error").textContent=`Brightness: ${e}`;
    }
  },180);
};
$("save").onclick=save;
$("add-widget").onclick=()=>{const kind=$("new-widget-kind").value;config.widgets.push({kind,render_mode:"text",enabled:true,left_text:"",right_text:"",x:20,y:20,width:180,height:42,font:"Segoe UI",font_size:24,colour:config.theme.foreground,secondary_colour:config.theme.accent,opacity:1,glow:0,shadow:0,use_thresholds:false,warning_threshold:70,critical_threshold:90,warning_colour:"#ffd166",critical_colour:"#ff4d6d",circle_thickness:16,circle_start_angle:-90,circle_sweep_angle:360,refresh_interval_ms:1000,label_format:defaults[kind]});selectedWidget=config.widgets.length-1;selectedWidgets=new Set([selectedWidget]);renderWidgets();renderOverlay();scheduleLivePreview();};
$("new-screen").onclick=async()=>{const name=askName(t("selectNewScreen"));if(!name)return;try{config=await invoke("new_screen",{name});currentScreen=name;bindConfig();await loadScreens(name);await refreshPreview();}catch(e){$("error").textContent=String(e);}};
$("save-screen").onclick=async()=>{readForm();const name=askName(t("saveScreenName"),currentScreen);if(!name)return;try{await invoke("save_screen",{name,config});currentScreen=name;await loadScreens(name);$("status").textContent=t("screenSaved",{name});}catch(e){$("error").textContent=String(e);}};
$("load-screen").onclick=async()=>{const name=$("screen-list").value;if(!name)return;config=await invoke("load_screen",{name});currentScreen=name;selectedWidget=-1;selectedWidgets.clear();bindConfig();await refreshPreview();};
$("delete-screen").onclick=async()=>{const name=$("screen-list").value;if(!name||!confirm(t("deleteConfirm",{name})))return;await invoke("delete_screen",{name});if(currentScreen===name)currentScreen="";await loadScreens();};
$("start").onclick=async()=>{try{await save();await invoke("start_rendering");}catch(e){$("error").textContent=`${t("renderStartFailed")}: ${e}`;}};
$("stop").onclick=async()=>{await invoke("stop_rendering");$("status").textContent=t("stopRequested");};
$("test-display").onclick=async()=>{try{await save();$("status").textContent=await invoke("test_display");}catch(e){$("error").textContent=`${t("testFailed")}: ${e}`;}};
$("test-sensors").onclick=async()=>{try{const s=await invoke("test_sensors");$("error").textContent="";$("status").textContent=`CPU ${fmt(s.cpu_temperature)}°C · GPU ${fmt(s.gpu_temperature)}°C / ${fmt(s.gpu_usage)}% / ${fmt(s.gpu_clock)} MHz · RAM ${fmt(s.ram_usage)}% · ${t("diskLabel")} ${fmt(s.disk_usage)}% · ${t("networkLabel")} ↓${fmt(s.network_download)} ↑${fmt(s.network_upload)} KB/s`;}catch(e){$("error").textContent=`${t("sensorTestFailed")}: ${e}`;}};
$("send-once").onclick=async()=>{try{await save();await invoke("render_once");}catch(e){$("error").textContent=`${t("sendFailed")}: ${e}`;}};
window.addEventListener("resize",renderOverlay);
document.addEventListener("pointerdown", event => {
  if (!event.target.closest("#object-menu")) hideObjectMenu();
});
document.querySelectorAll("[data-layout-action]").forEach(button =>
  button.onclick = () => applyLayout(button.dataset.layoutAction));
document.querySelectorAll("[data-preset]").forEach(button => button.onclick = () => applyPreset(button.dataset.preset));
function applyPreset(name){
  const styles={
    gaming:{background:"#030409",foreground:"#eaffff",accent:"#31f6ff",secondary:"#ff3b81",frame:80,glow:7,shadow:5,opacity:1,thresholds:true},
    minimal:{background:"#080b10",foreground:"#f4f7fb",accent:"#8da2b5",secondary:"#d6e1ea",frame:180,glow:0,shadow:0,opacity:.92,thresholds:false},
    idle:{background:"#000000",foreground:"#77818c",accent:"#405060",secondary:"#263746",frame:600,glow:0,shadow:0,opacity:.65,thresholds:false}
  };
  const p=styles[name];config.background.colour=p.background;config.theme.foreground=p.foreground;
  config.theme.accent=p.accent;config.frame_interval_ms=p.frame;
  config.widgets.forEach((w,index)=>{
    w.glow=p.glow;
    w.shadow=p.shadow;
    w.opacity=p.opacity;
    w.secondary_colour=index%2===0?p.accent:p.secondary;
    w.use_thresholds=p.thresholds && !["clock","date","text"].includes(w.kind);
    w.warning_colour="#ffd166";
    w.critical_colour="#ff3b5c";
    if(name==="gaming" && w.render_mode!=="text"){
      w.colour=index%2===0?"#31f6ff":"#ff3b81";
    }
    if(name==="minimal"){
      w.colour=p.foreground;
    }
    if(name==="idle"){
      w.colour=p.foreground;
    }
  });
  bindConfig();scheduleLivePreview();$("status").textContent=t("presetApplied",{name});
}
window.addEventListener("turzx-language-changed", refreshTranslatedUi);
function esc(v){return String(v).replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));}
function fmt(v){return v==null?"--":Math.round(v);}
boot().catch(e=>$("error").textContent=String(e));
