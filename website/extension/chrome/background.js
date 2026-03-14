// BLViewer Chrome Extension — Background Service Worker
// Handles context menus, download interception, and viewer tab management

const BIO_EXTENSIONS = [
  ".fasta", ".fa", ".fna", ".faa",
  ".fastq", ".fq",
  ".vcf",
  ".bed",
  ".gff", ".gff3", ".gtf",
  ".sam",
  ".csv", ".tsv"
];

// ── Context Menu ──────────────────────────────────────────────────

chrome.runtime.onInstalled.addListener(function () {
  // Right-click on links
  chrome.contextMenus.create({
    id: "blviewer-open-link",
    title: "Open in BLViewer",
    contexts: ["link"],
    targetUrlPatterns: BIO_EXTENSIONS.flatMap(function (ext) {
      return [
        "*://*/*" + ext,
        "*://*/*" + ext + "?*",
        "*://*/*" + ext + ".gz",
        "*://*/*" + ext.toUpperCase(),
      ];
    })
  });

  // Right-click on page (for any link not caught by patterns)
  chrome.contextMenus.create({
    id: "blviewer-open-page",
    title: "Open current page in BLViewer",
    contexts: ["page"],
    documentUrlPatterns: BIO_EXTENSIONS.flatMap(function (ext) {
      return ["*://*/*" + ext, "*://*/*" + ext + "?*"];
    })
  });

  // Right-click on selected text (analyze as sequence)
  chrome.contextMenus.create({
    id: "blviewer-analyze-selection",
    title: "Analyze selection in BLViewer",
    contexts: ["selection"]
  });
});

chrome.contextMenus.onClicked.addListener(function (info, tab) {
  if (info.menuItemId === "blviewer-open-link") {
    openUrlInViewer(info.linkUrl);
  } else if (info.menuItemId === "blviewer-open-page") {
    openUrlInViewer(info.pageUrl);
  } else if (info.menuItemId === "blviewer-analyze-selection") {
    openTextInViewer(info.selectionText);
  }
});

// ── Download Interception ─────────────────────────────────────────

chrome.downloads.onDeterminingFilename.addListener(function (item, suggest) {
  var filename = (item.filename || "").toLowerCase();
  var isBioFile = BIO_EXTENSIONS.some(function (ext) {
    return filename.endsWith(ext);
  });

  if (isBioFile && item.fileSize < 50 * 1024 * 1024) {
    // Store the download info and show a notification or open viewer
    // For now, let the download proceed normally — the popup shows recent downloads
    // Users can also right-click links to open directly
    chrome.storage.session.set({
      lastBioDownload: {
        url: item.url,
        filename: item.filename,
        size: item.fileSize,
        timestamp: Date.now()
      }
    });
  }

  // Always suggest the original filename (don't block downloads)
  suggest({ filename: item.filename });
});

// ── Viewer Tab Management ─────────────────────────────────────────

function openUrlInViewer(url) {
  var viewerUrl = chrome.runtime.getURL("viewer.html") + "?url=" + encodeURIComponent(url);
  chrome.tabs.create({ url: viewerUrl });
}

function openTextInViewer(text) {
  // Store text in session storage, open viewer, it will pick it up
  chrome.storage.session.set({ pendingText: text }, function () {
    chrome.tabs.create({ url: chrome.runtime.getURL("viewer.html") + "?source=clipboard" });
  });
}

// Listen for messages from popup or viewer
chrome.runtime.onMessage.addListener(function (msg, sender, sendResponse) {
  if (msg.type === "open-viewer") {
    chrome.tabs.create({ url: chrome.runtime.getURL("viewer.html") });
    sendResponse({ ok: true });
  } else if (msg.type === "open-viewer-url") {
    openUrlInViewer(msg.url);
    sendResponse({ ok: true });
  }
  return true;
});
