// BioGist — Firefox Background Script
// Firefox uses sidebar_action instead of sidePanel.
// This is a thin wrapper — most logic is identical to Chrome version.

// Firefox: sidebar opens via browser.sidebarAction (no programmatic open in MV3)
// The action button toggles the sidebar
browser.action.onClicked.addListener(() => {
  browser.sidebarAction.toggle();
});

// Context menu
browser.runtime.onInstalled.addListener(() => {
  browser.contextMenus.create({ id: "biogist-lookup", title: "Look up in BioGist", contexts: ["selection"] });
  browser.contextMenus.create({ id: "biogist-scan-selection", title: "Scan selection in BioGist", contexts: ["selection"] });
});

browser.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "biogist-lookup") {
    browser.sidebarAction.open();
    setTimeout(() => {
      browser.runtime.sendMessage({ type: "lookup", text: info.selectionText, tabId: tab.id });
    }, 500);
  }
  if (info.menuItemId === "biogist-scan-selection") {
    browser.sidebarAction.open();
    setTimeout(() => {
      browser.runtime.sendMessage({ type: "scan-text", text: info.selectionText, tabId: tab.id });
    }, 500);
  }
});

// ── Per-tab entity storage ──────────────────────────────────────────
const tabEntities = {};

browser.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "entities-detected") {
    const tabId = sender.tab ? sender.tab.id : (msg.tabId || null);
    if (tabId) {
      tabEntities[tabId] = {
        entities: msg.entities || [],
        title: (sender.tab ? sender.tab.title : msg.title) || "Unknown page",
        url: (sender.tab ? sender.tab.url : msg.url) || ""
      };
    }
  }

  if (msg.type === "badge-count") {
    if (sender.tab) {
      const count = msg.count || 0;
      browser.action.setBadgeText({ text: count > 0 ? String(count) : "", tabId: sender.tab.id });
      browser.action.setBadgeBackgroundColor({
        color: count > 10 ? "#059669" : count > 0 ? "#0891b2" : "#475569",
        tabId: sender.tab.id
      });
    }
  }

  if (msg.type === "scanning" && sender.tab) {
    browser.action.setBadgeText({ text: "...", tabId: sender.tab.id });
    browser.action.setBadgeBackgroundColor({ color: "#7c3aed", tabId: sender.tab.id });
  }

  if (msg.type === "get-tab-entities") {
    browser.tabs.query({ active: true, currentWindow: true }).then((tabs) => {
      const tabId = tabs[0] ? tabs[0].id : null;
      const stored = tabId && tabEntities[tabId] ? tabEntities[tabId] : null;
      sendResponse({
        entities: stored ? stored.entities : [],
        title: stored ? stored.title : "",
        url: stored ? stored.url : "",
        tabId
      });
    });
    return true;
  }

  if (msg.type === "store-pasted-entities") {
    tabEntities["pasted"] = {
      entities: msg.entities || [],
      title: "Pasted text",
      url: ""
    };
  }

  if (msg.type === "store-tab-entities") {
    if (msg.tabId) tabEntities[msg.tabId] = msg.entities || [];
  }

  if (msg.type === "get-all-tab-entities") {
    const all = [];
    const seen = new Set();
    const sources = [];
    for (const [tabId, stored] of Object.entries(tabEntities)) {
      if (!stored || !Array.isArray(stored.entities)) continue;
      const title = stored.title || "Tab " + tabId;
      sources.push({ tabId, title, count: stored.entities.length });
      stored.entities.forEach(e => {
        const key = (e.type || "") + ":" + (e.id || "");
        if (!seen.has(key)) { seen.add(key); all.push({ ...e, source: title }); }
      });
    }
    sendResponse({ entities: all, sources, tabCount: sources.length });
    return true;
  }

  if (msg.type === "get-specific-tab") {
    const stored = msg.tabId && tabEntities[msg.tabId] ? tabEntities[msg.tabId] : null;
    sendResponse({
      entities: stored ? stored.entities : [],
      title: stored ? stored.title : "",
      tabId: msg.tabId
    });
    return true;
  }

  if (msg.type === "clear-tab-entities") {
    if (msg.scope === "all") {
      for (const key of Object.keys(tabEntities)) delete tabEntities[key];
    } else if (msg.tabId) {
      delete tabEntities[msg.tabId];
    } else {
      browser.tabs.query({ active: true, currentWindow: true }).then((tabs) => {
        if (tabs[0]) delete tabEntities[tabs[0].id];
      });
    }
  }

  if (msg.type === "check-pdf") {
    browser.tabs.get(msg.tabId).then((tab) => {
      let isPdf = false;
      if (/\.pdf(\?|#|$)/i.test(tab.url || "")) isPdf = true;
      else if ((tab.title || "").endsWith(".pdf")) isPdf = true;
      sendResponse({ isPdf, url: tab.url, title: tab.title });
    });
    return true;
  }
});

browser.tabs.onRemoved.addListener((tabId) => { delete tabEntities[tabId]; });
browser.webNavigation.onCommitted.addListener((details) => {
  if (details.frameId === 0) delete tabEntities[details.tabId];
});

browser.tabs.onActivated.addListener((activeInfo) => {
  browser.runtime.sendMessage({ type: "tab-switched", tabId: activeInfo.tabId }).catch(() => {});
});
