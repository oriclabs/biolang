// BioGist — Sidebar Panel Script
// Renders detected entities, fetches details from NCBI/UniProt, manages cache.

(() => {
  "use strict";

  // --- State ---
  let allEntities = { gene: [], variant: [], accession: [], method: [], genome_build: [], sample_size: [], stat_method: [], platform: [], cell_line: [], tissue: [], drug: [], clinical_trial: [], funding: [], repository: [], p_value: [], finding: [], file: [], species: [] };
  let currentEntities = []; // flat list of {type, id, subtype} for Copy IDs
  let activeEntity = null;
  let searchFilter = "";
  let viewMode = "current"; // "current", "all", or a tab ID string
  let currentTabId = null;
  let pinnedEntities = new Set(); // "gene:BRCA1", "variant:rs123" etc.

  // All entity type keys (order used in settings UI and section rendering)
  const ALL_TYPES = ["gene","variant","accession","species","method","genome_build","sample_size","stat_method","platform","cell_line","tissue","drug","clinical_trial","funding","repository","p_value","finding","file"];
  let enabledTypes = new Set(ALL_TYPES); // all on by default

  // Load pinned entities from storage
  chrome.storage.local.get(["biogist_pinned", "biogist_enabled_types"], (data) => {
    if (data.biogist_pinned && Array.isArray(data.biogist_pinned)) {
      pinnedEntities = new Set(data.biogist_pinned);
    }
    if (data.biogist_enabled_types && Array.isArray(data.biogist_enabled_types)) {
      // Only apply if it looks intentional (not an empty leftover)
      if (data.biogist_enabled_types.length > 0) {
        enabledTypes = new Set(data.biogist_enabled_types);
      }
      // else keep all enabled (default)
    }
  });
  function savePinnedEntities() {
    chrome.storage.local.set({ biogist_pinned: Array.from(pinnedEntities) });
  }
  function saveEnabledTypes() {
    if (enabledTypes.size === ALL_TYPES.length) {
      // All enabled = default state, remove key so fresh installs work correctly
      chrome.storage.local.remove("biogist_enabled_types");
    } else {
      chrome.storage.local.set({ biogist_enabled_types: Array.from(enabledTypes) });
    }
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

  // --- Shared core (loaded from biogist-core.js) ---
  const core = window.BioGistCore;
  const TYPE_META = core.TYPE_META;
  const escapeHtml = core.escapeHtml;
  const truncate = core.truncate;
  function detectEntitiesFromText(text) { return core.scanText(text); }

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
    const result = {};
    for (const [type, items] of Object.entries(allEntities)) {
      if (!Array.isArray(items)) continue;
      if (!enabledTypes.has(type)) continue; // skip disabled types
      if (!searchFilter) {
        result[type] = items;
      } else {
        const q = searchFilter.toLowerCase();
        result[type] = items.filter((e) => {
          const id = typeof e === "string" ? e : (e.id || "");
          return id.toLowerCase().includes(q);
        });
      }
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
          <span class="entity-name">${escapeHtml(name)}${source ? '<span style="display:block;font-size:9px;color:#475569;font-weight:400;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + escapeHtml(source.substring(0, 50)) + '</span>' : ''}</span>
          ${count && count > 1 ? '<span style="font-size:10px;color:#64748b;margin-left:2px">\u00d7' + count + '</span>' : ''}
          <span class="entity-badge ${meta.cls}">${meta.badge}</span>
        `;
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

        // Inline "Open in BioPeek" button for file entities (toggle)
        if (type === "file") {
          const viewBtn = document.createElement("button");
          viewBtn.textContent = "\u{1F441}";
          viewBtn.title = "Download / Copy URL";
          viewBtn.style.cssText = "background:#7c3aed;color:#fff;border:none;padding:2px 6px;border-radius:4px;cursor:pointer;font-size:12px;flex-shrink:0;margin-left:4px;";
          if (activeFilePanel === name) viewBtn.style.background = "#06b6d4"; // highlight if active
          viewBtn.addEventListener("click", (e) => {
            e.stopPropagation();
            toggleFilePanel(name, viewBtn);
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

  // escapeHtml imported from BioGistCore

  // --- Open file in BioPeek ---
  let activeFilePanel = null; // track which URL's panel is open
  let activeFileBtn = null;   // track the highlighted eye button

  function closeFilePanel() {
    var existing = document.getElementById("file-action-panel");
    if (existing) existing.remove();
    if (activeFileBtn) {
      activeFileBtn.style.background = "#7c3aed";
      activeFileBtn = null;
    }
    activeFilePanel = null;
  }

  function toggleFilePanel(url, eyeBtn) {
    // If same URL panel is open, close it (toggle off)
    if (activeFilePanel === url) {
      closeFilePanel();
      return;
    }

    // Close any existing panel first
    closeFilePanel();

    activeFilePanel = url;
    activeFileBtn = eyeBtn;
    eyeBtn.style.background = "#06b6d4"; // highlight active

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
      '<div style="font-size:10px;color:#475569;margin-top:6px">Download and drop into BioPeek, or copy URL to paste in viewer.</div>';
    document.body.appendChild(panel);

    panel.querySelector(".file-panel-download").addEventListener("click", () => {
      showToast("Downloading...");
      fetch(url).then(r => r.blob()).then(blob => {
        var a = document.createElement("a");
        a.href = URL.createObjectURL(blob);
        a.download = name;
        a.click();
        setTimeout(() => URL.revokeObjectURL(a.href), 1000);
        showToast("Downloaded — drop it into BioPeek");
        closeFilePanel();
      }).catch(() => {
        navigator.clipboard.writeText(url);
        showToast("Download failed — URL copied instead");
        closeFilePanel();
      });
    });

    panel.querySelector(".file-panel-copy").addEventListener("click", () => {
      navigator.clipboard.writeText(url).then(() => {
        showToast("URL copied");
        closeFilePanel();
      });
    });

    panel.querySelector(".file-panel-close").addEventListener("click", () => closeFilePanel());
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

  async function fetchPubMedResults(type, name, container) {
    let query = name;
    if (type === "gene") query = name + "[Gene] AND human[Organism]";
    else if (type === "drug") query = name + "[MeSH Terms]";
    else if (type === "cell_line") query = '"' + name + '"[Title/Abstract]';
    else if (type === "method") query = '"' + name + '"[Title/Abstract] AND bioinformatics';
    else if (type === "variant" && name.startsWith("rs")) query = name;
    else if (type === "accession" && /^10\.\d{4}/.test(name)) query = name;

    const cacheKey = "pubmed:" + type + ":" + name;
    let articles = await cacheGet(cacheKey);

    if (!articles) {
      try {
        const searchResp = await fetchWithTimeout(
          "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&retmode=json&retmax=5&sort=relevance&term=" + encodeURIComponent(query)
        );
        const ids = searchResp && searchResp.esearchresult && searchResp.esearchresult.idlist;
        if (!ids || ids.length === 0) {
          container.innerHTML = '<div style="font-size:11px;color:#475569;text-align:center;padding:8px">No papers found on PubMed.</div>';
          return;
        }

        const summaryResp = await fetchWithTimeout(
          "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&retmode=json&id=" + ids.join(",")
        );
        const result = summaryResp && summaryResp.result;
        if (!result) {
          container.innerHTML = '<div style="font-size:11px;color:#f87171;text-align:center;padding:8px">Failed to fetch results.</div>';
          return;
        }

        articles = ids.map(id => {
          const a = result[id];
          if (!a) return null;
          const authors = a.authors && a.authors.length > 0 ? a.authors[0].name + (a.authors.length > 1 ? " et al." : "") : "";
          return {
            pmid: id,
            title: (a.title || "").replace(/<\/?[^>]+>/g, ""),
            authors: authors,
            journal: a.fulljournalname || a.source || "",
            year: (a.pubdate || "").split(" ")[0] || ""
          };
        }).filter(Boolean);

        if (articles.length > 0) await cacheSet(cacheKey, articles);
      } catch (e) {
        container.innerHTML = '<div style="font-size:11px;color:#f87171;text-align:center;padding:8px">PubMed request failed. Try again.</div>';
        return;
      }
    }

    if (!articles || articles.length === 0) {
      container.innerHTML = '<div style="font-size:11px;color:#475569;text-align:center;padding:8px">No papers found on PubMed.</div>';
      return;
    }

    // Render inline results
    container.innerHTML = '<div style="font-size:10px;color:#475569;text-transform:uppercase;font-weight:600;margin-bottom:6px">Related Papers (PubMed)</div>';

    articles.forEach(a => {
      const item = document.createElement("div");
      item.style.cssText = "padding:6px 0;border-bottom:1px solid rgba(30,41,59,0.5);";
      item.innerHTML =
        '<a href="https://pubmed.ncbi.nlm.nih.gov/' + a.pmid + '/" target="_blank" rel="noopener" style="font-size:11px;color:#818cf8;text-decoration:none;display:block;line-height:1.4">' + escapeHtml(a.title) + '</a>' +
        '<div style="font-size:10px;color:#64748b;margin-top:2px">' + escapeHtml(a.authors) + ' &mdash; ' + escapeHtml(a.journal) + ' (' + escapeHtml(a.year) + ')</div>';
      container.appendChild(item);
    });

    const moreLink = document.createElement("a");
    moreLink.href = "https://pubmed.ncbi.nlm.nih.gov/?term=" + encodeURIComponent(query);
    moreLink.target = "_blank";
    moreLink.rel = "noopener";
    moreLink.style.cssText = "display:block;text-align:center;font-size:10px;color:#06b6d4;margin-top:6px;text-decoration:none;";
    moreLink.textContent = "More on PubMed \u2192";
    container.appendChild(moreLink);
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

    // Inline PubMed search button (not auto — saves API calls)
    if (!["species","sample_size","stat_method","genome_build","platform","tissue","finding"].includes(type)) {
      const pubmedDiv = document.createElement("div");
      pubmedDiv.style.cssText = "margin-top:10px;padding-top:8px;border-top:1px solid #1e293b;";
      pubmedDiv.id = "pubmed-section";
      const pubmedBtn = document.createElement("button");
      pubmedBtn.className = "btn";
      pubmedBtn.style.cssText = "width:100%;justify-content:center;font-size:11px;gap:6px;";
      pubmedBtn.innerHTML = "&#x1F4DA; Find Related Papers";
      pubmedBtn.addEventListener("click", () => {
        pubmedBtn.innerHTML = '<div class="spinner" style="width:14px;height:14px;border-width:2px;display:inline-block;vertical-align:middle"></div> Searching PubMed...';
        pubmedBtn.disabled = true;
        fetchPubMedResults(type, name, pubmedDiv);
      });
      pubmedDiv.appendChild(pubmedBtn);
      $detailBody.appendChild(pubmedDiv);
    }

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

      // Cite button for DOI accessions
      if (type === "accession" && /^10\.\d{4}/.test(name)) {
        const citeBtn = document.createElement("button");
        citeBtn.textContent = "Cite";
        citeBtn.style.cssText = "flex:1;padding:5px;border-radius:6px;border:none;background:#7c3aed;color:#fff;font-size:11px;font-weight:600;cursor:pointer;";
        citeBtn.addEventListener("click", async () => {
          citeBtn.textContent = "Fetching...";
          citeBtn.disabled = true;
          try {
            const resp = await fetchWithTimeout("https://api.crossref.org/works/" + encodeURIComponent(name));
            const msg = resp.message;
            if (!msg) { showToast("DOI not found"); return; }

            const title = (msg.title && msg.title[0]) || "Untitled";
            const authors = (msg.author || []).map(a => {
              const g = a.given ? a.given.charAt(0) + "." : "";
              return a.family + (g ? ", " + g : "");
            });
            const year = ((msg["published-print"] || msg["published-online"] || msg.created || {})["date-parts"] || [[]])[0][0] || "n.d.";
            const journal = (msg["container-title"] && msg["container-title"][0]) || "";
            const vol = msg.volume || "";
            const pages = msg.page || "";
            const doi = msg.DOI || name;

            const authorStr = authors.length <= 3 ? authors.join(", ") : authors[0] + " et al.";
            const apa = authorStr + " (" + year + "). " + title + ". " + journal + (vol ? ", " + vol : "") + (pages ? ", " + pages : "") + ". https://doi.org/" + doi;
            const bibtex = "@article{" + doi.replace(/[/.]/g, "_") + ",\n  author = {" + authors.join(" and ") + "},\n  title = {" + title + "},\n  journal = {" + journal + "},\n  year = {" + year + "},\n  volume = {" + vol + "},\n  pages = {" + pages + "},\n  doi = {" + doi + "}\n}";
            const ris = "TY  - JOUR\n" + authors.map(a => "AU  - " + a).join("\n") + "\nTI  - " + title + "\nJO  - " + journal + "\nVL  - " + vol + "\nSP  - " + pages + "\nPY  - " + year + "\nDO  - " + doi + "\nER  -";

            // Show citation panel
            const citePanel = document.createElement("div");
            citePanel.style.cssText = "margin-top:10px;background:#020617;border:1px solid #1e293b;border-radius:6px;padding:10px;";
            citePanel.innerHTML = '<div style="font-size:10px;color:#64748b;text-transform:uppercase;font-weight:600;margin-bottom:6px">Citation</div>' +
              '<div style="display:flex;flex-wrap:wrap;gap:4px;margin-bottom:8px">' +
              '<button class="cite-tab" data-fmt="apa" style="padding:2px 8px;border-radius:4px;border:1px solid #334155;background:#1e293b;color:#e2e8f0;font-size:10px;cursor:pointer">APA</button>' +
              '<button class="cite-tab" data-fmt="vancouver" style="padding:2px 8px;border-radius:4px;border:1px solid #334155;background:none;color:#94a3b8;font-size:10px;cursor:pointer">Vancouver</button>' +
              '<button class="cite-tab" data-fmt="harvard" style="padding:2px 8px;border-radius:4px;border:1px solid #334155;background:none;color:#94a3b8;font-size:10px;cursor:pointer">Harvard</button>' +
              '<button class="cite-tab" data-fmt="bibtex" style="padding:2px 8px;border-radius:4px;border:1px solid #334155;background:none;color:#94a3b8;font-size:10px;cursor:pointer">BibTeX</button>' +
              '<button class="cite-tab" data-fmt="ris" style="padding:2px 8px;border-radius:4px;border:1px solid #334155;background:none;color:#94a3b8;font-size:10px;cursor:pointer">RIS</button>' +
              '</div>' +
              '<pre class="cite-text" style="font-size:11px;color:#a5f3fc;white-space:pre-wrap;word-break:break-all;margin:0;max-height:120px;overflow-y:auto">' + escapeHtml(apa) + '</pre>' +
              '<button class="cite-copy" style="margin-top:6px;width:100%;padding:4px;border-radius:4px;border:none;background:#334155;color:#e2e8f0;font-size:11px;font-weight:600;cursor:pointer">Copy Citation</button>';
            $detailBody.appendChild(citePanel);

            // Vancouver: numbered, Author(s). Title. Journal. Year;Vol:Pages. doi:
            const vancouver = authorStr + ". " + title + ". " + journal + ". " + year + (vol ? ";" + vol : "") + (pages ? ":" + pages : "") + ". doi:" + doi;

            // Harvard: Author(s) (Year) 'Title', Journal, Vol, pp. Pages. doi:
            const harvard = authorStr + " (" + year + ") '" + title + "', " + journal + (vol ? ", " + vol : "") + (pages ? ", pp. " + pages : "") + ". https://doi.org/" + doi;

            const formats = { apa, vancouver, harvard, bibtex, ris };
            let currentFmt = "apa";
            citePanel.querySelectorAll(".cite-tab").forEach(tab => {
              tab.addEventListener("click", () => {
                currentFmt = tab.dataset.fmt;
                citePanel.querySelector(".cite-text").textContent = formats[currentFmt];
                citePanel.querySelectorAll(".cite-tab").forEach(t => { t.style.background = "none"; t.style.color = "#94a3b8"; });
                tab.style.background = "#1e293b";
                tab.style.color = "#e2e8f0";
              });
            });
            citePanel.querySelector(".cite-copy").addEventListener("click", () => {
              navigator.clipboard.writeText(formats[currentFmt]);
              showToast("Citation copied (" + currentFmt.toUpperCase() + ")");
            });
          } catch (e) {
            showToast("Failed to fetch citation: " + e.message);
          }
          citeBtn.textContent = "Cite";
          citeBtn.disabled = false;
        });
        copyDiv.appendChild(citeBtn);
      }

      $detailBody.appendChild(copyDiv);
    }
  }

  // --- Source links builder ---
  function buildSourceLinks(type, name, info) {
    const links = [];
    const q = encodeURIComponent(name);
    const geneId = info && info["Gene ID"];
    const taxid = info && info["Taxonomy ID"];

    if (type === "gene") {
      if (geneId) links.push({ label: "NCBI Gene", url: `https://www.ncbi.nlm.nih.gov/gene/${geneId}` });
      links.push({ label: "Ensembl", url: `https://ensembl.org/Homo_sapiens/Gene/Summary?g=${name}` });
      if (info && info["UniProt"]) links.push({ label: "UniProt", url: `https://www.uniprot.org/uniprot/${info["UniProt"]}` });
      links.push({ label: "GeneCards", url: `https://www.genecards.org/cgi-bin/carddisp.pl?gene=${name}` });
      links.push({ label: "OMIM", url: `https://omim.org/search?search=${q}` });
      links.push({ label: "ClinicalTrials", url: `https://clinicaltrials.gov/search?term=${q}` });
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
      links.push({ label: "ClinicalTrials", url: `https://clinicaltrials.gov/search?term=${q}` });
    } else if (type === "accession") {
      if (/^GSE\d+$/i.test(name)) links.push({ label: "GEO", url: `https://www.ncbi.nlm.nih.gov/geo/query/acc.cgi?acc=${name}` });
      if (/^SRR\d+$/i.test(name)) links.push({ label: "SRA", url: `https://www.ncbi.nlm.nih.gov/sra/${name}` });
      if (/^SRP\d+$/i.test(name)) links.push({ label: "SRA Project", url: `https://www.ncbi.nlm.nih.gov/sra/?term=${name}` });
      if (/^PRJNA\d+$/i.test(name)) links.push({ label: "BioProject", url: `https://www.ncbi.nlm.nih.gov/bioproject/${name}` });
      if (/^PRJEB\d+$/i.test(name)) links.push({ label: "ENA", url: `https://www.ebi.ac.uk/ena/browser/view/${name}` });
      if (/^ENSG\d+$/i.test(name)) links.push({ label: "Ensembl", url: `https://ensembl.org/id/${name}` });
      if (/^ENST\d+$/i.test(name)) links.push({ label: "Ensembl", url: `https://ensembl.org/id/${name}` });
      if (/^NM_/i.test(name)) links.push({ label: "RefSeq", url: `https://www.ncbi.nlm.nih.gov/nuccore/${name}` });
      if (/^10\.\d{4}/.test(name)) links.push({ label: "DOI", url: `https://doi.org/${name}` });
    } else if (type === "species") {
      if (taxid) links.push({ label: "NCBI Taxonomy", url: `https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?id=${taxid}` });
      else links.push({ label: "NCBI Taxonomy", url: `https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?name=${q}` });
    } else if (type === "drug") {
      links.push({ label: "DrugBank", url: `https://go.drugbank.com/unearth/q?query=${q}` });
      links.push({ label: "ClinicalTrials", url: `https://clinicaltrials.gov/search?intr=${q}` });
      links.push({ label: "RxList", url: `https://www.rxlist.com/search/rxlist/${q}` });
    } else if (type === "cell_line") {
      links.push({ label: "Cellosaurus", url: `https://www.cellosaurus.org/search?input=${q}` });
    } else if (type === "method") {
      links.push({ label: "bio.tools", url: `https://bio.tools/?q=${q}` });
    } else if (type === "clinical_trial") {
      links.push({ label: "ClinicalTrials.gov", url: `https://clinicaltrials.gov/study/${name}` });
    } else if (type === "funding") {
      if (/^[RPUKFTMS]\d{2}/.test(name)) links.push({ label: "NIH Reporter", url: `https://reporter.nih.gov/search/${q}` });
    } else if (type === "repository") {
      links.push({ label: "Open", url: name.startsWith("http") ? name : `https://${name}` });
    }

    // Universal links for all types
    let pmQuery = q;
    if (type === "gene") pmQuery = encodeURIComponent(name + "[Gene] AND human[Organism]");
    else if (type === "drug") pmQuery = encodeURIComponent(name + "[MeSH Terms]");
    links.push({ label: "PubMed", url: `https://pubmed.ncbi.nlm.nih.gov/?term=${pmQuery}` });
    links.push({ label: "Scholar", url: `https://scholar.google.com/scholar?q=${q}` });

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
      case "gene":         info = await fetchGeneInfo(name); break;
      case "variant":      info = await fetchVariantInfo(name); break;
      case "accession":    info = await fetchAccessionInfo(name); break;
      case "file":         info = describeFile(name); break;
      case "species":      info = await fetchSpeciesInfo(name); break;
      case "method":       info = describeMethod(name); break;
      case "genome_build": info = describeGenomeBuild(name); break;
      case "sample_size":  info = { "Sample Size": name }; break;
      case "stat_method":  info = describeStatMethod(name); break;
      case "platform":     info = describePlatform(name); break;
      case "cell_line":    info = describeCellLine(name); break;
      case "tissue":       info = describeTissue(name); break;
      case "drug":         info = describeDrug(name); break;
      case "finding":      info = { "Finding": name }; break;
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

    // For DOIs, fetch citation count and related papers from OpenAlex
    if (/^10\.\d{4}/.test(accession)) {
      try {
        const oaUrl = "https://api.openalex.org/works/doi:" + encodeURIComponent(accession);
        const oaResp = await fetchWithTimeout(oaUrl);
        if (oaResp) {
          if (oaResp.title) info["Title"] = oaResp.title;
          if (oaResp.publication_year) info["Year"] = String(oaResp.publication_year);
          if (oaResp.cited_by_count !== undefined) info["Citations"] = String(oaResp.cited_by_count);
          if (oaResp.primary_location && oaResp.primary_location.source) {
            info["Journal"] = oaResp.primary_location.source.display_name || "";
          }
          if (oaResp.authorships && oaResp.authorships.length > 0) {
            info["Authors"] = oaResp.authorships.slice(0, 5).map(function(a) { return a.author.display_name; }).join(", ");
            if (oaResp.authorships.length > 5) info["Authors"] += " et al.";
          }
          if (oaResp.related_works && oaResp.related_works.length > 0) {
            info["Related Papers"] = oaResp.related_works.length + " found";
          }
          if (oaResp.open_access && oaResp.open_access.oa_url) {
            info["Open Access"] = oaResp.open_access.oa_url;
          }
        }
      } catch (_) { /* OpenAlex fetch failed */ }
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

  // --- Method/Tool details ---
  const TOOL_DB = {
    "BWA":           { cat: "Aligner", desc: "Burrows-Wheeler Aligner for short reads", url: "https://github.com/lh3/bwa" },
    "BWA-MEM":       { cat: "Aligner", desc: "BWA-MEM algorithm for long reads", url: "https://github.com/lh3/bwa" },
    "BWA-MEM2":      { cat: "Aligner", desc: "Faster BWA-MEM with AVX2/SSE", url: "https://github.com/bwa-mem2/bwa-mem2" },
    "Bowtie2":       { cat: "Aligner", desc: "Fast short-read aligner", url: "https://bowtie-bio.sourceforge.net/bowtie2" },
    "STAR":          { cat: "Aligner", desc: "Spliced transcript aligner for RNA-seq", url: "https://github.com/alexdobin/STAR" },
    "HISAT2":        { cat: "Aligner", desc: "Graph-based aligner for RNA-seq", url: "http://daehwankimlab.github.io/hisat2" },
    "minimap2":      { cat: "Aligner", desc: "Long-read and assembly aligner", url: "https://github.com/lh3/minimap2" },
    "Salmon":        { cat: "Quantification", desc: "Fast transcript quantification", url: "https://salmon.readthedocs.io" },
    "Kallisto":      { cat: "Quantification", desc: "Near-optimal RNA-seq quantification", url: "https://pachterlab.github.io/kallisto" },
    "GATK":          { cat: "Variant Calling", desc: "Genome Analysis Toolkit", url: "https://gatk.broadinstitute.org" },
    "HaplotypeCaller": { cat: "Variant Calling", desc: "GATK germline variant caller", url: "https://gatk.broadinstitute.org" },
    "Mutect2":       { cat: "Variant Calling", desc: "GATK somatic variant caller", url: "https://gatk.broadinstitute.org" },
    "DeepVariant":   { cat: "Variant Calling", desc: "Google's deep learning variant caller", url: "https://github.com/google/deepvariant" },
    "bcftools":      { cat: "Variant Tools", desc: "VCF/BCF manipulation toolkit", url: "https://samtools.github.io/bcftools" },
    "samtools":      { cat: "Alignment Tools", desc: "SAM/BAM/CRAM manipulation toolkit", url: "https://www.htslib.org" },
    "DESeq2":        { cat: "Differential Expression", desc: "RNA-seq differential expression (R/Bioconductor)", url: "https://bioconductor.org/packages/DESeq2" },
    "edgeR":         { cat: "Differential Expression", desc: "Empirical analysis of digital gene expression", url: "https://bioconductor.org/packages/edgeR" },
    "limma":         { cat: "Differential Expression", desc: "Linear models for microarray/RNA-seq", url: "https://bioconductor.org/packages/limma" },
    "Seurat":        { cat: "Single Cell", desc: "Single-cell RNA-seq analysis (R)", url: "https://satijalab.org/seurat" },
    "Scanpy":        { cat: "Single Cell", desc: "Single-cell RNA-seq analysis (Python)", url: "https://scanpy.readthedocs.io" },
    "CellRanger":    { cat: "Single Cell", desc: "10x Genomics processing pipeline", url: "https://www.10xgenomics.com/support/software/cell-ranger" },
    "FastQC":        { cat: "Quality Control", desc: "Sequencing quality assessment", url: "https://www.bioinformatics.babraham.ac.uk/projects/fastqc" },
    "MultiQC":       { cat: "Quality Control", desc: "Aggregate QC reports", url: "https://multiqc.info" },
    "Trimmomatic":   { cat: "Preprocessing", desc: "Adapter trimming for Illumina", url: "http://www.usadellab.org/cms/?page=trimmomatic" },
    "fastp":         { cat: "Preprocessing", desc: "Fast all-in-one FASTQ preprocessor", url: "https://github.com/OpenGene/fastp" },
    "BLAST":         { cat: "Sequence Search", desc: "Basic Local Alignment Search Tool", url: "https://blast.ncbi.nlm.nih.gov" },
    "MACS2":         { cat: "Peak Calling", desc: "ChIP-seq peak caller", url: "https://github.com/macs3-project/MACS" },
    "VEP":           { cat: "Annotation", desc: "Ensembl Variant Effect Predictor", url: "https://ensembl.org/info/docs/tools/vep" },
    "ANNOVAR":       { cat: "Annotation", desc: "Functional annotation of genetic variants", url: "https://annovar.openbioinformatics.org" },
    "SnpEff":        { cat: "Annotation", desc: "Variant annotation and effect prediction", url: "https://pcingola.github.io/SnpEff" },
    "Nextflow":      { cat: "Workflow", desc: "Data-driven computational pipelines", url: "https://www.nextflow.io" },
    "Snakemake":     { cat: "Workflow", desc: "Python-based workflow management", url: "https://snakemake.readthedocs.io" },
    "Docker":        { cat: "Containers", desc: "Application containerization", url: "https://www.docker.com" },
    "Singularity":   { cat: "Containers", desc: "HPC container platform", url: "https://sylabs.io/singularity" },
    "PLINK":         { cat: "GWAS", desc: "Whole-genome association analysis", url: "https://www.cog-genomics.org/plink" },
    "R":             { cat: "Language", desc: "Statistical computing language", url: "https://www.r-project.org" },
    "Python":        { cat: "Language", desc: "General-purpose programming language", url: "https://www.python.org" },
    "Kraken2":       { cat: "Metagenomics", desc: "Taxonomic classification of sequences", url: "https://ccb.jhu.edu/software/kraken2" },
    "QIIME2":        { cat: "Metagenomics", desc: "Microbiome bioinformatics platform", url: "https://qiime2.org" },
    "AlphaFold":     { cat: "Structure Prediction", desc: "DeepMind protein structure prediction", url: "https://alphafold.ebi.ac.uk" },
  };

  function describeMethod(name) {
    const t = TOOL_DB[name] || {};
    return {
      Tool: name,
      Category: t.cat || "Bioinformatics Tool",
      Description: t.desc || "Detected in the paper",
      Homepage: t.url || ""
    };
  }

  // --- Genome Build details ---
  function describeGenomeBuild(name) {
    const builds = {
      "GRCh38 (hg38)": { org: "Human", released: "2013", notes: "Current standard for human genomics" },
      "GRCh37 (hg19)": { org: "Human", released: "2009", notes: "Legacy build, still widely used. Consider liftover to GRCh38" },
      "T2T-CHM13":     { org: "Human", released: "2022", notes: "First complete human genome (telomere-to-telomere)" },
      "GRCm39 (mm39)": { org: "Mouse", released: "2020", notes: "Current mouse reference" },
      "GRCm38 (mm10)": { org: "Mouse", released: "2012", notes: "Legacy mouse reference" },
      "hg18 (NCBI36)": { org: "Human", released: "2006", notes: "Outdated. Liftover strongly recommended" },
    };
    const b = builds[name] || {};
    return {
      Build: name,
      Organism: b.org || "Unknown",
      Released: b.released || "",
      Notes: b.notes || ""
    };
  }

  // --- Sample Size details ---
  function describeSampleSize(name, entityObj) {
    return {
      "Sample Size": name,
      Context: (entityObj && entityObj.snippet) ? entityObj.snippet : "Found in the paper"
    };
  }

  // --- Statistical Method details ---
  const STAT_DB = {
    "t-test":               { cat: "Hypothesis Testing", use: "Compare means of two groups" },
    "Mann-Whitney":         { cat: "Non-parametric", use: "Compare two groups without normality assumption" },
    "Wilcoxon":             { cat: "Non-parametric", use: "Paired/signed-rank comparison" },
    "chi-square":           { cat: "Hypothesis Testing", use: "Test independence of categorical variables" },
    "Fisher's exact":       { cat: "Hypothesis Testing", use: "Exact test for small sample categorical data" },
    "ANOVA":                { cat: "Hypothesis Testing", use: "Compare means of 3+ groups" },
    "Kruskal-Wallis":       { cat: "Non-parametric", use: "Non-parametric alternative to one-way ANOVA" },
    "log-rank":             { cat: "Survival Analysis", use: "Compare survival curves between groups" },
    "Cox regression":       { cat: "Survival Analysis", use: "Model hazard ratios with covariates" },
    "Kaplan-Meier":         { cat: "Survival Analysis", use: "Estimate survival function" },
    "logistic regression":  { cat: "Regression", use: "Model binary outcome" },
    "linear regression":    { cat: "Regression", use: "Model continuous outcome" },
    "Bonferroni":           { cat: "Multiple Testing", use: "Conservative p-value correction" },
    "Benjamini-Hochberg":   { cat: "Multiple Testing", use: "FDR correction (less conservative)" },
    "FDR":                  { cat: "Multiple Testing", use: "False discovery rate control" },
    "PCA":                  { cat: "Dimensionality Reduction", use: "Reduce dimensions, visualize clusters" },
    "t-SNE":                { cat: "Dimensionality Reduction", use: "Non-linear embedding for visualization" },
    "UMAP":                 { cat: "Dimensionality Reduction", use: "Fast non-linear embedding" },
    "k-means":              { cat: "Clustering", use: "Partition data into k groups" },
    "random forest":        { cat: "Machine Learning", use: "Ensemble classification/regression" },
    "bootstrap":            { cat: "Resampling", use: "Estimate confidence intervals by resampling" },
    "Pearson":              { cat: "Correlation", use: "Linear correlation coefficient" },
    "Spearman":             { cat: "Correlation", use: "Rank-based correlation coefficient" },
    "fold change":          { cat: "Effect Size", use: "Ratio of expression between conditions" },
    "odds ratio":           { cat: "Effect Size", use: "Association strength in case-control studies" },
    "hazard ratio":         { cat: "Effect Size", use: "Relative risk over time in survival analysis" },
  };

  function describeStatMethod(name) {
    const s = STAT_DB[name.toLowerCase()] || STAT_DB[name] || {};
    return {
      Method: name,
      Category: s.cat || "Statistical Method",
      "Used for": s.use || "Statistical analysis"
    };
  }

  // --- Sequencing Platform details ---
  function describePlatform(name) {
    const platforms = {
      "Illumina":       { tech: "Short-read sequencing (SBS)", reads: "50-300 bp", throughput: "Up to 6 Tb/run (NovaSeq)" },
      "PacBio":         { tech: "Long-read sequencing (SMRT)", reads: "10-25 kb (HiFi)", throughput: "Up to 90 Gb/cell" },
      "Oxford Nanopore":{ tech: "Long-read sequencing (nanopore)", reads: "Unlimited (up to 4 Mb)", throughput: "Up to 290 Gb/flow cell" },
      "10x Genomics":   { tech: "Single-cell/spatial transcriptomics", reads: "Barcode + short reads", throughput: "10,000+ cells/sample" },
      "Ion Torrent":    { tech: "Semiconductor sequencing", reads: "200-600 bp", throughput: "Up to 50 Gb/run" },
      "Sanger":         { tech: "Chain termination sequencing", reads: "700-1000 bp", throughput: "1 read/reaction" },
    };
    const p = platforms[name] || {};
    return {
      Platform: name,
      Technology: p.tech || "Sequencing platform",
      "Read Length": p.reads || "",
      Throughput: p.throughput || ""
    };
  }

  // --- Cell Line details ---
  const CELL_DB = {
    "HeLa":       { origin: "Cervical cancer", species: "Human", notes: "Most widely used human cell line" },
    "HEK293":     { origin: "Embryonic kidney", species: "Human", notes: "Common for protein expression/transfection" },
    "HEK293T":    { origin: "Embryonic kidney (SV40 T)", species: "Human", notes: "HEK293 with SV40 large T antigen" },
    "MCF7":       { origin: "Breast cancer (ER+)", species: "Human", notes: "Estrogen receptor positive breast cancer" },
    "MCF-7":      { origin: "Breast cancer (ER+)", species: "Human", notes: "Estrogen receptor positive breast cancer" },
    "A549":       { origin: "Lung adenocarcinoma", species: "Human", notes: "Non-small cell lung cancer" },
    "HCT116":     { origin: "Colorectal carcinoma", species: "Human", notes: "Commonly used colon cancer line" },
    "K562":       { origin: "Chronic myelogenous leukemia", species: "Human", notes: "BCR-ABL positive CML" },
    "Jurkat":     { origin: "T-cell leukemia", species: "Human", notes: "T-cell signaling studies" },
    "U2OS":       { origin: "Osteosarcoma", species: "Human", notes: "DNA damage response studies" },
    "HepG2":      { origin: "Hepatocellular carcinoma", species: "Human", notes: "Liver metabolism studies" },
    "MDA-MB-231": { origin: "Triple-negative breast cancer", species: "Human", notes: "Highly metastatic, TNBC model" },
    "PC-3":       { origin: "Prostate adenocarcinoma", species: "Human", notes: "Androgen-independent prostate cancer" },
    "LNCaP":      { origin: "Prostate carcinoma", species: "Human", notes: "Androgen-sensitive prostate cancer" },
    "iPSC":       { origin: "Induced pluripotent stem cell", species: "Human/various", notes: "Reprogrammed somatic cells" },
    "NIH3T3":     { origin: "Embryonic fibroblast", species: "Mouse", notes: "Standard mouse fibroblast line" },
    "CHO":        { origin: "Ovary", species: "Chinese hamster", notes: "Biopharmaceutical protein production" },
    "Vero":       { origin: "Kidney", species: "African green monkey", notes: "Virology studies, vaccine production" },
  };

  function describeCellLine(name) {
    const c = CELL_DB[name] || {};
    return {
      "Cell Line": name,
      Origin: c.origin || "Cell line",
      Species: c.species || "",
      Notes: c.notes || "Detected in the paper"
    };
  }

  // --- Tissue details ---
  function describeTissue(name) {
    const organs = {
      "blood": "Circulatory", "serum": "Circulatory", "plasma": "Circulatory", "PBMC": "Immune/Blood",
      "brain": "Nervous System", "hippocampus": "Nervous System", "cerebellum": "Nervous System",
      "liver": "Digestive", "kidney": "Urinary", "lung": "Respiratory", "heart": "Cardiovascular",
      "pancreas": "Digestive/Endocrine", "spleen": "Immune", "thymus": "Immune",
      "breast": "Reproductive", "ovary": "Reproductive", "prostate": "Reproductive",
      "colon": "Digestive", "stomach": "Digestive", "skin": "Integumentary",
      "muscle": "Musculoskeletal", "bone": "Musculoskeletal", "adipose": "Connective",
      "tumor": "Neoplastic", "tumour": "Neoplastic", "biopsy": "Clinical Sample",
      "organoid": "In Vitro Model", "xenograft": "In Vivo Model", "PDX": "Patient-Derived Xenograft",
    };
    return {
      Tissue: name,
      "Organ System": organs[name.toLowerCase()] || "Biological Tissue",
    };
  }

  // --- Drug details ---
  const DRUG_DB = {
    "olaparib":      { cls: "PARP inhibitor", target: "PARP1/2", status: "Approved", use: "BRCA-mutant breast/ovarian cancer" },
    "pembrolizumab": { cls: "Anti-PD-1", target: "PD-1", status: "Approved", use: "Multiple cancers (immunotherapy)" },
    "nivolumab":     { cls: "Anti-PD-1", target: "PD-1", status: "Approved", use: "Melanoma, NSCLC, RCC" },
    "trastuzumab":   { cls: "Anti-HER2", target: "HER2/ERBB2", status: "Approved", use: "HER2+ breast/gastric cancer" },
    "imatinib":      { cls: "Tyrosine kinase inhibitor", target: "BCR-ABL, KIT", status: "Approved", use: "CML, GIST" },
    "osimertinib":   { cls: "EGFR inhibitor (3rd gen)", target: "EGFR T790M", status: "Approved", use: "EGFR-mutant NSCLC" },
    "vemurafenib":   { cls: "BRAF inhibitor", target: "BRAF V600E", status: "Approved", use: "BRAF-mutant melanoma" },
    "venetoclax":    { cls: "BCL-2 inhibitor", target: "BCL2", status: "Approved", use: "CLL, AML" },
    "palbociclib":   { cls: "CDK4/6 inhibitor", target: "CDK4, CDK6", status: "Approved", use: "HR+/HER2- breast cancer" },
    "ibrutinib":     { cls: "BTK inhibitor", target: "BTK", status: "Approved", use: "CLL, MCL, WM" },
    "cisplatin":     { cls: "Platinum agent", target: "DNA crosslinks", status: "Approved", use: "Multiple solid tumors" },
    "paclitaxel":    { cls: "Taxane", target: "Microtubules", status: "Approved", use: "Breast, ovarian, NSCLC" },
    "tamoxifen":     { cls: "SERM", target: "Estrogen receptor", status: "Approved", use: "ER+ breast cancer" },
    "metformin":     { cls: "Biguanide", target: "AMPK pathway", status: "Approved", use: "Type 2 diabetes (cancer research)" },
    "dexamethasone": { cls: "Corticosteroid", target: "Glucocorticoid receptor", status: "Approved", use: "Inflammation, myeloma" },
    "temozolomide":  { cls: "Alkylating agent", target: "DNA methylation", status: "Approved", use: "Glioblastoma" },
  };

  function describeDrug(name) {
    const d = DRUG_DB[name.toLowerCase()] || {};
    return {
      Drug: name,
      Class: d.cls || "Pharmaceutical",
      Target: d.target || "",
      "FDA Status": d.status || "",
      "Used for": d.use || "Detected in the paper"
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

  // truncate imported from BioGistCore

  // --- Toast notification ---
  function showToast(message) {
    let toast = document.createElement("div");
    toast.textContent = message;
    toast.style.cssText = "position:fixed;bottom:16px;left:50%;transform:translateX(-50%);background:#7c3aed;color:#fff;padding:6px 14px;border-radius:6px;font-size:12px;font-weight:600;z-index:100;opacity:1;transition:opacity 0.3s;";
    document.body.appendChild(toast);
    setTimeout(() => { toast.style.opacity = "0"; setTimeout(() => toast.remove(), 300); }, 1800);
  }

  // --- Export dialog ---
  // Export button — now triggered from More menu
  function openExportDialog() {
    // Remove existing dialog
    const existing = document.getElementById("export-dialog");
    if (existing) { existing.remove(); return; }

    const allTypes = Object.keys(TYPE_META);

    const dialog = document.createElement("div");
    dialog.id = "export-dialog";
    dialog.style.cssText = "position:fixed;top:0;left:0;right:0;bottom:0;background:#020617;z-index:50;display:flex;flex-direction:column;overflow-y:auto;";

    // Header
    dialog.innerHTML = '<div style="display:flex;justify-content:space-between;align-items:center;padding:10px 14px;background:#0f172a;border-bottom:1px solid #1e293b">' +
      '<button id="export-close" style="background:none;border:none;color:#06b6d4;font-size:14px;cursor:pointer;padding:4px 8px;border-radius:4px">&#8592; Back</button>' +
      '<span style="font-weight:700;font-size:14px;color:#f1f5f9;flex:1">Export Results</span></div>' +
      '<div style="padding:14px;flex:1">' +

      // Scope
      '<div style="margin-bottom:12px">' +
        '<div style="font-size:10px;color:#64748b;text-transform:uppercase;font-weight:600;margin-bottom:6px">Scope</div>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-scope" value="current" checked style="margin-right:6px">Current view</label>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-scope" value="all" style="margin-right:6px">All tabs</label>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-scope" value="pinned" style="margin-right:6px">Pinned only</label>' +
      '</div>' +

      // Include types
      '<div style="margin-bottom:12px">' +
        '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:6px">' +
          '<span style="font-size:10px;color:#64748b;text-transform:uppercase;font-weight:600">Include</span>' +
          '<button id="exp-select-all" style="font-size:10px;color:#06b6d4;background:none;border:none;cursor:pointer">Select all</button>' +
        '</div>' +
        '<div id="exp-types" style="display:grid;grid-template-columns:1fr 1fr;gap:2px">' +
          allTypes.map(t => {
            const meta = TYPE_META[t];
            return '<label style="display:flex;align-items:center;font-size:11px;color:#cbd5e1;cursor:pointer;padding:2px 0"><input type="checkbox" class="exp-type" value="' + t + '" checked style="margin-right:5px">' + (meta ? meta.icon + ' ' + meta.label : t) + '</label>';
          }).join('') +
        '</div>' +
      '</div>' +

      // Format
      '<div style="margin-bottom:12px">' +
        '<div style="font-size:10px;color:#64748b;text-transform:uppercase;font-weight:600;margin-bottom:6px">Format</div>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-format" value="ids" style="margin-right:6px">Plain text (IDs only)</label>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-format" value="json" checked style="margin-right:6px">JSON (with details)</label>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-format" value="markdown" style="margin-right:6px">Markdown</label>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-format" value="csv" style="margin-right:6px">CSV</label>' +
        '<label style="display:block;font-size:12px;color:#e2e8f0;margin:3px 0;cursor:pointer"><input type="radio" name="exp-format" value="bibtex" style="margin-right:6px">BibTeX (DOIs only)</label>' +
      '</div>' +

      // Buttons
      '<div style="display:flex;gap:8px">' +
        '<button id="exp-copy" style="flex:1;padding:8px;border-radius:6px;border:none;background:#06b6d4;color:#fff;font-size:12px;font-weight:600;cursor:pointer">Copy to Clipboard</button>' +
        '<button id="exp-download" style="flex:1;padding:8px;border-radius:6px;border:none;background:#334155;color:#e2e8f0;font-size:12px;font-weight:600;cursor:pointer">Download</button>' +
      '</div>' +
    '</div>';

    document.body.appendChild(dialog);

    // Close
    document.getElementById("export-close").addEventListener("click", () => dialog.remove());

    // Select all toggle
    document.getElementById("exp-select-all").addEventListener("click", () => {
      const boxes = dialog.querySelectorAll(".exp-type");
      const allChecked = Array.from(boxes).every(b => b.checked);
      boxes.forEach(b => { b.checked = !allChecked; });
    });

    // Build export data based on selections
    async function buildExport() {
      const scope = dialog.querySelector('input[name="exp-scope"]:checked').value;
      const format = dialog.querySelector('input[name="exp-format"]:checked').value;
      const selectedTypes = new Set(Array.from(dialog.querySelectorAll('.exp-type:checked')).map(cb => cb.value));

      // Get entities based on scope
      let entities = [];
      let sources = [];
      if (scope === "current") {
        entities = currentEntities.filter(e => selectedTypes.has(e.type));
      } else if (scope === "all") {
        const resp = await new Promise(r => chrome.runtime.sendMessage({ type: "get-all-tab-entities" }, r));
        if (resp) {
          entities = (resp.entities || []).filter(e => selectedTypes.has(e.type));
          sources = resp.sources || [];
        }
      } else if (scope === "pinned") {
        pinnedEntities.forEach(key => {
          const [t, id] = key.split(":");
          if (selectedTypes.has(t)) entities.push({ type: t, id });
        });
      }

      if (entities.length === 0) { showToast("No entities match your selection"); return null; }

      // Group
      const grouped = {};
      entities.forEach(e => {
        const t = e.type || "other";
        if (!grouped[t]) grouped[t] = [];
        grouped[t].push(e);
      });

      // Get cached details
      for (const [type, items] of Object.entries(grouped)) {
        for (const item of items) {
          const cached = await cacheGet(`biogist:v${CACHE_VERSION}:${type}:${item.id}`);
          if (cached) item.details = cached;
        }
      }

      // Format output
      let output = "";
      const timestamp = new Date().toISOString();

      if (format === "ids") {
        output = entities.map(e => e.id).join("\n");
      } else if (format === "json") {
        const obj = { ...grouped, _exported: timestamp, _scope: scope };
        if (sources.length > 0) obj._sources = sources.map(s => ({ title: s.title, count: s.count }));
        output = JSON.stringify(obj, null, 2);
      } else if (format === "markdown") {
        output = "## BioGist Export\n\n*" + timestamp + "*\n\n";
        if (sources.length > 0) {
          output += "**Sources:** " + sources.map(s => s.title + " (" + s.count + ")").join(", ") + "\n\n";
        }
        for (const [type, items] of Object.entries(grouped)) {
          const meta = TYPE_META[type];
          output += "### " + (meta ? meta.label : type) + "\n\n";
          items.forEach(e => {
            output += "- **" + e.id + "**";
            if (e.source) output += " *(from: " + e.source + ")*";
            if (e.details) {
              const dets = Object.entries(e.details).filter(([k, v]) => v && !String(v).startsWith("http")).map(([k, v]) => k + ": " + v).join("; ");
              if (dets) output += " — " + dets;
            }
            output += "\n";
          });
          output += "\n";
        }
      } else if (format === "csv") {
        output = "type,id,source,details\n";
        entities.forEach(e => {
          const dets = e.details ? Object.entries(e.details).map(([k, v]) => k + "=" + v).join("; ") : "";
          output += '"' + e.type + '","' + e.id + '","' + (e.source || "") + '","' + dets.replace(/"/g, '""') + '"\n';
        });
      } else if (format === "bibtex") {
        const dois = entities.filter(e => /^10\.\d{4}/.test(e.id));
        if (dois.length === 0) { showToast("No DOIs found"); return null; }
        output = dois.map(e => {
          if (e.details && e.details.bibtex) return e.details.bibtex;
          return "@misc{" + e.id.replace(/[/.]/g, "_") + ",\n  doi = {" + e.id + "}\n}";
        }).join("\n\n");
      }

      return { output, format, count: entities.length };
    }

    // Copy button
    document.getElementById("exp-copy").addEventListener("click", async () => {
      const result = await buildExport();
      if (!result) return;
      navigator.clipboard.writeText(result.output);
      showToast("Copied " + result.count + " entities (" + result.format + ")");
      dialog.remove();
    });

    // Download button
    document.getElementById("exp-download").addEventListener("click", async () => {
      const result = await buildExport();
      if (!result) return;
      const ext = { ids: "txt", json: "json", markdown: "md", csv: "csv", bibtex: "bib" }[result.format] || "txt";
      const blob = new Blob([result.output], { type: "text/plain" });
      const a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = "biogist-export." + ext;
      a.click();
      showToast("Downloaded biogist-export." + ext);
      dialog.remove();
    });
  }
  // Export wired via More menu below

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
      // Store as virtual "pasted" tab in background
      chrome.runtime.sendMessage({ type: "store-pasted-entities", entities });
      loadEntitiesFromArray(entities);
      showToast("Found " + entities.length + " entities");
      panel.remove();
      // Switch view to pasted
      viewMode = "pasted";
      refreshTabDropdown();
      setTimeout(() => { $tabSelect.value = "pasted"; }, 500);
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
            // Merge content script entities with core.scanText() results for full detection
            var allDetected = resp.entities;
            if (resp.pageText && core && core.scanText) {
              var coreEntities = core.scanText(resp.pageText);
              // Merge — deduplicate by type:id
              var seen = new Set();
              allDetected.forEach(function(e) { seen.add(e.type + ":" + e.id); });
              coreEntities.forEach(function(e) {
                if (!seen.has(e.type + ":" + e.id)) {
                  seen.add(e.type + ":" + e.id);
                  allDetected.push(e);
                }
              });
            }
            // Store merged entities back so tab switch retains all types
            chrome.runtime.sendMessage({
              type: "store-tab-entities",
              tabId: tabId,
              entities: allDetected,
              title: resp.title || "",
              url: resp.url || ""
            });
            loadEntitiesFromArray(allDetected);
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

    // Inject content script via activeTab + scripting, then scan
    function tryInjectAndScan(retriesLeft) {
      chrome.runtime.sendMessage({ type: "inject-and-scan", tabId }, (resp) => {
        if (chrome.runtime.lastError || (resp && resp.error)) {
          if (retriesLeft > 0) {
            // Retry after a short delay — content script may need time to initialize
            setTimeout(() => tryInjectAndScan(retriesLeft - 1), 1000);
            return;
          }
          $loading.style.display = "none";
          $empty.style.display = "block";
          $empty.querySelector("strong").textContent = "Scan failed";
          $empty.querySelector("p").innerHTML = 'Close this sidebar and reopen it, then click <b>Scan</b> again. If the problem persists, this may be a restricted page (e.g., browser internal pages) &mdash; use <b>Paste</b> to scan copied text instead.';
          return;
        }
        // Content script injected and scanning — poll for results
        pullResults();
      });
    }
    tryInjectAndScan(2);
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
  const $scanAllBtn = document.getElementById("btn-scan-all");

  function refreshTabDropdown() {
    chrome.runtime.sendMessage({ type: "get-all-tab-entities" }, (resp) => {
      if (chrome.runtime.lastError || !resp) return;
      const currentVal = $tabSelect.value;
      $tabSelect.innerHTML = '<option value="current">Current tab</option>';
      if (resp.sources && resp.sources.length > 0) {
        $tabSelect.innerHTML += '<option value="all">All tabs (' + resp.entities.length + ')</option>';
        resp.sources.filter(s => s.count > 0).forEach(s => {
          const title = (s.title || "").substring(0, 40);
          $tabSelect.innerHTML += '<option value="' + s.tabId + '">' + escapeHtml(title) + ' (' + s.count + ')</option>';
        });
        // tab count shown via dropdown options
      } else {
      }
      // Restore selection to match viewMode
      if (viewMode && $tabSelect.querySelector('option[value="' + viewMode + '"]')) {
        $tabSelect.value = viewMode;
      } else {
        $tabSelect.value = "current";
      }
    });
  }

  $tabSelect.addEventListener("change", () => {
    viewMode = $tabSelect.value;
    document.getElementById("detail-panel").classList.remove("active");
    activeEntity = null;

    if (viewMode === "current") {
      // Pull current tab's entities and merge with core.scanText()
      chrome.runtime.sendMessage({ type: "get-tab-entities" }, (resp) => {
        if (chrome.runtime.lastError) return;
        var allDetected = (resp && resp.entities) ? resp.entities : [];
        if (allDetected.length > 0 && resp.pageText && core && core.scanText) {
          var coreEntities = core.scanText(resp.pageText);
          var seen = new Set();
          allDetected.forEach(function(e) { seen.add(e.type + ":" + e.id); });
          var added = 0;
          coreEntities.forEach(function(e) {
            if (!seen.has(e.type + ":" + e.id)) { seen.add(e.type + ":" + e.id); allDetected.push(e); added++; }
          });
          if (added > 0 && resp.tabId) {
            chrome.runtime.sendMessage({ type: "store-tab-entities", tabId: resp.tabId, entities: allDetected, title: resp.title || "", url: resp.url || "" });
          }
        }
        loadEntitiesFromArray(allDetected);
        render();
      });
    } else if (viewMode === "all") {
      // Pull all tabs — merge each tab's pageText with core.scanText() for full types
      chrome.runtime.sendMessage({ type: "get-all-tab-entities" }, (resp) => {
        if (chrome.runtime.lastError || !resp) return;
        if (!resp.sources || resp.sources.length === 0) {
          loadEntitiesFromArray([]);
          render();
          return;
        }
        // For each tab, pull its entities+pageText and merge
        var pending = resp.sources.length;
        var allMerged = [];
        var globalSeen = new Set();
        resp.sources.forEach(function(src) {
          chrome.runtime.sendMessage({ type: "get-specific-tab", tabId: parseInt(src.tabId) || src.tabId }, function(tabResp) {
            var tabEntities = (tabResp && tabResp.entities) || [];
            // Merge with core.scanText if pageText available
            if (tabResp && tabResp.pageText && core && core.scanText) {
              var coreEntities = core.scanText(tabResp.pageText);
              var tabSeen = new Set();
              tabEntities.forEach(function(e) { tabSeen.add(e.type + ":" + e.id); });
              var added = 0;
              coreEntities.forEach(function(e) {
                if (!tabSeen.has(e.type + ":" + e.id)) { tabSeen.add(e.type + ":" + e.id); tabEntities.push(e); added++; }
              });
              // Store merged back
              if (added > 0) {
                chrome.runtime.sendMessage({ type: "store-tab-entities", tabId: parseInt(src.tabId) || src.tabId, entities: tabEntities, title: src.title || "" });
              }
            }
            // Add to global merged list
            var title = src.title || "Tab";
            tabEntities.forEach(function(e) {
              var key = e.type + ":" + e.id;
              if (!globalSeen.has(key)) { globalSeen.add(key); allMerged.push(Object.assign({}, e, { source: title })); }
            });
            pending--;
            if (pending <= 0) {
              loadEntitiesFromArray(allMerged);
              render();
            }
          });
        });
      });
    } else {
      // Specific tab ID or "pasted"
      const tabId = viewMode === "pasted" ? "pasted" : parseInt(viewMode);
      chrome.runtime.sendMessage({ type: "get-specific-tab", tabId }, (resp) => {
        if (chrome.runtime.lastError || !resp) return;
        var allDetected = resp.entities || [];
        // Merge with core.scanText() for full entity types
        if (allDetected.length > 0 && resp.pageText && core && core.scanText) {
          var coreEntities = core.scanText(resp.pageText);
          var seen = new Set();
          allDetected.forEach(function(e) { seen.add(e.type + ":" + e.id); });
          var added = 0;
          coreEntities.forEach(function(e) {
            if (!seen.has(e.type + ":" + e.id)) { seen.add(e.type + ":" + e.id); allDetected.push(e); added++; }
          });
          if (added > 0) {
            chrome.runtime.sendMessage({ type: "store-tab-entities", tabId: tabId, entities: allDetected, title: resp.title || "", url: resp.url || "" });
          }
        }
        loadEntitiesFromArray(allDetected);
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
    allEntities = { gene: [], variant: [], accession: [], method: [], genome_build: [], sample_size: [], stat_method: [], platform: [], cell_line: [], tissue: [], drug: [], clinical_trial: [], funding: [], repository: [], p_value: [], finding: [], file: [], species: [] };
    currentEntities = [];
    activeEntity = null;
    searchFilter = "";
    $search.value = "";
    document.getElementById("detail-panel").classList.remove("active");

    if (viewMode === "all") {
      // Clear all tabs
      chrome.runtime.sendMessage({ type: "clear-tab-entities", scope: "all" });
    } else if (viewMode === "pasted") {
      chrome.runtime.sendMessage({ type: "clear-tab-entities", tabId: "pasted" });
    } else if (viewMode !== "current") {
      // Specific tab
      chrome.runtime.sendMessage({ type: "clear-tab-entities", tabId: parseInt(viewMode) });
    } else {
      // Current tab
      chrome.runtime.sendMessage({ type: "clear-tab-entities" });
    }

    // Clear highlights on current page
    chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
      if (tabs[0]) chrome.tabs.sendMessage(tabs[0].id, { type: "clear-highlights" }).catch(() => {});
    });

    viewMode = "current";
    $tabSelect.value = "current";
    refreshTabDropdown();
    render();
  });

  // --- Search ---
  $search.addEventListener("input", (e) => {
    searchFilter = e.target.value.trim();
    render();
  });

  // --- Settings toggle ---
  document.getElementById("btn-settings").addEventListener("click", () => {
    buildEntityTypeToggles(); // refresh toggle state each time
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

  // --- Entity type toggles in settings ---
  function buildEntityTypeToggles() {
    const container = document.getElementById("entity-type-toggles");
    if (!container) return;
    container.innerHTML = "";
    ALL_TYPES.forEach(type => {
      const meta = TYPE_META[type];
      if (!meta) return;
      const row = document.createElement("div");
      row.className = "setting-row";
      row.innerHTML = '<span class="setting-label">' + meta.icon + ' ' + meta.label + '</span>';
      const tog = document.createElement("div");
      tog.className = "toggle" + (enabledTypes.has(type) ? " on" : "");
      tog.dataset.entityType = type;
      tog.addEventListener("click", () => {
        tog.classList.toggle("on");
        if (tog.classList.contains("on")) {
          enabledTypes.add(type);
        } else {
          enabledTypes.delete(type);
        }
        saveEnabledTypes();
        render();
        updateToggleAllBtn();
      });
      row.appendChild(tog);
      container.appendChild(row);
    });
    updateToggleAllBtn();
  }
  function updateToggleAllBtn() {
    const btn = document.getElementById("btn-toggle-all-types");
    if (!btn) return;
    btn.textContent = enabledTypes.size === ALL_TYPES.length ? "All off" : "All on";
  }
  var btnToggleAll = document.getElementById("btn-toggle-all-types");
  if (btnToggleAll) {
    btnToggleAll.addEventListener("click", () => {
      if (enabledTypes.size === ALL_TYPES.length) {
        enabledTypes.clear();
      } else {
        ALL_TYPES.forEach(t => enabledTypes.add(t));
      }
      saveEnabledTypes();
      buildEntityTypeToggles();
      render();
    });
  }
  // Build toggles after a tick (let storage load first)
  setTimeout(buildEntityTypeToggles, 100);

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
      closeFilePanel();
      refreshTabDropdown();
      // Only update view if in "current tab" mode
      if (viewMode === "current") {
        document.getElementById("detail-panel").classList.remove("active");
        chrome.runtime.sendMessage({ type: "get-tab-entities" }, (resp) => {
          if (chrome.runtime.lastError) return;
          var allDetected = (resp && resp.entities) ? resp.entities : [];
          // Merge with core.scanText() for full entity types (in case not merged yet)
          if (allDetected.length > 0 && resp.pageText && core && core.scanText) {
            var coreEntities = core.scanText(resp.pageText);
            var seen = new Set();
            allDetected.forEach(function(e) { seen.add(e.type + ":" + e.id); });
            var added = 0;
            coreEntities.forEach(function(e) {
              if (!seen.has(e.type + ":" + e.id)) {
                seen.add(e.type + ":" + e.id);
                allDetected.push(e);
                added++;
              }
            });
            // Store merged back if we added new types
            if (added > 0 && resp.tabId) {
              chrome.runtime.sendMessage({
                type: "store-tab-entities",
                tabId: resp.tabId,
                entities: allDetected,
                title: resp.title || "",
                url: resp.url || ""
              });
            }
          }
          loadEntitiesFromArray(allDetected);
          activeEntity = null;
          searchFilter = "";
          $search.value = "";
          if (allDetected.length === 0) {
            $empty.querySelector("strong").textContent = "Not scanned yet";
            $empty.querySelector("p").innerHTML = 'Click <b>Scan</b> to analyze this page.';
          }
          render();
        });
      }
    }
  });

  // Handle same-tab navigation — clear stale results
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.type === "tab-navigated" && viewMode === "current") {
      closeFilePanel();
      // Check if the navigated tab is the active one
      chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
        if (tabs[0] && tabs[0].id === msg.tabId) {
          loadEntitiesFromArray([]);
          activeEntity = null;
          document.getElementById("detail-panel").classList.remove("active");
          $empty.querySelector("strong").textContent = "Not scanned yet";
          $empty.querySelector("p").innerHTML = 'Click <b>Scan</b> to analyze this page.';
          render();
        }
      });
    }
  });

  // --- Load entities for current tab from background ---
  function loadEntitiesFromArray(rawEntities) {
    allEntities = { gene: [], variant: [], accession: [], method: [], genome_build: [], sample_size: [], stat_method: [], platform: [], cell_line: [], tissue: [], drug: [], clinical_trial: [], funding: [], repository: [], p_value: [], finding: [], file: [], species: [] };
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
      var allDetected = resp.entities;
      // Merge with core.scanText() for full entity types
      if (resp.pageText && core && core.scanText) {
        var coreEntities = core.scanText(resp.pageText);
        var seen = new Set();
        allDetected.forEach(function(e) { seen.add(e.type + ":" + e.id); });
        var added = 0;
        coreEntities.forEach(function(e) {
          if (!seen.has(e.type + ":" + e.id)) {
            seen.add(e.type + ":" + e.id);
            allDetected.push(e);
            added++;
          }
        });
        if (added > 0 && resp.tabId) {
          chrome.runtime.sendMessage({
            type: "store-tab-entities",
            tabId: resp.tabId,
            entities: allDetected,
            title: resp.title || "",
            url: resp.url || ""
          });
        }
      }
      loadEntitiesFromArray(allDetected);
      render();
    }
  });

  // ══════════════════════════════════════════════════════════════════
  // Feature: Notes on pinned entities
  // ══════════════════════════════════════════════════════════════════
  let entityNotes = {};
  chrome.storage.local.get("biogist_notes", (data) => {
    if (data.biogist_notes) entityNotes = data.biogist_notes;
  });
  function saveNotes() { chrome.storage.local.set({ biogist_notes: entityNotes }); }

  // Patch renderDetail to show notes — we inject into selectEntity
  const _origSelectEntity = typeof selectEntity === "function" ? selectEntity : null;
  // Note: selectEntity is already defined above, so we hook into detail rendering
  // We'll add note UI in a post-render hook

  function addNoteUI(type, name) {
    const key = type + ":" + name;
    const container = document.createElement("div");
    container.style.cssText = "margin-top:10px;padding-top:10px;border-top:1px solid #1e293b;";

    const existing = entityNotes[key] || "";
    if (existing) {
      container.innerHTML = '<div class="note-box">' + escapeHtml(existing) + '</div>';
    }

    const editBtn = document.createElement("button");
    editBtn.className = "btn";
    editBtn.style.cssText = "font-size:10px;margin-top:4px;width:100%;justify-content:center;";
    editBtn.textContent = existing ? "Edit Note" : "Add Note";
    editBtn.addEventListener("click", () => {
      const input = document.createElement("textarea");
      input.className = "note-input";
      input.value = existing;
      input.placeholder = "Add a personal note about this entity...";
      const saveBtn = document.createElement("button");
      saveBtn.className = "btn btn-primary";
      saveBtn.style.cssText = "font-size:10px;margin-top:4px;width:100%;justify-content:center;";
      saveBtn.textContent = "Save Note";
      saveBtn.addEventListener("click", () => {
        const val = input.value.trim();
        if (val) { entityNotes[key] = val; } else { delete entityNotes[key]; }
        saveNotes();
        // Re-render detail
        container.innerHTML = "";
        addNoteUI(type, name);
        $detailBody.appendChild(container);
      });
      container.innerHTML = "";
      container.appendChild(input);
      container.appendChild(saveBtn);
      if (existing) {
        const delBtn = document.createElement("button");
        delBtn.className = "btn";
        delBtn.style.cssText = "font-size:10px;margin-top:4px;width:100%;justify-content:center;color:#f87171;";
        delBtn.textContent = "Delete Note";
        delBtn.addEventListener("click", () => {
          delete entityNotes[key];
          saveNotes();
          container.innerHTML = "";
          addNoteUI(type, name);
          $detailBody.appendChild(container);
        });
        container.appendChild(delBtn);
      }
    });
    container.appendChild(editBtn);
    $detailBody.appendChild(container);
  }

  // Hook into detail panel rendering — observe when it opens
  const detailObserver = new MutationObserver(() => {
    if ($detail.classList.contains("active") && activeEntity) {
      const [type, ...rest] = activeEntity.split(":");
      const name = rest.join(":");
      if (!$detailBody.querySelector(".note-box") && !$detailBody.querySelector(".note-input")) {
        addNoteUI(type, name);
      }
    }
  });
  detailObserver.observe($detail, { attributes: true, attributeFilter: ["class"] });

  // ══════════════════════════════════════════════════════════════════
  // Feature: PubMed search in source links
  // ══════════════════════════════════════════════════════════════════
  // Extend buildSourceLinks in core to always include PubMed
  // We patch it here since core is shared
  const origBuildSourceLinks = core.buildSourceLinks;
  core.buildSourceLinks = function(type, id, subtype) {
    const links = origBuildSourceLinks ? origBuildSourceLinks(type, id, subtype) : [];
    const q = encodeURIComponent(id);

    // PubMed — all types
    let pmQuery = q;
    if (type === "gene") pmQuery = encodeURIComponent(id + "[Gene] AND human[Organism]");
    else if (type === "drug") pmQuery = encodeURIComponent(id + "[MeSH Terms]");
    links.push({ label: "PubMed", url: "https://pubmed.ncbi.nlm.nih.gov/?term=" + pmQuery });

    // Google Scholar — all types
    links.push({ label: "Scholar", url: "https://scholar.google.com/scholar?q=" + q });

    // Type-specific databases
    if (type === "gene") {
      links.push({ label: "OMIM", url: "https://omim.org/search?search=" + q });
      links.push({ label: "ClinicalTrials", url: "https://clinicaltrials.gov/search?term=" + q });
    }
    if (type === "drug") {
      links.push({ label: "DrugBank", url: "https://go.drugbank.com/unearth/q?query=" + q });
      links.push({ label: "ClinicalTrials", url: "https://clinicaltrials.gov/search?intr=" + q });
      links.push({ label: "RxList", url: "https://www.rxlist.com/search/rxlist/" + q });
    }
    if (type === "cell_line") {
      links.push({ label: "Cellosaurus", url: "https://www.cellosaurus.org/search?input=" + q });
    }
    if (type === "variant") {
      links.push({ label: "ClinicalTrials", url: "https://clinicaltrials.gov/search?term=" + q });
    }
    if (type === "species") {
      links.push({ label: "NCBI Taxonomy", url: "https://www.ncbi.nlm.nih.gov/Taxonomy/Browser/wwwtax.cgi?name=" + q });
    }
    if (type === "method") {
      links.push({ label: "bio.tools", url: "https://bio.tools/?q=" + q });
    }

    return links;
  };

  // ══════════════════════════════════════════════════════════════════
  // Feature: Trigger scan from keyboard shortcut
  // ══════════════════════════════════════════════════════════════════
  // Listen for trigger-scan from background (keyboard shortcut Ctrl+Shift+S)
  // Already in message listener but let's make sure it's handled:
  // (Adding to existing onMessage listener via a check)

  // ══════════════════════════════════════════════════════════════════
  // Feature: Compare two tabs
  // ══════════════════════════════════════════════════════════════════
  const $comparePanel = document.getElementById("compare-panel");
  const $compareA = document.getElementById("compare-a");
  const $compareB = document.getElementById("compare-b");

  // Add Compare button to header
  function openCompare() {
    // Populate dropdowns
    chrome.runtime.sendMessage({ type: "get-all-tab-entities" }, (resp) => {
      if (!resp || !resp.sources || resp.sources.length < 2) {
        showToast("Need at least 2 scanned tabs to compare");
        return;
      }
      $compareA.innerHTML = "";
      $compareB.innerHTML = "";
      resp.sources.forEach((s, i) => {
        const title = (s.title || "Tab").substring(0, 40);
        $compareA.innerHTML += '<option value="' + s.tabId + '">' + escapeHtml(title) + ' (' + s.count + ')</option>';
        $compareB.innerHTML += '<option value="' + s.tabId + '">' + escapeHtml(title) + ' (' + s.count + ')</option>';
      });
      if (resp.sources.length > 1) $compareB.selectedIndex = 1;
      $comparePanel.classList.add("visible");
    });
  }

  document.getElementById("btn-close-compare").addEventListener("click", () => $comparePanel.classList.remove("visible"));

  document.getElementById("btn-compare-run").addEventListener("click", () => {
    const idA = $compareA.value, idB = $compareB.value;
    if (idA === idB) { showToast("Select two different tabs"); return; }
    const body = document.getElementById("compare-body");
    body.innerHTML = '<div class="spinner-wrap"><div class="spinner"></div></div>';

    Promise.all([
      new Promise(r => chrome.runtime.sendMessage({ type: "get-specific-tab", tabId: parseInt(idA) || idA }, r)),
      new Promise(r => chrome.runtime.sendMessage({ type: "get-specific-tab", tabId: parseInt(idB) || idB }, r))
    ]).then(([respA, respB]) => {
      const setA = new Set((respA.entities || []).map(e => e.type + ":" + e.id));
      const setB = new Set((respB.entities || []).map(e => e.type + ":" + e.id));
      const onlyA = [], shared = [], onlyB = [];
      setA.forEach(k => { if (setB.has(k)) shared.push(k); else onlyA.push(k); });
      setB.forEach(k => { if (!setA.has(k)) onlyB.push(k); });

      const titleA = (respA.title || "Tab A").substring(0, 25);
      const titleB = (respB.title || "Tab B").substring(0, 25);
      const jaccard = shared.length / (onlyA.length + shared.length + onlyB.length) || 0;

      body.innerHTML = '<div style="text-align:center;font-size:11px;color:#94a3b8;padding:6px 0;border-bottom:1px solid #1e293b;margin-bottom:8px">' +
        'Only A: <b style="color:#f87171">' + onlyA.length + '</b> &nbsp; Shared: <b style="color:#4ade80">' + shared.length + '</b> &nbsp; Only B: <b style="color:#f87171">' + onlyB.length + '</b> &nbsp; Similarity: <b>' + (jaccard * 100).toFixed(0) + '%</b></div>';

      const cols = document.createElement("div");
      cols.className = "compare-cols";

      function renderCol(title, items, cls) {
        const col = document.createElement("div");
        col.className = "compare-col";
        col.innerHTML = '<div class="compare-col-header">' + escapeHtml(title) + '</div>';
        items.forEach(k => {
          const [t, ...r] = k.split(":");
          const meta = TYPE_META[t] || {};
          const d = document.createElement("div");
          d.className = "compare-item " + cls;
          d.textContent = (meta.icon || "") + " " + r.join(":");
          col.appendChild(d);
        });
        if (items.length === 0) col.innerHTML += '<div class="compare-item" style="color:#475569;font-style:italic">None</div>';
        return col;
      }

      cols.appendChild(renderCol("Only " + titleA, onlyA, "compare-only"));
      cols.appendChild(renderCol("Shared", shared, "compare-shared"));
      cols.appendChild(renderCol("Only " + titleB, onlyB, "compare-only"));
      body.appendChild(cols);
    });
  });

  // ══════════════════════════════════════════════════════════════════
  // Feature: Co-occurrence Matrix
  // ══════════════════════════════════════════════════════════════════
  const $matrixPanel = document.getElementById("matrix-panel");

  function openMatrix() {
    chrome.runtime.sendMessage({ type: "get-tab-entity-map" }, (tabMap) => {
      if (!tabMap || Object.keys(tabMap).length < 2) {
        showToast("Need at least 2 scanned tabs for co-occurrence");
        return;
      }
      const body = document.getElementById("matrix-body");
      body.innerHTML = "";

      // Build type-per-tab map
      const types = ALL_TYPES.filter(t => enabledTypes.has(t));
      const tabIds = Object.keys(tabMap);

      // Count: for each pair of types, how many tabs have both?
      const cooccur = {};
      types.forEach(t1 => { cooccur[t1] = {}; types.forEach(t2 => { cooccur[t1][t2] = 0; }); });

      tabIds.forEach(tid => {
        const ents = tabMap[tid].entities || [];
        const typesInTab = new Set(ents.map(e => e.type));
        types.forEach(t1 => {
          if (!typesInTab.has(t1)) return;
          types.forEach(t2 => {
            if (typesInTab.has(t2)) cooccur[t1][t2]++;
          });
        });
      });

      // Find max for color scale
      let maxVal = 0;
      types.forEach(t1 => types.forEach(t2 => { if (t1 !== t2 && cooccur[t1][t2] > maxVal) maxVal = cooccur[t1][t2]; }));

      // Render grid
      const n = types.length + 1;
      const grid = document.createElement("div");
      grid.className = "matrix-grid";
      grid.style.gridTemplateColumns = "80px " + types.map(() => "1fr").join(" ");

      // Header row
      grid.innerHTML = '<div class="matrix-label"></div>';
      types.forEach(t => {
        const meta = TYPE_META[t] || {};
        grid.innerHTML += '<div class="matrix-label" title="' + (meta.label || t) + '">' + (meta.icon || "") + '</div>';
      });

      // Data rows
      types.forEach(t1 => {
        const meta1 = TYPE_META[t1] || {};
        grid.innerHTML += '<div class="matrix-label" style="text-align:right">' + (meta1.icon || "") + ' ' + (meta1.label || t1).substring(0, 10) + '</div>';
        types.forEach(t2 => {
          const val = cooccur[t1][t2];
          const intensity = maxVal > 0 ? val / maxVal : 0;
          const bg = t1 === t2 ? "rgba(100,116,139,0.1)" : "rgba(124,58,237," + (intensity * 0.6).toFixed(2) + ")";
          grid.innerHTML += '<div class="matrix-cell" style="background:' + bg + '" title="' + (meta1.label || t1) + ' + ' + ((TYPE_META[t2] || {}).label || t2) + ': ' + val + ' tabs">' + (val > 0 ? val : "") + '</div>';
        });
      });

      body.appendChild(grid);
      body.innerHTML += '<div style="font-size:10px;color:#475569;margin-top:12px;text-align:center">Each cell shows how many tabs contain both entity types. Across ' + tabIds.length + ' scanned tabs.</div>';
      $matrixPanel.classList.add("visible");
    });
  }

  document.getElementById("btn-close-matrix").addEventListener("click", () => $matrixPanel.classList.remove("visible"));

  // ══════════════════════════════════════════════════════════════════
  // Feature: Entity History / Timeline
  // ══════════════════════════════════════════════════════════════════
  const $historyPanel = document.getElementById("history-panel");



  let historyData = []; // cached for filtering
  let historyTypeFilter = "all";
  let historySearchFilter = "";

  function renderHistory() {
    const body = document.getElementById("history-body");
    if (historyData.length === 0) {
      body.innerHTML = '<div style="text-align:center;color:#475569;padding:40px;font-size:12px">No history yet. Scan some pages to start tracking.</div>';
      return;
    }

    // Deduplicate
    const entityMap = {};
    historyData.forEach(h => {
      const key = h.type + ":" + h.id;
      if (!entityMap[key]) {
        entityMap[key] = { type: h.type, id: h.id, count: 0, lastTs: h.ts, lastTitle: h.title, lastUrl: h.url };
      }
      entityMap[key].count++;
      if (h.ts > entityMap[key].lastTs) {
        entityMap[key].lastTs = h.ts;
        entityMap[key].lastTitle = h.title;
        entityMap[key].lastUrl = h.url;
      }
    });

    // Apply filters
    let sorted = Object.values(entityMap).sort((a, b) => b.lastTs - a.lastTs);
    if (historyTypeFilter !== "all") {
      sorted = sorted.filter(e => e.type === historyTypeFilter);
    }
    if (historySearchFilter) {
      const q = historySearchFilter.toLowerCase();
      sorted = sorted.filter(e => e.id.toLowerCase().includes(q) || (e.lastTitle || "").toLowerCase().includes(q));
    }

    // Build filter bar
    let html = '<div style="display:flex;gap:6px;margin-bottom:10px;align-items:center">' +
      '<div style="position:relative;flex:1">' +
        '<span style="position:absolute;left:8px;top:50%;transform:translateY(-50%);font-size:12px;opacity:0.5;pointer-events:none">&#x1F50D;</span>' +
        '<input type="text" id="history-search" placeholder="Search history..." value="' + escapeHtml(historySearchFilter) + '" style="width:100%;padding:5px 8px 5px 28px;border-radius:4px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;font-size:11px">' +
      '</div>' +
      '<select id="history-type-filter" style="padding:5px 6px;border-radius:4px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;font-size:11px;cursor:pointer">' +
        '<option value="all"' + (historyTypeFilter === "all" ? " selected" : "") + '>All types</option>';
    ALL_TYPES.forEach(t => {
      const meta = TYPE_META[t] || {};
      html += '<option value="' + t + '"' + (historyTypeFilter === t ? " selected" : "") + '>' + (meta.icon || "") + ' ' + (meta.label || t) + '</option>';
    });
    html += '</select></div>';

    // Stats
    html += '<div style="font-size:10px;color:#64748b;margin-bottom:8px">' + sorted.length + ' unique entities' + (historyTypeFilter !== "all" || historySearchFilter ? ' (filtered)' : '') + ' across ' + historyData.length + ' total detections</div>';

    // Group by date
    const byDate = {};
    sorted.forEach(e => {
      const d = new Date(e.lastTs).toLocaleDateString();
      if (!byDate[d]) byDate[d] = [];
      byDate[d].push(e);
    });

    for (const [date, items] of Object.entries(byDate)) {
      html += '<div class="history-date">' + date + '</div>';
      items.slice(0, 100).forEach(e => {
        const meta = TYPE_META[e.type] || {};
        const time = new Date(e.lastTs).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
        html += '<div class="history-item">' +
          '<span style="font-size:12px">' + (meta.icon || "") + '</span>' +
          '<span class="history-item-id">' + escapeHtml(e.id) + '</span>' +
          '<span class="entity-badge ' + (meta.cls || "") + '" style="font-size:8px">' + (meta.badge || e.type) + '</span>' +
          '<span style="font-size:9px;color:#64748b">\u00d7' + e.count + '</span>' +
          '<span class="history-item-src" title="' + escapeHtml(e.lastTitle || "") + '">' + escapeHtml((e.lastTitle || "").substring(0, 30)) + '</span>' +
          '<span class="history-item-time">' + time + '</span>' +
          '</div>';
      });
    }

    if (sorted.length === 0) {
      html += '<div style="text-align:center;color:#475569;padding:20px;font-size:12px">No matches for current filter.</div>';
    }

    body.innerHTML = html;

    // Wire filter events
    document.getElementById("history-search").addEventListener("input", (e) => {
      historySearchFilter = e.target.value.trim();
      renderHistory();
    });
    document.getElementById("history-type-filter").addEventListener("change", (e) => {
      historyTypeFilter = e.target.value;
      renderHistory();
    });
  }

  function openHistory() {
    chrome.runtime.sendMessage({ type: "get-entity-history" }, (resp) => {
      historyData = (resp && resp.history) || [];
      historyTypeFilter = "all";
      historySearchFilter = "";
      renderHistory();
      $historyPanel.classList.add("visible");
    });
  }

  document.getElementById("btn-close-history").addEventListener("click", () => $historyPanel.classList.remove("visible"));
  document.getElementById("btn-clear-history").addEventListener("click", () => {
    chrome.runtime.sendMessage({ type: "clear-entity-history" });
    document.getElementById("history-body").innerHTML = '<div style="text-align:center;color:#475569;padding:40px;font-size:12px">History cleared.</div>';
    showToast("History cleared");
  });

  // ══════════════════════════════════════════════════════════════════
  // Feature: Batch scan URLs
  // ══════════════════════════════════════════════════════════════════
  const $batchPanel = document.getElementById("batch-panel");

  // Batch wired via More menu
  document.getElementById("btn-close-batch").addEventListener("click", () => $batchPanel.classList.remove("visible"));

  document.getElementById("btn-batch-run").addEventListener("click", () => {
    const text = document.getElementById("batch-urls").value.trim();
    if (!text) { showToast("Paste some URLs first"); return; }
    const urls = text.split("\n").map(u => u.trim()).filter(u => u.startsWith("http"));
    if (urls.length === 0) { showToast("No valid URLs found"); return; }
    if (urls.length > 20) { showToast("Max 20 URLs at a time"); return; }

    const prog = document.getElementById("batch-progress");
    prog.style.display = "block";
    prog.textContent = "Starting scan of " + urls.length + " URLs...";
    document.getElementById("btn-batch-run").disabled = true;

    chrome.runtime.sendMessage({ type: "batch-scan", urls });
  });

  // Listen for batch progress/complete
  // (Added to existing onMessage listener scope)

  // ══════════════════════════════════════════════════════════════════
  // Feature: Handle trigger-scan and batch messages
  // ══════════════════════════════════════════════════════════════════
  // Extend the existing onMessage listener
  const origOnMessage = chrome.runtime.onMessage.hasListeners ? null : null;
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.type === "trigger-scan") {
      // Keyboard shortcut triggered scan
      document.getElementById("btn-scan").click();
    }
    if (msg.type === "batch-scan-progress") {
      const prog = document.getElementById("batch-progress");
      if (prog) prog.textContent = "Scanning " + msg.done + "/" + msg.total + ": " + (msg.url || "").substring(0, 40) + "...";
    }
    if (msg.type === "batch-scan-complete") {
      const prog = document.getElementById("batch-progress");
      if (prog) prog.textContent = "Done! Scanned " + msg.total + " URLs.";
      document.getElementById("btn-batch-run").disabled = false;
      showToast("Batch scan complete: " + msg.total + " URLs");
      // Close batch panel and switch to All tabs view
      document.getElementById("batch-panel").classList.remove("visible");
      setTimeout(() => {
        const $tabSelect = document.getElementById("tab-select");
        refreshTabDropdown();
        setTimeout(() => { $tabSelect.value = "all"; $tabSelect.dispatchEvent(new Event("change")); }, 500);
      }, 1000);
    }
  });

  // --- Responsive header: inline buttons + More menu overflow ---
  const $moreMenu = document.getElementById("more-menu");
  const $btnMore = document.getElementById("btn-more");
  const toolButtons = [
    { inline: document.getElementById("btn-export-inline"), menu: document.getElementById("menu-export"), action: openExportDialog },
    { inline: document.getElementById("btn-history-inline"), menu: document.getElementById("menu-history"), action: openHistory },
    { inline: document.getElementById("btn-compare-inline"), menu: document.getElementById("menu-compare"), action: openCompare },
    { inline: document.getElementById("btn-matrix-inline"), menu: document.getElementById("menu-matrix"), action: openMatrix },
    { inline: document.getElementById("btn-batch-inline"), menu: document.getElementById("menu-batch"), action: () => $batchPanel.classList.add("visible") },
  ];

  // Wire inline buttons
  toolButtons.forEach(tb => {
    tb.inline.addEventListener("click", tb.action);
    tb.menu.addEventListener("click", () => { $moreMenu.classList.remove("open"); tb.action(); });
  });

  // More menu toggle
  $btnMore.addEventListener("click", (e) => { e.stopPropagation(); $moreMenu.classList.toggle("open"); });
  document.addEventListener("click", () => $moreMenu.classList.remove("open"));
  $moreMenu.addEventListener("click", (e) => e.stopPropagation());

  // Adaptive layout: measure header, hide overflowing tool buttons into More menu
  function adaptHeader() {
    // First show all inline, hide more
    toolButtons.forEach(tb => { tb.inline.style.display = ""; tb.menu.style.display = "none"; });
    $btnMore.style.display = "none";

    const header = document.querySelector(".header-actions");
    const maxWidth = header.parentElement.clientWidth - document.querySelector(".header-left").offsetWidth - 8;

    // Check if header overflows
    if (header.scrollWidth <= maxWidth) return; // all fit

    // Hide from right to left until it fits
    let overflowed = false;
    for (let i = toolButtons.length - 1; i >= 0; i--) {
      if (header.scrollWidth > maxWidth || overflowed) {
        toolButtons[i].inline.style.display = "none";
        toolButtons[i].menu.style.display = "";
        $btnMore.style.display = "";
        overflowed = true;
      }
    }
  }

  // Run on load and resize
  adaptHeader();
  new ResizeObserver(adaptHeader).observe(document.querySelector(".header"));

  // --- Initial render ---
  render();
})();
