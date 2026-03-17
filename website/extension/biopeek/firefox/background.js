// BioPeek — Firefox Background Script
// Uses browser.* namespace (Firefox MV3)

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

browser.runtime.onInstalled.addListener(function () {
  // Right-click on links
  browser.contextMenus.create({
    id: "biopeek-open-link",
    title: "Open in BioPeek",
    contexts: ["link"],
    targetUrlPatterns: BIO_EXTENSIONS.flatMap(function (ext) {
      return [
        "*://*/*" + ext,
        "*://*/*" + ext + "?*",
        "*://*/*" + ext + ".gz",
        "*://*/*" + ext.toUpperCase()
      ];
    })
  });

  // Right-click on page (for any link not caught by patterns)
  browser.contextMenus.create({
    id: "biopeek-open-page",
    title: "Open current page in BioPeek",
    contexts: ["page"],
    documentUrlPatterns: BIO_EXTENSIONS.flatMap(function (ext) {
      return ["*://*/*" + ext, "*://*/*" + ext + "?*"];
    })
  });

  // Right-click on selected text (analyze as sequence)
  browser.contextMenus.create({
    id: "biopeek-analyze-selection",
    title: "Analyze selection in BioPeek",
    contexts: ["selection"]
  });
});

browser.contextMenus.onClicked.addListener(function (info, tab) {
  if (info.menuItemId === "biopeek-open-link") {
    openUrlInViewer(info.linkUrl);
  } else if (info.menuItemId === "biopeek-open-page") {
    openUrlInViewer(info.pageUrl);
  } else if (info.menuItemId === "biopeek-analyze-selection") {
    openTextInViewer(info.selectionText);
  }
});

// ── Download Interception ─────────────────────────────────────────

browser.downloads.onChanged.addListener(function (delta) {
  if (delta.filename && delta.filename.current) {
    var filename = delta.filename.current.toLowerCase();
    var isBioFile = BIO_EXTENSIONS.some(function (ext) {
      return filename.endsWith(ext);
    });
    if (isBioFile) {
      browser.storage.session.set({
        lastBioDownload: {
          filename: delta.filename.current,
          timestamp: Date.now()
        }
      });
    }
  }
});

// ── Viewer Tab Management ─────────────────────────────────────────

function openUrlInViewer(url) {
  var viewerUrl = browser.runtime.getURL("viewer.html") + "?url=" + encodeURIComponent(url);
  browser.tabs.create({ url: viewerUrl });
}

function openTextInViewer(text) {
  browser.storage.session.set({ pendingText: text }).then(function () {
    browser.tabs.create({ url: browser.runtime.getURL("viewer.html") + "?source=clipboard" });
  });
}

// Register BioPeek extension ID so BioGist can find us
browser.storage.local.set({ biopeek_extension_id: browser.runtime.id });

// Listen for messages from popup, viewer, or external extensions (BioGist)
browser.runtime.onMessage.addListener(function (msg, sender, sendResponse) {
  if (msg.type === "open-viewer") {
    browser.tabs.create({ url: browser.runtime.getURL("viewer.html") });
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
      browser.storage.session.set({ pendingFile: pendingFile }).then(function() {
        browser.tabs.create({
          url: browser.runtime.getURL("viewer.html") + "?source=extension"
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
