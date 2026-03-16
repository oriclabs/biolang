// BioGist — Content Script
// Scans web pages for biological entities (genes, variants, accessions, bio-files).
// Highlights detected entities and communicates with the sidebar via the background worker.

(function() {
  "use strict";

  var detected = [];
  var highlighted = false;

  // ── Gene Symbol Set (~200 clinically/research-relevant symbols) ──────────

  var GENE_SYMBOLS = new Set([
    // Tumor suppressors & oncogenes
    "TP53","BRCA1","BRCA2","PTEN","RB1","APC","VHL","WT1","NF1","NF2",
    "CDKN2A","CDKN2B","SMAD4","FBXW7","BAP1","SMARCB1","STK11","MEN1",
    "TSC1","TSC2","PTCH1","SUFU",
    "MYC","MYCN","MYCL","KRAS","NRAS","HRAS","BRAF","RAF1",
    "PIK3CA","PIK3CB","PIK3R1","AKT1","AKT2","AKT3","MTOR",
    "EGFR","ERBB2","ERBB3","HER2","ALK","RET","MET","ROS1",
    "FGFR1","FGFR2","FGFR3","FGFR4","FLT3","KIT","PDGFRA","PDGFRB",
    "ABL1","BCR","JAK2","JAK1","JAK3","STAT3","STAT5A","STAT5B",
    "CDK4","CDK6","CDK2","CCND1","CCNE1","MDM2","MDM4",
    "MAP2K1","MAP2K2","MAP3K1","MAPK1",
    "CTNNB1","AXIN1","AXIN2","GSK3B","LEF1","TCF7",
    "NOTCH1","NOTCH2","NOTCH3","NOTCH4",

    // DNA damage repair
    "ATM","ATR","CHEK1","CHEK2","PALB2","RAD51","RAD51C","RAD51D",
    "BRIP1","BARD1","NBN","MRE11","POLE","POLD1","MUTYH","FANCA","FANCC",

    // Mismatch repair
    "MLH1","MSH2","MSH6","PMS2","EPCAM",

    // Epigenetics
    "DNMT3A","DNMT3B","DNMT1","TET2","TET1","IDH1","IDH2",
    "EZH2","SUZ12","EED","ARID1A","ARID1B","ARID2",
    "SMARCA4","SMARCA2","KMT2A","KMT2C","KMT2D",
    "SETD2","NSD1","NSD2","CREBBP","EP300",

    // Hematologic
    "NPM1","CEBPA","RUNX1","GATA1","GATA2","SPI1","FLI1",
    "PAX5","EBF1","IKZF1","IKZF3","BCL2","BCL6","MCL1",
    "MYD88","CARD11","CD79A","CD79B","BTK","PLCG2",
    "ASXL1","CALR","MPL","SF3B1","SRSF2","U2AF1","ZRSR2",

    // Signaling & growth factors
    "VEGFA","KDR","FGF2","PDGFB","TGFB1","TGFB2",
    "SMAD2","SMAD3","BMP4","WNT1","WNT3A","SHH","GLI1","GLI2","SMO",
    "IL6","IL2","IL7","IL10","IL1B","TNF","IFNG","CSF2",

    // Stem cell & development
    "SOX2","POU5F1","NANOG","KLF4","LIN28A","LIN28B",
    "TERT","TERC","DKC1","FOXL2","DICER1",

    // Housekeeping / reference genes
    "GAPDH","ACTB","HPRT1","B2M","RPLP0","TBP","HMBS",

    // Neurodegenerative
    "APP","PSEN1","PSEN2","APOE","MAPT","GRN","C9orf72",
    "SOD1","FUS","TARDBP","SNCA","LRRK2","PARK2","PINK1","GBA",
    "HTT","FMR1","DYRK1A","MECP2",

    // Cardiovascular & metabolic
    "LDLR","PCSK9","APOB","MTHFR","F5","F2","SERPINA1",
    "MYBPC3","MYH7","SCN5A","KCNQ1","KCNH2","LMNA","TTN","RYR2",

    // Pharmacogenomics
    "CYP2D6","CYP2C19","CYP2C9","CYP3A4","CYP3A5","CYP1A2","CYP2B6",
    "DPYD","UGT1A1","TPMT","NUDT15","SLCO1B1","ABCB1","VKORC1","NAT2",

    // Inherited disease
    "CFTR","DMD","SMN1","SMN2","HBB","HBA1","HBA2","G6PD",
    "PKD1","PKD2","COL1A1","COL1A2","GJB2","PAH","GAA","GLA",
    "HEXA","HEXB","SMPD1","NPC1","ATP7B",

    // Immune / immuno-oncology
    "CD274","PDCD1","CTLA4","LAG3","HAVCR2","TIGIT",
    "CD8A","CD4","FOXP3","CD19","MS4A1","CD38","TNFRSF17",
    "IFNGR1","IFNGR2",

    // COVID / infectious disease
    "ACE2","TMPRSS2","FURIN","BSG","DPP4",

    // Transcription factors & master regulators
    "MYB","JUN","FOS","ETS1","SRF","NFKB1","RELA","SP1",
    "TP63","TP73","GATA3","FOXA1","AR","ESR1","ESR2","PGR",

    // RNA biology
    "DROSHA","DGCR8","AGO2","XIST",

    // Other frequently studied
    "PARP1","PARP2","AURKA","AURKB","PLK1","WEE1","TTK",
    "SRC","LYN","FYN","NTRK1","NTRK2","NTRK3",
    "IGF1R","INSR","ERCC1","MGMT",

    // Chromatin / histone
    "ATRX","DAXX","H3F3A","HIST1H3B","PBRM1","KDM5C","KDM6A",
    "STAG2","RAD21","SMC1A","SMC3","BCOR","CUX1"
  ]);

  // Common English words & abbreviations to reject (avoids false-positive gene matches)
  var EXCLUDE = new Set([
    "THE","AND","FOR","NOT","ARE","BUT","WAS","HAS","HAD","HIS","HER","ITS",
    "ALL","CAN","DID","GET","GOT","LET","MAY","NEW","NOW","OLD","OUR","OWN",
    "SAY","SHE","TOO","USE","WAY","WHO","BOY","DAD","MOM","SET","TRY","ASK",
    "MEN","RUN","TOP","YES","ADD","AGE","AGO","END","FAR","FEW",
    "URL","PDF","CSV","TSV","TXT","PNG","JPG","SVG","API","AWS","GCP","CPU",
    "GPU","RAM","SSD","USB","MAC","USA","NIH","FDA","CDC",
    "DOI","PMID","NCI","MIT","BSD","GPL","HTTP","HTML","CSS","SQL",
    "GIT","YAML","JSON","TOML","WASM","REST","POST","FROM","WITH","THAT",
    "THIS","HAVE","BEEN","WILL","THAN","EACH","MAKE","LIKE","LONG","MANY",
    "MOST","MUCH","MUST","NAME","NEED","NEXT","ONLY","OPEN","OVER","PART",
    "PLAN","PLAY","READ","REAL","SAME","SAVE","SHOW","SIDE","SIZE","SOME",
    "SORT","STEP","STOP","SUCH","SURE","TAKE","TELL","TEST","TEXT","THEN",
    "THEY","TIME","TRUE","TURN","TYPE","UNIT","UPON","USED","USER","VERY",
    "WANT","WELL","WENT","WHAT","WHEN","WIDE","WORD","WORK","YEAR","ALSO",
    "BACK","CALL","COME","DOES","DOWN","EVEN","FIND","GIVE","GOES",
    "GOOD","HELP","HERE","HIGH","HOME","IDEA","INTO","JUST","KEEP",
    "KIND","KNOW","LAST","LATE","LEFT","LIFE","LINE","LIST","LIVE","LOOK",
    "MADE","MAIN","MEAN","MOVE","NOTE","PAGE","PASS","PATH","PICK","PULL",
    "PUSH","RATE","RULE","SCAN","SELF","SEND","VIEW","WAIT","WALK","WEEK",
    "ZERO","BEST","BOTH","CASE","CITY","COPY","DATA","DATE","DAYS","DONE",
    "DRAW","DROP","EDIT","FAIL","FAST","FILE","FILL","FLAG","FLOW","FORM",
    "FREE","FULL","HEAD","HOLD","HOST","ICON","INFO","ITEM",
    "JOIN","JUMP","KEYS","LACK","LAND","LEAD","LINK","LOAD","LOCK",
    "LOOP","LOSS","MARK","MASK","MASS","MATH","MODE"
  ]);

  // ── Detection Functions ──────────────────────────────────────────────────

  // Extract a snippet of text around a position
  function getSnippet(text, pos, radius) {
    radius = radius || 40;
    var start = Math.max(0, pos - radius);
    var end = Math.min(text.length, pos + radius);
    var s = text.substring(start, end).replace(/\s+/g, " ").trim();
    return (start > 0 ? "..." : "") + s + (end < text.length ? "..." : "");
  }

  function detectGenes(text) {
    var results = [];
    var seen = {};
    var re = /\b([A-Z][A-Z0-9]{1,9})\b/g;
    var match;
    while ((match = re.exec(text)) !== null) {
      var symbol = match[1];
      if (!EXCLUDE.has(symbol) && GENE_SYMBOLS.has(symbol)) {
        if (!seen[symbol]) {
          seen[symbol] = { type: "gene", id: symbol, position: match.index, count: 1, snippet: getSnippet(text, match.index) };
        } else {
          seen[symbol].count++;
        }
      }
    }
    // Also catch gene names with hyphens (e.g. HLA-A, HLA-B)
    var hyphenRe = /\b([A-Z][A-Z0-9]+-[A-Z0-9]+)\b/g;
    while ((match = hyphenRe.exec(text)) !== null) {
      if (GENE_SYMBOLS.has(match[1]) && !seen[match[1]]) {
        seen[match[1]] = { type: "gene", id: match[1], position: match.index, count: 1, snippet: getSnippet(text, match.index) };
      }
    }
    return Object.values(seen);
  }

  function detectVariants(text) {
    var results = [];
    var seen = {};
    var match;

    function addVariant(id, subtype, pos) {
      if (!seen[id]) {
        seen[id] = { type: "variant", id: id, subtype: subtype, position: pos, count: 1, snippet: getSnippet(text, pos) };
      } else {
        seen[id].count++;
      }
    }

    var rsRe = /\b(rs\d{3,12})\b/gi;
    while ((match = rsRe.exec(text)) !== null) addVariant(match[1].toLowerCase(), "rsid", match.index);

    var hgvsRe = /\b((?:NM_|NP_|NC_)\d+(?:\.\d+)?:[cpg]\.\S+?)(?=[\s,;)\]]|$)/g;
    while ((match = hgvsRe.exec(text)) !== null) addVariant(match[1], "hgvs", match.index);

    var clinvarRe = /\b(VCV\d{9,12})\b/g;
    while ((match = clinvarRe.exec(text)) !== null) addVariant(match[1], "clinvar", match.index);

    var cosmicRe = /\b(COSM\d{3,10})\b/g;
    while ((match = cosmicRe.exec(text)) !== null) addVariant(match[1], "cosmic", match.index);

    return Object.values(seen);
  }

  function detectAccessions(text) {
    var results = [];
    var seen = new Set();
    var match;

    var patterns = [
      // GEO
      { re: /\b(GSE\d{3,8})\b/g, subtype: "geo_series" },
      { re: /\b(GSM\d{3,8})\b/g, subtype: "geo_sample" },
      { re: /\b(GPL\d{2,6})\b/g, subtype: "geo_platform" },
      // SRA / ENA / DDBJ
      { re: /\b(SRR\d{5,10})\b/g, subtype: "sra_run" },
      { re: /\b(SRX\d{5,10})\b/g, subtype: "sra_experiment" },
      { re: /\b(SRP\d{5,10})\b/g, subtype: "sra_project" },
      { re: /\b(SRS\d{5,10})\b/g, subtype: "sra_sample" },
      { re: /\b(ERR\d{5,10})\b/g, subtype: "ena_run" },
      { re: /\b(ERX\d{5,10})\b/g, subtype: "ena_experiment" },
      { re: /\b(DRR\d{5,10})\b/g, subtype: "dra_run" },
      // BioProject / BioSample
      { re: /\b(PRJNA\d{4,8})\b/g, subtype: "bioproject" },
      { re: /\b(PRJEB\d{4,8})\b/g, subtype: "bioproject_eu" },
      { re: /\b(PRJDB\d{4,8})\b/g, subtype: "bioproject_jp" },
      { re: /\b(SAMN\d{6,10})\b/g, subtype: "biosample" },
      { re: /\b(SAME\d{6,10})\b/g, subtype: "biosample_eu" },
      // Assemblies
      { re: /\b(GCA_\d{9}\.\d+)\b/g, subtype: "genbank_assembly" },
      { re: /\b(GCF_\d{9}\.\d+)\b/g, subtype: "refseq_assembly" },
      // Ensembl
      { re: /\b(ENSG\d{11})\b/g, subtype: "ensembl_gene" },
      { re: /\b(ENST\d{11})\b/g, subtype: "ensembl_transcript" },
      { re: /\b(ENSP\d{11})\b/g, subtype: "ensembl_protein" },
      // RefSeq transcripts / proteins
      { re: /\b(NM_\d{6,9}(?:\.\d+)?)\b/g, subtype: "refseq_mrna" },
      { re: /\b(NP_\d{6,9}(?:\.\d+)?)\b/g, subtype: "refseq_protein" },
      { re: /\b(NC_\d{6,9}(?:\.\d+)?)\b/g, subtype: "refseq_chromosome" },
      // PDB IDs (4-char: digit + letter + 2 alphanum)
      { re: /\b(\d[A-Z][A-Z0-9]{2})\b/g, subtype: "pdb" },
      // PubMed IDs (PMID: prefix)
      { re: /PMID:\s*(\d{5,9})/g, subtype: "pubmed" },
      // DOI
      { re: /\b(10\.\d{4,9}\/[^\s,;)}\]]+)/g, subtype: "doi" },
    ];

    for (var i = 0; i < patterns.length; i++) {
      var p = patterns[i];
      p.re.lastIndex = 0;
      while ((match = p.re.exec(text)) !== null) {
        var id = match[1];
        var key = p.subtype + ":" + id;
        if (!seen.has(key)) {
          seen.add(key);
          results.push({ type: "accession", id: id, subtype: p.subtype, position: match.index });
        }
      }
    }

    return results;
  }

  // ── Species Detection ──────────────────────────────────────────────────

  function detectSpecies(text) {
    var species = [
      { pattern: /\b(Homo sapiens|human|H\. sapiens)\b/gi, name: "Human", taxid: "9606" },
      { pattern: /\b(Mus musculus|mouse|M\. musculus)\b/gi, name: "Mouse", taxid: "10090" },
      { pattern: /\b(Rattus norvegicus|rat|R\. norvegicus)\b/gi, name: "Rat", taxid: "10116" },
      { pattern: /\b(Drosophila melanogaster|fruit fly|D\. melanogaster)\b/gi, name: "Fruit fly", taxid: "7227" },
      { pattern: /\b(Caenorhabditis elegans|C\. elegans|nematode)\b/gi, name: "C. elegans", taxid: "6239" },
      { pattern: /\b(Danio rerio|zebrafish|D\. rerio)\b/gi, name: "Zebrafish", taxid: "7955" },
      { pattern: /\b(Saccharomyces cerevisiae|yeast|S\. cerevisiae|budding yeast)\b/gi, name: "Yeast", taxid: "4932" },
      { pattern: /\b(Arabidopsis thaliana|A\. thaliana)\b/gi, name: "Arabidopsis", taxid: "3702" },
      { pattern: /\b(Escherichia coli|E\. coli)\b/gi, name: "E. coli", taxid: "562" },
    ];
    var found = [];
    var seen = new Set();
    species.forEach(function(sp) {
      sp.pattern.lastIndex = 0;
      if (sp.pattern.test(text) && !seen.has(sp.name)) {
        seen.add(sp.name);
        found.push({ type: "species", id: sp.name, subtype: sp.taxid });
      }
    });
    return found;
  }

  function detectFileLinks() {
    var results = [];
    var seen = new Set();
    var links = document.querySelectorAll("a[href]");
    var bioExts = /\.(fastq|fq|fasta|fa|fna|vcf|bcf|bam|cram|bed|bedgraph|bigwig|bw|gff|gff3|gtf|sam|tsv|csv|parquet|h5ad|loom|mzml|mzxml|mgf)(\.gz|\.bgz|\.bz2|\.zst)?$/i;

    for (var i = 0; i < links.length; i++) {
      var a = links[i];
      var href = a.href;
      if (!href || seen.has(href)) continue;
      var m = href.match(bioExts);
      if (m) {
        seen.add(href);
        var ext = m[1].toUpperCase();
        if (m[2]) ext += m[2].toUpperCase();
        results.push({
          type: "file",
          id: href,
          subtype: ext,
          text: (a.textContent || "").trim().substring(0, 60)
        });
      }
    }
    return results;
  }

  // ── Page Scanner ─────────────────────────────────────────────────────────

  function getVisibleText() {
    if (!document.body) return "";
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
      acceptNode: function(node) {
        var parent = node.parentElement;
        if (!parent) return NodeFilter.FILTER_REJECT;
        var tag = parent.tagName;
        if (tag === "SCRIPT" || tag === "STYLE" || tag === "NOSCRIPT" || tag === "TEMPLATE") {
          return NodeFilter.FILTER_REJECT;
        }
        if (parent.offsetParent === null && tag !== "BODY" && tag !== "HTML") {
          return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      }
    });

    var chunks = [];
    var node;
    var charCount = 0;
    var charLimit = 500000;
    while ((node = walker.nextNode())) {
      var t = node.textContent;
      if (t.trim().length > 1) {
        chunks.push(t);
        charCount += t.length;
        if (charCount >= charLimit) break;
      }
    }
    return chunks.join(" ");
  }

  function scanPage() {
    var fullText = getVisibleText();

    var entities = [];
    entities = entities.concat(detectGenes(fullText));
    entities = entities.concat(detectVariants(fullText));
    entities = entities.concat(detectAccessions(fullText));
    entities = entities.concat(detectFileLinks());
    entities = entities.concat(detectSpecies(fullText));

    // Sort by position (files/species without position go last)
    entities.sort(function(a, b) {
      var pa = a.position != null ? a.position : Infinity;
      var pb = b.position != null ? b.position : Infinity;
      return pa - pb;
    });

    return entities;
  }

  // ── Highlighting ─────────────────────────────────────────────────────────

  var HIGHLIGHT_STYLES = {
    gene:      "border-bottom:2px solid #8b5cf6;cursor:pointer;padding-bottom:1px",
    variant:   "border-bottom:2px solid #06b6d4;cursor:pointer;padding-bottom:1px",
    accession: "border-bottom:2px solid #f59e0b;cursor:pointer;padding-bottom:1px"
  };

  function escapeRegExp(s) {
    return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  function highlightEntities(entities) {
    if (highlighted) return;

    // Build lookup sets per type
    var byType = { gene: new Set(), variant: new Set(), accession: new Set() };
    for (var i = 0; i < entities.length; i++) {
      var e = entities[i];
      if (byType[e.type]) byType[e.type].add(e.id);
    }

    // Walk text nodes and collect those containing any entity
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    var nodesToReplace = [];
    var node;

    while ((node = walker.nextNode())) {
      var text = node.textContent;
      if (text.trim().length < 2) continue;
      var parent = node.parentElement;
      if (!parent) continue;
      var tag = parent.tagName;
      if (tag === "SCRIPT" || tag === "STYLE" || tag === "NOSCRIPT" || tag === "TEXTAREA" || tag === "INPUT") continue;
      if (parent.classList && parent.classList.contains("biogist-hl")) continue;
      if (parent.closest && parent.closest(".biogist-wrap")) continue;

      var hasMatch = false;
      var iter, v;

      // Check genes
      iter = byType.gene.values();
      for (v = iter.next(); !v.done; v = iter.next()) {
        if (text.indexOf(v.value) !== -1) { hasMatch = true; break; }
      }
      // Check variants (case-insensitive for rsIDs)
      if (!hasMatch) {
        var lower = text.toLowerCase();
        iter = byType.variant.values();
        for (v = iter.next(); !v.done; v = iter.next()) {
          if (lower.indexOf(v.value.toLowerCase()) !== -1) { hasMatch = true; break; }
        }
      }
      // Check accessions
      if (!hasMatch) {
        iter = byType.accession.values();
        for (v = iter.next(); !v.done; v = iter.next()) {
          if (text.indexOf(v.value) !== -1) { hasMatch = true; break; }
        }
      }

      if (hasMatch) nodesToReplace.push(node);
    }

    // Replace text nodes in reverse to avoid DOM invalidation
    for (var j = nodesToReplace.length - 1; j >= 0; j--) {
      var tnode = nodesToReplace[j];
      if (!tnode.parentElement) continue;

      var html = tnode.textContent;
      var changed = false;

      // Highlight genes
      var gIter = byType.gene.values();
      for (var g = gIter.next(); !g.done; g = gIter.next()) {
        var gVal = g.value;
        var gRe = new RegExp("\\b(" + escapeRegExp(gVal) + ")\\b", "g");
        var newHtml = html.replace(gRe,
          '<span class="biogist-hl biogist-gene" data-entity="$1" data-type="gene" title="Gene: $1" style="' + HIGHLIGHT_STYLES.gene + '">$1</span>');
        if (newHtml !== html) { html = newHtml; changed = true; }
      }

      // Highlight variants
      var vIter = byType.variant.values();
      for (var vv = vIter.next(); !vv.done; vv = vIter.next()) {
        var vVal = vv.value;
        var vRe = new RegExp("\\b(" + escapeRegExp(vVal) + ")\\b", "gi");
        var newHtml2 = html.replace(vRe,
          '<span class="biogist-hl biogist-variant" data-entity="$1" data-type="variant" title="Variant: $1" style="' + HIGHLIGHT_STYLES.variant + '">$1</span>');
        if (newHtml2 !== html) { html = newHtml2; changed = true; }
      }

      // Highlight accessions
      var aIter = byType.accession.values();
      for (var aa = aIter.next(); !aa.done; aa = aIter.next()) {
        var aVal = aa.value;
        var aRe = new RegExp("\\b(" + escapeRegExp(aVal) + ")\\b", "g");
        var newHtml3 = html.replace(aRe,
          '<span class="biogist-hl biogist-accession" data-entity="$1" data-type="accession" title="Accession: $1" style="' + HIGHLIGHT_STYLES.accession + '">$1</span>');
        if (newHtml3 !== html) { html = newHtml3; changed = true; }
      }

      if (changed) {
        var wrapper = document.createElement("span");
        wrapper.className = "biogist-wrap";
        wrapper.innerHTML = html;
        tnode.parentElement.replaceChild(wrapper, tnode);
      }
    }

    // Attach click handler via event delegation (single listener)
    document.body.addEventListener("click", onHighlightClick, true);
    highlighted = true;
  }

  function onHighlightClick(e) {
    var el = e.target;
    if (!el.classList || !el.classList.contains("biogist-hl")) return;
    var entity = el.dataset.entity;
    var entityType = el.dataset.type;
    if (entity) {
      chrome.runtime.sendMessage({ type: "lookup", text: entity, entityType: entityType });
    }
  }

  function clearHighlights() {
    document.body.removeEventListener("click", onHighlightClick, true);

    var wrappers = document.querySelectorAll(".biogist-wrap");
    for (var i = 0; i < wrappers.length; i++) {
      var wrapper = wrappers[i];
      var parent = wrapper.parentNode;
      if (!parent) continue;
      var textNode = document.createTextNode(wrapper.textContent);
      parent.replaceChild(textNode, wrapper);
      parent.normalize();
    }

    highlighted = false;
  }

  // ── SPA Navigation Detection ─────────────────────────────────────────────
  // Detect URL changes in SPAs (pushState/popstate) and clear stale highlights
  var lastKnownUrl = location.href;

  function checkUrlChange() {
    if (location.href !== lastKnownUrl) {
      lastKnownUrl = location.href;
      clearHighlights();
      detected = [];
      // Re-scan if new page looks bio-relevant
      if (isBioPage()) {
        setTimeout(function() {
          detected = scanPage();
          if (detected.length > 0) {
            var pageText = document.body ? document.body.innerText.substring(0, 500000) : "";
            try {
              chrome.runtime.sendMessage({ type: "entities-detected", entities: detected, pageText: pageText, url: location.href, title: document.title });
              chrome.runtime.sendMessage({ type: "badge-count", count: detected.length });
            } catch(e) {}
          } else {
            try { chrome.runtime.sendMessage({ type: "badge-count", count: 0 }); } catch(e) {}
          }
        }, 2000);
      }
    }
  }

  // Listen for SPA navigation events
  window.addEventListener("popstate", checkUrlChange);
  // Intercept pushState/replaceState
  var origPushState = history.pushState;
  var origReplaceState = history.replaceState;
  history.pushState = function() { origPushState.apply(this, arguments); setTimeout(checkUrlChange, 100); };
  history.replaceState = function() { origReplaceState.apply(this, arguments); setTimeout(checkUrlChange, 100); };

  // ── Message Handling ─────────────────────────────────────────────────────

  chrome.runtime.onMessage.addListener(function(msg, sender, sendResponse) {
    if (msg.type === "scan") {
      try { chrome.runtime.sendMessage({ type: "scanning" }); } catch(e) {}
      if (isPdfPage()) {
        extractPdfText().then(function(pdfText) {
          if (pdfText.length >= 50) {
            detected = [];
            detected = detected.concat(detectGenes(pdfText));
            detected = detected.concat(detectVariants(pdfText));
            detected = detected.concat(detectAccessions(pdfText));
            detected = detected.concat(detectSpecies(pdfText));
          }
          chrome.runtime.sendMessage({
            type: "entities-detected",
            entities: detected,
            url: location.href,
            title: document.title || "PDF Document"
          });
          chrome.runtime.sendMessage({ type: "badge-count", count: detected.length });
        });
        sendResponse({ count: -1 }); // async; badge updates via message
        return true;
      }
      detected = scanPage();
      // Get page text for sidebar to run full detection (methods, drugs, etc.)
      var pageText = document.body ? document.body.innerText.substring(0, 500000) : "";
      chrome.runtime.sendMessage({
        type: "entities-detected",
        entities: detected,
        pageText: pageText,
        url: location.href,
        title: document.title
      });
      sendResponse({ count: detected.length });
      return true;
    }
    if (msg.type === "highlight") {
      highlightEntities(detected);
      sendResponse({ ok: true });
      return true;
    }
    if (msg.type === "clear-highlights") {
      clearHighlights();
      sendResponse({ ok: true });
      return true;
    }
    if (msg.type === "get-entities") {
      sendResponse({ entities: detected });
      return true;
    }
  });

  // ── PDF Support ──────────────────────────────────────────────────────────
  // Detect PDF pages and extract text for entity scanning.
  // Chrome's built-in PDF viewer uses a sandboxed <embed>, so we fetch the
  // PDF bytes directly and pull readable strings from text operators (Tj/TJ).

  function isPdfPage() {
    if (/\.pdf(\?|#|$)/i.test(location.href)) return true;
    if (document.querySelector('embed[type="application/pdf"]')) return true;
    var ct = document.contentType || "";
    if (ct.includes("application/pdf")) return true;
    return false;
  }

  async function extractPdfText() {
    try {
      var resp = await fetch(location.href);
      var buffer = await resp.arrayBuffer();
      var bytes = new Uint8Array(buffer);
      var decoder = new TextDecoder("latin1");
      var raw = decoder.decode(bytes);

      var text = "";
      var match;

      // Extract text from (string) Tj operators
      var tjRe = /\(([^)]{1,500})\)\s*Tj/g;
      while ((match = tjRe.exec(raw)) !== null) {
        text += match[1] + " ";
      }

      // Extract text from [(string)(string)] TJ arrays
      var tjArrayRe = /\[([^\]]{1,2000})\]\s*TJ/gi;
      while ((match = tjArrayRe.exec(raw)) !== null) {
        var inner = match[1];
        var strRe = /\(([^)]{1,500})\)/g;
        var m2;
        while ((m2 = strRe.exec(inner)) !== null) {
          text += m2[1];
        }
        text += " ";
      }

      // Unescape PDF string escape sequences
      text = text
        .replace(/\\n/g, "\n")
        .replace(/\\r/g, "")
        .replace(/\\t/g, " ")
        .replace(/\\\(/g, "(")
        .replace(/\\\)/g, ")")
        .replace(/\\\\/g, "\\")
        .replace(/\\(\d{3})/g, function(_, oct) {
          return String.fromCharCode(parseInt(oct, 8));
        });

      return text;
    } catch (e) {
      return "";
    }
  }

  // ── Auto-Scan Heuristic ──────────────────────────────────────────────────

  function isBioPage() {
    // Skip our own tools — they contain bio keywords but aren't research pages
    var host = document.location.hostname;
    var path = document.location.pathname;
    if (host === "lang.bio" || host === "localhost" || host === "127.0.0.1") return false;
    if (/viewer|studio|playground|biogist/i.test(path)) return false;

    var meta = "";
    var descTag = document.querySelector("meta[name='description']");
    if (descTag) meta = descTag.content || "";
    var kwTag = document.querySelector("meta[name='keywords']");
    if (kwTag) meta += " " + (kwTag.content || "");
    var text = (document.title + " " + meta + " " + host).toLowerCase();

    return /gene|genom|protein|sequenc|variant|mutation|snp|rna\b|dna\b|bioinformatics|pubmed|ncbi|biorxiv|medrxiv|nature\.com\/ng|nature\.com\/nbt|cell\.com|sciencedirect|nih\.gov|ebi\.ac\.uk|ensembl|uniprot|clinvar|omim|genbank|pdb\.org|rcsb/i.test(text);
  }

  // PDF pages: extract text and run entity detection
  if (isPdfPage()) {
    setTimeout(function() {
      extractPdfText().then(function(pdfText) {
        if (pdfText.length < 50) return;
        var pdfDetected = [];
        pdfDetected = pdfDetected.concat(detectGenes(pdfText));
        pdfDetected = pdfDetected.concat(detectVariants(pdfText));
        pdfDetected = pdfDetected.concat(detectAccessions(pdfText));
        pdfDetected = pdfDetected.concat(detectSpecies(pdfText));
        if (pdfDetected.length > 0) {
          detected = pdfDetected;
          try {
            chrome.runtime.sendMessage({
              type: "entities-detected",
              entities: detected,
              pageText: pdfText,
              url: location.href,
              title: document.title || "PDF Document"
            });
            chrome.runtime.sendMessage({ type: "badge-count", count: detected.length });
          } catch (e) {}
        }
      });
    }, 1000);
  }

  // Only auto-scan on pages that look biological; debounce until DOM stabilises
  else if (document.body && isBioPage()) {
    var scanTimeout = null;
    var hasScanned = false;

    function debouncedScan() {
      if (hasScanned) return;
      hasScanned = true;
      try { chrome.runtime.sendMessage({ type: "scanning" }); } catch(e) {}
      detected = scanPage();
      if (detected.length > 0) {
        var pageText = document.body ? document.body.innerText.substring(0, 500000) : "";
        try {
          chrome.runtime.sendMessage({
            type: "entities-detected",
            entities: detected,
            pageText: pageText,
            url: location.href,
            title: document.title
          });
          chrome.runtime.sendMessage({ type: "badge-count", count: detected.length });
        } catch (e) {}
      }
    }

    // Wait for DOM to stabilise — use MutationObserver with 2s debounce
    var observer = new MutationObserver(function() {
      clearTimeout(scanTimeout);
      scanTimeout = setTimeout(function() {
        observer.disconnect();
        debouncedScan();
      }, 2000);
    });
    observer.observe(document.body, { childList: true, subtree: true });

    // Fallback: scan after 5s even if DOM keeps changing
    setTimeout(function() {
      observer.disconnect();
      debouncedScan();
    }, 5000);
  }

  // ── Inject file into BLViewer when opened from BioGist ──
  // If we're on the viewer page with ?source=biogist, read stored file and inject
  if (location.href.includes("lang.bio/viewer") || location.href.includes("localhost") && location.href.includes("viewer")) {
    var params = new URLSearchParams(location.search);
    if (params.get("source") === "biogist") {
      chrome.storage.session.get("biogistFile", function(data) {
        if (data.biogistFile && data.biogistFile.content) {
          var f = data.biogistFile;
          // Wait for viewer to initialize, then inject the file
          var injectAttempts = 0;
          var injectInterval = setInterval(function() {
            injectAttempts++;
            var dropZone = document.getElementById("vw-drop-zone");
            if (dropZone || injectAttempts > 30) {
              clearInterval(injectInterval);
              if (dropZone) {
                // Create a synthetic file drop
                var blob = new Blob([f.content], { type: "text/plain" });
                var file = new File([blob], f.name, { type: "text/plain" });
                var dt = new DataTransfer();
                dt.items.add(file);
                var dropEvent = new DragEvent("drop", { dataTransfer: dt, bubbles: true });
                dropZone.dispatchEvent(dropEvent);
              }
              // Clean up stored file
              chrome.storage.session.remove("biogistFile");
            }
          }, 500);
        }
      });
    }
  }

})();
