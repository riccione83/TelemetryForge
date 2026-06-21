window.TurzxI18n = (() => {
  const translations = {
    en: {
      language:"Language",loading:"Loading…",startRendering:"Start rendering",stopRendering:"Stop rendering",
      startingRendering:"Starting…",stoppingRendering:"Stopping…",livePreview:"Live preview",
      testSensors:"Test sensors",sendFrame:"Send frame",previewAlt:"Display preview",
      fullscreenPreview:"Full screen",exitFullscreen:"Exit full screen",
      editorHint:"Drag to move. Drag on the background to select multiple objects, or use Ctrl+click. Right-click to align.",
      currentConfiguration:"Current configuration",selectScreen:"Select a screen…",load:"Load",saveAs:"Save as",
      new:"New",delete:"Delete",importPackage:"Import package",exportPackage:"Export package",
      quickScreens:"Quick screens",comPort:"COM port",comPlaceholder:"AUTO or COM3",refreshPorts:"Refresh COM ports",
      automation:"Automation",addRule:"+ Add rule",enableAutomation:"Enable automatic screen switching",
      defaultScreen:"Default screen",transition:"Transition",transitionDuration:"Duration (ms)",
      transitionNone:"None",transitionFade:"Fade",transitionSlide:"Slide",transitionDissolve:"Dissolve",transitionGlitch:"Glitch",
      automationHint:"Rules are evaluated from top to bottom. The first matching rule wins.",
      rule:"Rule",condition:"Condition",processName:"Process name",idleSeconds:"Idle time (seconds)",
      temperatureThreshold:"Temperature threshold (°C)",usageThreshold:"Usage threshold (%)",
      sustainSeconds:"Active for (seconds)",releaseSeconds:"Return delay (seconds)",
      targetScreen:"Target screen",noRules:"No automatic rules configured.",
      moveUp:"Move up",moveDown:"Move down",
      rule_process_running:"Process is running",rule_gpu_temperature_above:"GPU temperature above",
      rule_cpu_temperature_above:"CPU temperature above",rule_gpu_usage_above:"GPU usage above",
      rule_cpu_usage_above:"CPU usage above",rule_idle_for:"PC idle for",
      orientation:"Orientation",landscape:"Landscape",reverseLandscape:"Reverse landscape",portrait:"Portrait",
      reversePortrait:"Reverse portrait",width:"Width",height:"Height",brightness:"Brightness",
      frameInterval:"Frame interval (ms)",startWithWindows:"Start with Windows",backgroundTheme:"Background and theme",
      cpuTemperatureSource:"CPU temperature source",cpuCoreTemperature:"Core / Tctl-Tdie",
      cpuSocketTemperature:"CPU socket",cpuClockSource:"CPU clock source",
      cpuClockAverage:"Average clock",cpuClockEffective:"Average effective clock",fanSensor:"Fan sensor",
      noImage:"No image selected",chooseImage:"Choose image",fitMode:"Fit mode",
      backgroundColour:"Background colour",textColour:"Text colour",accentColour:"Accent colour",add:"+ Add",
      backgroundSource:"Background source",solidColour:"Solid colour",singleFile:"Single file",
      slideshowFolder:"Slideshow folder",slideshowInterval:"Change every (minutes)",
      noFolder:"No folder selected",chooseFolder:"Choose folder",
      saveConfiguration:"Save configuration",alignColumn:"Align in column",verticalSpacing:"Distribute vertically",
      alignDistribute:"Align and distribute",automatic:"Automatic",screenCurrent:"Current configuration",
      widgetCpuTemp:"CPU temperature",widgetCpuUsage:"CPU usage",widgetGpuTemp:"GPU temperature",
      widgetCpuClock:"CPU clock",
      widgetGpuUsage:"GPU usage",widgetGpuClock:"GPU clock",widgetRam:"RAM usage",widgetVram:"VRAM usage",
      widgetGpuPower:"GPU power",
      widgetDisk:"Disk usage",widgetUpload:"Network upload",widgetDownload:"Network download",
      widgetFan:"Fan speed",widgetClock:"Clock",widgetDate:"Date",widgetFps:"FPS",widgetText:"Free text",widgetGif:"Animated GIF",
      widgetVolume:"System volume",
      modeText:"Text",modeBar:"Bar",modeCircle:"Circle",modeGraph:"Graph",remove:"Remove",addBar:"+ Bar",
      collapse:"Collapse",expand:"Expand",collapseAll:"Collapse all",expandAll:"Expand all",
      addCircle:"+ Circle",addGraph:"+ Graph",type:"Type",visualisation:"Visualisation",textFormat:"Text / format",
      leftText:"Left text",rightText:"Right text",font:"Font",fontSize:"Font size",interval:"Interval (ms)",
      colour:"Colour",gradient:"Gradient",opacity:"Opacity",glow:"Glow",shadow:"Shadow",thresholds:"Thresholds",
      graphBackground:"Graph background",graphBackgroundOpacity:"Background alpha",
      gifFile:"GIF file",chooseGif:"Choose GIF",gifFps:"GIF FPS",gifLoop:"Loop GIF",gifFit:"GIF fit",
      warning:"Warning",critical:"Critical",warningColour:"Warning colour",criticalColour:"Critical colour",
      circleThickness:"Circle thickness",startAngle:"Start angle",circleSweep:"Circle sweep",configurationSaved:"Configuration saved",
      selectNewScreen:"New screen name:",saveScreenName:"Save screen as:",screenSaved:"Screen “{name}” saved",
      deleteConfirm:"Delete screen “{name}”?",stopRequested:"Stop requested",renderStartFailed:"Could not start rendering",
      testFailed:"Display test failed",sensorTestFailed:"Sensor test failed",sendFailed:"Could not send frame",
      presetApplied:"{name} mode applied",previewError:"Preview error",bridgeUnavailable:"Tauri bridge is unavailable.",
      diskLabel:"Disk",networkLabel:"Network",statusStopped:"Stopped",statusActive:"Rendering active",statusFrameSent:"Frame sent"
    },
    it: {
      language:"Lingua",loading:"Caricamento…",startRendering:"Avvia rendering",stopRendering:"Ferma rendering",
      startingRendering:"Avvio…",stoppingRendering:"Arresto…",livePreview:"Anteprima live",
      testSensors:"Test sensori",sendFrame:"Invia frame",previewAlt:"Anteprima display",
      fullscreenPreview:"Schermo intero",exitFullscreen:"Esci da schermo intero",
      editorHint:"Trascina per spostare. Trascina sullo sfondo per selezionare più oggetti, oppure usa Ctrl+clic. Tasto destro per allineare.",
      currentConfiguration:"Configurazione corrente",selectScreen:"Seleziona uno screen…",load:"Carica",saveAs:"Salva come",
      new:"Nuovo",delete:"Elimina",importPackage:"Importa pacchetto",exportPackage:"Esporta pacchetto",
      quickScreens:"Screen rapidi",comPort:"Porta COM",comPlaceholder:"AUTO oppure COM3",refreshPorts:"Aggiorna porte COM",
      automation:"Automazione",addRule:"+ Aggiungi regola",enableAutomation:"Abilita cambio screen automatico",
      defaultScreen:"Screen predefinito",transition:"Transizione",transitionDuration:"Durata (ms)",
      transitionNone:"Nessuna",transitionFade:"Dissolvenza",transitionSlide:"Scorrimento",transitionDissolve:"Dissoluzione",transitionGlitch:"Glitch",
      automationHint:"Le regole vengono valutate dall’alto verso il basso. Vince la prima corrispondente.",
      rule:"Regola",condition:"Condizione",processName:"Nome processo",idleSeconds:"Tempo inattivo (secondi)",
      temperatureThreshold:"Soglia temperatura (°C)",usageThreshold:"Soglia utilizzo (%)",
      sustainSeconds:"Attiva per (secondi)",releaseSeconds:"Ritardo ritorno (secondi)",
      targetScreen:"Screen destinazione",noRules:"Nessuna regola automatica configurata.",
      moveUp:"Sposta in alto",moveDown:"Sposta in basso",
      rule_process_running:"Processo in esecuzione",rule_gpu_temperature_above:"Temperatura GPU superiore a",
      rule_cpu_temperature_above:"Temperatura CPU superiore a",rule_gpu_usage_above:"Utilizzo GPU superiore a",
      rule_cpu_usage_above:"Utilizzo CPU superiore a",rule_idle_for:"PC inattivo da",
      orientation:"Orientamento",landscape:"Orizzontale",reverseLandscape:"Orizzontale invertito",portrait:"Verticale",
      reversePortrait:"Verticale invertito",width:"Larghezza",height:"Altezza",brightness:"Luminosità",
      frameInterval:"Intervallo frame (ms)",startWithWindows:"Avvia con Windows",backgroundTheme:"Sfondo e tema",
      cpuTemperatureSource:"Sorgente temperatura CPU",cpuCoreTemperature:"Core / Tctl-Tdie",
      cpuSocketTemperature:"Socket CPU",cpuClockSource:"Sorgente clock CPU",
      cpuClockAverage:"Clock medio",cpuClockEffective:"Clock medio effettivo",fanSensor:"Sensore ventola",
      noImage:"Nessuna immagine selezionata",chooseImage:"Scegli immagine",fitMode:"Adattamento",
      backgroundColour:"Colore sfondo",textColour:"Colore testo",accentColour:"Colore accento",add:"+ Aggiungi",
      backgroundSource:"Sorgente sfondo",solidColour:"Colore uniforme",singleFile:"File singolo",
      slideshowFolder:"Cartella slideshow",slideshowInterval:"Cambia ogni (minuti)",
      noFolder:"Nessuna cartella selezionata",chooseFolder:"Scegli cartella",
      saveConfiguration:"Salva configurazione",alignColumn:"Allinea in colonna",verticalSpacing:"Spaziatura verticale uniforme",
      alignDistribute:"Allinea e distribuisci",automatic:"Automatico",screenCurrent:"Configurazione corrente",
      widgetCpuTemp:"Temperatura CPU",widgetCpuUsage:"Utilizzo CPU",widgetGpuTemp:"Temperatura GPU",
      widgetCpuClock:"Clock CPU",
      widgetGpuUsage:"Utilizzo GPU",widgetGpuClock:"Clock GPU",widgetRam:"Utilizzo RAM",widgetVram:"Utilizzo VRAM",
      widgetGpuPower:"Potenza GPU",
      widgetDisk:"Utilizzo disco",widgetUpload:"Upload rete",widgetDownload:"Download rete",
      widgetFan:"Velocità ventola",widgetClock:"Orologio",widgetDate:"Data",widgetFps:"FPS",widgetText:"Testo libero",widgetGif:"GIF animata",
      widgetVolume:"Volume di sistema",
      modeText:"Testo",modeBar:"Barra",modeCircle:"Cerchio",modeGraph:"Grafico",remove:"Rimuovi",addBar:"+ Barra",
      collapse:"Comprimi",expand:"Espandi",collapseAll:"Comprimi tutti",expandAll:"Espandi tutti",
      addCircle:"+ Cerchio",addGraph:"+ Grafico",type:"Tipo",visualisation:"Visualizzazione",textFormat:"Testo / formato",
      leftText:"Testo sinistro",rightText:"Testo destro",font:"Font",fontSize:"Dimensione font",interval:"Intervallo (ms)",
      colour:"Colore",gradient:"Gradiente",opacity:"Opacità",glow:"Glow",shadow:"Ombra",thresholds:"Soglie",
      graphBackground:"Sfondo grafico",graphBackgroundOpacity:"Alpha sfondo",
      gifFile:"File GIF",chooseGif:"Scegli GIF",gifFps:"FPS GIF",gifLoop:"Ripeti GIF",gifFit:"Adattamento GIF",
      warning:"Avviso",critical:"Critico",warningColour:"Colore avviso",criticalColour:"Colore critico",
      circleThickness:"Spessore cerchio",startAngle:"Angolo iniziale",circleSweep:"Ampiezza cerchio",configurationSaved:"Configurazione salvata",
      selectNewScreen:"Nome del nuovo screen:",saveScreenName:"Salva lo screen con nome:",screenSaved:"Screen “{name}” salvato",
      deleteConfirm:"Eliminare lo screen “{name}”?",stopRequested:"Arresto richiesto",renderStartFailed:"Avvio rendering fallito",
      testFailed:"Test display fallito",sensorTestFailed:"Test sensori fallito",sendFailed:"Invio frame fallito",
      presetApplied:"Modalità {name} applicata",previewError:"Errore anteprima",bridgeUnavailable:"Bridge Tauri non disponibile.",
      diskLabel:"Disco",networkLabel:"Rete",statusStopped:"Fermo",statusActive:"Rendering attivo",statusFrameSent:"Frame inviato"
    }
  };
  let language = localStorage.getItem("turzx-language") || "en";
  if (!translations[language]) language = "en";
  const t = (key, vars={}) => {
    let value = translations[language][key] || translations.en[key] || key;
    for (const [name,replacement] of Object.entries(vars)) value=value.replace(`{${name}}`,replacement);
    return value;
  };
  const apply = () => {
    document.documentElement.lang=language;
    document.querySelectorAll("[data-i18n]").forEach(el=>el.textContent=t(el.dataset.i18n));
    document.querySelectorAll("[data-i18n-placeholder]").forEach(el=>el.placeholder=t(el.dataset.i18nPlaceholder));
    document.querySelectorAll("[data-i18n-title]").forEach(el=>el.title=t(el.dataset.i18nTitle));
    document.querySelectorAll("[data-i18n-alt]").forEach(el=>el.alt=t(el.dataset.i18nAlt));
    const picker=document.getElementById("language");if(picker)picker.value=language;
  };
  const setLanguage = value => { language=translations[value]?value:"en";localStorage.setItem("turzx-language",language);apply();window.dispatchEvent(new CustomEvent("turzx-language-changed")); };
  document.addEventListener("DOMContentLoaded",()=>{apply();document.getElementById("language")?.addEventListener("change",e=>setLanguage(e.target.value));});
  return {t,apply,setLanguage,get language(){return language;}};
})();
