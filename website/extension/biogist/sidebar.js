// BioGist — Sidebar Panel Script
// Renders detected entities, fetches details from NCBI/UniProt, manages cache.

(() => {
  "use strict";

  // --- State ---
  let allEntities = { gene: [], variant: [], accession: [], file: [], species: [] };
  let currentEntities = []; // flat list of {type, id, subtype} for Copy IDs
  let activeEntity = null;
  let searchFilter = "";
  let viewMode = "current"; // "current", "all", or a tab ID string
  let currentTabId = null;
  let pinnedEntities = new Set(); // "gene:BRCA1", "variant:rs123" etc.

  // Load pinned entities from storage
  chrome.storage.local.get("biogist_pinned", (data) => {
    if (data.biogist_pinned && Array.isArray(data.biogist_pinned)) {
      pinnedEntities = new Set(data.biogist_pinned);
    }
  });
  function savePinnedEntities() {
    chrome.storage.local.set({ biogist_pinned: Array.from(pinnedEntities) });
  }
  const collapsed = {};
  const expandedSections = {}; // tracks "Show all" state per type
  const CACHE_TTL = 24 * 60 * 60 * 1000; // 24 hours
  const CACHE_VERSION = 2; // increment to invalidate old cache

  // --- DOM refs ---
  const $sections = document.getElementById("entity-sections");
  const $loading = document.getElementById("loading");
  const $empty = document.getElementById("empty-state");
  const $detail = document.getElementById("detail-panel");
  const $detailTitle = document.getElementById("detail-title");
  const $detailBody = document.getElementById("detail-body");
  const $search = document.getElementById("search");
  const $settings = document.getElementById("settings-panel");

  // --- Entity metadata ---
  const TYPE_META = {
    gene:      { icon: "\u{1F9EC}", badge: "gene",      cls: "badge-gene",      label: "Genes" },
    variant:   { icon: "\u{1F52C}", badge: "variant",    cls: "badge-variant",   label: "Variants" },
    accession: { icon: "\u{1F4CA}", badge: "accession",  cls: "badge-accession", label: "Accessions" },
    file:      { icon: "\u{1F4C1}", badge: "file",       cls: "badge-file",      label: "Files" },
    species:   { icon: "\u{1F9AB}", badge: "species",    cls: "badge-species",   label: "Species" }
  };

  // --- Local entity detection (for PDF text, pasted text) ---
  const SIDEBAR_GENES = new Set([
    "TP53","BRCA1","BRCA2","PTEN","RB1","APC","VHL","KRAS","NRAS","BRAF",
    "PIK3CA","EGFR","ERBB2","HER2","ALK","RET","MET","ROS1","MYC","MYCN",
    "CDKN2A","CDK4","CDK6","MDM2","ATM","ATR","CHEK2","PALB2","RAD51",
    "MLH1","MSH2","MSH6","PMS2","JAK2","FLT3","KIT","ABL1","BCR",
    "NOTCH1","CTNNB1","SMAD4","STK11","NF1","NF2","IDH1","IDH2",
    "DNMT3A","TET2","NPM1","RUNX1","WT1","TERT","FGFR1","FGFR2","FGFR3",
    "MTOR","AKT1","EZH2","ARID1A","BCL2","BCL6","BTK","CFTR","HTT","DMD",
    "GBA","LRRK2","SNCA","APP","APOE","HBB","MTHFR","CYP2D6","ACE2",
    "GAPDH","ACTB","VEGFA","TNF","IL6","STAT3","SOX2","FOXL2","SHH"
  ]);
  const SIDEBAR_EXCLUDE = new Set([
    "THE","AND","FOR","NOT","WITH","FROM","BUT","ALL","ARE","WAS","SET",
    "MAP","LET","RUN","USE","AGE","END","TOP","CAN","MAY","HAS","HAD"
  ]);

  function detectEntitiesFromText(text) {
    const entities = [];
    const seen = new Set();
    let match;
    // Genes
    const geneRe = /\b([A-Z][A-Z0-9]{1,9})\b/g;
    while ((match = geneRe.exec(text)) !== null) {
      const s = match[1];
      if (SIDEBAR_GENES.has(s) && !SIDEBAR_EXCLUDE.has(s) && !seen.has("g:" + s)) {
        seen.add("g:" + s);
        entities.push({ type: "gene", id: s });
      }
    }
    // rsIDs
    const rsRe = /\b(rs\d{3,12})\b/gi;
    while ((match = rsRe.exec(text)) !== null) {
      const id = match[1].toLowerCase();
      if (!seen.has("v:" + id)) { seen.add("v:" + id); entities.push({ type: "variant", id }); }
    }
    // Accessions
    const accPats = [
      /\b(GSE\d{3,8})\b/g, /\b(SRR\d{5,10})\b/g, /\b(PRJNA\d{4,8})\b/g,
      /\b(ENSG\d{11})\b/g, /\b(PRJEB\d{4,8})\b/g, /\b(SRP\d{5,10})\b/g
    ];
    for (const re of accPats) {
      while ((match = re.exec(text)) !== null) {
        const id = match[1].toUpperCase();
        if (!seen.has("a:" + id)) { seen.add("a:" + id); entities.push({ type: "accession", id }); }
      }
    }
    // Species
    const specPats = [
      [/\b(?:Homo sapiens|human)\b/gi, "Human"],
      [/\b(?:Mus musculus|mouse)\b/gi, "Mouse"],
      [/\b(?:Drosophila melanogaster)\b/gi, "Fruit fly"],
      [/\b(?:E\. coli|Escherichia coli)\b/gi, "E. coli"],
    ];
    for (const [re, name] of specPats) {
      if (re.test(text) && !seen.has("s:" + name)) {
        seen.add("s:" + name);
        entities.push({ type: "species", id: name });
      }
    }
    return entities;
  }

  // --- Cache helpers ---
  async function cacheGet(key) {
    return new Promise((resolve) => {
      chrome.storage.local.get(key, (data) => {
        const entry = data[key];
        if (entry && Date.now() - entry.ts < CACHE_TTL) {
          resolve(entry.value);
        } else {
          resolve(null);
        }
      });
    });
  }

  async function cacheSet(key, value) {
    return new Promise((resolve) => {
      chrome.storage.local.set({ [key]: { value, ts: Date.now() } }, resolve);
    });
  }

  // --- Render ---
  function totalCount() {
    return Object.values(allEntities).reduce((s, a) => s + a.length, 0);
  }

  function filteredEntities() {
    if (!searchFilter) return allEntities;
    const q = searchFilter.toLowerCase();
    const result = {};
    for (const [type, items] of Object.entries(allEntities)) {
      if (!Array.isArray(items)) continue;
      result[type] = items.filter((e) => {
        const id = typeof e === "string" ? e : (e.id || "");
        return id.toLowerCase().includes(q);
      });
    }
    return result;
  }

  function render() {
    const entities = filteredEntities();
    const total = Object.values(entities).reduce((s, a) => s + a.length, 0);

    const hasPinned = pinnedEntities.size > 0;
    $empty.style.display = (total === 0 && !hasPinned) ? "block" : "none";
    $sections.innerHTML = "";

    // Show pinned entities section at top (always visible, regardless of tab)
    if (hasPinned) {
      const pinSection = document.createElement("div");
      pinSection.style.cssText = "padding:6px 14px;background:#1a1a2e;border-bottom:1px solid #1e293b;";
      pinSection.innerHTML = '<div style="font-size:10px;color:#fbbf24;font-weight:600;margin-bottom:4px">\u2605 Pinned</div>';
      const pinList = document.createElement("div");
      pinList.style.cssText = "display:flex;flex-wrap:wrap;gap:4px;";
      pinnedEntities.forEach(key => {
        const [pType, pName] = key.split(":");
        const chip = document.createElement("span");
        chip.style.cssText = "font-size:11px;padding:2px 8px;border-radius:10px;background:#334155;color:#e2e8f0;cursor:pointer;display:inline-flex;align-items:center;gap:4px;";
        chip.innerHTML = escapeHtml(pName) + '<span style="color:#64748b;font-size:9px">\u2715</span>';
        chip.addEventListener("click", () => selectEntity(pType, pName));
        chip.querySelector("span").addEventListener("click", (e) => {
          e.stopPropagation();
          pinnedEntities.delete(key);
          savePinnedEntities();
          render();
        });
        pinList.appendChild(chip);
      });
      pinSection.appendChild(pinList);
      $sections.appendChild(pinSection);
    }

    for (const [type, items] of Object.entries(entities)) {
      if (items.length === 0) continue;

      const meta = TYPE_META[type];
      const isCollapsed = collapsed[type] === true;

      const section = document.createElement("div");
      section.className = "section";

      // Header
      const header = document.createElement("div");
      header.className = "section-header";
      header.innerHTML = `
        <span class="section-title">${isCollapsed ? "\u25B6" : "\u25BC"} ${meta.label}</span>
        <span class="section-count">${items.length}</span>
      `;
      header.addEventListener("click", () => {
        collapsed[type] = !collapsed[type];
        render();
      });

      // Body
      const body = document.createElement("div");
      body.className = "section-body" + (isCollapsed ? " collapsed" : "");

      const ul = document.createElement("ul");
      ul.className = "entity-list";

      const MAX_VISIBLE = 5;
      const expanded = expandedSections[type] === true;
      // Sort pinned entities to top
      items.sort((a, b) => {
        const aId = typeof a === "string" ? a : (a.id || "");
        const bId = typeof b === "string" ? b : (b.id || "");
        const aPin = pinnedEntities.has(type + ":" + aId) ? 0 : 1;
        const bPin = pinnedEntities.has(type + ":" + bId) ? 0 : 1;
        return aPin - bPin;
      });
      const visibleItems = expanded ? items : items.slice(0, MAX_VISIBLE);

      for (const item of visibleItems) {
        const name = typeof item === "string" ? item : (item.id || String(item));
        const count = (typeof item === "object" && item.count) ? item.count : null;
        const source = (viewMode === "all" && typeof item === "object" && item.source) ? item.source : null;
        const li = document.createElement("li");
        li.className = "entity-item" + (activeEntity === `${type}:${name}` ? " active" : "");
        li.innerHTML = `
          <span class="entity-icon">${meta.icon}</span>
          <span class="entity-name">${escapeHtml(name)}</span>
          ${count && count > 1 ? '<span style="font-size:10px;color:#64748b;margin-left:2px">\u00d7' + count + '</span>' : ''}
          <span class="entity-badge ${meta.cls}">${meta.badge}</span>
        ` + (source ? `<div style="font-size:9px;color:#475569;margin-top:1px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;width:100%">${escapeHtml(source.substring(0, 60))}</div>` : '');
        li.addEventListener("click", () => selectEntity(type, name, item));

        // Pin/bookmark button
        const pinBtn = document.createElement("button");
        const isPinned = pinnedEntities.has(type + ":" + name);
        pinBtn.textContent = isPinned ? "\u2605" : "\u2606";
        pinBtn.title = isPinned ? "Unpin" : "Pin entity";
        pinBtn.style.cssText = "background:none;border:none;cursor:pointer;font-size:13px;padding:1px 3px;flex-shrink:0;color:" + (isPinned ? "#fbbf24" : "#475569") + ";";
        pinBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          const key = type + ":" + name;
          if (pinnedEntities.has(key)) {
            pinnedEntities.delete(key);
          } else {
            pinnedEntities.add(key);
          }
          savePinnedEntities();
          render();
        });
        li.appendChild(pinBtn);

        // Quick copy button
        const copyIcon = document.createElement("button");
        copyIcon.textContent = "\u{1F4CB}";
        copyIcon.title = "Copy " + name;
        copyIcon.style.cssText = "background:none;border:none;cursor:pointer;font-size:11px;padding:1px 3px;opacity:0.4;flex-shrink:0;";
        copyIcon.addEventListener("mouseenter", () => { copyIcon.style.opacity = "1"; });
        copyIcon.addEventListener("mouseleave", () => { copyIcon.style.opacity = "0.4"; });
        copyIcon.addEventListener("click", (e) => {
          e.stopPropagation();
          navigator.clipboard.writeText(name);
          showToast("Copied: " + name);
        });
        li.appendChild(copyIcon);

        // Inline "Open in BLViewer" button for file entities
        if (type === "file") {
          const viewBtn = document.createElement("button");
          viewBtn.textContent = "\u{1F441}";
          viewBtn.title = "Open in BLViewer";
          viewBtn.style.cssText = "background:#7c3aed;color:#fff;border:none;padding:2px 6px;border-radius:4px;cursor:pointer;font-size:12px;flex-shrink:0;margin-left:4px;";
          viewBtn.addEventListener("click", (e) => {
            e.stopPropagation();
            openInBLViewer(name);
          });
          li.appendChild(viewBtn);
        }

        ul.appendChild(li);
      }

      // "Show all" / "Show less" toggle
      if (items.length > MAX_VISIBLE) {
        const toggle = document.createElement("li");
        toggle.className = "entity-item";
        toggle.style.cssText = "justify-content:center;color:#06b6d4;cursor:pointer;font-size:11px;font-weight:600;opacity:0.8;";
        toggle.textContent = expanded ? "Show less" : `Show all ${items.length}`;
        toggle.addEventListener("click", () => {
          expandedSections[type] = !expanded;
          render();
        });
        ul.appendChild(toggle);
      }

      body.appendChild(ul);
      section.appendChild(header);
      section.appendChild(body);
      $sections.appendChild(section);
    }
  }

  function escapeHtml(s) {
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

  // --- Open file in BLViewer ---
  function openInBLViewer(url) {
    // Remove any existing file action panel
    var existing = document.getElementById("file-action-panel");
    if (existing) existing.remove();

    var name = url.split("/").pop().split("?")[0] || "file";
    try { name = decodeURIComponent(name); } catch(e) {}

    var panel = document.createElement("div");
    panel.id = "file-action-panel";
    panel.style.cssText = "position:fixed;bottom:0;left:0;right:0;background:#0f172a;border-top:1px solid #1e293b;padding:12px 14px;z-index:30;";
    panel.innerHTML =
      '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px">' +
        '<div style="font-size:12px;color:#94a3b8;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1;margin-right:8px">' + escapeHtml(name) + '</div>' +
        '<button class="file-panel-close" style="background:none;border:none;color:#64748b;cursor:pointer;font-size:16px;padding:0 4px">&times;</button>' +
      '</div>' +
      '<div style="display:flex;gap:8px">' +
        '<button class="file-panel-download" style="flex:1;padding:6px;border-radius:6px;border:none;background:#06b6d4;color:#fff;font-size:12px;font-weight:600;cursor:pointer">Download</button>' +
        '<button class="file-panel-copy" style="flex:1;padding:6px;border-radius:6px;border:none;background:#334155;color:#e2e8f0;font-size:12px;font-weight:600;cursor:pointer">Copy URL</button>' +
      '</div>' +
      '<div style="font-size:10px;color:#475569;margin-top:6px">Download and drop into BLViewer, or copy URL to paste in viewer.</div>';
    document.body.appendChild(panel);

    panel.querySelector(".file-panel-download").addEventListener("click", () => {
      showToast("Downloading...");
      fetch(url).then(r => r.blob()).then(blob => {
        var a = document.createElement("a");
        a.href = URL.createObjectURL(blob);
        a.download = name;
        a.click();
        setTimeout(() => URL.revokeObjectURL(a.href), 1000);
        showToast("Downloaded — drop it into BLViewer");
        panel.remove();
      }).catch(() => {
        navigator.clipboard.writeText(url);
        showToast("Download failed — URL copied instead");
        panel.remove();
      });
    });

    panel.querySelector(".file-panel-copy").addEventListener("click", () => {
      navigator.clipboard.writeText(url).then(() => {
        showToast("URL copied");
        panel.remove();
      });
    });

    panel.querySelector(".file-panel-close").addEventListener("click", () => panel.remove());
  }

  // --- Entity selection & detail fetch ---
  async function selectEntity(type, name, entityObj) {
    activeEntity = `${type}:${name}`;
    render();
    $detailTitle.textContent = name;
    $detailBody.innerHTML = '<div class="spinner-wrap"><div class="spinner"></div></div>';
    document.getElementById("detail-panel").classList.add("active");

    try {
      const info = await fetchEntityInfo(type, name);
      renderDetail(type, name, info, entityObj);
    } catch (err) {
      // Show error with retry button
      $detailBody.innerHTML = `
        <div class="detail-placeholder" style="text-align:center">
          <div style="color:#f87171;margin-bottom:8px">Failed to load details</div>
          <div style="font-size:11px;color:#64748b;margin-bottom:12px">${escapeHtml(err.message)}</div>
          <button id="retry-fetch" style="padding:5px 14px;border-radius:6px;border:none;background:#06b6d4;color:#fff;font-size:12px;font-weight:600;cursor:pointer">Retry</button>
        </div>`;
      document.getElementById("retry-fetch").addEventListener("click", () => {
        // Clear cache for this entity and retry
        const cacheKey = `biogist:v${CACHE_VERSION}:${type}:${name}`;
        chrome.storage.local.remove(cacheKey);
        selectEntity(type, name, entityObj);
      });
    }
  }

  function renderDetail(type, name, info, entityObj) {
    if (!info) {
      $detailBody.innerHTML = '<div class="detail-placeholder">No information found.</div>';
      return;
    }

    let html = "";

    // Show snippet context if available
    if (entityObj && entityObj.snippet) {
      html += `<div style="background:#020617;border:1px solid #1e293b;border-radius:6px;padding:8px;margin-bottom:10px;font-size:11px;color:#94a3b8;line-height:1.5;font-style:italic">"${escapeHtml(entityObj.snippet)}"</div>`;
    }
    // Show occurrence count
    if (entityObj && entityObj.count && entityObj.count > 1) {
      html += `<div style="font-size:11px;color:#64748b;margin-bottom:8px">Mentioned <strong style="color:#e2e8f0">${entityObj.count} times</strong> on this page</div>`;
    }

    for (const [label, value] of Object.entries(info)) {
      if (value === null || value === undefined || value === "") continue;
      if (label === "Link") continue; // We'll add source links separately at the bottom
      const displayVal = typeof value === "string" && value.startsWith("http")
        ? `<a href="${escapeHtml(value)}" target="_blank" rel="noopener" style="word-break:break-all">${escapeHtml(value)}</a>`
        : escapeHtml(String(value));
      html += `<div class="detail-row">
        <span class="detail-label">${escapeHtml(label)}</span>
        <span class="detail-value">${displayVal}</span>
      </div>`;
    }

    // Source links section
    const links = buildSourceLinks(type, name, info);
    if (links.length > 0) {
      html += '<div style="margin-top:10px;padding-top:8px;border-top:1px solid #1e293b">';
      html += '<div style="font-size:10px;color:#475569;text-transform:uppercase;font-weight:600;margin-bottom:6px">View on</div>';
      html += '<div style="display:flex;flex-wrap:wrap;gap:6px">';
      links.forEach(l => {
        html += `<a href="${escapeHtml(l.url)}" target="_blank" rel="noopener" style="font-size:11px;color:#06b6d4;background:#0f172a;border:1px solid #1e293b;padding:3px 8px;border-radius:4px;text-decoration:none">${escapeHtml(l.label)}</a>`;
      });
      html += '</div></div>';
    }

    $detailBody.innerHTML = html || '<div class="detail-placeholder">No additional details available.</div>';

    // SRA download command for SRR accessions
    if (type === "accession" && /^SRR\d+$/i.test(name)) {
      const cmd = "fasterq-dump " + name + " --split-3 -e 8";
      const cmdDiv = document.createElement("div");
      cmdDiv.style.cssText = "margin-top:8px;background:#020617;border:1px solid #334155;border-radius:6px;padding:8px;position:relative;";
      cmdDiv.innerHTML = `<div style="font-size:10px;color:#64748b;text-transform:uppercase;font-weight:600;margin-bottom:4px;">Download Command</div><code style="font-size:11px;color:#a5f3fc;word-break:break-all;">${escapeHtml(cmd)}</code>`;
      const copyBtn = document.createElement("button");
      copyBtn.textContent = "Copy";
      copyBtn.style.cssText = "position:absolute;top:6px;right:6px;background:#1e293b;color:#cbd5e1;border:none;padding:3px 8px;border-radius:4px;cursor:pointer;font-size:10px;font-weight:600;";
      copyBtn.addEventListener("click", () => {
        navigator.clipboard.writeText(cmd);
        showToast("Command copied");
      });
      cmdDiv.appendChild(copyBtn);
      $detailBody.appendChild(cmdDiv);
    }

    // Copy Details button — copies all info as text
    if (info && Object.keys(info).length > 0) {
      const copyDiv = document.createElement("div");
      copyDiv.style.cssText = "margin-top:10px;padding-top:8px;border-top:1px solid #1e293b;display:flex;gap:6px;";
      const copyTextBtn = document.createElement("button");
      copyTextBtn.textContent = "Copy Details";
      copyTextBtn.style.cssText = "flex:1;padding:5px;border-radius:6px;border:none;background:#334155;color:#e2e8f0;font-size:11px;font-weight:600;cursor:pointer;";
      copyTextBtn.addEventListener("click", () => {
        let text = name + " (" + type + ")\n";
        if (entityObj && entityObj.count > 1) text += "Mentioned " + entityObj.count + " times\n";
        for (const [k, v] of Object.entries(info)) {
          if (v && k !== "Link") text += k + ": " + v + "\n";
        }
        navigator.clipboard.writeText(text.trim());
        showToast("Details copied");
      });
      copyDiv.appendChild(copyTextBtn);
      $detailBody.appendChild(copyDiv);
    }
  }

  // --- Source links builder ---
  function buildSourceLinks(type, name, info) {
    const links = [];
    const geneId = info && info["Gene ID"];
    const taxid = info && info["Taxonomy ID"];

    if (type === "gene") {
      if (geneId) links.push({ label: "NCBI Gene", url: `https://www.ncbi.nlm.nih.gov/gene/${geneId}` });
      links.push({ label: "Ensembl", url: `https://www.ensembl.org/Homo_sapiens/Gene/Summary?g=${name}` });
      if (info && info["UniProt"]) links.push({ label: "UniProt", url: `https://www.uniprot.org/uniprot/${info["UniProt"]}` });
      links.push({ label: "GeneCards", url: `https://www.genecards.org/cgi-bin/carddisp.pl?gene=${name}` });
    } else if (type === "variant") {
      if (/^rs\d+$/i.test(name)) {
        links.push({ label: "dbSNP", url: `https://www.ncbi.nlm.nih.gov/snp/${name}` });
        links.push({ label: "gnomAD", url: `https://gnomad.broadinstitute.org/variant/${name}` });
        links.push({ label: "ClinVar", url: `https://www.ncbi.nlm.nih.gov/clinvar/?term=${name}` });
      } else if (/^VCV/i.test(name)) {
        links.push({ label: "ClinVar", url: `https://www.ncbi.nlm.nih.gov/clinvar/variation/${name.replace("VCV", "")}` });
      } else if (/^COSM/i.test(name)) {
        links.push({ label: "COSMIC", url: `https://cancer.sanger.ac.uk/cosmic/mutation/overview?id=${name.replace("COSM", "")}` });
      }
    } else if (type === "accession") {
      if (/^GSE\d+$/i.test(name)) links.push({ label: "GEO", url: `https://www.ncbi.nlm.nih.gov/geo/query/acc.cgi?acc=${name}` });
      if (/^SRR\d+$/i.test(name)) links.push({ label: "SRA", url: `https://www.ncbi.nlm.nih.gov/sra/${name}` });
      if (/^SRP\d+$/i.test(name)) links.push({ label: "SRA Project", url: `https://www.ncbi.nlm.nih.gov/sra/?term=${name}` });
      if (/^PRJNA\d+$/i.test(name)) links.push({ label: "BioProject", url: `https://www.ncbi.nlm.nih.gov/bioproject/${name}` });
      if (/^PRJEB\d+$/i.test(name)) links.push({ label: "ENA", url: `https://www.ebi.ac.uk/ena/browser/view/${name}` });
      if (/^ENSG\d+$/i.test(name)) links.push({ label: "Ensembl", url: `https://www.ensembl.org/id/${name}` });
      if (/^ENST\d+$/i.test(name)) links.push({ label: "Ensembl", url: `https://www.ensembl.org/id/${name}` });
      if (/^NM_/i.test(name)) links.push({ label: "RefSeq", url: `https://www.ncbi.nlm.nih.gov/nuccore/${name}` });
      if (/^10\.\d{4}/.test(name)) links.push({ label: "DOI", url: `https://doi.org/${name}` });
    } else if (type === "species") {
      if (taxid) links.push({ label: "NCBI Taxonomy", url: `https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?id=${taxid}` });
      if (taxid) links.push({ label: "Ensembl", url: `https://www.ensembl.org/id/${taxid}` });
    }
    return links;
  }

  // --- API fetches ---
  const NCBI_BASE = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";

  // Fetch with timeout (10s default)
  async function fetchWithTimeout(url, timeoutMs) {
    timeoutMs = timeoutMs || 10000;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      const resp = await fetch(url, { signal: controller.signal });
      clearTimeout(timer);
      if (!resp.ok) throw new Error("HTTP " + resp.status);
      return await resp.json();
    } catch (e) {
      clearTimeout(timer);
      if (e.name === "AbortError") throw new Error("Request timed out");
      throw e;
    }
  }

  async function fetchEntityInfo(type, name) {
    const cacheKey = `biogist:v${CACHE_VERSION}:${type}:${name}`;
    const cached = await cacheGet(cacheKey);
    if (cached) return cached;

    let info = null;
    switch (type) {
      case "gene":     info = await fetchGeneInfo(name); break;
      case "variant":  info = await fetchVariantInfo(name); break;
      case "accession": info = await fetchAccessionInfo(name); break;
      case "file":     info = describeFile(name); break;
      case "species":  info = await fetchSpeciesInfo(name); break;
    }

    if (info) await cacheSet(cacheKey, info);
    return info;
  }

  async function fetchGeneInfo(symbol) {
    const searchUrl = `${NCBI_BASE}/esearch.fcgi?db=gene&term=${encodeURIComponent(symbol)}[sym]+AND+human[orgn]&retmode=json`;
    const searchRes = await fetchWithTimeout(searchUrl);
    const ids = searchRes?.esearchresult?.idlist;
    if (!ids || ids.length === 0) return { Symbol: symbol, Status: "Not found in NCBI Gene" };

    const geneId = ids[0];
    const sumUrl = `${NCBI_BASE}/esummary.fcgi?db=gene&id=${geneId}&retmode=json`;
    const sumRes = await fetchWithTimeout(sumUrl);
    const doc = sumRes?.result?.[geneId];
    if (!doc) return { Symbol: symbol, "Gene ID": geneId };

    const info = {
      Symbol: doc.name || symbol,
      Name: doc.description || "",
      "Gene ID": geneId,
      Organism: doc.organism?.scientificname || "",
      Chromosome: doc.chromosome || "",
      "Map Location": doc.maplocation || "",
      Summary: truncate(doc.summary || "", 300),
      Link: `https://www.ncbi.nlm.nih.gov/gene/${geneId}`
    };

    // Fetch UniProt protein summary
    try {
      const uniprotData = await fetchUniprotForGene(symbol);
      if (uniprotData) {
        const acc = uniprotData.primaryAccession;
        if (acc) info["UniProt"] = acc;
        // Protein name
        const protName = uniprotData.proteinDescription?.recommendedName?.fullName?.value;
        if (protName) info["Protein"] = protName;
        // Function (from comments)
        const funcComments = (uniprotData.comments || []).filter(c => c.commentType === "FUNCTION");
        if (funcComments.length > 0 && funcComments[0].texts && funcComments[0].texts.length > 0) {
          info["Function"] = truncate(funcComments[0].texts[0].value, 250);
        }
        // Subcellular location
        const locComments = (uniprotData.comments || []).filter(c => c.commentType === "SUBCELLULAR LOCATION");
        if (locComments.length > 0 && locComments[0].subcellularLocations) {
          const locs = locComments[0].subcellularLocations.map(l => l.location?.value).filter(Boolean);
          if (locs.length > 0) info["Subcellular Location"] = locs.join(", ");
        }
        if (acc) info["UniProt Link"] = `https://www.uniprot.org/uniprotkb/${acc}`;
      }
    } catch (_) { /* UniProt fetch failed, continue with NCBI data */ }

    return info;
  }

  async function fetchUniprotForGene(geneSymbol) {
    const url = `https://rest.uniprot.org/uniprotkb/search?query=gene_exact:${encodeURIComponent(geneSymbol)}+AND+organism_id:9606&format=json&size=1&fields=accession,protein_name,cc_function,cc_subcellular_location`;
    const resp = await fetch(url);
    const data = await resp.json();
    if (data.results && data.results.length > 0) {
      return data.results[0];
    }
    return null;
  }

  async function fetchVariantInfo(rsid) {
    const cleanId = rsid.replace(/^rs/i, "");
    const searchUrl = `${NCBI_BASE}/esearch.fcgi?db=snp&term=${encodeURIComponent(rsid)}&retmode=json`;
    const searchRes = await fetchWithTimeout(searchUrl);
    const ids = searchRes?.esearchresult?.idlist;
    if (!ids || ids.length === 0) return { rsID: rsid, Status: "Not found in dbSNP" };

    const snpId = ids[0];
    const info = {
      rsID: `rs${snpId}`,
      "dbSNP ID": snpId,
      Link: `https://www.ncbi.nlm.nih.gov/snp/rs${snpId}`,
      ClinVar: `https://www.ncbi.nlm.nih.gov/clinvar/?term=rs${snpId}`
    };

    // Fetch gnomAD allele frequency and ClinVar significance via myvariant.info
    try {
      const mvUrl = `https://myvariant.info/v1/query?q=${encodeURIComponent(rsid)}&fields=gnomad_genome.af,clinvar.rcv,dbsnp.gene&size=1`;
      const mvRes = await fetchWithTimeout(mvUrl);
      if (mvRes.hits && mvRes.hits.length > 0) {
        const hit = mvRes.hits[0];
        // gnomAD allele frequency
        if (hit.gnomad_genome && hit.gnomad_genome.af) {
          const af = hit.gnomad_genome.af.af;
          if (af !== undefined && af !== null) {
            info["gnomAD AF"] = typeof af === "number" ? af.toExponential(3) : String(af);
          }
        }
        // ClinVar clinical significance
        if (hit.clinvar && hit.clinvar.rcv) {
          const rcv = Array.isArray(hit.clinvar.rcv) ? hit.clinvar.rcv : [hit.clinvar.rcv];
          const sigs = rcv.map(r => r.clinical_significance).filter(Boolean);
          const unique = [...new Set(sigs)];
          if (unique.length > 0) info["Clinical Significance"] = unique.join(", ");
        }
        // Gene from dbSNP
        if (hit.dbsnp && hit.dbsnp.gene) {
          const genes = Array.isArray(hit.dbsnp.gene) ? hit.dbsnp.gene : [hit.dbsnp.gene];
          const symbols = genes.map(g => g.symbol).filter(Boolean);
          if (symbols.length > 0) info["Gene"] = symbols.join(", ");
        }
      }
    } catch (_) { /* myvariant.info fetch failed, continue with basic info */ }

    return info;
  }

  async function fetchAccessionInfo(accession) {
    const prefix = accession.replace(/\d+$/, "").toUpperCase();
    const numericId = accession.replace(/^[A-Z]+/i, "");

    // Determine database from prefix
    const dbMap = {
      GSE: "gds", GSM: "gds", GDS: "gds",
      SRR: "sra", SRX: "sra", SRP: "sra",
      ERR: "sra", ERX: "sra", ERP: "sra",
      DRR: "sra", DRX: "sra", DRP: "sra",
      PRJNA: "bioproject", PRJEB: "bioproject", PRJDB: "bioproject"
    };

    const db = dbMap[prefix] || "nuccore";
    const searchUrl = `${NCBI_BASE}/esearch.fcgi?db=${db}&term=${encodeURIComponent(accession)}&retmode=json`;
    const searchRes = await fetchWithTimeout(searchUrl);
    const ids = searchRes?.esearchresult?.idlist;

    const info = { Accession: accession, Database: db.toUpperCase() };

    if (ids && ids.length > 0) {
      const uid = ids[0];
      info["NCBI UID"] = uid;

      try {
        const sumUrl = `${NCBI_BASE}/esummary.fcgi?db=${db}&id=${uid}&retmode=json`;
        const sumRes = await fetchWithTimeout(sumUrl);
        const doc = sumRes?.result?.[uid];
        if (doc) {
          if (doc.title) info.Title = truncate(doc.title, 200);
          if (doc.summary) info.Summary = truncate(doc.summary, 300);
          if (doc.gse) info.Series = doc.gse;
          if (doc.gpl) info.Platform = doc.gpl;
          if (doc.n_samples) info.Samples = doc.n_samples;
          if (doc.taxon) info.Organism = doc.taxon;
        }
      } catch (_) { /* summary fetch failed, continue with basic info */ }
    }

    // Add links
    if (prefix.startsWith("GSE")) {
      info.Link = `https://www.ncbi.nlm.nih.gov/geo/query/acc.cgi?acc=${accession}`;
    } else if (db === "sra") {
      info.Link = `https://www.ncbi.nlm.nih.gov/sra/${accession}`;
    } else if (db === "bioproject") {
      info.Link = `https://www.ncbi.nlm.nih.gov/bioproject/${accession}`;
    }

    return info;
  }

  function describeFile(filename) {
    const ext = filename.replace(/\.gz$/i, "").split(".").pop().toLowerCase();
    const descriptions = {
      fastq: "FASTQ sequencing reads",
      fq: "FASTQ sequencing reads",
      bam: "Binary Alignment Map",
      sam: "Sequence Alignment Map",
      vcf: "Variant Call Format",
      bed: "BED genomic intervals",
      gff: "General Feature Format",
      gtf: "Gene Transfer Format",
      fasta: "FASTA sequence",
      fa: "FASTA sequence",
      cram: "Compressed Reference Alignment Map"
    };
    const compressed = filename.endsWith(".gz");
    return {
      Filename: filename,
      Format: (descriptions[ext] || ext.toUpperCase()),
      Compressed: compressed ? "Yes (gzip)" : "No"
    };
  }

  const SPECIES_DATA = {
    "Human":      { taxid: "9606",  latin: "Homo sapiens",                genome: "GRCh38.p14",  genes: "~20,000 protein-coding", chromosomes: "23 pairs" },
    "Mouse":      { taxid: "10090", latin: "Mus musculus",                genome: "GRCm39",       genes: "~22,000 protein-coding", chromosomes: "20 pairs" },
    "Rat":        { taxid: "10116", latin: "Rattus norvegicus",           genome: "mRatBN7.2",    genes: "~22,000 protein-coding", chromosomes: "21 pairs" },
    "Fruit fly":  { taxid: "7227",  latin: "Drosophila melanogaster",     genome: "BDGP6",        genes: "~14,000 protein-coding", chromosomes: "4 pairs" },
    "C. elegans": { taxid: "6239",  latin: "Caenorhabditis elegans",      genome: "WBcel235",     genes: "~20,000 protein-coding", chromosomes: "6" },
    "Zebrafish":  { taxid: "7955",  latin: "Danio rerio",                 genome: "GRCz11",       genes: "~25,000 protein-coding", chromosomes: "25 pairs" },
    "Yeast":      { taxid: "4932",  latin: "Saccharomyces cerevisiae",    genome: "R64",          genes: "~6,000",                 chromosomes: "16" },
    "Arabidopsis":{ taxid: "3702",  latin: "Arabidopsis thaliana",        genome: "TAIR10",       genes: "~27,000 protein-coding", chromosomes: "5 pairs" },
    "E. coli":    { taxid: "562",   latin: "Escherichia coli",            genome: "K-12 MG1655",  genes: "~4,300",                 chromosomes: "1 circular" },
  };

  async function fetchSpeciesInfo(name) {
    const data = SPECIES_DATA[name];
    if (!data) return { Species: name, Status: "Unknown species" };

    const info = {
      "Common Name": name,
      "Scientific Name": data.latin,
      "Taxonomy ID": data.taxid,
      "Reference Genome": data.genome,
      "Protein-coding Genes": data.genes,
      "Chromosomes": data.chromosomes,
    };

    // Try to fetch additional info from NCBI taxonomy
    try {
      const url = `${NCBI_BASE}/esummary.fcgi?db=taxonomy&id=${data.taxid}&retmode=json`;
      const json = await fetchWithTimeout(url);
      const doc = json?.result?.[data.taxid];
      if (doc) {
        if (doc.division) info["Division"] = doc.division;
        if (doc.geneticcode?.gcname) info["Genetic Code"] = doc.geneticcode.gcname;
        if (doc.lineage) info["Lineage"] = truncate(doc.lineage, 200);
      }
    } catch (e) {
      // NCBI fetch failed — use static data only
    }

    return info;
  }

  function truncate(s, max) {
    return s.length > max ? s.slice(0, max) + "..." : s;
  }

  // --- Toast notification ---
  function showToast(message) {
    let toast = document.createElement("div");
    toast.textContent = message;
    toast.style.cssText = "position:fixed;bottom:16px;left:50%;transform:translateX(-50%);background:#7c3aed;color:#fff;padding:6px 14px;border-radius:6px;font-size:12px;font-weight:600;z-index:100;opacity:1;transition:opacity 0.3s;";
    document.body.appendChild(toast);
    setTimeout(() => { toast.style.opacity = "0"; setTimeout(() => toast.remove(), 300); }, 1800);
  }

  // --- Export button ---
  const $copyBtn = document.createElement("button");
  $copyBtn.className = "btn";
  $copyBtn.textContent = "Export";
  $copyBtn.title = "Export findings";
  $copyBtn.addEventListener("click", () => {
    if (currentEntities.length === 0) { showToast("No entities to export"); return; }

    // Remove existing export menu
    const existingMenu = document.getElementById("export-menu");
    if (existingMenu) { existingMenu.remove(); return; }

    const menu = document.createElement("div");
    menu.id = "export-menu";
    menu.style.cssText = "position:fixed;top:50px;right:10px;background:#1e293b;border:1px solid #334155;border-radius:8px;padding:4px;z-index:40;box-shadow:0 4px 12px rgba(0,0,0,0.3);";

    // Build full export object with cached details
    async function buildFullExport() {
      const obj = {};
      for (const [type, items] of Object.entries(allEntities)) {
        if (!Array.isArray(items) || items.length === 0) continue;
        const entries = [];
        for (const item of items) {
          const id = typeof item === "string" ? item : item.id;
          const entry = { id };
          // Try to get cached details
          const cached = await cacheGet(`biogist:v${CACHE_VERSION}:${type}:${id}`);
          if (cached) entry.details = cached;
          entries.push(entry);
        }
        obj[type] = entries;
      }
      // Add source URL and timestamp
      obj._source = await new Promise(r => {
        chrome.tabs.query({ active: true, currentWindow: true }, tabs => {
          r(tabs[0] ? { url: tabs[0].url, title: tabs[0].title } : {});
        });
      });
      obj._exported = new Date().toISOString();
      return obj;
    }

    function buildMarkdown(obj) {
      let md = "## BioGist Scan Results\n\n";
      if (obj._source) {
        md += "**Source:** " + (obj._source.title || "") + "\n";
        md += obj._source.url + "\n\n";
      }
      if (obj._exported) md += "*Exported: " + obj._exported + "*\n\n";
      for (const [type, entries] of Object.entries(obj)) {
        if (type.startsWith("_") || !entries || !Array.isArray(entries) || entries.length === 0) continue;
        md += "### " + (TYPE_META[type] ? TYPE_META[type].label : type) + "\n\n";
        for (const entry of entries) {
          md += "**" + entry.id + "**\n";
          if (entry.details) {
            for (const [k, v] of Object.entries(entry.details)) {
              if (v && !String(v).startsWith("http"))
                md += "- " + k + ": " + v + "\n";
            }
          }
          md += "\n";
        }
      }
      return md;
    }

    const options = [
      { label: "Copy IDs", action: () => {
        navigator.clipboard.writeText(currentEntities.map(e => e.id).join("\n"));
        showToast("Copied " + currentEntities.length + " IDs");
      }},
      { label: "Copy as JSON (with details)", action: async () => {
        showToast("Building export...");
        const obj = await buildFullExport();
        navigator.clipboard.writeText(JSON.stringify(obj, null, 2));
        showToast("Copied JSON with details");
      }},
      { label: "Copy as Markdown", action: async () => {
        showToast("Building export...");
        const obj = await buildFullExport();
        navigator.clipboard.writeText(buildMarkdown(obj));
        showToast("Copied Markdown");
      }},
      { label: "Download JSON", action: async () => {
        showToast("Building export...");
        const obj = await buildFullExport();
        const blob = new Blob([JSON.stringify(obj, null, 2)], { type: "application/json" });
        const a = document.createElement("a");
        a.href = URL.createObjectURL(blob);
        a.download = "biogist-scan.json";
        a.click();
        showToast("Downloaded biogist-scan.json");
      }},
    ];

    options.forEach(opt => {
      const btn = document.createElement("button");
      btn.textContent = opt.label;
      btn.style.cssText = "display:block;width:100%;text-align:left;padding:6px 12px;background:none;border:none;color:#e2e8f0;font-size:12px;cursor:pointer;border-radius:4px;";
      btn.addEventListener("mouseenter", () => { btn.style.background = "#334155"; });
      btn.addEventListener("mouseleave", () => { btn.style.background = "none"; });
      btn.addEventListener("click", () => { opt.action(); menu.remove(); });
      menu.appendChild(btn);
    });

    document.body.appendChild(menu);
    // Close on outside click
    setTimeout(() => {
      document.addEventListener("click", function closeMenu(e) {
        if (!menu.contains(e.target)) { menu.remove(); document.removeEventListener("click", closeMenu); }
      });
    }, 100);
  });
  document.querySelector(".header-actions").insertBefore($copyBtn, document.getElementById("btn-settings"));

  // --- Paste text button ---
  document.getElementById("btn-paste-text").addEventListener("click", () => {
    const existing = document.getElementById("paste-panel");
    if (existing) { existing.remove(); return; }

    const panel = document.createElement("div");
    panel.id = "paste-panel";
    panel.style.cssText = "padding:10px 14px;border-bottom:1px solid #1e293b;background:#0f172a;";
    panel.innerHTML =
      '<textarea id="paste-textarea" placeholder="Paste text from a PDF, paper, or any source..." style="width:100%;height:80px;padding:8px;border-radius:6px;border:1px solid #334155;background:#020617;color:#e2e8f0;font-size:12px;resize:vertical;font-family:system-ui,sans-serif;"></textarea>' +
      '<button id="paste-scan" style="margin-top:6px;width:100%;padding:5px;border-radius:6px;border:none;background:#06b6d4;color:#fff;font-size:12px;font-weight:600;cursor:pointer;">Scan Text</button>';

    const searchBar = document.querySelector(".search-bar");
    searchBar.parentNode.insertBefore(panel, searchBar);

    document.getElementById("paste-scan").addEventListener("click", () => {
      const text = document.getElementById("paste-textarea").value.trim();
      if (!text || text.length < 10) { showToast("Paste some text first"); return; }
      const entities = detectEntitiesFromText(text);
      loadEntitiesFromArray(entities);
      showToast("Found " + entities.length + " entities");
      panel.remove();
      render();
    });
  });

  // --- Scan button ---
  // --- Helper: send scan to tab, then pull results ---
  function scanTab(tabId) {
    $loading.style.display = "flex";
    $empty.style.display = "none";

    function pullResults() {
      let attempts = 0;
      const poll = setInterval(() => {
        attempts++;
        chrome.runtime.sendMessage({ type: "get-tab-entities" }, (resp) => {
          if (chrome.runtime.lastError) return;
          if (resp && resp.tabId === tabId && resp.entities && resp.entities.length > 0) {
            clearInterval(poll);
            $loading.style.display = "none";
            loadEntitiesFromArray(resp.entities);
            refreshTabDropdown();
            render();
          } else if (attempts >= 12) {
            clearInterval(poll);
            $loading.style.display = "none";
            loadEntitiesFromArray([]);
            render();
            showToast("No entities found");
          }
        });
      }, 500);
    }

    // Try sending scan
    chrome.tabs.sendMessage(tabId, { type: "scan" }, (resp) => {
      if (chrome.runtime.lastError) {
        // Content script not connected — page was open before extension loaded
        $loading.style.display = "none";
        $empty.style.display = "block";
        $empty.querySelector("strong").textContent = "Page needs reload";
        $empty.querySelector("p").innerHTML = 'This page was opened before BioGist. <button id="btn-reload-tab" style="background:#06b6d4;color:#fff;border:none;padding:4px 12px;border-radius:4px;cursor:pointer;font-size:12px;font-weight:600;margin-top:6px">Reload & Scan</button>';
        document.getElementById("btn-reload-tab").addEventListener("click", () => {
          chrome.tabs.reload(tabId, {}, () => {
            $empty.style.display = "none";
            $loading.style.display = "flex";
            // Wait for page reload + content script init, then scan
            setTimeout(() => scanTab(tabId), 3000);
          });
        });
        return;
      }
      // Content script responded — poll for results
      pullResults();
    });
  }

  document.getElementById("btn-scan").addEventListener("click", () => {
    viewMode = "current";
    $tabSelect.value = "current";
    $empty.querySelector("strong").textContent = "No entities detected";
    $empty.querySelector("p").innerHTML = 'Click <b>Scan</b> to analyze the current page.';

    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (!tabs[0]) return;
      currentTabId = tabs[0].id;
      scanTab(tabs[0].id);
    });
  });

  // --- Close detail (back to list) ---
  document.getElementById("btn-close-detail").addEventListener("click", () => {
    document.getElementById("detail-panel").classList.remove("active");
    activeEntity = null;
    render();
  });

  // --- Merge from all tabs ---
  // --- Tab dropdown ---
  const $tabSelect = document.getElementById("tab-select");
  const $tabCount = document.getElementById("tab-count");

  function refreshTabDropdown() {
    chrome.runtime.sendMessage({ type: "get-all-tab-entities" }, (resp) => {
      if (chrome.runtime.lastError || !resp) return;
      const currentVal = $tabSelect.value;
      $tabSelect.innerHTML = '<option value="current">Current tab</option>';
      if (resp.sources && resp.sources.length > 0) {
        $tabSelect.innerHTML += '<option value="all">All tabs (' + resp.entities.length + ')</option>';
        resp.sources.forEach(s => {
          const title = (s.title || "").substring(0, 40);
          $tabSelect.innerHTML += '<option value="' + s.tabId + '">' + escapeHtml(title) + ' (' + s.count + ')</option>';
        });
        $tabCount.textContent = resp.sources.length + " scanned";
      } else {
        $tabCount.textContent = "";
      }
      // Restore selection if still valid
      if (currentVal && $tabSelect.querySelector('option[value="' + currentVal + '"]')) {
        $tabSelect.value = currentVal;
      }
    });
  }

  $tabSelect.addEventListener("change", () => {
    viewMode = $tabSelect.value;
    document.getElementById("detail-panel").classList.remove("active");
    activeEntity = null;

    if (viewMode === "current") {
      // Pull current tab's entities
      chrome.runtime.sendMessage({ type: "get-tab-entities" }, (resp) => {
        if (chrome.runtime.lastError) return;
        loadEntitiesFromArray(resp && resp.entities ? resp.entities : []);
        render();
      });
    } else if (viewMode === "all") {
      // Pull all merged
      chrome.runtime.sendMessage({ type: "get-all-tab-entities" }, (resp) => {
        if (chrome.runtime.lastError || !resp) return;
        loadEntitiesFromArray(resp.entities || []);
        render();
      });
    } else {
      // Specific tab ID
      chrome.runtime.sendMessage({ type: "get-specific-tab", tabId: parseInt(viewMode) }, (resp) => {
        if (chrome.runtime.lastError || !resp) return;
        loadEntitiesFromArray(resp.entities || []);
        render();
      });
    }
  });

  // Refresh dropdown periodically and after scans
  setInterval(refreshTabDropdown, 5000);
  setTimeout(refreshTabDropdown, 1000);

  // --- Scan All Tabs ---
  document.getElementById("btn-scan-all").addEventListener("click", () => {
    showToast("Scanning all tabs...");
    chrome.tabs.query({ currentWindow: true }, (tabs) => {
      let scanned = 0;
      tabs.forEach(tab => {
        if (tab.url && !tab.url.startsWith("chrome://") && !tab.url.startsWith("chrome-extension://")) {
          chrome.tabs.sendMessage(tab.id, { type: "scan" }, () => {
            void chrome.runtime.lastError;
            scanned++;
            if (scanned === tabs.length) {
              setTimeout(() => {
                refreshTabDropdown();
                showToast("Scanned " + tabs.length + " tabs");
                // Switch to "All tabs" view
                $tabSelect.value = "all";
                $tabSelect.dispatchEvent(new Event("change"));
              }, 2000);
            }
          });
        } else {
          scanned++;
        }
      });
    });
  });

  // --- Clear button ---
  document.getElementById("btn-clear").addEventListener("click", () => {
    viewMode = "current";
    $tabSelect.value = "current";
    allEntities = { gene: [], variant: [], accession: [], file: [], species: [] };
    currentEntities = [];
    activeEntity = null;
    searchFilter = "";
    $search.value = "";
    document.getElementById("detail-panel").classList.remove("active");
    // Tell background + content script to clear
    chrome.runtime.sendMessage({ type: "clear-tab-entities" });
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs[0]) chrome.tabs.sendMessage(tabs[0].id, { type: "clear-highlights" }).catch(() => {});
    });
    render();
  });

  // --- Search ---
  $search.addEventListener("input", (e) => {
    searchFilter = e.target.value.trim();
    render();
  });

  // --- Settings toggle ---
  document.getElementById("btn-settings").addEventListener("click", () => {
    $settings.classList.add("visible");
  });
  document.getElementById("btn-close-settings").addEventListener("click", () => {
    $settings.classList.remove("visible");
  });

  // Toggle switches
  document.querySelectorAll(".toggle").forEach((el) => {
    const key = el.dataset.key;
    if (key === "theme") {
      // Theme toggle: off = dark (default), on = light
      chrome.storage.local.get("setting:theme", (data) => {
        if (data["setting:theme"] === true) {
          el.classList.add("on");
          document.body.classList.add("light-theme");
        }
      });
      el.addEventListener("click", () => {
        el.classList.toggle("on");
        const isLight = el.classList.contains("on");
        document.body.classList.toggle("light-theme", isLight);
        chrome.storage.local.set({ "setting:theme": isLight });
      });
      return;
    }
    // Load saved state
    chrome.storage.local.get(`setting:${key}`, (data) => {
      const val = data[`setting:${key}`];
      if (val === false) el.classList.remove("on");
    });
    el.addEventListener("click", () => {
      el.classList.toggle("on");
      const isOn = el.classList.contains("on");
      chrome.storage.local.set({ [`setting:${key}`]: isOn });
      // Wire highlight toggle to content script
      if (key === "highlight") {
        chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
          if (tabs[0]) {
            chrome.tabs.sendMessage(tabs[0].id, {
              type: isOn ? "highlight" : "clear-highlights"
            }).catch(() => {});
          }
        });
        showToast(isOn ? "Highlights enabled" : "Highlights disabled");
      }
    });
  });

  // --- Message listener (pull model — only handles lookup, scan-text, tab-switched) ---
  chrome.runtime.onMessage.addListener((msg) => {

    if (msg.type === "lookup") {
      // Direct lookup from context menu or BioGist link click
      const text = (msg.text || "").trim();
      if (!text) return;

      // Classify and add to entity list if not already present
      let type, id;
      if (/^rs\d+$/i.test(text)) {
        type = "variant"; id = text.toLowerCase();
      } else if (/^(?:GSE|GSM|GDS|SRR|SRX|SRP|ERR|ERX|ERP|DRR|DRX|DRP|PRJNA|PRJEB|PRJDB)\d+$/i.test(text)) {
        type = "accession"; id = text.toUpperCase();
      } else if (/\.(?:fastq|fq|bam|sam|vcf|bed|gff|gtf|fasta|fa|cram)(?:\.gz)?$/i.test(text)) {
        type = "file"; id = text;
      } else if (/^(Human|Mouse|Rat|Fruit fly|C\. elegans|Zebrafish|Yeast|Arabidopsis|E\. coli)$/i.test(text)) {
        type = "species"; id = text;
      } else {
        type = "gene"; id = text.toUpperCase();
      }

      // Ensure entity is in the list
      if (!allEntities[type]) allEntities[type] = [];
      const existing = allEntities[type].find(e => (typeof e === "string" ? e : e.id) === id);
      if (!existing) {
        allEntities[type].push({ type, id });
        currentEntities.push({ type, id });
      }
      render();
      selectEntity(type, id);
    }

    if (msg.type === "scan-text") {
      // Scan selected text from context menu
      const text = (msg.text || "").trim();
      if (text.length < 5) return;
      const entities = detectEntitiesFromText(text);
      if (entities.length > 0) {
        loadEntitiesFromArray(entities);
        showToast("Found " + entities.length + " entities in selection");
        document.getElementById("detail-panel").classList.remove("active");
        render();
      } else {
        showToast("No entities found in selection");
      }
    }

    if (msg.type === "tab-switched") {
      currentTabId = msg.tabId || null;
      refreshTabDropdown();
      // Only update view if in "current tab" mode
      if (viewMode === "current") {
        document.getElementById("detail-panel").classList.remove("active");
        chrome.runtime.sendMessage({ type: "get-tab-entities" }, (resp) => {
          if (chrome.runtime.lastError) return;
          const newEntities = (resp && resp.entities) ? resp.entities : [];
          loadEntitiesFromArray(newEntities);
          activeEntity = null;
          searchFilter = "";
          $search.value = "";
          if (newEntities.length === 0) {
            $empty.querySelector("strong").textContent = "Not scanned yet";
            $empty.querySelector("p").innerHTML = 'Click <b>Scan</b> to analyze this page.';
          }
          render();
        });
      }
    }
  });

  // --- Load entities for current tab from background ---
  function loadEntitiesFromArray(rawEntities) {
    allEntities = { gene: [], variant: [], accession: [], file: [], species: [] };
    if (Array.isArray(rawEntities)) {
      rawEntities.forEach(e => {
        let t = e.type || "accession";
        if (!allEntities[t]) allEntities[t] = [];
        allEntities[t].push(e);
      });
    }
    currentEntities = [];
    for (const [type, items] of Object.entries(allEntities)) {
      if (!Array.isArray(items)) continue;
      for (const entity of items) {
        currentEntities.push(typeof entity === "string" ? { type, id: entity } : entity);
      }
    }
  }

  // Ask background for current tab's entities on init
  chrome.runtime.sendMessage({ type: "get-tab-entities" }, (resp) => {
    if (chrome.runtime.lastError) return;
    if (resp && resp.tabId) currentTabId = resp.tabId;
    if (resp && resp.entities && resp.entities.length > 0) {
      loadEntitiesFromArray(resp.entities);
      render();
    }
  });

  // --- Initial render ---
  render();
})();
