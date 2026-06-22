const tauri = window.__TAURI__;
if (!tauri?.core?.invoke) throw new Error("The Tauri bridge is unavailable");
const invoke = tauri.core.invoke;
const $ = id => document.getElementById(id);
const t = (key, vars) => window.TurzxI18n.t(key, vars);
let config, currentScreen = "", selectedWidget = -1;
let selectedWidgets = new Set();
let collapsedWidgets = new Set();
let previewTimer;
let brightnessTimer;
let previewFullscreen = false;
let renderingActive = false;
let renderingBusy = false;
let previewInteractionActive = false;
let previewRefreshPending = false;
let autostartEnabled = false;
let gifPreviewBusy = false;
let availableScreens = [];
let availableSuperWidgets = [];
let availableFanSensors = [];

const nameKeys = {
  cpu_temperature:"widgetCpuTemp",cpu_usage:"widgetCpuUsage",cpu_clock:"widgetCpuClock",gpu_temperature:"widgetGpuTemp",
  gpu_usage:"widgetGpuUsage",gpu_clock:"widgetGpuClock",gpu_power:"widgetGpuPower",ram_usage:"widgetRam",vram_usage:"widgetVram",
  disk_usage:"widgetDisk",network_upload:"widgetUpload",network_download:"widgetDownload",
  fan_speed:"widgetFan",clock:"widgetClock",date:"widgetDate",fps:"widgetFps",text:"widgetText",gif:"widgetGif",
  volume:"widgetVolume",super_widget:"widgetSuperWidget"
};
const widgetName = kind => t(nameKeys[kind] || kind);
const defaults = {
  cpu_temperature: "{value}", cpu_usage: "{value}", cpu_clock: "{value}",
  gpu_temperature: "{value}", gpu_usage: "{value}",
  gpu_clock: "{value}", gpu_power: "{value}", ram_usage: "{value}", vram_usage: "{value}",
  disk_usage: "{value}", network_upload: "{value}", network_download: "{value}",
  fan_speed: "{value}", clock: "{value}", date: "{value}", fps: "{value}",
  text: "Testo libero", gif: "", volume: "{value}", super_widget: ""
};
const defaultAffixes = {
  cpu_temperature: {left:"CPU ", right:" °C"},
  gpu_temperature: {left:"GPU ", right:" °C"},
  cpu_clock: {left:"CPU ", right:""},
  gpu_clock: {left:"GPU ", right:""},
  volume: {left:"VOL ", right:"%"}
};
const fonts = ["Segoe UI","Segoe UI Bold","Arial","Arial Bold","Bahnschrift","Calibri","Calibri Bold","Consolas","Consolas Bold","Courier New","Impact","Tahoma","Trebuchet MS","Verdana"];
const modeKeys = {text:"modeText",bar:"modeBar",circle:"modeCircle",graph:"modeGraph"};
const modeName = mode => t(modeKeys[mode] || mode);

function refreshTranslatedUi() {
  window.TurzxI18n.apply();
  if ($("fullscreen-preview")) {
    $("fullscreen-preview").textContent = t(previewFullscreen ? "exitFullscreen" : "fullscreenPreview");
  }
  updateScreenSaveButton();
  updateRenderButton();
  rebuildWidgetPicker();
  if (config) {
    renderWidgets();
    renderOverlay();
    loadScreens();
    loadPorts();
  }
}

async function boot() {
  config = await invoke("get_config");
  collapseAllWidgets();
  refreshTranslatedUi();
  bindConfig();
  autostartEnabled = await invoke("get_autostart").catch(() => false);
  $("autostart").checked = autostartEnabled;
  await Promise.all([loadPorts(), loadScreens()]);
  await loadSuperWidgets();
  await loadFanSensors();
  await refreshPreview();
  await refreshStatus();
  setInterval(refreshStatus, 1200);
  setInterval(refreshGifPreview, 200);
}

async function loadSuperWidgets() {
  availableSuperWidgets = await invoke("list_superwidgets").catch(() => []);
  rebuildWidgetPicker();
  if (config) renderWidgets();
}

function rebuildWidgetPicker() {
  const picker = $("new-widget-kind");
  if (!picker) return;
  picker.innerHTML =
    `<optgroup label="${t("widgets")}">` +
    Object.keys(nameKeys).filter(kind => kind !== "super_widget")
      .map(kind => `<option value="${kind}">${widgetName(kind)}</option>`).join("") +
    `</optgroup>` +
    `<optgroup label="${t("superWidgets")}">` +
    availableSuperWidgets.map(item =>
      `<option value="super:${esc(item.id)}">${esc(item.name)}</option>`).join("") +
    `</optgroup>`;
}

function bindConfig() {
  config.automation ||= {enabled:false,default_screen:null,rules:[]};
  config.automation.rules ||= [];
  config.transition ||= {kind:"fade",duration_ms:450};
  const values = {
    orientation: config.display.orientation, width: config.display.width, height: config.display.height,
    brightness: config.display.brightness, frameInterval: config.frame_interval_ms,
    cpuTemperatureSource: config.cpu_temperature_source || "core",
    cpuClockSource: config.cpu_clock_source || "average",
    fanSensor: config.fan_sensor || "",
    backgroundPath: config.background.path || "", backgroundMode: config.background.mode,
    backgroundSource: config.background.source || "file",
    backgroundFolder: config.background.folder || "",
    slideshowInterval: config.background.slideshow_interval_minutes || 5,
    backgroundColour: config.background.colour, foreground: config.theme.foreground,
    accent: config.theme.accent
  };
  Object.entries(values).forEach(([id, value]) => $(id).value = value);
  syncColourPickers();
  $("screen").style.aspectRatio = `${config.display.width} / ${config.display.height}`;
  $("screen").style.setProperty("--display-ratio", config.display.width / config.display.height);
  $("automationEnabled").checked = !!config.automation.enabled;
  $("transitionKind").value = config.transition.kind || "fade";
  $("transitionDuration").value = config.transition.duration_ms || 450;
  renderWidgets();
  populateAutomationScreens();
  renderRules();
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
  availableScreens = screens;
  $("screen-list").innerHTML = `<option value="">${t("selectScreen")}</option>` +
    screens.map(n => `<option value="${esc(n)}">${esc(n)}</option>`).join("");
  if (screens.includes(selected)) $("screen-list").value = selected;
  $("screen-state").textContent = currentScreen || t("screenCurrent");
  updateScreenSaveButton();
  for (const slot of ["gaming","minimal","idle"]) {
    const select = $(`quick-${slot}`);
    const assigned = localStorage.getItem(`telemetryforge-quick-${slot}`) || "";
    select.className = "quick-screen-select";
    select.innerHTML = `<option value="">—</option>` +
      screens.map(name => `<option value="${esc(name)}">${esc(name)}</option>`).join("");
    select.value = screens.includes(assigned) ? assigned : "";
    select.onchange = () => localStorage.setItem(`telemetryforge-quick-${slot}`, select.value);
  }
  populateAutomationScreens();
  renderRules();
}

function updateScreenSaveButton() {
  const button = $("save-screen");
  if (!button) return;
  button.textContent = t(currentScreen ? "save" : "saveAs");
  button.title = t(currentScreen ? "saveCurrentScreen" : "saveAsNewScreen");
}

function screenOptions(selected = "", includeEmpty = true) {
  return `${includeEmpty ? `<option value="">—</option>` : ""}` +
    availableScreens.map(name =>
      `<option value="${esc(name)}" ${name === selected ? "selected" : ""}>${esc(name)}</option>`
    ).join("");
}

function populateAutomationScreens() {
  if (!$("defaultScreen") || !config?.automation) return;
  $("defaultScreen").innerHTML = screenOptions(config.automation.default_screen || "");
  $("defaultScreen").value = config.automation.default_screen || "";
}

function renderRules() {
  const root = $("automation-rules");
  if (!root || !config?.automation) return;
  root.innerHTML = config.automation.rules.map((rule, index) => {
    const condition = rule.kind === "process_running"
      ? `<label class="wide">${t("processName")}<input data-rule="${index}" data-rule-key="process_name" value="${esc(rule.process_name || "")}" placeholder="game.exe"></label>`
      : rule.kind === "idle_for"
        ? `<label>${t("idleSeconds")}<input data-rule="${index}" data-rule-key="idle_seconds" type="number" min="1" value="${rule.idle_seconds ?? 300}"></label>`
        : `<label>${rule.kind.includes("usage") ? t("usageThreshold") : t("temperatureThreshold")}<input data-rule="${index}" data-rule-key="threshold" type="number" min="0" max="100" value="${rule.threshold ?? 80}"></label>`;
    return `<div class="automation-rule">
      <label class="toggle"><input data-rule="${index}" data-rule-key="enabled" type="checkbox" ${rule.enabled !== false ? "checked" : ""}><strong>${t("rule")} ${index + 1}</strong></label>
      <label>${t("condition")}<select data-rule="${index}" data-rule-key="kind">
        ${["process_running","gpu_usage_above","cpu_usage_above","gpu_temperature_above","cpu_temperature_above","idle_for"].map(kind =>
          `<option value="${kind}" ${rule.kind === kind ? "selected" : ""}>${t(`rule_${kind}`)}</option>`
        ).join("")}
      </select></label>
      ${condition}
      <label>${t("sustainSeconds")}<input data-rule="${index}" data-rule-key="sustain_seconds" type="number" min="0" value="${rule.sustain_seconds ?? 3}"></label>
      <label>${t("releaseSeconds")}<input data-rule="${index}" data-rule-key="release_seconds" type="number" min="0" value="${rule.release_seconds ?? 8}"></label>
      <label>${t("targetScreen")}<select data-rule="${index}" data-rule-key="screen">${screenOptions(rule.screen || "")}</select></label>
      <div class="actions">
        <button class="secondary compact" data-move-rule="${index}" data-direction="-1" title="${t("moveUp")}">↑</button>
        <button class="secondary compact" data-move-rule="${index}" data-direction="1" title="${t("moveDown")}">↓</button>
        <button class="danger compact" data-delete-rule="${index}">${t("remove")}</button>
      </div>
    </div>`;
  }).join("") || `<p class="muted">${t("noRules")}</p>`;
  root.querySelectorAll("[data-rule]").forEach(input => {
    const update = () => {
      const rule = config.automation.rules[+input.dataset.rule];
      rule[input.dataset.ruleKey] = input.type === "checkbox" ? input.checked :
        input.type === "number" ? +input.value : input.value;
      if (input.dataset.ruleKey === "kind") renderRules();
    };
    input.oninput = update;
    input.onchange = update;
  });
  root.querySelectorAll("[data-delete-rule]").forEach(button => button.onclick = () => {
    config.automation.rules.splice(+button.dataset.deleteRule, 1);
    renderRules();
  });
  root.querySelectorAll("[data-move-rule]").forEach(button => button.onclick = () => {
    const from = +button.dataset.moveRule;
    const to = from + +button.dataset.direction;
    if (to < 0 || to >= config.automation.rules.length) return;
    [config.automation.rules[from], config.automation.rules[to]] =
      [config.automation.rules[to], config.automation.rules[from]];
    renderRules();
  });
}

async function loadFanSensors(snapshot) {
  snapshot ||= await invoke("test_sensors").catch(() => null);
  const sensors = snapshot?.fan_sensors || [];
  availableFanSensors = sensors;
  $("fanSensor").innerHTML = `<option value="">${t("automatic")}</option>` +
    sensors.map(sensor => `<option value="${esc(sensor.id)}">${esc(sensor.name)} — ${Math.round(sensor.value)} RPM</option>`).join("");
  $("fanSensor").value = config.fan_sensor || "";
}

function renderWidgets() {
  $("widgets").innerHTML = config.widgets.map((w, i) => `
    <div class="widget-card ${collapsedWidgets.has(i) ? "collapsed" : ""} ${selectedWidgets.has(i) || selectedWidget === i ? "selected" : ""}" data-card="${i}">
      <div class="widget-title">
        <div class="widget-heading" data-collapse="${i}" title="${collapsedWidgets.has(i) ? t("expand") : t("collapse")}">
          <button class="collapse-widget" type="button" aria-label="${collapsedWidgets.has(i) ? t("expand") : t("collapse")}"></button>
          <label class="toggle"><input data-i="${i}" data-k="enabled" type="checkbox" ${w.enabled ? "checked" : ""}><strong>${esc(w.kind === "super_widget" ? superWidgetName(w.superwidget_id) : `${widgetName(w.kind)} · ${modeName(w.render_mode || "text")}`)}</strong></label>
        </div>
        <div class="actions widget-actions">
          ${w.kind === "super_widget" ? "" : `
            <button class="secondary compact" data-add-bar="${i}">${t("addBar")}</button>
            <button class="secondary compact" data-add-circle="${i}">${t("addCircle")}</button>
            <button class="secondary compact" data-add-graph="${i}">${t("addGraph")}</button>
          `}
          <button class="danger compact" data-delete="${i}">${t("remove")}</button>
        </div>
      </div>
      <div class="widget-fields">
        ${w.kind === "super_widget" ? `
          <label class="wide">${t("superWidget")}<select data-i="${i}" data-k="superwidget_id">${availableSuperWidgets.map(item => `<option value="${esc(item.id)}" ${w.superwidget_id === item.id ? "selected" : ""}>${esc(item.name)}</option>`).join("")}</select></label>
          ${w.superwidget_id === "cpu-command-dial" ? `
            <label>${t("temperatureSource")}<select data-i="${i}" data-binding="temperature">
              <option value="" ${!w.superwidget_bindings?.temperature ? "selected" : ""}>${t("automatic")}</option>
              <option value="cpu_core" ${w.superwidget_bindings?.temperature === "cpu_core" ? "selected" : ""}>${t("cpuCoreTemperature")}</option>
              <option value="cpu_socket" ${w.superwidget_bindings?.temperature === "cpu_socket" ? "selected" : ""}>${t("cpuSocketTemperature")}</option>
            </select></label>
          ` : ""}
          <label class="wide">${t("fanSource")}<select data-i="${i}" data-binding="fan">
            <option value="" ${!w.superwidget_bindings?.fan ? "selected" : ""}>${t("automatic")}</option>
            ${availableFanSensors.map(sensor => `<option value="${esc(sensor.id)}" ${w.superwidget_bindings?.fan === sensor.id ? "selected" : ""}>${esc(sensor.name)}</option>`).join("")}
          </select></label>
          ${colourField(t("widgetBackground"), i, "superwidget_background_colour", w.superwidget_background_colour || "#000000")}
          <label>${t("backgroundAlpha")}<input data-i="${i}" data-k="superwidget_background_opacity" type="range" min="0" max="1" step="0.05" value="${w.superwidget_background_opacity ?? 0}"></label>
        ` : `
        <label>${t("type")}<select data-i="${i}" data-k="kind">${Object.keys(nameKeys).map(v => `<option value="${v}" ${w.kind === v ? "selected" : ""}>${widgetName(v)}</option>`).join("")}</select></label>
        <label>${t("visualisation")}<select data-i="${i}" data-k="render_mode">${Object.keys(modeKeys).map(v => `<option value="${v}" ${(w.render_mode || "text") === v ? "selected" : ""}>${modeName(v)}</option>`).join("")}</select></label>
        <label class="wide">${t("textFormat")}<input data-i="${i}" data-k="label_format" value="${esc(w.label_format)}" placeholder="GPU {value} MHz"></label>
        <label>${t("leftText")}<input data-i="${i}" data-k="left_text" value="${esc(w.left_text || "")}" placeholder="GPU "></label>
        <label>${t("rightText")}<input data-i="${i}" data-k="right_text" value="${esc(w.right_text || "")}" placeholder=" °C"></label>
        `}
        <label>X<input data-i="${i}" data-k="x" type="number" value="${w.x}"></label>
        <label>Y<input data-i="${i}" data-k="y" type="number" value="${w.y}"></label>
        <label>${t("width")}<input data-i="${i}" data-k="width" type="number" min="1" value="${w.width}"></label>
        <label>${t("height")}<input data-i="${i}" data-k="height" type="number" min="1" value="${w.height}"></label>
        ${w.kind === "super_widget" ? "" : `<label>${t("font")}<select data-i="${i}" data-k="font">${fonts.map(font => `<option value="${font}" ${(w.font || "Segoe UI") === font ? "selected" : ""}>${font}</option>`).join("")}</select></label>`}
        <label>${t("fontSize")}<input data-i="${i}" data-k="font_size" type="number" min="6" value="${w.font_size}"></label>
        <label>${t("interval")}<input data-i="${i}" data-k="refresh_interval_ms" type="number" min="100" value="${w.refresh_interval_ms}"></label>
        ${w.kind === "gif" ? `
          <label class="wide">${t("gifFile")}<div class="port-row"><input data-i="${i}" data-k="gif_path" value="${esc(w.gif_path || "")}" readonly><button type="button" class="secondary compact" data-select-gif="${i}">${t("chooseGif")}</button></div></label>
          <label>${t("gifFps")}<input data-i="${i}" data-k="gif_fps" type="number" min="1" max="30" value="${w.gif_fps ?? 8}"></label>
          <label class="toggle">${t("gifLoop")}<input data-i="${i}" data-k="gif_loop" type="checkbox" ${w.gif_loop !== false ? "checked" : ""}></label>
          <label>${t("gifFit")}<select data-i="${i}" data-k="gif_fit">${["contain","cover","stretch","centre"].map(mode => `<option value="${mode}" ${(w.gif_fit || "contain") === mode ? "selected" : ""}>${mode}</option>`).join("")}</select></label>
        ` : ""}
        ${w.kind === "super_widget" ? "" : colourField(t("colour"), i, "colour", w.colour)}
        ${colourField(t("gradient"), i, "secondary_colour", w.secondary_colour || w.colour)}
        <label>${t("opacity")}<input data-i="${i}" data-k="opacity" type="range" min="0.1" max="1" step="0.05" value="${w.opacity ?? 1}"></label>
        ${(w.render_mode || "text") === "graph" ? `
          ${colourField(t("graphBackground"), i, "graph_background_colour", w.graph_background_colour || "#000000")}
          <label>${t("graphBackgroundOpacity")}<input data-i="${i}" data-k="graph_background_opacity" type="range" min="0" max="1" step="0.05" value="${w.graph_background_opacity ?? 0}"></label>
        ` : ""}
        <label>${t("glow")}<input data-i="${i}" data-k="glow" type="range" min="0" max="16" value="${w.glow || 0}"></label>
        <label>${t("shadow")}<input data-i="${i}" data-k="shadow" type="range" min="0" max="16" value="${w.shadow || 0}"></label>
        <label class="toggle">${t("thresholds")}<input data-i="${i}" data-k="use_thresholds" type="checkbox" ${w.use_thresholds ? "checked" : ""}></label>
        <label>${t("warning")}<input data-i="${i}" data-k="warning_threshold" type="number" value="${w.warning_threshold ?? 70}"></label>
        <label>${t("critical")}<input data-i="${i}" data-k="critical_threshold" type="number" value="${w.critical_threshold ?? 90}"></label>
        ${colourField(t("warningColour"), i, "warning_colour", w.warning_colour || "#ffd166")}
        ${colourField(t("criticalColour"), i, "critical_colour", w.critical_colour || "#ff4d6d")}
        <label>${t("circleThickness")}<input data-i="${i}" data-k="circle_thickness" type="number" min="1" value="${w.circle_thickness ?? 16}"></label>
        <label>${t("startAngle")}<input data-i="${i}" data-k="circle_start_angle" type="number" value="${w.circle_start_angle ?? -90}"></label>
        ${w.kind === "super_widget" ? "" : `<label>${t("circleSweep")}<input data-i="${i}" data-k="circle_sweep_angle" type="number" min="1" max="360" value="${w.circle_sweep_angle ?? 360}"></label>`}
      </div>
    </div>`).join("");
  document.querySelectorAll("[data-card]").forEach(card => card.onclick = e => {
    if (e.target.closest("button,input,select,[data-collapse]")) return;
    selectedWidget = +card.dataset.card;
    selectedWidgets = new Set([selectedWidget]);
    renderWidgets(); renderOverlay();
  });
  document.querySelectorAll("[data-collapse]").forEach(heading => heading.onclick = e => {
    if (e.target.closest(".toggle")) return;
    e.preventDefault();
    e.stopPropagation();
    const index = +heading.dataset.collapse;
    if (collapsedWidgets.has(index)) collapsedWidgets.delete(index);
    else collapsedWidgets.add(index);
    renderWidgets();
  });
  document.querySelectorAll("[data-delete]").forEach(button => button.onclick = () => {
    const deleted = +button.dataset.delete;
    config.widgets.splice(deleted, 1); selectedWidget = -1; selectedWidgets.clear();
    collapsedWidgets = new Set([...collapsedWidgets]
      .filter(index => index !== deleted)
      .map(index => index > deleted ? index - 1 : index));
    renderWidgets(); renderOverlay();
  });
  document.querySelectorAll("[data-add-bar]").forEach(button => button.onclick = () => addVisualWidget(+button.dataset.addBar, "bar"));
  document.querySelectorAll("[data-add-circle]").forEach(button => button.onclick = () => addVisualWidget(+button.dataset.addCircle, "circle"));
  document.querySelectorAll("[data-add-graph]").forEach(button => button.onclick = () => addVisualWidget(+button.dataset.addGraph, "graph"));
  document.querySelectorAll("[data-select-gif]").forEach(button => button.onclick = async () => {
    const path = await invoke("select_gif");
    if (!path) return;
    const index = +button.dataset.selectGif;
    config.widgets[index].gif_path = path;
    renderWidgets();
    scheduleLivePreview();
  });
  document.querySelectorAll("[data-i][data-k]").forEach(input => {
    const update = () => {
      readWidgetInput(input);
      if (input.type === "color") syncColourPicker(input);
      renderOverlay();
      scheduleLivePreview();
      if (input.dataset.k === "render_mode") renderWidgets();
    };
    input.oninput = update;
    input.onchange = update;
  });
  document.querySelectorAll("[data-binding]").forEach(input => {
    const update = () => {
      const widget = config.widgets[+input.dataset.i];
      widget.superwidget_bindings ||= {};
      widget.superwidget_bindings[input.dataset.binding] = input.value;
      scheduleLivePreview();
    };
    input.oninput = update;
    input.onchange = update;
  });
  syncColourPickers();
}

function superWidgetName(id) {
  return availableSuperWidgets.find(item => item.id === id)?.name || t("superWidget");
}

function colourField(label, index, key, value) {
  const colour = normaliseColour(value);
  return `<label class="colour-field">${label}<div class="colour-control"><input data-i="${index}" data-k="${key}" type="color" value="${colour}"><output>${colour.toUpperCase()}</output></div></label>`;
}

function normaliseColour(value, fallback = "#ffffff") {
  const colour = String(value || "").trim();
  if (/^#[0-9a-f]{6}$/i.test(colour)) return colour.toLowerCase();
  if (/^#[0-9a-f]{3}$/i.test(colour)) {
    return `#${colour.slice(1).split("").map(char => char + char).join("")}`.toLowerCase();
  }
  return fallback;
}

function syncColourPicker(input) {
  input.value = normaliseColour(input.value);
  const output = input.closest(".colour-control")?.querySelector("output");
  if (output) output.value = input.value.toUpperCase();
}

function syncColourPickers() {
  document.querySelectorAll('input[type="color"]').forEach(syncColourPicker);
}

function setPreviewFullscreen(enabled) {
  previewFullscreen = enabled;
  document.body.classList.toggle("preview-fullscreen", enabled);
  document.querySelector(".preview-card").classList.toggle("fullscreen", enabled);
  const button = $("fullscreen-preview");
  button.textContent = t(enabled ? "exitFullscreen" : "fullscreenPreview");
  button.classList.toggle("danger", enabled);
  button.classList.toggle("secondary", !enabled);
  requestAnimationFrame(renderOverlay);
}

function collapseAllWidgets() {
  collapsedWidgets = new Set(config.widgets.map((_, index) => index));
}

function expandAllWidgets() {
  collapsedWidgets.clear();
}

function focusWidgetEditor(index) {
  selectedWidget = index;
  selectedWidgets = new Set([index]);
  collapsedWidgets.delete(index);
  if (previewFullscreen) setPreviewFullscreen(false);
  renderWidgets();
  renderOverlay();
  requestAnimationFrame(() => {
    document.querySelector(`[data-card="${index}"]`)?.scrollIntoView({
      behavior: "smooth",
      block: "center"
    });
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
    graph_background_colour: "#000000",
    graph_background_opacity: renderMode === "graph" ? 0.4 : 0,
    gif_path: null,
    gif_fps: 8,
    gif_loop: true,
    gif_fit: "contain",
    x: Math.min(config.display.width - 20, source.x + 20),
    y: Math.min(config.display.height - 20, source.y + 20),
    width: renderMode === "circle" ? 80 : 150,
    height: renderMode === "circle" ? 80 : renderMode === "graph" ? 70 : 20
  };
  config.widgets.push(visual);
  selectedWidget = config.widgets.length - 1;
  selectedWidgets = new Set([selectedWidget]);
  collapsedWidgets.delete(selectedWidget);
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
    if (previewInteractionActive) {
      previewRefreshPending = true;
      return;
    }
    try {
      const preview = await invoke("preview_config", {config});
      if (previewInteractionActive) {
        previewRefreshPending = true;
        return;
      }
      $("preview").src = preview;
      $("error").textContent = "";
    } catch (error) {
      $("error").textContent = `${t("previewError")}: ${error}`;
    }
  }, 160);
}

async function refreshGifPreview() {
  if (gifPreviewBusy || previewInteractionActive || document.hidden || !config) return;
  const animated = config.widgets.some(widget =>
    widget.enabled && widget.kind === "gif" && widget.gif_path);
  if (!animated) return;
  gifPreviewBusy = true;
  try {
    $("preview").src = await invoke("preview_config", {config});
  } catch (error) {
    $("error").textContent = `${t("previewError")}: ${error}`;
  } finally {
    gifPreviewBusy = false;
  }
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
      width: `${Math.max(w.width*sx,30)}px`, height: `${Math.max(w.height*sy,18)}px`,
      zIndex: selectedWidget === i ? "1002" : selectedWidgets.has(i) ? "1001" : String(i + 1)
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
      collapsedWidgets.delete(i);
      updateOverlayStacking();
      startDrag(event);
    };
    el.ondblclick = event => {
      event.preventDefault();
      event.stopPropagation();
      focusWidgetEditor(i);
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

function updateOverlayStacking() {
  const layer = $("widget-overlay");
  if (!layer) return;
  layer.querySelectorAll(".widget-handle").forEach(handle => {
    const index = +handle.dataset.index;
    const selected = selectedWidgets.has(index) || selectedWidget === index;
    handle.classList.toggle("selected", selected);
    handle.style.zIndex = selectedWidget === index
      ? "1002"
      : selectedWidgets.has(index) ? "1001" : String(index + 1);
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
  let moved = false;
  const box = document.createElement("div");
  box.className = "selection-box";
  layer.appendChild(box);
  layer.setPointerCapture(event.pointerId);

  const move = e => {
    moved ||= Math.abs(e.clientX - event.clientX) > 3 || Math.abs(e.clientY - event.clientY) > 3;
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
    updateOverlayStacking();
  };
  const end = () => {
    layer.onpointermove = layer.onpointerup = layer.onpointercancel = null;
    box.remove();
    if (!moved && !(event.ctrlKey || event.metaKey)) {
      selectedWidget = -1;
      selectedWidgets.clear();
    }
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
  previewInteractionActive = true;
  const w = config.widgets[index];
  const rect = $("widget-overlay").getBoundingClientRect();
  const startX = e.clientX, startY = e.clientY, startWidth = w.width, startHeight = w.height;
  selectedWidget = index;
  selectedWidgets = new Set([index]);
  updateOverlayStacking();
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
    previewInteractionActive = false;
    renderWidgets();
    await invoke("save_config", {config});
    previewRefreshPending = false;
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
  updateOverlayStacking();
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
  config.cpu_clock_source = $("cpuClockSource").value;
  config.fan_sensor = $("fanSensor").value || null;
  config.automation.enabled = $("automationEnabled").checked;
  config.automation.default_screen = $("defaultScreen").value || null;
  config.transition.kind = $("transitionKind").value;
  config.transition.duration_ms = Math.max(100, Math.min(3000, +$("transitionDuration").value || 450));
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
  if ($("autostart").checked !== autostartEnabled) {
    await invoke("set_autostart", {enabled:$("autostart").checked});
    autostartEnabled = $("autostart").checked;
  }
  bindConfig(); await refreshPreview(); $("status").textContent = t("configurationSaved");
}
async function refreshPreview() {
  if (previewInteractionActive) {
    previewRefreshPending = true;
    return;
  }
  try {
    $("error").textContent="";
    const preview = await invoke("get_preview");
    if (previewInteractionActive) {
      previewRefreshPending = true;
      return;
    }
    $("preview").src=preview;
    previewRefreshPending = false;
    requestAnimationFrame(renderOverlay);
  }
  catch(e) { $("error").textContent=String(e); }
}
async function refreshStatus() {
  const s=await invoke("get_status");
  renderingActive = s.running;
  if (!renderingBusy) updateRenderButton();
  const known={"Stopped":"statusStopped","Rendering active":"statusActive","Frame sent":"statusFrameSent"};
  $("status").textContent=known[s.message]?t(known[s.message]):s.message;
  if(s.running && !document.hidden) refreshPreview();
}

function updateRenderButton() {
  const button = $("render-toggle");
  if (!button) return;
  button.disabled = renderingBusy;
  button.classList.toggle("running", renderingActive);
  button.textContent = t(renderingBusy
    ? (renderingActive ? "stoppingRendering" : "startingRendering")
    : (renderingActive ? "stopRendering" : "startRendering"));
}

async function toggleRendering() {
  if (renderingBusy) return;
  renderingBusy = true;
  updateRenderButton();
  try {
    if (renderingActive) {
      await invoke("stop_rendering");
      renderingActive = false;
      $("status").textContent = t("stopRequested");
    } else {
      await save();
      await invoke("start_rendering");
      renderingActive = true;
    }
    $("error").textContent = "";
  } catch (error) {
    $("error").textContent = renderingActive
      ? String(error)
      : `${t("renderStartFailed")}: ${error}`;
  } finally {
    renderingBusy = false;
    await refreshStatus().catch(updateRenderButton);
  }
}
const askName=(message,initial="")=>window.prompt(message,initial)?.trim()||"";

$("choose-bg").onclick=async()=>{const p=await invoke("select_background");if(p){config.background.path=p;config.background.source="file";$("backgroundPath").value=p;$("backgroundSource").value="file";await save();}};
$("choose-bg-folder").onclick=async()=>{const p=await invoke("select_background_folder");if(p){config.background.folder=p;config.background.source="folder";$("backgroundFolder").value=p;$("backgroundSource").value="folder";await save();}};
$("backgroundSource").onchange=()=>{readForm();scheduleLivePreview();};
$("slideshowInterval").oninput=()=>{readForm();scheduleLivePreview();};
["backgroundColour","foreground","accent"].forEach(id => {
  $(id).oninput = () => {
    syncColourPicker($(id));
    readForm();
    scheduleLivePreview();
  };
});
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
$("add-rule").onclick=()=>{
  config.automation.rules.push({enabled:true,kind:"process_running",process_name:"",threshold:80,idle_seconds:300,sustain_seconds:3,release_seconds:8,screen:""});
  renderRules();
};
$("fullscreen-preview").onclick=()=>setPreviewFullscreen(!previewFullscreen);
$("collapse-widgets").onclick=()=>{collapseAllWidgets();renderWidgets();};
$("expand-widgets").onclick=()=>{expandAllWidgets();renderWidgets();};
$("add-widget").onclick=()=>{
  const selection=$("new-widget-kind").value;
  if(selection.startsWith("super:")){
    const item=availableSuperWidgets.find(value=>value.id===selection.slice(6));
    if(!item)return;
    config.widgets.push({kind:"super_widget",superwidget_id:item.id,superwidget_background_colour:"#000000",superwidget_background_opacity:0,superwidget_bindings:{},render_mode:"text",enabled:true,left_text:"",right_text:"",x:20,y:20,width:item.width,height:item.height,font:"Bahnschrift",font_size:24,colour:"#ffffff",secondary_colour:"#1f99ff",opacity:1,graph_background_colour:"#000000",graph_background_opacity:0,gif_path:null,gif_fps:8,gif_loop:true,gif_fit:"contain",glow:0,shadow:0,use_thresholds:false,warning_threshold:70,critical_threshold:90,warning_colour:"#ffd166",critical_colour:"#ff4d6d",circle_thickness:16,circle_start_angle:-90,circle_sweep_angle:360,refresh_interval_ms:500,label_format:""});
  }else{
    const kind=selection;const affix=defaultAffixes[kind]||{left:"",right:""};
    config.widgets.push({kind,superwidget_id:null,superwidget_background_colour:"#000000",superwidget_background_opacity:0,superwidget_bindings:{},render_mode:"text",enabled:true,left_text:affix.left,right_text:affix.right,x:20,y:20,width:kind==="gif"?96:180,height:kind==="gif"?96:42,font:"Segoe UI",font_size:24,colour:config.theme.foreground,secondary_colour:config.theme.accent,opacity:1,graph_background_colour:"#000000",graph_background_opacity:0,gif_path:null,gif_fps:8,gif_loop:true,gif_fit:"contain",glow:0,shadow:0,use_thresholds:false,warning_threshold:70,critical_threshold:90,warning_colour:"#ffd166",critical_colour:"#ff4d6d",circle_thickness:16,circle_start_angle:-90,circle_sweep_angle:360,refresh_interval_ms:1000,label_format:defaults[kind]});
  }
  selectedWidget=config.widgets.length-1;selectedWidgets=new Set([selectedWidget]);collapsedWidgets.delete(selectedWidget);renderWidgets();renderOverlay();scheduleLivePreview();
};
$("new-screen").onclick=async()=>{const name=askName(t("selectNewScreen"));if(!name)return;try{config=await invoke("new_screen",{name});currentScreen=name;collapseAllWidgets();bindConfig();await loadScreens(name);await refreshPreview();}catch(e){$("error").textContent=String(e);}};
$("save-screen").onclick=async()=>{
  readForm();
  const name=currentScreen||askName(t("saveScreenName"));
  if(!name)return;
  try{
    await invoke("save_screen",{name,config});
    currentScreen=name;
    await loadScreens(name);
    $("status").textContent=t("screenSaved",{name});
  }catch(e){$("error").textContent=String(e);}
};
$("load-screen").onclick=async()=>{const name=$("screen-list").value;if(!name)return;config=await invoke("load_screen",{name});currentScreen=name;selectedWidget=-1;selectedWidgets.clear();collapseAllWidgets();bindConfig();await refreshPreview();};
$("delete-screen").onclick=async()=>{const name=$("screen-list").value;if(!name||!confirm(t("deleteConfirm",{name})))return;await invoke("delete_screen",{name});if(currentScreen===name)currentScreen="";await loadScreens();};
$("export-package").onclick=async()=>{try{readForm();await invoke("export_package",{config});}catch(e){$("error").textContent=String(e);}};
$("import-package").onclick=async()=>{try{const imported=await invoke("import_package");if(!imported)return;config=imported;currentScreen="";collapseAllWidgets();bindConfig();await refreshPreview();}catch(e){$("error").textContent=String(e);}};
document.querySelectorAll("[data-quick-load]").forEach(button => button.onclick=async()=>{
  const slot=button.dataset.quickLoad;
  const name=$(`quick-${slot}`).value;
  if(!name)return;
  config=await invoke("load_screen",{name});
  currentScreen=name;
  collapseAllWidgets();
  bindConfig();
  await loadScreens(name);
  await refreshPreview();
});
$("render-toggle").onclick=toggleRendering;
$("test-display").onclick=async()=>{try{await save();$("status").textContent=await invoke("test_display");}catch(e){$("error").textContent=`${t("testFailed")}: ${e}`;}};
$("test-sensors").onclick=async()=>{try{const s=await invoke("test_sensors");await loadFanSensors(s);$("error").textContent="";$("status").textContent=`CPU ${fmt(s.cpu_temperature)}°C / ${fmt(s.cpu_clock)} MHz · GPU ${fmt(s.gpu_temperature)}°C / ${fmt(s.gpu_usage)}% / ${fmt(s.gpu_clock)} MHz / ${fmt(s.gpu_power)} W · RAM ${fmt(s.ram_usage)}% · VOL ${fmt(s.system_volume)}% · ${t("diskLabel")} ${fmt(s.disk_usage)}% · ${t("networkLabel")} ↓${fmt(s.network_download)} ↑${fmt(s.network_upload)} KB/s`;}catch(e){$("error").textContent=`${t("sensorTestFailed")}: ${e}`;}};
$("send-once").onclick=async()=>{try{await save();await invoke("render_once");}catch(e){$("error").textContent=`${t("sendFailed")}: ${e}`;}};
window.addEventListener("resize",renderOverlay);
window.addEventListener("keydown", event => {
  if (event.key === "Escape" && previewFullscreen) setPreviewFullscreen(false);
});
document.addEventListener("pointerdown", event => {
  if (!event.target.closest("#object-menu")) hideObjectMenu();
});
document.querySelectorAll("[data-layout-action]").forEach(button =>
  button.onclick = () => applyLayout(button.dataset.layoutAction));
window.addEventListener("turzx-language-changed", refreshTranslatedUi);
function esc(v){return String(v).replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));}
function fmt(v){return v==null?"--":Math.round(v);}
boot().catch(e=>$("error").textContent=String(e));
