(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const emit = (event, payload) => invoke("plugin:event|emit", { event, payload });
  const waitFor = async (read, description) => {
    await emit("qualification-step", description);
    for (let attempt = 0; attempt < 25; attempt++) {
      const value = read();
      if (value) return value;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    throw new Error(description);
  };
  try {
    await waitFor(() => document.querySelector("footer")?.textContent.includes("Backend ready"), "Backend readiness");
    if ("__PHASE__" === "import") {
      await emit("tauri://drag-drop", {paths: ["/usr/share/snmp/mibs/SNMPv2-MIB.txt"], position: {x: 20, y: 80}});
    }
    const module = await waitFor(() => [...document.querySelectorAll(".module-row")].find(row => row.querySelector("strong")?.textContent === "SNMPv2-MIB"), "Workspace import or restart");
    const findButton = text => [...document.querySelectorAll("button")].find(button => button.textContent === text);
    findButton("Reload").click();
    await waitFor(() => !findButton("Reload").disabled && document.querySelector(".module-footer").textContent.includes("OID nodes loaded"), "Reload");
    window.dispatchEvent(new KeyboardEvent("keydown", {key: "f", metaKey: true}));
    const input = document.activeElement;
    if (input.placeholder !== "Search name, OID, description") throw new Error("Search shortcut");
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, "sysLocation");
    input.dispatchEvent(new Event("input", {bubbles: true}));
    const hit = await waitFor(() => [...document.querySelectorAll(".tree-row")].find(row => row.textContent.startsWith("sysLocation ·")), "Name lookup");
    hit.click();
    await waitFor(() => document.querySelector(".details-content h2")?.textContent === "sysLocation", "Object details for " + hit.textContent);
    module.click();
    await waitFor(() => document.querySelector(".tree-scroll")?.textContent.includes("No matching objects"), "Module disabled");
    module.click();
    await waitFor(() => document.querySelector(".tree-scroll")?.textContent.includes("sysLocation"), "Module enabled");
    const fixture = __FIXTURE__;
    const name = "loopback-fixture";
    if ("__PHASE__" === "import") {
      await invoke("save_profile", {settings: {name, host: "127.0.0.1", port: fixture.port, retries: 0, timeoutMs: 2000},
        credentials: {version: "v2c", community: fixture.community}});
    }
    document.querySelector('[aria-label="Settings"]').click();
    const picker = await waitFor(() => [...document.querySelectorAll(".query-settings select")].find(el => [...el.options].some(option => option.value === name)), "Saved profile");
    picker.value = name;
    picker.dispatchEvent(new Event("change", {bubbles: true}));
    await waitFor(() => document.querySelector('[aria-label="Community"]').value === fixture.community, "Keychain credentials");
    document.querySelector('[aria-label="Settings"]').click();
    const rootInput = document.querySelector('[aria-label="Query OID"]');
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(rootInput, "1.3.6.1.2.1.1");
    rootInput.dispatchEvent(new Event("input", {bubbles: true}));
    for (const tab of ["Get next", "Walk", "Bulk walk"]) {
      findButton(tab).click();
      await waitFor(() => findButton(" Run " + tab) || [...document.querySelectorAll(".run-button")].find(b => b.textContent.trim() === "Run " + tab), "Select " + tab);
      document.querySelector(".run-button").click();
      await waitFor(() => !document.querySelector(".run-button").disabled && document.querySelector(".result-summary").textContent.includes("results"), "Desktop " + tab);
      if (!document.querySelector(".results-table .type-value")) throw new Error("Missing binding");
    }
    findButton("History").click();
    await waitFor(() => document.querySelector('[aria-label="Query history"]')?.textContent.includes("bulkWalk"), "Query history");
    await emit("qualification-result", "passed");
  } catch (error) {
    await emit("qualification-result", String(error));
  }
})();
