// BioPeek Chrome Extension — Background Service Worker
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
    id: "biopeek-open-link",
    title: "Open in BioPeek",
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
    id: "biopeek-open-page",
    title: "Open current page in BioPeek",
    contexts: ["page"],
    documentUrlPatterns: BIO_EXTENSIONS.flatMap(function (ext) {
      return ["*://*/*" + ext, "*://*/*" + ext + "?*"];
    })
  });

  // Right-click on selected text (analyze as sequence)
  chrome.contextMenus.create({
    id: "biopeek-analyze-selection",
    title: "Analyze selection in BioPeek",
    contexts: ["selection"]
  });
});

chrome.contextMenus.onClicked.addListener(function (info, tab) {
  if (info.menuItemId === "biopeek-open-link") {
    openUrlInViewer(info.linkUrl);
  } else if (info.menuItemId === "biopeek-open-page") {
    openUrlInViewer(info.pageUrl);
  } else if (info.menuItemId === "biopeek-analyze-selection") {
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

// Register BioPeek extension ID so BioGist can find us
chrome.storage.local.set({ biopeek_extension_id: chrome.runtime.id });

// Listen for messages from popup, viewer, or external extensions (BioGist)
chrome.runtime.onMessage.addListener(function (msg, sender, sendResponse) {
  if (msg.type === "open-viewer") {
    chrome.tabs.create({ url: chrome.runtime.getURL("viewer.html") });
    sendResponse({ ok: true });
  } else if (msg.type === "open-viewer-url") {
    openUrlInViewer(msg.url);
    sendResponse({ ok: true });
  } else if (msg.type === "fetch-for-viewer") {
    // BioGist or other extension asks us to fetch a URL (we bypass CORS)
    var name = msg.url.split("/").pop().split("?")[0] || "remote-file.txt";
    var isGz = /\.(gz|bgz)$/i.test(name);

    fetch(msg.url).then(function(resp) {
      if (!resp.ok) throw new Error("HTTP " + resp.status);
      return isGz ? resp.arrayBuffer() : resp.text();
    }).then(function(data) {
      var pendingFile;
      if (isGz) {
        // Encode binary as base64 for session storage
        var bytes = new Uint8Array(data);
        var chunks = [];
        var chunkSize = 8192;
        for (var i = 0; i < bytes.length; i += chunkSize) {
          var slice = bytes.subarray(i, Math.min(i + chunkSize, bytes.length));
          chunks.push(String.fromCharCode.apply(null, slice));
        }
        pendingFile = { name: name, content: btoa(chunks.join("")), binary: true };
      } else {
        pendingFile = { name: name, content: data, binary: false };
      }
      chrome.storage.session.set({ pendingFile: pendingFile }, function() {
        chrome.tabs.create({
          url: chrome.runtime.getURL("viewer.html") + "?source=extension"
        });
      });
      sendResponse({ ok: true });
    }).catch(function(err) {
      sendResponse({ ok: false, error: String(err) });
    });
    return true; // keep sendResponse alive for async
  }
  return true;
});
