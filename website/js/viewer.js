(function() {
  "use strict";

  // ── State ──────────────────────────────────────────────────────
  var files = [];         // [{name, format, size, text, parsed, stats}]
  var activeTab = -1;
  var currentView = "table";
  var sortCol = -1;
  var sortAsc = true;
  var sortCols = []; // [{col, asc}] for multi-column sort (Feature 7)
  var searchTerm = "";
  var colFilters = {}; // {colIndex: Set of allowed values}
  var rowDetailEnabled = localStorage.getItem("vw-row-detail") === "1"; // default off
  var bookmarkedRows = {}; // {tabIdx: Set of rowIdx}
  var bookmarkMode = false;
  var pinColumnsMode = false;
  var pinnedCols = new Set(); // Feature 4: per-column pinning
  var motifTerm = ""; // regex pattern for sequence motif search
  var tabSortState = {}; // {tabIdx: {col, asc}}
  var modifiedCells = new Set(); // Feature 18: "rowIdx:colIdx" strings
  var savedViews = JSON.parse(localStorage.getItem("vw-saved-views") || "{}"); // Feature 9
  var wasm = null;
  var wasmLoading = false;
  var wasmQueue = [];
  var MAX_PREVIEW = 50 * 1024 * 1024; // 50 MB text preview limit
  var pageSize = 100;   // rows per page (pagination)
  var currentPage = 0;  // current page index
  var hiddenCols = {};  // {tabIdx: Set of colIndex}
  var heatmapCols = {}; // {colIndex: {min, max, enabled}}
  var groupByCol = -1;  // column index for row grouping (-1 = off)
  var highlightRule = null; // {col, op, value} for conditional row highlighting
  var undoStack = []; // state snapshots for undo
  var cellNavRow = -1; // keyboard cell navigation
  var cellNavCol = -1;
  var showMinimap = false; // row density minimap off by default
  var isFullscreen = false; // Feature 5: fullscreen mode
  var showRowNumbers = true; // Feature 6: row number toggle
  var textWrap = false; // Feature 10: text wrap toggle
  var formatNumbers = false; // Feature 11: numeric formatting toggle
  var regexSearch = false; // Feature 16: regex search mode
  var splitViewMode = false; // Feature 19: split view (dual tab panes)
  var splitViewTab = -1; // Feature 19: right pane tab index
  var bookmarkFilterActive = false; // Feature 17: show only starred rows

  // ── History DB (IndexedDB) ───────────────────────────────────────
  var HistoryDB = (function() {
    var DB_NAME = "BLViewerHistory";
    var DB_VERSION = 1;
    var STORE_NAME = "files";
    var MAX_ENTRIES = 50;
    var SESSION_LIMIT = 5 * 1024 * 1024; // 5 MB — store content for files under this

    var dbPromise = null;

    function open() {
      if (dbPromise) return dbPromise;
      dbPromise = new Promise(function(resolve, reject) {
        var req = indexedDB.open(DB_NAME, DB_VERSION);
        req.onupgradeneeded = function(e) {
          var db = e.target.result;
          if (!db.objectStoreNames.contains(STORE_NAME)) {
            var store = db.createObjectStore(STORE_NAME, { keyPath: "id" });
            store.createIndex("date", "date", { unique: false });
          }
        };
        req.onsuccess = function() { resolve(req.result); };
        req.onerror = function() { reject(req.error); };
      });
      return dbPromise;
    }

    function addEntry(entry) {
      return open().then(function(db) {
        return new Promise(function(resolve, reject) {
          var tx = db.transaction(STORE_NAME, "readwrite");
          var store = tx.objectStore(STORE_NAME);
          store.put(entry);
          tx.oncomplete = function() { pruneOld(db).then(resolve); };
          tx.onerror = function() { reject(tx.error); };
        });
      });
    }

    function pruneOld(db) {
      return new Promise(function(resolve) {
        var tx = db.transaction(STORE_NAME, "readwrite");
        var store = tx.objectStore(STORE_NAME);
        var idx = store.index("date");
        var all = [];
        idx.openCursor().onsuccess = function(e) {
          var cursor = e.target.result;
          if (cursor) { all.push(cursor.value.id); cursor.continue(); }
          else {
            if (all.length > MAX_ENTRIES) {
              // all is sorted by date asc — remove oldest
              var toRemove = all.slice(0, all.length - MAX_ENTRIES);
              toRemove.forEach(function(id) { store.delete(id); });
            }
            resolve();
          }
        };
      });
    }

    function getAll() {
      return open().then(function(db) {
        return new Promise(function(resolve, reject) {
          var tx = db.transaction(STORE_NAME, "readonly");
          var store = tx.objectStore(STORE_NAME);
          var req = store.getAll();
          req.onsuccess = function() {
            var results = req.result || [];
            results.sort(function(a, b) { return b.date - a.date; });
            resolve(results);
          };
          req.onerror = function() { reject(req.error); };
        });
      });
    }

    function getById(id) {
      return open().then(function(db) {
        return new Promise(function(resolve, reject) {
          var tx = db.transaction(STORE_NAME, "readonly");
          var req = tx.objectStore(STORE_NAME).get(id);
          req.onsuccess = function() { resolve(req.result || null); };
          req.onerror = function() { reject(req.error); };
        });
      });
    }

    function remove(id) {
      return open().then(function(db) {
        return new Promise(function(resolve, reject) {
          var tx = db.transaction(STORE_NAME, "readwrite");
          tx.objectStore(STORE_NAME).delete(id);
          tx.oncomplete = function() { resolve(); };
          tx.onerror = function() { reject(tx.error); };
        });
      });
    }

    function clearAll() {
      return open().then(function(db) {
        return new Promise(function(resolve, reject) {
          var tx = db.transaction(STORE_NAME, "readwrite");
          tx.objectStore(STORE_NAME).clear();
          tx.oncomplete = function() { resolve(); };
          tx.onerror = function() { reject(tx.error); };
        });
      });
    }

    return { open: open, addEntry: addEntry, getAll: getAll, getById: getById, remove: remove, clearAll: clearAll, SESSION_LIMIT: SESSION_LIMIT };
  })();

  // ── DOM refs ───────────────────────────────────────────────────
  var dropZone = document.getElementById("vw-drop-zone");
  var workspace = document.getElementById("vw-workspace");
  var tabsEl = document.getElementById("vw-tabs");
  var addTabBtn = document.getElementById("vw-add-tab");
  var contentEl = document.getElementById("vw-content");
  var searchInput = document.getElementById("vw-search");
  var formatChip = document.getElementById("vw-format-chip");
  var countChip = document.getElementById("vw-count-chip");
  var sizeChip = document.getElementById("vw-size-chip");
  var footerRows = document.getElementById("vw-footer-rows");
  var footerFilter = document.getElementById("vw-footer-filter");
  var footerParse = document.getElementById("vw-footer-parse-time");
  var fileInput = document.getElementById("vw-file-input");
  var openBtn = document.getElementById("vw-open-btn");

  // ── Format detection ───────────────────────────────────────────
  var FORMAT_MAP = {
    fasta: { icon: "🧬", color: "#34d399" },
    fastq: { icon: "📊", color: "#60a5fa" },
    vcf:   { icon: "🔬", color: "#f87171" },
    bed:   { icon: "📍", color: "#fbbf24" },
    gff:   { icon: "📐", color: "#a78bfa" },
    csv:   { icon: "📋", color: "#22d3ee" },
    tsv:   { icon: "📋", color: "#22d3ee" },
    sam:   { icon: "🗂️", color: "#fb923c" },
    txt:   { icon: "📄", color: "#94a3b8" }
  };

  // Stage 0: Extension-based rejection for non-data text files
  var REJECT_EXTS = {
    // Source code / scripts
    bl: "BioLang script", py: "Python script", r: "R script", rb: "Ruby script",
    js: "JavaScript", ts: "TypeScript", jsx: "JSX", tsx: "TSX",
    rs: "Rust source", go: "Go source", java: "Java source", kt: "Kotlin source",
    cpp: "C++ source", c: "C source", h: "C/C++ header", hpp: "C++ header",
    cs: "C# source", swift: "Swift source", scala: "Scala source",
    sh: "Shell script", bash: "Bash script", zsh: "Zsh script", ps1: "PowerShell script",
    pl: "Perl script", lua: "Lua script", jl: "Julia source",
    // Markup / config
    html: "HTML file", htm: "HTML file", xml: "XML file", svg: "SVG image",
    css: "CSS stylesheet", scss: "SCSS stylesheet", less: "LESS stylesheet",
    json: "JSON file", yaml: "YAML file", yml: "YAML file", toml: "TOML file",
    ini: "INI config", cfg: "Config file", conf: "Config file",
    // Documents
    md: "Markdown document", rst: "reStructuredText", tex: "LaTeX document",
    log: "Log file",
    // Compiled / build
    lock: "Lock file", sum: "Checksum file",
    wasm: "WebAssembly binary", o: "Object file", a: "Archive",
    exe: "Executable", dll: "DLL", so: "Shared library", dylib: "Dynamic library"
  };

  // Known bioinformatics data extensions (allowed even if not in FORMAT_MAP directly)
  var BIO_EXTS = {
    fa: 1, fna: 1, faa: 1, fasta: 1, fq: 1, fastq: 1,
    vcf: 1, bed: 1, gff: 1, gff3: 1, gtf: 1,
    csv: 1, tsv: 1, sam: 1, txt: 1, dat: 1, tab: 1,
    maf: 1, wig: 1, bedgraph: 1, psl: 1, chain: 1, axt: 1,
    pdb: 1, mol: 1, sdf: 1, phr: 1, nhr: 1, qual: 1
  };

  function checkExtension(name) {
    var ext = (name.match(/\.([^.]+)$/) || [,""])[1].toLowerCase();
    if (BIO_EXTS[ext] || FORMAT_MAP[ext]) return null; // known data format
    var desc = REJECT_EXTS[ext];
    if (desc) return { reason: desc, hint: "BLViewer is for bioinformatics data files (FASTA, FASTQ, VCF, BED, GFF, SAM, CSV, TSV).\nThis appears to be a " + desc.toLowerCase() + "." };
    return null; // unknown extension — allow and let content detection handle it
  }

  // Stage 1: Magic byte detection for binary files (called on raw ArrayBuffer)
  // Returns {ok:true} for text files, or {ok:false, reason, hint} for binary/unsupported
  function checkMagicBytes(bytes, name) {
    if (bytes.length < 4) return { ok: true }; // too small to check
    var b = new Uint8Array(bytes.slice(0, 16));

    // Gzip / BGZF (covers .gz, .bgz, .bam)
    if (b[0] === 0x1F && b[1] === 0x8B) {
      var ext = (name.match(/\.([^.]+?)(?:\.gz)?$/i) || [,""])[1].toLowerCase();
      if (ext === "bam" || name.toLowerCase().endsWith(".bam")) {
        return { ok: false, reason: "BAM (binary alignment)", hint: "BAM is a compressed binary format. Convert to SAM for viewing:\n\n  samtools view -h file.bam > file.sam\n\nFor a specific region (much faster):\n  samtools view -h file.bam chr1:1000-50000 > region.sam\n\nOr in BioLang:\n  read_sam(\"file.bam\") |> head(100)" };
      }
      if (ext === "bcf") {
        return { ok: false, reason: "BCF (binary variant)", hint: "BCF is the binary equivalent of VCF. Convert for viewing:\n\n  bcftools view file.bcf > file.vcf\n\nFor a specific region:\n  bcftools view file.bcf chr1:1000-50000 > region.vcf" };
      }
      // Allow gzipped text files — will be decompressed in browser
      return { ok: true, gzipped: true };
    }

    // BAM magic: "BAM\1"
    if (b[0] === 0x42 && b[1] === 0x41 && b[2] === 0x4D && b[3] === 0x01) {
      return { ok: false, reason: "BAM (binary alignment)", hint: "BAM is a compressed binary format. Convert to SAM for viewing:\n\n  samtools view -h file.bam > file.sam\n\nFor a specific region (much faster):\n  samtools view -h file.bam chr1:1000-50000 > region.sam\n\nOr in BioLang:\n  read_sam(\"file.bam\") |> head(100)" };
    }

    // BCF magic: "BCF\2"
    if (b[0] === 0x42 && b[1] === 0x43 && b[2] === 0x46) {
      return { ok: false, reason: "BCF (binary variant)", hint: "BCF is the binary equivalent of VCF. Convert for viewing:\n\n  bcftools view file.bcf > file.vcf\n\nFor a specific region:\n  bcftools view file.bcf chr1:1000-50000 > region.vcf" };
    }

    // CRAM magic: "CRAM"
    if (b[0] === 0x43 && b[1] === 0x52 && b[2] === 0x41 && b[3] === 0x4D) {
      return { ok: false, reason: "CRAM (compressed alignment)", hint: "CRAM is a highly compressed alignment format. Convert to SAM:\n\n  samtools view -h -T ref.fa file.cram > file.sam\n\nNote: CRAM requires the reference FASTA used during alignment.\n\nFor a specific region:\n  samtools view -h -T ref.fa file.cram chr1:1000-50000 > region.sam" };
    }

    // PDF: "%PDF"
    if (b[0] === 0x25 && b[1] === 0x50 && b[2] === 0x44 && b[3] === 0x46) {
      return { ok: false, reason: "PDF document", hint: "This is a PDF file, not a bioinformatics data file." };
    }

    // ZIP / XLSX / DOCX: "PK\x03\x04"
    if (b[0] === 0x50 && b[1] === 0x4B && b[2] === 0x03 && b[3] === 0x04) {
      var ext = (name.match(/\.([^.]+)$/) || [,""])[1].toLowerCase();
      if (ext === "xlsx" || ext === "xls") {
        return { ok: false, reason: "Excel spreadsheet", hint: "Export as CSV from Excel, then drop the .csv file here." };
      }
      if (ext === "docx" || ext === "doc") {
        return { ok: false, reason: "Word document", hint: "This is a Word file, not a data file." };
      }
      return { ok: false, reason: "ZIP archive", hint: "Extract the archive first, then drop individual files." };
    }

    // PNG: "\x89PNG"
    if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4E && b[3] === 0x47) {
      return { ok: false, reason: "PNG image", hint: "This is an image file, not a data file." };
    }

    // JPEG: "\xFF\xD8\xFF"
    if (b[0] === 0xFF && b[1] === 0xD8 && b[2] === 0xFF) {
      return { ok: false, reason: "JPEG image", hint: "This is an image file, not a data file." };
    }

    // GIF: "GIF8"
    if (b[0] === 0x47 && b[1] === 0x49 && b[2] === 0x46 && b[3] === 0x38) {
      return { ok: false, reason: "GIF image", hint: "This is an image file, not a data file." };
    }

    // BZ2: "BZ"
    if (b[0] === 0x42 && b[1] === 0x5A && b[2] === 0x68) {
      return { ok: false, reason: "Bzip2 compressed", hint: "Decompress first: bunzip2 file.bz2" };
    }

    // XZ: "\xFD7zXZ"
    if (b[0] === 0xFD && b[1] === 0x37 && b[2] === 0x7A && b[3] === 0x58 && b[4] === 0x5A) {
      return { ok: false, reason: "XZ compressed", hint: "Decompress first: unxz file.xz" };
    }

    // Zstandard: "\x28\xB5\x2F\xFD"
    if (b[0] === 0x28 && b[1] === 0xB5 && b[2] === 0x2F && b[3] === 0xFD) {
      return { ok: false, reason: "Zstandard compressed", hint: "Decompress first: unzstd file.zst" };
    }

    // SFF: ".sff"
    if (b[0] === 0x2E && b[1] === 0x73 && b[2] === 0x66 && b[3] === 0x66) {
      return { ok: false, reason: "SFF (Standard Flowgram Format)", hint: "Convert to FASTQ: sff2fastq file.sff" };
    }

    // Check for high ratio of non-printable bytes (likely binary)
    var nonPrint = 0;
    var check = Math.min(b.length, 16);
    for (var i = 0; i < check; i++) {
      if (b[i] < 9 || (b[i] > 13 && b[i] < 32 && b[i] !== 27)) nonPrint++;
    }
    if (nonPrint > check * 0.3) {
      return { ok: false, reason: "Binary file", hint: "This appears to be a binary file. BLViewer supports text-based formats only." };
    }

    return { ok: true };
  }

  // Stage 2: Content-based format detection (text files)
  function detectFormat(name, text) {
    var ext = (name.match(/\.([^.]+)$/) || [,"txt"])[1].toLowerCase();
    if (ext === "fa" || ext === "fna" || ext === "faa") ext = "fasta";
    if (ext === "fq") ext = "fastq";
    if (ext === "gff3" || ext === "gtf") ext = "gff";

    // Content sniff first (overrides wrong extensions)
    var head = text.substring(0, 2000);
    var firstLine = head.split("\n")[0] || "";

    // Strong signatures (unique first chars / headers)
    if (head.indexOf("##fileformat=VCF") !== -1) return "vcf";
    if (head.indexOf("##gff-version") !== -1) return "gff";

    // FASTA: starts with >
    if (firstLine.charAt(0) === ">" && /^>[^\t]+/.test(firstLine)) return "fasta";

    // FASTQ: starts with @ and has + separator line
    if (firstLine.charAt(0) === "@" && head.indexOf("\n+\n") !== -1) return "fastq";

    // SAM: starts with @HD/@SQ/@RG/@PG header, or first data line has 11+ tab fields
    if (/^@(HD|SQ|RG|PG|CO)\t/m.test(head)) return "sam";
    if (firstLine.charAt(0) !== "@" && firstLine.split("\t").length >= 11) {
      // Verify SAM-like structure: col2=int(flag), col4=int(pos), col5=int(mapq)
      var samParts = firstLine.split("\t");
      if (!isNaN(parseInt(samParts[1])) && !isNaN(parseInt(samParts[3])) && !isNaN(parseInt(samParts[4]))) return "sam";
    }

    // GFF: 9 tab-separated columns, col4 and col5 are integers
    var gffLines = head.split("\n").filter(function(l) { return l && l.charAt(0) !== "#"; });
    if (gffLines.length > 0) {
      var gp = gffLines[0].split("\t");
      if (gp.length === 9 && !isNaN(parseInt(gp[3])) && !isNaN(parseInt(gp[4]))) return "gff";
    }

    // BED: 3+ tab columns, col2 and col3 are integers
    if (/^\S+\t\d+\t\d+/m.test(head)) {
      var bedLines = head.split("\n").filter(function(l) { return l && l.charAt(0) !== "#" && !l.startsWith("track") && !l.startsWith("browser"); });
      if (bedLines.length > 0) {
        var bp = bedLines[0].split("\t");
        if (bp.length >= 3 && !isNaN(parseInt(bp[1])) && !isNaN(parseInt(bp[2]))) return "bed";
      }
    }

    // If extension matches a known format, trust it (weaker signals)
    if (FORMAT_MAP[ext]) return ext;

    // CSV/TSV sniffing
    if (firstLine.indexOf(",") !== -1 && firstLine.split(",").length > 2) return "csv";
    if (firstLine.indexOf("\t") !== -1 && firstLine.split("\t").length > 2) return "tsv";

    return "txt";
  }

  // Stage 3: Post-parse validation — check if parsed result looks reasonable
  function validateParsed(result, name) {
    var warnings = [];

    if (result.rows.length === 0) {
      warnings.push("No records parsed. The file may be empty or in an unexpected format.");
    }

    if (result.format === "fasta") {
      // FASTA should have sequences
      if (result.rows.length > 0 && !result.rows[0][3]) {
        warnings.push("FASTA records found but sequences are empty.");
      }
    }

    if (result.format === "fastq") {
      // FASTQ should have quality strings matching sequence length
      if (result.rows.length > 0) {
        var seq = String(result.rows[0][3] || "");
        var qual = String(result.rows[0][4] || "");
        if (seq.length > 0 && qual.length > 0 && seq.length !== qual.length) {
          warnings.push("Sequence and quality lengths don't match (seq=" + seq.length + ", qual=" + qual.length + "). File may be malformed.");
        }
      }
    }

    if (result.format === "vcf") {
      // VCF should have the standard columns
      if (result.columns.length < 8) {
        warnings.push("VCF has fewer than 8 columns. Expected: CHROM, POS, ID, REF, ALT, QUAL, FILTER, INFO.");
      }
    }

    if (result.format === "sam") {
      // SAM should have 11 standard columns
      if (result.rows.length > 0 && result.rows[0].length < 11) {
        warnings.push("SAM records have fewer than 11 fields. File may be truncated or malformed.");
      }
    }

    if (result.format === "bed") {
      // BED: start should be < end
      if (result.rows.length > 0) {
        var badBed = result.rows.filter(function(r) { return r[1] > r[2]; });
        if (badBed.length > 0) {
          warnings.push(badBed.length + " regions have start > end. BED coordinates may be incorrect.");
        }
      }
    }

    // Check for very low parse rate (might be wrong format)
    if (result.format !== "txt" && result.rows.length > 0) {
      // Estimate total lines vs parsed records
      var approxLines = (result._rawLineCount || 0);
      if (approxLines > 10 && result.rows.length < approxLines * 0.1) {
        warnings.push("Only " + result.rows.length + " of ~" + approxLines + " lines parsed. Format may be misdetected.");
      }
    }

    result.warnings = warnings;
    return result;
  }

  // ── Parsers ────────────────────────────────────────────────────
  function parseFasta(text) {
    var records = [];
    var lines = text.split("\n");
    var header = "", seq = [];
    function fastaGC(s) {
      var gc = 0, total = 0;
      for (var j = 0; j < s.length; j++) {
        var ch = s.charCodeAt(j) | 32; // lowercase
        if (ch === 97 || ch === 116 || ch === 99 || ch === 103) { total++; if (ch === 99 || ch === 103) gc++; }
      }
      return total > 0 ? Math.round(gc / total * 1000) / 10 : 0;
    }
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (line.charAt(0) === ">") {
        if (header || seq.length) {
          var s = seq.join("");
          records.push({ id: header.split(/\s/)[0], description: header, sequence: s, length: s.length, gc: fastaGC(s) });
        }
        header = line.substring(1);
        seq = [];
      } else if (line) {
        seq.push(line);
      }
    }
    if (header || seq.length) {
      var s = seq.join("");
      records.push({ id: header.split(/\s/)[0], description: header, sequence: s, length: s.length, gc: fastaGC(s) });
    }
    return {
      columns: ["id", "description", "length", "gc_pct", "sequence"],
      colTypes: ["str", "str", "num", "num", "seq"],
      rows: records.map(function(r) { return [r.id, r.description, r.length, r.gc, r.sequence]; }),
      stats: fastaStats(records)
    };
  }

  function fastaStats(records) {
    if (!records.length) return {};
    var lens = records.map(function(r) { return r.length; });
    var total = lens.reduce(function(a, b) { return a + b; }, 0);
    var sorted = lens.slice().sort(function(a, b) { return b - a; });
    var half = total / 2; var cum = 0; var n50 = 0;
    for (var i = 0; i < sorted.length; i++) { cum += sorted[i]; if (cum >= half) { n50 = sorted[i]; break; } }
    var gcCount = 0; var totalBp = 0;
    records.forEach(function(r) {
      for (var j = 0; j < r.sequence.length; j++) {
        var c = r.sequence.charAt(j).toUpperCase();
        if (c === "G" || c === "C") gcCount++;
        if (c !== "N") totalBp++;
      }
    });
    return {
      "Sequences": records.length, "Total bp": total,
      "Avg length": Math.round(total / records.length),
      "Min length": sorted[sorted.length - 1], "Max length": sorted[0],
      "N50": n50, "GC %": totalBp ? (gcCount / totalBp * 100).toFixed(1) + "%" : "N/A"
    };
  }

  function parseFastq(text) {
    var records = [];
    var lines = text.split("\n");
    for (var i = 0; i + 3 < lines.length; i += 4) {
      if (lines[i].charAt(0) !== "@") continue;
      var id = lines[i].substring(1).split(/\s/)[0];
      var seq = lines[i + 1].trimEnd();
      var qual = lines[i + 3].trimEnd();
      var avgQ = 0;
      for (var j = 0; j < qual.length; j++) avgQ += qual.charCodeAt(j) - 33;
      avgQ = qual.length ? (avgQ / qual.length).toFixed(1) : 0;
      records.push([id, seq.length, avgQ, seq, qual]);
    }
    var quals = records.map(function(r) { return parseFloat(r[2]); });
    var lens = records.map(function(r) { return r[1]; });
    return {
      columns: ["id", "length", "avg_qual", "sequence", "quality"],
      colTypes: ["str", "num", "num", "seq", "qual"],
      rows: records,
      stats: {
        "Reads": records.length,
        "Total bp": lens.reduce(function(a, b) { return a + b; }, 0),
        "Avg length": records.length ? Math.round(lens.reduce(function(a, b) { return a + b; }, 0) / records.length) : 0,
        "Min length": records.length ? Math.min.apply(null, lens) : 0,
        "Max length": records.length ? Math.max.apply(null, lens) : 0,
        "Mean quality": records.length ? (quals.reduce(function(a, b) { return a + b; }, 0) / records.length).toFixed(1) : 0,
        "Q30+ reads": records.length ? (quals.filter(function(q) { return q >= 30; }).length / records.length * 100).toFixed(1) + "%" : "0%"
      }
    };
  }

  function parseVcf(text) {
    var rows = [];
    var lines = text.split("\n");
    var headerLine = null;
    var metaLines = 0;
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line) continue;
      if (line.substring(0, 2) === "##") { metaLines++; continue; }
      if (line.charAt(0) === "#") { headerLine = line.substring(1).split("\t"); continue; }
      var parts = line.split("\t");
      if (parts.length < 8) continue;
      rows.push([parts[0], parseInt(parts[1]), parts[2] || ".", parts[3], parts[4], parts[5], parts[6], parts[7]]);
    }
    var cols = headerLine ? headerLine.slice(0, 8) : ["CHROM", "POS", "ID", "REF", "ALT", "QUAL", "FILTER", "INFO"];
    var types = ["str", "num", "str", "seq", "seq", "str", "str", "str"];
    // Variant type stats
    var snps = 0, indels = 0, other = 0;
    rows.forEach(function(r) {
      var ref = String(r[3]), alt = String(r[4]);
      var alts = alt.split(",");
      var isSnp = alts.every(function(a) { return a.length === ref.length && a.length === 1; });
      if (isSnp) snps++; else if (alts.some(function(a) { return a.length !== ref.length; })) indels++; else other++;
    });
    // Ti/Tv ratio
    var ti = 0, tv = 0;
    rows.forEach(function(r) {
      var ref = String(r[3]).toUpperCase(), alt = String(r[4]).toUpperCase();
      if (ref.length !== 1 || alt.length !== 1) return;
      var pair = ref + alt;
      if (pair === "AG" || pair === "GA" || pair === "CT" || pair === "TC") ti++;
      else tv++;
    });
    return {
      columns: cols, colTypes: types, rows: rows,
      stats: {
        "Variants": rows.length, "SNPs": snps, "Indels": indels, "Other": other,
        "Ti/Tv ratio": tv ? (ti / tv).toFixed(2) : "N/A",
        "Meta lines": metaLines,
        "Chromosomes": new Set(rows.map(function(r) { return r[0]; })).size
      }
    };
  }

  function parseBed(text) {
    var rows = [];
    var lines = text.split("\n");
    var maxCols = 0;
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line || line.charAt(0) === "#" || line.substring(0, 5) === "track" || line.substring(0, 7) === "browser") continue;
      var parts = line.split("\t");
      if (parts.length < 3) continue;
      parts[1] = parseInt(parts[1]); parts[2] = parseInt(parts[2]);
      if (maxCols < parts.length) maxCols = parts.length;
      rows.push(parts);
    }
    var colNames = ["chrom", "start", "end", "name", "score", "strand", "thickStart", "thickEnd", "itemRgb", "blockCount", "blockSizes", "blockStarts"];
    var cols = colNames.slice(0, maxCols);
    var types = cols.map(function(c) { return (c === "start" || c === "end" || c === "score") ? "num" : "str"; });
    var totalBp = rows.reduce(function(a, r) { return a + (r[2] - r[1]); }, 0);
    return {
      columns: cols, colTypes: types, rows: rows,
      stats: {
        "Regions": rows.length, "Total bp covered": totalBp,
        "Avg region size": rows.length ? Math.round(totalBp / rows.length) : 0,
        "Chromosomes": new Set(rows.map(function(r) { return r[0]; })).size
      }
    };
  }

  function parseGff(text) {
    var rows = [];
    var lines = text.split("\n");
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line || line.charAt(0) === "#") continue;
      var parts = line.split("\t");
      if (parts.length < 9) continue;
      parts[3] = parseInt(parts[3]); parts[4] = parseInt(parts[4]);
      rows.push(parts.slice(0, 9));
    }
    var featureTypes = {};
    rows.forEach(function(r) { featureTypes[r[2]] = (featureTypes[r[2]] || 0) + 1; });
    var topFeatures = Object.entries(featureTypes).sort(function(a, b) { return b[1] - a[1]; }).slice(0, 5);
    var stats = { "Features": rows.length, "Chromosomes": new Set(rows.map(function(r) { return r[0]; })).size };
    topFeatures.forEach(function(f) { stats[f[0]] = f[1]; });
    return {
      columns: ["seqid", "source", "type", "start", "end", "score", "strand", "phase", "attributes"],
      colTypes: ["str", "str", "str", "num", "num", "str", "str", "str", "str"],
      rows: rows, stats: stats
    };
  }

  function parseCsv(text, sep) {
    var lines = text.split("\n");
    var header = splitCsvLine(lines[0], sep);
    var rows = [];
    for (var i = 1; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line) continue;
      rows.push(splitCsvLine(line, sep));
    }
    // Infer types from first 100 rows
    var types = header.map(function(_, ci) {
      var nums = 0;
      var count = Math.min(rows.length, 100);
      for (var ri = 0; ri < count; ri++) {
        if (rows[ri][ci] !== undefined && rows[ri][ci] !== "" && !isNaN(Number(rows[ri][ci]))) nums++;
      }
      return nums > count * 0.7 ? "num" : "str";
    });
    // Convert numeric columns
    rows.forEach(function(row) {
      types.forEach(function(t, ci) {
        if (t === "num" && row[ci] !== undefined && row[ci] !== "") row[ci] = Number(row[ci]);
      });
    });
    // Stats for numeric columns
    var stats = { "Rows": rows.length, "Columns": header.length };
    header.forEach(function(col, ci) {
      if (types[ci] === "num") {
        var vals = rows.map(function(r) { return r[ci]; }).filter(function(v) { return typeof v === "number" && !isNaN(v); });
        if (vals.length) {
          stats[col + " (mean)"] = (vals.reduce(function(a, b) { return a + b; }, 0) / vals.length).toFixed(2);
        }
      }
    });
    return { columns: header, colTypes: types, rows: rows, stats: stats };
  }

  function splitCsvLine(line, sep) {
    if (sep === "\t") return line.split("\t");
    var result = []; var field = ""; var inQ = false;
    for (var i = 0; i < line.length; i++) {
      var c = line.charAt(i);
      if (inQ) {
        if (c === '"' && line.charAt(i + 1) === '"') { field += '"'; i++; }
        else if (c === '"') inQ = false;
        else field += c;
      } else {
        if (c === '"') inQ = true;
        else if (c === sep) { result.push(field); field = ""; }
        else field += c;
      }
    }
    result.push(field);
    return result;
  }

  function parseSam(text) {
    var rows = [];
    var lines = text.split("\n");
    var headerLines = 0;
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line) continue;
      if (line.charAt(0) === "@") { headerLines++; continue; }
      var parts = line.split("\t");
      if (parts.length < 11) continue;
      rows.push([parts[0], parseInt(parts[1]), parts[2], parseInt(parts[3]), parseInt(parts[4]), parts[5], parts[6], parseInt(parts[7]), parseInt(parts[8]), parts[9], parts[10]]);
    }
    var mapped = rows.filter(function(r) { return (r[1] & 4) === 0; }).length;
    var paired = rows.filter(function(r) { return (r[1] & 1) !== 0; }).length;
    return {
      columns: ["QNAME", "FLAG", "RNAME", "POS", "MAPQ", "CIGAR", "RNEXT", "PNEXT", "TLEN", "SEQ", "QUAL"],
      colTypes: ["str", "num", "str", "num", "num", "str", "str", "num", "num", "seq", "qual"],
      rows: rows,
      stats: {
        "Alignments": rows.length, "Mapped": mapped,
        "Unmapped": rows.length - mapped,
        "Paired": paired,
        "Map rate": rows.length ? (mapped / rows.length * 100).toFixed(1) + "%" : "0%",
        "Header lines": headerLines
      }
    };
  }

  function parseFile(name, text) {
    var fmt = detectFormat(name, text);
    var t0 = performance.now();
    var result;
    switch (fmt) {
      case "fasta": result = parseFasta(text); break;
      case "fastq": result = parseFastq(text); break;
      case "vcf":   result = parseVcf(text); break;
      case "bed":   result = parseBed(text); break;
      case "gff":   result = parseGff(text); break;
      case "csv":   result = parseCsv(text, ","); break;
      case "tsv":   result = parseCsv(text, "\t"); break;
      case "sam":   result = parseSam(text); break;
      default:
        // Auto-detect delimiter for plain text files
        var autoDelim = detectDelimiter(text);
        if (autoDelim !== "\t" || text.indexOf("\t") !== -1) {
          var firstLine = text.split("\n")[0] || "";
          if (firstLine.split(autoDelim).length >= 3) {
            result = parseCsv(text, autoDelim);
            fmt = autoDelim === "," ? "csv" : autoDelim === "\t" ? "tsv" : "tsv";
            break;
          }
        }
        result = parsePlain(text);
        break;
    }
    result.parseTime = ((performance.now() - t0) / 1000).toFixed(3);
    result.format = fmt;
    // Estimate total non-empty lines for validation
    var lineCount = 0;
    for (var i = 0; i < text.length; i++) { if (text.charAt(i) === "\n") lineCount++; }
    result._rawLineCount = lineCount;
    return result;
  }

  function parsePlain(text) {
    var lines = text.split("\n");
    return {
      columns: ["line", "content"],
      colTypes: ["num", "str"],
      rows: lines.map(function(l, i) { return [i + 1, l]; }),
      stats: { "Lines": lines.length, "Characters": text.length },
      format: "txt"
    };
  }

  // ── Loading overlay ────────────────────────────────────────────
  function showLoadingOverlay(fileName, fileSize) {
    hideLoadingOverlay();
    var overlay = document.createElement("div");
    overlay.id = "vw-loading-overlay";
    overlay.style.cssText = "position:fixed;top:0;left:0;right:0;bottom:0;z-index:9999;" +
      "background:rgba(0,0,0,0.6);display:flex;align-items:center;justify-content:center;";
    var sizeMB = (fileSize / (1024 * 1024)).toFixed(1);
    var card = document.createElement("div");
    card.style.cssText = "background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;" +
      "padding:32px 40px;text-align:center;font-family:var(--vw-sans);box-shadow:0 8px 30px rgba(0,0,0,0.4);min-width:300px;";
    card.innerHTML =
      '<div style="margin-bottom:16px">' +
        '<div class="vw-loading-spinner" style="width:36px;height:36px;border:3px solid var(--vw-border);' +
        'border-top-color:var(--vw-accent);border-radius:50%;animation:vw-spin 0.8s linear infinite;margin:0 auto"></div>' +
      '</div>' +
      '<div style="color:var(--vw-text);font-size:14px;font-weight:600;margin-bottom:6px" id="vw-loading-title">Loading ' + escapeHtml(fileName) + '</div>' +
      '<div style="color:var(--vw-text-dim);font-size:12px" id="vw-loading-detail">' + sizeMB + ' MB — reading file...</div>' +
      '<div style="margin-top:14px;height:4px;background:var(--vw-border);border-radius:2px;overflow:hidden">' +
        '<div id="vw-loading-bar" style="height:100%;width:0%;background:var(--vw-accent);border-radius:2px;transition:width 0.3s"></div>' +
      '</div>';
    overlay.appendChild(card);
    document.body.appendChild(overlay);
    // Add spin animation if not already present
    if (!document.getElementById("vw-spin-style")) {
      var style = document.createElement("style");
      style.id = "vw-spin-style";
      style.textContent = "@keyframes vw-spin { to { transform: rotate(360deg); } }";
      document.head.appendChild(style);
    }
  }
  function updateLoadingOverlay(phase, pct) {
    var detail = document.getElementById("vw-loading-detail");
    var bar = document.getElementById("vw-loading-bar");
    if (detail) detail.textContent = phase;
    if (bar) bar.style.width = pct + "%";
  }
  function hideLoadingOverlay() {
    var el = document.getElementById("vw-loading-overlay");
    if (el) el.remove();
  }

  // ── GZ decompression helpers ───────────────────────────────────
  function isGzipped(buffer) {
    var bytes = new Uint8Array(buffer);
    return bytes.length >= 2 && bytes[0] === 0x1f && bytes[1] === 0x8b;
  }

  async function decompressGz(buffer) {
    var ds = new DecompressionStream("gzip");
    var stream = new Response(buffer).body.pipeThrough(ds);
    var decompressed = await new Response(stream).text();
    return decompressed;
  }

  function stripGzExtension(name) {
    return name.replace(/\.(gz|bgz)$/i, "");
  }

  // ── Chunked preview for large files ───────────────────────────
  var MAX_PREVIEW_LINES = 50000;
  var MAX_PREVIEW_BYTES = 5 * 1024 * 1024; // 5MB threshold for line-based preview

  function previewText(text, maxLines) {
    var lines = text.split("\n");
    if (lines.length <= maxLines) return { text: text, truncated: false, totalLines: lines.length };
    return { text: lines.slice(0, maxLines).join("\n"), truncated: true, totalLines: lines.length };
  }

  // ── File loading ───────────────────────────────────────────────
  function loadFiles(fileList) {
    var toLoad = Array.from(fileList);
    toLoad.forEach(function(file) {
      // Stage 0: Extension check for non-data text files
      var extCheck = checkExtension(file.name);
      if (extCheck) {
        showFileError(file.name, extCheck.reason, extCheck.hint);
        return;
      }

      // Show loading overlay for files > 1MB
      if (file.size > 1024 * 1024) {
        showLoadingOverlay(file.name, file.size);
      }

      // Stage 1: Read first 16 bytes as ArrayBuffer for magic byte check
      var headerReader = new FileReader();
      headerReader.onload = function() {
        var check = checkMagicBytes(headerReader.result, file.name);
        if (!check.ok) {
          hideLoadingOverlay();
          showFileError(file.name, check.reason, check.hint);
          return;
        }

        // Gzipped file — read entire file as ArrayBuffer, decompress in browser
        if (check.gzipped) {
          var gzReader = new FileReader();
          updateLoadingOverlay("Reading compressed file...", 20);
          gzReader.onprogress = function(e) { if (e.lengthComputable) updateLoadingOverlay("Reading... " + Math.round(e.loaded / 1024 / 1024) + " MB", 15 + Math.round(e.loaded / e.total * 25)); };
          gzReader.onload = function() {
            updateLoadingOverlay("Decompressing...", 50);
            decompressGz(gzReader.result).then(function(text) {
              var decompName = stripGzExtension(file.name);
              var decompSize = text.length;
              updateLoadingOverlay("Parsing data...", 75);
              setTimeout(function() { addFile(decompName, decompSize, text, false, null); hideLoadingOverlay(); }, 10);
            }).catch(function(err) {
              hideLoadingOverlay();
              showFileError(file.name, "Decompression failed", "Could not decompress this gzip file in the browser.\n\nError: " + (err.message || err) + "\n\nTry decompressing locally:\n  gunzip " + file.name);
            });
          };
          gzReader.readAsArrayBuffer(file);
          return;
        }

        // Magic bytes OK — read as text
        if (file.size > MAX_PREVIEW) {
          var reader = new FileReader();
          var blob = file.slice(0, MAX_PREVIEW);
          updateLoadingOverlay("Reading first 50 MB...", 30);
          reader.onprogress = function(e) { if (e.lengthComputable) updateLoadingOverlay("Reading... " + Math.round(e.loaded / 1024 / 1024) + " MB", 20 + Math.round(e.loaded / e.total * 30)); };
          reader.onload = function() {
            updateLoadingOverlay("Parsing data...", 60);
            setTimeout(function() { addFile(file.name, file.size, reader.result, true, file); hideLoadingOverlay(); }, 10);
          };
          reader.readAsText(blob);
        } else {
          var reader = new FileReader();
          updateLoadingOverlay("Reading file...", 30);
          reader.onprogress = function(e) { if (e.lengthComputable) updateLoadingOverlay("Reading... " + Math.round(e.loaded / 1024 / 1024) + " MB", 20 + Math.round(e.loaded / e.total * 40)); };
          reader.onload = function() {
            updateLoadingOverlay("Parsing data...", 70);
            setTimeout(function() { addFile(file.name, file.size, reader.result, false, file); hideLoadingOverlay(); }, 10);
          };
          reader.readAsText(file);
        }
      };
      headerReader.readAsArrayBuffer(file.slice(0, 16));
    });
  }

  function showFileError(name, reason, hint) {
    // Show inline error in the drop zone or as a notification
    var existing = document.getElementById("vw-file-error");
    if (existing) existing.remove();

    var errDiv = document.createElement("div");
    errDiv.id = "vw-file-error";
    errDiv.style.cssText = "position:fixed;top:80px;left:50%;transform:translateX(-50%);z-index:500;" +
      "background:var(--vw-panel);border:1px solid var(--vw-red);border-radius:10px;padding:16px 24px;" +
      "box-shadow:0 8px 30px rgba(0,0,0,0.4);max-width:480px;font-family:var(--vw-sans);";
    errDiv.innerHTML =
      '<div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">' +
        '<span style="color:var(--vw-red);font-size:18px">&#9888;</span>' +
        '<strong style="color:var(--vw-text);font-size:14px">' + escapeHtml(name) + '</strong>' +
      '</div>' +
      '<div style="color:var(--vw-red);font-size:13px;font-weight:600;margin-bottom:6px">' + escapeHtml(reason) + '</div>' +
      '<div style="color:var(--vw-text-dim);font-size:12px;white-space:pre-line;line-height:1.5;font-family:var(--vw-mono)">' + escapeHtml(hint) + '</div>' +
      '<button style="margin-top:10px;padding:4px 14px;border-radius:6px;border:1px solid var(--vw-border);' +
        'background:var(--vw-panel);color:var(--vw-text-dim);cursor:pointer;font-size:12px;font-family:var(--vw-sans)" ' +
        '">Dismiss</button>';
    document.body.appendChild(errDiv);
    errDiv.querySelector("button").addEventListener("click", function() { errDiv.remove(); });

    // Auto-dismiss after 10s
    setTimeout(function() { if (errDiv.parentElement) errDiv.remove(); }, 10000);
  }

  function addFile(name, size, text, truncated, fileRef) {
    // Chunked preview: for large files, only parse first N lines
    var fullText = null;
    var linePreview = null;
    if (text.length > MAX_PREVIEW_BYTES) {
      linePreview = previewText(text, MAX_PREVIEW_LINES);
      if (linePreview.truncated) {
        fullText = text; // keep reference for "Load Full File"
        text = linePreview.text;
        truncated = true;
      }
    }

    var parsed = parseFile(name, text);

    // Stage 3: Post-parse validation
    validateParsed(parsed, name);

    // Keep raw text only for raw view and browser bridge — for files >10MB, discard to save memory
    var keepText = size <= 10 * 1024 * 1024 ? text : null;
    var rawPreview = text.substring(0, 500000); // keep first 500K chars for raw view

    files.push({ name: name, size: size, text: keepText, rawPreview: rawPreview, parsed: parsed, truncated: truncated, fileRef: fileRef || null,
      _fullText: fullText, _previewLines: linePreview ? MAX_PREVIEW_LINES : null, _totalLines: linePreview ? linePreview.totalLines : null });
    dropZone.style.display = "none";
    workspace.classList.add("active");
    renderTabs();
    switchTab(files.length - 1);

    // Record in history DB
    var entryId = name + "_" + Date.now();
    var histStats = {};
    if (parsed.stats) {
      // Copy a few key stats for the history card
      var s = parsed.stats;
      if (s.sequences !== undefined) histStats.sequences = s.sequences;
      if (s.gc !== undefined) histStats.gc = s.gc;
      if (s.q30 !== undefined) histStats.q30 = s.q30;
      if (s.meanQual !== undefined) histStats.meanQual = s.meanQual;
      if (s.variants !== undefined) histStats.variants = s.variants;
      if (s.intervals !== undefined) histStats.intervals = s.intervals;
    }
    HistoryDB.addEntry({
      id: entryId,
      name: name,
      format: parsed.format || "unknown",
      size: size,
      rowCount: parsed.rows ? parsed.rows.length : 0,
      date: Date.now(),
      stats: histStats,
      content: size <= HistoryDB.SESSION_LIMIT ? text : null
    }).catch(function() {});
    localStorage.setItem("vw-last-session", entryId);

    // Show validation warnings if any
    if (parsed.warnings && parsed.warnings.length > 0) {
      showFileWarning(name, parsed.warnings);
    }

    // Show validation panel for format-specific issues
    var lastFile = files[files.length - 1];
    if (lastFile) {
      var vIssues = validateFile(lastFile);
      if (vIssues.length > 0) showValidationPanel(lastFile);
    }
  }

  function showFileWarning(name, warnings) {
    var existing = document.getElementById("vw-file-warning");
    if (existing) existing.remove();

    var warnDiv = document.createElement("div");
    warnDiv.id = "vw-file-warning";
    warnDiv.style.cssText = "position:fixed;top:80px;right:20px;z-index:500;" +
      "background:var(--vw-panel);border:1px solid var(--vw-amber);border-radius:10px;padding:14px 20px;" +
      "box-shadow:0 8px 30px rgba(0,0,0,0.3);max-width:400px;font-family:var(--vw-sans);";
    warnDiv.innerHTML =
      '<div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">' +
        '<span style="color:var(--vw-amber);font-size:16px">&#9888;</span>' +
        '<strong style="color:var(--vw-text);font-size:13px">' + escapeHtml(name) + '</strong>' +
      '</div>' +
      '<ul style="color:var(--vw-amber);font-size:12px;margin:0;padding-left:18px;line-height:1.6">' +
        warnings.map(function(w) { return '<li>' + escapeHtml(w) + '</li>'; }).join("") +
      '</ul>' +
      '<button style="margin-top:8px;padding:3px 12px;border-radius:6px;border:1px solid var(--vw-border);' +
        'background:var(--vw-panel);color:var(--vw-text-dim);cursor:pointer;font-size:11px;font-family:var(--vw-sans)" ' +
        '">Dismiss</button>';
    document.body.appendChild(warnDiv);
    warnDiv.querySelector("button").addEventListener("click", function() { warnDiv.remove(); });

    setTimeout(function() { if (warnDiv.parentElement) warnDiv.remove(); }, 8000);
  }

  // ── Tab management ─────────────────────────────────────────────
  // Feature 1: Tab color map by format
  var TAB_FORMAT_COLORS = {
    vcf: "#60a5fa", fasta: "#34d399", fastq: "#2dd4bf", bed: "#fbbf24",
    gff: "#a78bfa", sam: "#f87171", csv: "#fb923c", tsv: "#94a3b8"
  };

  function renderTabs() {
    // Remove all tabs except the add button
    var existing = tabsEl.querySelectorAll(".vw-tab");
    existing.forEach(function(el) { el.remove(); });

    files.forEach(function(f, i) {
      var tab = document.createElement("div");
      tab.className = "vw-tab" + (i === activeTab ? " active" : "");
      var info = FORMAT_MAP[f.parsed.format] || FORMAT_MAP.txt;
      // Feature 1: Colored left border by format
      var fmtColor = TAB_FORMAT_COLORS[f.parsed.format] || "#94a3b8";
      tab.style.borderLeft = "3px solid " + fmtColor;
      // Feature 6: Row count in tab title
      var rc = f.parsed.rows.length;
      var rcLabel = rc < 1000 ? String(rc) : rc < 1000000 ? (rc / 1000).toFixed(rc < 10000 ? 1 : 0) + "K" : (rc / 1000000).toFixed(1) + "M";
      var displayName = f.displayName || f.name;
      tab.innerHTML = '<span class="vw-tab-icon">' + info.icon + '</span>' +
        '<span class="vw-tab-label">' + escapeHtml(displayName) + ' <span style="color:var(--vw-text-muted);font-size:10px">(' + rcLabel + ')</span></span>' +
        '<span class="vw-tab-close" data-idx="' + i + '">&times;</span>';
      // Feature 2: Tab tooltip with file info
      var colCount = f.parsed.columns ? f.parsed.columns.length : 0;
      var parseTime = f.parsed.parseTime || "?";
      tab.title = f.parsed.format.toUpperCase() + " | " + formatBytes(f.size) + " | " +
        rc.toLocaleString() + " rows | " + colCount + " cols | parsed in " + parseTime + "s";
      tab.addEventListener("click", function(e) {
        if (e.target.classList.contains("vw-tab-close")) {
          closeTab(parseInt(e.target.dataset.idx));
        } else {
          switchTab(i);
        }
      });
      // Feature 3: Rename tab on double-click
      (function(tabIdx, tabEl) {
        var labelSpan = tabEl.querySelector ? null : null; // will query after insert
        tabEl.addEventListener("dblclick", function(e) {
          if (e.target.classList.contains("vw-tab-close")) return;
          var lbl = tabEl.querySelector(".vw-tab-label");
          if (!lbl) return;
          var curName = files[tabIdx].displayName || files[tabIdx].name;
          var input = document.createElement("input");
          input.type = "text";
          input.value = curName;
          input.style.cssText = "width:100px;font-size:11px;padding:1px 4px;border:1px solid var(--vw-accent);border-radius:3px;background:var(--vw-tab-bg);color:var(--vw-text);outline:none;font-family:var(--vw-sans);";
          input.addEventListener("click", function(ev) { ev.stopPropagation(); });
          input.addEventListener("dblclick", function(ev) { ev.stopPropagation(); });
          lbl.textContent = "";
          lbl.appendChild(input);
          input.focus();
          input.select();
          function commit() {
            var newName = input.value.trim();
            if (newName && newName !== files[tabIdx].name) {
              files[tabIdx].displayName = newName;
            } else {
              delete files[tabIdx].displayName;
            }
            renderTabs();
          }
          input.addEventListener("blur", commit);
          input.addEventListener("keydown", function(ev) {
            if (ev.key === "Enter") { ev.preventDefault(); commit(); }
            else if (ev.key === "Escape") { renderTabs(); }
          });
        });
      })(i, tab);
      // Feature 4: Tab context menu (right-click) for Duplicate
      (function(tabIdx) {
        tab.addEventListener("contextmenu", function(e) {
          e.preventDefault();
          closeContextMenu();
          var menu = document.createElement("div");
          menu.className = "vw-ctx-menu";
          menu.id = "vw-ctx-menu";
          var items = [
            { label: "Duplicate tab", action: function() {
              var src = files[tabIdx];
              var clone = {
                name: (src.displayName || src.name) + " (copy)",
                size: src.size,
                text: src.text,
                rawPreview: src.rawPreview,
                parsed: JSON.parse(JSON.stringify(src.parsed)),
                truncated: src.truncated
              };
              files.push(clone);
              renderTabs();
              switchTab(files.length - 1);
            }},
            { label: "Close tab", action: function() { closeTab(tabIdx); }},
            { label: "Close other tabs", action: function() {
              var keep = files[tabIdx];
              files.length = 0;
              files.push(keep);
              activeTab = 0;
              renderTabs();
              updateToolbar();
              renderView();
            }}
          ];
          items.forEach(function(item) {
            var div = document.createElement("div");
            div.className = "vw-ctx-item";
            div.textContent = item.label;
            div.addEventListener("click", function() { item.action(); closeContextMenu(); });
            menu.appendChild(div);
          });
          menu.style.left = Math.min(e.clientX, window.innerWidth - 200) + "px";
          menu.style.top = Math.min(e.clientY, window.innerHeight - items.length * 32 - 16) + "px";
          document.body.appendChild(menu);
          setTimeout(function() {
            document.addEventListener("mousedown", function handler(ev) {
              if (!menu.contains(ev.target)) { closeContextMenu(); document.removeEventListener("mousedown", handler); }
            });
          }, 0);
        });
      })(i);
      tabsEl.insertBefore(tab, addTabBtn);
    });
  }

  function switchTab(idx) {
    // Save current sort state before switching
    if (activeTab >= 0) {
      tabSortState[activeTab] = { col: sortCol, asc: sortAsc, sortCols: sortCols.slice() };
    }
    activeTab = idx;
    searchTerm = "";
    colFilters = {};
    motifTerm = "";
    currentPage = 0;
    groupByCol = -1;
    searchInput.value = "";
    var motifInput = document.getElementById("vw-motif-input");
    if (motifInput) motifInput.value = "";
    // Restore sort state for this tab
    var saved = tabSortState[idx];
    if (saved) { sortCol = saved.col; sortAsc = saved.asc; sortCols = saved.sortCols || []; }
    else { sortCol = -1; sortAsc = true; sortCols = []; }
    renderTabs();
    updateToolbar();
    renderView();
  }

  function closeTab(idx) {
    files.splice(idx, 1);
    if (files.length === 0) {
      workspace.classList.remove("active");
      dropZone.style.display = "";
      activeTab = -1;
      localStorage.removeItem("vw-last-session");
      renderRecentFiles();
    } else {
      if (activeTab >= files.length) activeTab = files.length - 1;
      if (activeTab === idx) activeTab = Math.max(0, idx - 1);
      else if (activeTab > idx) activeTab--;
    }
    renderTabs();
    if (activeTab >= 0) { updateToolbar(); renderView(); }
  }

  // ── Toolbar update ─────────────────────────────────────────────
  var BROWSER_FORMATS = { sam: 1, vcf: 1, bed: 1, gff: 1 };
  var browserBtn = document.getElementById("vw-browser-btn");

  function updateToolbar() {
    var f = files[activeTab];
    if (!f) return;
    var info = FORMAT_MAP[f.parsed.format] || FORMAT_MAP.txt;
    formatChip.textContent = f.parsed.format.toUpperCase();
    countChip.textContent = f.parsed.rows.length + " records";
    sizeChip.textContent = formatBytes(f.size);
    footerParse.textContent = "parsed in " + f.parsed.parseTime + "s";
    // Show "Open in Browser" for coordinate-based formats
    browserBtn.style.display = "none"; // BioBrowser bridge disabled — needs more testing
    // Show motif search for formats with sequence columns
    var hasSeq = f.parsed.colTypes.some(function(t) { return t === "seq"; });
    var motifInput = document.getElementById("vw-motif-input");
    if (motifInput) motifInput.style.display = hasSeq ? "" : "none";

    // Show diff button when 2+ same-format files are open
    var diffBtn = document.getElementById("vw-diff-btn");
    var mergeBtn = document.getElementById("vw-merge-btn");
    if (diffBtn || mergeBtn) {
      var sameCount = files.filter(function(other) { return other.parsed.format === f.parsed.format; }).length;
      if (diffBtn) diffBtn.style.display = sameCount >= 2 ? "" : "none";
      if (mergeBtn) mergeBtn.style.display = sameCount >= 2 ? "" : "none";
    }
  }

  // ── View rendering ─────────────────────────────────────────────
  function renderView() {
    contentEl.innerHTML = "";
    selectedRows.clear();
    var f = files[activeTab];
    if (!f) return;

    // Line-preview banner: show when file was truncated by line count
    if (f._fullText && f._previewLines) {
      var banner = document.createElement("div");
      banner.className = "vw-preview-banner";
      banner.style.cssText = "display:flex;align-items:center;justify-content:space-between;padding:8px 16px;" +
        "background:rgba(255,180,0,0.1);border:1px solid var(--vw-amber);border-radius:8px;margin:8px 12px;" +
        "font-family:var(--vw-sans);font-size:13px;color:var(--vw-amber);";
      var infoSpan = document.createElement("span");
      infoSpan.textContent = "Showing first " + f._previewLines.toLocaleString() + " lines of ~" + f._totalLines.toLocaleString() + " total (preview mode).";
      banner.appendChild(infoSpan);
      var loadBtn = document.createElement("button");
      loadBtn.className = "vw-tbtn";
      loadBtn.style.cssText = "margin-left:12px;padding:4px 14px;border-radius:6px;border:1px solid var(--vw-amber);" +
        "background:transparent;color:var(--vw-amber);cursor:pointer;font-size:12px;font-weight:600;font-family:var(--vw-sans);white-space:nowrap;";
      loadBtn.textContent = "Load Full File";
      loadBtn.addEventListener("click", function() {
        var idx = activeTab;
        var entry = files[idx];
        if (!entry || !entry._fullText) return;
        showLoadingOverlay(entry.name, entry.size);
        updateLoadingOverlay("Parsing full file...", 50);
        setTimeout(function() {
          var fullParsed = parseFile(entry.name, entry._fullText);
          validateParsed(fullParsed, entry.name);
          entry.parsed = fullParsed;
          entry.truncated = false;
          entry.text = entry._fullText.length <= 10 * 1024 * 1024 ? entry._fullText : null;
          entry.rawPreview = entry._fullText.substring(0, 500000);
          entry._fullText = null;
          entry._previewLines = null;
          entry._totalLines = null;
          hideLoadingOverlay();
          renderView();
          updateFooter(entry);
        }, 10);
      });
      banner.appendChild(loadBtn);
      contentEl.appendChild(banner);
    }

    if (splitMode && currentView === "table") {
      // Split view: table on left, stats on right
      var container = document.createElement("div");
      container.className = "vw-split-container";
      var left = document.createElement("div");
      left.className = "vw-split-left";
      var right = document.createElement("div");
      right.className = "vw-split-right";
      container.appendChild(left);
      container.appendChild(right);
      contentEl.appendChild(container);

      // Render table into left pane
      var origContent = contentEl;
      contentEl = left;
      var strip = renderChromStrip(f);
      if (strip) contentEl.appendChild(strip);
      renderTableView(f);
      contentEl = origContent;

      // Render stats into right pane
      var statsGrid = document.createElement("div");
      statsGrid.className = "vw-stat-grid";
      if (f.parsed.stats) {
        Object.keys(f.parsed.stats).forEach(function(key) {
          var card = document.createElement("div");
          card.className = "vw-stat-card";
          card.innerHTML = '<div class="vw-stat-label">' + escapeHtml(key) + '</div>' +
            '<div class="vw-stat-value">' + escapeHtml(String(f.parsed.stats[key])) + '</div>';
          statsGrid.appendChild(card);
        });
      }
      right.innerHTML = '<div style="padding:12px 16px;font-family:var(--vw-sans);font-size:13px;font-weight:600;color:var(--vw-accent);border-bottom:1px solid var(--vw-border)">Summary</div>';
      right.appendChild(statsGrid);
    } else if (transposeMode && currentView === "table") {
      renderTransposeView(f);
    } else {
      switch (currentView) {
        case "table":
          var strip = renderChromStrip(f);
          if (strip) contentEl.appendChild(strip);
          renderTableView(f);
          break;
        case "stats": renderStatsView(f); break;
        case "raw": renderRawView(f); break;
        case "console": renderConsoleView(f); break;
      }
    }
    updateFooter(f);
  }

  function renderTransposeView(f) {
    var wrap = document.createElement("div");
    wrap.className = "vw-table-wrap";
    var table = document.createElement("table");
    table.className = "vw-table";
    var rows = getFilteredRows(f);
    var maxCols = Math.min(rows.length, 50); // Show first 50 rows as columns

    // Transpose indicator
    var notice = document.createElement("div");
    notice.style.cssText = "padding:6px 12px;font-family:var(--vw-sans);font-size:11px;color:var(--vw-accent);background:var(--vw-tab-bg);border-bottom:1px solid var(--vw-border);display:flex;justify-content:space-between;align-items:center;";
    notice.innerHTML = '<span>Transpose View — columns are rows (Ctrl+T to toggle)</span>' +
      '<span style="color:var(--vw-text-muted)">Showing ' + maxCols + ' of ' + rows.length + ' rows</span>';
    wrap.appendChild(notice);

    // Each original column becomes a row
    f.parsed.columns.forEach(function(col, ci) {
      if (hiddenCols[activeTab] && hiddenCols[activeTab].has(ci)) return;
      var tr = document.createElement("tr");
      var th = document.createElement("th");
      th.textContent = col;
      th.style.cssText = "position:sticky;left:0;background:var(--vw-panel);z-index:1;min-width:120px;font-weight:600;";
      tr.appendChild(th);

      for (var ri = 0; ri < maxCols; ri++) {
        var td = document.createElement("td");
        var val = rows[ri].row[ci];
        var colType = f.parsed.colTypes[ci];
        if (colType === "seq") {
          td.className = "seq-cell";
          td.innerHTML = colorSequence(String(val).substring(0, 40));
        } else if (colType === "num") {
          td.className = "num-cell";
          td.textContent = typeof val === "number" ? val.toLocaleString() : val;
        } else {
          td.textContent = String(val).substring(0, 60);
        }
        tr.appendChild(td);
      }
      table.appendChild(tr);
    });

    wrap.appendChild(table);
    contentEl.appendChild(wrap);
  }

  function showColumnFilter(f, colIndex, colName, anchorEl) {
    // Remove existing dropdown
    var existing = document.getElementById("vw-col-filter");
    if (existing) existing.remove();

    // Collect unique values (sample up to 10000 rows)
    var uniqMap = {};
    var n = Math.min(f.parsed.rows.length, 10000);
    for (var i = 0; i < n; i++) {
      var v = String(f.parsed.rows[i][colIndex]);
      uniqMap[v] = (uniqMap[v] || 0) + 1;
    }
    var entries = Object.entries(uniqMap).sort(function(a, b) { return b[1] - a[1]; });
    if (entries.length > 200) entries = entries.slice(0, 200);

    var currentFilter = colFilters[colIndex];

    var drop = document.createElement("div");
    drop.id = "vw-col-filter";
    drop.style.cssText = "position:absolute;z-index:200;background:var(--vw-panel);border:1px solid var(--vw-border);" +
      "border-radius:8px;padding:8px;box-shadow:0 8px 30px rgba(0,0,0,0.4);max-height:320px;width:220px;" +
      "display:flex;flex-direction:column;font-family:var(--vw-sans);font-size:12px;";

    // Header
    var hdr = document.createElement("div");
    hdr.style.cssText = "display:flex;justify-content:space-between;align-items:center;padding:0 4px 6px;border-bottom:1px solid var(--vw-border);margin-bottom:6px;";
    hdr.innerHTML = '<span style="color:var(--vw-accent);font-weight:600">' + escapeHtml(colName) + '</span>';
    var clearBtn = document.createElement("button");
    clearBtn.textContent = "Clear";
    clearBtn.style.cssText = "background:none;border:none;color:var(--vw-text-dim);cursor:pointer;font-size:11px;padding:2px 6px;";
    clearBtn.addEventListener("click", function() {
      delete colFilters[colIndex];
      drop.remove();
      renderView();
    });
    hdr.appendChild(clearBtn);
    drop.appendChild(hdr);

    // Action buttons: Hide, Heatmap, Group by
    var actions = document.createElement("div");
    actions.style.cssText = "display:flex;gap:4px;padding:0 4px 6px;border-bottom:1px solid var(--vw-border);margin-bottom:6px;flex-wrap:wrap;";

    var hideBtn = document.createElement("button");
    hideBtn.textContent = "Hide column";
    hideBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-text);cursor:pointer;font-size:10px;";
    hideBtn.addEventListener("click", function() {
      pushUndo();
      if (!hiddenCols[activeTab]) hiddenCols[activeTab] = new Set();
      hiddenCols[activeTab].add(colIndex);
      drop.remove();
      renderView();
    });
    actions.appendChild(hideBtn);

    if (f.parsed.colTypes[colIndex] === "num") {
      // Quick histogram popup
      var chartBtn = document.createElement("button");
      chartBtn.textContent = "Chart";
      chartBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-cyan);cursor:pointer;font-size:10px;";
      chartBtn.addEventListener("click", function() {
        var vals = [];
        for (var ri = 0; ri < f.parsed.rows.length; ri++) {
          var v = f.parsed.rows[ri][colIndex];
          if (typeof v === "number" && !isNaN(v)) vals.push(v);
        }
        drop.remove();
        showQuickChart(colName, vals);
      });
      actions.appendChild(chartBtn);

      var histBtn = document.createElement("button");
      histBtn.textContent = "Quick Histogram";
      histBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-green);cursor:pointer;font-size:10px;";
      histBtn.addEventListener("click", function() {
        drop.remove();
        showQuickHistogram(f, colIndex);
      });
      actions.appendChild(histBtn);

      var heatBtn = document.createElement("button");
      var isHeat = heatmapCols[colIndex] && heatmapCols[colIndex].enabled;
      heatBtn.textContent = isHeat ? "Remove heatmap" : "Heatmap";
      heatBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:" + (isHeat ? "var(--vw-green)" : "var(--vw-text)") + ";cursor:pointer;font-size:10px;";
      heatBtn.addEventListener("click", function() {
        if (isHeat) {
          delete heatmapCols[colIndex];
        } else {
          var vals = [];
          for (var ri = 0; ri < f.parsed.rows.length; ri++) {
            var v = f.parsed.rows[ri][colIndex];
            if (typeof v === "number" && !isNaN(v)) vals.push(v);
          }
          if (vals.length) {
            heatmapCols[colIndex] = { min: Math.min.apply(null, vals), max: Math.max.apply(null, vals), enabled: true };
          }
        }
        drop.remove();
        renderView();
      });
      actions.appendChild(heatBtn);
    }

    // Feature 4: Pin/Unpin column
    var pinBtn = document.createElement("button");
    pinBtn.textContent = pinnedCols.has(colIndex) ? "Unpin column" : "Pin column";
    pinBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-text);cursor:pointer;font-size:10px;";
    pinBtn.addEventListener("click", function() {
      if (pinnedCols.has(colIndex)) pinnedCols.delete(colIndex);
      else pinnedCols.add(colIndex);
      drop.remove();
      renderView();
    });
    actions.appendChild(pinBtn);

    // Feature 19: Column type override
    var typeNumBtn = document.createElement("button");
    typeNumBtn.textContent = "Treat as Number";
    typeNumBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-cyan);cursor:pointer;font-size:10px;";
    typeNumBtn.addEventListener("click", function() {
      pushUndo();
      f.parsed.colTypes[colIndex] = "num";
      for (var ri = 0; ri < f.parsed.rows.length; ri++) {
        var v = parseFloat(f.parsed.rows[ri][colIndex]);
        if (!isNaN(v)) f.parsed.rows[ri][colIndex] = v;
      }
      f.parsed._summaryCache = null;
      f.parsed._colHints = null;
      _filterCache = null;
      drop.remove();
      renderView();
    });
    actions.appendChild(typeNumBtn);

    var typeTextBtn = document.createElement("button");
    typeTextBtn.textContent = "Treat as Text";
    typeTextBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-text);cursor:pointer;font-size:10px;";
    typeTextBtn.addEventListener("click", function() {
      pushUndo();
      f.parsed.colTypes[colIndex] = "str";
      for (var ri = 0; ri < f.parsed.rows.length; ri++) {
        f.parsed.rows[ri][colIndex] = String(f.parsed.rows[ri][colIndex]);
      }
      f.parsed._summaryCache = null;
      f.parsed._colHints = null;
      _filterCache = null;
      drop.remove();
      renderView();
    });
    actions.appendChild(typeTextBtn);

    var grpBtn = document.createElement("button");
    grpBtn.textContent = groupByCol === colIndex ? "Ungroup" : "Group by";
    grpBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-text);cursor:pointer;font-size:10px;";
    grpBtn.addEventListener("click", function() {
      groupByCol = groupByCol === colIndex ? -1 : colIndex;
      currentPage = 0;
      drop.remove();
      renderView();
    });
    actions.appendChild(grpBtn);

    // Feature 7: Copy entire column values
    var copyColBtn = document.createElement("button");
    copyColBtn.textContent = "Copy column";
    copyColBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-text);cursor:pointer;font-size:10px;";
    copyColBtn.addEventListener("click", function() {
      var filtered = getFilteredRows(f);
      var vals = filtered.map(function(item) { return String(item.row[colIndex]); });
      navigator.clipboard.writeText(vals.join("\n"));
      showToast("Copied " + vals.length + " values from " + colName);
      drop.remove();
    });
    actions.appendChild(copyColBtn);

    // Feature 8: Show unique values
    var uniqBtn = document.createElement("button");
    uniqBtn.textContent = "Unique values";
    uniqBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-accent);cursor:pointer;font-size:10px;";
    uniqBtn.addEventListener("click", function() {
      drop.remove();
      showUniqueValuesPopup(f, colIndex, colName);
    });
    actions.appendChild(uniqBtn);

    // IGV/UCSC link for genomic columns
    var colNameLower = colName.toLowerCase();
    if (colNameLower === "chrom" || colNameLower === "chr" || colNameLower === "#chrom") {
      var ucscBtn = document.createElement("button");
      ucscBtn.textContent = "UCSC Browser";
      ucscBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-cyan);cursor:pointer;font-size:10px;";
      ucscBtn.addEventListener("click", function() {
        var row = f.parsed.rows[0];
        if (row) {
          var chromVal = row[colIndex];
          var posIdx = f.parsed.columns.findIndex(function(c) { return /^(pos|start|chromstart)/i.test(c); });
          var pos = posIdx >= 0 ? row[posIdx] : 1;
          var endIdx = f.parsed.columns.findIndex(function(c) { return /^(end|chromend)/i.test(c); });
          var endPos = endIdx >= 0 ? row[endIdx] : parseInt(pos) + 1000;
          window.open("https://genome.ucsc.edu/cgi-bin/hgTracks?db=hg38&position=" + chromVal + ":" + pos + "-" + endPos, "_blank");
        }
        drop.remove();
      });
      actions.appendChild(ucscBtn);
    }

    // Computed column options
    var computeOpts = [];
    if (f.parsed.colTypes[colIndex] === "seq") {
      computeOpts.push({ label: "+ GC%", type: "gc" });
      computeOpts.push({ label: "+ Length", type: "length" });
    } else if (f.parsed.colTypes[colIndex] === "num") {
      computeOpts.push({ label: "+ log₂", type: "log2" });
    } else {
      computeOpts.push({ label: "+ Length", type: "length" });
      computeOpts.push({ label: "+ UPPER", type: "upper" });
    }
    computeOpts.forEach(function(opt) {
      var btn = document.createElement("button");
      btn.textContent = opt.label;
      btn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-green);cursor:pointer;font-size:10px;";
      btn.addEventListener("click", function() { pushUndo(); addComputedColumn(f, opt.type, colIndex); drop.remove(); });
      actions.appendChild(btn);
    });

    drop.appendChild(actions);

    // Show hidden columns restore button if any are hidden
    if (hiddenCols[activeTab] && hiddenCols[activeTab].size > 0) {
      var restoreBtn = document.createElement("button");
      restoreBtn.textContent = "Show all " + hiddenCols[activeTab].size + " hidden cols";
      restoreBtn.style.cssText = "background:none;border:none;color:var(--vw-accent);cursor:pointer;font-size:10px;padding:0 4px 6px;text-decoration:underline;";
      restoreBtn.addEventListener("click", function() {
        hiddenCols[activeTab] = new Set();
        drop.remove();
        renderView();
      });
      drop.appendChild(restoreBtn);
    }

    // Scrollable list
    var list = document.createElement("div");
    list.style.cssText = "overflow-y:auto;max-height:250px;";
    entries.forEach(function(entry) {
      var val = entry[0], count = entry[1];
      var label = document.createElement("label");
      label.style.cssText = "display:flex;align-items:center;gap:6px;padding:3px 4px;cursor:pointer;border-radius:4px;color:var(--vw-text);";
      label.addEventListener("mouseenter", function() { label.style.background = "var(--vw-row-hover)"; });
      label.addEventListener("mouseleave", function() { label.style.background = ""; });
      var cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = !currentFilter || currentFilter.has(val);
      cb.style.cssText = "accent-color:var(--vw-accent);";
      cb.addEventListener("change", function() {
        if (!colFilters[colIndex]) {
          colFilters[colIndex] = new Set(entries.map(function(e) { return e[0]; }));
        }
        if (cb.checked) colFilters[colIndex].add(val);
        else colFilters[colIndex].delete(val);
        // If all are checked, remove filter
        if (colFilters[colIndex].size >= entries.length) delete colFilters[colIndex];
        renderView();
        searchInput.focus(); // don't steal focus
      });
      label.appendChild(cb);
      var text = document.createElement("span");
      text.style.cssText = "flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;";
      text.textContent = val.length > 25 ? val.substring(0, 24) + "\u2026" : val;
      text.title = val;
      label.appendChild(text);
      var cnt = document.createElement("span");
      cnt.style.cssText = "color:var(--vw-text-muted);font-size:10px;font-family:var(--vw-mono);";
      cnt.textContent = count.toLocaleString();
      label.appendChild(cnt);
      list.appendChild(label);
    });
    drop.appendChild(list);

    // Position relative to header cell
    var rect = anchorEl.getBoundingClientRect();
    drop.style.left = Math.min(rect.left, window.innerWidth - 240) + "px";
    drop.style.top = rect.bottom + "px";
    drop.style.position = "fixed";
    document.body.appendChild(drop);

    // Close on click outside
    function closeFilter(e) {
      if (!drop.contains(e.target)) {
        drop.remove();
        document.removeEventListener("mousedown", closeFilter);
      }
    }
    setTimeout(function() { document.addEventListener("mousedown", closeFilter); }, 0);
  }

  function renderTableView(f) {
    var wrap = document.createElement("div");
    wrap.className = "vw-table-wrap";

    var table = document.createElement("table");
    table.className = "vw-table";

    // Header
    var thead = document.createElement("thead");
    var tr = document.createElement("tr");
    // Bookmark column (if enabled)
    if (bookmarkMode) {
      var bmTh = document.createElement("th");
      bmTh.textContent = "★";
      bmTh.style.width = "30px";
      bmTh.style.textAlign = "center";
      bmTh.style.cursor = "pointer";
      bmTh.title = "Click to export bookmarked rows";
      bmTh.addEventListener("click", function() { exportBookmarked(f); });
      tr.appendChild(bmTh);
    }

    // Row number column (Feature 6: toggleable)
    var rowTh = document.createElement("th");
    rowTh.textContent = "#";
    rowTh.style.width = "50px";
    rowTh.style.textAlign = "right";
    if (!showRowNumbers) rowTh.style.display = "none";
    tr.appendChild(rowTh);

    // Pre-compute column hints (cached per file)
    if (!f.parsed._colHints) {
      f.parsed._colHints = f.parsed.columns.map(function(col, ci) {
        var type = f.parsed.colTypes[ci];
        if (type === "num") {
          var vals = [];
          var count = Math.min(f.parsed.rows.length, 10000);
          for (var ri = 0; ri < count; ri++) {
            var v = f.parsed.rows[ri][ci];
            if (typeof v === "number" && !isNaN(v)) vals.push(v);
          }
          if (!vals.length) return col + " (numeric)";
          vals.sort(function(a, b) { return a - b; });
          var sum = vals.reduce(function(a, b) { return a + b; }, 0);
          var mean = sum / vals.length;
          var med = vals.length % 2 === 0
            ? (vals[vals.length / 2 - 1] + vals[vals.length / 2]) / 2
            : vals[Math.floor(vals.length / 2)];
          return col + " (numeric)\nMin: " + vals[0].toLocaleString() +
            "  Max: " + vals[vals.length - 1].toLocaleString() +
            "\nMean: " + mean.toLocaleString(undefined, {maximumFractionDigits: 2}) +
            "  Median: " + med.toLocaleString(undefined, {maximumFractionDigits: 2}) +
            "\n" + vals.length.toLocaleString() + " values";
        } else if (type === "seq") {
          return col + " (sequence)\nClick to sort";
        } else if (type === "qual") {
          return col + " (quality scores)\nClick to sort";
        } else {
          var uniq = new Set();
          var count = Math.min(f.parsed.rows.length, 5000);
          for (var ri = 0; ri < count; ri++) {
            uniq.add(f.parsed.rows[ri][ci]);
            if (uniq.size > 100) break;
          }
          return col + " (text)\n" + (uniq.size > 100 ? "100+" : uniq.size) + " unique values\nClick to sort";
        }
      });
    }

    f.parsed.columns.forEach(function(col, ci) {
      var th = document.createElement("th");
      // Feature 3 & 7: Sort indicator with multi-sort priority
      var arrow = "";
      var isSorted = false;
      if (sortCols.length > 1) {
        for (var si = 0; si < sortCols.length; si++) {
          if (sortCols[si].col === ci) {
            var circled = ["\u2460","\u2461","\u2462","\u2463","\u2464","\u2465","\u2466","\u2467","\u2468"];
            arrow = " " + (circled[si] || (si + 1)) + (sortCols[si].asc ? "\u25B2" : "\u25BC");
            isSorted = true;
            break;
          }
        }
      } else if (sortCol === ci) {
        arrow = sortAsc ? " \u25B2" : " \u25BC";
        isSorted = true;
      }
      // Feature 5: Inline filter icon
      var filterIcon = '<span class="col-filter-icon" style="margin-left:4px;cursor:pointer;opacity:0.4;font-size:10px" title="Filter this column">\u25BC</span>';
      th.innerHTML = escapeHtml(col) + '<span class="sort-arrow">' + arrow + '</span>' + filterIcon;
      if (isSorted) th.classList.add("sorted");
      if (colFilters[ci]) th.style.borderBottom = "2px solid var(--vw-green)";
      if (f.parsed.colTypes[ci] === "num") th.classList.add("num-col");
      th.title = f.parsed._colHints[ci];
      th.addEventListener("click", function(e) {
        if (e.target.classList.contains("col-resizer")) return;
        if (e.target.classList.contains("col-filter-icon")) return;
        pushUndo();
        if (e.shiftKey) {
          // Feature 7: Multi-column sort — Shift+click adds secondary sort
          var existing = -1;
          for (var si = 0; si < sortCols.length; si++) {
            if (sortCols[si].col === ci) { existing = si; break; }
          }
          if (existing >= 0) {
            if (sortCols[existing].asc) sortCols[existing].asc = false;
            else sortCols.splice(existing, 1);
          } else {
            sortCols.push({ col: ci, asc: true });
          }
          // Keep legacy sortCol in sync with first entry
          if (sortCols.length > 0) { sortCol = sortCols[0].col; sortAsc = sortCols[0].asc; }
          else { sortCol = -1; sortAsc = true; }
        } else {
          // Regular click: single-column sort
          if (sortCol === ci) {
            if (sortAsc) { sortAsc = false; }
            else { sortCol = -1; sortAsc = true; }
          } else {
            sortCol = ci; sortAsc = true;
          }
          sortCols = sortCol >= 0 ? [{ col: sortCol, asc: sortAsc }] : [];
        }
        currentPage = 0;
        if (sortCol >= 0) tabSortState[activeTab] = { col: sortCol, asc: sortAsc, sortCols: sortCols.slice() };
        else delete tabSortState[activeTab];
        renderView();
      });
      // Feature 5: Inline filter icon click handler
      (function(colIndex, colName, thEl) {
        var filterBtn = thEl.querySelector(".col-filter-icon");
        if (filterBtn) {
          filterBtn.addEventListener("click", function(e) {
            e.stopPropagation();
            showColumnFilter(f, colIndex, colName, thEl);
          });
        }
      })(ci, col, th);
      // Column resizer handle
      var resizer = document.createElement("div");
      resizer.className = "col-resizer";
      resizer.addEventListener("mousedown", function(e) {
        e.preventDefault(); e.stopPropagation();
        resizer.classList.add("resizing");
        var startX = e.clientX;
        var startW = th.offsetWidth;
        function onMove(ev) {
          var newW = Math.max(40, startW + ev.clientX - startX);
          th.style.width = newW + "px";
          th.style.minWidth = newW + "px";
        }
        function onUp() {
          resizer.classList.remove("resizing");
          document.removeEventListener("mousemove", onMove);
          document.removeEventListener("mouseup", onUp);
        }
        document.addEventListener("mousemove", onMove);
        document.addEventListener("mouseup", onUp);
      });
      th.appendChild(resizer);
      // Right-click for column filter dropdown
      (function(colIndex, colName) {
        th.addEventListener("contextmenu", function(e) {
          e.preventDefault();
          showColumnFilter(f, colIndex, colName, th);
        });
      })(ci, col);
      // Column drag reorder (Feature 10)
      setupColumnDrag(th, ci, f);
      // Feature 4: Pin columns — custom pinned set or legacy toggle
      if (pinnedCols.has(ci) || (pinColumnsMode && ci < 2 && f.parsed.columns.length > 6)) {
        th.classList.add("pinned");
        // Calculate cumulative left offset for multiple pinned columns
        var leftPx = 0;
        if (ci > 0) {
          var preceding = document.querySelectorAll(".vw-table thead tr:first-child th.pinned");
          preceding.forEach(function(pth) { leftPx += pth.offsetWidth || 80; });
        }
        th.style.left = leftPx + "px";
      }
      // Hidden columns
      if (hiddenCols[activeTab] && hiddenCols[activeTab].has(ci)) th.style.display = "none";
      tr.appendChild(th);
    });
    thead.appendChild(tr);

    // Frozen summary row — cached per file, computed over ALL rows for accurate stats
    if (!f.parsed._summaryCache) {
      f.parsed._summaryCache = f.parsed.columns.map(function(col, ci) {
        var colType = f.parsed.colTypes[ci];
        var totalRows = f.parsed.rows.length;
        if (colType === "num") {
          var sum = 0, count = 0, mn = Infinity, mx = -Infinity;
          for (var ri = 0; ri < totalRows; ri++) {
            var v = f.parsed.rows[ri][ci];
            if (typeof v === "number" && !isNaN(v)) { sum += v; count++; if (v < mn) mn = v; if (v > mx) mx = v; }
          }
          if (count) return { text: "\u03bc" + (sum / count).toFixed(1), title: "Min: " + mn + " Max: " + mx + " Mean: " + (sum / count).toFixed(2) + " n=" + count + "/" + totalRows, type: "num" };
          return { text: "", title: "", type: "num" };
        } else if (colType === "seq") {
          return { text: "seq", title: "", type: "seq" };
        } else {
          var uniq = new Set();
          for (var ri = 0; ri < totalRows; ri++) uniq.add(f.parsed.rows[ri][ci]);
          return { text: (uniq.size > 100 ? "100+" : uniq.size) + " uniq", title: uniq.size + " unique values in " + totalRows + " rows", type: "text" };
        }
      });
    }
    var summaryTr = document.createElement("tr");
    summaryTr.className = "vw-summary-row";
    summaryTr.style.cssText = "background:var(--vw-tab-bg);font-size:10px;color:var(--vw-text-dim);border-bottom:2px solid var(--vw-border);position:sticky;top:0;z-index:2;";
    if (bookmarkMode) {
      var sTd = document.createElement("td");
      sTd.style.width = "30px";
      summaryTr.appendChild(sTd);
    }
    var sNumTd = document.createElement("td");
    var ff = files[activeTab];
    if (ff && ff.truncated) {
      sNumTd.textContent = "\u03a3~";
      sNumTd.style.cssText = "text-align:right;width:50px;font-weight:600;color:var(--vw-amber);";
      sNumTd.title = "Stats are approximate — file truncated to first 50 MB of " + formatBytes(ff.size);
    } else {
      sNumTd.textContent = "\u03a3";
      sNumTd.style.cssText = "text-align:right;width:50px;font-weight:600;color:var(--vw-accent);";
    }
    if (!showRowNumbers) sNumTd.style.display = "none";
    summaryTr.appendChild(sNumTd);
    f.parsed._summaryCache.forEach(function(sc, ci) {
      var td = document.createElement("td");
      td.style.cssText = "font-family:var(--vw-mono);padding:2px 6px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:150px;";
      td.textContent = sc.text;
      if (sc.title) td.title = sc.title;
      if (sc.type === "seq") td.style.color = "var(--vw-green)";
      if (hiddenCols[activeTab] && hiddenCols[activeTab].has(ci)) td.style.display = "none";
      summaryTr.appendChild(td);
    });
    thead.appendChild(summaryTr);
    table.appendChild(thead);

    // Body
    var tbody = document.createElement("tbody");
    var rows = getFilteredRows(f);

    // Pre-compute anomaly set for row highlighting
    var anomalySet = {};
    if (!f.parsed._anomalyCache) f.parsed._anomalyCache = detectAnomalies(f);
    f.parsed._anomalyCache.forEach(function(a) { anomalySet[a.row] = a; });

    // Sort — Feature 7: Multi-column sort
    if (sortCols.length > 0) {
      rows = rows.slice().sort(function(a, b) {
        for (var si = 0; si < sortCols.length; si++) {
          var sc = sortCols[si];
          var va = a.row[sc.col], vb = b.row[sc.col];
          var cmp = 0;
          if (typeof va === "number" && typeof vb === "number") cmp = va - vb;
          else cmp = String(va).localeCompare(String(vb));
          if (cmp !== 0) return sc.asc ? cmp : -cmp;
        }
        return 0;
      });
    } else if (sortCol >= 0) {
      rows = rows.slice().sort(function(a, b) {
        var va = a.row[sortCol], vb = b.row[sortCol];
        if (typeof va === "number" && typeof vb === "number") return sortAsc ? va - vb : vb - va;
        va = String(va); vb = String(vb);
        return sortAsc ? va.localeCompare(vb) : vb.localeCompare(va);
      });
    }

    // Sort by group column first if grouping is active
    if (groupByCol >= 0 && groupByCol < f.parsed.columns.length) {
      rows = rows.slice().sort(function(a, b) {
        var ga = String(a.row[groupByCol]), gb = String(b.row[groupByCol]);
        if (ga !== gb) return ga.localeCompare(gb);
        if (sortCol >= 0) {
          var va = a.row[sortCol], vb = b.row[sortCol];
          if (typeof va === "number" && typeof vb === "number") return sortAsc ? va - vb : vb - va;
          return sortAsc ? String(va).localeCompare(String(vb)) : String(vb).localeCompare(String(va));
        }
        return 0;
      });
    }

    // Column name lookup for format-specific coloring
    var colIdx = {};
    f.parsed.columns.forEach(function(c, i) { colIdx[c] = i; });
    var isSam = f.parsed.format === "sam";
    var isVcf = f.parsed.format === "vcf";
    var isFasta = f.parsed.format === "fasta";

    // Pagination: slice rows by current page
    var totalFiltered = rows.length;
    var totalPages = Math.ceil(totalFiltered / pageSize) || 1;
    if (currentPage >= totalPages) currentPage = totalPages - 1;
    if (currentPage < 0) currentPage = 0;
    var pageStart = currentPage * pageSize;
    var pageEnd = Math.min(pageStart + pageSize, totalFiltered);
    var pageRows = rows.slice(pageStart, pageEnd);

    // Group-by tracking — pre-compute counts in O(n) once
    var lastGroupVal = null;
    var visibleColCount = f.parsed.columns.length + 1 + (bookmarkMode ? 1 : 0);
    var grpCounts = null;
    if (groupByCol >= 0 && groupByCol < f.parsed.columns.length) {
      grpCounts = {};
      for (var gi = 0; gi < rows.length; gi++) {
        var gk = String(rows[gi].row[groupByCol]);
        grpCounts[gk] = (grpCounts[gk] || 0) + 1;
      }
    }

    for (var i = 0; i < pageRows.length; i++) {
      var item = pageRows[i];

      // Group separator row
      if (grpCounts) {
        var grpVal = String(item.row[groupByCol]);
        if (grpVal !== lastGroupVal) {
          lastGroupVal = grpVal;
          var grpRow = document.createElement("tr");
          grpRow.className = "vw-group-row";
          grpRow.style.cssText = "background:var(--vw-tab-bg);border-top:2px solid var(--vw-accent);";
          var grpTd = document.createElement("td");
          grpTd.colSpan = visibleColCount;
          grpTd.style.cssText = "padding:4px 12px;font-family:var(--vw-sans);font-size:11px;font-weight:600;color:var(--vw-accent);";
          grpTd.textContent = f.parsed.columns[groupByCol] + ": " + grpVal + " (" + (grpCounts[grpVal] || 0) + " rows)";
          grpRow.appendChild(grpTd);
          tbody.appendChild(grpRow);
        }
      }

      var row = document.createElement("tr");

      // Row anomaly highlighting
      if (isSam) {
        var samFlag = item.row[colIdx["FLAG"]];
        if (samFlag & 4) row.className = "row-unmapped";
        else if (item.row[colIdx["MAPQ"]] < 10) row.className = "row-lowmapq";
      }
      if (isVcf && colIdx["FILTER"] !== undefined) {
        var filt = String(item.row[colIdx["FILTER"]]);
        if (filt !== "PASS" && filt !== ".") row.className = "row-fail-filter";
      }

      // Anomaly highlighting
      if (anomalySet[item.idx]) {
        row.classList.add("row-anomaly");
        row.title = anomalySet[item.idx].msg;
      }

      // Conditional highlighting
      if (highlightRule && matchesHighlightRule(item.row)) {
        row.style.borderLeft = "3px solid #f59e0b";
        row.style.background = "rgba(245,158,11,0.08)";
      }

      // Bookmark styling
      var bms = bookmarkedRows[activeTab] || new Set();
      if (bms.has(item.idx)) row.classList.add("bookmarked");

      // Click row for detail (if enabled)
      if (rowDetailEnabled) row.style.cursor = "pointer";
      (function(idx) {
        row.addEventListener("click", function(e) {
          if (e.target.classList.contains("bookmark-star")) return;
          if (rowDetailEnabled) showDetailPanel(f, idx);
        });
      })(item.idx);

      // Bookmark star column
      if (bookmarkMode) {
        var bmTd = document.createElement("td");
        bmTd.style.textAlign = "center";
        bmTd.style.width = "30px";
        var star = document.createElement("span");
        star.className = "bookmark-star" + (bms.has(item.idx) ? " active" : "");
        star.textContent = bms.has(item.idx) ? "★" : "☆";
        (function(idx, starEl) {
          starEl.addEventListener("click", function(e) {
            e.stopPropagation();
            if (!bookmarkedRows[activeTab]) bookmarkedRows[activeTab] = new Set();
            if (bookmarkedRows[activeTab].has(idx)) {
              bookmarkedRows[activeTab].delete(idx);
              starEl.textContent = "☆";
              starEl.classList.remove("active");
              starEl.closest("tr").classList.remove("bookmarked");
            } else {
              bookmarkedRows[activeTab].add(idx);
              starEl.textContent = "★";
              starEl.classList.add("active");
              starEl.closest("tr").classList.add("bookmarked");
            }
          });
        })(item.idx, star);
        bmTd.appendChild(star);
        row.appendChild(bmTd);
      }

      // Row number (Feature 6: toggleable)
      var numTd = document.createElement("td");
      numTd.textContent = item.idx + 1;
      numTd.className = "num-cell";
      numTd.style.width = "50px";
      if (!showRowNumbers) numTd.style.display = "none";
      row.appendChild(numTd);

      for (var ci = 0; ci < f.parsed.columns.length; ci++) {
        var td = document.createElement("td");
        var val = item.row[ci];
        var colType = f.parsed.colTypes[ci];
        var colName = f.parsed.columns[ci];

        // Format-specific coloring for SAM/VCF columns
        if (colName === "FLAG" && isSam) {
          td.innerHTML = colorFlag(val);
        } else if (colName === "CIGAR" && isSam) {
          td.innerHTML = colorCigar(String(val));
        } else if (colName === "FILTER" && isVcf) {
          td.innerHTML = colorFilter(val);
        } else if (colType === "seq") {
          td.className = "seq-cell";
          var seqStr = String(val).substring(0, 120);
          if (motifTerm && motifTerm.length > 0) {
            td.innerHTML = colorSequenceWithMotif(seqStr, motifTerm);
          } else if (isFasta && seqStr.length > 6) {
            td.innerHTML = colorSequenceWithCodons(seqStr);
          } else {
            td.innerHTML = colorSequence(seqStr);
          }
          if (String(val).length > 120) td.innerHTML += '<span style="color:var(--vw-text-muted)">... (' + String(val).length + ' bp)</span>';
        } else if (colType === "qual") {
          td.className = "qual-cell";
          var qstr = String(val);
          // FASTQ quality heatmap bar
          if (f.parsed.format === "fastq" && (colName === "quality" || colName === "Quality")) {
            var qualBar = "";
            for (var qi = 0; qi < Math.min(qstr.length, 50); qi++) {
              var q = qstr.charCodeAt(qi) - 33;
              var color = q >= 30 ? "#4ade80" : q >= 20 ? "#fbbf24" : "#f87171";
              qualBar += '<span style="display:inline-block;width:3px;height:12px;background:' + color + ';margin:0 0.5px;border-radius:1px" title="Q' + q + '"></span>';
            }
            td.innerHTML = '<div style="display:flex;align-items:center;gap:4px"><span style="font-size:10px;color:var(--vw-text-muted)">' + qstr.length + 'bp</span><span>' + qualBar + '</span></div>';
          } else {
            td.innerHTML = colorQuality(qstr.substring(0, 80)) + (qstr.length > 80 ? '<span style="color:var(--vw-text-muted)">...</span>' : '');
          }
        } else if (colType === "num") {
          td.className = "num-cell";
          // Feature 11: numeric formatting toggle
          if (formatNumbers && typeof val === "number") {
            td.textContent = val.toLocaleString();
          } else {
            td.textContent = typeof val === "number" ? val.toLocaleString() : val;
          }
        } else if (colName === "strand" || colName === "Strand") {
          // Strand direction indicator for BED/GFF
          var cellVal = String(val);
          if (cellVal === "+") {
            td.innerHTML = '<span style="color:#4ade80" title="Forward strand">&#8594; +</span>';
          } else if (cellVal === "-") {
            td.innerHTML = '<span style="color:#f87171" title="Reverse strand">&#8592; -</span>';
          } else {
            td.textContent = cellVal;
          }
        } else {
          td.textContent = String(val).substring(0, 200);
          if (String(val).length > 200) td.title = String(val);
        }
        // Feature 10: Text wrap toggle
        if (textWrap) {
          td.style.whiteSpace = "normal";
          td.style.wordBreak = "break-all";
        }
        // Feature 4: Pin columns
        if (pinnedCols.has(ci) || (pinColumnsMode && ci < 2 && f.parsed.columns.length > 6)) {
          td.classList.add("pinned");
        }
        // Hidden columns
        if (hiddenCols[activeTab] && hiddenCols[activeTab].has(ci)) td.style.display = "none";
        // Heatmap coloring
        if (heatmapCols[ci] && heatmapCols[ci].enabled && typeof val === "number" && !isNaN(val)) {
          var hm = heatmapCols[ci];
          var range = hm.max - hm.min || 1;
          var ratio = Math.max(0, Math.min(1, (val - hm.min) / range));
          // Blue (cold) → Yellow (mid) → Red (hot)
          var r = Math.round(ratio < 0.5 ? ratio * 2 * 255 : 255);
          var g = Math.round(ratio < 0.5 ? ratio * 2 * 200 : (1 - ratio) * 2 * 200);
          var b = Math.round(ratio < 0.5 ? (1 - ratio * 2) * 255 : 0);
          td.style.background = "rgba(" + r + "," + g + "," + b + ",0.25)";
        }
        // Right-click context menu for copy (Feature 4)
        (function(rowData, colIndex) {
          td.addEventListener("contextmenu", function(e) {
            showContextMenu(e, f, rowData, colIndex);
          });
        })(item.row, ci);
        row.appendChild(td);
      }
      // Multi-select for selection summary (Feature 11) — Ctrl+click
      (function(idx) {
        row.addEventListener("click", function(e) {
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            if (selectedRows.has(idx)) { selectedRows.delete(idx); row.style.outline = ""; }
            else { selectedRows.add(idx); row.style.outline = "1px solid var(--vw-cyan)"; }
            updateSelectionSummary(f);
          }
        });
      })(item.idx);
      tbody.appendChild(row);
    }
    table.appendChild(tbody);
    wrap.appendChild(table);

    // Pagination controls
    if (totalPages > 1) {
      var pagBar = document.createElement("div");
      pagBar.className = "vw-pagination";
      pagBar.style.cssText = "display:flex;align-items:center;justify-content:center;gap:8px;padding:8px 12px;font-family:var(--vw-sans);font-size:12px;color:var(--vw-text-dim);border-top:1px solid var(--vw-border);";

      var prevBtn = document.createElement("button");
      prevBtn.textContent = "◀ Prev";
      prevBtn.disabled = currentPage === 0;
      prevBtn.style.cssText = "background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:4px;padding:3px 10px;color:var(--vw-text);cursor:pointer;font-size:11px;";
      if (prevBtn.disabled) prevBtn.style.opacity = "0.4";
      prevBtn.addEventListener("click", function() { currentPage--; renderView(); });
      pagBar.appendChild(prevBtn);

      var pageInfo = document.createElement("span");
      pageInfo.textContent = "Page " + (currentPage + 1) + " of " + totalPages + " (" + totalFiltered.toLocaleString() + " rows)";
      pagBar.appendChild(pageInfo);

      var nextBtn = document.createElement("button");
      nextBtn.textContent = "Next ▶";
      nextBtn.disabled = currentPage >= totalPages - 1;
      nextBtn.style.cssText = "background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:4px;padding:3px 10px;color:var(--vw-text);cursor:pointer;font-size:11px;";
      if (nextBtn.disabled) nextBtn.style.opacity = "0.4";
      nextBtn.addEventListener("click", function() { currentPage++; renderView(); });
      pagBar.appendChild(nextBtn);

      // Page size selector
      var sizeLabel = document.createElement("span");
      sizeLabel.textContent = "Show:";
      sizeLabel.style.marginLeft = "12px";
      pagBar.appendChild(sizeLabel);
      var sizeSelect = document.createElement("select");
      sizeSelect.style.cssText = "background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:4px;padding:2px 4px;color:var(--vw-text);font-size:11px;";
      [50, 100, 250, 500, 1000].forEach(function(n) {
        var opt = document.createElement("option");
        opt.value = n;
        opt.textContent = n;
        if (n === pageSize) opt.selected = true;
        sizeSelect.appendChild(opt);
      });
      sizeSelect.addEventListener("change", function() {
        pageSize = parseInt(this.value);
        currentPage = 0;
        renderView();
      });
      pagBar.appendChild(sizeSelect);

      // Minimap toggle (only show toggle when there are enough pages)
      if (totalPages >= 3 && totalFiltered >= 300) {
        var mmToggle = document.createElement("button");
        mmToggle.textContent = showMinimap ? "Hide Map" : "Map";
        mmToggle.title = "Toggle row density minimap";
        mmToggle.style.cssText = "background:" + (showMinimap ? "var(--vw-accent)" : "var(--vw-tab-bg)") + ";border:1px solid var(--vw-border);border-radius:4px;padding:3px 8px;color:var(--vw-text);cursor:pointer;font-size:11px;margin-left:8px;";
        mmToggle.addEventListener("click", function() {
          showMinimap = !showMinimap;
          renderView();
        });
        pagBar.appendChild(mmToggle);
      }

      // Jump to page input
      if (totalPages > 10) {
        var jumpLabel = document.createElement("span");
        jumpLabel.textContent = "Go:";
        jumpLabel.style.marginLeft = "8px";
        pagBar.appendChild(jumpLabel);
        var jumpInput = document.createElement("input");
        jumpInput.type = "number";
        jumpInput.min = 1;
        jumpInput.max = totalPages;
        jumpInput.value = currentPage + 1;
        jumpInput.style.cssText = "width:60px;background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:4px;padding:2px 4px;color:var(--vw-text);font-size:11px;text-align:center;";
        jumpInput.addEventListener("keydown", function(e) {
          if (e.key === "Enter") {
            var pg = parseInt(this.value) - 1;
            if (pg >= 0 && pg < totalPages) { currentPage = pg; renderView(); }
          }
        });
        pagBar.appendChild(jumpInput);
      }

      wrap.appendChild(pagBar);
    }

    contentEl.appendChild(wrap);

    // Mini-map: row density strip (toggled via toolbar, off by default)
    if (showMinimap && totalPages >= 3 && totalFiltered >= 300) {
      var minimap = document.createElement("canvas");
      minimap.className = "vw-minimap";
      minimap.width = 16;
      minimap.height = 200;
      minimap.style.cssText = "position:absolute;right:20px;top:0;width:14px;height:100%;cursor:pointer;opacity:0.5;border-radius:3px;border:1px solid var(--vw-border);";
      minimap.title = "Row density map — click to jump to page";
      var mmCtx = minimap.getContext("2d");
      // Draw density: color rows by first numeric column value
      var numCol = f.parsed.colTypes.findIndex(function(t) { return t === "num"; });
      var mmH = 200;
      var rowsPerPixel = Math.max(1, Math.ceil(totalFiltered / mmH));
      for (var py = 0; py < mmH; py++) {
        var rStart = py * rowsPerPixel;
        var rEnd = Math.min(rStart + rowsPerPixel, totalFiltered);
        if (numCol >= 0 && rEnd > rStart) {
          var avg = 0, cnt = 0;
          for (var ri = rStart; ri < rEnd; ri++) {
            var v = rows[ri] ? rows[ri].row[numCol] : 0;
            if (typeof v === "number" && !isNaN(v)) { avg += v; cnt++; }
          }
          avg = cnt ? avg / cnt : 0;
          var ratio = 0.5;
          if (heatmapCols[numCol]) {
            var hm = heatmapCols[numCol];
            ratio = Math.max(0, Math.min(1, (avg - hm.min) / ((hm.max - hm.min) || 1)));
          }
          var r = Math.round(ratio * 200 + 55);
          var g = Math.round((1 - ratio) * 150 + 55);
          mmCtx.fillStyle = "rgb(" + r + "," + g + ",180)";
        } else {
          mmCtx.fillStyle = "rgba(100,100,200,0.3)";
        }
        mmCtx.fillRect(0, py, 16, 1);
      }
      // Current page indicator
      var pageStartFrac = pageStart / totalFiltered;
      var pageEndFrac = pageEnd / totalFiltered;
      mmCtx.strokeStyle = "rgba(255,255,255,0.8)";
      mmCtx.lineWidth = 2;
      mmCtx.strokeRect(0, pageStartFrac * mmH, 16, Math.max(3, (pageEndFrac - pageStartFrac) * mmH));

      minimap.addEventListener("click", function(e) {
        var rect = minimap.getBoundingClientRect();
        var frac = (e.clientY - rect.top) / rect.height;
        currentPage = Math.floor(frac * totalPages);
        renderView();
      });
      wrap.style.position = "relative";
      wrap.appendChild(minimap);
    }

    // Keyboard navigation for table (cell-level with arrow keys)
    wrap.tabIndex = 0;
    wrap.style.outline = "none";
    var selectedRowIdx = -1;
    var selectedColIdx = -1;
    var prevCell = null;

    function highlightCell(ri, ci) {
      if (prevCell) { prevCell.style.outline = ""; prevCell.style.background = ""; }
      var trs = tbody.querySelectorAll("tr:not(.vw-group-row)");
      var maxR = trs.length - 1;
      var maxC = f.parsed.columns.length + (bookmarkMode ? 1 : 0); // +1 for row num, already included
      selectedRowIdx = Math.max(0, Math.min(ri, maxR));
      selectedColIdx = Math.max(0, Math.min(ci, maxC));
      if (trs[selectedRowIdx]) {
        var cells = trs[selectedRowIdx].querySelectorAll("td");
        if (cells[selectedColIdx]) {
          cells[selectedColIdx].style.outline = "2px solid var(--vw-accent)";
          cells[selectedColIdx].style.background = "rgba(139,92,246,0.1)";
          cells[selectedColIdx].scrollIntoView({ block: "nearest", inline: "nearest" });
          prevCell = cells[selectedColIdx];
        }
        trs[selectedRowIdx].scrollIntoView({ block: "nearest" });
      }
    }

    function selectRow(idx) {
      highlightCell(idx, Math.max(0, selectedColIdx));
    }

    wrap.addEventListener("keydown", function(e) {
      if (e.key === "ArrowDown" || e.key === "j") { e.preventDefault(); highlightCell(selectedRowIdx + 1, selectedColIdx); }
      else if (e.key === "ArrowUp" || e.key === "k") { e.preventDefault(); highlightCell(selectedRowIdx - 1, selectedColIdx); }
      else if (e.key === "ArrowRight" || e.key === "l") { e.preventDefault(); highlightCell(selectedRowIdx, selectedColIdx + 1); }
      else if (e.key === "ArrowLeft" || e.key === "h") { e.preventDefault(); highlightCell(selectedRowIdx, selectedColIdx - 1); }
      else if (e.key === "Enter") {
        e.preventDefault();
        // Copy cell value to clipboard
        if (prevCell) { navigator.clipboard.writeText(prevCell.textContent); showToast("Copied: " + prevCell.textContent.substring(0, 40)); }
      }
      else if (e.key === "Escape") { closeDetailPanel(); if (prevCell) { prevCell.style.outline = ""; prevCell.style.background = ""; prevCell = null; } }
      else if (e.key === "g" && !e.ctrlKey) { e.preventDefault(); highlightCell(0, selectedColIdx); }
      else if (e.key === "G") { e.preventDefault(); var trs = tbody.querySelectorAll("tr:not(.vw-group-row)"); highlightCell(trs.length - 1, selectedColIdx); }
      else if (e.key === "g" && e.ctrlKey) { e.preventDefault(); showGotoRow(pageRows, selectRow); }
      else if (e.key === "/") { e.preventDefault(); searchInput.focus(); }
      // Ctrl+Z for undo
      else if (e.key === "z" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); popUndo(); }
    });
    wrap.focus();
  }

  // ── SVG Chart Primitives ──────────────────────────────────────
  var SVG_NS = "http://www.w3.org/2000/svg";

  function svgEl(tag, attrs) {
    var el = document.createElementNS(SVG_NS, tag);
    if (attrs) Object.keys(attrs).forEach(function(k) { el.setAttribute(k, attrs[k]); });
    return el;
  }

  // Histogram: values → SVG bar chart
  function svgHistogram(values, opts) {
    opts = opts || {};
    var w = opts.width || 460, h = opts.height || 160;
    var pad = { top: 16, right: 12, bottom: 32, left: 48 };
    var bins = opts.bins || 25;
    var barColor = opts.color || "#7c3aed";
    var label = opts.label || "";

    if (!values.length) return document.createTextNode("(no data)");
    values = values.filter(function(v) { return typeof v === "number" && !isNaN(v); });
    if (!values.length) return document.createTextNode("(no numeric data)");

    var min = Math.min.apply(null, values), max = Math.max.apply(null, values);
    if (min === max) { max = min + 1; }
    var binW = (max - min) / bins;
    var counts = new Array(bins).fill(0);
    values.forEach(function(v) { counts[Math.min(Math.floor((v - min) / binW), bins - 1)]++; });
    var maxC = Math.max.apply(null, counts);

    var cw = w - pad.left - pad.right, ch = h - pad.top - pad.bottom;
    var svg = svgEl("svg", { width: w, height: h, viewBox: "0 0 " + w + " " + h });

    // Bars
    var bw = cw / bins;
    for (var i = 0; i < bins; i++) {
      var bh = maxC ? (counts[i] / maxC) * ch : 0;
      var bar = svgEl("rect", {
        x: pad.left + i * bw + 0.5, y: pad.top + ch - bh,
        width: Math.max(bw - 1, 1), height: bh,
        fill: barColor, rx: 1
      });
      bar.innerHTML = "<title>" + (min + i * binW).toFixed(1) + "–" + (min + (i + 1) * binW).toFixed(1) + ": " + counts[i] + "</title>";
      svg.appendChild(bar);
    }

    // Y-axis ticks
    for (var t = 0; t <= 4; t++) {
      var yv = Math.round(maxC * t / 4);
      var yy = pad.top + ch - (t / 4) * ch;
      var tick = svgEl("text", { x: pad.left - 6, y: yy + 3, fill: "#64748b", "font-size": 9, "text-anchor": "end", "font-family": "var(--vw-mono)" });
      tick.textContent = yv >= 1000 ? (yv / 1000).toFixed(0) + "k" : yv;
      svg.appendChild(tick);
      if (t > 0) {
        svg.appendChild(svgEl("line", { x1: pad.left, x2: w - pad.right, y1: yy, y2: yy, stroke: "#1e293b", "stroke-dasharray": "2,3" }));
      }
    }

    // X-axis labels
    var xSteps = [0, Math.floor(bins / 2), bins - 1];
    xSteps.forEach(function(xi) {
      var xl = svgEl("text", { x: pad.left + xi * bw + bw / 2, y: h - 6, fill: "#64748b", "font-size": 9, "text-anchor": "middle", "font-family": "var(--vw-mono)" });
      xl.textContent = (min + xi * binW).toFixed(1);
      svg.appendChild(xl);
    });

    // Label
    if (label) {
      var lb = svgEl("text", { x: w / 2, y: h - 18, fill: "#94a3b8", "font-size": 10, "text-anchor": "middle", "font-family": "var(--vw-sans)" });
      lb.textContent = label;
      svg.appendChild(lb);
    }

    // Axis lines
    svg.appendChild(svgEl("line", { x1: pad.left, x2: pad.left, y1: pad.top, y2: pad.top + ch, stroke: "#334155" }));
    svg.appendChild(svgEl("line", { x1: pad.left, x2: w - pad.right, y1: pad.top + ch, y2: pad.top + ch, stroke: "#334155" }));

    return svg;
  }

  // Bar chart: [{label, value, color?}] → SVG horizontal bar chart
  function svgBarChart(items, opts) {
    opts = opts || {};
    var w = opts.width || 460, barH = 22, gap = 4;
    var pad = { top: 8, right: 12, bottom: 8, left: opts.labelWidth || 80 };
    var h = pad.top + items.length * (barH + gap) + pad.bottom;
    var maxV = Math.max.apply(null, items.map(function(d) { return d.value; })) || 1;
    var cw = w - pad.left - pad.right;

    var svg = svgEl("svg", { width: w, height: h, viewBox: "0 0 " + w + " " + h });

    items.forEach(function(d, i) {
      var y = pad.top + i * (barH + gap);
      var bw = (d.value / maxV) * cw;
      var color = d.color || "#7c3aed";

      // Label
      var lbl = svgEl("text", { x: pad.left - 6, y: y + barH / 2 + 4, fill: "#94a3b8", "font-size": 10, "text-anchor": "end", "font-family": "var(--vw-mono)" });
      lbl.textContent = d.label.length > 10 ? d.label.substring(0, 9) + "\u2026" : d.label;
      svg.appendChild(lbl);

      // Bar
      svg.appendChild(svgEl("rect", { x: pad.left, y: y, width: Math.max(bw, 2), height: barH, fill: color, rx: 3, opacity: 0.85 }));

      // Value
      var vt = svgEl("text", { x: pad.left + bw + 6, y: y + barH / 2 + 4, fill: "#e2e8f0", "font-size": 10, "font-family": "var(--vw-mono)" });
      vt.textContent = d.value.toLocaleString();
      svg.appendChild(vt);
    });

    return svg;
  }

  // Per-base quality heatmap (FASTQ): compute mean quality at each position
  function svgPerBaseQuality(rows, opts) {
    opts = opts || {};
    var w = opts.width || 460, h = opts.height || 130;
    var pad = { top: 16, right: 12, bottom: 28, left: 40 };
    var maxPos = 0;
    var sampleN = Math.min(rows.length, 5000);

    // Accumulate quality sums per position
    for (var ri = 0; ri < sampleN; ri++) {
      var q = String(rows[ri][4] || rows[ri][rows[ri].length - 1]); // quality column
      if (q.length > maxPos) maxPos = q.length;
    }
    if (maxPos === 0) return document.createTextNode("(no quality data)");

    var posCount = Math.min(maxPos, 150); // cap at 150 positions
    var sums = new Float64Array(posCount);
    var counts = new Uint32Array(posCount);

    for (var ri = 0; ri < sampleN; ri++) {
      var q = String(rows[ri][4] || rows[ri][rows[ri].length - 1]);
      var len = Math.min(q.length, posCount);
      for (var p = 0; p < len; p++) {
        sums[p] += q.charCodeAt(p) - 33;
        counts[p]++;
      }
    }

    var means = [];
    for (var p = 0; p < posCount; p++) {
      means.push(counts[p] ? sums[p] / counts[p] : 0);
    }

    var cw = w - pad.left - pad.right, ch = h - pad.top - pad.bottom;
    var svg = svgEl("svg", { width: w, height: h, viewBox: "0 0 " + w + " " + h });

    // Quality zone backgrounds
    var maxQ = 42;
    var zones = [
      { y1: 0, y2: 10, color: "rgba(248,113,113,0.08)" },   // bad
      { y1: 10, y2: 20, color: "rgba(251,191,36,0.08)" },    // poor
      { y1: 20, y2: 30, color: "rgba(34,211,238,0.06)" },    // ok
      { y1: 30, y2: maxQ, color: "rgba(52,211,153,0.06)" }   // good
    ];
    zones.forEach(function(z) {
      var zy1 = pad.top + ch - (Math.min(z.y2, maxQ) / maxQ) * ch;
      var zy2 = pad.top + ch - (z.y1 / maxQ) * ch;
      svg.appendChild(svgEl("rect", { x: pad.left, y: zy1, width: cw, height: zy2 - zy1, fill: z.color }));
    });

    // Q threshold lines
    [10, 20, 30].forEach(function(qv) {
      var ly = pad.top + ch - (qv / maxQ) * ch;
      svg.appendChild(svgEl("line", { x1: pad.left, x2: w - pad.right, y1: ly, y2: ly, stroke: "#334155", "stroke-dasharray": "3,4" }));
      var lt = svgEl("text", { x: pad.left - 4, y: ly + 3, fill: "#64748b", "font-size": 8, "text-anchor": "end", "font-family": "var(--vw-mono)" });
      lt.textContent = "Q" + qv;
      svg.appendChild(lt);
    });

    // Bars per position
    var barW = Math.max(cw / posCount - 0.5, 1);
    for (var p = 0; p < posCount; p++) {
      var qval = means[p];
      var bh = (qval / maxQ) * ch;
      var color = qval >= 30 ? "#34d399" : qval >= 20 ? "#fbbf24" : qval >= 10 ? "#fb923c" : "#f87171";
      var bar = svgEl("rect", {
        x: pad.left + (p / posCount) * cw, y: pad.top + ch - bh,
        width: barW, height: bh, fill: color, rx: 0.5
      });
      bar.innerHTML = "<title>Pos " + (p + 1) + ": Q" + qval.toFixed(1) + "</title>";
      svg.appendChild(bar);
    }

    // X-axis labels
    var xTicks = [1, Math.floor(posCount / 4), Math.floor(posCount / 2), Math.floor(posCount * 3 / 4), posCount];
    xTicks.forEach(function(pos) {
      var xt = svgEl("text", { x: pad.left + ((pos - 1) / posCount) * cw, y: h - 6, fill: "#64748b", "font-size": 9, "text-anchor": "middle", "font-family": "var(--vw-mono)" });
      xt.textContent = pos;
      svg.appendChild(xt);
    });

    // Axis
    svg.appendChild(svgEl("line", { x1: pad.left, x2: pad.left, y1: pad.top, y2: pad.top + ch, stroke: "#334155" }));
    svg.appendChild(svgEl("line", { x1: pad.left, x2: w - pad.right, y1: pad.top + ch, y2: pad.top + ch, stroke: "#334155" }));

    // Label
    var lb = svgEl("text", { x: w / 2, y: h - 16, fill: "#94a3b8", "font-size": 10, "text-anchor": "middle", "font-family": "var(--vw-sans)" });
    lb.textContent = "Position in read";
    svg.appendChild(lb);

    return svg;
  }

  // GC content per sequence (FASTA)
  function svgGcContent(rows) {
    var gcValues = [];
    var sampleN = Math.min(rows.length, 5000);
    for (var ri = 0; ri < sampleN; ri++) {
      var seq = String(rows[ri][3] || rows[ri][rows[ri].length - 1]); // sequence column
      if (!seq) continue;
      var gc = 0, total = 0;
      for (var j = 0; j < seq.length; j++) {
        var c = seq.charAt(j).toUpperCase();
        if (c === "G" || c === "C") { gc++; total++; }
        else if (c === "A" || c === "T" || c === "U") { total++; }
      }
      if (total > 0) gcValues.push((gc / total) * 100);
    }
    return svgHistogram(gcValues, { color: "#22d3ee", label: "GC %", bins: 30 });
  }

  // Chromosome distribution bar chart (VCF/BED/GFF/SAM)
  function svgChromDistribution(rows, chromCol) {
    var chromCounts = {};
    rows.forEach(function(r) {
      var chr = String(r[chromCol] || "");
      if (chr && chr !== "*") chromCounts[chr] = (chromCounts[chr] || 0) + 1;
    });
    // Sort: chr1-22, chrX, chrY, then others
    var entries = Object.entries(chromCounts).sort(function(a, b) {
      var na = a[0].replace(/^chr/i, ""), nb = b[0].replace(/^chr/i, "");
      var ia = parseInt(na), ib = parseInt(nb);
      if (!isNaN(ia) && !isNaN(ib)) return ia - ib;
      if (!isNaN(ia)) return -1;
      if (!isNaN(ib)) return 1;
      return na.localeCompare(nb);
    }).slice(0, 24); // top 24 chromosomes

    var colors = ["#7c3aed", "#a78bfa", "#6d28d9", "#8b5cf6", "#c4b5fd"];
    return svgBarChart(entries.map(function(e, i) {
      return { label: e[0], value: e[1], color: colors[i % colors.length] };
    }), { labelWidth: 60 });
  }

  // Variant type donut (VCF)
  function svgVariantDonut(stats) {
    var items = [
      { label: "SNPs", value: stats["SNPs"] || 0, color: "#34d399" },
      { label: "Indels", value: stats["Indels"] || 0, color: "#60a5fa" },
      { label: "Other", value: stats["Other"] || 0, color: "#fbbf24" }
    ].filter(function(d) { return d.value > 0; });
    if (!items.length) return document.createTextNode("(no variants)");

    var total = items.reduce(function(a, d) { return a + d.value; }, 0);
    var w = 200, h = 200, cx = 100, cy = 100, r = 70, ir = 42;
    var svg = svgEl("svg", { width: w, height: h, viewBox: "0 0 " + w + " " + h });

    var startAngle = -Math.PI / 2;
    items.forEach(function(d) {
      var angle = (d.value / total) * Math.PI * 2;
      var endAngle = startAngle + angle;
      var large = angle > Math.PI ? 1 : 0;
      var x1 = cx + r * Math.cos(startAngle), y1 = cy + r * Math.sin(startAngle);
      var x2 = cx + r * Math.cos(endAngle), y2 = cy + r * Math.sin(endAngle);
      var ix1 = cx + ir * Math.cos(startAngle), iy1 = cy + ir * Math.sin(startAngle);
      var ix2 = cx + ir * Math.cos(endAngle), iy2 = cy + ir * Math.sin(endAngle);
      var path = "M" + ix1 + "," + iy1 + " L" + x1 + "," + y1 +
        " A" + r + "," + r + " 0 " + large + ",1 " + x2 + "," + y2 +
        " L" + ix2 + "," + iy2 +
        " A" + ir + "," + ir + " 0 " + large + ",0 " + ix1 + "," + iy1 + "Z";
      var p = svgEl("path", { d: path, fill: d.color, opacity: 0.85 });
      p.innerHTML = "<title>" + d.label + ": " + d.value.toLocaleString() + " (" + (d.value / total * 100).toFixed(1) + "%)</title>";
      svg.appendChild(p);

      // Label on arc midpoint
      var mid = startAngle + angle / 2;
      var lx = cx + (r + 16) * Math.cos(mid), ly = cy + (r + 16) * Math.sin(mid);
      if (d.value / total > 0.05) {
        var lt = svgEl("text", { x: lx, y: ly + 4, fill: "#e2e8f0", "font-size": 10, "text-anchor": "middle", "font-family": "var(--vw-mono)" });
        lt.textContent = d.label;
        svg.appendChild(lt);
      }

      startAngle = endAngle;
    });

    // Center text
    var ct = svgEl("text", { x: cx, y: cy - 4, fill: "#e2e8f0", "font-size": 18, "font-weight": "bold", "text-anchor": "middle", "font-family": "var(--vw-mono)" });
    ct.textContent = total.toLocaleString();
    svg.appendChild(ct);
    var ct2 = svgEl("text", { x: cx, y: cy + 14, fill: "#64748b", "font-size": 10, "text-anchor": "middle", "font-family": "var(--vw-sans)" });
    ct2.textContent = "variants";
    svg.appendChild(ct2);

    return svg;
  }

  // Feature type bar chart (GFF)
  function svgFeatureTypes(rows) {
    var counts = {};
    rows.forEach(function(r) { var t = String(r[2] || ""); if (t) counts[t] = (counts[t] || 0) + 1; });
    var entries = Object.entries(counts).sort(function(a, b) { return b[1] - a[1]; }).slice(0, 12);
    var colors = ["#a78bfa", "#7c3aed", "#6d28d9", "#818cf8", "#c084fc", "#e879f9"];
    return svgBarChart(entries.map(function(e, i) {
      return { label: e[0], value: e[1], color: colors[i % colors.length] };
    }), { labelWidth: 70 });
  }

  // Numeric column histogram for CSV/TSV
  function svgNumericColumns(f) {
    var charts = [];
    var colors = ["#7c3aed", "#22d3ee", "#34d399", "#fbbf24", "#f87171", "#a78bfa"];
    f.parsed.columns.forEach(function(col, ci) {
      if (f.parsed.colTypes[ci] !== "num") return;
      var vals = [];
      var n = Math.min(f.parsed.rows.length, 10000);
      for (var ri = 0; ri < n; ri++) {
        var v = f.parsed.rows[ri][ci];
        if (typeof v === "number" && !isNaN(v)) vals.push(v);
      }
      if (vals.length < 3) return;
      charts.push({ col: col, svg: svgHistogram(vals, { color: colors[charts.length % colors.length], label: col }) });
    });
    return charts;
  }

  // ── Stats View ──────────────────────────────────────────────────
  function getStatsText(stats, format) {
    var keys = Object.keys(stats);
    if (format === "json") {
      return JSON.stringify(stats, null, 2);
    }
    // TSV
    return keys.join("\t") + "\n" + keys.map(function(k) { return String(stats[k]); }).join("\t");
  }

  function downloadStats(f, format) {
    var stats = f.parsed.stats || {};
    var text = getStatsText(stats, format);
    var ext = format === "json" ? ".json" : ".tsv";
    var mime = format === "json" ? "application/json" : "text/tab-separated-values";
    var base = f.name.replace(/\.[^.]+$/, "");
    var blob = new Blob([text], { type: mime });
    var a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = base + "_stats" + ext;
    a.click();
    URL.revokeObjectURL(a.href);
  }

  function copyStats(f, btn) {
    var stats = f.parsed.stats || {};
    var text = getStatsText(stats, "json");
    navigator.clipboard.writeText(text).then(function() {
      var orig = btn.textContent;
      btn.textContent = "Copied!";
      btn.classList.add("copied");
      setTimeout(function() { btn.textContent = orig; btn.classList.remove("copied"); }, 2000);
    });
  }

  function exportChartsPng(f) {
    var svgs = contentEl.querySelectorAll(".vw-stat-card svg");
    if (!svgs.length) return;
    var gap = 20;
    var maxW = 0;
    var totalH = gap;
    var items = [];
    svgs.forEach(function(svg) {
      var w = parseInt(svg.getAttribute("width")) || svg.getBoundingClientRect().width || 500;
      var h = parseInt(svg.getAttribute("height")) || svg.getBoundingClientRect().height || 200;
      if (w > maxW) maxW = w;
      items.push({ svg: svg, w: w, h: h });
      totalH += h + gap;
    });
    var canvas = document.createElement("canvas");
    var scale = 2;
    canvas.width = (maxW + gap * 2) * scale;
    canvas.height = totalH * scale;
    var ctx = canvas.getContext("2d");
    ctx.scale(scale, scale);
    ctx.fillStyle = "#0d1117";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    var y = gap;
    var pending = items.length;
    items.forEach(function(item, idx) {
      var svgClone = item.svg.cloneNode(true);
      svgClone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      var blob = new Blob([new XMLSerializer().serializeToString(svgClone)], { type: "image/svg+xml" });
      var url = URL.createObjectURL(blob);
      var img = new Image();
      img.onload = function() {
        var yOff = gap;
        for (var j = 0; j < idx; j++) yOff += items[j].h + gap;
        ctx.drawImage(img, gap, yOff, item.w, item.h);
        URL.revokeObjectURL(url);
        pending--;
        if (pending === 0) {
          var a = document.createElement("a");
          a.href = canvas.toDataURL("image/png");
          a.download = f.name.replace(/\.[^.]+$/, "") + "_charts.png";
          a.click();
        }
      };
      img.src = url;
    });
  }

  function renderStatsView(f) {
    // Stats export bar
    var bar = document.createElement("div");
    bar.className = "vw-stats-bar";
    bar.innerHTML = '<span class="vw-stats-bar-label">Statistics — ' + escapeHtml(f.name) + '</span>';
    var btnJson = document.createElement("button");
    btnJson.className = "vw-stats-btn"; btnJson.textContent = "Save JSON";
    btnJson.addEventListener("click", function() { downloadStats(f, "json"); });
    var btnTsv = document.createElement("button");
    btnTsv.className = "vw-stats-btn"; btnTsv.textContent = "Save TSV";
    btnTsv.addEventListener("click", function() { downloadStats(f, "tsv"); });
    var btnCopy = document.createElement("button");
    btnCopy.className = "vw-stats-btn"; btnCopy.textContent = "Copy";
    btnCopy.addEventListener("click", function() { copyStats(f, btnCopy); });
    bar.appendChild(btnJson);
    bar.appendChild(btnTsv);
    bar.appendChild(btnCopy);
    contentEl.appendChild(bar);

    var container = document.createElement("div");
    container.className = "vw-stats";

    var stats = f.parsed.stats || {};

    // Feature 5: FASTQ QC verdict badge
    if (f.parsed.format === "fastq") {
      var qc = fastqQcVerdict(stats);
      var qcCard = document.createElement("div");
      qcCard.className = "vw-stat-card";
      qcCard.style.gridColumn = "span 2";
      qcCard.innerHTML = '<div class="vw-stat-label">QC Verdict</div>' +
        '<div style="display:flex;align-items:center;gap:12px;margin-top:4px">' +
        '<span class="vw-qc-badge ' + qc.cls + '">' + qc.verdict + '</span>' +
        '<span style="font-family:var(--vw-mono);font-size:12px;color:var(--vw-text-dim)">' + escapeHtml(qc.detail) + '</span>' +
        '</div>';
      container.appendChild(qcCard);
    }

    // Feature 1: Generate sparkline data for numeric stats
    var sparkData = {};
    if (f.parsed.format === "fasta") {
      sparkData["Avg length"] = f.parsed.rows.slice(0, 200).map(function(r) { return r[2]; });
    } else if (f.parsed.format === "fastq") {
      sparkData["Mean quality"] = f.parsed.rows.slice(0, 200).map(function(r) { return parseFloat(r[2]); });
      sparkData["Avg length"] = f.parsed.rows.slice(0, 200).map(function(r) { return r[1]; });
    } else if (f.parsed.format === "sam") {
      sparkData["Alignments"] = null; // no sparkline for simple count
    }

    Object.keys(stats).forEach(function(key) {
      var card = document.createElement("div");
      card.className = "vw-stat-card";
      var valueDiv = document.createElement("div");
      valueDiv.className = "vw-stat-value";
      valueDiv.textContent = String(stats[key]);
      card.innerHTML = '<div class="vw-stat-label">' + escapeHtml(key) + '</div>';
      card.appendChild(valueDiv);
      // Feature 1: Add sparkline if data available
      if (sparkData[key] && sparkData[key].length > 1) {
        var spark = svgSparkline(sparkData[key], 50, 16, "#a78bfa");
        if (spark) valueDiv.appendChild(spark);
      }
      container.appendChild(card);
    });

    // ── Format-specific SVG charts ──

    // FASTA: Length distribution + GC content
    if (f.parsed.format === "fasta") {
      var lenVals = f.parsed.rows.map(function(r) { return r[2]; });
      addChartCard(container, "Length Distribution", svgHistogram(lenVals, { color: "#34d399", label: "Sequence length (bp)" }), 3);
      addChartCard(container, "GC Content Distribution", svgGcContent(f.parsed.rows), 3);
    }

    // FASTQ: Per-base quality + Length distribution + Quality distribution + GC
    if (f.parsed.format === "fastq") {
      addChartCard(container, "Per-Base Quality", svgPerBaseQuality(f.parsed.rows), 3);
      var fqLens = f.parsed.rows.map(function(r) { return r[1]; });
      addChartCard(container, "Read Length Distribution", svgHistogram(fqLens, { color: "#60a5fa", label: "Read length (bp)" }), 3);
      var fqQuals = f.parsed.rows.map(function(r) { return parseFloat(r[2]); });
      addChartCard(container, "Mean Quality Distribution", svgHistogram(fqQuals, { color: "#34d399", label: "Mean Phred score" }), 3);
    }

    // VCF: Variant type donut + Chromosome distribution
    if (f.parsed.format === "vcf") {
      addChartCard(container, "Variant Types", svgVariantDonut(f.parsed.stats), 2);
      addChartCard(container, "Variants by Chromosome", svgChromDistribution(f.parsed.rows, 0), 3);
    }

    // BED: Region size distribution + Chromosome distribution
    if (f.parsed.format === "bed") {
      var bedSizes = f.parsed.rows.map(function(r) { return r[2] - r[1]; });
      addChartCard(container, "Region Size Distribution", svgHistogram(bedSizes, { color: "#fbbf24", label: "Region size (bp)" }), 3);
      addChartCard(container, "Regions by Chromosome", svgChromDistribution(f.parsed.rows, 0), 3);
    }

    // GFF: Feature types + Chromosome distribution
    if (f.parsed.format === "gff") {
      addChartCard(container, "Feature Types", svgFeatureTypes(f.parsed.rows), 3);
      addChartCard(container, "Features by Chromosome", svgChromDistribution(f.parsed.rows, 0), 3);
    }

    // SAM: MAPQ distribution + Chromosome distribution + Read length distribution
    if (f.parsed.format === "sam") {
      var mapqs = f.parsed.rows.map(function(r) { return r[4]; });
      addChartCard(container, "Mapping Quality Distribution", svgHistogram(mapqs, { color: "#a78bfa", label: "MAPQ", bins: 30 }), 3);
      addChartCard(container, "Alignments by Chromosome", svgChromDistribution(f.parsed.rows, 2), 3);
      var readLens = f.parsed.rows.map(function(r) { return String(r[9] || "").length; });
      addChartCard(container, "Read Length Distribution", svgHistogram(readLens, { color: "#22d3ee", label: "Read length (bp)" }), 3);
    }

    // CSV/TSV: Histograms for numeric columns (up to 4)
    if (f.parsed.format === "csv" || f.parsed.format === "tsv") {
      var numCharts = svgNumericColumns(f);
      numCharts.slice(0, 4).forEach(function(nc) {
        addChartCard(container, nc.col + " Distribution", nc.svg, 2);
      });
    }

    // Feature 8: Stats comparison across tabs
    renderStatsComparison(container, f.parsed.format);

    // Truncated warning + full-file stats button
    if (f.truncated) {
      var warn = document.createElement("div");
      warn.className = "vw-stat-card";
      warn.style.cssText = "border-color:var(--vw-amber);grid-column:span 4;";
      var loadedRows = f.parsed.rows.length.toLocaleString();
      var estTime = Math.max(1, Math.round(f.size / (1024 * 1024) / 100));

      var warnInner = document.createElement("div");
      warnInner.innerHTML = '<div class="vw-stat-label" style="color:var(--vw-amber)">Partial Load (first 50 MB of ' + formatBytes(f.size) + ')</div>' +
        '<div style="font-size:13px;color:var(--vw-text-dim);margin:6px 0">' + loadedRows + ' rows loaded — stats above reflect only the loaded portion.</div>';

      // Full-file stats button (only if we have the File reference)
      if (f.fileRef) {
        // Show existing full stats if already computed
        if (f._fullStats) {
          renderFullStatsResults(warnInner, f._fullStats, f.parsed.columns, f.parsed.colTypes);
        } else {
          var btnRow = document.createElement("div");
          btnRow.style.cssText = "display:flex;align-items:center;gap:12px;margin-top:8px;";
          var computeBtn = document.createElement("button");
          computeBtn.textContent = "Compute Full-File Stats";
          computeBtn.style.cssText = "background:var(--vw-accent);color:#fff;border:none;border-radius:6px;padding:8px 18px;font-size:13px;font-weight:600;cursor:pointer;font-family:var(--vw-sans);";
          computeBtn.title = "Stream the entire file to compute accurate statistics without loading it all into memory";
          var estLabel = document.createElement("span");
          estLabel.style.cssText = "font-size:12px;color:var(--vw-text-muted);";
          estLabel.textContent = "Streams " + formatBytes(f.size) + " in chunks — est. ~" + estTime + "s";
          btnRow.appendChild(computeBtn);
          btnRow.appendChild(estLabel);
          warnInner.appendChild(btnRow);

          // Progress bar (hidden initially)
          var progWrap = document.createElement("div");
          progWrap.style.cssText = "display:none;margin-top:10px;";
          progWrap.innerHTML = '<div style="display:flex;justify-content:space-between;font-size:11px;color:var(--vw-text-dim);margin-bottom:4px"><span id="vw-fullstats-phase">Streaming...</span><span id="vw-fullstats-pct">0%</span></div>' +
            '<div style="height:4px;background:var(--vw-border);border-radius:2px;overflow:hidden"><div id="vw-fullstats-bar" style="height:100%;width:0%;background:var(--vw-accent);border-radius:2px;transition:width 0.2s"></div></div>';
          warnInner.appendChild(progWrap);

          // Results container
          var resultsDiv = document.createElement("div");
          resultsDiv.id = "vw-fullstats-results";
          warnInner.appendChild(resultsDiv);

          computeBtn.addEventListener("click", function() {
            computeBtn.disabled = true;
            computeBtn.style.opacity = "0.5";
            computeBtn.textContent = "Computing...";
            progWrap.style.display = "";
            streamFullFileStats(f, function(phase, pct) {
              var phaseEl = document.getElementById("vw-fullstats-phase");
              var pctEl = document.getElementById("vw-fullstats-pct");
              var barEl = document.getElementById("vw-fullstats-bar");
              if (phaseEl) phaseEl.textContent = phase;
              if (pctEl) pctEl.textContent = pct + "%";
              if (barEl) barEl.style.width = pct + "%";
            }, function(stats) {
              f._fullStats = stats;
              // Update summary cache with full stats
              updateSummaryCacheFromFullStats(f, stats);
              computeBtn.style.display = "none";
              estLabel.style.display = "none";
              progWrap.style.display = "none";
              var resEl = document.getElementById("vw-fullstats-results");
              if (resEl) renderFullStatsResults(resEl, stats, f.parsed.columns, f.parsed.colTypes);
            });
          });
        }
      }
      warn.appendChild(warnInner);
      container.appendChild(warn);
    }

    contentEl.appendChild(container);
  }

  // ── Full-file streaming stats engine ─────────────────────────
  function streamFullFileStats(f, onProgress, onComplete) {
    var fileRef = f.fileRef;
    var fmt = f.parsed.format;
    var colCount = f.parsed.columns.length;
    var colTypes = f.parsed.colTypes;
    var CHUNK_SIZE = 8 * 1024 * 1024; // 8 MB chunks
    var fileSize = fileRef.size;
    var offset = 0;
    var leftover = ""; // partial line from previous chunk
    var totalRows = 0;
    var headerSkipped = false;

    // Per-column accumulators
    var colStats = [];
    for (var ci = 0; ci < colCount; ci++) {
      colStats.push({
        count: 0,
        numCount: 0,
        sum: 0,
        sumSq: 0, // for Welford's variance
        min: Infinity,
        max: -Infinity,
        // Reservoir sample for median approximation (keep 10000)
        reservoir: [],
        reservoirMax: 10000,
        // Unique tracking (capped)
        uniqSet: new Set(),
        uniqCapped: false
      });
    }

    // Determine line parser based on format
    function isHeaderLine(line) {
      if (fmt === "vcf" || fmt === "gff") return line.charAt(0) === "#";
      if (fmt === "sam") return line.charAt(0) === "@";
      if (fmt === "fasta") return line.charAt(0) === ">";
      if (fmt === "fastq") return false; // handled differently
      return false;
    }

    function getDelimiter() {
      if (fmt === "csv") return ",";
      if (fmt === "tsv" || fmt === "vcf" || fmt === "bed" || fmt === "gff" || fmt === "sam") return "\t";
      return detectDelimiter(leftover || "");
    }

    var delim = getDelimiter();
    var isFasta = (fmt === "fasta");
    var isFastq = (fmt === "fastq");
    var fastqLineInRecord = 0; // 0=header, 1=seq, 2=plus, 3=qual
    var fastaSeqLen = 0;

    function processLine(line) {
      if (!line) return;

      // FASTA: special handling — track sequence lengths
      if (isFasta) {
        if (line.charAt(0) === ">") {
          if (fastaSeqLen > 0) {
            // Finish previous sequence
            accumulateValue(1, fastaSeqLen); // col 1 = sequence (track length)
            totalRows++;
          }
          fastaSeqLen = 0;
          // col 0 = name
          var fastaName = line.substring(1).split(/\s/)[0];
          accumulateValue(0, fastaName);
        } else {
          fastaSeqLen += line.length;
        }
        return;
      }

      // FASTQ: 4-line records
      if (isFastq) {
        if (fastqLineInRecord === 0) {
          // header line
          if (line.charAt(0) === "@") {
            accumulateValue(0, line.substring(1).split(/\s/)[0]);
          }
        } else if (fastqLineInRecord === 1) {
          // sequence
          accumulateValue(1, line);
          accumulateValue(1, line.length); // also track length as numeric
        } else if (fastqLineInRecord === 3) {
          // quality
          accumulateValue(3, line);
          // Compute mean quality
          var qSum = 0;
          for (var qi = 0; qi < line.length; qi++) qSum += line.charCodeAt(qi) - 33;
          var meanQ = line.length > 0 ? qSum / line.length : 0;
          accumulateValue(3, meanQ);
          totalRows++;
        }
        fastqLineInRecord = (fastqLineInRecord + 1) % 4;
        return;
      }

      // Tabular formats: split by delimiter
      if (!headerSkipped && (fmt === "csv" || fmt === "tsv")) {
        headerSkipped = true;
        return; // skip header row
      }
      if (isHeaderLine(line)) return;

      var fields = line.split(delim);
      totalRows++;

      for (var ci = 0; ci < Math.min(fields.length, colCount); ci++) {
        var raw = fields[ci];
        if (colTypes[ci] === "num") {
          var num = parseFloat(raw);
          if (!isNaN(num)) accumulateValue(ci, num);
        } else {
          accumulateValue(ci, raw);
        }
      }
    }

    function accumulateValue(ci, val) {
      if (ci >= colStats.length) return;
      var cs = colStats[ci];
      cs.count++;

      if (typeof val === "number" && !isNaN(val)) {
        cs.numCount++;
        cs.sum += val;
        cs.sumSq += val * val;
        if (val < cs.min) cs.min = val;
        if (val > cs.max) cs.max = val;

        // Reservoir sampling for median
        if (cs.reservoir.length < cs.reservoirMax) {
          cs.reservoir.push(val);
        } else {
          var j = Math.floor(Math.random() * cs.numCount);
          if (j < cs.reservoirMax) cs.reservoir[j] = val;
        }
      }

      // Track uniques (cap at 10001 to detect "too many")
      if (!cs.uniqCapped) {
        var strVal = String(val);
        if (strVal.length <= 100) cs.uniqSet.add(strVal); // don't track very long strings
        if (cs.uniqSet.size > 10000) cs.uniqCapped = true;
      }
    }

    function readNextChunk() {
      if (offset >= fileSize) {
        // Finish last FASTA sequence
        if (isFasta && fastaSeqLen > 0) {
          accumulateValue(1, fastaSeqLen);
          totalRows++;
        }
        finalize();
        return;
      }

      var end = Math.min(offset + CHUNK_SIZE, fileSize);
      var blob = fileRef.slice(offset, end);
      var reader = new FileReader();
      reader.onload = function() {
        var text = leftover + reader.result;
        // Find last newline — keep the incomplete line for next chunk
        var lastNl = text.lastIndexOf("\n");
        if (lastNl === -1 && end < fileSize) {
          // No newline in chunk — keep all as leftover
          leftover = text;
        } else if (end < fileSize) {
          leftover = text.substring(lastNl + 1);
          text = text.substring(0, lastNl);
        } else {
          leftover = "";
          // Last chunk — process everything
        }

        // Process lines
        var lines = text.split("\n");
        for (var i = 0; i < lines.length; i++) {
          var ln = lines[i];
          if (ln.length > 0 && ln.charAt(ln.length - 1) === "\r") ln = ln.substring(0, ln.length - 1);
          if (ln.length > 0) processLine(ln);
        }

        offset = end;
        var pct = Math.round(offset / fileSize * 100);
        onProgress("Streaming... " + Math.round(offset / (1024 * 1024)) + " / " + Math.round(fileSize / (1024 * 1024)) + " MB — " + totalRows.toLocaleString() + " rows", pct);

        // Yield to UI thread then continue
        setTimeout(readNextChunk, 0);
      };
      reader.readAsText(blob);
    }

    function finalize() {
      // Compute final stats per column
      var results = {
        totalRows: totalRows,
        fileSize: fileSize,
        columns: []
      };
      for (var ci = 0; ci < colStats.length; ci++) {
        var cs = colStats[ci];
        var col = {
          name: f.parsed.columns[ci],
          type: colTypes[ci],
          count: cs.count,
          unique: cs.uniqCapped ? ">10,000" : cs.uniqSet.size
        };
        if (cs.numCount > 0) {
          col.numCount = cs.numCount;
          col.sum = cs.sum;
          col.mean = cs.sum / cs.numCount;
          col.min = cs.min;
          col.max = cs.max;
          // Variance via E[X^2] - (E[X])^2
          var meanSq = cs.sumSq / cs.numCount;
          var sqMean = col.mean * col.mean;
          col.variance = Math.max(0, meanSq - sqMean);
          col.stdev = Math.sqrt(col.variance);
          // Approximate median from reservoir
          if (cs.reservoir.length > 0) {
            cs.reservoir.sort(function(a, b) { return a - b; });
            var mid = Math.floor(cs.reservoir.length / 2);
            col.median = cs.reservoir.length % 2 === 0
              ? (cs.reservoir[mid - 1] + cs.reservoir[mid]) / 2
              : cs.reservoir[mid];
          }
        }
        results.columns.push(col);
      }
      onProgress("Complete — " + totalRows.toLocaleString() + " rows analyzed", 100);
      onComplete(results);
    }

    // Start
    onProgress("Starting stream...", 0);
    setTimeout(readNextChunk, 10);
  }

  function renderFullStatsResults(container, stats, columns, colTypes) {
    container.innerHTML = "";
    var header = document.createElement("div");
    header.style.cssText = "margin:12px 0 8px;font-size:13px;font-weight:600;color:var(--vw-accent);font-family:var(--vw-sans);";
    header.textContent = "Full-File Stats — " + stats.totalRows.toLocaleString() + " total rows (" + formatBytes(stats.fileSize) + ")";
    container.appendChild(header);

    var table = document.createElement("table");
    table.style.cssText = "width:100%;border-collapse:collapse;font-family:var(--vw-mono);font-size:11px;";
    var thead = document.createElement("thead");
    var htr = document.createElement("tr");
    ["Column", "Type", "Count", "Unique", "Min", "Max", "Mean", "Median", "Stdev"].forEach(function(h) {
      var th = document.createElement("th");
      th.textContent = h;
      th.style.cssText = "text-align:left;padding:4px 8px;border-bottom:2px solid var(--vw-border);color:var(--vw-text-dim);font-size:10px;";
      htr.appendChild(th);
    });
    thead.appendChild(htr);
    table.appendChild(thead);

    var tbody = document.createElement("tbody");
    stats.columns.forEach(function(col) {
      var tr = document.createElement("tr");
      tr.style.borderBottom = "1px solid var(--vw-border)";
      var vals = [
        col.name,
        col.type,
        col.count ? col.count.toLocaleString() : "-",
        col.unique !== undefined ? String(col.unique) : "-",
        col.min !== undefined && col.min !== Infinity ? formatStatNum(col.min) : "-",
        col.max !== undefined && col.max !== -Infinity ? formatStatNum(col.max) : "-",
        col.mean !== undefined ? formatStatNum(col.mean) : "-",
        col.median !== undefined ? formatStatNum(col.median) + " ~" : "-",
        col.stdev !== undefined ? formatStatNum(col.stdev) : "-"
      ];
      vals.forEach(function(v) {
        var td = document.createElement("td");
        td.textContent = v;
        td.style.cssText = "padding:3px 8px;color:var(--vw-text);";
        tr.appendChild(td);
      });
      tbody.appendChild(tr);
    });
    table.appendChild(tbody);
    container.appendChild(table);

    var note = document.createElement("div");
    note.style.cssText = "font-size:10px;color:var(--vw-text-muted);margin-top:6px;font-family:var(--vw-sans);";
    note.textContent = "Median values marked with ~ are approximated via reservoir sampling (10,000 samples). All other stats are exact.";
    container.appendChild(note);
  }

  function formatStatNum(n) {
    if (n === undefined || n === null) return "-";
    if (Number.isInteger(n) && Math.abs(n) < 1e9) return n.toLocaleString();
    if (Math.abs(n) < 0.001 && n !== 0) return n.toExponential(3);
    if (Math.abs(n) >= 1e6) return n.toExponential(3);
    return n.toFixed(Math.abs(n) < 10 ? 3 : 1);
  }

  function updateSummaryCacheFromFullStats(f, stats) {
    // Replace the partial summary cache with full-file stats
    f.parsed._summaryCache = stats.columns.map(function(col) {
      if (col.numCount > 0) {
        return {
          text: "\u03bc" + formatStatNum(col.mean),
          title: "Full file — Min: " + formatStatNum(col.min) + " Max: " + formatStatNum(col.max) +
            " Mean: " + formatStatNum(col.mean) + " Stdev: " + formatStatNum(col.stdev) +
            " Median~: " + formatStatNum(col.median) + " n=" + col.numCount.toLocaleString() + "/" + stats.totalRows.toLocaleString(),
          type: "num"
        };
      } else if (col.type === "seq") {
        return { text: "seq", title: "", type: "seq" };
      } else {
        var uLabel = typeof col.unique === "string" ? col.unique : (col.unique > 100 ? "100+" : col.unique);
        return { text: uLabel + " uniq", title: col.unique + " unique values in " + stats.totalRows.toLocaleString() + " rows (full file)", type: "text" };
      }
    });
  }

  function addChartCard(container, title, svgNode, span) {
    var card = document.createElement("div");
    card.className = "vw-stat-card";
    card.style.gridColumn = "span " + (span || 2);
    card.innerHTML = '<div class="vw-stat-label">' + escapeHtml(title) + '</div>';
    card.appendChild(svgNode);
    // Individual chart save button
    var saveBtn = document.createElement("button");
    saveBtn.className = "vw-chart-save";
    saveBtn.textContent = "Save PNG";
    saveBtn.title = "Save this chart as PNG image";
    saveBtn.addEventListener("click", function() {
      var svg = card.querySelector("svg");
      if (!svg) return;
      saveSingleChartPng(svg, title.replace(/[^a-zA-Z0-9]+/g, "_").toLowerCase());
    });
    card.appendChild(saveBtn);
    container.appendChild(card);
  }

  function saveSingleChartPng(svg, filename) {
    var w = parseInt(svg.getAttribute("width")) || svg.getBoundingClientRect().width || 500;
    var h = parseInt(svg.getAttribute("height")) || svg.getBoundingClientRect().height || 200;
    var scale = 2;
    var canvas = document.createElement("canvas");
    canvas.width = w * scale;
    canvas.height = h * scale;
    var ctx = canvas.getContext("2d");
    ctx.scale(scale, scale);
    ctx.fillStyle = "#0d1117";
    ctx.fillRect(0, 0, w, h);
    var clone = svg.cloneNode(true);
    clone.setAttribute("xmlns", SVG_NS);
    var blob = new Blob([new XMLSerializer().serializeToString(clone)], { type: "image/svg+xml" });
    var url = URL.createObjectURL(blob);
    var img = new Image();
    img.onload = function() {
      ctx.drawImage(img, 0, 0, w, h);
      URL.revokeObjectURL(url);
      var a = document.createElement("a");
      a.href = canvas.toDataURL("image/png");
      a.download = filename + ".png";
      a.click();
    };
    img.src = url;
  }

  function renderRawView(f) {
    var raw = document.createElement("div");
    raw.className = "vw-raw";
    var source = f.text || f.rawPreview || "";
    var lines = source.split("\n");
    var limit = Math.min(lines.length, 10000);
    var html = [];
    for (var i = 0; i < limit; i++) {
      html.push('<span class="line-num">' + (i + 1) + '</span>' + escapeHtml(lines[i]));
    }
    if (lines.length > 10000) {
      html.push('<span style="color:var(--vw-text-muted)">\n... ' + (lines.length - 10000).toLocaleString() + ' more lines ...</span>');
    }
    raw.innerHTML = html.join("\n");
    contentEl.appendChild(raw);
  }

  function renderConsoleView(f) {
    var con = document.createElement("div");
    con.className = "vw-console";
    var out = document.createElement("div");
    out.className = "vw-console-output";
    out.id = "vw-console-output";
    out.innerHTML = '<span style="color:var(--vw-green)">BioLang Console</span>\n' +
      '<span style="color:var(--vw-text-dim)">File loaded as variable: data (Table with ' + f.parsed.rows.length + ' rows)</span>\n' +
      '<span style="color:var(--vw-text-dim)">Columns: ' + f.parsed.columns.join(", ") + '</span>\n' +
      '<span style="color:var(--vw-text-dim)">Try: data |> head(5), mean(column(data, "' + (f.parsed.columns.find(function(c) { return f.parsed.colTypes[f.parsed.columns.indexOf(c)] === "num"; }) || f.parsed.columns[0]) + '")), etc.</span>\n\n';
    con.appendChild(out);

    var inputRow = document.createElement("div");
    inputRow.className = "vw-console-input";
    var input = document.createElement("input");
    input.id = "vw-console-input";
    input.placeholder = "BioLang expression...";
    input.addEventListener("keydown", function(e) {
      if (e.key === "Enter") runConsoleExpr();
    });
    var runBtn = document.createElement("button");
    runBtn.textContent = "Run";
    runBtn.addEventListener("click", runConsoleExpr);
    inputRow.appendChild(input);
    inputRow.appendChild(runBtn);
    con.appendChild(inputRow);
    contentEl.appendChild(con);

    // Pre-load the data into WASM
    injectDataIntoWasm(f);
  }

  function injectDataIntoWasm(f) {
    loadWasm(function(err) {
      if (err) {
        appendConsole("Error loading WASM: " + String(err), "error");
        return;
      }
      // Build a BioLang table expression from the parsed data
      var cols = f.parsed.columns;
      var numCols = f.parsed.colTypes.map(function(t) { return t === "num"; });

      // For large datasets, only inject first 10000 rows
      var maxRows = Math.min(f.parsed.rows.length, 10000);
      var code = "let data = table({\n";
      cols.forEach(function(col, ci) {
        var vals = [];
        for (var ri = 0; ri < maxRows; ri++) {
          var v = f.parsed.rows[ri][ci];
          if (numCols[ci]) {
            vals.push(v === undefined || v === "" ? "0" : String(v));
          } else {
            vals.push('"' + String(v || "").replace(/\\/g, "\\\\").replace(/"/g, '\\"').substring(0, 200) + '"');
          }
        }
        code += '  "' + col + '": [' + vals.join(", ") + ']';
        if (ci < cols.length - 1) code += ",";
        code += "\n";
      });
      code += "})\n";

      try {
        var result = JSON.parse(wasm.evaluate(code));
        if (result.ok) {
          appendConsole("Loaded " + maxRows + " rows into 'data' variable", "info");
          if (maxRows < f.parsed.rows.length) {
            appendConsole("(showing first " + maxRows + " of " + f.parsed.rows.length + " rows)", "dim");
          }
        } else {
          appendConsole("Error loading data: " + (result.error || "unknown"), "error");
        }
      } catch (e) {
        appendConsole("Error: " + String(e), "error");
      }
    });
  }

  function runConsoleExpr() {
    var input = document.getElementById("vw-console-input");
    var expr = input.value.trim();
    if (!expr) return;
    input.value = "";

    appendConsole("bl> " + expr, "cmd");

    if (!wasm) {
      appendConsole("WASM not loaded yet...", "error");
      return;
    }

    try {
      var result = JSON.parse(wasm.evaluate(expr));
      if (result.output && result.output.trim()) {
        appendConsole(result.output.trimEnd(), "out");
      }
      if (result.ok) {
        if (result.value && result.value !== "nil" && result.value !== "Nil") {
          appendConsole(result.value, "value");
        }
      } else {
        appendConsole(result.error || "Error", "error");
      }
    } catch (e) {
      appendConsole("Runtime error: " + String(e), "error");
    }
  }

  function appendConsole(text, type) {
    var out = document.getElementById("vw-console-output");
    if (!out) return;
    var colors = { cmd: "var(--vw-cyan)", out: "var(--vw-text)", value: "var(--vw-green)", error: "var(--vw-red)", info: "var(--vw-accent)", dim: "var(--vw-text-muted)" };
    out.innerHTML += '<span style="color:' + (colors[type] || colors.out) + '">' + escapeHtml(text) + '</span>\n';
    out.scrollTop = out.scrollHeight;
  }

  // ── Motif highlighting ─────────────────────────────────────────
  function colorSequenceWithMotif(seq, pattern) {
    try {
      var re = new RegExp(pattern, "gi");
    } catch (e) {
      return colorSequence(seq); // invalid regex, fallback
    }
    // Find all motif match ranges
    var matches = [];
    var m;
    while ((m = re.exec(seq)) !== null) {
      matches.push({ start: m.index, end: m.index + m[0].length });
      if (m[0].length === 0) break; // prevent infinite loop on zero-length match
    }
    if (!matches.length) return colorSequence(seq);

    var parts = [];
    var mi = 0;
    for (var i = 0; i < seq.length; i++) {
      var inMotif = false;
      while (mi < matches.length && matches[mi].end <= i) mi++;
      if (mi < matches.length && i >= matches[mi].start && i < matches[mi].end) inMotif = true;

      var c = seq.charAt(i).toUpperCase();
      var ntClass = (c === "A" || c === "T" || c === "C" || c === "G" || c === "U" || c === "N")
        ? ' class="nt-' + (c === "U" ? "T" : c) + '"' : "";
      if (inMotif) {
        parts.push('<span' + ntClass + ' style="background:rgba(52,211,153,0.3);border-radius:2px">' + seq.charAt(i) + '</span>');
      } else {
        parts.push(ntClass ? '<span' + ntClass + '>' + seq.charAt(i) + '</span>' : seq.charAt(i));
      }
    }
    return parts.join("");
  }

  // ── Go to row ─────────────────────────────────────────────────
  function showGotoRow(rows, selectRow) {
    var existing = document.querySelector(".vw-goto-overlay");
    if (existing) { existing.remove(); return; }

    var overlay = document.createElement("div");
    overlay.className = "vw-goto-overlay";
    overlay.innerHTML = '<div style="font-size:13px;color:var(--vw-text-dim);margin-bottom:8px">Go to row (1–' + rows.length + ')</div>';
    var inp = document.createElement("input");
    inp.type = "number";
    inp.min = 1;
    inp.max = rows.length;
    inp.placeholder = "Row number...";
    inp.addEventListener("keydown", function(e) {
      if (e.key === "Enter") {
        var n = parseInt(inp.value);
        if (n >= 1 && n <= rows.length) {
          selectRow(n - 1);
          overlay.remove();
        }
      } else if (e.key === "Escape") {
        overlay.remove();
      }
    });
    overlay.appendChild(inp);
    document.body.appendChild(overlay);
    inp.focus();

    // Close on click outside
    function closeOverlay(ev) {
      if (!overlay.contains(ev.target)) {
        overlay.remove();
        document.removeEventListener("mousedown", closeOverlay);
      }
    }
    setTimeout(function() { document.addEventListener("mousedown", closeOverlay); }, 0);
  }

  // ── Export bookmarked rows ────────────────────────────────────
  function exportBookmarked(f) {
    var bms = bookmarkedRows[activeTab];
    if (!bms || bms.size === 0) {
      alert("No bookmarked rows. Click ☆ stars to bookmark rows first.");
      return;
    }
    var lines = [f.parsed.columns.join("\t")];
    bms.forEach(function(idx) {
      if (f.parsed.rows[idx]) lines.push(f.parsed.rows[idx].join("\t"));
    });
    var blob = new Blob([lines.join("\n")], { type: "text/tab-separated-values" });
    var a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = f.name.replace(/\.[^.]+$/, "") + "_bookmarked.tsv";
    a.click();
    URL.revokeObjectURL(a.href);
  }

  // ── Feature 1: Sparkline generator ────────────────────────────
  function svgSparkline(values, w, h, color) {
    w = w || 50; h = h || 16; color = color || "#a78bfa";
    if (!values || values.length < 2) return null;
    var min = Infinity, max = -Infinity;
    for (var i = 0; i < values.length; i++) {
      if (values[i] < min) min = values[i];
      if (values[i] > max) max = values[i];
    }
    if (min === max) max = min + 1;
    var range = max - min;
    var step = w / (values.length - 1);
    var pts = [];
    for (var i = 0; i < values.length; i++) {
      pts.push((i * step).toFixed(1) + "," + (h - ((values[i] - min) / range) * (h - 2) - 1).toFixed(1));
    }
    var s = document.createElementNS(SVG_NS, "svg");
    s.setAttribute("width", w);
    s.setAttribute("height", h);
    s.setAttribute("viewBox", "0 0 " + w + " " + h);
    s.classList.add("vw-sparkline");
    var polyline = document.createElementNS(SVG_NS, "polyline");
    polyline.setAttribute("points", pts.join(" "));
    polyline.setAttribute("fill", "none");
    polyline.setAttribute("stroke", color);
    polyline.setAttribute("stroke-width", "1.5");
    polyline.setAttribute("stroke-linecap", "round");
    polyline.setAttribute("stroke-linejoin", "round");
    s.appendChild(polyline);
    return s;
  }

  // ── Feature 2: Keyboard shortcut help overlay ────────────────
  function showHelpOverlay() {
    if (document.querySelector(".vw-help-overlay")) { closeHelpOverlay(); return; }
    var backdrop = document.createElement("div");
    backdrop.className = "vw-help-backdrop";
    backdrop.addEventListener("click", closeHelpOverlay);
    document.body.appendChild(backdrop);

    var overlay = document.createElement("div");
    overlay.className = "vw-help-overlay";
    overlay.innerHTML =
      '<button class="vw-help-close">&times;</button>' +
      '<h3>Keyboard Shortcuts</h3>' +
      '<table>' +
      '<tr><td>?</td><td>Show/hide this help</td></tr>' +
      '<tr><td>j / &darr;</td><td>Next row</td></tr>' +
      '<tr><td>k / &uarr;</td><td>Previous row</td></tr>' +
      '<tr><td>g</td><td>Jump to first row</td></tr>' +
      '<tr><td>G</td><td>Jump to last row</td></tr>' +
      '<tr><td>Ctrl+G</td><td>Go to row number</td></tr>' +
      '<tr><td>Enter</td><td>Open row detail (if enabled)</td></tr>' +
      '<tr><td>/</td><td>Focus search box</td></tr>' +
      '<tr><td>Esc</td><td>Close panel / overlay</td></tr>' +
      '<tr><td>1-4</td><td>Switch view (Table/Stats/Raw/BioLang)</td></tr>' +
      '</table>' +
      '<h3 style="margin-top:14px">Mouse</h3>' +
      '<table>' +
      '<tr><td>Click header</td><td>Sort: asc &rarr; desc &rarr; default</td></tr>' +
      '<tr><td>Right-click header</td><td>Column value filter</td></tr>' +
      '<tr><td>Right-click cell</td><td>Copy cell / row</td></tr>' +
      '<tr><td>Drag header</td><td>Reorder columns</td></tr>' +
      '<tr><td>Ctrl+click row</td><td>Multi-select rows (summary in footer)</td></tr>' +
      '<tr><td>Hover num header</td><td>Quick stats tooltip</td></tr>' +
      '</table>' +
      '<h3 style="margin-top:14px">Toolbar</h3>' +
      '<table>' +
      '<tr><td>&#9734; Bookmarks</td><td>Toggle bookmark column</td></tr>' +
      '<tr><td>&#128204; Pin Cols</td><td>Freeze first 2 columns (wide tables)</td></tr>' +
      '<tr><td>&#8597; Detail</td><td>Toggle row click detail panel</td></tr>' +
      '<tr><td>&#9788; Theme</td><td>Toggle light/dark mode</td></tr>' +
      '</table>';
    document.body.appendChild(overlay);
    overlay.querySelector(".vw-help-close").addEventListener("click", closeHelpOverlay);
  }

  function closeHelpOverlay() {
    var o = document.querySelector(".vw-help-overlay");
    var b = document.querySelector(".vw-help-backdrop");
    if (o) o.remove();
    if (b) b.remove();
  }

  // ── Feature 3: Chromosome/category distribution strip ────────
  function renderChromStrip(f) {
    var chromFormats = { sam: 2, vcf: 0, bed: 0, gff: 0 };
    if (!(f.parsed.format in chromFormats)) return null;
    var chromCol = chromFormats[f.parsed.format];
    var counts = {};
    f.parsed.rows.forEach(function(r) {
      var c = String(r[chromCol] || "");
      if (c && c !== "*") counts[c] = (counts[c] || 0) + 1;
    });
    var entries = Object.entries(counts).sort(function(a, b) {
      var na = a[0].replace(/^chr/i, ""), nb = b[0].replace(/^chr/i, "");
      var ia = parseInt(na), ib = parseInt(nb);
      if (!isNaN(ia) && !isNaN(ib)) return ia - ib;
      if (!isNaN(ia)) return -1;
      if (!isNaN(ib)) return 1;
      return na.localeCompare(nb);
    }).slice(0, 30);
    if (entries.length < 2) return null;

    var strip = document.createElement("div");
    strip.className = "vw-chrom-strip";
    var label = document.createElement("span");
    label.style.cssText = "font-family:var(--vw-sans);font-size:10px;font-weight:600;color:var(--vw-text-muted);text-transform:uppercase;letter-spacing:0.5px;white-space:nowrap;margin-right:4px;";
    label.textContent = f.parsed.format === "sam" ? "Chroms" : "Regions";
    strip.appendChild(label);

    entries.forEach(function(e) {
      var tag = document.createElement("span");
      tag.className = "vw-chrom-tag";
      tag.innerHTML = escapeHtml(e[0]) + ' <span class="chrom-count">' + e[1].toLocaleString() + '</span>';
      tag.addEventListener("click", function() {
        // Quick filter to this chromosome
        if (!colFilters[chromCol]) colFilters[chromCol] = new Set([e[0]]);
        else if (colFilters[chromCol].has(e[0]) && colFilters[chromCol].size === 1) delete colFilters[chromCol];
        else colFilters[chromCol] = new Set([e[0]]);
        renderView();
      });
      strip.appendChild(tag);
    });
    return strip;
  }

  // ── Feature 4: Context menu (copy cell/row) ──────────────────
  function showContextMenu(e, f, rowData, colIndex) {
    e.preventDefault();
    closeContextMenu();
    var menu = document.createElement("div");
    menu.className = "vw-ctx-menu";
    menu.id = "vw-ctx-menu";

    var cellVal = String(rowData[colIndex] || "");
    var items = [
      { label: "Copy cell", key: "", action: function() { navigator.clipboard.writeText(cellVal); } },
      { label: "Copy row (TSV)", key: "", action: function() { navigator.clipboard.writeText(rowData.join("\t")); } },
      { label: "Copy row (CSV)", key: "", action: function() {
        navigator.clipboard.writeText(rowData.map(function(v) {
          var s = String(v);
          return (s.indexOf(",") !== -1 || s.indexOf('"') !== -1) ? '"' + s.replace(/"/g, '""') + '"' : s;
        }).join(","));
      }},
      { label: "Copy as Markdown", key: "", action: function() {
        var cols = f.parsed.columns;
        var hdr = "| " + cols.join(" | ") + " |";
        var sep = "| " + cols.map(function() { return "---"; }).join(" | ") + " |";
        var row = "| " + rowData.map(function(v) { return String(v).substring(0, 50).replace(/\|/g, "\\|"); }).join(" | ") + " |";
        navigator.clipboard.writeText(hdr + "\n" + sep + "\n" + row);
      }},
      { label: "Copy selection as Markdown", key: "", action: function() {
        if (selectedRows.size === 0) { navigator.clipboard.writeText("(no rows selected)"); return; }
        var cols = f.parsed.columns;
        var lines = ["| " + cols.join(" | ") + " |", "| " + cols.map(function() { return "---"; }).join(" | ") + " |"];
        selectedRows.forEach(function(idx) {
          if (f.parsed.rows[idx]) {
            lines.push("| " + f.parsed.rows[idx].map(function(v) { return String(v).substring(0, 50).replace(/\|/g, "\\|"); }).join(" | ") + " |");
          }
        });
        navigator.clipboard.writeText(lines.join("\n"));
      }}
    ];

    // Feature 12: Reverse complement for sequence cells
    var colType = f.parsed.colTypes[colIndex];
    if (colType === "seq" && cellVal.length >= 1) {
      items.push({ label: "Copy Reverse Complement", key: "", action: function() {
        navigator.clipboard.writeText(reverseComplement(cellVal));
        showToast("Reverse complement copied");
      }});
    }
    // Translate to protein for sequence cells
    if (colType === "seq" && cellVal.length >= 3 && cellVal.length <= 9000) {
      items.push({ label: "Translate to Protein", key: "", action: function() {
        navigator.clipboard.writeText(translateDNA(cellVal));
        showToast("Protein translation copied");
      }});
    }
    // Copy as FASTA for sequence cells
    if (colType === "seq" || (colType === "str" && /^[ATCGNatcgn]{10,}$/.test(cellVal))) {
      items.push({ label: "Copy as FASTA", key: "", action: function() {
        var header = ">" + (rowData[0] || "sequence") + "\n";
        var seq = cellVal.match(/.{1,80}/g).join("\n");
        navigator.clipboard.writeText(header + seq);
        showToast("Copied as FASTA");
      }});
    }
    // BLAST option for sequence columns
    if (colType === "seq" && cellVal.length >= 10) {
      items.push({ label: "BLAST this sequence", key: "", action: function() { openBLAST(cellVal); } });
    }
    // Also offer BLAST for FASTA sequence column
    if (f.parsed.format === "fasta" && colIndex === 1 && cellVal.length >= 10) {
      if (colType !== "seq") { // avoid duplicate if already added above
        items.push({ label: "BLAST this sequence", key: "", action: function() { openBLAST(cellVal); } });
      }
    }

    // UCSC Genome Browser link for genomic-coordinate formats
    if (f.parsed.format === "vcf" || f.parsed.format === "bed" || f.parsed.format === "gff" || f.parsed.format === "sam") {
      var chromIdx = f.parsed.columns.findIndex(function(c) { return /^(chrom|#chrom|chr|seqname|rname)/i.test(c); });
      var posIdx = f.parsed.columns.findIndex(function(c) { return /^(pos|start|chromstart)/i.test(c); });
      if (chromIdx >= 0 && posIdx >= 0) {
        var ucscChrom = String(rowData[chromIdx]);
        var ucscPos = parseInt(rowData[posIdx]) || 1;
        var endIdx = f.parsed.columns.findIndex(function(c) { return /^(end|chromend)/i.test(c); });
        var ucscEnd = endIdx >= 0 ? parseInt(rowData[endIdx]) || ucscPos + 100 : ucscPos + 100;
        items.push({ label: "Open in UCSC Browser", key: "", action: function() {
          window.open("https://genome.ucsc.edu/cgi-bin/hgTracks?db=hg38&position=" + ucscChrom + ":" + ucscPos + "-" + ucscEnd, "_blank");
        }});
      }
    }

    // Feature 13: dbSNP Lookup for rs IDs in VCF
    if (f.parsed.format === "vcf" && /^rs\d+/.test(cellVal)) {
      items.push({ label: "Lookup in dbSNP: " + cellVal, key: "", action: function() {
        window.open("https://www.ncbi.nlm.nih.gov/snp/" + cellVal, "_blank");
      }});
    }

    // Feature 14: GeneCards link for gene names
    if (/^[A-Z][A-Z0-9]{1,9}$/.test(cellVal)) {
      items.push({ label: "GeneCards: " + cellVal, key: "", action: function() {
        window.open("https://www.genecards.org/cgi-bin/carddisp.pl?gene=" + encodeURIComponent(cellVal), "_blank");
      }});
    }

    // Feature 15: Ensembl Region Link for genomic formats
    if (f.parsed.format === "vcf" || f.parsed.format === "bed" || f.parsed.format === "gff" || f.parsed.format === "sam") {
      var ensChromIdx = f.parsed.columns.findIndex(function(c) { return /^(chrom|#chrom|chr|seqname|seqid|rname)/i.test(c); });
      var ensPosIdx = f.parsed.columns.findIndex(function(c) { return /^(pos|start|chromstart)/i.test(c); });
      if (ensChromIdx >= 0 && ensPosIdx >= 0) {
        var ensChrom = String(rowData[ensChromIdx]).replace(/^chr/i, "");
        var ensStart = parseInt(rowData[ensPosIdx]) || 1;
        var ensEndIdx = f.parsed.columns.findIndex(function(c) { return /^(end|chromend)/i.test(c); });
        var ensEnd = ensEndIdx >= 0 ? (parseInt(rowData[ensEndIdx]) || ensStart + 100) : ensStart + 100;
        items.push({ label: "View in Ensembl", key: "", action: function() {
          window.open("https://ensembl.org/Homo_sapiens/Location/View?r=" + ensChrom + ":" + ensStart + "-" + ensEnd, "_blank");
        }});
      }
    }

    // Copy as BED interval for VCF rows
    if (f.parsed && f.parsed.format === "vcf") {
      var bedChromIdx = f.parsed.columns.indexOf("CHROM");
      var bedPosIdx = f.parsed.columns.indexOf("POS");
      var bedRefIdx = f.parsed.columns.indexOf("REF");
      if (bedChromIdx >= 0 && bedPosIdx >= 0) {
        items.push({ label: "Copy as BED", key: "", action: function() {
          var chrom = rowData[bedChromIdx];
          var pos = parseInt(rowData[bedPosIdx]);
          var refLen = bedRefIdx >= 0 ? String(rowData[bedRefIdx]).length : 1;
          var bed = chrom + "\t" + (pos - 1) + "\t" + (pos - 1 + refLen);
          navigator.clipboard.writeText(bed);
          showToast("Copied as BED interval");
        }});
        // Copy selected rows as BED when multiple rows are selected
        if (selectedRows && selectedRows.size > 1) {
          items.push({ label: "Copy selected as BED", key: "", action: function() {
            var lines = [];
            selectedRows.forEach(function(idx) {
              var row = f.parsed.rows[idx];
              if (row) {
                var chrom = row[bedChromIdx];
                var pos = parseInt(row[bedPosIdx]);
                var refLen = bedRefIdx >= 0 ? String(row[bedRefIdx]).length : 1;
                lines.push(chrom + "\t" + (pos - 1) + "\t" + (pos - 1 + refLen));
              }
            });
            navigator.clipboard.writeText(lines.join("\n"));
            showToast("Copied " + lines.length + " BED intervals");
          }});
        }
      }
    }

    // Cross-tab highlight — search for this value in all other tabs
    if (cellVal.length >= 2 && cellVal.length <= 100) {
      items.push({ label: "Find in other tabs", key: "", action: function() {
        var matches = 0;
        files.forEach(function(other, ti) {
          if (ti === activeTab) return;
          for (var ri = 0; ri < Math.min(other.parsed.rows.length, 10000); ri++) {
            if (other.parsed.rows[ri].some(function(c) { return String(c) === cellVal; })) {
              matches++;
              break;
            }
          }
        });
        if (matches > 0) {
          showToast("\"" + cellVal.substring(0, 30) + "\" found in " + matches + " other tab(s)");
          // Set as search term to highlight
          searchTerm = cellVal;
          searchInput.value = cellVal;
        } else {
          showToast("\"" + cellVal.substring(0, 30) + "\" not found in other tabs");
        }
      }});
    }

    items.forEach(function(item) {
      var div = document.createElement("div");
      div.className = "vw-ctx-item";
      div.innerHTML = escapeHtml(item.label) + (item.key ? '<span class="ctx-key">' + item.key + '</span>' : '');
      div.addEventListener("click", function() { item.action(); closeContextMenu(); });
      menu.appendChild(div);
    });

    menu.style.left = Math.min(e.clientX, window.innerWidth - 200) + "px";
    menu.style.top = Math.min(e.clientY, window.innerHeight - items.length * 32 - 16) + "px";
    document.body.appendChild(menu);
    setTimeout(function() {
      document.addEventListener("mousedown", function handler(ev) {
        if (!menu.contains(ev.target)) { closeContextMenu(); document.removeEventListener("mousedown", handler); }
      });
    }, 0);
  }

  function closeContextMenu() {
    var m = document.getElementById("vw-ctx-menu");
    if (m) m.remove();
  }

  // ── Feature 5: FASTQ QC verdict ──────────────────────────────
  function fastqQcVerdict(stats) {
    var q30str = String(stats["Q30+ reads"] || "0");
    var q30 = parseFloat(q30str);
    var meanQ = parseFloat(stats["Mean quality"] || 0);
    if (q30 >= 80 && meanQ >= 25) return { verdict: "PASS", cls: "vw-qc-pass", detail: "Q30 " + q30str + ", Mean Q" + meanQ };
    if (q30 >= 60 && meanQ >= 20) return { verdict: "WARN", cls: "vw-qc-warn", detail: "Q30 " + q30str + ", Mean Q" + meanQ };
    return { verdict: "FAIL", cls: "vw-qc-fail", detail: "Q30 " + q30str + ", Mean Q" + meanQ };
  }

  // ── Feature 6: VCF INFO field parsing ────────────────────────
  function parseInfoField(infoStr) {
    if (!infoStr || infoStr === ".") return [];
    return String(infoStr).split(";").map(function(pair) {
      var eq = pair.indexOf("=");
      if (eq === -1) return { key: pair, value: "true" };
      return { key: pair.substring(0, eq), value: pair.substring(eq + 1) };
    });
  }

  // ── Feature 8: Stats comparison across tabs ──────────────────
  function renderStatsComparison(container, currentFormat) {
    var sameFormatFiles = [];
    files.forEach(function(f, i) {
      if (f.parsed.format === currentFormat) sameFormatFiles.push({ idx: i, file: f });
    });
    if (sameFormatFiles.length < 2) return;

    var card = document.createElement("div");
    card.className = "vw-stat-card";
    card.style.gridColumn = "span " + Math.min(sameFormatFiles.length + 1, 4);
    card.innerHTML = '<div class="vw-stat-label">Comparison — ' + currentFormat.toUpperCase() + ' files</div>';

    var table = document.createElement("table");
    table.className = "vw-compare-table";
    var thead = document.createElement("thead");
    var htr = document.createElement("tr");
    htr.innerHTML = '<th>Metric</th>';
    sameFormatFiles.forEach(function(sf) {
      var th = document.createElement("th");
      th.textContent = sf.file.name.length > 18 ? sf.file.name.substring(0, 17) + "\u2026" : sf.file.name;
      th.title = sf.file.name;
      if (sf.idx === activeTab) th.style.color = "var(--vw-accent)";
      htr.appendChild(th);
    });
    thead.appendChild(htr);
    table.appendChild(thead);

    // Collect all stat keys
    var allKeys = {};
    sameFormatFiles.forEach(function(sf) {
      Object.keys(sf.file.parsed.stats || {}).forEach(function(k) { allKeys[k] = 1; });
    });

    var tbody = document.createElement("tbody");
    Object.keys(allKeys).forEach(function(key) {
      var tr = document.createElement("tr");
      var keyTd = document.createElement("td");
      keyTd.textContent = key;
      keyTd.style.color = "var(--vw-text-dim)";
      tr.appendChild(keyTd);
      sameFormatFiles.forEach(function(sf) {
        var td = document.createElement("td");
        td.textContent = String((sf.file.parsed.stats || {})[key] || "—");
        tr.appendChild(td);
      });
      tbody.appendChild(tr);
    });
    table.appendChild(tbody);
    card.appendChild(table);
    container.appendChild(card);
  }

  // ── Feature 9: View transition animation ─────────────────────
  function animateViewSwitch(callback) {
    contentEl.classList.add("view-fade");
    contentEl.classList.remove("view-show");
    setTimeout(function() {
      callback();
      contentEl.classList.remove("view-fade");
      contentEl.classList.add("view-show");
      setTimeout(function() { contentEl.classList.remove("view-show"); }, 150);
    }, 120);
  }

  // ── Feature 10: Column drag reorder ──────────────────────────
  var dragColIdx = -1;

  function setupColumnDrag(th, colIdx, f) {
    th.draggable = true;
    th.addEventListener("dragstart", function(e) {
      dragColIdx = colIdx;
      th.classList.add("dragging");
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(colIdx));
    });
    th.addEventListener("dragend", function() {
      th.classList.remove("dragging");
      dragColIdx = -1;
      // Clean up all drag-over classes
      document.querySelectorAll(".drag-over-left,.drag-over-right").forEach(function(el) {
        el.classList.remove("drag-over-left", "drag-over-right");
      });
    });
    th.addEventListener("dragover", function(e) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      if (dragColIdx === -1 || dragColIdx === colIdx) return;
      th.classList.remove("drag-over-left", "drag-over-right");
      if (dragColIdx < colIdx) th.classList.add("drag-over-right");
      else th.classList.add("drag-over-left");
    });
    th.addEventListener("dragleave", function() {
      th.classList.remove("drag-over-left", "drag-over-right");
    });
    th.addEventListener("drop", function(e) {
      e.preventDefault();
      th.classList.remove("drag-over-left", "drag-over-right");
      if (dragColIdx === -1 || dragColIdx === colIdx) return;
      reorderColumns(f, dragColIdx, colIdx);
      dragColIdx = -1;
      renderView();
    });
  }

  function reorderColumns(f, fromIdx, toIdx) {
    function moveItem(arr, from, to) {
      var item = arr.splice(from, 1)[0];
      arr.splice(to, 0, item);
    }
    moveItem(f.parsed.columns, fromIdx, toIdx);
    moveItem(f.parsed.colTypes, fromIdx, toIdx);
    f.parsed.rows.forEach(function(row) { moveItem(row, fromIdx, toIdx); });
    f.parsed._colHints = null; // invalidate cache
    // Adjust sort column
    if (sortCol === fromIdx) sortCol = toIdx;
    else if (fromIdx < sortCol && toIdx >= sortCol) sortCol--;
    else if (fromIdx > sortCol && toIdx <= sortCol) sortCol++;
    if (sortCol >= 0) tabSortState[activeTab] = { col: sortCol, asc: sortAsc };
  }

  // ── Feature 11: Selection summary ────────────────────────────
  var selectedRows = new Set();

  function updateSelectionSummary(f) {
    var selEl = document.getElementById("vw-footer-selection");
    if (!selEl) return;
    if (selectedRows.size === 0) {
      selEl.style.display = "none";
      return;
    }
    selEl.style.display = "";
    var parts = [selectedRows.size + " selected"];
    // Compute stats for up to 3 numeric columns
    var numCols = [];
    f.parsed.colTypes.forEach(function(t, i) { if (t === "num") numCols.push(i); });
    var showCols = numCols.slice(0, 3);
    showCols.forEach(function(ci) {
      var sum = 0, count = 0, mn = Infinity, mx = -Infinity;
      selectedRows.forEach(function(idx) {
        var v = f.parsed.rows[idx] ? f.parsed.rows[idx][ci] : NaN;
        if (typeof v === "number" && !isNaN(v)) { sum += v; count++; if (v < mn) mn = v; if (v > mx) mx = v; }
      });
      if (count > 0) {
        var fmt = function(n) { return n.toLocaleString(undefined, {maximumFractionDigits:1}); };
        parts.push(f.parsed.columns[ci] + ": Σ" + fmt(sum) + " μ" + fmt(sum / count) + " [" + fmt(mn) + "–" + fmt(mx) + "]");
      }
    });
    // For sequence formats, show total bp
    var seqCols = [];
    f.parsed.colTypes.forEach(function(t, i) { if (t === "seq") seqCols.push(i); });
    if (seqCols.length > 0) {
      var totalBp = 0;
      selectedRows.forEach(function(idx) {
        var s = f.parsed.rows[idx] ? String(f.parsed.rows[idx][seqCols[0]] || "") : "";
        totalBp += s.length;
      });
      if (totalBp > 0) parts.push(totalBp.toLocaleString() + " bp");
    }
    selEl.textContent = parts.join(" | ");
  }

  // ── Filtering ──────────────────────────────────────────────────
  // Cache the wrapped rows array per file to avoid repeated .map() on large datasets
  var _cachedFile = null, _cachedAll = null;
  function getAllWrapped(f) {
    if (_cachedFile === f && _cachedAll) return _cachedAll;
    _cachedFile = f;
    _cachedAll = f.parsed.rows.map(function(r, i) { return { row: r, idx: i }; });
    return _cachedAll;
  }

  var _filterCache = null, _filterKey = "";
  function getFilteredRows(f) {
    // Cache key: avoid recomputing when called multiple times per render
    var key = activeTab + "|" + searchTerm + "|" + motifTerm + "|" + Object.keys(colFilters).join(",") + "|" + f.parsed.rows.length + "|rx:" + regexSearch + "|bf:" + bookmarkFilterActive;
    if (_filterCache && _filterKey === key) return _filterCache;

    var rows = getAllWrapped(f);
    // Apply column filters
    var hasColFilters = Object.keys(colFilters).length > 0;
    if (hasColFilters) {
      rows = rows.filter(function(item) {
        for (var ci in colFilters) {
          if (!colFilters[ci].has(String(item.row[ci]))) return false;
        }
        return true;
      });
    }
    // Motif filter on sequence columns
    if (motifTerm && motifTerm.length > 0) {
      try {
        var motifRe = new RegExp(motifTerm, "i");
        var ff = files[activeTab];
        var seqCols = ff ? ff.parsed.colTypes.map(function(t, i) { return t === "seq" ? i : -1; }).filter(function(i) { return i >= 0; }) : [];
        if (seqCols.length > 0) {
          rows = rows.filter(function(item) {
            return seqCols.some(function(ci) {
              return motifRe.test(String(item.row[ci]));
            });
          });
        }
      } catch (e) { /* invalid regex — skip filter */ }
    }
    if (searchTerm) {
      if (regexSearch) {
        // Feature 16: Regex search mode
        try {
          var searchRe = new RegExp(searchTerm, "i");
          rows = rows.filter(function(item) {
            return item.row.some(function(cell) {
              return searchRe.test(String(cell));
            });
          });
        } catch (e) { /* invalid regex — skip filter */ }
      } else {
        var term = searchTerm.toLowerCase();
        rows = rows.filter(function(item) {
          return item.row.some(function(cell) {
            return String(cell).toLowerCase().indexOf(term) !== -1;
          });
        });
      }
    }
    // Feature 17: Bookmark filter — show only starred rows
    if (bookmarkFilterActive && bookmarkedRows[activeTab] && bookmarkedRows[activeTab].size > 0) {
      var bms = bookmarkedRows[activeTab];
      rows = rows.filter(function(item) { return bms.has(item.idx); });
    }
    _filterCache = rows;
    _filterKey = key;
    return rows;
  }

  function updateFooter(f) {
    var ff = files[activeTab];
    var hasColFilters = Object.keys(colFilters).length > 0;
    var filtered = (searchTerm || hasColFilters) ? getFilteredRows(f) : getAllWrapped(f);
    var total = filtered.length;
    var totalPages = Math.ceil(total / pageSize) || 1;
    // Show row range for current page
    var pageLabel = "";
    if (totalPages > 1) {
      var pStart = currentPage * pageSize + 1;
      var pEnd = Math.min((currentPage + 1) * pageSize, total);
      pageLabel = " · showing " + pStart.toLocaleString() + "-" + pEnd.toLocaleString() + " · page " + (currentPage + 1) + "/" + totalPages;
    }
    var groupLabel = "";
    if (groupByCol >= 0 && groupByCol < f.parsed.columns.length) {
      var groups = new Set();
      filtered.forEach(function(item) { groups.add(String(item.row[groupByCol])); });
      groupLabel = " · " + groups.size + " groups";
    }
    var hlLabel = highlightRule ? " · highlight: " + f.parsed.columns[highlightRule.col] + " " + highlightRule.op + " " + highlightRule.value : "";
    var truncLabel = (ff && ff.truncated) ? " · PARTIAL (first 50 MB of " + formatBytes(ff.size) + ")" : "";
    footerRows.textContent = total.toLocaleString() + " rows" + pageLabel + groupLabel + hlLabel + truncLabel;
    if (ff && ff.truncated) footerRows.style.color = "var(--vw-amber)";
    else footerRows.style.color = "";
    if ((searchTerm || hasColFilters) && total !== f.parsed.rows.length) {
      footerFilter.style.display = "";
      footerFilter.textContent = "(filtered from " + f.parsed.rows.length.toLocaleString() + ")";
    } else {
      footerFilter.style.display = "none";
    }
  }

  // ── Helpers ────────────────────────────────────────────────────
  function escapeHtml(s) {
    var d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  function formatBytes(b) {
    if (b < 1024) return b + " B";
    if (b < 1048576) return (b / 1024).toFixed(1) + " KB";
    if (b < 1073741824) return (b / 1048576).toFixed(1) + " MB";
    return (b / 1073741824).toFixed(2) + " GB";
  }

  function colorSequence(seq) {
    var parts = [];
    for (var i = 0; i < seq.length; i++) {
      var c = seq.charAt(i).toUpperCase();
      if (c === "A" || c === "T" || c === "C" || c === "G" || c === "U" || c === "N") {
        parts.push('<span class="nt-' + (c === "U" ? "T" : c) + '">' + seq.charAt(i) + '</span>');
      } else {
        parts.push(seq.charAt(i));
      }
    }
    return parts.join("");
  }

  function colorSequenceWithCodons(seq) {
    if (seq.length <= 6) return colorSequence(seq);
    var parts = [];
    for (var i = 0; i < seq.length; i++) {
      var c = seq.charAt(i).toUpperCase();
      var codonGroup = Math.floor(i / 3) % 2;
      var bgOpacity = codonGroup === 0 ? "0.05" : "0.12";
      var inner;
      if (c === "A" || c === "T" || c === "C" || c === "G" || c === "U" || c === "N") {
        inner = '<span class="nt-' + (c === "U" ? "T" : c) + '">' + seq.charAt(i) + '</span>';
      } else {
        inner = seq.charAt(i);
      }
      parts.push('<span style="background:rgba(100,116,139,' + bgOpacity + ')">' + inner + '</span>');
    }
    return parts.join("");
  }

  function colorQuality(qual) {
    var parts = [];
    for (var i = 0; i < qual.length; i++) {
      var q = qual.charCodeAt(i) - 33;
      var cls = q >= 30 ? "q-hi" : q >= 20 ? "q-md" : q >= 10 ? "q-lo" : "q-bad";
      parts.push('<span class="' + cls + '">' + qual.charAt(i) + '</span>');
    }
    return parts.join("");
  }

  function colorCigar(cigar) {
    if (!cigar || cigar === "*") return '<span class="cigar-S">*</span>';
    return cigar.replace(/(\d+)([MIDNSHP=X])/g, function(_, n, op) {
      return '<span class="cigar-' + op + '">' + n + op + '</span>';
    });
  }

  function colorFlag(flag) {
    var parts = [];
    if (flag & 4) parts.push('<span class="flag-unmapped">unmapped</span>');
    else {
      if (flag & 1) parts.push('<span class="flag-paired">paired</span>');
      if (flag & 256) parts.push('<span class="flag-secondary">secondary</span>');
      if (flag & 2048) parts.push('<span class="flag-secondary">supplementary</span>');
      if (flag & 16) parts.push('reverse');
    }
    return flag + (parts.length ? ' <span style="font-size:10px;color:var(--vw-text-muted)">(' + parts.join(", ") + ')</span>' : '');
  }

  function colorFilter(val) {
    var s = String(val);
    if (s === "PASS" || s === ".") return '<span class="filter-pass">' + s + '</span>';
    return '<span class="filter-fail">' + escapeHtml(s) + '</span>';
  }

  // ── Row detail panel ──
  var detailPanel = null;

  function showDetailPanel(f, rowIdx) {
    closeDetailPanel();
    var row = f.parsed.rows[rowIdx];
    if (!row) return;

    detailPanel = document.createElement("div");
    detailPanel.className = "vw-detail-panel";

    var header = document.createElement("div");
    header.className = "vw-detail-header";
    header.innerHTML = '<span>Record #' + (rowIdx + 1) + ' — ' + f.parsed.format.toUpperCase() + '</span>';
    var closeBtn = document.createElement("button");
    closeBtn.className = "vw-detail-close";
    closeBtn.textContent = "\u00d7";
    closeBtn.addEventListener("click", closeDetailPanel);
    header.appendChild(closeBtn);
    detailPanel.appendChild(header);

    var body = document.createElement("div");
    body.className = "vw-detail-body";

    f.parsed.columns.forEach(function(col, ci) {
      var dr = document.createElement("div");
      dr.className = "vw-detail-row";
      var label = document.createElement("div");
      label.className = "vw-detail-label";
      label.textContent = col;
      var value = document.createElement("div");
      value.className = "vw-detail-value";
      var val = row[ci];
      var colType = f.parsed.colTypes[ci];

      if (colType === "seq") {
        value.classList.add("vw-detail-seq");
        value.innerHTML = colorSequence(String(val));
      } else if (colType === "qual") {
        value.classList.add("vw-detail-seq");
        value.innerHTML = colorQuality(String(val));
      } else if (col === "FLAG" && f.parsed.format === "sam") {
        value.innerHTML = colorFlag(val);
      } else if (col === "CIGAR" && f.parsed.format === "sam") {
        value.innerHTML = colorCigar(String(val));
      } else if (col === "FILTER" && f.parsed.format === "vcf") {
        value.innerHTML = colorFilter(val);
      } else if (colType === "num") {
        value.style.color = "var(--vw-cyan)";
        value.textContent = typeof val === "number" ? val.toLocaleString() : val;
      } else {
        value.textContent = String(val);
      }
      dr.appendChild(label);
      dr.appendChild(value);
      body.appendChild(dr);
    });

    // Extra info for SAM
    if (f.parsed.format === "sam") {
      var flag = row[1];
      var extras = [];
      if (flag & 4) extras.push("UNMAPPED");
      if (flag & 1) extras.push("PAIRED");
      if (flag & 2) extras.push("PROPER_PAIR");
      if (flag & 16) extras.push("REVERSE");
      if (flag & 32) extras.push("MATE_REVERSE");
      if (flag & 64) extras.push("READ1");
      if (flag & 128) extras.push("READ2");
      if (flag & 256) extras.push("SECONDARY");
      if (flag & 512) extras.push("FAILED_QC");
      if (flag & 1024) extras.push("DUPLICATE");
      if (flag & 2048) extras.push("SUPPLEMENTARY");
      if (extras.length) {
        var dr2 = document.createElement("div");
        dr2.className = "vw-detail-row";
        dr2.innerHTML = '<div class="vw-detail-label">Flag bits</div><div class="vw-detail-value">' + extras.join(" | ") + '</div>';
        body.appendChild(dr2);
      }
    }

    // Extra info for VCF: parse INFO field into key-value pairs
    if (f.parsed.format === "vcf") {
      var infoIdx = f.parsed.columns.indexOf("INFO");
      if (infoIdx === -1) infoIdx = f.parsed.columns.indexOf("info");
      if (infoIdx >= 0 && row[infoIdx] && row[infoIdx] !== ".") {
        var infoPairs = parseInfoField(row[infoIdx]);
        if (infoPairs.length) {
          var infoHeader = document.createElement("div");
          infoHeader.className = "vw-detail-row";
          infoHeader.innerHTML = '<div class="vw-detail-label" style="color:var(--vw-accent);font-weight:700;border-top:1px solid var(--vw-border);padding-top:8px;margin-top:4px">INFO Fields (' + infoPairs.length + ')</div><div class="vw-detail-value"></div>';
          body.appendChild(infoHeader);
          infoPairs.forEach(function(kv) {
            var dr3 = document.createElement("div");
            dr3.className = "vw-detail-row";
            dr3.innerHTML = '<div class="vw-detail-label" style="padding-left:12px;font-family:monospace;font-size:12px">' + escapeHtml(kv.key) + '</div><div class="vw-detail-value" style="font-family:monospace;font-size:12px">' + escapeHtml(kv.value) + '</div>';
            body.appendChild(dr3);
          });
        }
      }
    }

    detailPanel.appendChild(body);
    document.body.appendChild(detailPanel);
  }

  function closeDetailPanel() {
    if (detailPanel) {
      detailPanel.remove();
      detailPanel = null;
    }
  }

  function makeAsciiHistogram(values) {
    if (!values.length) return "(no data)";
    values = values.filter(function(v) { return typeof v === "number" && !isNaN(v); });
    if (!values.length) return "(no numeric data)";
    var min = Math.min.apply(null, values);
    var max = Math.max.apply(null, values);
    if (min === max) return "All values: " + min;
    var bins = 20;
    var binWidth = (max - min) / bins;
    var counts = new Array(bins).fill(0);
    values.forEach(function(v) {
      var b = Math.min(Math.floor((v - min) / binWidth), bins - 1);
      counts[b]++;
    });
    var maxCount = Math.max.apply(null, counts);
    var barWidth = 30;
    var lines = [];
    for (var i = 0; i < bins; i++) {
      var lo = (min + i * binWidth).toFixed(1);
      var hi = (min + (i + 1) * binWidth).toFixed(1);
      var bar = Math.round(counts[i] / maxCount * barWidth);
      lines.push(padLeft(lo, 8) + " |" + "█".repeat(bar) + (counts[i] ? " " + counts[i] : ""));
    }
    return lines.join("\n");
  }

  function padLeft(s, n) { while (s.length < n) s = " " + s; return s; }

  // ── Export ─────────────────────────────────────────────────────
  function exportData(format) {
    var f = files[activeTab];
    if (!f) return;
    var rows = getFilteredRows(f);

    // Subset modes
    if (format.indexOf(":selected") !== -1) {
      rows = rows.filter(function(item) { return selectedRows.has(item.idx); });
      format = format.split(":")[0];
    } else if (format.indexOf(":page") !== -1) {
      var ps = currentPage * pageSize;
      rows = rows.slice(ps, ps + pageSize);
      format = format.split(":")[0];
    }

    var text, mime, ext;

    if (format === "tsv") {
      var lines = [f.parsed.columns.join("\t")];
      rows.forEach(function(item) { lines.push(item.row.join("\t")); });
      text = lines.join("\n");
      mime = "text/tab-separated-values";
      ext = ".tsv";
    } else if (format === "bed" && f.parsed.format === "bed") {
      var lines = [];
      rows.forEach(function(item) { lines.push(item.row.slice(0, Math.min(item.row.length, 12)).join("\t")); });
      text = lines.join("\n");
      mime = "text/plain";
      ext = ".bed";
    } else if (format === "vcf" && f.parsed.format === "vcf") {
      var lines = ["#" + f.parsed.columns.join("\t")];
      rows.forEach(function(item) { lines.push(item.row.join("\t")); });
      text = lines.join("\n");
      mime = "text/plain";
      ext = ".vcf";
    } else {
      // CSV (default)
      var lines = [f.parsed.columns.join(",")];
      rows.forEach(function(item) {
        lines.push(item.row.map(function(v) {
          var s = String(v);
          if (s.indexOf(",") !== -1 || s.indexOf('"') !== -1 || s.indexOf("\n") !== -1) {
            return '"' + s.replace(/"/g, '""') + '"';
          }
          return s;
        }).join(","));
      });
      text = lines.join("\n");
      mime = "text/csv";
      ext = ".csv";
    }

    var blob = new Blob([text], { type: mime });
    var a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = f.name.replace(/\.[^.]+$/, "") + (searchTerm || Object.keys(colFilters).length ? "_filtered" : "") + ext;
    a.click();
  }

  // ── WASM ───────────────────────────────────────────────────────
  function loadWasm(callback) {
    if (wasm) { callback(null); return; }
    if (wasmLoading) { wasmQueue.push(callback); return; }
    wasmLoading = true;
    wasmQueue.push(callback);

    var script = document.createElement("script");
    script.type = "module";
    // Chrome extensions block inline module scripts (CSP); use external loader
    if (typeof chrome !== "undefined" && chrome.runtime && chrome.runtime.getURL) {
      script.src = chrome.runtime.getURL("wasm-loader.js");
    } else {
      script.textContent = [
        'try {',
        '  var mod = await import("./wasm/bl_wasm.js");',
        '  await mod.default();',
        '  mod.init();',
        '  window.__blWasm = { evaluate: mod.evaluate, reset: mod.reset };',
        '  window.dispatchEvent(new Event("bl-wasm-ready"));',
        '} catch(e) {',
        '  window.__blWasmError = e;',
        '  window.dispatchEvent(new Event("bl-wasm-error"));',
        '}'
      ].join("\n");
    }
    document.head.appendChild(script);

    window.addEventListener("bl-wasm-ready", function() {
      wasm = window.__blWasm;
      wasmLoading = false;
      var q = wasmQueue.slice(); wasmQueue = [];
      q.forEach(function(cb) { cb(null); });
    }, { once: true });

    window.addEventListener("bl-wasm-error", function() {
      wasmLoading = false;
      var err = window.__blWasmError || new Error("WASM load failed");
      var q = wasmQueue.slice(); wasmQueue = [];
      q.forEach(function(cb) { cb(err); });
    }, { once: true });
  }

  // ── Feature: Smart Clipboard Paste Detection ──────────────────
  document.addEventListener("paste", function(e) {
    // Only handle paste when not in an input field
    var tag = (document.activeElement || {}).tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return;
    var text = (e.clipboardData || window.clipboardData || { getData: function() { return ""; } }).getData("text");
    if (!text || text.length < 5) return;

    // Auto-detect format from content
    var name = "pasted";
    if (text.charAt(0) === ">") name = "pasted.fasta";
    else if (text.charAt(0) === "@" && text.indexOf("+\n") >= 0) name = "pasted.fastq";
    else if (text.indexOf("##fileformat=VCF") === 0) name = "pasted.vcf";
    else if (text.indexOf("##gff") === 0 || text.indexOf("##gff-version") === 0) name = "pasted.gff";
    else if (/^chr\w+\t\d+\t\d+/.test(text)) name = "pasted.bed";
    else if (text.indexOf(",") >= 0 && text.indexOf("\n") >= 0) name = "pasted.csv";
    else if (text.indexOf("\t") >= 0 && text.indexOf("\n") >= 0) name = "pasted.tsv";
    else if (/^[ATCGNatcgn\s]{20,}$/.test(text.trim())) name = "pasted.fasta";
    else name = "pasted.txt";

    // If raw sequence without FASTA header, wrap it
    if (name === "pasted.fasta" && text.charAt(0) !== ">") {
      text = ">pasted_sequence\n" + text.trim();
    }

    e.preventDefault();
    addFile(name, text.length, text, false);
  });

  // ── Feature: Sequence Tools Popup (2) ────────────────────────
  var seqPopup = null;

  function reverseComplement(seq) {
    var comp = { A: "T", T: "A", C: "G", G: "C", a: "t", t: "a", c: "g", g: "c", N: "N", n: "n" };
    return seq.split("").reverse().map(function(c) { return comp[c] || c; }).join("");
  }

  function calcGC(seq) {
    var s = seq.toUpperCase();
    var gc = 0, total = 0;
    for (var i = 0; i < s.length; i++) {
      if ("ATCG".indexOf(s[i]) >= 0) { total++; if (s[i] === "G" || s[i] === "C") gc++; }
    }
    return total > 0 ? (gc / total * 100).toFixed(1) + "%" : "N/A";
  }

  function translateDNA(seq) {
    var codons = {
      TTT:"F",TTC:"F",TTA:"L",TTG:"L",CTT:"L",CTC:"L",CTA:"L",CTG:"L",
      ATT:"I",ATC:"I",ATA:"I",ATG:"M",GTT:"V",GTC:"V",GTA:"V",GTG:"V",
      TCT:"S",TCC:"S",TCA:"S",TCG:"S",CCT:"P",CCC:"P",CCA:"P",CCG:"P",
      ACT:"T",ACC:"T",ACA:"T",ACG:"T",GCT:"A",GCC:"A",GCA:"A",GCG:"A",
      TAT:"Y",TAC:"Y",TAA:"*",TAG:"*",CAT:"H",CAC:"H",CAA:"Q",CAG:"Q",
      AAT:"N",AAC:"N",AAA:"K",AAG:"K",GAT:"D",GAC:"D",GAA:"E",GAG:"E",
      TGT:"C",TGC:"C",TGA:"*",TGG:"W",CGT:"R",CGC:"R",CGA:"R",CGG:"R",
      AGT:"S",AGC:"S",AGA:"R",AGG:"R",GGT:"G",GGC:"G",GGA:"G",GGG:"G"
    };
    var s = seq.toUpperCase().replace(/[^ATCG]/g, "");
    var aa = [];
    for (var i = 0; i + 2 < s.length; i += 3) aa.push(codons[s.substr(i, 3)] || "?");
    return aa.join("");
  }

  function compressDNA2bit(fastaText) {
    var records = [];
    var lines = fastaText.split("\n");
    var header = "", seq = [];
    function flush() {
      if (header || seq.length) {
        var s = seq.join("");
        var bytes = [];
        var map = {A:0, a:0, C:1, c:1, G:2, g:2, T:3, t:3};
        for (var i = 0; i < s.length; i += 4) {
          var b = 0;
          for (var j = 0; j < 4 && i+j < s.length; j++) {
            b |= ((map[s[i+j]] || 0) << (6 - j*2));
          }
          bytes.push(b);
        }
        var bin = String.fromCharCode.apply(null, bytes);
        records.push([header, btoa(bin), s.length]);
      }
    }
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (line.charAt(0) === ">") {
        flush();
        header = line.substring(1);
        seq = [];
      } else { seq.push(line); }
    }
    flush();
    return btoa(unescape(encodeURIComponent(JSON.stringify(records))));
  }

  function decompressDNA2bit(encoded) {
    var json = decodeURIComponent(escape(atob(encoded)));
    var records = JSON.parse(json);
    var bases = "ACGT";
    var lines = [];
    for (var r = 0; r < records.length; r++) {
      lines.push(">" + records[r][0]);
      var bin = atob(records[r][1]);
      var len = records[r][2];
      var seq = "";
      for (var i = 0; i < bin.length; i++) {
        var b = bin.charCodeAt(i);
        for (var j = 0; j < 4 && seq.length < len; j++) {
          seq += bases[(b >> (6 - j*2)) & 3];
        }
      }
      for (var i = 0; i < seq.length; i += 80) lines.push(seq.substring(i, i + 80));
    }
    return lines.join("\n");
  }

  function showSeqPopup(x, y, seq) {
    closeSeqPopup();
    if (!seq || seq.length < 2) return;
    var clean = seq.replace(/\s/g, "");
    if (!/^[ATCGNatcgn]+$/.test(clean)) return; // not a DNA sequence

    seqPopup = document.createElement("div");
    seqPopup.className = "vw-seq-popup";
    var rows = [
      { label: "Length", value: clean.length + " bp" },
      { label: "GC%", value: calcGC(clean) },
      { label: "Rev Comp", value: clean.length <= 200 ? reverseComplement(clean) : reverseComplement(clean).substring(0, 200) + "..." },
    ];
    if (clean.length >= 3 && clean.length <= 3000) {
      rows.push({ label: "Protein", value: translateDNA(clean) });
    }
    seqPopup.innerHTML = rows.map(function(r) {
      return '<div class="vw-seq-popup-row"><span class="vw-seq-popup-label">' + r.label + '</span><span class="vw-seq-popup-value">' + escapeHtml(r.value) + '</span><button class="vw-seq-popup-copy" title="Copy" data-val="' + escapeHtml(r.value) + '">&#128203;</button></div>';
    }).join("");

    // Position
    seqPopup.style.left = Math.min(x, window.innerWidth - 440) + "px";
    seqPopup.style.top = Math.min(y + 10, window.innerHeight - 200) + "px";
    document.body.appendChild(seqPopup);

    seqPopup.querySelectorAll(".vw-seq-popup-copy").forEach(function(btn) {
      btn.addEventListener("click", function() {
        navigator.clipboard.writeText(this.getAttribute("data-val")).then(function() {
          btn.textContent = "\u2713";
          setTimeout(function() { btn.innerHTML = "&#128203;"; }, 1500);
        });
      });
    });
  }

  function closeSeqPopup() {
    if (seqPopup) { seqPopup.remove(); seqPopup = null; }
  }

  document.addEventListener("mouseup", function(e) {
    var sel = window.getSelection();
    var text = sel ? sel.toString().trim() : "";
    if (text.length >= 3 && /^[ATCGNatcgn\s]+$/.test(text)) {
      showSeqPopup(e.clientX, e.clientY, text);
    }
  });

  document.addEventListener("mousedown", function(e) {
    if (seqPopup && !seqPopup.contains(e.target)) closeSeqPopup();
  });

  // ── Feature: Column Histogram Sidebar (3) ────────────────────
  var colSidebar = document.getElementById("vw-col-sidebar");
  var colSidebarClose = document.getElementById("vw-col-sidebar-close");

  function showColumnSidebar(f, colIdx) {
    if (!colSidebar) return;
    var colName = f.parsed.columns[colIdx];
    document.getElementById("vw-col-sidebar-title").textContent = colName;
    var body = document.getElementById("vw-col-sidebar-body");
    body.innerHTML = "";

    var values = f.parsed.rows.map(function(r) { return r[colIdx]; });
    var nonNull = values.filter(function(v) { return v != null && v !== "" && v !== "."; });
    var nums = nonNull.map(Number).filter(function(n) { return !isNaN(n); });
    var isNumeric = nums.length > nonNull.length * 0.7;

    // Basic stats
    var statsHtml = '<div style="margin-bottom:12px">';
    statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Total rows</span><span class="vw-col-stat-value">' + values.length + '</span></div>';
    statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Non-empty</span><span class="vw-col-stat-value">' + nonNull.length + '</span></div>';
    statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Empty/null</span><span class="vw-col-stat-value">' + (values.length - nonNull.length) + '</span></div>';

    if (isNumeric && nums.length > 0) {
      nums.sort(function(a, b) { return a - b; });
      var sum = nums.reduce(function(a, b) { return a + b; }, 0);
      var mean = sum / nums.length;
      var median = nums.length % 2 === 0 ? (nums[nums.length / 2 - 1] + nums[nums.length / 2]) / 2 : nums[Math.floor(nums.length / 2)];
      var variance = nums.reduce(function(s, v) { return s + (v - mean) * (v - mean); }, 0) / nums.length;
      statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Min</span><span class="vw-col-stat-value">' + nums[0] + '</span></div>';
      statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Max</span><span class="vw-col-stat-value">' + nums[nums.length - 1] + '</span></div>';
      statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Mean</span><span class="vw-col-stat-value">' + mean.toFixed(2) + '</span></div>';
      statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Median</span><span class="vw-col-stat-value">' + median + '</span></div>';
      statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Std Dev</span><span class="vw-col-stat-value">' + Math.sqrt(variance).toFixed(2) + '</span></div>';
      statsHtml += '</div>';

      // Histogram SVG
      var bins = 20;
      var min = nums[0], max = nums[nums.length - 1];
      var range = max - min || 1;
      var binWidth = range / bins;
      var counts = new Array(bins).fill(0);
      nums.forEach(function(v) {
        var bi = Math.min(Math.floor((v - min) / binWidth), bins - 1);
        counts[bi]++;
      });
      var maxCount = Math.max.apply(null, counts);
      var svgW = 280, svgH = 120, barW = svgW / bins;
      var svg = '<svg width="' + svgW + '" height="' + svgH + '" xmlns="http://www.w3.org/2000/svg">';
      counts.forEach(function(c, i) {
        var h = maxCount > 0 ? (c / maxCount) * (svgH - 20) : 0;
        svg += '<rect x="' + (i * barW) + '" y="' + (svgH - 16 - h) + '" width="' + (barW - 1) + '" height="' + h + '" fill="#7c3aed" opacity="0.7" rx="1"/>';
      });
      svg += '<text x="0" y="' + (svgH - 2) + '" fill="#64748b" font-size="9" font-family="system-ui">' + min.toFixed(1) + '</text>';
      svg += '<text x="' + (svgW - 40) + '" y="' + (svgH - 2) + '" fill="#64748b" font-size="9" font-family="system-ui">' + max.toFixed(1) + '</text>';
      svg += '</svg>';
      body.innerHTML = statsHtml + svg;
    } else {
      // Categorical: top values
      var freq = {};
      nonNull.forEach(function(v) { freq[String(v)] = (freq[String(v)] || 0) + 1; });
      var unique = Object.keys(freq);
      statsHtml += '<div class="vw-col-stat-row"><span class="vw-col-stat-label">Unique</span><span class="vw-col-stat-value">' + unique.length + '</span></div>';
      statsHtml += '</div>';

      // Top 15 values
      unique.sort(function(a, b) { return freq[b] - freq[a]; });
      var topHtml = '<div style="margin-top:8px"><div style="font-family:var(--vw-sans);font-size:10px;color:var(--vw-text-dim);text-transform:uppercase;font-weight:600;margin-bottom:4px">Top Values</div>';
      unique.slice(0, 15).forEach(function(v) {
        var pct = (freq[v] / values.length * 100).toFixed(1);
        topHtml += '<div class="vw-col-top-val"><span>' + escapeHtml(v) + '</span><span>' + freq[v] + ' (' + pct + '%)</span></div>';
      });
      if (unique.length > 15) topHtml += '<div style="color:var(--vw-text-muted);font-size:10px;padding-top:4px">...and ' + (unique.length - 15) + ' more</div>';
      topHtml += '</div>';
      body.innerHTML = statsHtml + topHtml;
    }

    colSidebar.classList.add("open");
  }

  if (colSidebarClose) {
    colSidebarClose.addEventListener("click", function() { colSidebar.classList.remove("open"); });
  }
  document.addEventListener("keydown", function(e) {
    if (e.key === "Escape" && colSidebar && colSidebar.classList.contains("open")) colSidebar.classList.remove("open");
  });

  // ── Feature: Format Conversion Export (4) ────────────────────
  function convertFormat(f, targetFormat) {
    var rows = f.parsed.rows;
    var cols = f.parsed.columns;
    var lines = [];

    if (targetFormat === "bed" && (f.parsed.format === "vcf" || f.parsed.format === "gff")) {
      // VCF/GFF → BED: extract chrom, start, end
      rows.forEach(function(r) {
        if (f.parsed.format === "vcf") {
          var chrom = r[cols.indexOf("CHROM")] || r[0];
          var pos = parseInt(r[cols.indexOf("POS")] || r[1]) - 1;
          var ref = r[cols.indexOf("REF")] || r[3] || "";
          lines.push(chrom + "\t" + pos + "\t" + (pos + ref.length));
        } else if (f.parsed.format === "gff") {
          lines.push(r[0] + "\t" + (parseInt(r[3]) - 1) + "\t" + r[4]);
        }
      });
    } else if (targetFormat === "csv") {
      lines.push(cols.join(","));
      rows.forEach(function(r) {
        lines.push(r.map(function(v) {
          var s = String(v == null ? "" : v);
          return s.indexOf(",") >= 0 || s.indexOf('"') >= 0 ? '"' + s.replace(/"/g, '""') + '"' : s;
        }).join(","));
      });
    } else if (targetFormat === "tsv") {
      lines.push(cols.join("\t"));
      rows.forEach(function(r) { lines.push(r.join("\t")); });
    } else if (targetFormat === "fasta-rc" && f.parsed.format === "fasta") {
      rows.forEach(function(r) {
        lines.push(">" + r[0] + " reverse_complement");
        lines.push(reverseComplement(r[1] || ""));
      });
    }

    if (lines.length === 0) return null;
    return lines.join("\n");
  }

  // ── Feature: Transpose Mode ──────────────────────────────────
  var transposeMode = false;

  // ── Feature: Column Search (Ctrl+Shift+F) ──────────────────
  function showColumnSearch() {
    var existing = document.getElementById("vw-colsearch");
    if (existing) { existing.remove(); return; }
    var f = files[activeTab];
    if (!f) return;

    var overlay = document.createElement("div");
    overlay.id = "vw-colsearch";
    overlay.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:16px;box-shadow:0 12px 40px rgba(0,0,0,0.5);width:320px;font-family:var(--vw-sans);";

    var title = document.createElement("div");
    title.style.cssText = "font-weight:600;color:var(--vw-accent);margin-bottom:8px;font-size:13px;";
    title.textContent = "Find Column (Ctrl+Shift+F)";
    overlay.appendChild(title);

    var input = document.createElement("input");
    input.type = "text";
    input.placeholder = "Type column name...";
    input.style.cssText = "width:100%;background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:6px;padding:6px 10px;color:var(--vw-text);font-size:13px;margin-bottom:8px;box-sizing:border-box;outline:none;";
    overlay.appendChild(input);

    var results = document.createElement("div");
    results.style.cssText = "max-height:200px;overflow-y:auto;";
    overlay.appendChild(results);

    function updateResults() {
      var term = input.value.toLowerCase();
      results.innerHTML = "";
      f.parsed.columns.forEach(function(col, ci) {
        if (term && col.toLowerCase().indexOf(term) === -1) return;
        if (hiddenCols[activeTab] && hiddenCols[activeTab].has(ci)) return;
        var div = document.createElement("div");
        div.style.cssText = "padding:4px 8px;cursor:pointer;border-radius:4px;font-size:12px;color:var(--vw-text);display:flex;justify-content:space-between;";
        div.innerHTML = '<span>' + escapeHtml(col) + '</span><span style="color:var(--vw-text-muted);font-size:10px">' + (f.parsed.colTypes[ci] || "text") + '</span>';
        div.addEventListener("mouseenter", function() { div.style.background = "var(--vw-row-hover)"; });
        div.addEventListener("mouseleave", function() { div.style.background = ""; });
        div.addEventListener("click", function() {
          overlay.remove();
          // Scroll to column in table
          var th = document.querySelectorAll(".vw-table thead tr:first-child th")[ci + 1 + (bookmarkMode ? 1 : 0)];
          if (th) th.scrollIntoView({ behavior: "smooth", block: "nearest", inline: "center" });
          sortCol = ci; sortAsc = true; currentPage = 0;
          tabSortState[activeTab] = { col: sortCol, asc: sortAsc };
          renderView();
        });
        results.appendChild(div);
      });
    }
    input.addEventListener("input", updateResults);
    input.addEventListener("keydown", function(e) {
      if (e.key === "Escape") overlay.remove();
      if (e.key === "Enter") {
        var first = results.querySelector("div");
        if (first) first.click();
      }
    });
    document.body.appendChild(overlay);
    updateResults();
    input.focus();
  }

  // ── Feature: File Merge ────────────────────────────────────
  function mergeFiles() {
    var f = files[activeTab];
    if (!f) return;
    var sameFormat = files.filter(function(other) { return other.parsed.format === f.parsed.format && other !== f; });
    if (sameFormat.length === 0) return;

    var mergedRows = f.parsed.rows.slice();
    sameFormat.forEach(function(other) {
      other.parsed.rows.forEach(function(r) { mergedRows.push(r); });
    });

    var mergedText = f.parsed.columns.join("\t") + "\n" + mergedRows.map(function(r) { return r.join("\t"); }).join("\n");
    var mergedName = "merged_" + f.parsed.format + "_" + (sameFormat.length + 1) + "files." + f.parsed.format;
    addFile(mergedName, mergedText.length, mergedText, false);
  }

  // ── Feature: Quick Chart Popup ──────────────────────────────
  function showQuickChart(colName, values) {
    var existing = document.getElementById("vw-quick-chart");
    if (existing) existing.remove();
    if (!values.length) return;

    var popup = document.createElement("div");
    popup.id = "vw-quick-chart";
    popup.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:16px;box-shadow:0 12px 40px rgba(0,0,0,0.5);font-family:var(--vw-sans);";

    var header = document.createElement("div");
    header.style.cssText = "display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;";
    header.innerHTML = '<span style="font-weight:600;color:var(--vw-accent);font-size:13px">' + escapeHtml(colName) + '</span>';
    var closeBtn = document.createElement("button");
    closeBtn.textContent = "\u00d7";
    closeBtn.style.cssText = "background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:18px;padding:0 4px;";
    closeBtn.addEventListener("click", function() { popup.remove(); });
    header.appendChild(closeBtn);
    popup.appendChild(header);

    // Stats line
    values.sort(function(a, b) { return a - b; });
    var sum = values.reduce(function(a, b) { return a + b; }, 0);
    var mean = sum / values.length;
    var med = values.length % 2 === 0 ? (values[values.length / 2 - 1] + values[values.length / 2]) / 2 : values[Math.floor(values.length / 2)];
    var statsLine = document.createElement("div");
    statsLine.style.cssText = "font-size:11px;color:var(--vw-text-dim);margin-bottom:8px;font-family:var(--vw-mono);";
    var fmt = function(n) { return n.toLocaleString(undefined, {maximumFractionDigits:2}); };
    statsLine.textContent = "n=" + values.length + "  min=" + fmt(values[0]) + "  max=" + fmt(values[values.length - 1]) + "  mean=" + fmt(mean) + "  median=" + fmt(med);
    popup.appendChild(statsLine);

    // Histogram SVG
    popup.appendChild(svgHistogram(values, { width: 480, height: 180, color: "#7c3aed", label: colName }));

    document.body.appendChild(popup);
    document.addEventListener("keydown", function handler(e) {
      if (e.key === "Escape") { popup.remove(); document.removeEventListener("keydown", handler); }
    });
  }

  // ── Quick Histogram (right-click column header) ──────────────
  function isNumericColumn(f, colIndex) {
    return f.parsed.colTypes[colIndex] === "num";
  }

  function showQuickHistogram(f, colIndex) {
    var colName = f.parsed.columns[colIndex];
    var vals = [];
    for (var ri = 0; ri < f.parsed.rows.length; ri++) {
      var v = f.parsed.rows[ri][colIndex];
      if (typeof v === "number" && !isNaN(v)) vals.push(v);
    }
    if (!vals.length) return;
    var existing = document.getElementById("vw-quick-histogram");
    if (existing) existing.remove();

    vals.sort(function(a, b) { return a - b; });
    var n = vals.length;
    var sum = 0; for (var i = 0; i < n; i++) sum += vals[i];
    var mean = sum / n;
    var med = n % 2 === 0 ? (vals[n / 2 - 1] + vals[n / 2]) / 2 : vals[Math.floor(n / 2)];
    var sqSum = 0; for (var i = 0; i < n; i++) sqSum += (vals[i] - mean) * (vals[i] - mean);
    var stdev = Math.sqrt(sqSum / n);
    var fmt = function(x) { return x.toLocaleString(undefined, {maximumFractionDigits: 2}); };

    var popup = document.createElement("div");
    popup.id = "vw-quick-histogram";
    popup.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:16px;box-shadow:0 12px 40px rgba(0,0,0,0.5);font-family:var(--vw-sans);";

    var header = document.createElement("div");
    header.style.cssText = "display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;";
    header.innerHTML = '<span style="font-weight:600;color:var(--vw-accent);font-size:13px">Histogram: ' + escapeHtml(colName) + '</span>';
    var closeBtn = document.createElement("button");
    closeBtn.textContent = "\u00d7";
    closeBtn.style.cssText = "background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:18px;padding:0 4px;";
    closeBtn.addEventListener("click", function() { popup.remove(); });
    header.appendChild(closeBtn);
    popup.appendChild(header);

    popup.appendChild(svgHistogram(vals, { width: 480, height: 180, color: "#7c3aed", label: colName }));

    var stats = document.createElement("div");
    stats.style.cssText = "font-size:11px;color:var(--vw-text-dim);margin-top:8px;font-family:var(--vw-mono);line-height:1.6;";
    stats.innerHTML = "n=" + n + "  min=" + fmt(vals[0]) + "  max=" + fmt(vals[n - 1]) +
      "<br>mean=" + fmt(mean) + "  median=" + fmt(med) + "  stdev=" + fmt(stdev);
    popup.appendChild(stats);

    document.body.appendChild(popup);
    document.addEventListener("keydown", function handler(e) {
      if (e.key === "Escape") { popup.remove(); document.removeEventListener("keydown", handler); }
    });
  }

  // ── Feature: Computed Columns ────────────────────────────────
  function addComputedColumn(f, type, sourceCol) {
    var colName, computeFn;
    if (type === "gc") {
      colName = "GC%";
      computeFn = function(row) {
        var seq = String(row[sourceCol] || "").toUpperCase();
        if (!seq.length) return 0;
        var gc = 0;
        for (var i = 0; i < seq.length; i++) { if (seq[i] === "G" || seq[i] === "C") gc++; }
        return Math.round(gc / seq.length * 10000) / 100;
      };
    } else if (type === "length") {
      colName = f.parsed.columns[sourceCol] + "_len";
      computeFn = function(row) { return String(row[sourceCol] || "").length; };
    } else if (type === "upper") {
      colName = f.parsed.columns[sourceCol] + "_upper";
      computeFn = function(row) { return String(row[sourceCol] || "").toUpperCase(); };
    } else if (type === "log2") {
      colName = "log2(" + f.parsed.columns[sourceCol] + ")";
      computeFn = function(row) {
        var v = parseFloat(row[sourceCol]);
        return isNaN(v) || v <= 0 ? 0 : Math.round(Math.log2(v) * 100) / 100;
      };
    } else {
      return;
    }

    f.parsed.columns.push(colName);
    f.parsed.colTypes.push(type === "upper" ? "text" : "num");
    f.parsed.rows.forEach(function(row) { row.push(computeFn(row)); });
    // Clear caches
    f.parsed._colHints = null;
    f.parsed._anomalyCache = null;
    f.parsed._summaryCache = null;
    _cachedFile = null;
    _filterCache = null;
    renderView();
  }

  // ── Feature: Conditional Highlighting ──────────────────────
  function showHighlightDialog() {
    var existing = document.getElementById("vw-highlight-dialog");
    if (existing) { existing.remove(); return; }
    var f = files[activeTab];
    if (!f) return;

    var dlg = document.createElement("div");
    dlg.id = "vw-highlight-dialog";
    dlg.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:16px;box-shadow:0 12px 40px rgba(0,0,0,0.5);width:320px;font-family:var(--vw-sans);";

    dlg.innerHTML = '<div style="font-weight:600;color:var(--vw-accent);margin-bottom:10px;font-size:13px">Conditional Highlight</div>';

    // Column select
    var colSel = document.createElement("select");
    colSel.style.cssText = "width:100%;background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:6px;padding:5px;color:var(--vw-text);font-size:12px;margin-bottom:6px;";
    f.parsed.columns.forEach(function(col, ci) {
      var opt = document.createElement("option");
      opt.value = ci;
      opt.textContent = col;
      colSel.appendChild(opt);
    });
    dlg.appendChild(colSel);

    // Operator select
    var opSel = document.createElement("select");
    opSel.style.cssText = "width:100%;background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:6px;padding:5px;color:var(--vw-text);font-size:12px;margin-bottom:6px;";
    [">", ">=", "<", "<=", "=", "!=", "contains"].forEach(function(op) {
      var opt = document.createElement("option");
      opt.value = op; opt.textContent = op;
      opSel.appendChild(opt);
    });
    dlg.appendChild(opSel);

    // Value input
    var valInput = document.createElement("input");
    valInput.type = "text";
    valInput.placeholder = "Value...";
    valInput.style.cssText = "width:100%;background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:6px;padding:5px;color:var(--vw-text);font-size:12px;margin-bottom:10px;box-sizing:border-box;";
    dlg.appendChild(valInput);

    var btnRow = document.createElement("div");
    btnRow.style.cssText = "display:flex;gap:6px;justify-content:flex-end;";

    var clearBtn = document.createElement("button");
    clearBtn.textContent = "Clear";
    clearBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:6px;padding:4px 12px;color:var(--vw-text-dim);cursor:pointer;font-size:12px;";
    clearBtn.addEventListener("click", function() { highlightRule = null; dlg.remove(); renderView(); });
    btnRow.appendChild(clearBtn);

    var applyBtn = document.createElement("button");
    applyBtn.textContent = "Apply";
    applyBtn.style.cssText = "background:var(--vw-accent);border:none;border-radius:6px;padding:4px 12px;color:white;cursor:pointer;font-size:12px;font-weight:600;";
    applyBtn.addEventListener("click", function() {
      highlightRule = { col: parseInt(colSel.value), op: opSel.value, value: valInput.value };
      dlg.remove();
      renderView();
    });
    btnRow.appendChild(applyBtn);
    dlg.appendChild(btnRow);

    document.body.appendChild(dlg);
    valInput.focus();
    valInput.addEventListener("keydown", function(e) { if (e.key === "Escape") dlg.remove(); if (e.key === "Enter") applyBtn.click(); });
  }

  function matchesHighlightRule(row) {
    if (!highlightRule) return false;
    var val = row[highlightRule.col];
    var cmp = highlightRule.value;
    var op = highlightRule.op;
    if (op === "contains") return String(val).toLowerCase().indexOf(cmp.toLowerCase()) !== -1;
    var numVal = parseFloat(val), numCmp = parseFloat(cmp);
    if (op === "=" || op === "==") return String(val) === cmp;
    if (op === "!=") return String(val) !== cmp;
    if (isNaN(numVal) || isNaN(numCmp)) return false;
    if (op === ">") return numVal > numCmp;
    if (op === ">=") return numVal >= numCmp;
    if (op === "<") return numVal < numCmp;
    if (op === "<=") return numVal <= numCmp;
    return false;
  }

  // ── Feature: Auto-detect delimiter ─────────────────────────
  function detectDelimiter(text) {
    var lines = text.split("\n").slice(0, 5).filter(function(l) { return l.trim().length > 0 && l[0] !== "#"; });
    if (lines.length === 0) return "\t";
    var delims = ["\t", ",", ";", "|"];
    var best = "\t", bestScore = 0;
    delims.forEach(function(d) {
      var counts = lines.map(function(l) { return l.split(d).length - 1; });
      var allSame = counts.every(function(c) { return c === counts[0] && c > 0; });
      if (allSame && counts[0] > bestScore) { bestScore = counts[0]; best = d; }
    });
    return best;
  }

  // ── Feature: Undo Stack ────────────────────────────────────
  function pushUndo() {
    undoStack.push({
      sortCol: sortCol, sortAsc: sortAsc, currentPage: currentPage,
      sortCols: sortCols.slice(),
      pinnedCols: Array.from(pinnedCols),
      groupByCol: groupByCol, searchTerm: searchTerm, transposeMode: transposeMode,
      pageSize: pageSize,
      colFilters: JSON.parse(JSON.stringify(Object.keys(colFilters).reduce(function(acc, k) { acc[k] = Array.from(colFilters[k]); return acc; }, {}))),
      hiddenCols: hiddenCols[activeTab] ? Array.from(hiddenCols[activeTab]) : [],
      highlightRule: highlightRule ? JSON.parse(JSON.stringify(highlightRule)) : null
    });
    if (undoStack.length > 20) undoStack.shift();
  }

  function popUndo() {
    if (undoStack.length === 0) return;
    var state = undoStack.pop();
    sortCol = state.sortCol;
    sortAsc = state.sortAsc;
    sortCols = state.sortCols || [];
    pinnedCols = new Set(state.pinnedCols || []);
    currentPage = state.currentPage;
    if (state.pageSize) pageSize = state.pageSize;
    groupByCol = state.groupByCol;
    searchTerm = state.searchTerm;
    transposeMode = state.transposeMode;
    highlightRule = state.highlightRule;
    // Restore colFilters
    colFilters = {};
    Object.keys(state.colFilters).forEach(function(k) { colFilters[k] = new Set(state.colFilters[k]); });
    // Restore hiddenCols
    hiddenCols[activeTab] = new Set(state.hiddenCols);
    searchInput.value = searchTerm;
    renderView();
  }

  // ── Feature: Split View (5) ──────────────────────────────────
  var splitMode = false;

  // ── Feature: BLAST Search (6) ────────────────────────────────
  function openBLAST(seq) {
    var clean = seq.replace(/\s/g, "").substring(0, 8000);
    var url = "https://blast.ncbi.nlm.nih.gov/Blast.cgi?PROGRAM=blastn&DATABASE=nt&QUERY=" + encodeURIComponent(clean);
    window.open(url, "_blank");
  }

  // ── Feature: Anomaly Detection (7) ───────────────────────────
  function detectAnomalies(f) {
    var anomalies = []; // [{rowIdx, message, severity}]
    var fmt = f.parsed.format;
    var rows = f.parsed.rows;

    if (fmt === "fasta") {
      // Detect duplicate IDs
      var ids = {};
      rows.forEach(function(r, i) {
        var id = r[0];
        if (ids[id] !== undefined) anomalies.push({ row: i, msg: "Duplicate ID: " + id, sev: "warn" });
        else ids[id] = i;
      });
      // Detect unusually short/long sequences
      var lens = rows.map(function(r) { return (r[1] || "").length; });
      if (lens.length > 5) {
        lens.sort(function(a, b) { return a - b; });
        var q1 = lens[Math.floor(lens.length * 0.25)];
        var q3 = lens[Math.floor(lens.length * 0.75)];
        var iqr = q3 - q1;
        var lo = q1 - 1.5 * iqr, hi = q3 + 1.5 * iqr;
        rows.forEach(function(r, i) {
          var l = (r[1] || "").length;
          if (l < lo && l > 0) anomalies.push({ row: i, msg: "Unusually short: " + l + " bp", sev: "warn" });
          if (l > hi) anomalies.push({ row: i, msg: "Unusually long: " + l + " bp", sev: "warn" });
        });
      }
    } else if (fmt === "bed") {
      rows.forEach(function(r, i) {
        var start = parseInt(r[1]), end = parseInt(r[2]);
        if (start > end) anomalies.push({ row: i, msg: "Start > End: " + start + " > " + end, sev: "err" });
        if (start < 0) anomalies.push({ row: i, msg: "Negative coordinate: " + start, sev: "err" });
      });
    } else if (fmt === "vcf") {
      var seen = {};
      rows.forEach(function(r, i) {
        var key = r[0] + ":" + r[1] + ":" + r[3] + ">" + r[4];
        if (seen[key]) anomalies.push({ row: i, msg: "Duplicate variant: " + key, sev: "warn" });
        seen[key] = true;
        if (r[5] !== undefined && r[5] !== "." && parseFloat(r[5]) < 20) anomalies.push({ row: i, msg: "Low QUAL: " + r[5], sev: "warn" });
      });
    } else if (fmt === "fastq") {
      rows.forEach(function(r, i) {
        if (r[1] && r[3] && r[1].length !== r[3].length) anomalies.push({ row: i, msg: "Seq/qual length mismatch", sev: "err" });
      });
    }

    return anomalies;
  }

  // ── Feature: File Validation Report (8) ──────────────────────
  function validateFile(f) {
    var issues = []; // [{msg, severity: "warn"|"err"}]
    var fmt = f.parsed.format;
    var rows = f.parsed.rows;

    if (fmt === "vcf") {
      var rawText = f.text || f.rawPreview || "";
      if (rawText.indexOf("##fileformat=VCF") === -1) issues.push({ msg: "Missing ##fileformat=VCF header line", sev: "warn" });
      if (f.parsed.columns.indexOf("CHROM") === -1) issues.push({ msg: "Missing standard VCF column headers", sev: "err" });
    } else if (fmt === "bed") {
      rows.forEach(function(r, i) {
        if (i > 10) return; // sample first 10
        if (r.length < 3) issues.push({ msg: "Row " + (i + 1) + ": fewer than 3 columns", sev: "err" });
      });
    } else if (fmt === "fasta") {
      if (rows.length === 0) issues.push({ msg: "No sequences found", sev: "err" });
      rows.forEach(function(r, i) {
        if (i > 20) return;
        if (!r[0] || r[0].trim() === "") issues.push({ msg: "Row " + (i + 1) + ": empty sequence ID", sev: "warn" });
      });
    } else if (fmt === "gff") {
      rows.forEach(function(r, i) {
        if (i > 10) return;
        if (r.length < 9) issues.push({ msg: "Row " + (i + 1) + ": fewer than 9 columns (GFF requires 9)", sev: "warn" });
      });
    }

    // Add anomaly count
    var anomalies = detectAnomalies(f);
    if (anomalies.length > 0) issues.push({ msg: anomalies.length + " data anomalies detected (duplicate IDs, outliers, etc.)", sev: "warn" });

    return issues;
  }

  function showValidationPanel(f) {
    closeValidationPanel();
    var issues = validateFile(f);
    if (issues.length === 0) return;

    var panel = document.createElement("div");
    panel.className = "vw-validation-panel";
    panel.id = "vw-validation-panel";
    panel.innerHTML =
      '<div class="vw-validation-header"><span>&#9888; ' + issues.length + ' validation issue' + (issues.length > 1 ? 's' : '') + '</span><button style="background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:16px">&times;</button></div>' +
      '<div class="vw-validation-body">' + issues.map(function(iss) {
        return '<div class="vw-validation-item"><span class="v-' + iss.sev + '">' + (iss.sev === "err" ? "&#10006;" : "&#9888;") + '</span> ' + escapeHtml(iss.msg) + '</div>';
      }).join("") + '</div>';
    document.body.appendChild(panel);
    panel.querySelector(".vw-validation-header button").addEventListener("click", function() { panel.remove(); });
  }

  function closeValidationPanel() {
    var p = document.getElementById("vw-validation-panel");
    if (p) p.remove();
  }

  // ── Feature: Screenshot (9) ──────────────────────────────────
  function screenshotView() {
    var f = files[activeTab];
    if (!f) return;
    var isDark = document.documentElement.classList.contains("dark");
    screenshotCanvas(f, isDark);
  }

  function drawTitle(ctx, f, isDark, canvasW) {
    ctx.fillStyle = isDark ? "#e2e8f0" : "#1e293b";
    ctx.font = "bold 16px system-ui";
    ctx.fillText("BLViewer — " + f.name, 20, 28);
    ctx.font = "12px monospace";
    ctx.fillStyle = isDark ? "#64748b" : "#94a3b8";
    var info = f.parsed.format.toUpperCase() + " | " + (f.parsed.rows ? f.parsed.rows.length.toLocaleString() : 0) + " rows | " + formatBytes(f.size);
    if (f.truncated) info += " (partial — first 50 MB)";
    ctx.fillText(info, 20, 48);
  }

  function screenshotCanvas(f, isDark) {
    var scale = 2;
    var padX = 10;
    var padY = 4;
    var rowH = 18;
    var headerH = 22;
    var charW = 7.2; // monospace char width at 11px
    var titleH = 60;

    var cols = f.parsed.columns;
    var colTypes = f.parsed.colTypes;

    // Nucleotide colors for screenshot
    var ntColors = {
      A: isDark ? "#34d399" : "#059669", // green
      T: isDark ? "#f87171" : "#dc2626", // red
      C: isDark ? "#60a5fa" : "#2563eb", // blue
      G: isDark ? "#fbbf24" : "#d97706", // amber
      U: isDark ? "#f87171" : "#dc2626", // red (like T)
      N: isDark ? "#94a3b8" : "#64748b"  // gray
    };

    // Determine visible rows (current page)
    var rows = getFilteredRows(f);
    var maxRows = Math.min(rows.length, pageSize, 100); // cap at current page size
    var pageStart = currentPage * pageSize;
    var displayRows = rows.slice(pageStart, pageStart + maxRows);

    if (currentView === "stats" && f.parsed.stats) {
      // Stats view screenshot — use simple layout
      var statsKeys = Object.keys(f.parsed.stats);
      var canvasH = titleH + statsKeys.length * 22 + 40;
      var canvas = document.createElement("canvas");
      canvas.width = 800 * scale;
      canvas.height = canvasH * scale;
      var ctx = canvas.getContext("2d");
      ctx.fillStyle = isDark ? "#0a0e1a" : "#f8fafc";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.scale(scale, scale);
      drawTitle(ctx, f, isDark, 800);
      var y = titleH + 10;
      ctx.font = "12px monospace";
      statsKeys.forEach(function(key) {
        ctx.fillStyle = isDark ? "#a78bfa" : "#7c3aed";
        ctx.fillText(key, 20, y);
        ctx.fillStyle = isDark ? "#e2e8f0" : "#1e293b";
        ctx.fillText(String(f.parsed.stats[key]), 200, y);
        y += 22;
      });
      ctx.fillStyle = isDark ? "#1e293b" : "#e2e8f0";
      ctx.font = "9px system-ui";
      ctx.fillText("BLViewer — lang.bio", 800 - 120, canvasH - 8);
    } else if (f.parsed.rows) {
      // Table view screenshot — match actual column widths from DOM
      var maxCharPerCol = 80; // allow up to 80 chars per column

      // Measure column widths from actual data (sample more rows, allow wider)
      var tmpCanvas = document.createElement("canvas");
      var tmpCtx = tmpCanvas.getContext("2d");
      tmpCtx.font = "11px monospace";

      var colWidths = cols.map(function(c, ci) {
        // Start with header width
        var maxW = tmpCtx.measureText(c).width + padX * 2;
        // Sample visible rows
        for (var ri = 0; ri < displayRows.length; ri++) {
          var cellText = String(displayRows[ri].row[ci] || "");
          var truncText = cellText.substring(0, maxCharPerCol);
          var w = tmpCtx.measureText(truncText).width + padX * 2;
          if (w > maxW) maxW = w;
        }
        // Cap per column but much wider than before
        return Math.min(maxW, 500);
      });

      var totalW = colWidths.reduce(function(a, b) { return a + b; }, 0) + 24;
      var canvasW = Math.max(totalW, 600);
      var canvasH = titleH + headerH + displayRows.length * rowH + 40;

      var canvas = document.createElement("canvas");
      canvas.width = Math.ceil(canvasW) * scale;
      canvas.height = Math.ceil(canvasH) * scale;
      var ctx = canvas.getContext("2d");
      ctx.fillStyle = isDark ? "#0a0e1a" : "#f8fafc";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.scale(scale, scale);

      drawTitle(ctx, f, isDark, canvasW);

      var y = titleH;

      // Header background
      ctx.fillStyle = isDark ? "#151b2b" : "#f1f5f9";
      ctx.fillRect(12, y, totalW - 24, headerH);

      // Header text
      ctx.font = "bold 11px monospace";
      var x = 12;
      cols.forEach(function(c, ci) {
        ctx.fillStyle = isDark ? "#a78bfa" : "#7c3aed";
        ctx.save();
        ctx.beginPath();
        ctx.rect(x, y, colWidths[ci], headerH);
        ctx.clip();
        ctx.fillText(c, x + padX, y + 15);
        ctx.restore();
        x += colWidths[ci];
      });
      y += headerH;

      // Header bottom border
      ctx.fillStyle = isDark ? "#334155" : "#cbd5e1";
      ctx.fillRect(12, y - 1, totalW - 24, 1.5);

      // Data rows
      ctx.font = "11px monospace";
      for (var ri = 0; ri < displayRows.length; ri++) {
        // Alternating row bg
        if (ri % 2 === 0) {
          ctx.fillStyle = isDark ? "rgba(15,23,42,0.4)" : "rgba(241,245,249,0.5)";
          ctx.fillRect(12, y, totalW - 24, rowH);
        }

        x = 12;
        var row = displayRows[ri].row;
        for (var ci = 0; ci < cols.length; ci++) {
          var rawVal = row[ci];
          var val = String(rawVal || "");
          var type = colTypes[ci];
          var colName = cols[ci];

          ctx.save();
          ctx.beginPath();
          ctx.rect(x, y, colWidths[ci], rowH);
          ctx.clip();

          // Sequence columns: per-base ATCG coloring
          if (type === "seq" && val.length > 0) {
            var cx = x + padX;
            for (var si = 0; si < Math.min(val.length, maxCharPerCol); si++) {
              var base = val.charAt(si).toUpperCase();
              ctx.fillStyle = ntColors[base] || (isDark ? "#94a3b8" : "#64748b");
              ctx.fillText(val.charAt(si), cx, y + 13);
              cx += charW;
            }
            if (val.length > maxCharPerCol) {
              ctx.fillStyle = isDark ? "#475569" : "#94a3b8";
              ctx.fillText("...", cx, y + 13);
            }
          } else {
            // Determine text color
            if (type === "num") {
              ctx.fillStyle = isDark ? "#22d3ee" : "#0891b2";
            } else {
              ctx.fillStyle = isDark ? "#e2e8f0" : "#1e293b";
            }
            // Special column coloring
            if (colName === "FILTER") {
              ctx.fillStyle = val === "PASS" ? (isDark ? "#34d399" : "#059669") : (isDark ? "#f87171" : "#dc2626");
            } else if (colName === "QUAL") {
              var q = parseFloat(val);
              ctx.fillStyle = q >= 100 ? (isDark ? "#34d399" : "#059669") :
                              q >= 30 ? (isDark ? "#fbbf24" : "#d97706") :
                              (isDark ? "#f87171" : "#dc2626");
            } else if (colName === "STRAND" || colName === "strand") {
              ctx.fillStyle = isDark ? "#a78bfa" : "#7c3aed";
            } else if (colName === "FLAG" || colName === "flag") {
              var flg = parseInt(val);
              if (flg & 4) ctx.fillStyle = isDark ? "#f87171" : "#dc2626"; // unmapped
            }

            // Quality string: per-char coloring
            if (colName === "QUAL_STR" || colName === "quality" || (f.parsed.format === "fastq" && ci === 3)) {
              var cx = x + padX;
              for (var qi = 0; qi < Math.min(val.length, maxCharPerCol); qi++) {
                var qScore = val.charCodeAt(qi) - 33;
                ctx.fillStyle = qScore >= 30 ? (isDark ? "#34d399" : "#059669") :
                                qScore >= 20 ? (isDark ? "#fbbf24" : "#d97706") :
                                qScore >= 10 ? (isDark ? "#fb923c" : "#ea580c") :
                                (isDark ? "#f87171" : "#dc2626");
                ctx.fillText(val.charAt(qi), cx, y + 13);
                cx += charW;
              }
            } else {
              var displayVal = val.substring(0, maxCharPerCol);
              ctx.fillText(displayVal, x + padX, y + 13);
              if (val.length > maxCharPerCol) {
                ctx.fillStyle = isDark ? "#475569" : "#94a3b8";
                ctx.fillText("...", x + padX + tmpCtx.measureText(displayVal).width + 2, y + 13);
              }
            }
          }

          ctx.restore();
          x += colWidths[ci];
        }

        // Row border
        ctx.fillStyle = isDark ? "#1e293b" : "#e2e8f0";
        ctx.fillRect(12, y + rowH - 0.5, totalW - 24, 0.5);
        y += rowH;
      }

      // "N more rows" footer
      var totalFiltered = rows.length;
      if (totalFiltered > displayRows.length) {
        y += 6;
        ctx.fillStyle = isDark ? "#475569" : "#94a3b8";
        ctx.font = "10px system-ui";
        var moreLabel = totalFiltered > pageSize
          ? "Page " + (currentPage + 1) + " — showing " + displayRows.length + " of " + totalFiltered.toLocaleString() + " rows"
          : "... and " + (totalFiltered - displayRows.length).toLocaleString() + " more rows";
        ctx.fillText(moreLabel, 20, y + 10);
      }

      // Watermark
      ctx.fillStyle = isDark ? "#1e293b" : "#e2e8f0";
      ctx.font = "9px system-ui";
      ctx.fillText("BLViewer — lang.bio", canvasW - 120, canvasH - 8);
    }

    canvas.toBlob(function(blob) {
      // Download as PNG file
      var fileName = f.name.replace(/\.[^.]+$/, "") + "-screenshot.png";
      downloadBlob(blob, fileName);
      // Also copy to clipboard if supported (bonus — user gets both)
      if (navigator.clipboard && typeof ClipboardItem !== "undefined") {
        navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]).then(function() {
          showToast("Screenshot downloaded & copied to clipboard");
        }).catch(function() {
          showToast("Screenshot downloaded");
        });
      } else {
        showToast("Screenshot downloaded");
      }
    });
  }

  function downloadBlob(blob, filename) {
    var url = URL.createObjectURL(blob);
    var a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    setTimeout(function() { URL.revokeObjectURL(url); }, 1000);
  }

  function showToast(msg) {
    var toast = document.createElement("div");
    toast.style.cssText = "position:fixed;bottom:60px;left:50%;transform:translateX(-50%);z-index:400;background:var(--vw-accent);color:white;padding:8px 20px;border-radius:8px;font-family:var(--vw-sans);font-size:13px;box-shadow:0 4px 16px rgba(0,0,0,0.3);animation:fadeIn 0.2s;";
    toast.textContent = msg;
    document.body.appendChild(toast);
    setTimeout(function() { toast.style.opacity = "0"; toast.style.transition = "opacity 0.3s"; }, 2000);
    setTimeout(function() { toast.remove(); }, 2500);
  }

  // ── Feature: Shareable Link (10) ─────────────────────────────
  function generateShareLink() {
    var f = files[activeTab];
    if (!f) return;
    var text = f.text || f.rawPreview || "";
    if (text.length > 100 * 1024) {
      alert("File too large to encode in URL (max 100KB). Use Export instead.");
      return;
    }
    // Detect FASTA — use 2-bit compression for smaller URLs
    var compressed;
    if (f.name && /\.(fa|fasta|fna|fas)$/i.test(f.name)) {
      compressed = "2b:" + compressDNA2bit(text);
    } else {
      compressed = btoa(unescape(encodeURIComponent(text)));
    }
    var base = window.location.origin + window.location.pathname;
    var shareUrl = base + "?name=" + encodeURIComponent(f.name) + "&data=" + encodeURIComponent(compressed);

    if (shareUrl.length > 32000) {
      alert("Encoded URL too long (" + Math.round(shareUrl.length / 1024) + "KB). Try a smaller file.");
      return;
    }

    // Show overlay
    var backdrop = document.createElement("div");
    backdrop.className = "vw-help-backdrop";
    backdrop.addEventListener("click", function() { backdrop.remove(); overlay.remove(); });
    document.body.appendChild(backdrop);

    var overlay = document.createElement("div");
    overlay.className = "vw-share-overlay";
    overlay.innerHTML =
      '<h3 style="font-size:14px;font-weight:700;color:var(--vw-text);margin:0 0 8px">Shareable Link</h3>' +
      '<p style="font-size:11px;color:var(--vw-text-dim);margin:0 0 8px">Copy this URL — anyone who opens it will see this file in BLViewer.</p>' +
      '<textarea readonly id="vw-share-url"></textarea>' +
      '<div style="display:flex;gap:8px;margin-top:8px">' +
        '<button class="vw-tbtn" id="vw-share-copy" style="flex:1">Copy to clipboard</button>' +
        '<button class="vw-tbtn" id="vw-share-close">Close</button>' +
      '</div>';
    document.body.appendChild(overlay);
    document.getElementById("vw-share-close").addEventListener("click", function() {
      overlay.remove();
      var bd = document.querySelector(".vw-help-backdrop");
      if (bd) bd.remove();
    });
    document.getElementById("vw-share-url").value = shareUrl;
    document.getElementById("vw-share-copy").addEventListener("click", function() {
      navigator.clipboard.writeText(shareUrl).then(function() {
        document.getElementById("vw-share-copy").textContent = "Copied!";
        setTimeout(function() { document.getElementById("vw-share-copy").textContent = "Copy to clipboard"; }, 2000);
      });
    });
  }

  // ── Feature: Row Diff Between Tabs (11) ──────────────────────
  function showDiffView() {
    if (files.length < 2) { alert("Open at least 2 files to compare."); return; }

    // Find two files of the same format
    var f1 = files[activeTab];
    var f2 = null;
    for (var i = 0; i < files.length; i++) {
      if (i !== activeTab && files[i].parsed.format === f1.parsed.format) { f2 = files[i]; break; }
    }
    if (!f2) { alert("No other file with the same format (" + f1.parsed.format + ") is open."); return; }

    // Build row keys for comparison
    function rowKey(r, fmt) {
      if (fmt === "vcf") return r[0] + ":" + r[1] + ":" + r[3] + ">" + r[4];
      if (fmt === "bed") return r[0] + ":" + r[1] + "-" + r[2];
      if (fmt === "fasta" || fmt === "fastq") return r[0];
      return r.join("\t");
    }

    var keys1 = {};
    f1.parsed.rows.forEach(function(r, i) { keys1[rowKey(r, f1.parsed.format)] = i; });
    var keys2 = {};
    f2.parsed.rows.forEach(function(r, i) { keys2[rowKey(r, f2.parsed.format)] = i; });

    var onlyIn1 = [], onlyIn2 = [], shared = 0;
    Object.keys(keys1).forEach(function(k) {
      if (keys2[k] !== undefined) shared++;
      else onlyIn1.push(keys1[k]);
    });
    Object.keys(keys2).forEach(function(k) {
      if (keys1[k] === undefined) onlyIn2.push(keys2[k]);
    });

    // Render diff in content area
    var content = document.getElementById("vw-content");
    content.innerHTML = "";
    var panel = document.createElement("div");
    panel.className = "vw-diff-panel";
    panel.innerHTML =
      '<div class="vw-diff-summary">' +
        '<strong>Comparing:</strong> ' + escapeHtml(f1.name) + ' vs ' + escapeHtml(f2.name) + '<br>' +
        '<span style="color:var(--vw-green)">' + shared + ' shared</span> · ' +
        '<span style="color:var(--vw-red)">' + onlyIn1.length + ' only in ' + escapeHtml(f1.name) + '</span> · ' +
        '<span style="color:var(--vw-cyan)">' + onlyIn2.length + ' only in ' + escapeHtml(f2.name) + '</span>' +
      '</div>';

    // Table of differences
    var table = document.createElement("table");
    table.className = "vw-table";
    table.style.fontSize = "11px";
    var thead = '<thead><tr><th>Source</th>';
    f1.parsed.columns.slice(0, 6).forEach(function(c) { thead += '<th>' + escapeHtml(c) + '</th>'; });
    thead += '</tr></thead>';
    table.innerHTML = thead;

    var tbody = document.createElement("tbody");
    var maxShow = 200;

    onlyIn1.slice(0, maxShow).forEach(function(idx) {
      var tr = document.createElement("tr");
      tr.className = "vw-diff-removed";
      tr.innerHTML = '<td><span class="vw-diff-badge-rm">&#9664; ' + escapeHtml(f1.name.substring(0, 15)) + '</span></td>';
      f1.parsed.rows[idx].slice(0, 6).forEach(function(v) { tr.innerHTML += '<td>' + escapeHtml(String(v || "")) + '</td>'; });
      tbody.appendChild(tr);
    });

    onlyIn2.slice(0, maxShow).forEach(function(idx) {
      var tr = document.createElement("tr");
      tr.className = "vw-diff-added";
      tr.innerHTML = '<td><span class="vw-diff-badge-add">&#9654; ' + escapeHtml(f2.name.substring(0, 15)) + '</span></td>';
      f2.parsed.rows[idx].slice(0, 6).forEach(function(v) { tr.innerHTML += '<td>' + escapeHtml(String(v || "")) + '</td>'; });
      tbody.appendChild(tr);
    });

    table.appendChild(tbody);
    panel.appendChild(table);

    if (onlyIn1.length + onlyIn2.length > maxShow) {
      panel.innerHTML += '<div style="padding:8px;color:var(--vw-text-muted);font-size:11px">Showing first ' + maxShow + ' of ' + (onlyIn1.length + onlyIn2.length) + ' differences</div>';
    }

    content.appendChild(panel);
  }

  // ── Feature: Regex Capture Groups (12) ───────────────────────
  function searchWithCaptures(f, pattern) {
    try {
      var re = new RegExp(pattern, "g");
      var groups = [];
      f.parsed.rows.forEach(function(row, ri) {
        row.forEach(function(val, ci) {
          var s = String(val || "");
          var m;
          re.lastIndex = 0;
          while ((m = re.exec(s)) !== null) {
            if (m.length > 1) {
              groups.push({ row: ri, col: ci, match: m[0], captures: m.slice(1) });
            }
            if (!re.global) break;
          }
        });
      });
      return groups;
    } catch (e) { return []; }
  }

  // ── Wire new toolbar buttons ─────────────────────────────────

  // Split toggle
  document.getElementById("vw-split-toggle").addEventListener("click", function() {
    splitMode = !splitMode;
    this.classList.toggle("active", splitMode);
    renderView();
  });

  // Diff button
  document.getElementById("vw-diff-btn").addEventListener("click", function() {
    showDiffView();
  });

  // Merge button
  document.getElementById("vw-merge-btn").addEventListener("click", function() {
    mergeFiles();
  });

  // Transpose button
  document.getElementById("vw-transpose-btn").addEventListener("click", function() {
    transposeMode = !transposeMode;
    this.classList.toggle("active", transposeMode);
    renderView();
  });

  // Highlight button
  document.getElementById("vw-highlight-btn").addEventListener("click", function() {
    showHighlightDialog();
  });

  // Screenshot button
  document.getElementById("vw-screenshot-btn").addEventListener("click", screenshotView);

  // Share button
  document.getElementById("vw-share-btn").addEventListener("click", generateShareLink);

  // Show diff button when 2+ files open — hook into renderTabs
  var origRenderTabs = typeof renderTabs === "function" ? renderTabs : null;
  // We'll update diff button visibility in updateToolbar instead

  // Column sidebar: intercept double-click on column headers
  document.addEventListener("dblclick", function(e) {
    var th = e.target.closest && e.target.closest(".vw-table th");
    if (!th) return;
    var f = files[activeTab];
    if (!f) return;
    var idx = Array.prototype.indexOf.call(th.parentNode.children, th);
    // Adjust for bookmark column
    if (bookmarkMode) idx--;
    if (idx >= 0 && idx < f.parsed.columns.length) {
      showColumnSidebar(f, idx);
    }
  });

  // ── PWA Install (with persistent dismissal) ───────────────────
  var deferredPrompt = null;
  window.addEventListener("beforeinstallprompt", function(e) {
    e.preventDefault();
    deferredPrompt = e;
    if (!localStorage.getItem("vw-install-dismissed")) {
      document.getElementById("vw-install-banner").style.display = "flex";
    }
  });

  document.getElementById("vw-install-btn").addEventListener("click", function() {
    if (deferredPrompt) {
      deferredPrompt.prompt();
      deferredPrompt.userChoice.then(function() { deferredPrompt = null; });
    }
    document.getElementById("vw-install-banner").style.display = "none";
    localStorage.setItem("vw-install-dismissed", "1");
  });

  document.getElementById("vw-install-dismiss").addEventListener("click", function() {
    document.getElementById("vw-install-banner").style.display = "none";
    localStorage.setItem("vw-install-dismissed", "1");
  });

  window.addEventListener("appinstalled", function() {
    document.getElementById("vw-install-banner").style.display = "none";
    localStorage.setItem("vw-install-dismissed", "1");
    deferredPrompt = null;
  });

  // ── Experimental banner dismiss ────────────────────────────────
  (function() {
    var expBanner = document.getElementById("vw-experimental-banner");
    var expDismiss = document.getElementById("vw-experimental-dismiss");
    if (expBanner && expDismiss) {
      if (sessionStorage.getItem("vw-exp-dismissed")) {
        expBanner.style.display = "none";
      }
      expDismiss.addEventListener("click", function() {
        expBanner.style.display = "none";
        sessionStorage.setItem("vw-exp-dismissed", "1");
      });
    }
  })();

  // ── Event wiring ───────────────────────────────────────────────

  // Drag and drop
  // Global keyboard shortcuts
  document.addEventListener("keydown", function(e) {
    if (e.key === "Escape") { closeDetailPanel(); closeHelpOverlay(); closeContextMenu(); var csp = document.getElementById("vw-colsearch"); if (csp) csp.remove(); }
    if (e.key === "?" && e.target.tagName !== "INPUT" && e.target.tagName !== "TEXTAREA") {
      e.preventDefault();
      showHelpOverlay();
    }
    // Ctrl+Shift+F: column search
    if (e.key === "F" && e.ctrlKey && e.shiftKey && activeTab >= 0) {
      e.preventDefault();
      showColumnSearch();
    }
    // Ctrl+T: transpose toggle
    if (e.key === "t" && e.ctrlKey && !e.shiftKey && activeTab >= 0 && e.target.tagName !== "INPUT") {
      e.preventDefault();
      transposeMode = !transposeMode;
      renderView();
    }
  });

  document.addEventListener("dragover", function(e) { e.preventDefault(); });
  document.addEventListener("drop", function(e) {
    e.preventDefault();
    dropZone.classList.remove("drag-over");
    if (e.dataTransfer.files.length) loadFiles(e.dataTransfer.files);
  });
  dropZone.addEventListener("dragenter", function() { dropZone.classList.add("drag-over"); });
  dropZone.addEventListener("dragleave", function(e) {
    if (!dropZone.contains(e.relatedTarget)) dropZone.classList.remove("drag-over");
  });

  // File input
  openBtn.addEventListener("click", function() { fileInput.click(); });
  addTabBtn.addEventListener("click", function() { fileInput.click(); });
  fileInput.addEventListener("change", function() {
    if (fileInput.files.length) loadFiles(fileInput.files);
    fileInput.value = "";
  });

  // ── URL input loading ───────────────────────────────────────────
  (function() {
    var urlInput = document.getElementById("vw-url-input");
    var urlLoadBtn = document.getElementById("vw-url-load");
    if (!urlInput || !urlLoadBtn) return;
    function loadFromUrl() {
      var url = urlInput.value.trim();
      if (!url) return;
      if (!/^https?:\/\//i.test(url)) { url = "https://" + url; }
      var name = url.split("/").pop().split("?")[0].split("#")[0] || "remote-file.txt";
      footerRows.textContent = "Loading " + name + "...";
      urlInput.disabled = true;
      urlLoadBtn.disabled = true;
      urlLoadBtn.textContent = "Loading...";
      // Try direct fetch first, fall back to CORS proxy
      fetch(url).then(function(resp) {
        if (!resp.ok) throw new Error("HTTP " + resp.status);
        return resp.text();
      }).catch(function() {
        // CORS blocked — try via proxy
        footerRows.textContent = "Retrying via proxy...";
        return fetch("https://api.allorigins.win/raw?url=" + encodeURIComponent(url)).then(function(resp) {
          if (!resp.ok) throw new Error("Proxy HTTP " + resp.status);
          return resp.text();
        });
      }).then(function(text) {
        addFile(name, text.length, text, true);
        urlInput.value = "";
      }).catch(function(err) {
        showFileError(name, "Failed to load URL", String(err) + "\n\nThe server may block external access. Try downloading the file and dropping it here instead.");
      }).finally(function() {
        urlInput.disabled = false;
        urlLoadBtn.disabled = false;
        urlLoadBtn.textContent = "Load";
      });
    }
    urlLoadBtn.addEventListener("click", loadFromUrl);
    urlInput.addEventListener("keydown", function(e) {
      if (e.key === "Enter") loadFromUrl();
    });
  })();

  // Search (debounced with progress indicator)
  var searchTimer = 0;
  searchInput.addEventListener("input", function() {
    clearTimeout(searchTimer);
    var val = searchInput.value.trim();
    // Show searching indicator immediately
    if (val !== searchTerm) {
      footerRows.textContent = "Searching...";
      contentEl.style.opacity = "0.5";
      contentEl.style.pointerEvents = "none";
    }
    searchTimer = setTimeout(function() {
      searchTerm = val;
      currentPage = 0;
      contentEl.style.opacity = "";
      contentEl.style.pointerEvents = "";
      if (currentView === "table") renderView();
      else updateFooter(files[activeTab]);
      searchInput.focus();

      // Regex capture groups: show results panel if pattern has capture groups
      var capturePanel = document.getElementById("vw-capture-panel");
      if (capturePanel) capturePanel.remove();
      if (val && val.indexOf("(") !== -1 && activeTab >= 0) {
        try {
          new RegExp(val); // validate
          var f = files[activeTab];
          if (f) {
            var captures = searchWithCaptures(f, val);
            if (captures.length > 0) {
              var panel = document.createElement("div");
              panel.id = "vw-capture-panel";
              panel.style.cssText = "position:fixed;bottom:40px;right:16px;z-index:150;background:var(--vw-panel);" +
                "border:1px solid var(--vw-border);border-radius:10px;padding:12px;box-shadow:0 8px 30px rgba(0,0,0,0.3);" +
                "max-height:300px;max-width:420px;overflow:auto;font-family:var(--vw-mono);font-size:11px;";
              panel.innerHTML = '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">' +
                '<span style="font-family:var(--vw-sans);font-weight:600;color:var(--vw-accent)">Capture Groups (' + captures.length + ')</span>' +
                '<button style="background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:14px">&times;</button></div>';
              var tbl = '<table style="width:100%;border-collapse:collapse"><thead><tr><th style="text-align:left;padding:2px 6px;color:var(--vw-text-dim)">Row</th><th style="text-align:left;padding:2px 6px;color:var(--vw-text-dim)">Match</th><th style="text-align:left;padding:2px 6px;color:var(--vw-text-dim)">Groups</th></tr></thead><tbody>';
              captures.slice(0, 50).forEach(function(c) {
                tbl += '<tr><td style="padding:2px 6px;color:var(--vw-text-muted)">' + (c.row + 1) + '</td>' +
                  '<td style="padding:2px 6px;color:var(--vw-green)">' + escapeHtml(c.match.substring(0, 30)) + '</td>' +
                  '<td style="padding:2px 6px;color:var(--vw-cyan)">' + c.captures.map(function(g) { return escapeHtml(String(g || "").substring(0, 20)); }).join(", ") + '</td></tr>';
              });
              tbl += '</tbody></table>';
              if (captures.length > 50) tbl += '<div style="color:var(--vw-text-muted);padding:4px 6px">...and ' + (captures.length - 50) + ' more</div>';
              panel.innerHTML += tbl;
              document.body.appendChild(panel);
              panel.querySelector("button").addEventListener("click", function() { panel.remove(); });
            }
          }
        } catch(e) { /* invalid regex, ignore */ }
      }
    }, 250);
  });

  // View mode buttons (with transition animation - Feature 9)
  ["table", "stats", "raw", "console"].forEach(function(v) {
    document.getElementById("vw-view-" + v).addEventListener("click", function() {
      if (currentView === v) return;
      currentView = v;
      document.querySelectorAll(".vw-tbtn[data-view]").forEach(function(b) { b.classList.remove("active"); });
      document.querySelector('.vw-tbtn[data-view="' + v + '"]').classList.add("active");
      searchInput.style.display = v === "console" ? "none" : "";
      animateViewSwitch(function() { renderView(); });
    });
  });

  // Number keys 1-4 to switch views
  document.addEventListener("keydown", function(e) {
    if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA") return;
    var viewMap = { "1": "table", "2": "stats", "3": "raw", "4": "console" };
    if (viewMap[e.key] && activeTab >= 0) {
      var v = viewMap[e.key];
      if (currentView === v) return;
      currentView = v;
      document.querySelectorAll(".vw-tbtn[data-view]").forEach(function(b) { b.classList.remove("active"); });
      document.querySelector('.vw-tbtn[data-view="' + v + '"]').classList.add("active");
      searchInput.style.display = v === "console" ? "none" : "";
      animateViewSwitch(function() { renderView(); });
    }
  });

  // Theme toggle
  document.getElementById("vw-theme-toggle").addEventListener("click", function() {
    var isDark = document.documentElement.classList.contains("dark");
    if (isDark) {
      document.documentElement.classList.remove("dark");
      localStorage.setItem("theme", "light");
    } else {
      document.documentElement.classList.add("dark");
      localStorage.setItem("theme", "dark");
    }
  });

  // Pin columns toggle
  var pinToggleBtn = document.getElementById("vw-pin-toggle");
  pinToggleBtn.addEventListener("click", function() {
    pinColumnsMode = !pinColumnsMode;
    pinToggleBtn.classList.toggle("active", pinColumnsMode);
    if (currentView === "table") renderView();
  });

  // Bookmark toggle
  var bmToggleBtn = document.getElementById("vw-bookmark-toggle");
  bmToggleBtn.addEventListener("click", function() {
    bookmarkMode = !bookmarkMode;
    bmToggleBtn.classList.toggle("active", bookmarkMode);
    bmToggleBtn.innerHTML = bookmarkMode ? "&#9733; Bookmarks" : "&#9734; Bookmarks";
    if (currentView === "table") renderView();
  });

  // Motif search input (debounced)
  var motifTimer = 0;
  var motifInput = document.getElementById("vw-motif-input");
  motifInput.addEventListener("input", function() {
    clearTimeout(motifTimer);
    var val = this.value.trim();
    // Show searching indicator immediately
    if (val !== motifTerm) {
      footerRows.textContent = "Highlighting motifs...";
      contentEl.style.opacity = "0.5";
      contentEl.style.pointerEvents = "none";
    }
    motifTimer = setTimeout(function() {
      motifTerm = val;
      contentEl.style.opacity = "";
      contentEl.style.pointerEvents = "";
      if (currentView === "table") renderView();
      else updateFooter(files[activeTab]);
      motifInput.focus();
    }, 350);
  });

  // Global Ctrl+K — Quick search (focus the search input)
  // Global Ctrl+Shift+G — Jump to coordinate (focus the coordinate input)
  // Global Ctrl+E — Export current view
  document.addEventListener("keydown", function(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === "k") {
      e.preventDefault();
      var searchInput = document.getElementById("vw-search");
      if (searchInput) searchInput.focus();
    }
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "G") {
      e.preventDefault();
      var jumpInput = document.querySelector(".vw-jump-input");
      if (jumpInput) jumpInput.focus();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === "e") {
      e.preventDefault();
      var exportBtn = document.getElementById("vw-export-btn");
      if (exportBtn) exportBtn.click();
    }
  });

  // Global Ctrl+G for go-to-row
  document.addEventListener("keydown", function(e) {
    if (e.ctrlKey && e.key === "g" && currentView === "table" && activeTab >= 0) {
      e.preventDefault();
      var f = files[activeTab];
      if (!f) return;
      var rows = getFilteredRows(f);
      showGotoRow(rows, function(idx) {
        // Scroll to row in existing table
        var trs = contentEl.querySelectorAll(".vw-table tbody tr");
        if (trs[idx]) {
          trs[idx].style.outline = "1px solid var(--vw-accent)";
          trs[idx].scrollIntoView({ block: "center" });
        }
      });
    }
  });

  // Row detail toggle
  var detailToggleBtn = document.getElementById("vw-detail-toggle");
  if (rowDetailEnabled) detailToggleBtn.classList.add("active");
  detailToggleBtn.addEventListener("click", function() {
    rowDetailEnabled = !rowDetailEnabled;
    localStorage.setItem("vw-row-detail", rowDetailEnabled ? "1" : "0");
    detailToggleBtn.classList.toggle("active", rowDetailEnabled);
    if (!rowDetailEnabled) closeDetailPanel();
  });

  // Export
  document.getElementById("vw-export-btn").addEventListener("click", function(e) {
    // Show export format picker
    var existing = document.getElementById("vw-export-menu");
    if (existing) { existing.remove(); return; }
    var f = files[activeTab];
    if (!f) return;

    var menu = document.createElement("div");
    menu.id = "vw-export-menu";
    menu.style.cssText = "position:fixed;z-index:200;background:var(--vw-panel);border:1px solid var(--vw-border);" +
      "border-radius:8px;padding:4px;box-shadow:0 8px 20px rgba(0,0,0,0.3);font-family:var(--vw-sans);font-size:12px;";

    var formats = [{ label: "CSV", fmt: "csv" }, { label: "TSV", fmt: "tsv" }];
    if (f.parsed.format === "bed") formats.push({ label: "BED", fmt: "bed" });
    if (f.parsed.format === "vcf") formats.push({ label: "VCF", fmt: "vcf" });

    // Subset export options
    if (selectedRows.size > 0) formats.push({ label: "Export selected (" + selectedRows.size + " rows)", fmt: "csv:selected" });
    formats.push({ label: "Export current page", fmt: "csv:page" });

    // Format conversion options
    if (f.parsed.format === "vcf") formats.push({ label: "Convert to BED", fmt: "convert:bed" });
    if (f.parsed.format === "gff") formats.push({ label: "Convert to BED", fmt: "convert:bed" });
    if (f.parsed.format === "fasta") formats.push({ label: "Reverse complement FASTA", fmt: "convert:fasta-rc" });

    formats.forEach(function(item) {
      var btn = document.createElement("div");
      btn.style.cssText = "padding:6px 16px;cursor:pointer;border-radius:4px;color:var(--vw-text);";
      btn.textContent = item.fmt.startsWith("convert:") || item.fmt.indexOf(":") !== -1 ? item.label : "Export as " + item.label;
      if (item.fmt.startsWith("convert:")) btn.style.color = "var(--vw-cyan)";
      if (item.fmt.indexOf(":selected") !== -1 || item.fmt.indexOf(":page") !== -1) btn.style.color = "var(--vw-accent)";
      btn.addEventListener("mouseenter", function() { btn.style.background = "var(--vw-row-hover)"; });
      btn.addEventListener("mouseleave", function() { btn.style.background = ""; });
      btn.addEventListener("click", function() {
        if (item.fmt.startsWith("convert:")) {
          var target = item.fmt.split(":")[1];
          var converted = convertFormat(f, target);
          if (converted) {
            var ext = target === "fasta-rc" ? ".rc.fa" : "." + target;
            var blob = new Blob([converted], { type: "text/plain" });
            var url = URL.createObjectURL(blob);
            var a = document.createElement("a");
            a.href = url;
            a.download = f.name.replace(/\.[^.]+$/, "") + ext;
            a.click();
            URL.revokeObjectURL(url);
          }
        } else {
          exportData(item.fmt);
        }
        menu.remove();
      });
      menu.appendChild(btn);
    });

    var rect = e.target.getBoundingClientRect();
    menu.style.left = rect.left + "px";
    menu.style.top = (rect.bottom + 4) + "px";
    document.body.appendChild(menu);

    function closeMenu(ev) {
      if (!menu.contains(ev.target) && ev.target !== e.target) {
        menu.remove();
        document.removeEventListener("mousedown", closeMenu);
      }
    }
    setTimeout(function() { document.addEventListener("mousedown", closeMenu); }, 0);
  });

  // Open in BioBrowser — pass file text via sessionStorage
  browserBtn.addEventListener("click", function() {
    var f = files[activeTab];
    if (!f || !BROWSER_FORMATS[f.parsed.format]) return;
    if (!f.text) {
      alert("File is too large to pass to BioBrowser (" + formatBytes(f.size) + "). Please drop the file directly in BioBrowser.");
      window.open("browser.html", "_blank");
      return;
    }
    try {
      // Store file data for BioBrowser to pick up
      sessionStorage.setItem("biobrowser_file", JSON.stringify({
        name: f.name,
        text: f.text,
        format: f.parsed.format
      }));
      window.open("browser.html", "_blank");
    } catch (e) {
      // sessionStorage may fail for very large files (>5MB quota)
      if (e.name === "QuotaExceededError") {
        // Fallback: open browser and let user re-drop
        alert("File is too large to pass directly (" + formatBytes(f.size) + "). BioBrowser will open — please drop the file again.");
        window.open("browser.html", "_blank");
      }
    }
  });

  // Register service worker
  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.register("sw.js").catch(function() {});
  }

  // Handle files opened via PWA file_handlers
  if ("launchQueue" in window) {
    window.launchQueue.setConsumer(function(params) {
      params.files.forEach(function(handle) {
        handle.getFile().then(function(file) {
          loadFiles([file]);
        });
      });
    });
  }

  // ── URL parameter loading ──────────────────────────────────────
  // Support ?url=https://... to load files from public URLs
  // Also handles web+bioview: protocol (Feature 3)
  (function() {
    var params = new URLSearchParams(window.location.search);

    // web+bioview: protocol handler — arrives as ?bioview=<encoded-url>
    var bioview = params.get("bioview");
    if (bioview) {
      // Strip protocol prefix if present
      var fileUrl = bioview.replace(/^web\+bioview:/, "");
      if (fileUrl.startsWith("http://") || fileUrl.startsWith("https://")) {
        var bvName = fileUrl.split("/").pop().split("?")[0] || "remote-file.txt";
        footerRows.textContent = "Loading " + bvName + "...";
        fetch(fileUrl).then(function(resp) {
          if (!resp.ok) throw new Error("HTTP " + resp.status);
          return resp.text();
        }).then(function(text) {
          addFile(bvName, text.length, text, false);
        }).catch(function(err) {
          showFileError(bvName, "Protocol handler failed", String(err));
        });
        return;
      }
    }

    // Share link decoder: ?data=<base64>&name=<filename>
    var shareData = params.get("data");
    if (shareData) {
      try {
        var raw = decodeURIComponent(shareData);
        var decoded;
        if (raw.indexOf("2b:") === 0) {
          decoded = decompressDNA2bit(raw.substring(3));
        } else {
          decoded = decodeURIComponent(escape(atob(raw)));
        }
        var shareName = params.get("name") || "shared-file.txt";
        addFile(shareName, decoded.length, decoded, false);
      } catch (e) {
        console.warn("Failed to decode shared link:", e);
      }
      return;
    }

    var url = params.get("url");
    if (!url) return;
    var name = url.split("/").pop().split("?")[0] || "remote-file.txt";
    footerRows.textContent = "Loading " + name + "...";
    dropZone.querySelector(".vw-drop-title").textContent = "Loading remote file...";
    dropZone.querySelector(".vw-drop-sub").textContent = url;

    fetch(url).then(function(resp) {
      if (!resp.ok) throw new Error("HTTP " + resp.status);
      return resp.text();
    }).catch(function() {
      // CORS blocked — try via proxy
      dropZone.querySelector(".vw-drop-sub").textContent = "Retrying via proxy...";
      return fetch("https://api.allorigins.win/raw?url=" + encodeURIComponent(url)).then(function(resp) {
        if (!resp.ok) throw new Error("Proxy HTTP " + resp.status);
        return resp.text();
      });
    }).then(function(text) {
      addFile(name, text.length, text, false);
    }).catch(function(err) {
      showFileError(name, "Failed to load URL", String(err) + "\n\nThe server may block external access. Try downloading the file and dropping it here instead.");
    });
  })();

  // ── Receive files from BioGist extension via postMessage ──
  window.addEventListener("message", function(e) {
    if (e.data && e.data.type === "biogist-file" && e.data.content) {
      addFile(e.data.name || "biogist-file.txt", e.data.content.length, e.data.content, false);
    }
  });

  // ── Recent Files (Feature 4) ─────────────────────────────────
  function formatTimeAgo(ts) {
    var diff = Date.now() - ts;
    var mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return mins + "m ago";
    var hrs = Math.floor(mins / 60);
    if (hrs < 24) return hrs + "h ago";
    var days = Math.floor(hrs / 24);
    if (days < 30) return days + "d ago";
    return new Date(ts).toLocaleDateString();
  }

  function renderRecentFiles() {
    var panel = document.getElementById("vw-recent-panel");
    var list = document.getElementById("vw-recent-list");
    if (!panel || !list) return;

    HistoryDB.getAll().then(function(entries) {
      if (entries.length === 0) {
        panel.style.display = "none";
        return;
      }
      list.innerHTML = "";
      entries.forEach(function(entry) {
        var item = document.createElement("div");
        item.className = "vw-recent-item";

        var badge = '<span class="vw-recent-badge">' + escapeHtml(entry.format) + '</span>';
        var cached = entry.content ? '<span class="vw-recent-cached" title="Cached — instant restore"></span>' : '';
        var name = '<span class="vw-recent-name">' + escapeHtml(entry.name) + '</span>';
        var meta = '<span class="vw-recent-meta">' + formatBytes(entry.size) + ' · ' + (entry.rowCount || 0) + ' rows · ' + formatTimeAgo(entry.date) + '</span>';
        var removeBtn = '<button class="vw-recent-remove" title="Remove from history">&times;</button>';

        item.innerHTML = badge + cached + name + meta + removeBtn;

        // Click to restore
        item.addEventListener("click", function(e) {
          if (e.target.classList.contains("vw-recent-remove")) return;
          if (entry.content) {
            addFile(entry.name, entry.size, entry.content, false);
          } else {
            // No cached content — prompt file picker
            var msg = "\"" + entry.name + "\" is too large to cache. Please re-select it.";
            alert(msg);
            document.getElementById("vw-file-input").click();
          }
        });

        // Remove button
        item.querySelector(".vw-recent-remove").addEventListener("click", function(e) {
          e.stopPropagation();
          HistoryDB.remove(entry.id).then(function() {
            item.remove();
            // Hide panel if empty
            if (!list.querySelector(".vw-recent-item")) panel.style.display = "none";
          });
        });

        list.appendChild(item);
      });
      panel.style.display = "";
    }).catch(function() {
      panel.style.display = "none";
    });
  }

  // Clear history button
  var clearHistBtn = document.getElementById("vw-clear-history");
  if (clearHistBtn) {
    clearHistBtn.addEventListener("click", function() {
      HistoryDB.clearAll().then(function() {
        var panel = document.getElementById("vw-recent-panel");
        if (panel) panel.style.display = "none";
        localStorage.removeItem("vw-last-session");
      });
    });
  }

  // ── Session Restore (Feature 5) ──────────────────────────────
  function checkSessionRestore() {
    var lastId = localStorage.getItem("vw-last-session");
    if (!lastId) return;
    // Don't show restore if files are already open (e.g. from URL or launchQueue)
    if (files.length > 0) return;

    HistoryDB.getById(lastId).then(function(entry) {
      if (!entry) return;
      var banner = document.getElementById("vw-restore-banner");
      var text = document.getElementById("vw-restore-text");
      var btn = document.getElementById("vw-restore-btn");
      var dismiss = document.getElementById("vw-restore-dismiss");
      if (!banner) return;

      if (entry.content) {
        text.textContent = "Restore previous session: " + entry.name + " (" + formatBytes(entry.size) + ")?";
        btn.textContent = "Restore";
        btn.onclick = function() {
          addFile(entry.name, entry.size, entry.content, false);
          banner.style.display = "none";
        };
      } else {
        text.textContent = "Re-open " + entry.name + "? (" + formatBytes(entry.size) + " — must re-select file)";
        btn.textContent = "Browse...";
        btn.onclick = function() {
          document.getElementById("vw-file-input").click();
          banner.style.display = "none";
        };
      }
      dismiss.onclick = function() {
        banner.style.display = "none";
        localStorage.removeItem("vw-last-session");
      };
      banner.style.display = "flex";
    }).catch(function() {});
  }

  // ── Feature 1: Dark/Light Theme Toggle (enhanced) ─────────────
  // Already handled by vw-theme-toggle button above.
  // Restore theme from localStorage on init
  (function() {
    var storedTheme = localStorage.getItem("vw-theme") || localStorage.getItem("theme");
    if (storedTheme === "light") {
      document.documentElement.classList.remove("dark");
    } else if (storedTheme === "dark") {
      document.documentElement.classList.add("dark");
    }
  })();

  // ── Feature 9: Saved Views/Presets ──────────────────────────────
  function saveCurrentView(name) {
    var view = {
      sortCol: sortCol, sortAsc: sortAsc, sortCols: sortCols.slice(),
      pinnedCols: Array.from(pinnedCols),
      groupByCol: groupByCol,
      pageSize: pageSize,
      colFilters: Object.keys(colFilters).reduce(function(acc, k) { acc[k] = Array.from(colFilters[k]); return acc; }, {}),
      hiddenCols: hiddenCols[activeTab] ? Array.from(hiddenCols[activeTab]) : [],
      highlightRule: highlightRule ? JSON.parse(JSON.stringify(highlightRule)) : null
    };
    savedViews[name] = view;
    localStorage.setItem("vw-saved-views", JSON.stringify(savedViews));
  }

  function restoreView(name) {
    var view = savedViews[name];
    if (!view) return;
    pushUndo();
    sortCol = view.sortCol !== undefined ? view.sortCol : -1;
    sortAsc = view.sortAsc !== undefined ? view.sortAsc : true;
    sortCols = view.sortCols || [];
    pinnedCols = new Set(view.pinnedCols || []);
    groupByCol = view.groupByCol !== undefined ? view.groupByCol : -1;
    if (view.pageSize) pageSize = view.pageSize;
    colFilters = {};
    if (view.colFilters) Object.keys(view.colFilters).forEach(function(k) { colFilters[k] = new Set(view.colFilters[k]); });
    hiddenCols[activeTab] = new Set(view.hiddenCols || []);
    highlightRule = view.highlightRule || null;
    renderView();
  }

  function deleteSavedView(name) {
    delete savedViews[name];
    localStorage.setItem("vw-saved-views", JSON.stringify(savedViews));
  }

  function showViewsDropdown(anchorEl) {
    var existing = document.getElementById("vw-views-menu");
    if (existing) { existing.remove(); return; }

    var menu = document.createElement("div");
    menu.id = "vw-views-menu";
    menu.style.cssText = "position:fixed;z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:8px;padding:6px;box-shadow:0 8px 24px rgba(0,0,0,0.4);font-family:var(--vw-sans);font-size:12px;min-width:180px;";

    // Save current
    var saveItem = document.createElement("div");
    saveItem.className = "vw-ctx-item";
    saveItem.textContent = "+ Save current view";
    saveItem.style.color = "var(--vw-accent)";
    saveItem.addEventListener("click", function() {
      var name = prompt("View name:");
      if (name && name.trim()) { saveCurrentView(name.trim()); showToast("View saved: " + name.trim()); }
      menu.remove();
    });
    menu.appendChild(saveItem);

    // List saved views
    var names = Object.keys(savedViews);
    if (names.length > 0) {
      var sep = document.createElement("div");
      sep.style.cssText = "height:1px;background:var(--vw-border);margin:4px 0;";
      menu.appendChild(sep);
    }
    names.forEach(function(name) {
      var row = document.createElement("div");
      row.style.cssText = "display:flex;align-items:center;gap:4px;padding:4px 8px;border-radius:4px;cursor:pointer;";
      row.addEventListener("mouseenter", function() { row.style.background = "var(--vw-row-hover)"; });
      row.addEventListener("mouseleave", function() { row.style.background = ""; });
      var label = document.createElement("span");
      label.style.cssText = "flex:1;color:var(--vw-text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;";
      label.textContent = name;
      label.addEventListener("click", function() { restoreView(name); menu.remove(); });
      var delBtn = document.createElement("button");
      delBtn.textContent = "\u00d7";
      delBtn.style.cssText = "background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:14px;padding:0 2px;";
      delBtn.addEventListener("click", function(e) { e.stopPropagation(); deleteSavedView(name); row.remove(); });
      row.appendChild(label);
      row.appendChild(delBtn);
      menu.appendChild(row);
    });

    var rect = anchorEl.getBoundingClientRect();
    menu.style.left = rect.left + "px";
    menu.style.top = (rect.bottom + 4) + "px";
    document.body.appendChild(menu);
    setTimeout(function() {
      document.addEventListener("mousedown", function handler(ev) {
        if (!menu.contains(ev.target)) { menu.remove(); document.removeEventListener("mousedown", handler); }
      });
    }, 0);
  }

  // ── Feature 10: Selection Stats Bar ─────────────────────────────
  function renderSelectionStatsBar(f) {
    var existing = document.getElementById("vw-sel-stats-bar");
    if (existing) existing.remove();
    if (!f || selectedRows.size === 0) return;

    // Find first numeric column
    var numCol = -1;
    for (var ci = 0; ci < f.parsed.colTypes.length; ci++) {
      if (f.parsed.colTypes[ci] === "num") { numCol = ci; break; }
    }
    if (numCol < 0) return;

    var count = 0, sum = 0, mn = Infinity, mx = -Infinity;
    selectedRows.forEach(function(idx) {
      var v = f.parsed.rows[idx] ? f.parsed.rows[idx][numCol] : NaN;
      if (typeof v === "number" && !isNaN(v)) { sum += v; count++; if (v < mn) mn = v; if (v > mx) mx = v; }
    });
    if (count === 0) return;
    var mean = sum / count;

    var bar = document.createElement("div");
    bar.id = "vw-sel-stats-bar";
    bar.style.cssText = "display:flex;align-items:center;gap:16px;padding:6px 12px;background:var(--vw-tab-bg);border-top:1px solid var(--vw-border);font-family:var(--vw-mono);font-size:11px;color:var(--vw-text-dim);flex-shrink:0;";
    var fmt = function(n) { return n.toLocaleString(undefined, {maximumFractionDigits: 2}); };
    bar.innerHTML =
      '<span style="color:var(--vw-accent);font-weight:600">' + f.parsed.columns[numCol] + '</span>' +
      '<span>Count: <b style="color:var(--vw-text)">' + count + '</b></span>' +
      '<span>Sum: <b style="color:var(--vw-cyan)">' + fmt(sum) + '</b></span>' +
      '<span>Mean: <b style="color:var(--vw-text)">' + fmt(mean) + '</b></span>' +
      '<span>Min: <b style="color:var(--vw-text)">' + fmt(mn) + '</b></span>' +
      '<span>Max: <b style="color:var(--vw-text)">' + fmt(mx) + '</b></span>';

    // Insert before footer
    var footer = document.querySelector(".vw-footer");
    if (footer) footer.parentNode.insertBefore(bar, footer);
  }

  // Hook selection stats into selection changes
  var origUpdateSelectionSummary = updateSelectionSummary;
  updateSelectionSummary = function(f) {
    origUpdateSelectionSummary(f);
    renderSelectionStatsBar(f);
  };

  // ── Feature 11: VCF INFO Field Parser ──────────────────────────
  function expandVcfInfo(f) {
    if (f.parsed.format !== "vcf") return;
    var infoIdx = f.parsed.columns.indexOf("INFO");
    if (infoIdx === -1) infoIdx = f.parsed.columns.indexOf("info");
    if (infoIdx < 0) { showToast("No INFO column found"); return; }

    // Collect unique keys from loaded rows
    var keys = {};
    var n = f.parsed.rows.length;
    for (var i = 0; i < n; i++) {
      var infoPairs = parseInfoField(f.parsed.rows[i][infoIdx]);
      for (var j = 0; j < infoPairs.length; j++) {
        keys[infoPairs[j].key] = 1;
      }
    }
    var keyList = Object.keys(keys);
    if (keyList.length === 0) { showToast("No INFO fields found"); return; }

    pushUndo();
    // Add new columns for each INFO key
    keyList.forEach(function(key) {
      f.parsed.columns.push("INFO_" + key);
      // Determine type: check if all values are numeric
      var isNum = true;
      for (var ri = 0; ri < Math.min(n, 100); ri++) {
        var pairs = parseInfoField(f.parsed.rows[ri][infoIdx]);
        var found = false;
        for (var pi = 0; pi < pairs.length; pi++) {
          if (pairs[pi].key === key) { if (isNaN(parseFloat(pairs[pi].value))) isNum = false; found = true; break; }
        }
      }
      f.parsed.colTypes.push(isNum ? "num" : "str");
    });

    // Populate values
    for (var ri = 0; ri < n; ri++) {
      var infoPairs = parseInfoField(f.parsed.rows[ri][infoIdx]);
      var infoMap = {};
      for (var pi = 0; pi < infoPairs.length; pi++) infoMap[infoPairs[pi].key] = infoPairs[pi].value;
      for (var ki = 0; ki < keyList.length; ki++) {
        var val = infoMap[keyList[ki]] || "";
        var colType = f.parsed.colTypes[f.parsed.columns.length - keyList.length + ki];
        f.parsed.rows[ri].push(colType === "num" && val !== "" ? parseFloat(val) || 0 : val);
      }
    }

    // Invalidate caches
    f.parsed._colHints = null;
    f.parsed._summaryCache = null;
    f.parsed._anomalyCache = null;
    _cachedFile = null;
    _filterCache = null;
    showToast("Expanded " + keyList.length + " INFO fields");
    renderView();
  }

  // ── Feature 13: GC% Auto-Offer ─────────────────────────────────
  function showGcOffer(f) {
    if (f.parsed.format !== "fasta" && f.parsed.format !== "fastq") return;
    // Check if GC% column already exists
    if (f.parsed.columns.indexOf("GC%") >= 0) return;

    var bar = document.createElement("div");
    bar.id = "vw-gc-offer";
    bar.style.cssText = "display:flex;align-items:center;gap:12px;padding:8px 16px;background:linear-gradient(90deg,rgba(52,211,153,0.1),rgba(34,211,238,0.08));border-bottom:1px solid rgba(52,211,153,0.25);font-family:var(--vw-sans);font-size:13px;color:var(--vw-green);flex-shrink:0;";
    bar.innerHTML = '<span>Add GC% column?</span>';
    var yesBtn = document.createElement("button");
    yesBtn.textContent = "Yes";
    yesBtn.style.cssText = "background:var(--vw-green);color:#000;border:none;border-radius:6px;padding:4px 14px;cursor:pointer;font-size:12px;font-weight:600;";
    yesBtn.addEventListener("click", function() {
      var seqCol = f.parsed.colTypes.indexOf("seq");
      if (seqCol >= 0) { pushUndo(); addComputedColumn(f, "gc", seqCol); }
      bar.remove();
    });
    var noBtn = document.createElement("button");
    noBtn.textContent = "No";
    noBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:6px;padding:4px 14px;cursor:pointer;font-size:12px;color:var(--vw-text-dim);";
    noBtn.addEventListener("click", function() { bar.remove(); });
    bar.appendChild(yesBtn);
    bar.appendChild(noBtn);

    // Insert after toolbar
    var toolbar = document.getElementById("vw-toolbar");
    if (toolbar && toolbar.nextSibling) toolbar.parentNode.insertBefore(bar, toolbar.nextSibling);
    else if (toolbar) toolbar.parentNode.appendChild(bar);
  }

  // ── Feature 14: Genomic Coordinate Jump ──────────────────────────
  function setupCoordJump() {
    var existing = document.getElementById("vw-coord-input");
    if (existing) existing.remove();
    var f = files[activeTab];
    if (!f) return;
    var coordFormats = { vcf: 1, bed: 1, gff: 1, sam: 1 };
    if (!coordFormats[f.parsed.format]) return;

    var input = document.createElement("input");
    input.id = "vw-coord-input";
    input.type = "text";
    input.placeholder = "chr1:12345 or chr1:1k-5k";
    input.title = "Jump to coordinate (chr1:12345) or filter range (chr1:1000-5000)";
    input.style.cssText = "font-family:var(--vw-mono);font-size:11px;padding:3px 8px;border-radius:5px;border:1px solid var(--vw-border);background:var(--vw-tab-bg);color:var(--vw-text);outline:none;width:170px;";
    input.addEventListener("keydown", function(e) {
      if (e.key === "Enter") {
        var val = input.value.trim();
        // Try range format first: chr1:1000-5000
        var rangeMatch = val.match(/^(chr[A-Za-z0-9]+|\d+):(\d+)[-\u2013](\d+)$/i);
        // Then try single-position: chr1:12345
        var posMatch = !rangeMatch ? val.match(/^(chr[A-Za-z0-9]+|\d+):(\d+)$/i) : null;
        if (!rangeMatch && !posMatch) { showToast("Use chr1:12345 or chr1:1000-5000"); return; }
        // Find chrom and position columns
        var chromCol = -1, posCol = -1;
        f.parsed.columns.forEach(function(c, i) {
          if (/^(chrom|#chrom|chr|seqid|rname)/i.test(c)) chromCol = i;
          if (/^(pos|start|chromstart)/i.test(c)) posCol = i;
        });
        if (chromCol < 0 || posCol < 0) { showToast("No chrom/pos columns found"); return; }

        if (posMatch) {
          // Single-position jump: scroll to first row at or after this position
          var jumpChrom = posMatch[1], jumpPos = parseInt(posMatch[2]);
          var rows = f.parsed.rows;
          var bestIdx = -1, bestDist = Infinity;
          for (var ri = 0; ri < rows.length; ri++) {
            if (String(rows[ri][chromCol]).toLowerCase() === jumpChrom.toLowerCase()) {
              var rp = parseInt(rows[ri][posCol]);
              if (!isNaN(rp) && rp >= jumpPos && (rp - jumpPos) < bestDist) {
                bestDist = rp - jumpPos;
                bestIdx = ri;
                if (bestDist === 0) break;
              }
            }
          }
          if (bestIdx < 0) { showToast("No rows found at " + jumpChrom + ":" + jumpPos); return; }
          // Clear any active filters and jump to the page
          colFilters = {};
          _filterCache = null;
          currentPage = Math.floor(bestIdx / pageSize);
          renderView();
          setTimeout(function() {
            var rowInPage = bestIdx % pageSize;
            var tbody = document.querySelector(".vw-table tbody");
            if (tbody) {
              var trs = tbody.querySelectorAll("tr:not(.vw-group-row)");
              if (trs[rowInPage]) {
                trs[rowInPage].classList.add("vw-row-flash");
                trs[rowInPage].scrollIntoView({ block: "center" });
              }
            }
          }, 50);
          showToast("Jumped to " + jumpChrom + ":" + rows[bestIdx][posCol]);
        } else {
          // Range filter (existing behavior)
          var chrom = rangeMatch[1], startPos = parseInt(rangeMatch[2]), endPos = parseInt(rangeMatch[3]);
          colFilters[chromCol] = new Set([chrom]);
          currentPage = 0;
          var origFilterFn = getFilteredRows;
          _filterCache = null;
          var filtered = getFilteredRows(f).filter(function(item) {
            var pos = parseInt(item.row[posCol]);
            return !isNaN(pos) && pos >= startPos && pos <= endPos;
          });
          _filterCache = filtered;
          _filterKey = activeTab + "|coord:" + val + "|" + f.parsed.rows.length;
          renderView();
          showToast("Filtered to " + chrom + ":" + startPos + "-" + endPos);
        }
      }
    });

    // Insert into toolbar
    var toolbar = document.getElementById("vw-toolbar");
    var sep = document.createElement("div");
    sep.className = "vw-toolbar-sep";
    sep.id = "vw-coord-sep";
    var searchEl = document.getElementById("vw-search");
    if (searchEl && searchEl.parentNode) {
      searchEl.parentNode.insertBefore(sep, searchEl.nextSibling);
      sep.parentNode.insertBefore(input, sep.nextSibling);
    }
  }

  // ── Feature 15: Enhanced Keyboard Shortcuts Help ────────────────
  // Already has showHelpOverlay — extend it with more shortcuts
  var origShowHelpOverlay = showHelpOverlay;
  showHelpOverlay = function() {
    if (document.querySelector(".vw-help-overlay")) { closeHelpOverlay(); return; }
    var backdrop = document.createElement("div");
    backdrop.className = "vw-help-backdrop";
    backdrop.addEventListener("click", closeHelpOverlay);
    document.body.appendChild(backdrop);

    var overlay = document.createElement("div");
    overlay.className = "vw-help-overlay";
    overlay.innerHTML =
      '<button class="vw-help-close">&times;</button>' +
      '<h3>Keyboard Shortcuts</h3>' +
      '<table>' +
      '<tr><td>?</td><td>Show/hide this help</td></tr>' +
      '<tr><td>j / &darr;</td><td>Next row</td></tr>' +
      '<tr><td>k / &uarr;</td><td>Previous row</td></tr>' +
      '<tr><td>h / &larr;</td><td>Previous column</td></tr>' +
      '<tr><td>l / &rarr;</td><td>Next column</td></tr>' +
      '<tr><td>g</td><td>Jump to first row</td></tr>' +
      '<tr><td>G</td><td>Jump to last row</td></tr>' +
      '<tr><td>Ctrl+G</td><td>Go to row number</td></tr>' +
      '<tr><td>Ctrl+T</td><td>Toggle transpose view</td></tr>' +
      '<tr><td>Ctrl+Shift+F</td><td>Find column by name</td></tr>' +
      '<tr><td>Ctrl+K</td><td>Focus search</td></tr>' +
      '<tr><td>Ctrl+Shift+G</td><td>Jump to coordinate</td></tr>' +
      '<tr><td>Ctrl+E</td><td>Export menu</td></tr>' +
      '<tr><td>Ctrl+Z</td><td>Undo last action</td></tr>' +
      '<tr><td>Enter</td><td>Copy cell value / confirm</td></tr>' +
      '<tr><td>/</td><td>Focus search box</td></tr>' +
      '<tr><td>Esc</td><td>Close panel / overlay</td></tr>' +
      '<tr><td>1-4</td><td>Switch view (Table/Stats/Raw/BioLang)</td></tr>' +
      '<tr><td>Space</td><td>Toggle row selection (with focus)</td></tr>' +
      '<tr><td>Tab</td><td>Next tab (in table navigation)</td></tr>' +
      '</table>' +
      '<h3 style="margin-top:14px">Mouse</h3>' +
      '<table>' +
      '<tr><td>Click header</td><td>Sort: asc &rarr; desc &rarr; default</td></tr>' +
      '<tr><td>Shift+Click hdr</td><td>Add secondary sort column</td></tr>' +
      '<tr><td>Right-click hdr</td><td>Column filter / pin / type</td></tr>' +
      '<tr><td>Right-click cell</td><td>Copy / BLAST / rev complement</td></tr>' +
      '<tr><td>Drag header</td><td>Reorder columns</td></tr>' +
      '<tr><td>Drag header edge</td><td>Resize column width</td></tr>' +
      '<tr><td>Ctrl+click row</td><td>Multi-select (stats in footer)</td></tr>' +
      '<tr><td>Double-click hdr</td><td>Column histogram sidebar</td></tr>' +
      '<tr><td>Double-click cell</td><td>Edit cell value in-place</td></tr>' +
      '</table>' +
      '<h3 style="margin-top:14px">Toolbar</h3>' +
      '<table>' +
      '<tr><td>&#9734; Bookmarks</td><td>Toggle bookmark column</td></tr>' +
      '<tr><td>&#128204; Pin Cols</td><td>Freeze first 2 columns</td></tr>' +
      '<tr><td>&#8597; Detail</td><td>Toggle row click detail panel</td></tr>' +
      '<tr><td>&#9788; Theme</td><td>Toggle light/dark mode</td></tr>' +
      '<tr><td>&#128194; Views</td><td>Save/restore view presets</td></tr>' +
      '</table>';
    document.body.appendChild(overlay);
    overlay.querySelector(".vw-help-close").addEventListener("click", closeHelpOverlay);
  };

  // ── Feature 17: Drag Column Header to Group ─────────────────────
  var groupDropZone = null;

  function showGroupDropZone() {
    if (groupDropZone) return;
    groupDropZone = document.createElement("div");
    groupDropZone.id = "vw-group-dropzone";
    groupDropZone.style.cssText = "display:flex;align-items:center;justify-content:center;gap:8px;padding:6px 12px;background:rgba(124,58,237,0.08);border:2px dashed var(--vw-accent);border-radius:6px;margin:0 12px;font-family:var(--vw-sans);font-size:12px;color:var(--vw-accent);min-height:28px;flex-shrink:0;";
    groupDropZone.textContent = "Drop column here to group by";
    groupDropZone.addEventListener("dragover", function(e) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      groupDropZone.style.background = "rgba(124,58,237,0.15)";
    });
    groupDropZone.addEventListener("dragleave", function() {
      groupDropZone.style.background = "rgba(124,58,237,0.08)";
    });
    groupDropZone.addEventListener("drop", function(e) {
      e.preventDefault();
      var colIdx = parseInt(e.dataTransfer.getData("text/plain"));
      if (!isNaN(colIdx) && colIdx >= 0) {
        groupByCol = colIdx;
        currentPage = 0;
        renderView();
      }
      hideGroupDropZone();
    });
    var content = document.getElementById("vw-content");
    if (content) content.parentNode.insertBefore(groupDropZone, content);
  }

  function hideGroupDropZone() {
    if (groupDropZone) { groupDropZone.remove(); groupDropZone = null; }
  }

  // Listen for drag events on th to show/hide group drop zone
  document.addEventListener("dragstart", function(e) {
    var th = e.target.closest && e.target.closest(".vw-table th");
    if (th) showGroupDropZone();
  });
  document.addEventListener("dragend", function() {
    setTimeout(hideGroupDropZone, 100);
  });

  // ── Feature 18: Cell Editing ──────────────────────────────────
  document.addEventListener("dblclick", function(e) {
    var td = e.target.closest && e.target.closest(".vw-table td");
    if (!td) return;
    var th = e.target.closest && e.target.closest(".vw-table th");
    if (th) return; // don't edit headers

    var f = files[activeTab];
    if (!f) return;

    var tr = td.closest("tr");
    if (!tr || tr.classList.contains("vw-summary-row") || tr.classList.contains("vw-group-row")) return;

    // Get row and col index
    var tbody = td.closest("tbody");
    if (!tbody) return;
    var trs = Array.prototype.slice.call(tbody.querySelectorAll("tr:not(.vw-group-row)"));
    var rowVisIdx = trs.indexOf(tr);
    if (rowVisIdx < 0) return;

    var cells = Array.prototype.slice.call(tr.querySelectorAll("td"));
    var colVisIdx = cells.indexOf(td);
    // Adjust for bookmark and row-number columns
    var colOffset = 1 + (bookmarkMode ? 1 : 0);
    var colIdx = colVisIdx - colOffset;
    if (colIdx < 0 || colIdx >= f.parsed.columns.length) return;

    // Get actual row index from filtered/sorted/paged rows
    var rows = getFilteredRows(f);
    if (sortCols.length > 0) {
      rows = rows.slice().sort(function(a, b) {
        for (var si = 0; si < sortCols.length; si++) {
          var sc = sortCols[si];
          var va = a.row[sc.col], vb = b.row[sc.col];
          var cmp = 0;
          if (typeof va === "number" && typeof vb === "number") cmp = va - vb;
          else cmp = String(va).localeCompare(String(vb));
          if (cmp !== 0) return sc.asc ? cmp : -cmp;
        }
        return 0;
      });
    } else if (sortCol >= 0) {
      rows = rows.slice().sort(function(a, b) {
        var va = a.row[sortCol], vb = b.row[sortCol];
        if (typeof va === "number" && typeof vb === "number") return sortAsc ? va - vb : vb - va;
        return sortAsc ? String(va).localeCompare(String(vb)) : String(vb).localeCompare(String(va));
      });
    }
    var pageStart = currentPage * pageSize;
    var actualIdx = pageStart + rowVisIdx;
    if (actualIdx >= rows.length) return;
    var rowIdx = rows[actualIdx].idx;

    var oldValue = String(f.parsed.rows[rowIdx][colIdx] || "");
    var rect = td.getBoundingClientRect();

    var input = document.createElement("input");
    input.type = "text";
    input.value = oldValue;
    input.style.cssText = "position:fixed;z-index:400;left:" + rect.left + "px;top:" + rect.top + "px;width:" + Math.max(rect.width, 60) + "px;height:" + rect.height + "px;font-family:var(--vw-mono);font-size:12px;padding:2px 6px;border:2px solid var(--vw-accent);background:var(--vw-panel);color:var(--vw-text);outline:none;box-sizing:border-box;";
    document.body.appendChild(input);
    input.focus();
    input.select();

    function commitEdit() {
      var newValue = input.value;
      if (newValue !== oldValue) {
        var colType = f.parsed.colTypes[colIdx];
        if (colType === "num") {
          var num = parseFloat(newValue);
          f.parsed.rows[rowIdx][colIdx] = isNaN(num) ? newValue : num;
        } else {
          f.parsed.rows[rowIdx][colIdx] = newValue;
        }
        modifiedCells.add(rowIdx + ":" + colIdx);
        f.parsed._summaryCache = null;
        _cachedFile = null;
        _filterCache = null;
        td.style.borderLeft = "3px solid var(--vw-amber)";
      }
      input.remove();
      // Update the cell text without full re-render
      if (newValue !== oldValue) {
        td.textContent = newValue;
      }
    }

    input.addEventListener("blur", commitEdit);
    input.addEventListener("keydown", function(ev) {
      if (ev.key === "Enter") { ev.preventDefault(); commitEdit(); }
      else if (ev.key === "Escape") { input.remove(); }
    });
  });

  // ── Feature 1 (enhanced): Update theme toggle to save as vw-theme ──
  // Patch the existing theme toggle to also save vw-theme
  (function() {
    var themeBtn = document.getElementById("vw-theme-toggle");
    if (!themeBtn) return;
    // Remove old listener by replacing the node
    var newBtn = themeBtn.cloneNode(true);
    themeBtn.parentNode.replaceChild(newBtn, themeBtn);
    newBtn.addEventListener("click", function() {
      var isDark = document.documentElement.classList.contains("dark");
      if (isDark) {
        document.documentElement.classList.remove("dark");
        localStorage.setItem("vw-theme", "light");
        localStorage.setItem("theme", "light");
      } else {
        document.documentElement.classList.add("dark");
        localStorage.setItem("vw-theme", "dark");
        localStorage.setItem("theme", "dark");
      }
    });
  })();

  // ── Feature 9 & 11 & 14: Add toolbar buttons ──────────────────
  (function() {
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (!rightGroup) return;

    // Feature 9: Views button
    var viewsBtn = document.createElement("button");
    viewsBtn.className = "vw-tbtn";
    viewsBtn.innerHTML = "&#128194; Views";
    viewsBtn.title = "Save/restore view presets";
    viewsBtn.id = "vw-views-btn";
    viewsBtn.addEventListener("click", function() { showViewsDropdown(viewsBtn); });
    rightGroup.insertBefore(viewsBtn, rightGroup.firstChild);

    // Feature 11: VCF INFO expand button
    var infoBtn = document.createElement("button");
    infoBtn.className = "vw-tbtn";
    infoBtn.innerHTML = "&#9776; Expand INFO";
    infoBtn.title = "Parse VCF INFO column into separate columns";
    infoBtn.id = "vw-info-btn";
    infoBtn.style.display = "none";
    infoBtn.addEventListener("click", function() {
      var f = files[activeTab];
      if (f) expandVcfInfo(f);
    });
    rightGroup.insertBefore(infoBtn, rightGroup.firstChild);
  })();

  // Show/hide INFO button based on format
  var origUpdateToolbar = updateToolbar;
  updateToolbar = function() {
    origUpdateToolbar();
    var f = files[activeTab];
    if (!f) return;
    var infoBtn = document.getElementById("vw-info-btn");
    if (infoBtn) infoBtn.style.display = f.parsed.format === "vcf" ? "" : "none";

    // Feature 14: Coordinate jump input
    var existingCoord = document.getElementById("vw-coord-input");
    var existingSep = document.getElementById("vw-coord-sep");
    if (existingCoord) existingCoord.remove();
    if (existingSep) existingSep.remove();
    var coordFormats = { vcf: 1, bed: 1, gff: 1, sam: 1 };
    if (coordFormats[f.parsed.format]) setupCoordJump();
  };

  // ── Feature 13: Auto-offer GC% on file load ────────────────────
  var origAddFile = addFile;
  addFile = function(name, size, text, truncated, fileRef) {
    origAddFile(name, size, text, truncated, fileRef);
    var f = files[files.length - 1];
    if (f && (f.parsed.format === "fasta" || f.parsed.format === "fastq")) {
      setTimeout(function() { showGcOffer(f); }, 300);
    }
  };

  // ── Feature 5: Full-Screen Mode ──────────────────────────────
  (function() {
    var fsBtn = document.createElement("button");
    fsBtn.className = "vw-tbtn";
    fsBtn.innerHTML = "\u26F6 Fullscreen";
    fsBtn.title = "Toggle fullscreen (F11)";
    fsBtn.id = "vw-fullscreen-btn";
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(fsBtn, rightGroup.firstChild);

    var exitFsBtn = document.createElement("button");
    exitFsBtn.textContent = "Exit fullscreen";
    exitFsBtn.style.cssText = "position:fixed;top:8px;right:8px;z-index:10001;display:none;padding:4px 12px;border-radius:6px;border:1px solid var(--vw-border);background:var(--vw-panel);color:var(--vw-text);cursor:pointer;font-size:11px;font-family:var(--vw-sans);opacity:0.7;";
    exitFsBtn.id = "vw-exit-fs-btn";
    document.body.appendChild(exitFsBtn);

    function toggleFullscreen() {
      isFullscreen = !isFullscreen;
      if (isFullscreen) {
        workspace.classList.add("vw-fullscreen");
        exitFsBtn.style.display = "";
        fsBtn.classList.add("active");
      } else {
        workspace.classList.remove("vw-fullscreen");
        exitFsBtn.style.display = "none";
        fsBtn.classList.remove("active");
      }
    }

    fsBtn.addEventListener("click", toggleFullscreen);
    exitFsBtn.addEventListener("click", toggleFullscreen);

    document.addEventListener("keydown", function(e) {
      if (e.key === "F11" && activeTab >= 0) {
        e.preventDefault();
        toggleFullscreen();
      }
    });

    // Inject fullscreen CSS
    var fsStyle = document.createElement("style");
    fsStyle.textContent =
      ".vw-fullscreen { position:fixed!important;top:0!important;left:0!important;right:0!important;bottom:0!important;z-index:10000!important;margin:0!important;border-radius:0!important; }" +
      ".vw-fullscreen #vw-toolbar,.vw-fullscreen .vw-footer { display:none!important; }" +
      ".vw-fullscreen #vw-content { height:calc(100vh - 40px)!important; }";
    document.head.appendChild(fsStyle);
  })();

  // ── Feature 6: Row Number Toggle Button ────────────────────────
  (function() {
    var btn = document.createElement("button");
    btn.className = "vw-tbtn" + (showRowNumbers ? " active" : "");
    btn.innerHTML = "# Rows";
    btn.title = "Toggle row numbers";
    btn.id = "vw-rownum-toggle";
    btn.addEventListener("click", function() {
      showRowNumbers = !showRowNumbers;
      btn.classList.toggle("active", showRowNumbers);
      if (currentView === "table") renderView();
    });
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(btn, rightGroup.firstChild);
  })();

  // ── Feature 8: Unique Values Popup ─────────────────────────────
  function showUniqueValuesPopup(f, colIndex, colName) {
    var existing = document.getElementById("vw-unique-popup");
    if (existing) existing.remove();

    var filtered = getFilteredRows(f);
    var counts = new Map();
    filtered.forEach(function(item) {
      var v = String(item.row[colIndex]);
      counts.set(v, (counts.get(v) || 0) + 1);
    });

    var sorted = Array.from(counts.entries()).sort(function(a, b) { return b[1] - a[1]; });
    var top50 = sorted.slice(0, 50);

    var overlay = document.createElement("div");
    overlay.id = "vw-unique-popup";
    overlay.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:16px;box-shadow:0 12px 40px rgba(0,0,0,0.5);width:360px;max-height:500px;overflow-y:auto;font-family:var(--vw-sans);";

    overlay.innerHTML =
      '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px">' +
        '<span style="font-weight:600;color:var(--vw-accent);font-size:13px">' + escapeHtml(colName) + ' — ' + counts.size + ' unique values</span>' +
        '<span style="cursor:pointer;color:var(--vw-text-dim);font-size:16px" id="vw-unique-close">&times;</span>' +
      '</div>';

    var list = document.createElement("div");
    top50.forEach(function(entry) {
      var row = document.createElement("div");
      row.style.cssText = "display:flex;justify-content:space-between;padding:3px 6px;font-size:12px;border-bottom:1px solid var(--vw-border);";
      row.innerHTML = '<span style="color:var(--vw-text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:250px" title="' + escapeHtml(entry[0]) + '">' + escapeHtml(entry[0]) + '</span>' +
        '<span style="color:var(--vw-text-muted);flex-shrink:0;margin-left:8px">' + entry[1].toLocaleString() + '</span>';
      list.appendChild(row);
    });
    if (sorted.length > 50) {
      var more = document.createElement("div");
      more.style.cssText = "padding:6px;font-size:11px;color:var(--vw-text-dim);text-align:center;";
      more.textContent = "... and " + (sorted.length - 50) + " more";
      list.appendChild(more);
    }
    overlay.appendChild(list);
    document.body.appendChild(overlay);

    document.getElementById("vw-unique-close").addEventListener("click", function() { overlay.remove(); });
    function closeOverlay(ev) {
      if (!overlay.contains(ev.target)) { overlay.remove(); document.removeEventListener("mousedown", closeOverlay); }
    }
    setTimeout(function() { document.addEventListener("mousedown", closeOverlay); }, 0);
  }

  // ── Feature 9: Column Auto-Fit on double-click resizer ─────────
  (function() {
    // Inject handler via event delegation on table headers
    document.addEventListener("dblclick", function(e) {
      if (!e.target.classList.contains("col-resizer")) return;
      var th = e.target.closest("th");
      if (!th) return;
      var f = files[activeTab];
      if (!f) return;

      var colIdx = Array.from(th.parentNode.children).indexOf(th) - 1 - (bookmarkMode ? 1 : 0);
      if (colIdx < 0 || colIdx >= f.parsed.columns.length) return;

      // Measure widest value on current page using canvas
      var canvas = document.createElement("canvas");
      var ctx = canvas.getContext("2d");
      ctx.font = "12px monospace";

      var maxW = ctx.measureText(f.parsed.columns[colIdx]).width;
      var tbody = document.querySelector(".vw-table tbody");
      if (tbody) {
        var trs = tbody.querySelectorAll("tr:not(.vw-group-row)");
        var offset = bookmarkMode ? 2 : 1;
        for (var i = 0; i < trs.length; i++) {
          var cells = trs[i].querySelectorAll("td");
          if (cells[colIdx + offset]) {
            var w = ctx.measureText(cells[colIdx + offset].textContent.substring(0, 100)).width;
            if (w > maxW) maxW = w;
          }
        }
      }

      var finalW = Math.max(60, Math.min(maxW + 24, 600));
      th.style.width = finalW + "px";
      th.style.minWidth = finalW + "px";
    });
  })();

  // ── Feature 10: Text Wrap Toggle Button ────────────────────────
  (function() {
    var btn = document.createElement("button");
    btn.className = "vw-tbtn";
    btn.innerHTML = "Wrap";
    btn.title = "Toggle text wrapping in cells";
    btn.id = "vw-wrap-toggle";
    btn.addEventListener("click", function() {
      textWrap = !textWrap;
      btn.classList.toggle("active", textWrap);
      if (currentView === "table") renderView();
    });
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(btn, rightGroup.firstChild);
  })();

  // ── Feature 11: Numeric Formatting Toggle Button ────────────────
  (function() {
    var btn = document.createElement("button");
    btn.className = "vw-tbtn";
    btn.innerHTML = "1,234";
    btn.title = "Toggle number formatting with locale separators";
    btn.id = "vw-numfmt-toggle";
    btn.addEventListener("click", function() {
      formatNumbers = !formatNumbers;
      btn.classList.toggle("active", formatNumbers);
      if (currentView === "table") renderView();
    });
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(btn, rightGroup.firstChild);
  })();

  // ── Feature 16: Regex Search Toggle ─────────────────────────────
  (function() {
    var btn = document.createElement("button");
    btn.className = "vw-tbtn";
    btn.innerHTML = ".*";
    btn.title = "Toggle regex search mode";
    btn.id = "vw-regex-toggle";
    btn.style.cssText += "font-family:var(--vw-mono);font-size:11px;padding:2px 6px;min-width:unset;";

    btn.addEventListener("click", function() {
      regexSearch = !regexSearch;
      btn.classList.toggle("active", regexSearch);
      // Validate current search term as regex
      if (regexSearch && searchTerm) {
        try {
          new RegExp(searchTerm, "i");
          searchInput.style.borderColor = "";
        } catch (e) {
          searchInput.style.borderColor = "var(--vw-red)";
        }
      } else {
        searchInput.style.borderColor = "";
      }
      _filterCache = null;
      if (activeTab >= 0) renderView();
    });

    // Insert next to search input
    var searchWrap = searchInput.parentNode;
    if (searchWrap) searchWrap.insertBefore(btn, searchInput.nextSibling);
    else {
      var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
      if (rightGroup) rightGroup.insertBefore(btn, rightGroup.firstChild);
    }

    // Validate regex on input
    searchInput.addEventListener("input", function() {
      if (regexSearch && searchInput.value) {
        try {
          new RegExp(searchInput.value, "i");
          searchInput.style.borderColor = "";
        } catch (e) {
          searchInput.style.borderColor = "var(--vw-red)";
        }
      } else {
        searchInput.style.borderColor = "";
      }
    });
  })();

  // ── Feature 17: Bookmark Filter Toggle ──────────────────────────
  (function() {
    var btn = document.createElement("button");
    btn.className = "vw-tbtn";
    btn.innerHTML = "\u2605 Starred only";
    btn.title = "Show only bookmarked/starred rows";
    btn.id = "vw-bookmark-filter";
    btn.style.display = "none"; // show only when bookmarks exist
    btn.addEventListener("click", function() {
      bookmarkFilterActive = !bookmarkFilterActive;
      btn.classList.toggle("active", bookmarkFilterActive);
      _filterCache = null;
      currentPage = 0;
      if (activeTab >= 0) renderView();
    });
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(btn, rightGroup.firstChild);

    // Show/hide based on bookmark existence - patch into renderView
    var origRenderView = renderView;
    renderView = function() {
      origRenderView();
      var bms = bookmarkedRows[activeTab];
      btn.style.display = (bms && bms.size > 0) ? "" : "none";
    };
  })();

  // ── Feature 18: Export as HTML ─────────────────────────────────
  // Inject into export menu by patching the export button handler
  (function() {
    var origHandler = document.getElementById("vw-export-btn");
    if (!origHandler) return;
    // We'll add "Export as HTML" item to the export menu by monkey-patching
    var origClick = origHandler.onclick; // may be null, event listener used instead
    // Add via MutationObserver on the export menu creation
    var observer = new MutationObserver(function(mutations) {
      mutations.forEach(function(m) {
        m.addedNodes.forEach(function(node) {
          if (node.id === "vw-export-menu" && !node.dataset.htmlAdded) {
            node.dataset.htmlAdded = "1";
            var htmlItem = document.createElement("div");
            htmlItem.style.cssText = "padding:6px 16px;cursor:pointer;border-radius:4px;color:var(--vw-accent);";
            htmlItem.textContent = "Export as HTML";
            htmlItem.addEventListener("mouseenter", function() { htmlItem.style.background = "var(--vw-row-hover)"; });
            htmlItem.addEventListener("mouseleave", function() { htmlItem.style.background = ""; });
            htmlItem.addEventListener("click", function() {
              exportAsHtml();
              node.remove();
            });
            node.appendChild(htmlItem);
          }
        });
      });
    });
    observer.observe(document.body, { childList: true });
  })();

  function exportAsHtml() {
    var f = files[activeTab];
    if (!f) return;
    var rows = getFilteredRows(f);
    var ps = currentPage * pageSize;
    var pageRows = rows.slice(ps, ps + pageSize);

    var html = '<!DOCTYPE html><html><head><meta charset="utf-8"><title>' + escapeHtml(f.displayName || f.name) + '</title>';
    html += '<style>';
    html += 'body{font-family:system-ui,sans-serif;background:#1a1b26;color:#c0caf5;margin:20px}';
    html += 'table{border-collapse:collapse;width:100%;font-size:12px}';
    html += 'th{background:#24283b;padding:6px 10px;text-align:left;border-bottom:2px solid #414868;font-weight:600}';
    html += 'td{padding:4px 10px;border-bottom:1px solid #2a2e3f}';
    html += 'tr:hover{background:#292e42}';
    html += '.nt-A{color:#34d399}.nt-T{color:#f87171}.nt-C{color:#60a5fa}.nt-G{color:#fbbf24}.nt-N{color:#6b7280}';
    html += '.num-cell{text-align:right;font-family:monospace}';
    html += 'h2{color:#7c3aed;font-size:16px;margin:0 0 12px}';
    html += '</style></head><body>';
    html += '<h2>' + escapeHtml(f.displayName || f.name) + ' — ' + f.parsed.format.toUpperCase() + '</h2>';
    html += '<p style="color:#565f89;font-size:11px">' + f.parsed.rows.length.toLocaleString() + ' total rows, showing ' + pageRows.length + ' rows</p>';
    html += '<table><thead><tr>';
    html += '<th>#</th>';
    f.parsed.columns.forEach(function(col) { html += '<th>' + escapeHtml(col) + '</th>'; });
    html += '</tr></thead><tbody>';
    pageRows.forEach(function(item) {
      html += '<tr><td class="num-cell">' + (item.idx + 1) + '</td>';
      item.row.forEach(function(val, ci) {
        var colType = f.parsed.colTypes[ci];
        if (colType === "seq") {
          html += '<td>';
          var seq = String(val).substring(0, 120);
          for (var si = 0; si < seq.length; si++) {
            var ch = seq.charAt(si).toUpperCase();
            if ("ATCGUN".indexOf(ch) >= 0) html += '<span class="nt-' + (ch === "U" ? "T" : ch) + '">' + seq.charAt(si) + '</span>';
            else html += seq.charAt(si);
          }
          html += '</td>';
        } else if (colType === "num") {
          html += '<td class="num-cell">' + escapeHtml(String(val)) + '</td>';
        } else {
          html += '<td>' + escapeHtml(String(val).substring(0, 200)) + '</td>';
        }
      });
      html += '</tr>';
    });
    html += '</tbody></table>';
    html += '<p style="color:#565f89;font-size:10px;margin-top:16px">Generated by BLViewer</p>';
    html += '</body></html>';

    var blob = new Blob([html], { type: "text/html" });
    downloadBlob(blob, (f.displayName || f.name).replace(/\.[^.]+$/, "") + ".html");
    showToast("Exported " + pageRows.length + " rows as HTML");
  }

  // Feature 19 split view uses the existing splitMode button — no duplicate needed.

  // Patch renderView for split view (Feature 19)
  var _origRenderViewForSplit = renderView;
  renderView = function() {
    if (!splitViewMode || currentView !== "table" || files.length < 2 || activeTab < 0) {
      _origRenderViewForSplit();
      return;
    }

    // Split view: two panes side by side
    contentEl.innerHTML = "";
    selectedRows.clear();
    var f = files[activeTab];
    if (!f) return;

    var container = document.createElement("div");
    container.style.cssText = "display:flex;height:100%;gap:2px;";

    // Left pane — current tab
    var leftPane = document.createElement("div");
    leftPane.style.cssText = "flex:1;overflow:auto;border-right:2px solid var(--vw-border);";

    // Right pane — selected tab
    var rightPane = document.createElement("div");
    rightPane.style.cssText = "flex:1;overflow:auto;";

    container.appendChild(leftPane);
    container.appendChild(rightPane);
    contentEl.appendChild(container);

    // Tab selector for right pane
    var selector = document.createElement("select");
    selector.style.cssText = "position:absolute;top:4px;right:8px;z-index:10;background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:4px;padding:2px 6px;color:var(--vw-text);font-size:11px;font-family:var(--vw-sans);";
    files.forEach(function(file, idx) {
      var opt = document.createElement("option");
      opt.value = idx;
      opt.textContent = file.displayName || file.name;
      if (idx === splitViewTab) opt.selected = true;
      selector.appendChild(opt);
    });
    selector.addEventListener("change", function() {
      splitViewTab = parseInt(this.value);
      renderView();
    });
    rightPane.style.position = "relative";
    rightPane.appendChild(selector);

    // Render left pane
    var origContentEl = contentEl;
    contentEl = leftPane;
    renderTableView(f);
    contentEl = origContentEl;

    // Render right pane
    if (splitViewTab >= 0 && splitViewTab < files.length) {
      var f2 = files[splitViewTab];
      var rightContent = document.createElement("div");
      rightContent.style.cssText = "margin-top:28px;";
      rightPane.appendChild(rightContent);

      contentEl = rightContent;
      // Save/restore state for right pane
      var savedSort = sortCol, savedAsc = sortAsc, savedSortCols = sortCols.slice();
      var savedPage = currentPage, savedActive = activeTab;
      sortCol = -1; sortAsc = true; sortCols = []; currentPage = 0;
      renderTableView(f2);
      sortCol = savedSort; sortAsc = savedAsc; sortCols = savedSortCols;
      currentPage = savedPage; activeTab = savedActive;
      contentEl = origContentEl;
    }

    updateFooter(f);
  };

  // ── Feature 20: Goto Row Input near pagination ──────────────────
  // Inject CSS for row highlight animation
  (function() {
    var style = document.createElement("style");
    style.textContent = "@keyframes vw-row-flash{0%{background:rgba(139,92,246,0.3)}100%{background:transparent}}.vw-row-flash{animation:vw-row-flash 1.5s ease-out}";
    document.head.appendChild(style);
  })();

  // Patch renderTableView to add goto-row input in pagination
  var _origRenderTableView = renderTableView;
  renderTableView = function(f) {
    _origRenderTableView(f);

    // Find pagination bar and add goto-row input
    var pagBar = contentEl.querySelector(".vw-pagination");
    if (!pagBar) return;

    var gotoLabel = document.createElement("span");
    gotoLabel.textContent = "Row:";
    gotoLabel.style.cssText = "margin-left:12px;color:var(--vw-text-dim);font-size:11px;";
    pagBar.appendChild(gotoLabel);

    var gotoInput = document.createElement("input");
    gotoInput.type = "number";
    gotoInput.min = 1;
    gotoInput.max = f.parsed.rows.length;
    gotoInput.placeholder = "Go to row...";
    gotoInput.style.cssText = "width:80px;background:var(--vw-tab-bg);border:1px solid var(--vw-border);border-radius:4px;padding:2px 4px;color:var(--vw-text);font-size:11px;text-align:center;";
    gotoInput.addEventListener("keydown", function(e) {
      if (e.key === "Enter") {
        var rowNum = parseInt(gotoInput.value);
        if (isNaN(rowNum) || rowNum < 1 || rowNum > f.parsed.rows.length) {
          showToast("Invalid row number (1-" + f.parsed.rows.length + ")");
          return;
        }
        // Find the page containing this row
        var targetPage = Math.floor((rowNum - 1) / pageSize);
        if (targetPage !== currentPage) {
          currentPage = targetPage;
          renderView();
          // After re-render, flash the row
          setTimeout(function() {
            var rowInPage = (rowNum - 1) % pageSize;
            var tbody = document.querySelector(".vw-table tbody");
            if (tbody) {
              var trs = tbody.querySelectorAll("tr:not(.vw-group-row)");
              if (trs[rowInPage]) {
                trs[rowInPage].classList.add("vw-row-flash");
                trs[rowInPage].scrollIntoView({ block: "center" });
              }
            }
          }, 50);
        } else {
          // Same page, just flash
          var rowInPage = (rowNum - 1) % pageSize;
          var tbody = document.querySelector(".vw-table tbody");
          if (tbody) {
            var trs = tbody.querySelectorAll("tr:not(.vw-group-row)");
            if (trs[rowInPage]) {
              trs[rowInPage].classList.add("vw-row-flash");
              trs[rowInPage].scrollIntoView({ block: "center" });
            }
          }
        }
      }
    });
    pagBar.appendChild(gotoInput);
  };

  // Initialize history + session restore on load
  renderRecentFiles();
  // Delay session restore check to let URL/launchQueue load first
  setTimeout(checkSessionRestore, 500);

  // ══════════════════════════════════════════════════════════════════
  // New Features (lazy, IIFE-wrapped, zero load-time impact)
  // ══════════════════════════════════════════════════════════════════

  // ── Feature N1: Conditional Formatting (cell color rules) ──────
  (function() {
    var COND_COLORS = {
      red: "rgba(248,113,113,0.3)",
      green: "rgba(52,211,153,0.3)",
      blue: "rgba(96,165,250,0.3)",
      amber: "rgba(251,191,36,0.3)",
      purple: "rgba(167,139,250,0.3)"
    };

    // Inject "Conditional Format" button into showColumnFilter
    var _origShowColumnFilter = showColumnFilter;
    showColumnFilter = function(f, colIndex, colName, anchorEl) {
      _origShowColumnFilter(f, colIndex, colName, anchorEl);
      var drop = document.getElementById("vw-col-filter");
      if (!drop) return;
      // Find the actions div (second child with flex layout)
      var actionsDiv = drop.querySelectorAll("div")[1];
      if (!actionsDiv || actionsDiv.querySelector(".vw-cond-fmt-btn")) return;

      var cfBtn = document.createElement("button");
      cfBtn.className = "vw-cond-fmt-btn";
      cfBtn.textContent = "Conditional Format";
      cfBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-amber);cursor:pointer;font-size:10px;";
      cfBtn.addEventListener("click", function() {
        drop.remove();
        showConditionalFormatDialog(f, colIndex, colName);
      });
      actionsDiv.appendChild(cfBtn);
    };

    function showConditionalFormatDialog(f, colIndex, colName) {
      var existing = document.getElementById("vw-cond-fmt-dlg");
      if (existing) existing.remove();

      if (!f.conditionalRules) f.conditionalRules = [];

      var dlg = document.createElement("div");
      dlg.id = "vw-cond-fmt-dlg";
      dlg.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:20px;box-shadow:0 12px 40px rgba(0,0,0,0.5);width:360px;font-family:var(--vw-sans);font-size:13px;";

      dlg.innerHTML =
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">' +
        '<span style="font-weight:600;color:var(--vw-accent)">Conditional Format: ' + escapeHtml(colName) + '</span>' +
        '<button id="vw-cond-close" style="background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:18px">&times;</button></div>' +
        '<div style="display:flex;gap:6px;margin-bottom:10px;flex-wrap:wrap">' +
        '<select id="vw-cond-op" style="background:var(--vw-bg);border:1px solid var(--vw-border);border-radius:4px;padding:4px 8px;color:var(--vw-text);font-size:12px">' +
        '<option value="gt">&gt;</option><option value="lt">&lt;</option><option value="eq">==</option><option value="neq">!=</option><option value="gte">&gt;=</option><option value="lte">&lt;=</option><option value="contains">contains</option><option value="startsWith">starts with</option><option value="empty">is empty</option></select>' +
        '<input id="vw-cond-val" placeholder="value" style="flex:1;background:var(--vw-bg);border:1px solid var(--vw-border);border-radius:4px;padding:4px 8px;color:var(--vw-text);font-size:12px;min-width:80px">' +
        '</div>' +
        '<div style="display:flex;gap:6px;margin-bottom:12px">' +
        Object.keys(COND_COLORS).map(function(c) {
          return '<button class="vw-cond-color-btn" data-color="' + c + '" style="width:28px;height:28px;border-radius:50%;border:2px solid transparent;cursor:pointer;background:' + COND_COLORS[c].replace("0.3", "0.8") + '"></button>';
        }).join("") +
        '</div>' +
        '<div style="display:flex;gap:8px">' +
        '<button id="vw-cond-add" style="flex:1;background:var(--vw-accent);color:white;border:none;border-radius:6px;padding:6px;cursor:pointer;font-size:12px">Add Rule</button>' +
        '<button id="vw-cond-clear" style="background:none;border:1px solid var(--vw-border);border-radius:6px;padding:6px 12px;color:var(--vw-text-dim);cursor:pointer;font-size:12px">Clear All</button>' +
        '</div>' +
        '<div id="vw-cond-rules-list" style="margin-top:10px;max-height:120px;overflow:auto"></div>';

      document.body.appendChild(dlg);

      var selectedColor = "red";
      dlg.querySelectorAll(".vw-cond-color-btn").forEach(function(btn) {
        btn.addEventListener("click", function() {
          dlg.querySelectorAll(".vw-cond-color-btn").forEach(function(b) { b.style.borderColor = "transparent"; });
          btn.style.borderColor = "var(--vw-text)";
          selectedColor = btn.getAttribute("data-color");
        });
      });
      // Default first selected
      dlg.querySelector('.vw-cond-color-btn[data-color="red"]').style.borderColor = "var(--vw-text)";

      function refreshRulesList() {
        var list = document.getElementById("vw-cond-rules-list");
        if (!list) return;
        var rules = (f.conditionalRules || []).filter(function(r) { return r.col === colIndex; });
        if (rules.length === 0) { list.innerHTML = '<span style="color:var(--vw-text-muted);font-size:11px">No rules set</span>'; return; }
        list.innerHTML = rules.map(function(r, i) {
          return '<div style="display:flex;align-items:center;gap:6px;padding:3px 0;font-size:11px;color:var(--vw-text-dim)">' +
            '<span style="width:10px;height:10px;border-radius:50%;background:' + (COND_COLORS[r.color] || COND_COLORS.red) + ';display:inline-block"></span>' +
            '<span>' + escapeHtml(colName) + ' ' + r.op + ' ' + escapeHtml(String(r.value)) + '</span></div>';
        }).join("");
      }
      refreshRulesList();

      document.getElementById("vw-cond-close").addEventListener("click", function() { dlg.remove(); });
      document.getElementById("vw-cond-add").addEventListener("click", function() {
        var op = document.getElementById("vw-cond-op").value;
        var val = document.getElementById("vw-cond-val").value;
        if (!f.conditionalRules) f.conditionalRules = [];
        f.conditionalRules.push({ col: colIndex, op: op, value: val, color: selectedColor });
        f.parsed._summaryCache = null;
        refreshRulesList();
        renderView();
      });
      document.getElementById("vw-cond-clear").addEventListener("click", function() {
        f.conditionalRules = (f.conditionalRules || []).filter(function(r) { return r.col !== colIndex; });
        f.parsed._summaryCache = null;
        refreshRulesList();
        renderView();
      });

      // Close on Escape
      document.addEventListener("keydown", function handler(e) {
        if (e.key === "Escape") { dlg.remove(); document.removeEventListener("keydown", handler); }
      });
    }

    // Patch renderTableView to apply conditional formatting to cells
    var _origRenderTableView = renderTableView;
    renderTableView = function(f) {
      _origRenderTableView(f);
      if (!f.conditionalRules || f.conditionalRules.length === 0) return;
      // Apply rules to visible cells via DOM traversal (lazy — only on rendered cells)
      var tbody = contentEl.querySelector(".vw-table tbody");
      if (!tbody) return;
      var trs = tbody.querySelectorAll("tr:not(.vw-group-row)");
      trs.forEach(function(tr) {
        var tds = tr.querySelectorAll("td");
        // Offset: bookmark col + row number col
        var offset = (bookmarkMode ? 1 : 0) + 1;
        f.conditionalRules.forEach(function(rule) {
          var tdIdx = rule.col + offset;
          var td = tds[tdIdx];
          if (!td) return;
          var cellText = td.textContent;
          var cellNum = parseFloat(cellText);
          var match = false;
          switch (rule.op) {
            case "gt": match = !isNaN(cellNum) && cellNum > parseFloat(rule.value); break;
            case "lt": match = !isNaN(cellNum) && cellNum < parseFloat(rule.value); break;
            case "eq": match = cellText === rule.value || (!isNaN(cellNum) && cellNum === parseFloat(rule.value)); break;
            case "neq": match = cellText !== rule.value; break;
            case "gte": match = !isNaN(cellNum) && cellNum >= parseFloat(rule.value); break;
            case "lte": match = !isNaN(cellNum) && cellNum <= parseFloat(rule.value); break;
            case "contains": match = cellText.toLowerCase().indexOf(rule.value.toLowerCase()) !== -1; break;
            case "startsWith": match = cellText.toLowerCase().indexOf(rule.value.toLowerCase()) === 0; break;
            case "empty": match = cellText.trim() === ""; break;
          }
          if (match) {
            td.style.background = COND_COLORS[rule.color] || COND_COLORS.red;
          }
        });
      });
    };
  })();

  // ── Feature N2: Find & Replace ─────────────────────────────────
  (function() {
    var replaceVisible = false;
    var replaceWrap = null;

    function ensureReplaceUI() {
      if (replaceWrap) return;
      // Find the search input parent
      var searchParent = searchInput.parentElement;
      if (!searchParent) return;

      // Toggle button next to search
      var toggleBtn = document.createElement("button");
      toggleBtn.id = "vw-replace-toggle";
      toggleBtn.textContent = "Replace";
      toggleBtn.title = "Toggle Find & Replace";
      toggleBtn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-text-dim);cursor:pointer;font-size:10px;margin-left:4px;";
      searchParent.appendChild(toggleBtn);

      replaceWrap = document.createElement("div");
      replaceWrap.id = "vw-replace-bar";
      replaceWrap.style.cssText = "display:none;align-items:center;gap:6px;padding:4px 12px;background:var(--vw-tab-bg);border-bottom:1px solid var(--vw-border);font-family:var(--vw-sans);font-size:12px;";
      replaceWrap.innerHTML =
        '<input id="vw-replace-input" placeholder="Replace with..." style="background:var(--vw-bg);border:1px solid var(--vw-border);border-radius:4px;padding:4px 8px;color:var(--vw-text);font-size:12px;width:180px">' +
        '<button id="vw-replace-one" style="background:none;border:1px solid var(--vw-border);border-radius:4px;padding:3px 10px;color:var(--vw-text);cursor:pointer;font-size:11px">Replace</button>' +
        '<button id="vw-replace-all" style="background:none;border:1px solid var(--vw-border);border-radius:4px;padding:3px 10px;color:var(--vw-accent);cursor:pointer;font-size:11px">Replace All</button>';
      searchParent.parentElement.insertBefore(replaceWrap, searchParent.nextSibling);

      toggleBtn.addEventListener("click", function() {
        replaceVisible = !replaceVisible;
        replaceWrap.style.display = replaceVisible ? "flex" : "none";
        toggleBtn.style.color = replaceVisible ? "var(--vw-accent)" : "var(--vw-text-dim)";
        if (replaceVisible) document.getElementById("vw-replace-input").focus();
      });

      document.getElementById("vw-replace-one").addEventListener("click", function() {
        doReplace(false);
      });
      document.getElementById("vw-replace-all").addEventListener("click", function() {
        doReplace(true);
      });
    }

    function doReplace(all) {
      var f = files[activeTab];
      if (!f || !f.parsed) return;
      var find = searchInput.value.trim();
      var replaceVal = document.getElementById("vw-replace-input").value;
      if (!find) { showToast("Enter a search term first"); return; }

      pushUndo();
      var count = 0;
      var firstFound = false;
      for (var ri = 0; ri < f.parsed.rows.length; ri++) {
        for (var ci = 0; ci < f.parsed.columns.length; ci++) {
          if (f.parsed.colTypes[ci] === "seq" || f.parsed.colTypes[ci] === "qual") continue; // skip sequence/quality
          var val = String(f.parsed.rows[ri][ci]);
          if (val.indexOf(find) !== -1) {
            if (!all && firstFound) continue;
            var newVal = val.split(find).join(replaceVal);
            f.parsed.rows[ri][ci] = f.parsed.colTypes[ci] === "num" ? (isNaN(parseFloat(newVal)) ? newVal : parseFloat(newVal)) : newVal;
            count++;
            if (!all) { firstFound = true; break; }
          }
        }
        if (!all && firstFound) break;
      }

      if (count > 0) {
        f.parsed._summaryCache = null;
        f.parsed._colHints = null;
        _filterCache = null;
        renderView();
        showToast("Replaced " + count + " occurrence" + (count > 1 ? "s" : ""));
      } else {
        showToast("No matches found for \"" + find + "\"");
      }
    }

    // Lazily create UI on first search focus
    searchInput.addEventListener("focus", function onFirstFocus() {
      ensureReplaceUI();
      searchInput.removeEventListener("focus", onFirstFocus);
    });
  })();

  // ── Feature N3 + N11: Header Sparklines (tiny SVG bars in numeric column headers) ──
  (function() {
    // Patch renderTableView to inject sparklines after rendering
    var _origRenderTV3 = renderTableView;
    renderTableView = function(f) {
      _origRenderTV3(f);
      if (!f.parsed || !f.parsed.columns) return;

      // Lazily compute sparkline data
      if (!f.parsed._sparklines) {
        f.parsed._sparklines = {};
        f.parsed.columns.forEach(function(col, ci) {
          if (f.parsed.colTypes[ci] !== "num") return;
          var vals = [];
          var step = Math.max(1, Math.floor(f.parsed.rows.length / 1000));
          for (var ri = 0; ri < f.parsed.rows.length; ri += step) {
            var v = f.parsed.rows[ri][ci];
            if (typeof v === "number" && !isNaN(v)) vals.push(v);
          }
          if (vals.length < 3) return;

          // Compute 10-bin histogram
          var mn = Infinity, mx = -Infinity;
          for (var i = 0; i < vals.length; i++) { if (vals[i] < mn) mn = vals[i]; if (vals[i] > mx) mx = vals[i]; }
          if (mn === mx) return;
          var bins = new Array(10).fill(0);
          var binW = (mx - mn) / 10;
          for (var i = 0; i < vals.length; i++) {
            var b = Math.min(9, Math.floor((vals[i] - mn) / binW));
            bins[b]++;
          }
          f.parsed._sparklines[ci] = bins;
        });
      }

      // Inject SVG sparklines into th elements
      var ths = contentEl.querySelectorAll(".vw-table thead tr:first-child th");
      var offset = (bookmarkMode ? 1 : 0) + 1; // bookmark + row number cols
      f.parsed.columns.forEach(function(col, ci) {
        var bins = f.parsed._sparklines[ci];
        if (!bins) return;
        var th = ths[ci + offset];
        if (!th || th.querySelector(".vw-sparkline")) return;
        var maxBin = Math.max.apply(null, bins);
        if (maxBin === 0) return;

        var svg = '<svg class="vw-sparkline" width="50" height="16" style="display:inline-block;vertical-align:middle;margin-left:4px;opacity:0.7">';
        for (var i = 0; i < 10; i++) {
          var h = Math.round((bins[i] / maxBin) * 14) + 1;
          svg += '<rect x="' + (i * 5) + '" y="' + (16 - h) + '" width="4" height="' + h + '" fill="var(--vw-accent)" opacity="0.4" rx="0.5"/>';
        }
        svg += '</svg>';
        th.insertAdjacentHTML("beforeend", svg);
      });
    };
  })();

  // ── Feature N4: Diff Two Rows ──────────────────────────────────
  (function() {
    // Patch updateSelectionSummary to show Diff button when 2 rows selected
    var _origSelSum4 = updateSelectionSummary;
    updateSelectionSummary = function(f) {
      _origSelSum4(f);
      var existingDiff = document.getElementById("vw-diff-btn");
      if (existingDiff) existingDiff.remove();
      if (selectedRows.size !== 2) return;

      var selEl = document.getElementById("vw-footer-selection");
      if (!selEl) return;
      var btn = document.createElement("button");
      btn.id = "vw-diff-btn";
      btn.textContent = "Diff Rows";
      btn.style.cssText = "background:var(--vw-accent);color:white;border:none;border-radius:4px;padding:3px 10px;cursor:pointer;font-size:11px;margin-left:8px;";
      btn.addEventListener("click", function() { showRowDiff(f); });
      selEl.appendChild(btn);
    };

    function showRowDiff(f) {
      var indices = Array.from(selectedRows);
      if (indices.length !== 2) return;
      var rowA = f.parsed.rows[indices[0]];
      var rowB = f.parsed.rows[indices[1]];
      if (!rowA || !rowB) return;

      var existing = document.getElementById("vw-diff-overlay");
      if (existing) existing.remove();

      var overlay = document.createElement("div");
      overlay.id = "vw-diff-overlay";
      overlay.style.cssText = "position:fixed;top:0;left:0;width:100%;height:100%;z-index:350;background:rgba(0,0,0,0.6);display:flex;align-items:center;justify-content:center;";

      var panel = document.createElement("div");
      panel.style.cssText = "background:var(--vw-panel);border-radius:12px;padding:20px;max-width:700px;width:90%;max-height:80vh;overflow:auto;box-shadow:0 12px 40px rgba(0,0,0,0.5);font-family:var(--vw-sans);";

      var html = '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">' +
        '<span style="font-weight:600;color:var(--vw-accent);font-size:14px">Row Diff: #' + (indices[0] + 1) + ' vs #' + (indices[1] + 1) + '</span>' +
        '<button id="vw-diff-close" style="background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:20px">&times;</button></div>' +
        '<table style="width:100%;border-collapse:collapse;font-size:12px">' +
        '<thead><tr><th style="text-align:left;padding:6px 8px;color:var(--vw-text-dim);border-bottom:1px solid var(--vw-border)">Column</th>' +
        '<th style="text-align:left;padding:6px 8px;color:var(--vw-text-dim);border-bottom:1px solid var(--vw-border)">Row #' + (indices[0] + 1) + '</th>' +
        '<th style="text-align:left;padding:6px 8px;color:var(--vw-text-dim);border-bottom:1px solid var(--vw-border)">Row #' + (indices[1] + 1) + '</th></tr></thead><tbody>';

      f.parsed.columns.forEach(function(col, ci) {
        var vA = String(rowA[ci] != null ? rowA[ci] : "");
        var vB = String(rowB[ci] != null ? rowB[ci] : "");
        var differs = vA !== vB;
        var bgStyle = differs ? "background:rgba(251,191,36,0.15)" : "";
        var colorA = differs ? "color:var(--vw-red)" : "color:var(--vw-text)";
        var colorB = differs ? "color:var(--vw-green)" : "color:var(--vw-text)";
        html += '<tr style="' + bgStyle + '">' +
          '<td style="padding:5px 8px;font-weight:600;color:var(--vw-accent);border-bottom:1px solid var(--vw-border);white-space:nowrap">' + escapeHtml(col) + '</td>' +
          '<td style="padding:5px 8px;font-family:var(--vw-mono);font-size:11px;border-bottom:1px solid var(--vw-border);max-width:250px;overflow:hidden;text-overflow:ellipsis;' + colorA + '" title="' + escapeHtml(vA) + '">' + escapeHtml(vA.substring(0, 100)) + '</td>' +
          '<td style="padding:5px 8px;font-family:var(--vw-mono);font-size:11px;border-bottom:1px solid var(--vw-border);max-width:250px;overflow:hidden;text-overflow:ellipsis;' + colorB + '" title="' + escapeHtml(vB) + '">' + escapeHtml(vB.substring(0, 100)) + '</td></tr>';
      });
      html += '</tbody></table>';
      panel.innerHTML = html;
      overlay.appendChild(panel);
      document.body.appendChild(overlay);

      overlay.addEventListener("click", function(e) { if (e.target === overlay) overlay.remove(); });
      document.getElementById("vw-diff-close").addEventListener("click", function() { overlay.remove(); });
      document.addEventListener("keydown", function handler(e) {
        if (e.key === "Escape") { overlay.remove(); document.removeEventListener("keydown", handler); }
      });
    }
  })();

  // ── Feature N5: Enhanced Selection Stats in Footer ─────────────
  // (Already partially implemented above at line ~6033. This extends it to show
  //  stats for ALL numeric columns in the selection, not just the first one.)
  (function() {
    var _origRenderSelStats = renderSelectionStatsBar;
    renderSelectionStatsBar = function(f) {
      var existing = document.getElementById("vw-sel-stats-bar");
      if (existing) existing.remove();
      if (!f || selectedRows.size === 0) return;

      var numCols = [];
      for (var ci = 0; ci < f.parsed.colTypes.length; ci++) {
        if (f.parsed.colTypes[ci] === "num") numCols.push(ci);
      }
      if (numCols.length === 0) return;

      var bar = document.createElement("div");
      bar.id = "vw-sel-stats-bar";
      bar.style.cssText = "display:flex;align-items:center;gap:12px;padding:6px 12px;background:var(--vw-tab-bg);border-top:1px solid var(--vw-border);font-family:var(--vw-mono);font-size:11px;color:var(--vw-text-dim);flex-shrink:0;flex-wrap:wrap;";
      var fmt = function(n) { return n.toLocaleString(undefined, {maximumFractionDigits: 2}); };

      // Show stats for up to 5 numeric columns
      numCols.slice(0, 5).forEach(function(ci) {
        var count = 0, sum = 0, mn = Infinity, mx = -Infinity;
        selectedRows.forEach(function(idx) {
          var v = f.parsed.rows[idx] ? f.parsed.rows[idx][ci] : NaN;
          if (typeof v === "number" && !isNaN(v)) { sum += v; count++; if (v < mn) mn = v; if (v > mx) mx = v; }
        });
        if (count === 0) return;
        bar.innerHTML +=
          '<span style="color:var(--vw-accent);font-weight:600">' + escapeHtml(f.parsed.columns[ci]) + ':</span>' +
          '<span>n=' + count + '</span>' +
          '<span>\u03a3=' + fmt(sum) + '</span>' +
          '<span>\u03bc=' + fmt(sum / count) + '</span>' +
          '<span>[' + fmt(mn) + '\u2013' + fmt(mx) + ']</span>' +
          '<span style="margin-right:8px">|</span>';
      });

      var footer = document.querySelector(".vw-footer");
      if (footer) footer.parentNode.insertBefore(bar, footer);
    };
  })();

  // ── Feature N6: JSON Cell Expander ─────────────────────────────
  (function() {
    function isJsonLike(val) {
      var s = String(val).trim();
      return (s.charAt(0) === "{" || s.charAt(0) === "[") && s.length > 2;
    }

    function tryParseJson(val) {
      try { return JSON.parse(String(val)); } catch(e) { return null; }
    }

    function syntaxHighlight(json) {
      var str = JSON.stringify(json, null, 2);
      return escapeHtml(str)
        .replace(/"([^"]+)":/g, '<span style="color:var(--vw-cyan)">"$1"</span>:')
        .replace(/: "([^"]*)"/g, ': <span style="color:var(--vw-green)">"$1"</span>')
        .replace(/: (\d+\.?\d*)/g, ': <span style="color:var(--vw-amber)">$1</span>')
        .replace(/: (true|false|null)/g, ': <span style="color:var(--vw-accent)">$1</span>');
    }

    function showJsonPopup(x, y, jsonObj) {
      var existing = document.getElementById("vw-json-popup");
      if (existing) existing.remove();

      var popup = document.createElement("div");
      popup.id = "vw-json-popup";
      popup.style.cssText = "position:fixed;z-index:350;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:10px;padding:12px;box-shadow:0 8px 30px rgba(0,0,0,0.4);max-width:500px;max-height:400px;overflow:auto;font-family:var(--vw-mono);font-size:11px;line-height:1.5;";
      popup.style.left = Math.min(x, window.innerWidth - 520) + "px";
      popup.style.top = Math.min(y, window.innerHeight - 420) + "px";

      popup.innerHTML =
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">' +
        '<span style="font-family:var(--vw-sans);font-weight:600;color:var(--vw-accent);font-size:12px">JSON Cell</span>' +
        '<div><button id="vw-json-copy" style="background:none;border:none;color:var(--vw-text-dim);cursor:pointer;font-size:12px;margin-right:6px" title="Copy JSON">Copy</button>' +
        '<button id="vw-json-close" style="background:none;border:none;color:var(--vw-text-muted);cursor:pointer;font-size:16px">&times;</button></div></div>' +
        '<pre style="margin:0;white-space:pre-wrap;word-break:break-all;color:var(--vw-text)">' + syntaxHighlight(jsonObj) + '</pre>';

      document.body.appendChild(popup);

      document.getElementById("vw-json-close").addEventListener("click", function() { popup.remove(); });
      document.getElementById("vw-json-copy").addEventListener("click", function() {
        navigator.clipboard.writeText(JSON.stringify(jsonObj, null, 2));
        showToast("JSON copied to clipboard");
      });

      // Close on outside click
      setTimeout(function() {
        document.addEventListener("mousedown", function handler(ev) {
          if (!popup.contains(ev.target)) { popup.remove(); document.removeEventListener("mousedown", handler); }
        });
      }, 0);
      document.addEventListener("keydown", function handler(e) {
        if (e.key === "Escape") { popup.remove(); document.removeEventListener("keydown", handler); }
      });
    }

    // Listen for double-click on table cells
    document.addEventListener("dblclick", function(e) {
      var td = e.target.closest && e.target.closest("td");
      if (!td) return;
      var table = td.closest(".vw-table");
      if (!table) return;
      var text = td.textContent;
      if (!isJsonLike(text)) return;
      var json = tryParseJson(text);
      if (!json) return;
      e.preventDefault();
      e.stopPropagation();
      showJsonPopup(e.clientX, e.clientY, json);
    });
  })();

  // ── Feature N7: Copy Column Name ───────────────────────────────
  (function() {
    // Add "Copy column name" to the column filter dropdown
    var _origShowColumnFilter7 = showColumnFilter;
    showColumnFilter = function(f, colIndex, colName, anchorEl) {
      _origShowColumnFilter7(f, colIndex, colName, anchorEl);
      var drop = document.getElementById("vw-col-filter");
      if (!drop) return;
      var actionsDiv = drop.querySelectorAll("div")[1];
      if (!actionsDiv || actionsDiv.querySelector(".vw-copy-colname-btn")) return;

      var btn = document.createElement("button");
      btn.className = "vw-copy-colname-btn";
      btn.textContent = "Copy column name";
      btn.style.cssText = "background:none;border:1px solid var(--vw-border);border-radius:4px;padding:2px 8px;color:var(--vw-text);cursor:pointer;font-size:10px;";
      btn.addEventListener("click", function() {
        navigator.clipboard.writeText(colName);
        showToast("Copied: " + colName);
        drop.remove();
      });
      actionsDiv.appendChild(btn);
    };
  })();

  // ── Feature N8: Paste to Filter ────────────────────────────────
  (function() {
    var lastClickedCol = -1;

    // Track last clicked column header or cell
    document.addEventListener("click", function(e) {
      var th = e.target.closest && e.target.closest("th");
      if (th) {
        var allTh = Array.from(th.parentElement.querySelectorAll("th"));
        var idx = allTh.indexOf(th);
        var offset = (bookmarkMode ? 1 : 0) + 1;
        lastClickedCol = idx - offset;
        return;
      }
      var td = e.target.closest && e.target.closest("td");
      if (td && td.closest(".vw-table")) {
        var allTd = Array.from(td.parentElement.querySelectorAll("td"));
        var idx = allTd.indexOf(td);
        var offset = (bookmarkMode ? 1 : 0) + 1;
        lastClickedCol = idx - offset;
      }
    });

    document.addEventListener("paste", function(e) {
      // Only handle if focus is on table area (not in inputs)
      var active = document.activeElement;
      if (active && (active.tagName === "INPUT" || active.tagName === "TEXTAREA" || active.tagName === "SELECT")) return;

      var f = files[activeTab];
      if (!f || !f.parsed) return;

      var pastedText = (e.clipboardData || window.clipboardData).getData("text");
      if (!pastedText || pastedText.trim().length === 0) return;

      e.preventDefault();
      var col = lastClickedCol >= 0 && lastClickedCol < f.parsed.columns.length ? lastClickedCol : 0;
      var colName = f.parsed.columns[col];

      // Parse values (newline or comma separated)
      var values = pastedText.split(/[\n,]/).map(function(v) { return v.trim(); }).filter(function(v) { return v.length > 0; });
      if (values.length === 0) return;

      // Show confirmation dialog
      var existing = document.getElementById("vw-paste-filter-dlg");
      if (existing) existing.remove();

      var dlg = document.createElement("div");
      dlg.id = "vw-paste-filter-dlg";
      dlg.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:20px;box-shadow:0 12px 40px rgba(0,0,0,0.5);width:320px;font-family:var(--vw-sans);font-size:13px;";
      dlg.innerHTML =
        '<div style="font-weight:600;color:var(--vw-accent);margin-bottom:8px">Filter by Pasted Values</div>' +
        '<div style="color:var(--vw-text-dim);font-size:12px;margin-bottom:12px">' +
        'Filter column <b style="color:var(--vw-text)">' + escapeHtml(colName) + '</b> by ' + values.length + ' pasted value' + (values.length > 1 ? 's' : '') + ':</div>' +
        '<div style="max-height:100px;overflow:auto;background:var(--vw-bg);border-radius:6px;padding:6px 8px;font-family:var(--vw-mono);font-size:11px;color:var(--vw-text-dim);margin-bottom:12px">' +
        values.slice(0, 20).map(function(v) { return escapeHtml(v); }).join("<br>") +
        (values.length > 20 ? '<br><span style="color:var(--vw-text-muted)">...and ' + (values.length - 20) + ' more</span>' : '') +
        '</div>' +
        '<div style="display:flex;gap:8px">' +
        '<button id="vw-paste-apply" style="flex:1;background:var(--vw-accent);color:white;border:none;border-radius:6px;padding:6px;cursor:pointer;font-size:12px">Apply Filter</button>' +
        '<button id="vw-paste-cancel" style="background:none;border:1px solid var(--vw-border);border-radius:6px;padding:6px 12px;color:var(--vw-text-dim);cursor:pointer;font-size:12px">Cancel</button></div>';
      document.body.appendChild(dlg);

      document.getElementById("vw-paste-apply").addEventListener("click", function() {
        pushUndo();
        colFilters[col] = new Set(values);
        currentPage = 0;
        _filterCache = null;
        dlg.remove();
        renderView();
        showToast("Filtered " + colName + " by " + values.length + " values");
      });
      document.getElementById("vw-paste-cancel").addEventListener("click", function() { dlg.remove(); });
      document.addEventListener("keydown", function handler(e) {
        if (e.key === "Escape") { dlg.remove(); document.removeEventListener("keydown", handler); }
      });
    });
  })();

  // ── Feature N9: Undo Toast ─────────────────────────────────────
  (function() {
    // Patch popUndo to show toast
    var _origPopUndo = popUndo;
    popUndo = function() {
      if (undoStack.length === 0) {
        showToast("Nothing to undo");
        return;
      }
      _origPopUndo();
      showToast("Undo: restored previous state");
    };

    // Patch pushUndo to show subtle hint (only after user edits, not sort/filter)
    var _origPushUndo = pushUndo;
    var pushCount = 0;
    pushUndo = function() {
      _origPushUndo();
      pushCount++;
      // Only show hint on first few pushes to avoid noise
      if (pushCount <= 3) {
        // Don't show toast for pushUndo to avoid clutter; just let the undo toast be informative
      }
    };
  })();

  // ── Feature N10: Data Quality Badge (all formats) ──────────────
  (function() {
    function computeQuality(f) {
      if (f._qualityBadge) return f._qualityBadge;
      if (!f.parsed || !f.parsed.rows || f.parsed.rows.length === 0) {
        f._qualityBadge = { score: 0, label: "N/A", color: "var(--vw-text-muted)" };
        return f._qualityBadge;
      }

      var totalCells = 0, nonEmpty = 0, validNums = 0, numCells = 0;
      var sampleSize = Math.min(f.parsed.rows.length, 5000);
      var idCols = []; // columns that might be IDs (first text column)

      f.parsed.columns.forEach(function(col, ci) {
        if (f.parsed.colTypes[ci] === "str" && /^(id|name|sample|accession)/i.test(col)) idCols.push(ci);
      });

      for (var ri = 0; ri < sampleSize; ri++) {
        for (var ci = 0; ci < f.parsed.columns.length; ci++) {
          totalCells++;
          var val = f.parsed.rows[ri][ci];
          var s = String(val != null ? val : "").trim();
          if (s !== "" && s !== "." && s !== "NA" && s !== "N/A") nonEmpty++;
          if (f.parsed.colTypes[ci] === "num") {
            numCells++;
            if (typeof val === "number" && !isNaN(val)) validNums++;
          }
        }
      }

      var completeness = totalCells > 0 ? nonEmpty / totalCells : 0;
      var validity = numCells > 0 ? validNums / numCells : 1;

      // Uniqueness of ID columns
      var uniqueness = 1;
      if (idCols.length > 0) {
        var idCol = idCols[0];
        var seen = new Set();
        var total = 0;
        for (var ri = 0; ri < sampleSize; ri++) {
          seen.add(String(f.parsed.rows[ri][idCol]));
          total++;
        }
        uniqueness = total > 0 ? seen.size / total : 1;
      }

      var score = Math.round((completeness * 0.4 + validity * 0.35 + uniqueness * 0.25) * 100);
      var label, color;
      if (score >= 90) { label = "Excellent"; color = "var(--vw-green)"; }
      else if (score >= 75) { label = "Good"; color = "var(--vw-cyan)"; }
      else if (score >= 50) { label = "Fair"; color = "var(--vw-amber)"; }
      else { label = "Poor"; color = "var(--vw-red, #f87171)"; }

      f._qualityBadge = {
        score: score, label: label, color: color,
        detail: "Completeness: " + Math.round(completeness * 100) + "%, Validity: " + Math.round(validity * 100) + "%, Uniqueness: " + Math.round(uniqueness * 100) + "%"
      };
      return f._qualityBadge;
    }

    // Add quality badge to tab tooltips on hover
    document.addEventListener("mouseover", function(e) {
      var tab = e.target.closest && e.target.closest(".vw-tab");
      if (!tab) return;
      var tabIdx = Array.from(tab.parentElement.children).indexOf(tab);
      // Adjust for add-tab button
      var adjustedIdx = tabIdx;
      if (adjustedIdx < 0 || adjustedIdx >= files.length) return;
      var f = files[adjustedIdx];
      if (!f || !f.parsed) return;

      var q = computeQuality(f);
      if (tab.title && tab.title.indexOf("Quality:") !== -1) return; // already has badge
      var existing = tab.title || "";
      tab.title = existing + (existing ? "\n" : "") + "Quality: " + q.score + "% (" + q.label + ") \u2014 " + q.detail;
    });
  })();

  // ── Feature: Copy as BioLang Button ─────────────────────────────
  (function() {
    function formatToReadFn(format, name) {
      var map = {
        vcf: "read_vcf", csv: "read_csv", tsv: "read_tsv",
        fasta: "read_fasta", fastq: "read_fastq",
        bed: "read_bed", gff: "read_gff", sam: "read_sam"
      };
      return (map[format] || "read_csv") + '("' + name + '")';
    }

    function generateBioLangSnippet(f) {
      var lines = [];
      lines.push("let data = " + formatToReadFn(f.parsed.format, f.name));

      // Collect for stream formats
      if (f.parsed.format === "fasta" || f.parsed.format === "fastq") {
        lines.push("  |> collect()");
      }

      // Column filters -> filter pipes
      var cols = f.parsed.columns;
      for (var ci in colFilters) {
        if (!colFilters.hasOwnProperty(ci)) continue;
        var colName = cols[ci] || ("col" + ci);
        var allowed = Array.from(colFilters[ci]);
        if (allowed.length === 1) {
          var val = allowed[0];
          if (!isNaN(val) && val !== "") {
            lines.push('  |> filter(|r| r.' + colName + ' == ' + val + ')');
          } else {
            lines.push('  |> filter(|r| r.' + colName + ' == "' + val.replace(/"/g, '\\"') + '")');
          }
        } else if (allowed.length <= 5) {
          var vals = allowed.map(function(v) {
            return !isNaN(v) && v !== "" ? v : '"' + v.replace(/"/g, '\\"') + '"';
          });
          lines.push('  |> filter(|r| [' + vals.join(", ") + '] |> contains(r.' + colName + '))');
        }
      }

      // Search term -> filter pipe
      if (searchTerm) {
        lines.push('  |> filter(|r| str(r) |> contains("' + searchTerm.replace(/"/g, '\\"') + '"))');
      }

      // Sort -> sort_by pipe
      if (sortCols.length > 0) {
        var sc = sortCols[0];
        var sName = cols[sc.col] || ("col" + sc.col);
        if (sc.asc) {
          lines.push('  |> sort_by(|a, b| a.' + sName + ' - b.' + sName + ')');
        } else {
          lines.push('  |> sort_by(|a, b| b.' + sName + ' - a.' + sName + ')');
        }
      } else if (sortCol >= 0) {
        var sName = cols[sortCol] || ("col" + sortCol);
        if (sortAsc) {
          lines.push('  |> sort_by(|a, b| a.' + sName + ' - b.' + sName + ')');
        } else {
          lines.push('  |> sort_by(|a, b| b.' + sName + ' - a.' + sName + ')');
        }
      }

      // Hidden columns -> select (show only visible)
      var hidden = hiddenCols[activeTab];
      if (hidden && hidden.size > 0) {
        var visible = [];
        for (var i = 0; i < cols.length; i++) {
          if (!hidden.has(i)) visible.push('"' + cols[i] + '"');
        }
        if (visible.length > 0 && visible.length < cols.length) {
          lines.push('  |> select(' + visible.join(", ") + ')');
        }
      }

      return lines.join("\n");
    }

    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (!rightGroup) return;

    var blBtn = document.createElement("button");
    blBtn.className = "vw-tbtn";
    blBtn.textContent = "{ } BioLang";
    blBtn.title = "Copy BioLang code for current view";
    blBtn.id = "vw-biolang-btn";
    blBtn.addEventListener("click", function() {
      var f = files[activeTab];
      if (!f) { showToast("No file open"); return; }
      var code = generateBioLangSnippet(f);
      navigator.clipboard.writeText(code).then(function() {
        showToast("BioLang code copied to clipboard");
      }, function() {
        prompt("Copy this BioLang code:", code);
      });
    });
    rightGroup.insertBefore(blBtn, rightGroup.firstChild);
  })();

  // ── Feature: Sequence Motif Search Button ───────────────────────
  (function() {
    function showMotifResults(results, motif, f) {
      var existing = document.getElementById("vw-motif-results");
      if (existing) existing.remove();

      var overlay = document.createElement("div");
      overlay.id = "vw-motif-results";
      overlay.style.cssText = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);z-index:300;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:12px;padding:16px;box-shadow:0 12px 40px rgba(0,0,0,0.5);width:420px;max-height:500px;display:flex;flex-direction:column;font-family:var(--vw-sans);";

      // Header
      var header = document.createElement("div");
      header.style.cssText = "display:flex;justify-content:space-between;align-items:center;margin-bottom:10px;flex-shrink:0;";
      header.innerHTML =
        '<span style="font-weight:600;color:var(--vw-accent);font-size:13px">Motif: ' + escapeHtml(motif) + ' \u2014 ' + results.length + ' match' + (results.length !== 1 ? 'es' : '') + '</span>' +
        '<span style="cursor:pointer;color:var(--vw-text-dim);font-size:16px" id="vw-motif-close">&times;</span>';
      overlay.appendChild(header);

      if (results.length === 0) {
        var empty = document.createElement("div");
        empty.style.cssText = "padding:20px;text-align:center;color:var(--vw-text-dim);font-size:13px;";
        empty.textContent = "No matches found for \"" + motif + "\"";
        overlay.appendChild(empty);
      } else {
        // Column headers
        var colHeader = document.createElement("div");
        colHeader.style.cssText = "display:flex;padding:4px 6px;font-size:11px;color:var(--vw-text-dim);border-bottom:1px solid var(--vw-border);flex-shrink:0;font-weight:600;";
        colHeader.innerHTML =
          '<span style="width:50px">Row</span>' +
          '<span style="width:70px">Position</span>' +
          '<span style="flex:1">Match</span>' +
          '<span style="width:120px;text-align:right">Context</span>';
        overlay.appendChild(colHeader);

        // Scrollable list
        var list = document.createElement("div");
        list.style.cssText = "overflow-y:auto;flex:1;";

        var nameCol = -1;
        var idCols = ["id", "name", "header", "ID"];
        for (var ic = 0; ic < idCols.length; ic++) {
          var idx = f.parsed.columns.indexOf(idCols[ic]);
          if (idx >= 0) { nameCol = idx; break; }
        }

        var maxShow = Math.min(results.length, 200);
        for (var ri = 0; ri < maxShow; ri++) {
          var r = results[ri];
          var row = document.createElement("div");
          row.style.cssText = "display:flex;padding:4px 6px;font-size:12px;border-bottom:1px solid var(--vw-border);cursor:pointer;align-items:center;";
          row.dataset.rowIdx = r.row;

          var rowLabel = nameCol >= 0 ? String(f.parsed.rows[r.row][nameCol]).substring(0, 15) : String(r.row + 1);
          var seqStr = String(f.parsed.rows[r.row][r.seqCol]);
          var ctxStart = Math.max(0, r.position - 5);
          var ctxEnd = Math.min(seqStr.length, r.position + r.match.length + 5);
          var context = (ctxStart > 0 ? "\u2026" : "") +
            escapeHtml(seqStr.substring(ctxStart, r.position)) +
            '<span style="color:var(--vw-accent);font-weight:700">' + escapeHtml(r.match) + '</span>' +
            escapeHtml(seqStr.substring(r.position + r.match.length, ctxEnd)) +
            (ctxEnd < seqStr.length ? "\u2026" : "");

          row.innerHTML =
            '<span style="width:50px;color:var(--vw-text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap" title="Row ' + (r.row + 1) + '">' + escapeHtml(rowLabel) + '</span>' +
            '<span style="width:70px;color:var(--vw-text-muted);font-family:var(--vw-mono);font-size:11px">' + (r.position + 1) + '</span>' +
            '<span style="flex:1;font-family:var(--vw-mono);font-size:11px;color:var(--vw-text)">' + escapeHtml(r.match) + '</span>' +
            '<span style="width:120px;text-align:right;font-family:var(--vw-mono);font-size:10px;color:var(--vw-text-dim);overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + context + '</span>';

          row.addEventListener("mouseenter", function() { this.style.background = "var(--vw-row-hover)"; });
          row.addEventListener("mouseleave", function() { this.style.background = ""; });

          (function(rowIdx) {
            row.addEventListener("click", function() {
              overlay.remove();
              // Navigate to the row's page and highlight it
              var filtered = getFilteredRows(f);
              for (var fi = 0; fi < filtered.length; fi++) {
                if (filtered[fi].idx === rowIdx) {
                  currentPage = Math.floor(fi / pageSize);
                  renderView();
                  setTimeout(function() {
                    var pageRow = fi % pageSize;
                    var trs = contentEl.querySelectorAll(".vw-table tbody tr:not(.vw-group-row)");
                    if (trs[pageRow]) {
                      trs[pageRow].style.outline = "2px solid var(--vw-accent)";
                      trs[pageRow].scrollIntoView({ block: "center" });
                      setTimeout(function() { trs[pageRow].style.outline = ""; }, 3000);
                    }
                  }, 100);
                  break;
                }
              }
            });
          })(r.row);

          list.appendChild(row);
        }

        if (results.length > maxShow) {
          var more = document.createElement("div");
          more.style.cssText = "padding:6px;font-size:11px;color:var(--vw-text-dim);text-align:center;";
          more.textContent = "... and " + (results.length - maxShow) + " more matches";
          list.appendChild(more);
        }

        overlay.appendChild(list);
      }

      // Summary stats
      var summary = document.createElement("div");
      summary.style.cssText = "margin-top:8px;padding-top:8px;border-top:1px solid var(--vw-border);font-size:11px;color:var(--vw-text-dim);flex-shrink:0;";
      var uniqueRows = new Set(results.map(function(r) { return r.row; }));
      summary.textContent = results.length + " matches across " + uniqueRows.size + " sequence" + (uniqueRows.size !== 1 ? "s" : "");
      overlay.appendChild(summary);

      document.body.appendChild(overlay);

      document.getElementById("vw-motif-close").addEventListener("click", function() { overlay.remove(); });
      function closeOverlay(ev) {
        if (!overlay.contains(ev.target)) { overlay.remove(); document.removeEventListener("mousedown", closeOverlay); }
      }
      setTimeout(function() { document.addEventListener("mousedown", closeOverlay); }, 0);
    }

    // Patch updateToolbar to show/hide motif search button
    var prevUpdateToolbar = updateToolbar;
    updateToolbar = function() {
      prevUpdateToolbar();
      var f = files[activeTab];
      var btn = document.getElementById("vw-motif-search-btn");
      if (!btn) return;
      if (f && (f.parsed.format === "fasta" || f.parsed.format === "fastq")) {
        btn.style.display = "";
      } else {
        btn.style.display = "none";
      }
    };

    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (!rightGroup) return;

    var motifSearchBtn = document.createElement("button");
    motifSearchBtn.className = "vw-tbtn";
    motifSearchBtn.textContent = "Motif";
    motifSearchBtn.title = "Search for DNA motif across all sequences";
    motifSearchBtn.id = "vw-motif-search-btn";
    motifSearchBtn.style.display = "none";
    motifSearchBtn.addEventListener("click", function() {
      var f = files[activeTab];
      if (!f) return;
      var motif = prompt("Enter DNA motif (e.g., GAATTC, or regex like GA[AT]TTC):");
      if (!motif) return;

      // Find sequence columns
      var seqCols = [];
      if (f.parsed.colTypes) {
        f.parsed.colTypes.forEach(function(t, i) {
          if (t === "seq") seqCols.push(i);
        });
      }
      if (seqCols.length === 0) {
        var seqNames = ["sequence", "seq", "SEQ", "SEQUENCE"];
        for (var si = 0; si < seqNames.length; si++) {
          var idx = f.parsed.columns.indexOf(seqNames[si]);
          if (idx >= 0) { seqCols.push(idx); break; }
        }
      }
      if (seqCols.length === 0) {
        showToast("No sequence column found");
        return;
      }

      var results = [];
      try {
        var re = new RegExp(motif.toUpperCase(), "gi");
        f.parsed.rows.forEach(function(row, i) {
          for (var sc = 0; sc < seqCols.length; sc++) {
            var seq = String(row[seqCols[sc]]).toUpperCase();
            re.lastIndex = 0;
            var match;
            while ((match = re.exec(seq)) !== null) {
              results.push({ row: i, position: match.index, match: match[0], seqCol: seqCols[sc] });
              if (match[0].length === 0) { re.lastIndex++; }
            }
          }
        });
      } catch (e) {
        showToast("Invalid motif regex: " + e.message);
        return;
      }

      showMotifResults(results, motif, f);
    });
    rightGroup.insertBefore(motifSearchBtn, rightGroup.firstChild);
  })();

  // ── Feature 21: FASTQ QC Summary Panel ──────────────────────────
  (function() {
    var qcBtn = document.createElement("button");
    qcBtn.className = "vw-tbtn";
    qcBtn.innerHTML = "\u2695 QC";
    qcBtn.title = "FASTQ quality control summary";
    qcBtn.id = "vw-fastq-qc-btn";
    qcBtn.style.display = "none";
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(qcBtn, rightGroup.firstChild);

    qcBtn.addEventListener("click", function() {
      var existing = document.getElementById("vw-fastq-qc-panel");
      if (existing) { existing.remove(); return; }
      var f = files[activeTab];
      if (!f || f.parsed.format !== "fastq") return;

      var rows = f.parsed.rows;
      var stats = f.parsed.stats || {};
      var quals = rows.map(function(r) { return parseFloat(r[2]); });
      var lens = rows.map(function(r) { return r[1]; });
      var totalBases = lens.reduce(function(a, b) { return a + b; }, 0);

      // Compute GC content
      var gcSum = 0, gcTotal = 0;
      var sampleN = Math.min(rows.length, 5000);
      for (var i = 0; i < sampleN; i++) {
        var seq = String(rows[i][3] || "");
        for (var j = 0; j < seq.length; j++) {
          var ch = seq.charAt(j).toUpperCase();
          if (ch === "G" || ch === "C") { gcSum++; gcTotal++; }
          else if (ch === "A" || ch === "T" || ch === "U" || ch === "N") { gcTotal++; }
        }
      }
      var gcPct = gcTotal > 0 ? (gcSum / gcTotal * 100).toFixed(1) : "N/A";

      // Q20+ and Q30+ percentages
      var q20Count = 0, q30Count = 0;
      for (var i = 0; i < quals.length; i++) {
        if (quals[i] >= 20) q20Count++;
        if (quals[i] >= 30) q30Count++;
      }
      var q20Pct = rows.length > 0 ? (q20Count / rows.length * 100).toFixed(1) : "0";
      var q30Pct = rows.length > 0 ? (q30Count / rows.length * 100).toFixed(1) : "0";

      // Median quality
      var sortedQ = quals.slice().sort(function(a, b) { return a - b; });
      var medianQ = sortedQ.length > 0 ? sortedQ[Math.floor(sortedQ.length / 2)].toFixed(1) : "0";

      // N50 read length
      var sortedLens = lens.slice().sort(function(a, b) { return b - a; });
      var cumLen = 0, n50 = 0, halfTotal = totalBases / 2;
      for (var i = 0; i < sortedLens.length; i++) {
        cumLen += sortedLens[i];
        if (cumLen >= halfTotal) { n50 = sortedLens[i]; break; }
      }

      // Build read-length histogram as inline SVG
      var histBins = 20;
      var minLen = Math.min.apply(null, lens), maxLen = Math.max.apply(null, lens);
      var binWidth = maxLen > minLen ? (maxLen - minLen) / histBins : 1;
      var bins = new Array(histBins).fill(0);
      for (var i = 0; i < lens.length; i++) {
        var bi = Math.min(Math.floor((lens[i] - minLen) / binWidth), histBins - 1);
        bins[bi]++;
      }
      var maxBin = Math.max.apply(null, bins) || 1;
      var svgW = 320, svgH = 80, barW = svgW / histBins;
      var histSvg = '<svg width="' + svgW + '" height="' + (svgH + 16) + '" style="display:block;margin:8px 0">';
      for (var i = 0; i < histBins; i++) {
        var bh = (bins[i] / maxBin) * svgH;
        histSvg += '<rect x="' + (i * barW) + '" y="' + (svgH - bh) + '" width="' + (barW - 1) + '" height="' + bh + '" fill="#60a5fa" rx="1"/>';
      }
      histSvg += '<text x="0" y="' + (svgH + 12) + '" fill="#94a3b8" font-size="9" font-family="var(--vw-mono)">' + minLen + ' bp</text>';
      histSvg += '<text x="' + svgW + '" y="' + (svgH + 12) + '" fill="#94a3b8" font-size="9" font-family="var(--vw-mono)" text-anchor="end">' + maxLen + ' bp</text>';
      histSvg += '</svg>';

      // QC verdict
      var verdict = "PASS", verdictColor = "var(--vw-green)";
      var meanQ = parseFloat(stats["Mean quality"] || 0);
      if (parseFloat(q30Pct) < 60 || meanQ < 20) { verdict = "FAIL"; verdictColor = "var(--vw-red, #f87171)"; }
      else if (parseFloat(q30Pct) < 80 || meanQ < 25) { verdict = "WARN"; verdictColor = "var(--vw-amber, #fbbf24)"; }

      var panel = document.createElement("div");
      panel.id = "vw-fastq-qc-panel";
      panel.style.cssText = "position:absolute;top:40px;right:10px;z-index:9000;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:10px;padding:16px 20px;box-shadow:0 8px 32px rgba(0,0,0,0.4);font-family:var(--vw-sans);max-width:380px;";
      panel.innerHTML =
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">' +
        '<span style="font-weight:600;font-size:14px;color:var(--vw-text)">FASTQ QC Summary</span>' +
        '<span style="font-weight:700;font-size:13px;color:' + verdictColor + ';border:1px solid ' + verdictColor + ';border-radius:4px;padding:2px 8px">' + verdict + '</span>' +
        '</div>' +
        '<table style="width:100%;font-size:12px;border-collapse:collapse;color:var(--vw-text)">' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">Total reads</td><td style="text-align:right;font-family:var(--vw-mono)">' + rows.length.toLocaleString() + '</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">Total bases</td><td style="text-align:right;font-family:var(--vw-mono)">' + totalBases.toLocaleString() + '</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">Mean length</td><td style="text-align:right;font-family:var(--vw-mono)">' + (rows.length ? Math.round(totalBases / rows.length) : 0) + ' bp</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">N50</td><td style="text-align:right;font-family:var(--vw-mono)">' + n50.toLocaleString() + ' bp</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">Mean quality</td><td style="text-align:right;font-family:var(--vw-mono)">Q' + (stats["Mean quality"] || "0") + '</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">Median quality</td><td style="text-align:right;font-family:var(--vw-mono)">Q' + medianQ + '</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">Q20+ reads</td><td style="text-align:right;font-family:var(--vw-mono)">' + q20Pct + '%</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">Q30+ reads</td><td style="text-align:right;font-family:var(--vw-mono)">' + q30Pct + '%</td></tr>' +
        '<tr><td style="padding:3px 0;color:var(--vw-text-dim)">GC content</td><td style="text-align:right;font-family:var(--vw-mono)">' + gcPct + '%</td></tr>' +
        '</table>' +
        '<div style="margin-top:10px;font-size:11px;font-weight:600;color:var(--vw-text-dim)">Read Length Distribution</div>' +
        histSvg +
        '<button id="vw-fastq-qc-close" style="position:absolute;top:8px;right:10px;background:none;border:none;color:var(--vw-text-dim);cursor:pointer;font-size:16px">&times;</button>';

      // Insert relative to toolbar
      var toolbar = document.getElementById("vw-toolbar");
      if (toolbar && toolbar.parentNode) {
        toolbar.parentNode.style.position = "relative";
        toolbar.parentNode.appendChild(panel);
      } else {
        document.body.appendChild(panel);
      }

      document.getElementById("vw-fastq-qc-close").addEventListener("click", function() { panel.remove(); });
      // Close on outside click
      setTimeout(function() {
        document.addEventListener("click", function handler(e) {
          if (!panel.contains(e.target) && e.target !== qcBtn) {
            panel.remove();
            document.removeEventListener("click", handler);
          }
        });
      }, 100);
    });

    // Show/hide QC button based on format — patch updateToolbar
    var _origUpdateToolbarQC = updateToolbar;
    updateToolbar = function() {
      _origUpdateToolbarQC();
      var f = files[activeTab];
      qcBtn.style.display = (f && f.parsed && f.parsed.format === "fastq") ? "" : "none";
    };
  })();

  // ── Feature 22: VCF Variant Density Panel ──────────────────────────
  (function() {
    var densBtn = document.createElement("button");
    densBtn.className = "vw-tbtn";
    densBtn.innerHTML = "\u2593 Density";
    densBtn.title = "Variant density per chromosome";
    densBtn.id = "vw-vcf-density-btn";
    densBtn.style.display = "none";
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(densBtn, rightGroup.firstChild);

    densBtn.addEventListener("click", function() {
      var existing = document.getElementById("vw-vcf-density-panel");
      if (existing) { existing.remove(); return; }
      var f = files[activeTab];
      if (!f || f.parsed.format !== "vcf") return;

      // Count variants per chromosome
      var chromCol = f.parsed.columns.indexOf("CHROM");
      if (chromCol < 0) chromCol = 0;
      var chromCounts = {};
      f.parsed.rows.forEach(function(r) {
        var c = String(r[chromCol] || "");
        if (c && c !== ".") chromCounts[c] = (chromCounts[c] || 0) + 1;
      });

      // Sort chromosomes naturally
      var entries = Object.entries(chromCounts).sort(function(a, b) {
        var na = a[0].replace(/^chr/i, ""), nb = b[0].replace(/^chr/i, "");
        var ia = parseInt(na), ib = parseInt(nb);
        if (!isNaN(ia) && !isNaN(ib)) return ia - ib;
        if (!isNaN(ia)) return -1;
        if (!isNaN(ib)) return 1;
        return na.localeCompare(nb);
      }).slice(0, 30);

      var maxCount = Math.max.apply(null, entries.map(function(e) { return e[1]; })) || 1;
      var totalVariants = f.parsed.rows.length;

      // Build inline SVG horizontal bar chart
      var barH = 18, gap = 3, labelW = 65, valueW = 50;
      var chartW = 360, chartAreaW = chartW - labelW - valueW;
      var svgH = entries.length * (barH + gap) + 10;
      var colors = ["#7c3aed", "#a78bfa", "#6d28d9", "#8b5cf6", "#c4b5fd"];

      var svgParts = ['<svg width="' + chartW + '" height="' + svgH + '" style="display:block">'];
      entries.forEach(function(entry, i) {
        var y = i * (barH + gap) + 5;
        var bw = (entry[1] / maxCount) * chartAreaW;
        var pct = (entry[1] / totalVariants * 100).toFixed(1);
        var color = colors[i % colors.length];
        svgParts.push('<text x="' + (labelW - 4) + '" y="' + (y + barH / 2 + 4) + '" fill="#94a3b8" font-size="10" text-anchor="end" font-family="var(--vw-mono)">' + entry[0] + '</text>');
        svgParts.push('<rect x="' + labelW + '" y="' + y + '" width="' + bw + '" height="' + barH + '" fill="' + color + '" rx="3" opacity="0.85"/>');
        svgParts.push('<text x="' + (labelW + chartAreaW + 4) + '" y="' + (y + barH / 2 + 4) + '" fill="#c0caf5" font-size="10" font-family="var(--vw-mono)">' + entry[1].toLocaleString() + ' (' + pct + '%)</text>');
      });
      svgParts.push('</svg>');

      var panel = document.createElement("div");
      panel.id = "vw-vcf-density-panel";
      panel.style.cssText = "position:absolute;top:40px;right:10px;z-index:9000;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:10px;padding:16px 20px;box-shadow:0 8px 32px rgba(0,0,0,0.4);font-family:var(--vw-sans);max-height:80vh;overflow-y:auto;";
      panel.innerHTML =
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">' +
        '<span style="font-weight:600;font-size:14px;color:var(--vw-text)">Variant Density</span>' +
        '<span style="font-size:11px;color:var(--vw-text-dim)">' + totalVariants.toLocaleString() + ' total across ' + entries.length + ' chromosomes</span>' +
        '</div>' +
        svgParts.join("") +
        '<button id="vw-density-close" style="position:absolute;top:8px;right:10px;background:none;border:none;color:var(--vw-text-dim);cursor:pointer;font-size:16px">&times;</button>';

      var toolbar = document.getElementById("vw-toolbar");
      if (toolbar && toolbar.parentNode) {
        toolbar.parentNode.style.position = "relative";
        toolbar.parentNode.appendChild(panel);
      } else {
        document.body.appendChild(panel);
      }

      document.getElementById("vw-density-close").addEventListener("click", function() { panel.remove(); });
      setTimeout(function() {
        document.addEventListener("click", function handler(e) {
          if (!panel.contains(e.target) && e.target !== densBtn) {
            panel.remove();
            document.removeEventListener("click", handler);
          }
        });
      }, 100);
    });

    // Show/hide Density button based on format — patch updateToolbar
    var _origUpdateToolbarDens = updateToolbar;
    updateToolbar = function() {
      _origUpdateToolbarDens();
      var f = files[activeTab];
      densBtn.style.display = (f && f.parsed && f.parsed.format === "vcf") ? "" : "none";
    };
  })();

  // ── Feature: VCF FILTER Pie Chart ──────────────────────────────
  (function() {
    var filterBtn = document.createElement("button");
    filterBtn.className = "vw-tbtn";
    filterBtn.innerHTML = "\u25D4 Filters";
    filterBtn.title = "FILTER value breakdown (PASS/FAIL pie chart)";
    filterBtn.id = "vw-vcf-filter-btn";
    filterBtn.style.display = "none";
    var rightGroup = document.querySelector("#vw-toolbar > div:last-child");
    if (rightGroup) rightGroup.insertBefore(filterBtn, rightGroup.firstChild);

    filterBtn.addEventListener("click", function() {
      var existing = document.getElementById("vw-vcf-filter-panel");
      if (existing) { existing.remove(); return; }
      var f = files[activeTab];
      if (!f || f.parsed.format !== "vcf") return;

      var filterIdx = f.parsed.columns.indexOf("FILTER");
      if (filterIdx < 0) { showToast("No FILTER column found"); return; }

      // Count FILTER values
      var filterCounts = {};
      f.parsed.rows.forEach(function(r) {
        var v = String(r[filterIdx] || ".");
        filterCounts[v] = (filterCounts[v] || 0) + 1;
      });
      var total = f.parsed.rows.length;
      var entries = Object.entries(filterCounts).sort(function(a, b) { return b[1] - a[1]; });

      // Build SVG donut chart
      var svgSize = 160, cx = svgSize / 2, cy = svgSize / 2, radius = 55, innerR = 32;
      var pieColors = { "PASS": "#34d399", ".": "#94a3b8" };
      var failColors = ["#f87171", "#fbbf24", "#fb923c", "#a78bfa", "#60a5fa", "#e879f9"];
      var failIdx = 0;
      entries.forEach(function(e) {
        if (!pieColors[e[0]]) {
          pieColors[e[0]] = failColors[failIdx % failColors.length];
          failIdx++;
        }
      });

      var svgParts = ['<svg width="' + svgSize + '" height="' + svgSize + '" viewBox="0 0 ' + svgSize + ' ' + svgSize + '">'];
      var startAngle = -Math.PI / 2;
      entries.forEach(function(entry) {
        var sliceAngle = (entry[1] / total) * 2 * Math.PI;
        if (sliceAngle < 0.001) return;
        var endAngle = startAngle + sliceAngle;
        var largeArc = sliceAngle > Math.PI ? 1 : 0;
        var x1 = cx + radius * Math.cos(startAngle), y1 = cy + radius * Math.sin(startAngle);
        var x2 = cx + radius * Math.cos(endAngle), y2 = cy + radius * Math.sin(endAngle);
        var ix1 = cx + innerR * Math.cos(startAngle), iy1 = cy + innerR * Math.sin(startAngle);
        var ix2 = cx + innerR * Math.cos(endAngle), iy2 = cy + innerR * Math.sin(endAngle);
        var d = "M" + x1 + "," + y1 + " A" + radius + "," + radius + " 0 " + largeArc + " 1 " + x2 + "," + y2 +
                " L" + ix2 + "," + iy2 + " A" + innerR + "," + innerR + " 0 " + largeArc + " 0 " + ix1 + "," + iy1 + " Z";
        svgParts.push('<path d="' + d + '" fill="' + (pieColors[entry[0]] || "#94a3b8") + '" opacity="0.9"/>');
        startAngle = endAngle;
      });
      // Center label
      svgParts.push('<text x="' + cx + '" y="' + (cy - 4) + '" text-anchor="middle" fill="#e2e8f0" font-size="18" font-weight="700" font-family="var(--vw-mono)">' + total.toLocaleString() + '</text>');
      svgParts.push('<text x="' + cx + '" y="' + (cy + 12) + '" text-anchor="middle" fill="#94a3b8" font-size="10" font-family="var(--vw-sans)">variants</text>');
      svgParts.push('</svg>');

      // Build legend
      var legendParts = [];
      entries.forEach(function(entry) {
        var pct = (entry[1] / total * 100).toFixed(1);
        legendParts.push('<div style="display:flex;align-items:center;gap:6px;padding:2px 0">' +
          '<span style="display:inline-block;width:10px;height:10px;border-radius:2px;background:' + (pieColors[entry[0]] || "#94a3b8") + '"></span>' +
          '<span style="color:var(--vw-text);font-size:12px;font-family:var(--vw-mono)">' + escapeHtml(entry[0]) + '</span>' +
          '<span style="color:var(--vw-text-dim);font-size:11px;margin-left:auto">' + entry[1].toLocaleString() + ' (' + pct + '%)</span></div>');
      });

      var panel = document.createElement("div");
      panel.id = "vw-vcf-filter-panel";
      panel.style.cssText = "position:absolute;top:40px;right:10px;z-index:9000;background:var(--vw-panel);border:1px solid var(--vw-border);border-radius:10px;padding:16px 20px;box-shadow:0 8px 32px rgba(0,0,0,0.4);font-family:var(--vw-sans);max-width:320px;";
      panel.innerHTML =
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px">' +
        '<span style="font-weight:600;font-size:14px;color:var(--vw-text)">FILTER Breakdown</span>' +
        '<button id="vw-vcf-filter-close" style="background:none;border:none;color:var(--vw-text-dim);cursor:pointer;font-size:16px">&times;</button></div>' +
        '<div style="display:flex;align-items:center;gap:16px">' +
        '<div>' + svgParts.join("") + '</div>' +
        '<div style="flex:1">' + legendParts.join("") + '</div></div>';

      var toolbar = document.getElementById("vw-toolbar");
      if (toolbar && toolbar.parentNode) {
        toolbar.parentNode.style.position = "relative";
        toolbar.parentNode.appendChild(panel);
      } else {
        document.body.appendChild(panel);
      }

      document.getElementById("vw-vcf-filter-close").addEventListener("click", function() { panel.remove(); });
      setTimeout(function() {
        document.addEventListener("click", function handler(e) {
          if (!panel.contains(e.target) && e.target !== filterBtn) {
            panel.remove();
            document.removeEventListener("click", handler);
          }
        });
      }, 100);
    });

    // Show/hide Filters button based on format — patch updateToolbar
    var _origUpdateToolbarFilter = updateToolbar;
    updateToolbar = function() {
      _origUpdateToolbarFilter();
      var f = files[activeTab];
      filterBtn.style.display = (f && f.parsed && f.parsed.format === "vcf") ? "" : "none";
    };
  })();

})();
