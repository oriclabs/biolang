// BioGist — Background Service Worker
// Opens sidebar, manages context menu, relays messages between content script and sidebar.


// Open sidebar when extension icon is clicked
chrome.action.onClicked.addListener(async (tab) => {
  await chrome.sidePanel.open({ tabId: tab.id });
});

// Context menu for selected text
chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "biogist-lookup",
    title: "Look up in BioGist",
    contexts: ["selection"]
  });
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  if (info.menuItemId === "biogist-lookup") {
    chrome.sidePanel.open({ tabId: tab.id });
    // Give sidebar time to initialize before sending the lookup
    setTimeout(() => {
      chrome.runtime.sendMessage({
        type: "lookup",
        text: info.selectionText,
        tabId: tab.id
      });
    }, 500);
  }
});

// Per-tab entity storage
const tabEntities = {};

// Relay messages between content script and sidebar
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "entities-detected") {
    // Store per-tab
    if (sender.tab) {
      tabEntities[sender.tab.id] = msg.entities || [];
    }
    // Forward to sidebar with tab ID
    chrome.runtime.sendMessage({
      ...msg,
      tabId: sender.tab ? sender.tab.id : null
    }).catch(() => {});
  }
  if (msg.type === "badge-count") {
    if (msg.count > 0 && sender.tab) {
      chrome.action.setBadgeText({ text: String(msg.count), tabId: sender.tab.id });
      chrome.action.setBadgeBackgroundColor({ color: "#0891b2", tabId: sender.tab.id });
    }
  }
  if (msg.type === "get-tab-entities") {
    // Sidebar asks for current tab's entities
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      const tabId = tabs[0] ? tabs[0].id : null;
      const entities = tabId && tabEntities[tabId] ? tabEntities[tabId] : [];
      sendResponse({ entities: entities, tabId: tabId });
    });
    return true; // async sendResponse
  }
  if (msg.type === "clear-tab-entities") {
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs[0]) delete tabEntities[tabs[0].id];
    });
  }
  if (msg.type === "scan-page") {
    const tabId = msg.tabId;
    // Check if tab is a PDF — handle in background (content scripts can't run on Chrome PDF viewer)
    chrome.tabs.get(tabId, (tab) => {
      if (!tab) return;
      // Detect PDF: direct .pdf URL, or Chrome PDF viewer with embedded URL
      let pdfUrl = null;
      if (/\.pdf(\?|#|$)/i.test(tab.url || "")) {
        pdfUrl = tab.url;
      } else if (tab.url && tab.url.includes("mhjfbmdgcfjbbpaeojofohoefgiehjai")) {
        // Chrome's built-in PDF viewer — extract the actual PDF URL
        try { pdfUrl = new URL(tab.url).searchParams.get("url") || new URL(tab.url).hash.slice(1); } catch(e) {}
      } else if ((tab.title || "").endsWith(".pdf")) {
        pdfUrl = tab.url;
      }
      if (pdfUrl) {
        console.log("[BioGist] PDF detected:", pdfUrl);
        scanPdfInBackground(pdfUrl, tabId);
        return;
      }
      // Normal page — inject content script if needed, then scan
      chrome.tabs.sendMessage(tabId, { type: "scan" }).catch(() => {
        chrome.scripting.executeScript({
          target: { tabId: tabId },
          files: ["content.js"]
        }).then(() => {
          setTimeout(() => {
            chrome.tabs.sendMessage(tabId, { type: "scan" }).catch(() => {
              chrome.runtime.sendMessage({ type: "entities-detected", entities: [] }).catch(() => {});
            });
          }, 500);
        }).catch(() => {
          chrome.runtime.sendMessage({ type: "entities-detected", entities: [] }).catch(() => {});
        });
      });
    });
  }
});

// ── PDF detection ──────────────────────────────────────────────────
// PDFs can't be scanned by content scripts. Tell user to use Paste button.

function scanPdfInBackground(url, tabId) {
  // PDFs can't be scanned automatically — tell user to use Paste button
  chrome.runtime.sendMessage({
    type: "entities-detected",
    entities: [],
    tabId,
    error: "PDF detected — use the Paste button in the sidebar to scan copied text, or use: bl -e 'read_pdf(\"file.pdf\") |> scan_bio()'"
  }).catch(() => {});
}

// Clean up when tabs close
chrome.tabs.onRemoved.addListener((tabId) => {
  delete tabEntities[tabId];
});

// When user switches tabs, send that tab's stored entities to sidebar
chrome.tabs.onActivated.addListener((activeInfo) => {
  const entities = tabEntities[activeInfo.tabId] || [];
  chrome.runtime.sendMessage({
    type: "tab-switched",
    entities: entities,
    tabId: activeInfo.tabId
  }).catch(() => {});
});
