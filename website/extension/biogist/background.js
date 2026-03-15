// BioGist — Background Service Worker (Pull Model)
// Stores entities per tab. Sidebar pulls data — no unsolicited pushes.

// Open sidebar when extension icon is clicked
chrome.action.onClicked.addListener(async (tab) => {
  await chrome.sidePanel.open({ tabId: tab.id });
});

// Context menu
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({ id: "biogist-lookup", title: "Look up in BioGist", contexts: ["selection"] });
  chrome.contextMenus.create({ id: "biogist-scan-selection", title: "Scan selection in BioGist", contexts: ["selection"] });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "biogist-lookup") {
    chrome.sidePanel.open({ tabId: tab.id });
    setTimeout(() => {
      chrome.runtime.sendMessage({ type: "lookup", text: info.selectionText, tabId: tab.id });
    }, 500);
  }
  if (info.menuItemId === "biogist-scan-selection") {
    chrome.sidePanel.open({ tabId: tab.id });
    setTimeout(() => {
      chrome.runtime.sendMessage({ type: "scan-text", text: info.selectionText, tabId: tab.id });
    }, 500);
  }
});

// ── Per-tab entity storage ──────────────────────────────────────────

const tabEntities = {};

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {

  // Content script reports detected entities — store silently, don't forward
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

  // Badge updates
  if (msg.type === "badge-count") {
    if (sender.tab) {
      const count = msg.count || 0;
      chrome.action.setBadgeText({ text: count > 0 ? String(count) : "", tabId: sender.tab.id });
      chrome.action.setBadgeBackgroundColor({
        color: count > 10 ? "#059669" : count > 0 ? "#0891b2" : "#475569",
        tabId: sender.tab.id
      });
    }
  }

  if (msg.type === "scanning" && sender.tab) {
    chrome.action.setBadgeText({ text: "...", tabId: sender.tab.id });
    chrome.action.setBadgeBackgroundColor({ color: "#7c3aed", tabId: sender.tab.id });
  }

  // Sidebar requests: get entities for current tab
  if (msg.type === "get-tab-entities") {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
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

  // Sidebar requests: store entities for a specific tab (after sidebar-initiated scan)
  if (msg.type === "store-tab-entities") {
    if (msg.tabId) {
      tabEntities[msg.tabId] = msg.entities || [];
    }
  }

  // Sidebar requests: get all tabs merged
  if (msg.type === "get-all-tab-entities") {
    const all = [];
    const seen = new Set();
    const sources = [];
    for (const [tabId, stored] of Object.entries(tabEntities)) {
      if (!stored || !Array.isArray(stored.entities)) continue;
      const title = stored.title || "Tab " + tabId;
      sources.push({ tabId, title: title, count: stored.entities.length });
      stored.entities.forEach(e => {
        const key = (e.type || "") + ":" + (e.id || "");
        if (!seen.has(key)) {
          seen.add(key);
          all.push({ ...e, source: title });
        }
      });
    }
    sendResponse({ entities: all, sources, tabCount: sources.length });
    return true;
  }

  // Get entities for a specific tab by ID
  if (msg.type === "get-specific-tab") {
    const stored = msg.tabId && tabEntities[msg.tabId] ? tabEntities[msg.tabId] : null;
    sendResponse({
      entities: stored ? stored.entities : [],
      title: stored ? stored.title : "",
      tabId: msg.tabId
    });
    return true;
  }

  // Clear current tab's entities
  if (msg.type === "clear-tab-entities") {
    if (msg.tabId) {
      delete tabEntities[msg.tabId];
    } else {
      chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        if (tabs[0]) delete tabEntities[tabs[0].id];
      });
    }
  }

  // PDF detection for scan-page (kept for compatibility)
  if (msg.type === "check-pdf") {
    chrome.tabs.get(msg.tabId, (tab) => {
      if (!tab) { sendResponse({ isPdf: false }); return; }
      let isPdf = false;
      if (/\.pdf(\?|#|$)/i.test(tab.url || "")) isPdf = true;
      else if (tab.url && tab.url.includes("mhjfbmdgcfjbbpaeojofohoefgiehjai")) isPdf = true;
      else if ((tab.title || "").endsWith(".pdf")) isPdf = true;
      sendResponse({ isPdf, url: tab.url, title: tab.title });
    });
    return true;
  }
});

// Clean up
chrome.tabs.onRemoved.addListener((tabId) => { delete tabEntities[tabId]; });
chrome.webNavigation.onCommitted.addListener((details) => {
  if (details.frameId === 0) delete tabEntities[details.tabId];
});

// Tab switch — just notify sidebar to pull new data (no entity payload)
chrome.tabs.onActivated.addListener((activeInfo) => {
  chrome.runtime.sendMessage({ type: "tab-switched", tabId: activeInfo.tabId }).catch(() => {});
});
