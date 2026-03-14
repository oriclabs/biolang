// BLViewer Chrome Extension — Popup Script

(function () {
  "use strict";

  var fileInput = document.getElementById("file-input");
  var dropZone = document.getElementById("drop-zone");
  var statusEl = document.getElementById("status");

  // ── File handling ─────────────────────────────────────────────

  function openFilesInViewer(fileList) {
    if (!fileList || fileList.length === 0) return;

    Array.from(fileList).forEach(function (file) {
      var reader = new FileReader();
      reader.onload = function () {
        // Store file content in session storage, open viewer tab
        var data = {
          name: file.name,
          size: file.size,
          content: reader.result,
          timestamp: Date.now()
        };

        // Use chrome.storage.session for passing data to viewer tab
        chrome.storage.session.set({ pendingFile: data }, function () {
          chrome.tabs.create({ url: chrome.runtime.getURL("viewer.html") + "?source=extension" });
          addToRecent(file.name, file.size);
        });
      };
      reader.readAsText(file);
    });
  }

  // Drop zone click → file picker
  dropZone.addEventListener("click", function () {
    fileInput.click();
  });

  fileInput.addEventListener("change", function () {
    openFilesInViewer(this.files);
  });

  // Drag and drop on popup
  dropZone.addEventListener("dragover", function (e) {
    e.preventDefault();
    e.stopPropagation();
    dropZone.classList.add("drag-over");
  });
  dropZone.addEventListener("dragleave", function () {
    dropZone.classList.remove("drag-over");
  });
  dropZone.addEventListener("drop", function (e) {
    e.preventDefault();
    e.stopPropagation();
    dropZone.classList.remove("drag-over");
    openFilesInViewer(e.dataTransfer.files);
  });

  // ── Buttons ───────────────────────────────────────────────────

  // Open full viewer tab
  document.getElementById("open-viewer").addEventListener("click", function () {
    chrome.tabs.create({ url: chrome.runtime.getURL("viewer.html") });
  });

  // Browse files button
  document.getElementById("open-file").addEventListener("click", function () {
    fileInput.click();
  });

  // Help button
  document.getElementById("open-help").addEventListener("click", function () {
    chrome.tabs.create({ url: chrome.runtime.getURL("help.html") });
  });

  // Open URL
  document.getElementById("url-go").addEventListener("click", function () {
    var url = document.getElementById("url-input").value.trim();
    if (!url) return;
    if (!url.startsWith("http://") && !url.startsWith("https://")) {
      url = "https://" + url;
    }
    chrome.runtime.sendMessage({ type: "open-viewer-url", url: url });
    window.close();
  });

  // Enter key in URL input
  document.getElementById("url-input").addEventListener("keydown", function (e) {
    if (e.key === "Enter") document.getElementById("url-go").click();
  });

  // ── Recent Files ──────────────────────────────────────────────

  function formatBytes(b) {
    if (b < 1024) return b + " B";
    if (b < 1048576) return (b / 1024).toFixed(1) + " KB";
    return (b / 1048576).toFixed(1) + " MB";
  }

  function formatTimeAgo(ts) {
    var diff = Date.now() - ts;
    var mins = Math.floor(diff / 60000);
    if (mins < 1) return "now";
    if (mins < 60) return mins + "m";
    var hrs = Math.floor(mins / 60);
    if (hrs < 24) return hrs + "h";
    var days = Math.floor(hrs / 24);
    return days + "d";
  }

  function getFormatFromName(name) {
    var ext = name.split(".").pop().toLowerCase();
    var map = {
      fasta: "fasta", fa: "fasta", fna: "fasta", faa: "fasta",
      fastq: "fastq", fq: "fastq",
      vcf: "vcf", bed: "bed",
      gff: "gff", gff3: "gff", gtf: "gff",
      sam: "sam", csv: "csv", tsv: "tsv"
    };
    return map[ext] || ext;
  }

  function addToRecent(name, size) {
    chrome.storage.local.get({ recentFiles: [] }, function (data) {
      var recent = data.recentFiles;
      // Remove duplicate
      recent = recent.filter(function (r) { return r.name !== name; });
      // Add to front
      recent.unshift({
        name: name,
        size: size,
        format: getFormatFromName(name),
        date: Date.now()
      });
      // Cap at 20
      if (recent.length > 20) recent = recent.slice(0, 20);
      chrome.storage.local.set({ recentFiles: recent });
    });
  }

  function renderRecent() {
    chrome.storage.local.get({ recentFiles: [] }, function (data) {
      var recent = data.recentFiles;
      var section = document.getElementById("recent-section");
      var list = document.getElementById("recent-list");

      if (recent.length === 0) {
        section.style.display = "none";
        return;
      }

      list.innerHTML = "";
      recent.forEach(function (entry) {
        var item = document.createElement("div");
        item.className = "recent-item";
        item.innerHTML =
          '<span class="recent-badge">' + escapeHtml(entry.format) + '</span>' +
          '<span class="recent-name" title="' + escapeHtml(entry.name) + '">' + escapeHtml(entry.name) + '</span>' +
          '<span class="recent-meta">' + formatBytes(entry.size) + ' · ' + formatTimeAgo(entry.date) + '</span>';

        item.addEventListener("click", function () {
          // Open viewer — user will need to re-select the file
          // (we don't cache file content in chrome.storage.local due to size limits)
          chrome.tabs.create({ url: chrome.runtime.getURL("viewer.html") });
        });

        list.appendChild(item);
      });
      section.style.display = "";
    });
  }

  // Clear recent
  document.getElementById("clear-recent").addEventListener("click", function () {
    chrome.storage.local.set({ recentFiles: [] }, function () {
      document.getElementById("recent-section").style.display = "none";
    });
  });

  function escapeHtml(s) {
    var d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  // ── Init ──────────────────────────────────────────────────────
  renderRecent();
})();
