// Extension-specific: load file from chrome.storage.session (passed from popup/background)
(function() {
  if (typeof chrome === "undefined" || !chrome.storage) return;
  var params = new URLSearchParams(window.location.search);
  var source = params.get("source");

  if (source === "extension") {
    chrome.storage.session.get("pendingFile", function(data) {
      if (data && data.pendingFile) {
        var f = data.pendingFile;
        var blob = new Blob([f.content], { type: "text/plain" });
        var file = new File([blob], f.name, { type: "text/plain" });
        var dt = new DataTransfer();
        dt.items.add(file);
        var dropEvent = new DragEvent("drop", { dataTransfer: dt, bubbles: true });
        document.getElementById("vw-drop-zone").dispatchEvent(dropEvent);
        chrome.storage.session.remove("pendingFile");
      }
    });
  } else if (source === "clipboard") {
    chrome.storage.session.get("pendingText", function(data) {
      if (data && data.pendingText) {
        var text = data.pendingText;
        var blob = new Blob([text], { type: "text/plain" });
        var file = new File([blob], "selection.txt", { type: "text/plain" });
        var dt = new DataTransfer();
        dt.items.add(file);
        var dropEvent = new DragEvent("drop", { dataTransfer: dt, bubbles: true });
        document.getElementById("vw-drop-zone").dispatchEvent(dropEvent);
        chrome.storage.session.remove("pendingText");
      }
    });
  }
})();
