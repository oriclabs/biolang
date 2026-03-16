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
      const title = (sender.tab ? sender.tab.title : msg.title) || "Unknown page";
      const url = (sender.tab ? sender.tab.url : msg.url) || "";
      tabEntities[tabId] = {
        entities: msg.entities || [],
        pageText: msg.pageText || "",
        title, url
      };
      // Record to persistent history
      recordHistory(msg.entities || [], title, url);
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
        pageText: stored ? stored.pageText : "",
        title: stored ? stored.title : "",
        url: stored ? stored.url : "",
        tabId
      });
    });
    return true;
  }

  // Sidebar requests: store entities for a specific tab (after sidebar-initiated scan or merge)
  if (msg.type === "store-tab-entities") {
    if (msg.tabId) {
      tabEntities[msg.tabId] = {
        entities: msg.entities || [],
        pageText: (tabEntities[msg.tabId] && tabEntities[msg.tabId].pageText) || "",
        title: msg.title || (tabEntities[msg.tabId] && tabEntities[msg.tabId].title) || "",
        url: msg.url || (tabEntities[msg.tabId] && tabEntities[msg.tabId].url) || ""
      };
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
      pageText: stored ? (stored.pageText || "") : "",
      title: stored ? stored.title : "",
      url: stored ? (stored.url || "") : "",
      tabId: msg.tabId
    });
    return true;
  }

  // Store pasted text entities under a virtual tab ID
  if (msg.type === "store-pasted-entities") {
    tabEntities["pasted"] = {
      entities: msg.entities || [],
      title: "Pasted text",
      url: ""
    };
  }

  // Clear entities
  if (msg.type === "clear-tab-entities") {
    if (msg.scope === "all") {
      // Clear everything
      for (const key of Object.keys(tabEntities)) delete tabEntities[key];
    } else if (msg.tabId) {
      delete tabEntities[msg.tabId];
    } else {
      chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        if (tabs[0]) delete tabEntities[tabs[0].id];
      });
    }
  }

  // Get entity map for co-occurrence (per-tab, not merged)
  if (msg.type === "get-tab-entity-map") {
    const result = {};
    for (const [tabId, stored] of Object.entries(tabEntities)) {
      if (stored && Array.isArray(stored.entities) && stored.entities.length > 0) {
        result[tabId] = { title: stored.title || "Tab " + tabId, entities: stored.entities };
      }
    }
    sendResponse(result);
    return true;
  }

  // Get entity history
  if (msg.type === "get-entity-history") {
    chrome.storage.local.get("biogist_history", (data) => {
      sendResponse({ history: data.biogist_history || [] });
    });
    return true;
  }

  // Clear entity history
  if (msg.type === "clear-entity-history") {
    chrome.storage.local.remove("biogist_history");
    sendResponse({ ok: true });
    return true;
  }

  // Batch scan URLs
  if (msg.type === "batch-scan") {
    batchQueue = (msg.urls || []).filter(u => u.startsWith("http"));
    batchTotal = batchQueue.length;
    batchDone = 0;
    if (batchQueue.length > 0) processBatchNext();
    sendResponse({ started: batchQueue.length });
    return true;
  }

  // Inject content script and scan (activeTab mode — no content_scripts in manifest)
  if (msg.type === "inject-and-scan") {
    const tabId = msg.tabId;
    if (!tabId) return;
    chrome.scripting.executeScript({ target: { tabId }, files: ["content.js"] }, () => {
      if (chrome.runtime.lastError) {
        // Can't inject (e.g., chrome:// pages)
        sendResponse({ error: "Cannot scan this page" });
        return;
      }
      // Give content script a moment to initialize, then send scan
      setTimeout(() => {
        chrome.tabs.sendMessage(tabId, { type: "scan" }, (resp) => {
          sendResponse(resp || { count: 0 });
        });
      }, 500);
    });
    return true;
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

// Keyboard shortcut: scan-page
chrome.commands.onCommand.addListener((command, tab) => {
  if (command === "scan-page" && tab) {
    chrome.sidePanel.open({ tabId: tab.id });
    setTimeout(() => {
      chrome.runtime.sendMessage({ type: "trigger-scan", tabId: tab.id }).catch(() => {});
    }, 500);
  }
});

// History persistence
function recordHistory(entities, title, url) {
  if (!entities || entities.length === 0) return;
  chrome.storage.local.get("biogist_history", (data) => {
    const history = data.biogist_history || [];
    const ts = Date.now();
    entities.forEach(e => {
      history.push({ type: e.type, id: e.id, url: url || "", title: title || "", ts });
    });
    // Cap at 2000 entries
    const trimmed = history.length > 2000 ? history.slice(history.length - 2000) : history;
    chrome.storage.local.set({ biogist_history: trimmed });
  });
}

// Batch scan
let batchQueue = [];
let batchTotal = 0;
let batchDone = 0;

function processBatchNext() {
  if (batchQueue.length === 0) {
    chrome.runtime.sendMessage({ type: "batch-scan-complete", total: batchTotal }).catch(() => {});
    return;
  }
  const url = batchQueue.shift();
  batchDone++;
  chrome.runtime.sendMessage({ type: "batch-scan-progress", done: batchDone, total: batchTotal, url }).catch(() => {});

  // Check if a tab with this URL is already open
  chrome.tabs.query({}, (allTabs) => {
    const existing = allTabs.find(t => t.url && t.url.replace(/#.*$/, "") === url.replace(/#.*$/, ""));
    if (existing) {
      // Tab already open — send scan message to it
      const tabId = existing.id;
      chrome.tabs.sendMessage(tabId, { type: "scan" }, (resp) => {
        if (chrome.runtime.lastError) {
          // Content script not injected on this tab — skip it
          setTimeout(() => processBatchNext(), 500);
        } else {
          setTimeout(() => processBatchNext(), 2000);
        }
      });
    } else {
      // Open new tab
      chrome.tabs.create({ url, active: false }, (tab) => {
        const tabId = tab.id;
        const onUpdated = (id, info) => {
          if (id === tabId && info.status === "complete") {
            chrome.tabs.onUpdated.removeListener(onUpdated);
            setTimeout(() => processBatchNext(), 3000);
          }
        };
        chrome.tabs.onUpdated.addListener(onUpdated);
        setTimeout(() => {
          chrome.tabs.onUpdated.removeListener(onUpdated);
          processBatchNext();
        }, 15000);
      });
    }
  });
}

// Clean up
chrome.tabs.onRemoved.addListener((tabId) => { delete tabEntities[tabId]; });
chrome.webNavigation.onCommitted.addListener((details) => {
  if (details.frameId === 0) {
    delete tabEntities[details.tabId];
    // Notify sidebar that this tab navigated (so it can clear stale results)
    chrome.runtime.sendMessage({ type: "tab-navigated", tabId: details.tabId }).catch(() => {});
  }
});

// Tab switch — just notify sidebar to pull new data (no entity payload)
chrome.tabs.onActivated.addListener((activeInfo) => {
  chrome.runtime.sendMessage({ type: "tab-switched", tabId: activeInfo.tabId }).catch(() => {});
});
