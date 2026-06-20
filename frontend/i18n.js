window.TurzxI18n = (() => {
  const translations = {
    en: {
      language:"Language",loading:"Loading…",startRendering:"Start rendering",livePreview:"Live preview",
      testSensors:"Test sensors",sendFrame:"Send frame",previewAlt:"Display preview",
      editorHint:"Drag to move. Drag on the background to select multiple objects, or use Ctrl+click. Right-click to align.",
      currentConfiguration:"Current configuration",selectScreen:"Select a screen…",load:"Load",saveAs:"Save as",
      new:"New",delete:"Delete",comPort:"COM port",comPlaceholder:"AUTO or COM3",refreshPorts:"Refresh COM ports",
      orientation:"Orientation",landscape:"Landscape",reverseLandscape:"Reverse landscape",portrait:"Portrait",
      reversePortrait:"Reverse portrait",width:"Width",height:"Height",brightness:"Brightness",
      frameInterval:"Frame interval (ms)",startWithWindows:"Start with Windows",backgroundTheme:"Background and theme",
      cpuTemperatureSource:"CPU temperature source",cpuCoreTemperature:"Core / Tctl-Tdie",
      cpuSocketTemperature:"CPU socket",
      neonSample:"Neon sample",noImage:"No image selected",chooseImage:"Choose image",fitMode:"Fit mode",
      backgroundColour:"Background colour",textColour:"Text colour",accentColour:"Accent colour",add:"+ Add",
      backgroundSource:"Background source",solidColour:"Solid colour",singleFile:"Single file",
      slideshowFolder:"Slideshow folder",slideshowInterval:"Change every (minutes)",
      noFolder:"No folder selected",chooseFolder:"Choose folder",
      saveConfiguration:"Save configuration",alignColumn:"Align in column",verticalSpacing:"Distribute vertically",
      alignDistribute:"Align and distribute",automatic:"Automatic",screenCurrent:"Current configuration",
      widgetCpuTemp:"CPU temperature",widgetCpuUsage:"CPU usage",widgetGpuTemp:"GPU temperature",
      widgetGpuUsage:"GPU usage",widgetGpuClock:"GPU clock",widgetRam:"RAM usage",widgetVram:"VRAM usage",
      widgetDisk:"Disk usage",widgetUpload:"Network upload",widgetDownload:"Network download",
      widgetFan:"Fan speed",widgetClock:"Clock",widgetDate:"Date",widgetFps:"FPS",widgetText:"Free text",
      modeText:"Text",modeBar:"Bar",modeCircle:"Circle",modeGraph:"Graph",remove:"Remove",addBar:"+ Bar",
      addCircle:"+ Circle",addGraph:"+ Graph",type:"Type",visualisation:"Visualisation",textFormat:"Text / format",
      leftText:"Left text",rightText:"Right text",font:"Font",fontSize:"Font size",interval:"Interval (ms)",
      colour:"Colour",gradient:"Gradient",opacity:"Opacity",glow:"Glow",shadow:"Shadow",thresholds:"Thresholds",
      warning:"Warning",critical:"Critical",warningColour:"Warning colour",criticalColour:"Critical colour",
      circleThickness:"Circle thickness",startAngle:"Start angle",circleSweep:"Circle sweep",configurationSaved:"Configuration saved",
      selectNewScreen:"New screen name:",saveScreenName:"Save screen as:",screenSaved:"Screen “{name}” saved",
      deleteConfirm:"Delete screen “{name}”?",stopRequested:"Stop requested",renderStartFailed:"Could not start rendering",
      testFailed:"Display test failed",sensorTestFailed:"Sensor test failed",sendFailed:"Could not send frame",
      presetApplied:"{name} mode applied",previewError:"Preview error",bridgeUnavailable:"Tauri bridge is unavailable.",
      diskLabel:"Disk",networkLabel:"Network",statusStopped:"Stopped",statusActive:"Rendering active",statusFrameSent:"Frame sent"
    },
    it: {
      language:"Lingua",loading:"Caricamento…",startRendering:"Avvia rendering",livePreview:"Anteprima live",
      testSensors:"Test sensori",sendFrame:"Invia frame",previewAlt:"Anteprima display",
      editorHint:"Trascina per spostare. Trascina sullo sfondo per selezionare più oggetti, oppure usa Ctrl+clic. Tasto destro per allineare.",
      currentConfiguration:"Configurazione corrente",selectScreen:"Seleziona uno screen…",load:"Carica",saveAs:"Salva come",
      new:"Nuovo",delete:"Elimina",comPort:"Porta COM",comPlaceholder:"AUTO oppure COM3",refreshPorts:"Aggiorna porte COM",
      orientation:"Orientamento",landscape:"Orizzontale",reverseLandscape:"Orizzontale invertito",portrait:"Verticale",
      reversePortrait:"Verticale invertito",width:"Larghezza",height:"Altezza",brightness:"Luminosità",
      frameInterval:"Intervallo frame (ms)",startWithWindows:"Avvia con Windows",backgroundTheme:"Sfondo e tema",
      cpuTemperatureSource:"Sorgente temperatura CPU",cpuCoreTemperature:"Core / Tctl-Tdie",
      cpuSocketTemperature:"Socket CPU",
      neonSample:"Sample neon",noImage:"Nessuna immagine selezionata",chooseImage:"Scegli immagine",fitMode:"Adattamento",
      backgroundColour:"Colore sfondo",textColour:"Colore testo",accentColour:"Colore accento",add:"+ Aggiungi",
      backgroundSource:"Sorgente sfondo",solidColour:"Colore uniforme",singleFile:"File singolo",
      slideshowFolder:"Cartella slideshow",slideshowInterval:"Cambia ogni (minuti)",
      noFolder:"Nessuna cartella selezionata",chooseFolder:"Scegli cartella",
      saveConfiguration:"Salva configurazione",alignColumn:"Allinea in colonna",verticalSpacing:"Spaziatura verticale uniforme",
      alignDistribute:"Allinea e distribuisci",automatic:"Automatico",screenCurrent:"Configurazione corrente",
      widgetCpuTemp:"Temperatura CPU",widgetCpuUsage:"Utilizzo CPU",widgetGpuTemp:"Temperatura GPU",
      widgetGpuUsage:"Utilizzo GPU",widgetGpuClock:"Clock GPU",widgetRam:"Utilizzo RAM",widgetVram:"Utilizzo VRAM",
      widgetDisk:"Utilizzo disco",widgetUpload:"Upload rete",widgetDownload:"Download rete",
      widgetFan:"Velocità ventola",widgetClock:"Orologio",widgetDate:"Data",widgetFps:"FPS",widgetText:"Testo libero",
      modeText:"Testo",modeBar:"Barra",modeCircle:"Cerchio",modeGraph:"Grafico",remove:"Rimuovi",addBar:"+ Barra",
      addCircle:"+ Cerchio",addGraph:"+ Grafico",type:"Tipo",visualisation:"Visualizzazione",textFormat:"Testo / formato",
      leftText:"Testo sinistro",rightText:"Testo destro",font:"Font",fontSize:"Dimensione font",interval:"Intervallo (ms)",
      colour:"Colore",gradient:"Gradiente",opacity:"Opacità",glow:"Glow",shadow:"Ombra",thresholds:"Soglie",
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
