(function() {
  "use strict";

  // ── State ──────────────────────────────────────────────────────
  var tracks = [];       // [{name, format, data, visible, color}]
  var chroms = {};       // {name: maxPos}
  var view = { chrom: "", start: 0, end: 10000 };
  var DPR = window.devicePixelRatio || 1;

  // Base colors
  var BASE_COLORS = { A: "#34d399", T: "#f87171", C: "#60a5fa", G: "#fbbf24", N: "#94a3b8" };
  var TRACK_COLORS = ["#7c3aed", "#22d3ee", "#34d399", "#fbbf24", "#f87171", "#a78bfa", "#fb923c", "#e879f9"];

  // ── DOM refs ───────────────────────────────────────────────────
  var drop = document.getElementById("gb-drop");
  var workspace = document.getElementById("gb-workspace");
  var chromSelect = document.getElementById("gb-chrom");
  var locInput = document.getElementById("gb-loc");
  var goBtn = document.getElementById("gb-go");
  var zoomSlider = document.getElementById("gb-zoom");
  var rulerCanvas = document.getElementById("gb-ruler-canvas");
  var tracksEl = document.getElementById("gb-tracks");
  var trackListEl = document.getElementById("gb-track-list");
  var tooltip = document.getElementById("gb-tooltip");
  var footerLoc = document.getElementById("gb-footer-loc");
  var footerInfo = document.getElementById("gb-footer-info");
  var fileInput = document.getElementById("gb-file-input");
  var addFileInput = document.getElementById("gb-add-file");

  // ── Format parsers ────────────────────────────────────────────
  function detectFormat(name, text) {
    var ext = (name.match(/\.([^.]+)$/) || [,""])[1].toLowerCase();
    if (ext === "gff3" || ext === "gtf") ext = "gff";
    if (["sam", "vcf", "bed", "gff"].indexOf(ext) !== -1) return ext;
    var head = text.substring(0, 500);
    if (head.indexOf("##fileformat=VCF") !== -1) return "vcf";
    if (head.indexOf("##gff") !== -1 || /^\S+\t\S+\t\S+\t\d+\t\d+/m.test(head)) return "gff";
    if (/^\S+\t\d+\t\d+/m.test(head)) return "bed";
    if (head.charAt(0) === "@" || /^\S+\t\d+\t\S+\t\d+\t\d+/.test(head)) return "sam";
    return "unknown";
  }

  function parseSam(text) {
    var records = [];
    var lines = text.split("\n");
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line || line.charAt(0) === "@") continue;
      var p = line.split("\t");
      if (p.length < 11) continue;
      var flag = parseInt(p[1]);
      if (flag & 4) continue; // skip unmapped
      var chrom = p[2];
      var pos = parseInt(p[3]);
      var mapq = parseInt(p[4]);
      var cigar = p[5];
      var seq = p[9];
      var qual = p[10];
      var name = p[0];
      // Calculate alignment end from CIGAR
      var end = pos;
      var parts = cigar.match(/\d+[MIDNSHP=X]/g) || [];
      for (var j = 0; j < parts.length; j++) {
        var n = parseInt(parts[j]);
        var op = parts[j].charAt(parts[j].length - 1);
        if (op === "M" || op === "D" || op === "N" || op === "=" || op === "X") end += n;
      }
      records.push({ chrom: chrom, start: pos, end: end, name: name, flag: flag, mapq: mapq, cigar: cigar, seq: seq, qual: qual, strand: (flag & 16) ? "-" : "+" });
      updateChrom(chrom, end);
    }
    return { type: "alignment", records: records };
  }

  function parseVcf(text) {
    var records = [];
    var lines = text.split("\n");
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line || line.charAt(0) === "#") continue;
      var p = line.split("\t");
      if (p.length < 8) continue;
      var chrom = p[0], pos = parseInt(p[1]), id = p[2], ref = p[3], alt = p[4], qual = p[5], filter = p[6], info = p[7];
      var end = pos + ref.length;
      records.push({ chrom: chrom, start: pos, end: end, id: id, ref: ref, alt: alt, qual: qual, filter: filter, info: info });
      updateChrom(chrom, end);
    }
    return { type: "variant", records: records };
  }

  function parseBed(text) {
    var records = [];
    var lines = text.split("\n");
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line || line.charAt(0) === "#" || line.substring(0, 5) === "track" || line.substring(0, 7) === "browser") continue;
      var p = line.split("\t");
      if (p.length < 3) continue;
      var chrom = p[0], start = parseInt(p[1]), end = parseInt(p[2]);
      var name = p[3] || "", score = p[4] || "", strand = p[5] || ".";
      records.push({ chrom: chrom, start: start, end: end, name: name, score: score, strand: strand });
      updateChrom(chrom, end);
    }
    return { type: "region", records: records };
  }

  function parseGff(text) {
    var records = [];
    var lines = text.split("\n");
    for (var i = 0; i < lines.length; i++) {
      var line = lines[i].trimEnd();
      if (!line || line.charAt(0) === "#") continue;
      var p = line.split("\t");
      if (p.length < 9) continue;
      var chrom = p[0], source = p[1], ftype = p[2], start = parseInt(p[3]), end = parseInt(p[4]);
      var strand = p[6] || ".";
      var attrs = p[8] || "";
      // Extract Name/ID from attributes
      var nameMatch = attrs.match(/(?:Name|gene_name|gene_id)=([^;]+)/i);
      var name = nameMatch ? nameMatch[1] : ftype;
      records.push({ chrom: chrom, start: start, end: end, name: name, type: ftype, source: source, strand: strand, attrs: attrs });
      updateChrom(chrom, end);
    }
    return { type: "annotation", records: records };
  }

  function updateChrom(name, pos) {
    if (!chroms[name] || chroms[name] < pos) chroms[name] = pos;
  }

  // ── Spatial index: group by chrom + sort by start for binary search ──
  function buildIndex(records) {
    var byChrom = {};
    for (var i = 0; i < records.length; i++) {
      var r = records[i];
      if (!byChrom[r.chrom]) byChrom[r.chrom] = [];
      byChrom[r.chrom].push(r);
    }
    var keys = Object.keys(byChrom);
    for (var k = 0; k < keys.length; k++) {
      byChrom[keys[k]].sort(function(a, b) { return a.start - b.start; });
    }
    return byChrom;
  }

  // Binary search: find first record with end >= queryStart
  function searchStart(arr, queryStart) {
    var lo = 0, hi = arr.length;
    while (lo < hi) {
      var mid = (lo + hi) >>> 1;
      if (arr[mid].end < queryStart) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  }

  // Query records overlapping [start, end) on a given chrom
  function queryRange(index, chrom, start, end) {
    var arr = index[chrom];
    if (!arr || !arr.length) return [];
    var i = searchStart(arr, start);
    var result = [];
    while (i < arr.length && arr[i].start <= end) {
      result.push(arr[i]);
      i++;
    }
    return result;
  }

  // ── File loading ──────────────────────────────────────────────
  // Magic byte check for binary files
  function checkMagic(bytes, name) {
    if (bytes.length < 4) return null;
    var b = new Uint8Array(bytes.slice(0, 16));
    if (b[0] === 0x1F && b[1] === 0x8B) {
      if (name.toLowerCase().endsWith(".bam")) return "BAM is a compressed binary format.\n\nConvert to SAM:  samtools view -h file.bam > file.sam\nExtract a region: samtools view -h file.bam chr1:1000-50000 > region.sam";
      return "Gzip compressed \u2014 decompress first: gunzip file.gz";
    }
    if (b[0] === 0x42 && b[1] === 0x41 && b[2] === 0x4D && b[3] === 0x01) return "BAM is a compressed binary format.\n\nConvert to SAM:  samtools view -h file.bam > file.sam\nExtract a region: samtools view -h file.bam chr1:1000-50000 > region.sam";
    if (b[0] === 0x42 && b[1] === 0x43 && b[2] === 0x46) return "BCF is the binary equivalent of VCF.\n\nConvert: bcftools view file.bcf > file.vcf\nRegion:  bcftools view file.bcf chr1:1000-50000 > region.vcf";
    if (b[0] === 0x43 && b[1] === 0x52 && b[2] === 0x41 && b[3] === 0x4D) return "CRAM is a compressed alignment format (requires reference FASTA).\n\nConvert: samtools view -h -T ref.fa file.cram > file.sam\nRegion:  samtools view -h -T ref.fa file.cram chr1:1000-50000 > region.sam";
    if (b[0] === 0x25 && b[1] === 0x50 && b[2] === 0x44 && b[3] === 0x46) return "PDF document \u2014 not a genomic data file";
    if (b[0] === 0x50 && b[1] === 0x4B && b[2] === 0x03 && b[3] === 0x04) return "ZIP archive \u2014 extract first, then drop individual files";
    if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4E && b[3] === 0x47) return "PNG image \u2014 not a data file";
    if (b[0] === 0xFF && b[1] === 0xD8 && b[2] === 0xFF) return "JPEG image \u2014 not a data file";
    var nonPrint = 0;
    for (var i = 0; i < Math.min(b.length, 16); i++) {
      if (b[i] < 9 || (b[i] > 13 && b[i] < 32 && b[i] !== 27)) nonPrint++;
    }
    if (nonPrint > 5) return "Binary file \u2014 BioBrowser supports text-based SAM, VCF, BED, and GFF only";
    return null;
  }

  function loadFiles(fileList) {
    Array.from(fileList).forEach(function(file) {
      // Check magic bytes first
      var headerReader = new FileReader();
      headerReader.onload = function() {
        var err = checkMagic(headerReader.result, file.name);
        if (err) {
          alert(file.name + "\n\n" + err);
          return;
        }
        loadFileText(file);
      };
      headerReader.readAsArrayBuffer(file.slice(0, 16));
    });
  }

  function loadFileText(file) {
    var reader = new FileReader();
    reader.onload = function() {
        var text = reader.result;
        var fmt = detectFormat(file.name, text);
        var data;
        switch (fmt) {
          case "sam": data = parseSam(text); break;
          case "vcf": data = parseVcf(text); break;
          case "bed": data = parseBed(text); break;
          case "gff": data = parseGff(text); break;
          default:
            alert("Unsupported format: " + file.name + "\n\nBioBrowser supports SAM, VCF, BED, and GFF files.\nFor other formats, use BioPeek instead.");
            return;
        }
        if (data.records.length === 0) {
          alert(file.name + "\n\nNo records parsed. The file may be empty or in an unexpected format.");
          return;
        }
        data.index = buildIndex(data.records);
        tracks.push({
          name: file.name,
          format: fmt,
          data: data,
          visible: true,
          color: TRACK_COLORS[tracks.length % TRACK_COLORS.length],
          height: data.type === "alignment" ? 200 : 60
        });
        onTracksChanged();
      };
      reader.readAsText(file.size > 100 * 1024 * 1024 ? file.slice(0, 100 * 1024 * 1024) : file);
  }

  function onTracksChanged() {
    if (!tracks.length) return;
    drop.style.display = "none";
    workspace.style.display = "flex";

    // Rebuild chromosome list
    var chromList = Object.keys(chroms).sort(function(a, b) {
      var na = a.replace(/^chr/i, ""), nb = b.replace(/^chr/i, "");
      var ia = parseInt(na), ib = parseInt(nb);
      if (!isNaN(ia) && !isNaN(ib)) return ia - ib;
      if (!isNaN(ia)) return -1;
      if (!isNaN(ib)) return 1;
      return na.localeCompare(nb);
    });

    chromSelect.innerHTML = "";
    chromList.forEach(function(c) {
      var opt = document.createElement("option");
      opt.value = c;
      opt.textContent = c;
      chromSelect.appendChild(opt);
    });

    // Set initial view if not set
    if (!view.chrom && chromList.length) {
      view.chrom = chromList[0];
      // Find a region with data
      var firstRec = null;
      for (var ti = 0; ti < tracks.length; ti++) {
        for (var ri = 0; ri < tracks[ti].data.records.length; ri++) {
          if (tracks[ti].data.records[ri].chrom === view.chrom) {
            firstRec = tracks[ti].data.records[ri];
            break;
          }
        }
        if (firstRec) break;
      }
      if (firstRec) {
        var center = Math.floor((firstRec.start + firstRec.end) / 2);
        view.start = Math.max(0, center - 5000);
        view.end = center + 5000;
      }
    }

    chromSelect.value = view.chrom;
    renderTrackChips();
    render();
  }

  // ── Rendering ─────────────────────────────────────────────────
  function render() {
    updateLocInput();
    renderRuler();
    renderTracks();
    footerLoc.textContent = view.chrom + ":" + view.start.toLocaleString() + "-" + view.end.toLocaleString() + " (" + (view.end - view.start).toLocaleString() + " bp)";
    footerInfo.textContent = tracks.length + " track" + (tracks.length !== 1 ? "s" : "");
  }

  function updateLocInput() {
    locInput.value = view.chrom + ":" + view.start.toLocaleString() + "-" + view.end.toLocaleString();
  }

  function renderRuler() {
    var rect = rulerCanvas.parentElement.getBoundingClientRect();
    var w = rect.width, h = 28;
    rulerCanvas.width = w * DPR;
    rulerCanvas.height = h * DPR;
    rulerCanvas.style.width = w + "px";
    rulerCanvas.style.height = h + "px";
    var ctx = rulerCanvas.getContext("2d");
    ctx.scale(DPR, DPR);
    ctx.clearRect(0, 0, w, h);

    var range = view.end - view.start;
    if (range <= 0) return;

    // Tick interval
    var tickInterval = 1;
    var candidates = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000, 500000, 1000000, 2000000, 5000000];
    for (var ci = 0; ci < candidates.length; ci++) {
      if (range / candidates[ci] < 15) { tickInterval = candidates[ci]; break; }
    }

    ctx.strokeStyle = "#334155";
    ctx.fillStyle = "#64748b";
    ctx.font = "10px 'JetBrains Mono', monospace";
    ctx.textAlign = "center";

    // Bottom line
    ctx.beginPath();
    ctx.moveTo(0, h - 1);
    ctx.lineTo(w, h - 1);
    ctx.stroke();

    var firstTick = Math.ceil(view.start / tickInterval) * tickInterval;
    for (var pos = firstTick; pos <= view.end; pos += tickInterval) {
      var x = ((pos - view.start) / range) * w;
      ctx.beginPath();
      ctx.moveTo(x, h - 1);
      ctx.lineTo(x, h - 8);
      ctx.stroke();

      var label;
      if (pos >= 1000000) label = (pos / 1000000).toFixed(pos % 1000000 === 0 ? 0 : 1) + "M";
      else if (pos >= 1000) label = (pos / 1000).toFixed(pos % 1000 === 0 ? 0 : 1) + "k";
      else label = String(pos);
      ctx.fillText(label, x, h - 12);
    }
  }

  function renderTracks() {
    tracksEl.innerHTML = "";
    tracks.forEach(function(track, ti) {
      if (!track.visible) return;
      var div = document.createElement("div");
      div.className = "gb-track";
      div.id = "gb-track-" + ti;

      // Header
      var header = document.createElement("div");
      header.className = "gb-track-header";
      var icons = { alignment: "\uD83D\uDDC2\uFE0F", variant: "\uD83D\uDD2C", region: "\uD83D\uDCCD", annotation: "\uD83D\uDCD0" };
      header.innerHTML = '<span class="gb-track-icon">' + (icons[track.data.type] || "\uD83D\uDCC4") + '</span>' +
        '<span class="gb-track-name">' + escHtml(track.name) + '</span>' +
        '<span class="gb-track-info">' + track.data.records.length + ' records</span>' +
        '<span class="gb-track-toggle">\u25BC</span>';
      header.addEventListener("click", function() {
        div.classList.toggle("collapsed");
      });
      div.appendChild(header);

      // Canvas
      var canvasWrap = document.createElement("div");
      canvasWrap.className = "gb-track-canvas-wrap";
      var canvas = document.createElement("canvas");
      canvas.style.height = track.height + "px";
      canvasWrap.appendChild(canvas);
      div.appendChild(canvasWrap);
      tracksEl.appendChild(div);

      // Render after DOM attach
      requestAnimationFrame(function() {
        renderTrackCanvas(canvas, track);
        setupTrackMouse(canvas, track);
      });
    });
  }

  function renderTrackCanvas(canvas, track) {
    var rect = canvas.parentElement.getBoundingClientRect();
    var w = rect.width, h = track.height;
    canvas.width = w * DPR;
    canvas.height = h * DPR;
    canvas.style.width = w + "px";
    canvas.style.height = h + "px";
    var ctx = canvas.getContext("2d");
    ctx.scale(DPR, DPR);
    ctx.clearRect(0, 0, w, h);

    var range = view.end - view.start;
    if (range <= 0) return;

    // Filter records in view (uses spatial index for O(log n + k) instead of O(n))
    var inView = track.data.index
      ? queryRange(track.data.index, view.chrom, view.start, view.end)
      : track.data.records.filter(function(r) {
          return r.chrom === view.chrom && r.end >= view.start && r.start <= view.end;
        });

    switch (track.data.type) {
      case "alignment": renderAlignments(ctx, w, h, inView, range, track); break;
      case "variant": renderVariants(ctx, w, h, inView, range, track); break;
      case "region": renderRegions(ctx, w, h, inView, range, track); break;
      case "annotation": renderAnnotations(ctx, w, h, inView, range, track); break;
    }

    // Store inView for mouse interaction
    track._inView = inView;
    track._canvasWidth = w;
  }

  // ── Alignment track rendering (read pileup) ───────────────────
  function renderAlignments(ctx, w, h, records, range, track) {
    if (!records.length) {
      ctx.fillStyle = "#475569";
      ctx.font = "12px 'Inter', sans-serif";
      ctx.fillText("No alignments in this region", 20, 30);
      return;
    }

    // Cap records for rendering performance (pack at most 50K — more than enough to fill any viewport)
    var capped = records.length > 50000;
    var toRender = capped ? records.slice(0, 50000) : records;

    // Pack reads into rows (no overlap)
    var rows = [];
    var sorted = toRender.slice().sort(function(a, b) { return a.start - b.start; });
    sorted.forEach(function(r) {
      var placed = false;
      for (var ri = 0; ri < rows.length; ri++) {
        if (rows[ri].length === 0 || rows[ri][rows[ri].length - 1].end + 2 < r.start) {
          rows[ri].push(r);
          placed = true;
          break;
        }
      }
      if (!placed) rows.push([r]);
    });

    var rowH = 10, gap = 2;
    var maxRows = Math.floor(h / (rowH + gap));
    var bpPerPx = range / w;
    var showBases = bpPerPx < 0.5; // Show individual bases when zoomed in enough

    for (var ri = 0; ri < Math.min(rows.length, maxRows); ri++) {
      var y = ri * (rowH + gap);
      rows[ri].forEach(function(r) {
        var x1 = Math.max(0, ((r.start - view.start) / range) * w);
        var x2 = Math.min(w, ((r.end - view.start) / range) * w);
        var rw = Math.max(x2 - x1, 1);

        // Color by mapping quality
        var alpha = Math.max(0.3, Math.min(1, r.mapq / 60));
        var color = r.strand === "-" ? "rgba(248, 113, 113," + alpha + ")" : "rgba(96, 165, 250," + alpha + ")";
        if (r.mapq < 10) color = "rgba(148, 163, 184," + alpha + ")";

        ctx.fillStyle = color;
        ctx.fillRect(x1, y, rw, rowH);

        // Strand arrow
        if (rw > 8) {
          ctx.fillStyle = "rgba(255,255,255,0.3)";
          ctx.beginPath();
          if (r.strand === "+") {
            ctx.moveTo(x2 - 4, y + 2);
            ctx.lineTo(x2, y + rowH / 2);
            ctx.lineTo(x2 - 4, y + rowH - 2);
          } else {
            ctx.moveTo(x1 + 4, y + 2);
            ctx.lineTo(x1, y + rowH / 2);
            ctx.lineTo(x1 + 4, y + rowH - 2);
          }
          ctx.fill();
        }

        // Base-level rendering when zoomed in
        if (showBases && r.seq && r.seq !== "*") {
          ctx.font = "8px 'JetBrains Mono', monospace";
          ctx.textAlign = "center";
          // Use contrasting base colors on strand backgrounds:
          // Forward (blue bg): A=green, T=white, C=yellow, G=amber, N=white
          // Reverse (red bg):  A=green, T=white, C=cyan, G=yellow, N=white
          var isRev = r.strand === "-";
          var baseCols = isRev
            ? { A: "#34d399", T: "#ffffff", C: "#22d3ee", G: "#fbbf24", N: "#e2e8f0" }
            : { A: "#34d399", T: "#ffffff", C: "#fbbf24", G: "#fb923c", N: "#e2e8f0" };
          for (var bi = 0; bi < r.seq.length; bi++) {
            var bx = ((r.start + bi - view.start) / range) * w;
            if (bx < 0 || bx > w) continue;
            var base = r.seq.charAt(bi).toUpperCase();
            ctx.fillStyle = baseCols[base] || "#e2e8f0";
            ctx.fillText(base, bx + (1 / bpPerPx) / 2, y + rowH - 1);
          }
        }
      });
    }

    if (rows.length > maxRows || capped) {
      ctx.fillStyle = "#475569";
      ctx.font = "10px 'Inter', sans-serif";
      var msg = capped ? (records.length.toLocaleString() + " reads in view (showing " + toRender.length.toLocaleString() + ")") : ((rows.length - maxRows) + " more rows...");
      ctx.fillText(msg, 8, h - 4);
    }
  }

  // ── Variant track rendering ───────────────────────────────────
  function renderVariants(ctx, w, h, records, range, track) {
    if (!records.length) {
      ctx.fillStyle = "#475569";
      ctx.font = "12px 'Inter', sans-serif";
      ctx.fillText("No variants in this region", 20, 30);
      return;
    }

    var bpPerPx = range / w;
    records.forEach(function(r) {
      var x = ((r.start - view.start) / range) * w;
      var vw = Math.max(((r.end - r.start) / range) * w, 3);

      // Color by type
      var isSnp = r.ref.length === 1 && r.alt.length === 1;
      var color = isSnp ? "#34d399" : (r.alt.length > r.ref.length ? "#60a5fa" : "#f87171");
      if (r.filter !== "PASS" && r.filter !== ".") color = "#94a3b8";

      // Lollipop style
      var stemX = x + vw / 2;
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(stemX, h - 2);
      ctx.lineTo(stemX, 16);
      ctx.stroke();

      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(stemX, 12, 4, 0, Math.PI * 2);
      ctx.fill();

      // Label when zoomed in
      if (bpPerPx < 10) {
        ctx.fillStyle = "#e2e8f0";
        ctx.font = "9px 'JetBrains Mono', monospace";
        ctx.textAlign = "center";
        var label = r.ref + ">" + r.alt;
        if (label.length > 12) label = label.substring(0, 11) + "\u2026";
        ctx.fillText(label, stemX, h - 6);
      }
    });
  }

  // ── Region track rendering (BED) ──────────────────────────────
  function renderRegions(ctx, w, h, records, range, track) {
    if (!records.length) {
      ctx.fillStyle = "#475569";
      ctx.font = "12px 'Inter', sans-serif";
      ctx.fillText("No regions in this range", 20, 30);
      return;
    }

    var rowH = 18, gap = 4;
    var rows = [];
    var sorted = records.slice().sort(function(a, b) { return a.start - b.start; });
    sorted.forEach(function(r) {
      var placed = false;
      for (var ri = 0; ri < rows.length; ri++) {
        if (rows[ri].length === 0 || rows[ri][rows[ri].length - 1].end < r.start) {
          rows[ri].push(r);
          placed = true;
          break;
        }
      }
      if (!placed) rows.push([r]);
    });

    var maxRows = Math.floor(h / (rowH + gap));
    for (var ri = 0; ri < Math.min(rows.length, maxRows); ri++) {
      var y = ri * (rowH + gap) + 4;
      rows[ri].forEach(function(r) {
        var x1 = Math.max(0, ((r.start - view.start) / range) * w);
        var x2 = Math.min(w, ((r.end - view.start) / range) * w);
        var rw = Math.max(x2 - x1, 2);

        ctx.fillStyle = track.color;
        ctx.globalAlpha = 0.7;
        ctx.fillRect(x1, y, rw, rowH);
        ctx.globalAlpha = 1;

        // Border
        ctx.strokeStyle = track.color;
        ctx.lineWidth = 1;
        ctx.strokeRect(x1 + 0.5, y + 0.5, rw - 1, rowH - 1);

        // Label
        if (r.name && rw > 30) {
          ctx.fillStyle = "#e2e8f0";
          ctx.font = "10px 'JetBrains Mono', monospace";
          ctx.textAlign = "left";
          ctx.fillText(r.name.substring(0, Math.floor(rw / 6)), x1 + 4, y + 13);
        }
      });
    }
  }

  // ── Annotation track rendering (GFF) ──────────────────────────
  function renderAnnotations(ctx, w, h, records, range, track) {
    if (!records.length) {
      ctx.fillStyle = "#475569";
      ctx.font = "12px 'Inter', sans-serif";
      ctx.fillText("No annotations in this region", 20, 30);
      return;
    }

    var featureColors = {
      gene: "#a78bfa", mRNA: "#60a5fa", exon: "#34d399", CDS: "#22d3ee",
      five_prime_UTR: "#fbbf24", three_prime_UTR: "#fb923c", intron: "#475569"
    };

    var rowH = 16, gap = 3;
    var rows = [];
    var sorted = records.slice().sort(function(a, b) { return a.start - b.start; });
    sorted.forEach(function(r) {
      var placed = false;
      for (var ri = 0; ri < rows.length; ri++) {
        if (rows[ri].length === 0 || rows[ri][rows[ri].length - 1].end + 100 < r.start) {
          rows[ri].push(r);
          placed = true;
          break;
        }
      }
      if (!placed) rows.push([r]);
    });

    var maxRows = Math.floor(h / (rowH + gap));
    for (var ri = 0; ri < Math.min(rows.length, maxRows); ri++) {
      var y = ri * (rowH + gap) + 2;
      rows[ri].forEach(function(r) {
        var x1 = Math.max(0, ((r.start - view.start) / range) * w);
        var x2 = Math.min(w, ((r.end - view.start) / range) * w);
        var rw = Math.max(x2 - x1, 2);

        var color = featureColors[r.type] || track.color;
        ctx.fillStyle = color;

        if (r.type === "gene" || r.type === "mRNA") {
          // Gene: thin line with arrow
          ctx.fillRect(x1, y + rowH / 2 - 1, rw, 2);
          // Arrow
          ctx.beginPath();
          if (r.strand === "+") {
            ctx.moveTo(x2 - 4, y + 2);
            ctx.lineTo(x2, y + rowH / 2);
            ctx.lineTo(x2 - 4, y + rowH - 2);
          } else if (r.strand === "-") {
            ctx.moveTo(x1 + 4, y + 2);
            ctx.lineTo(x1, y + rowH / 2);
            ctx.lineTo(x1 + 4, y + rowH - 2);
          }
          ctx.fill();
        } else {
          // Exon/CDS: thick block
          ctx.fillRect(x1, y + 2, rw, rowH - 4);
        }

        // Label
        if (r.name && rw > 40) {
          ctx.fillStyle = "#e2e8f0";
          ctx.font = "9px 'JetBrains Mono', monospace";
          ctx.textAlign = "left";
          ctx.fillText(r.name.substring(0, Math.floor(rw / 5.5)), x1 + 4, y + rowH - 3);
        }
      });
    }
  }

  // ── Track chips ───────────────────────────────────────────────
  function renderTrackChips() {
    trackListEl.innerHTML = "";
    tracks.forEach(function(track, ti) {
      var chip = document.createElement("span");
      chip.className = "gb-track-chip" + (track.visible ? " active" : "");
      chip.style.borderColor = track.visible ? track.color : "";
      chip.style.color = track.visible ? track.color : "";
      chip.innerHTML = escHtml(track.name) + ' <span class="close" data-idx="' + ti + '">&times;</span>';
      chip.addEventListener("click", function(e) {
        if (e.target.classList.contains("close")) {
          tracks.splice(ti, 1);
          onTracksChanged();
        } else {
          track.visible = !track.visible;
          renderTrackChips();
          render();
        }
      });
      trackListEl.appendChild(chip);
    });
  }

  // ── Mouse interaction (tooltips) ──────────────────────────────
  function setupTrackMouse(canvas, track) {
    canvas.addEventListener("mousemove", function(e) {
      var rect = canvas.getBoundingClientRect();
      var mx = e.clientX - rect.left;
      var range = view.end - view.start;
      var gpos = view.start + (mx / rect.width) * range;

      var hit = null;
      if (track._inView) {
        for (var i = 0; i < track._inView.length; i++) {
          var r = track._inView[i];
          if (gpos >= r.start && gpos <= r.end) { hit = r; break; }
        }
      }

      if (hit) {
        var lines = [];
        if (hit.name) lines.push("<b>" + escHtml(hit.name) + "</b>");
        lines.push(hit.chrom + ":" + hit.start.toLocaleString() + "-" + hit.end.toLocaleString());
        if (hit.mapq !== undefined) lines.push("MAPQ: " + hit.mapq + " | Strand: " + hit.strand);
        if (hit.cigar && hit.cigar !== "*") lines.push("CIGAR: " + escHtml(hit.cigar));
        if (hit.ref) lines.push(hit.ref + " > " + hit.alt);
        if (hit.filter) lines.push("Filter: " + hit.filter);
        if (hit.type) lines.push("Type: " + hit.type);
        if (hit.strand && !hit.mapq) lines.push("Strand: " + hit.strand);

        tooltip.innerHTML = lines.join("<br>");
        tooltip.style.display = "block";
        tooltip.style.left = Math.min(e.clientX + 12, window.innerWidth - 420) + "px";
        tooltip.style.top = (e.clientY - 10) + "px";
      } else {
        tooltip.style.display = "none";
      }
    });

    canvas.addEventListener("mouseleave", function() {
      tooltip.style.display = "none";
    });

    // Pan by dragging (throttled with rAF)
    var dragging = false, dragStartX = 0, dragStartView = 0, dragRaf = 0;
    canvas.addEventListener("mousedown", function(e) {
      dragging = true;
      dragStartX = e.clientX;
      dragStartView = view.start;
      canvas.style.cursor = "grabbing";
    });
    document.addEventListener("mousemove", function(e) {
      if (!dragging) return;
      var rect = canvas.getBoundingClientRect();
      var dx = e.clientX - dragStartX;
      var range = view.end - view.start;
      var dpx = (dx / rect.width) * range;
      view.start = Math.max(0, Math.round(dragStartView - dpx));
      view.end = view.start + range;
      if (!dragRaf) {
        dragRaf = requestAnimationFrame(function() { dragRaf = 0; render(); });
      }
    });
    document.addEventListener("mouseup", function() {
      if (dragging) {
        dragging = false;
        canvas.style.cursor = "";
        render(); // final render at exact position
      }
    });

    // Zoom with scroll wheel
    canvas.addEventListener("wheel", function(e) {
      e.preventDefault();
      var rect = canvas.getBoundingClientRect();
      var frac = (e.clientX - rect.left) / rect.width;
      var range = view.end - view.start;
      var factor = e.deltaY > 0 ? 1.3 : 0.77;
      var newRange = Math.max(100, Math.min(chroms[view.chrom] || 1e9, Math.round(range * factor)));
      var center = view.start + frac * range;
      view.start = Math.max(0, Math.round(center - frac * newRange));
      view.end = view.start + newRange;
      render();
    }, { passive: false });
  }

  // ── Navigation controls ───────────────────────────────────────
  function parseLocation(s) {
    s = s.trim().replace(/,/g, "");
    var m = s.match(/^(\S+):(\d+)-(\d+)$/);
    if (m) return { chrom: m[1], start: parseInt(m[2]), end: parseInt(m[3]) };
    m = s.match(/^(\S+):(\d+)$/);
    if (m) { var p = parseInt(m[2]); return { chrom: m[1], start: Math.max(0, p - 5000), end: p + 5000 }; }
    return null;
  }

  goBtn.addEventListener("click", function() {
    var loc = parseLocation(locInput.value);
    if (loc) {
      view.chrom = loc.chrom;
      view.start = loc.start;
      view.end = loc.end;
      chromSelect.value = view.chrom;
      render();
    }
  });

  locInput.addEventListener("keydown", function(e) {
    if (e.key === "Enter") goBtn.click();
  });

  chromSelect.addEventListener("change", function() {
    view.chrom = chromSelect.value;
    // Find first record on this chrom
    var first = null;
    for (var ti = 0; ti < tracks.length; ti++) {
      for (var ri = 0; ri < tracks[ti].data.records.length; ri++) {
        if (tracks[ti].data.records[ri].chrom === view.chrom) {
          first = tracks[ti].data.records[ri];
          break;
        }
      }
      if (first) break;
    }
    if (first) {
      var center = Math.floor((first.start + first.end) / 2);
      view.start = Math.max(0, center - 5000);
      view.end = center + 5000;
    } else {
      view.start = 0;
      view.end = 10000;
    }
    render();
  });

  document.getElementById("gb-zoom-in").addEventListener("click", function() {
    zoom(0.6);
  });
  document.getElementById("gb-zoom-out").addEventListener("click", function() {
    zoom(1.6);
  });
  zoomSlider.addEventListener("input", function() {
    var range = view.end - view.start;
    var center = view.start + range / 2;
    // Slider 1=zoomed out, 20=zoomed in; map to bp range
    var v = parseInt(zoomSlider.value);
    var maxRange = chroms[view.chrom] || 1e8;
    var newRange = Math.round(maxRange * Math.pow(0.6, v));
    newRange = Math.max(100, Math.min(maxRange, newRange));
    view.start = Math.max(0, Math.round(center - newRange / 2));
    view.end = view.start + newRange;
    render();
  });

  document.getElementById("gb-left").addEventListener("click", function() { pan(-0.3); });
  document.getElementById("gb-right").addEventListener("click", function() { pan(0.3); });

  function zoom(factor) {
    var range = view.end - view.start;
    var center = view.start + range / 2;
    var newRange = Math.max(100, Math.round(range * factor));
    view.start = Math.max(0, Math.round(center - newRange / 2));
    view.end = view.start + newRange;
    render();
  }

  function pan(fraction) {
    var range = view.end - view.start;
    var shift = Math.round(range * fraction);
    view.start = Math.max(0, view.start + shift);
    view.end = view.start + range;
    render();
  }

  // ── Keyboard shortcuts ────────────────────────────────────────
  document.addEventListener("keydown", function(e) {
    if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA" || e.target.tagName === "SELECT") return;
    if (e.key === "ArrowLeft" || e.key === "a") { pan(-0.3); e.preventDefault(); }
    else if (e.key === "ArrowRight" || e.key === "d") { pan(0.3); e.preventDefault(); }
    else if (e.key === "=" || e.key === "+") { zoom(0.6); e.preventDefault(); }
    else if (e.key === "-") { zoom(1.6); e.preventDefault(); }
    else if (e.key === "/") { locInput.focus(); e.preventDefault(); }
  });

  // ── Drag and drop ─────────────────────────────────────────────
  document.addEventListener("dragover", function(e) { e.preventDefault(); });
  document.addEventListener("drop", function(e) {
    e.preventDefault();
    drop.classList.remove("drag-over");
    if (e.dataTransfer.files.length) loadFiles(e.dataTransfer.files);
  });
  drop.addEventListener("dragenter", function() { drop.classList.add("drag-over"); });
  drop.addEventListener("dragleave", function(e) {
    if (!drop.contains(e.relatedTarget)) drop.classList.remove("drag-over");
  });

  // File inputs
  document.getElementById("gb-open-btn").addEventListener("click", function() { fileInput.click(); });
  fileInput.addEventListener("change", function() {
    if (fileInput.files.length) loadFiles(fileInput.files);
    fileInput.value = "";
  });
  document.getElementById("gb-add-track").addEventListener("click", function() { addFileInput.click(); });
  addFileInput.addEventListener("change", function() {
    if (addFileInput.files.length) loadFiles(addFileInput.files);
    addFileInput.value = "";
  });

  // Resize
  window.addEventListener("resize", function() {
    if (tracks.length) render();
  });

  // ── Pick up file from BioPeek via sessionStorage ──────────
  try {
    var incoming = sessionStorage.getItem("biobrowser_file");
    if (incoming) {
      sessionStorage.removeItem("biobrowser_file");
      var payload = JSON.parse(incoming);
      var fmt = payload.format || detectFormat(payload.name, payload.text);
      var data;
      switch (fmt) {
        case "sam": data = parseSam(payload.text); break;
        case "vcf": data = parseVcf(payload.text); break;
        case "bed": data = parseBed(payload.text); break;
        case "gff": data = parseGff(payload.text); break;
      }
      if (data && data.records.length > 0) {
        data.index = buildIndex(data.records);
        tracks.push({
          name: payload.name,
          format: fmt,
          data: data,
          visible: true,
          color: TRACK_COLORS[0],
          height: data.type === "alignment" ? 200 : 60
        });
        onTracksChanged();
      }
    }
  } catch (e) {
    // sessionStorage not available or parse failed — ignore
  }

  // ── Helpers ───────────────────────────────────────────────────
  function escHtml(s) {
    var d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }
})();
