(() => {
  if (window.__TAURI__?.core?.invoke) return;

  window.TelemetryForgeRemote = true;
  window.__TAURI__ = {
    core: {
      invoke: async (command, arguments_ = {}) => {
        const response = await fetch(`/api/invoke/${encodeURIComponent(command)}`, {
          method: "POST",
          headers: {"Content-Type": "application/json"},
          body: JSON.stringify(arguments_ || {})
        });
        const payload = await response.json();
        if (!payload.ok) throw new Error(payload.error || `Remote command failed: ${command}`);
        return payload.result;
      }
    }
  };

  window.addEventListener("DOMContentLoaded", () => {
    document.body.classList.add("remote-deck");
    const header = document.querySelector("header > div");
    if (header) {
      const badge = document.createElement("p");
      badge.id = "remote-badge";
      badge.className = "remote-badge";
      badge.textContent = "REMOTE DECK · LAN ONLY";
      header.appendChild(badge);
    }
    ["choose-bg", "choose-bg-folder", "import-package", "export-package", "import-superwidget"]
      .forEach(id => {
        const element = document.getElementById(id);
        if (!element) return;
        element.disabled = true;
        element.title = "File upload support will be added in the next Remote Deck phase.";
      });
  });
})();
