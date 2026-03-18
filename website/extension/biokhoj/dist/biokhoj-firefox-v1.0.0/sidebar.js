// BioKhoj — Sidebar Logic
// Lightweight research radar sidebar for the BioKhoj Chrome extension.

(function () {
  "use strict";

  // ── DOM refs ────────────────────────────────────────────────────────

  const $addInput     = document.getElementById("add-input");
  const $btnAdd       = document.getElementById("btn-add");
  const $btnRefresh   = document.getElementById("btn-refresh");
  const $btnTheme     = document.getElementById("btn-theme");
  const $btnOpenPwa   = document.getElementById("btn-open-pwa");
  const $btnOpenBk    = document.getElementById("btn-open-biokhoj");
  const $watchChips   = document.getElementById("watchlist-chips");
  const $watchCount   = document.getElementById("watchlist-count");
  const $paperList    = document.getElementById("paper-list");
  const $papersCount  = document.getElementById("papers-count");
  const $loading      = document.getElementById("loading");
  const $emptyState   = document.getElementById("empty-state");
  const $statusText   = document.getElementById("status-text");
  const $toast        = document.getElementById("toast");
  const $btnMore      = document.getElementById("btn-more");
  const $moreMenu     = document.getElementById("more-menu");
  const $paperSearch  = document.getElementById("paper-search");
  const $paperSort    = document.getElementById("paper-sort");
  const $searchFilter = document.getElementById("search-filter");

  const MAX_PAPERS_SHOWN = 20;

  // ── Skeleton / Error / Onboarding DOM refs ────────────────────────
  const $skeletonWatchlist = document.getElementById("skeleton-watchlist");
  const $skeletonPapers   = document.getElementById("skeleton-papers");
  const $errorBar         = document.getElementById("error-bar");
  const $errorBarText     = document.getElementById("error-bar-text");
  const $errorBarAction   = document.getElementById("error-bar-action");
  const $onboardingCard   = document.getElementById("onboarding-card");

  // ── State ───────────────────────────────────────────────────────────

  let watchlist = [];
  let papers = [];
  let darkTheme = true;
  let readingList = [];
  let settings = {};
  let currentDetailPaper = null;
  let citationData = {};
  let journalClubIndex = 0;
  let selectedPaperIndex = -1;
  let onboarded = false;

  // ── Init ────────────────────────────────────────────────────────────

  loadTheme();
  loadReadingList();
  loadSettings();
  loadCitations();
  loadOnboardingState();
  refresh();

  // ── Entity Classification ───────────────────────────────────────────

  function classifyEntity(text) {
    const trimmed = (text || "").trim();
    if (!trimmed) return null;

    if (/^rs\d+$/i.test(trimmed)) return { name: trimmed, type: "variant" };
    if (/^(c|g|p|m)\.\d/i.test(trimmed)) return { name: trimmed, type: "variant" };
    if (/^(NM_|NP_|NC_)\d+.*:[cpg]\./i.test(trimmed)) return { name: trimmed, type: "variant" };

    if (/(?:mab|nib|lib|vir|tin|ide|ine|ase|pril|olol|statin|sartan|dipine|cycline|mycin|cillin)$/i.test(trimmed)) {
      return { name: trimmed, type: "drug" };
    }

    if (/(?:oma|emia|itis|osis|pathy|trophy|plasia|carcinoma|lymphoma|leukemia|syndrome)$/i.test(trimmed)) {
      return { name: trimmed, type: "disease" };
    }

    if (/(?:pathway|signaling|cascade|axis)$/i.test(trimmed)) {
      return { name: trimmed, type: "pathway" };
    }

    if (/^[A-Z][A-Z0-9]{1,9}$/.test(trimmed)) return { name: trimmed, type: "gene" };

    return { name: trimmed, type: "gene" };
  }

  // ── Messaging helpers ───────────────────────────────────────────────

  function sendMsg(msg) {
    return new Promise((resolve, reject) => {
      try {
        chrome.runtime.sendMessage(msg, (resp) => {
          if (chrome.runtime.lastError) {
            console.warn("BioKhoj sendMsg error:", chrome.runtime.lastError.message);
            // Service worker may be waking up — retry once after short delay
            setTimeout(() => {
              chrome.runtime.sendMessage(msg, (resp2) => {
                if (chrome.runtime.lastError) {
                  reject(new Error(chrome.runtime.lastError.message));
                } else {
                  resolve(resp2 || {});
                }
              });
            }, 500);
          } else {
            resolve(resp || {});
          }
        });
      } catch (e) {
        reject(e);
      }
    });
  }

  // ── Refresh all data ────────────────────────────────────────────────

  async function refresh() {
    showSkeletons(true);
    setStatus("Loading...");

    try {
      const wlResp = await sendMsg({ type: "get-watchlist" });
      watchlist = wlResp.watchlist || [];

      const ppResp = await sendMsg({ type: "get-new-papers", since: 0 });
      papers = ppResp.papers || [];

      renderWatchlist();
      renderPapers();
      updateEmptyState();
      updateOnboarding();
      if ($searchFilter) $searchFilter.style.display = papers.length > 0 ? "flex" : "none";
      // Show/hide watch empty state
      var $watchEmpty = document.getElementById("watch-empty");
      if ($watchEmpty) $watchEmpty.style.display = watchlist.length === 0 ? "" : "none";
      dismissError();

      const countResp = await sendMsg({ type: "get-paper-count" });
      setStatus(countResp.unread > 0
        ? `${countResp.unread} unread of ${countResp.total} papers`
        : `${countResp.total} papers tracked`
      );
    } catch (e) {
      console.warn("BioKhoj sidebar refresh error:", e);
      setStatus("Error loading data");
      showError("Could not load data. Extension may need reloading.", "Retry", () => refresh());
    }

    showSkeletons(false);
  }

  // ── Render Watchlist ────────────────────────────────────────────────

  function renderWatchlist() {
    $watchChips.innerHTML = "";
    $watchCount.textContent = watchlist.length;

    watchlist.forEach((entity) => {
      const chip = document.createElement("span");
      const ec = entityColor(entity.name);
      chip.className = "chip";
      chip.style.background = ec.bg;
      chip.style.color = ec.fg;

      const nameSpan = document.createElement("span");
      nameSpan.textContent = entity.name;
      chip.appendChild(nameSpan);

      // Count unread papers for this entity
      const unread = papers.filter(p => !p.read && p.entity === entity.name).length;
      if (unread > 0) {
        const badge = document.createElement("span");
        badge.className = "chip-badge";
        badge.textContent = unread;
        chip.appendChild(badge);
      }

      const remove = document.createElement("span");
      remove.className = "chip-remove";
      remove.textContent = "\u00D7";
      remove.title = "Remove";
      remove.addEventListener("click", (e) => {
        e.stopPropagation();
        removeEntity(entity.name, entity.type);
      });
      chip.appendChild(remove);

      $watchChips.appendChild(chip);
    });
  }

  // ── Render Papers ───────────────────────────────────────────────────

  function getFilteredSortedPapers() {
    let filtered = [...papers];
    const query = ($paperSearch ? $paperSearch.value : "").toLowerCase().trim();
    if (query) {
      filtered = filtered.filter(p =>
        (p.title || "").toLowerCase().includes(query) ||
        (p.journal || "").toLowerCase().includes(query) ||
        (p.entity || "").toLowerCase().includes(query) ||
        (p.authors || "").toLowerCase().includes(query)
      );
    }
    const sortBy = $paperSort ? $paperSort.value : "signal";
    filtered.sort((a, b) => {
      if (sortBy === "signal") return (b.signalScore || 0) - (a.signalScore || 0);
      if (sortBy === "date") return (b.fetchedAt || 0) - (a.fetchedAt || 0);
      if (sortBy === "journal") return (a.journal || "").localeCompare(b.journal || "");
      if (sortBy === "title") return (a.title || "").localeCompare(b.title || "");
      return 0;
    });
    return filtered;
  }

  function renderPapers() {
    $paperList.innerHTML = "";
    const filtered = getFilteredSortedPapers();
    const shown = filtered.slice(0, MAX_PAPERS_SHOWN);
    $papersCount.textContent = papers.length;

    // Date grouping
    const now = Date.now();
    const todayStart = new Date(); todayStart.setHours(0,0,0,0);
    const weekStart = new Date(todayStart); weekStart.setDate(weekStart.getDate() - 7);
    let lastGroup = "";

    shown.forEach((paper, idx) => {
      // Insert date group headers
      const paperTime = paper.fetchedAt || new Date(paper.date || 0).getTime() || 0;
      let group;
      if (paperTime >= todayStart.getTime()) group = "Today";
      else if (paperTime >= weekStart.getTime()) group = "This week";
      else group = "Older";

      if (group !== lastGroup) {
        const groupCount = shown.filter(p => {
          const t = p.fetchedAt || new Date(p.date || 0).getTime() || 0;
          if (group === "Today") return t >= todayStart.getTime();
          if (group === "This week") return t >= weekStart.getTime() && t < todayStart.getTime();
          return t < weekStart.getTime();
        }).length;
        const header = document.createElement("li");
        header.className = "date-group-header";
        header.innerHTML = group + "<span>(" + groupCount + ")</span>";
        $paperList.appendChild(header);
        lastGroup = group;
      }

      const li = document.createElement("li");
      // Signal color bar class
      const score = paper.signalScore || 0;
      const barClass = score >= 70 ? " signal-bar-high" : score >= 40 ? " signal-bar-mid" : " signal-bar-low";
      li.className = "paper-item" + (paper.read ? " read" : "") + barClass + (idx === selectedPaperIndex ? " paper-selected" : "");
      li.setAttribute("data-paper-index", idx);
      li.setAttribute("data-pmid", paper.pmid || "");

      // Selection checkbox
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.className = "paper-checkbox";
      cb.addEventListener("change", () => {
        li.classList.toggle("selected", cb.checked);
        updateSelectCount();
      });
      li.appendChild(cb);

      // Title
      const titleDiv = document.createElement("div");
      titleDiv.className = "paper-title";
      titleDiv.textContent = paper.title;
      li.appendChild(titleDiv);

      // Meta row: journal, date, signal badge, entity chip
      const metaDiv = document.createElement("div");
      metaDiv.className = "paper-meta";

      if (paper.journal) {
        const journal = document.createElement("span");
        journal.className = "paper-journal";
        journal.textContent = truncate(paper.journal, 30);
        metaDiv.appendChild(journal);
      }

      if (paper.date) {
        const date = document.createElement("span");
        date.textContent = paper.date;
        metaDiv.appendChild(date);
      }

      // Signal Score badge (reuses score from bar class above)
      const signalBadge = document.createElement("span");
      signalBadge.className = "signal-badge " + signalClass(score);
      signalBadge.textContent = signalLabel(score);
      metaDiv.appendChild(signalBadge);

      // Entity chip (color-coded by name)
      if (paper.entity) {
        const ec = entityColor(paper.entity);
        const entityChip = document.createElement("span");
        entityChip.className = "paper-entity-chip";
        entityChip.style.background = ec.bg;
        entityChip.style.color = ec.fg;
        entityChip.textContent = paper.entity;
        metaDiv.appendChild(entityChip);
      }

      li.appendChild(metaDiv);

      // Actions
      const actionsDiv = document.createElement("div");
      actionsDiv.className = "paper-actions";

      const openBtn = makeActionBtn("Open", () => {
        window.open(paper.url, "_blank");
        trackHistory(paper);
        if (!paper.read) markRead(paper.pmid);
      });
      actionsDiv.appendChild(openBtn);

      const citeBtn = makeActionBtn("Cite", () => {
        const citation = formatCitation(paper);
        navigator.clipboard.writeText(citation).then(() => {
          showToast("Citation copied");
        });
      });
      actionsDiv.appendChild(citeBtn);

      const isSaved = isInReadingList(paper.pmid);
      const saveBtn = makeActionBtn(isSaved ? "Unsave" : "Save", () => {
        toggleSave(paper);
      });
      if (isSaved) saveBtn.style.color = "#F4C430";
      actionsDiv.appendChild(saveBtn);

      const readBtn = makeActionBtn(paper.read ? "Mark Unread" : "Mark Read", () => {
        toggleRead(paper.pmid, !paper.read);
      });
      actionsDiv.appendChild(readBtn);

      li.appendChild(actionsDiv);

      // Click title to open paper detail overlay
      titleDiv.addEventListener("click", (e) => {
        e.stopPropagation();
        openPaperDetail(paper);
      });

      $paperList.appendChild(li);
    });
  }

  // ── Signal Score helpers ────────────────────────────────────────────

  function signalClass(score) {
    if (score >= 70) return "signal-high";
    if (score >= 40) return "signal-mid";
    return "signal-low";
  }

  function signalLabel(score) {
    if (score >= 70) return "\u2605 " + score;
    if (score >= 40) return score;
    return "\u26A0 " + score;
  }

  // ── Add entity ──────────────────────────────────────────────────────

  async function addEntity(nameOverride) {
    const text = nameOverride || $addInput.value.trim();
    if (!text) return;

    const entity = classifyEntity(text);
    if (!entity) return;

    const resp = await sendMsg({ type: "add-watch", entity });
    if (resp.ok) {
      if (!nameOverride) $addInput.value = "";
      showToast(`Watching ${entity.name} (${entity.type})`);
      dismissOnboarding();
      await refresh();
      // Auto-check if this is the first entity and no papers yet
      if (papers.length === 0) {
        await checkNow();
      }
    } else if (resp.reason === "duplicate") {
      showToast(`${entity.name} is already watched`);
    }
  }

  // ── Remove entity ──────────────────────────────────────────────────

  async function removeEntity(name, type) {
    await sendMsg({ type: "remove-watch", name, entityType: type });
    showToast(`Removed ${name}`);
    refresh();
  }

  // ── Mark read ──────────────────────────────────────────────────────

  async function markRead(pmid) {
    await toggleRead(pmid, true);
  }

  async function toggleRead(pmid, readState) {
    await sendMsg({ type: "mark-read", pmid, read: readState });
    papers = papers.map(p => p.pmid === pmid ? { ...p, read: readState } : p);
    renderPapers();
    renderWatchlist();
  }

  // ── Check now ──────────────────────────────────────────────────────

  async function checkNow() {
    $btnRefresh.disabled = true;

    if (papers.length > 0) {
      // Has existing papers — show bottom loading indicator instead of replacing content
      showBottomLoader(true);
      setStatus("Checking PubMed + bioRxiv...");
    } else {
      // Empty — show full skeleton
      showSkeletons(true);
      setStatus("Checking PubMed + bioRxiv...");
    }

    try {
      const resp = await sendMsg({ type: "check-now" });
      if (resp && resp.error === "rate_limit") {
        showError("\u23F3 Rate limited \u2014 waiting 30 seconds.", "Dismiss", () => dismissError());
        showToast("Rate limited by PubMed");
      } else {
        showToast("Check complete");
        dismissError();
      }
    } catch (e) {
      showError("\u26A0\uFE0F PubMed is slow or unavailable.", "Retry", () => checkNow());
      showToast("Check failed");
    }

    showBottomLoader(false);
    $btnRefresh.disabled = false;
    refresh();
  }

  function showBottomLoader(show) {
    // Top progress bar (always visible)
    var topBar = document.getElementById("check-progress-bar");
    if (show) {
      if (!topBar) {
        topBar = document.createElement("div");
        topBar.id = "check-progress-bar";
        topBar.style.cssText = "height:2px;background:linear-gradient(90deg,#F4C430,#e0a800,#F4C430);background-size:200% 100%;animation:shimmer 1.5s infinite;position:sticky;top:0;z-index:5;";
        var panel = document.getElementById("panel-recent");
        if (panel) panel.insertBefore(topBar, panel.firstChild);
      }
      // Bottom spinner in paper list
      var existing = document.getElementById("bottom-loader");
      if (!existing) {
        var loader = document.createElement("li");
        loader.id = "bottom-loader";
        loader.style.cssText = "padding:12px 14px;text-align:center;border:none;";
        loader.innerHTML =
          '<div style="display:flex;align-items:center;justify-content:center;gap:8px">' +
            '<div style="width:14px;height:14px;border:2px solid rgba(244,196,48,0.3);border-top-color:#F4C430;border-radius:50%;animation:spin 0.8s linear infinite"></div>' +
            '<span style="font-size:11px;color:#64748b">Checking for new papers...</span>' +
          '</div>';
        $paperList.appendChild(loader);
      }
    } else {
      if (topBar) topBar.remove();
      var existing = document.getElementById("bottom-loader");
      if (existing) existing.remove();
    }
  }

  // ── Theme toggle ───────────────────────────────────────────────────

  function loadTheme() {
    chrome.storage.local.get("biokhoj_settings", (data) => {
      const settings = data.biokhoj_settings || {};
      darkTheme = settings.theme !== "light";
      applyTheme();
    });
  }

  function toggleTheme() {
    darkTheme = !darkTheme;
    applyTheme();
    chrome.storage.local.get("biokhoj_settings", (data) => {
      const settings = data.biokhoj_settings || {};
      settings.theme = darkTheme ? "dark" : "light";
      chrome.storage.local.set({ biokhoj_settings: settings });
    });
  }

  function applyTheme() {
    document.body.classList.toggle("light-theme", !darkTheme);
    $btnTheme.textContent = darkTheme ? "\u263E" : "\u2600";
  }

  // ── UI Helpers ─────────────────────────────────────────────────────

  function showLoading(show) {
    $loading.style.display = show ? "flex" : "none";
  }

  function showSkeletons(show) {
    if (show) {
      if ($skeletonWatchlist) $skeletonWatchlist.style.display = "flex";
      if ($skeletonPapers) $skeletonPapers.style.display = "block";
      if ($loading) $loading.style.display = "none";
    } else {
      if ($skeletonWatchlist) $skeletonWatchlist.style.display = "none";
      if ($skeletonPapers) $skeletonPapers.style.display = "none";
    }
  }

  // ── Error bar ─────────────────────────────────────────────────────

  function showError(message, actionLabel, actionFn) {
    $errorBarText.textContent = message;
    $errorBarAction.textContent = actionLabel;
    $errorBar.className = "error-bar error-bar-warning visible";
    if (message.includes("Rate limited")) {
      $errorBar.className = "error-bar error-bar-ratelimit visible";
    }
    $errorBarAction.onclick = (e) => {
      e.stopPropagation();
      actionFn();
    };
  }

  function dismissError() {
    $errorBar.classList.remove("visible");
  }

  // ── Onboarding ────────────────────────────────────────────────────

  function loadOnboardingState() {
    chrome.storage.local.get("biokhoj_onboarded", (data) => {
      onboarded = !!data.biokhoj_onboarded;
    });
  }

  function updateOnboarding() {
    if (!$onboardingCard) return;
    if (onboarded || watchlist.length > 0) {
      $onboardingCard.style.display = "none";
      return;
    }
    $onboardingCard.style.display = "block";
  }

  function dismissOnboarding() {
    onboarded = true;
    if ($onboardingCard) $onboardingCard.style.display = "none";
    chrome.storage.local.set({ biokhoj_onboarded: true });
  }

  function updateEmptyState() {
    const hasContent = papers.length > 0;
    if ($emptyState) {
      $emptyState.style.display = hasContent ? "none" : "block";
      var emptyText = document.getElementById("empty-state-text");
      var emptyBtn = document.getElementById("btn-empty-check");
      if (emptyText && emptyBtn) {
        if (watchlist.length > 0 && papers.length === 0) {
          emptyText.textContent = "You're watching " + watchlist.length + " entit" + (watchlist.length === 1 ? "y" : "ies") + ". Check for papers now.";
          emptyBtn.style.display = "";
        } else if (watchlist.length === 0) {
          emptyText.textContent = "No papers yet. Add entities to your watchlist to get started.";
          emptyBtn.style.display = "none";
        } else {
          emptyBtn.style.display = "none";
        }
      }
    }
  }

  function setStatus(text) {
    if ($statusText) $statusText.textContent = text;
  }

  function showToast(message) {
    $toast.textContent = message;
    $toast.classList.add("show");
    setTimeout(() => $toast.classList.remove("show"), 2000);
  }

  function truncate(str, len) {
    if (!str) return "";
    return str.length > len ? str.slice(0, len) + "\u2026" : str;
  }

  function makeActionBtn(label, onClick) {
    const btn = document.createElement("button");
    btn.className = "paper-action";
    btn.textContent = label;
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      onClick();
    });
    return btn;
  }

  function formatCitation(paper) {
    const authors = paper.authors || "Unknown";
    const title = paper.title || "Untitled";
    const journal = paper.journal || "";
    const date = paper.date || "";
    const pmid = paper.pmid || "";
    let cite = `${authors}. ${title}`;
    if (journal) cite += ` ${journal}.`;
    if (date) cite += ` ${date}.`;
    if (pmid) cite += ` PMID: ${pmid}.`;
    return cite;
  }

  // ── More Menu ──────────────────────────────────────────────────────

  function toggleMoreMenu() {
    $moreMenu.classList.toggle("open");
  }

  function closeMoreMenu() {
    $moreMenu.classList.remove("open");
  }

  document.addEventListener("click", (e) => {
    if (!e.target.closest(".more-wrap")) {
      closeMoreMenu();
    }
    // Also close RL export menu
    const rlExMenu = document.getElementById("rl-export-menu");
    if (rlExMenu && !e.target.closest("#rl-export-btn") && !e.target.closest("#rl-export-menu")) {
      rlExMenu.classList.remove("open");
    }
  });

  // ── Overlay helpers ───────────────────────────────────────────────

  function openOverlay(id) {
    closeMoreMenu();
    document.getElementById(id).classList.add("visible");
  }

  function closeOverlay(id) {
    document.getElementById(id).classList.remove("visible");
  }

  // ── Reading List ──────────────────────────────────────────────────

  function loadReadingList() {
    chrome.storage.local.get("biokhoj_reading_list", (data) => {
      readingList = data.biokhoj_reading_list || [];
    });
  }

  function saveReadingListToStorage() {
    chrome.storage.local.set({ biokhoj_reading_list: readingList });
  }

  function isInReadingList(pmid) {
    return readingList.some(p => p.pmid === pmid);
  }

  function toggleSave(paper) {
    if (isInReadingList(paper.pmid)) {
      removeFromReadingList(paper.pmid);
      renderPapers();
      showToast("Removed from reading list");
      return;
    }
    saveToReadingList(paper);
  }

  function saveToReadingList(paper) {
    if (isInReadingList(paper.pmid)) return;
    readingList.unshift({
      ...paper,
      savedAt: Date.now(),
      notes: "",
      tags: []
    });
    saveReadingListToStorage();
    renderPapers();
    showToast("Saved to reading list");
  }

  function removeFromReadingList(pmid) {
    readingList = readingList.filter(p => p.pmid !== pmid);
    saveReadingListToStorage();
    renderReadingList();
    renderPapers();
  }

  function updateReadingListNote(pmid, note) {
    readingList = readingList.map(p => p.pmid === pmid ? { ...p, notes: note } : p);
    saveReadingListToStorage();
  }

  function toggleReadingListTag(pmid, tag) {
    readingList = readingList.map(p => {
      if (p.pmid !== pmid) return p;
      const tags = p.tags || [];
      const idx = tags.indexOf(tag);
      if (idx >= 0) tags.splice(idx, 1);
      else tags.push(tag);
      return { ...p, tags };
    });
    saveReadingListToStorage();
    renderReadingList();
  }

  function getAllReadingListTags() {
    const tags = new Set();
    readingList.forEach(p => (p.tags || []).forEach(t => tags.add(t)));
    return [...tags].sort();
  }

  function renderReadingList() {
    const $list = document.getElementById("rl-list");
    const $empty = document.getElementById("rl-empty");
    const $count = document.getElementById("rl-count");
    const $search = document.getElementById("rl-search");
    const $sort = document.getElementById("rl-sort");
    const $tagFilter = document.getElementById("rl-tag-filter");

    if (!$list) return;

    // Populate tag filter
    const allTags = getAllReadingListTags();
    const currentTagFilter = $tagFilter.value;
    $tagFilter.innerHTML = '<option value="">All tags</option>';
    allTags.forEach(t => {
      const opt = document.createElement("option");
      opt.value = t;
      opt.textContent = t;
      if (t === currentTagFilter) opt.selected = true;
      $tagFilter.appendChild(opt);
    });

    let items = [...readingList];
    $count.textContent = items.length + " saved";

    // Filter
    const query = ($search.value || "").toLowerCase().trim();
    if (query) {
      items = items.filter(p =>
        (p.title || "").toLowerCase().includes(query) ||
        (p.journal || "").toLowerCase().includes(query) ||
        (p.notes || "").toLowerCase().includes(query)
      );
    }
    if (currentTagFilter) {
      items = items.filter(p => (p.tags || []).includes(currentTagFilter));
    }

    // Sort
    const sortBy = $sort.value;
    items.sort((a, b) => {
      if (sortBy === "signal") return (b.signalScore || 0) - (a.signalScore || 0);
      if (sortBy === "title") return (a.title || "").localeCompare(b.title || "");
      return (b.savedAt || 0) - (a.savedAt || 0);
    });

    $list.innerHTML = "";
    $empty.style.display = items.length === 0 ? "block" : "none";

    items.forEach((paper) => {
      const div = document.createElement("div");
      div.className = "rl-item";

      const title = document.createElement("div");
      title.className = "rl-title";
      title.textContent = paper.title;
      title.style.cursor = "pointer";
      title.addEventListener("click", () => openPaperDetail(paper));
      div.appendChild(title);

      const meta = document.createElement("div");
      meta.className = "rl-meta";
      const metaText = [paper.journal, paper.date].filter(Boolean).join(" \u00B7 ");
      meta.textContent = metaText;
      if (paper.signalScore) {
        meta.textContent += " \u00B7 Signal: " + paper.signalScore;
      }

      // Citation badge
      const cite = citationData[paper.pmid];
      if (cite && cite.currentCount !== undefined) {
        const citeBadge = document.createElement("span");
        citeBadge.className = "rl-citation-badge";
        citeBadge.textContent = "\uD83D\uDCC8 " + cite.currentCount + " citations";
        meta.appendChild(citeBadge);

        // Sparkline for citation growth
        if (cite.history && cite.history.length >= 2) {
          const spark = document.createElement("span");
          spark.className = "rl-citation-sparkline";
          spark.textContent = citationSparkline(cite.history);
          spark.title = "Citation history: " + cite.history.map(h => h.count).join(" \u2192 ");
          meta.appendChild(spark);
        }
      }
      div.appendChild(meta);

      // Tags
      const tagsDiv = document.createElement("div");
      tagsDiv.className = "rl-tags";
      const defaultTags = ["important", "review-later", "methods", "results"];
      defaultTags.forEach(tag => {
        const chip = document.createElement("span");
        chip.className = "rl-tag" + ((paper.tags || []).includes(tag) ? " active" : "");
        chip.textContent = tag;
        chip.addEventListener("click", () => toggleReadingListTag(paper.pmid, tag));
        tagsDiv.appendChild(chip);
      });
      div.appendChild(tagsDiv);

      // Notes
      if (paper.notes) {
        const noteText = document.createElement("div");
        noteText.className = "rl-note-text";
        noteText.textContent = paper.notes;
        div.appendChild(noteText);
      }

      // Actions
      const actions = document.createElement("div");
      actions.className = "rl-actions";
      actions.appendChild(makeActionBtn("Edit Note", () => {
        const existing = paper.notes || "";
        const note = prompt("Note for this paper:", existing);
        if (note !== null) {
          updateReadingListNote(paper.pmid, note);
          renderReadingList();
        }
      }));
      actions.appendChild(makeActionBtn("Open", () => window.open(paper.url, "_blank")));
      actions.appendChild(makeActionBtn("Remove", () => removeFromReadingList(paper.pmid)));
      div.appendChild(actions);

      $list.appendChild(div);
    });
  }

  // ── Trends ────────────────────────────────────────────────────────

  function renderTrends() {
    const $list = document.getElementById("trends-list");
    const $empty = document.getElementById("trends-empty");
    if (!$list) return;

    $list.innerHTML = "";

    if (watchlist.length === 0) {
      $empty.style.display = "block";
      return;
    }
    $empty.style.display = "none";

    const now = Date.now();
    const weekMs = 7 * 24 * 60 * 60 * 1000;

    watchlist.forEach((entity) => {
      const entityPapers = papers.filter(p => p.entity === entity.name);
      // Compute weekly counts for last 4 weeks
      const weeks = [0, 0, 0, 0];
      entityPapers.forEach(p => {
        const age = now - (p.fetchedAt || 0);
        const weekIdx = Math.min(3, Math.floor(age / weekMs));
        weeks[3 - weekIdx]++;
      });

      const bars = "\u2581\u2582\u2583\u2584\u2585\u2586\u2587\u2588";
      const maxCount = Math.max(1, ...weeks);
      const sparkline = weeks.map(c => bars[Math.min(bars.length - 1, Math.floor((c / maxCount) * (bars.length - 1)))]).join("");

      // Detect rising trend
      const recent = weeks[3];
      const prev = weeks[2];
      const isRising = recent > prev && recent > 0;

      const row = document.createElement("div");
      row.className = "trend-row";

      const nameSpan = document.createElement("span");
      nameSpan.className = "trend-entity";
      nameSpan.textContent = entity.name;
      row.appendChild(nameSpan);

      const sparkSpan = document.createElement("span");
      sparkSpan.className = "trend-sparkline";
      sparkSpan.textContent = sparkline;
      sparkSpan.title = "Weekly papers: " + weeks.join(", ");
      row.appendChild(sparkSpan);

      const countSpan = document.createElement("span");
      countSpan.className = "trend-count";
      countSpan.textContent = entityPapers.length;
      row.appendChild(countSpan);

      if (isRising) {
        const badge = document.createElement("span");
        badge.className = "trend-badge-rising";
        badge.textContent = "\u2191 Rising";
        row.appendChild(badge);
      }

      $list.appendChild(row);
    });
  }

  // ── Settings Panel ────────────────────────────────────────────────

  function loadSettings() {
    chrome.storage.local.get("biokhoj_settings", (data) => {
      settings = data.biokhoj_settings || {};
      applySettingsToUI();
    });
  }

  function saveSettings() {
    chrome.storage.local.set({ biokhoj_settings: settings });
  }

  function applySettingsToUI() {
    const $apiKey = document.getElementById("setting-api-key");
    const $interval = document.getElementById("setting-interval");
    const $toggleNotif = document.getElementById("toggle-notifications");
    const $toggleThemeSetting = document.getElementById("toggle-theme-setting");
    const $journalTiers = document.getElementById("setting-journal-tiers");

    if ($apiKey) $apiKey.value = settings.ncbiApiKey || "";
    if ($interval) $interval.value = settings.checkIntervalHours || 4;
    if ($toggleNotif) $toggleNotif.classList.toggle("on", settings.notifications !== false);
    if ($toggleThemeSetting) $toggleThemeSetting.classList.toggle("on", settings.theme === "light");
    if ($journalTiers) {
      $journalTiers.value = settings.journalTiers
        ? JSON.stringify(settings.journalTiers, null, 2)
        : JSON.stringify({
            top: ["nature", "science", "cell", "lancet", "nejm", "new england", "jama", "bmj", "pnas"],
            mid: ["plos", "genome", "nucleic acids", "bioinformatics", "molecular", "cancer"]
          }, null, 2);
    }
  }

  function initSettingsListeners() {
    const $apiKey = document.getElementById("setting-api-key");
    const $interval = document.getElementById("setting-interval");
    const $toggleNotif = document.getElementById("toggle-notifications");
    const $toggleThemeSetting = document.getElementById("toggle-theme-setting");
    const $journalTiers = document.getElementById("setting-journal-tiers");
    const $btnExportWl = document.getElementById("btn-export-watchlist");
    const $btnImportWl = document.getElementById("btn-import-watchlist");
    const $importFile = document.getElementById("import-watchlist-file");
    const $btnClearAll = document.getElementById("btn-clear-all-data");

    if ($apiKey) $apiKey.addEventListener("change", () => {
      settings.ncbiApiKey = $apiKey.value.trim();
      saveSettings();
      showToast("API key saved");
    });

    if ($interval) $interval.addEventListener("change", () => {
      settings.checkIntervalHours = parseInt($interval.value) || 4;
      saveSettings();
      showToast("Interval updated");
    });

    if ($toggleNotif) $toggleNotif.addEventListener("click", () => {
      $toggleNotif.classList.toggle("on");
      settings.notifications = $toggleNotif.classList.contains("on");
      saveSettings();
    });

    if ($toggleThemeSetting) $toggleThemeSetting.addEventListener("click", () => {
      $toggleThemeSetting.classList.toggle("on");
      darkTheme = !$toggleThemeSetting.classList.contains("on");
      settings.theme = darkTheme ? "dark" : "light";
      applyTheme();
      saveSettings();
    });

    if ($journalTiers) $journalTiers.addEventListener("change", () => {
      try {
        settings.journalTiers = JSON.parse($journalTiers.value);
        saveSettings();
        showToast("Journal tiers saved");
      } catch (e) {
        showToast("Invalid JSON");
      }
    });

    if ($btnExportWl) $btnExportWl.addEventListener("click", () => {
      const json = JSON.stringify({ watchlist, readingList }, null, 2);
      downloadText("biokhoj-data.json", json, "application/json");
      showToast("Exported");
    });

    if ($btnImportWl) $btnImportWl.addEventListener("click", () => {
      $importFile.click();
    });

    if ($importFile) $importFile.addEventListener("change", (e) => {
      const file = e.target.files[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = (ev) => {
        try {
          const data = JSON.parse(ev.target.result);
          if (data.watchlist) {
            data.watchlist.forEach(entity => {
              sendMsg({ type: "add-watch", entity });
            });
          }
          if (data.readingList) {
            data.readingList.forEach(p => {
              if (!isInReadingList(p.pmid)) readingList.push(p);
            });
            saveReadingListToStorage();
          }
          showToast("Imported successfully");
          refresh();
        } catch (err) {
          showToast("Invalid JSON file");
        }
      };
      reader.readAsText(file);
      $importFile.value = "";
    });

    if ($btnClearAll) $btnClearAll.addEventListener("click", () => {
      if (confirm("Clear ALL BioKhoj data? This cannot be undone.")) {
        chrome.storage.local.remove([
          "biokhoj_watchlist", "biokhoj_papers",
          "biokhoj_reading_list", "biokhoj_settings",
          "biokhoj_citations"
        ], () => {
          watchlist = [];
          papers = [];
          readingList = [];
          settings = {};
          citationData = {};
          renderWatchlist();
          renderPapers();
          updateEmptyState();
          applySettingsToUI();
          showToast("All data cleared");
        });
      }
    });
  }

  // ── Paper Detail ──────────────────────────────────────────────────

  function openPaperDetail(paper) {
    currentDetailPaper = paper;
    const panel = document.getElementById("paper-detail-panel");

    document.getElementById("pd-title").textContent = truncate(paper.title, 40);
    document.getElementById("pd-full-title").textContent = paper.title || "Untitled";
    document.getElementById("pd-authors").textContent = paper.authors || "Unknown";
    document.getElementById("pd-journal-date").textContent =
      [paper.journal, paper.date].filter(Boolean).join(" \u2014 ");

    // Study type badge
    const studyBadge = document.getElementById("pd-study-badge");
    const studyType = detectStudyType(paper);
    studyBadge.className = "study-badge study-" + studyType.cls;
    studyBadge.textContent = studyType.label;

    // Clinical indicators
    const clinicalBadges = document.getElementById("pd-clinical-badges");
    clinicalBadges.innerHTML = "";
    const indicators = detectClinicalIndicators(paper);
    indicators.forEach(ind => {
      const badge = document.createElement("span");
      badge.className = "clinical-badge clinical-" + ind.cls;
      badge.textContent = ind.label;
      clinicalBadges.appendChild(badge);
    });

    // Signal Score breakdown
    const scoreGrid = document.getElementById("pd-score-grid");
    scoreGrid.innerHTML = "";
    const breakdown = computeSignalBreakdown(paper);
    Object.entries(breakdown).forEach(([key, val]) => {
      const item = document.createElement("div");
      item.className = "pd-score-item";
      item.innerHTML = '<div class="pd-score-name">' + key + '</div><div class="pd-score-val">' + val + '</div>';
      scoreGrid.appendChild(item);
    });

    // Matched entities
    const entitiesDiv = document.getElementById("pd-entities");
    entitiesDiv.innerHTML = "";
    const matchedEntities = getMatchedEntities(paper);
    matchedEntities.forEach(ent => {
      const chip = document.createElement("span");
      chip.className = "pd-chip chip-" + (ent.type || "gene");
      chip.textContent = ent.name;
      chip.addEventListener("click", () => {
        addEntity(ent.name);
        showToast("Watching " + ent.name);
      });
      entitiesDiv.appendChild(chip);
    });

    // Abstract (if available)
    const abstractField = document.getElementById("pd-abstract-field");
    const abstractDiv = document.getElementById("pd-abstract");
    if (paper.abstract) {
      abstractField.style.display = "block";
      abstractDiv.textContent = paper.abstract;
    } else {
      abstractField.style.display = "none";
    }

    // Context snapshot
    const contextField = document.getElementById("pd-context-field");
    const contextDiv = document.getElementById("pd-context");
    if (paper.contextSnapshot) {
      contextField.style.display = "block";
      contextDiv.textContent = paper.contextSnapshot;
    } else {
      contextField.style.display = "none";
    }

    panel.classList.add("visible");
  }

  function detectStudyType(paper) {
    const title = (paper.title || "").toLowerCase();
    if (/meta-analysis|meta analysis/.test(title)) return { label: "Meta-analysis", cls: "meta" };
    if (/systematic review/.test(title)) return { label: "Systematic Review", cls: "review" };
    if (/randomized|rct|clinical trial|phase [i-iv]/.test(title)) return { label: "Clinical Trial", cls: "rct" };
    if (/cohort|longitudinal|prospective|retrospective/.test(title)) return { label: "Cohort Study", cls: "cohort" };
    if (/case.?control|case report|case series/.test(title)) return { label: "Case Study", cls: "case" };
    if (/review/.test(title)) return { label: "Review", cls: "review" };
    return { label: "Research", cls: "default" };
  }

  function detectClinicalIndicators(paper) {
    const title = (paper.title || "").toLowerCase();
    const indicators = [];
    if (/fda|approved|clearance/.test(title)) indicators.push({ label: "FDA", cls: "fda" });
    if (/clinical trial|phase [i-iv]|nct\d/.test(title)) indicators.push({ label: "Trial", cls: "trial" });
    if (/off-label|compassionate/.test(title)) indicators.push({ label: "Off-label", cls: "offlabel" });
    return indicators;
  }

  function computeSignalBreakdown(paper) {
    const title = (paper.title || "").toLowerCase();
    const journal = (paper.journal || "").toLowerCase();
    const tiers = (settings.journalTiers) || {
      top: ["nature", "science", "cell", "lancet", "nejm", "new england", "jama", "bmj", "pnas"],
      mid: ["plos", "genome", "nucleic acids", "bioinformatics", "molecular", "cancer"]
    };

    let journalScore = 0;
    if (tiers.top && tiers.top.some(j => journal.includes(j))) journalScore = 20;
    else if (tiers.mid && tiers.mid.some(j => journal.includes(j))) journalScore = 10;

    const thisYear = new Date().getFullYear().toString();
    const recencyScore = (paper.date || "").includes(thisYear) ? 10 : 0;

    const hotWords = ["crispr", "single-cell", "gwas", "novel", "therapeutic", "clinical trial", "phase", "breakthrough"];
    const topicScore = hotWords.some(w => title.includes(w)) ? 10 : 0;

    const base = 50;
    return {
      "Base": base,
      "Journal": "+" + journalScore,
      "Recency": "+" + recencyScore,
      "Topic": "+" + topicScore,
      "Total": Math.min(100, base + journalScore + recencyScore + topicScore)
    };
  }

  function getMatchedEntities(paper) {
    const matched = [];
    const entity = paper.entity;
    if (entity) {
      const wl = watchlist.find(w => w.name === entity);
      matched.push({ name: entity, type: wl ? wl.type : "gene" });
    }
    // Check if any other watchlist entities appear in the title
    watchlist.forEach(w => {
      if (w.name !== entity && (paper.title || "").toLowerCase().includes(w.name.toLowerCase())) {
        matched.push({ name: w.name, type: w.type });
      }
    });
    return matched;
  }

  function initPaperDetailListeners() {
    document.getElementById("btn-close-paper-detail").addEventListener("click", () => {
      closeOverlay("paper-detail-panel");
    });

    document.getElementById("pd-save").addEventListener("click", () => {
      if (currentDetailPaper) {
        saveToReadingList(currentDetailPaper);
      }
    });

    document.getElementById("pd-cite").addEventListener("click", () => {
      if (currentDetailPaper) {
        navigator.clipboard.writeText(formatCitation(currentDetailPaper)).then(() => showToast("Citation copied"));
      }
    });

    document.getElementById("pd-watch-entities").addEventListener("click", () => {
      if (currentDetailPaper) {
        const entities = getMatchedEntities(currentDetailPaper);
        entities.forEach(ent => {
          sendMsg({ type: "add-watch", entity: ent });
        });
        showToast("Entities added to watchlist");
        refresh();
      }
    });

    document.getElementById("pd-open-link").addEventListener("click", () => {
      if (currentDetailPaper) {
        window.open(currentDetailPaper.url, "_blank");
        trackHistory(currentDetailPaper);
        if (!currentDetailPaper.read) markRead(currentDetailPaper.pmid);
      }
    });
  }

  // ── Export ────────────────────────────────────────────────────────

  function getExportPapers() {
    const scope = document.querySelector('input[name="export-scope"]:checked').value;
    if (scope === "reading") return readingList;
    if (scope === "watchlist") return watchlist.map(w => ({ entity: w.name, type: w.type }));
    return papers;
  }

  function formatBibTeX(list) {
    return list.map((p, i) => {
      if (!p.title) return ""; // watchlist item
      const id = p.pmid || ("paper" + i);
      return "@article{" + id + ",\n" +
        "  title = {" + (p.title || "") + "},\n" +
        "  author = {" + (p.authors || "") + "},\n" +
        "  journal = {" + (p.journal || "") + "},\n" +
        "  year = {" + ((p.date || "").split(" ")[0] || "") + "},\n" +
        "  pmid = {" + (p.pmid || "") + "}\n" +
        "}";
    }).filter(Boolean).join("\n\n");
  }

  function formatRIS(list) {
    return list.map(p => {
      if (!p.title) return "";
      return "TY  - JOUR\n" +
        "TI  - " + (p.title || "") + "\n" +
        "AU  - " + (p.authors || "") + "\n" +
        "JO  - " + (p.journal || "") + "\n" +
        "PY  - " + ((p.date || "").split(" ")[0] || "") + "\n" +
        "AN  - PMID:" + (p.pmid || "") + "\n" +
        "UR  - " + (p.url || "") + "\n" +
        "ER  - ";
    }).filter(Boolean).join("\n\n");
  }

  function formatMarkdownExport(list) {
    return list.map((p, i) => {
      if (p.type) return "- **" + p.name + "** (" + p.type + ")"; // watchlist entity
      return (i + 1) + ". **" + (p.title || "Untitled") + "**\n" +
        "   - " + [p.authors, p.journal, p.date].filter(Boolean).join(", ") + "\n" +
        "   - PMID: " + (p.pmid || "N/A") + " | [Link](" + (p.url || "#") + ")" +
        (p.notes ? "\n   - Note: " + p.notes : "");
    }).join("\n\n");
  }

  function formatJSONExport(list) {
    return JSON.stringify(list, null, 2);
  }

  function updateExportPreview() {
    const $preview = document.getElementById("export-preview");
    const list = getExportPapers();
    const format = document.querySelector('input[name="export-format"]:checked').value;

    let output = "";
    if (format === "bibtex") output = formatBibTeX(list);
    else if (format === "ris") output = formatRIS(list);
    else if (format === "markdown") output = formatMarkdownExport(list);
    else output = formatJSONExport(list);

    $preview.textContent = output || "(no data to export)";
  }

  function initExportListeners() {
    document.querySelectorAll('input[name="export-scope"], input[name="export-format"]').forEach(el => {
      el.addEventListener("change", updateExportPreview);
    });

    document.getElementById("btn-export-copy").addEventListener("click", () => {
      const text = document.getElementById("export-preview").textContent;
      navigator.clipboard.writeText(text).then(() => showToast("Copied to clipboard"));
    });

    document.getElementById("btn-export-download").addEventListener("click", () => {
      const text = document.getElementById("export-preview").textContent;
      const format = document.querySelector('input[name="export-format"]:checked').value;
      const ext = { bibtex: "bib", ris: "ris", markdown: "md", json: "json" }[format] || "txt";
      downloadText("biokhoj-export." + ext, text);
    });
  }

  function downloadText(filename, text, mime) {
    const blob = new Blob([text], { type: mime || "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  // ── Weekly Digest ─────────────────────────────────────────────────

  function generateWeeklyDigest() {
    const now = Date.now();
    const weekMs = 7 * 24 * 60 * 60 * 1000;
    const weekPapers = papers.filter(p => (p.fetchedAt || 0) > (now - weekMs));

    if (weekPapers.length === 0) {
      showToast("No papers from this week");
      return;
    }

    const byEntity = {};
    weekPapers.forEach(p => {
      const ent = p.entity || "Other";
      if (!byEntity[ent]) byEntity[ent] = [];
      byEntity[ent].push(p);
    });

    let md = "# BioKhoj Weekly Digest\n";
    md += "_" + new Date().toLocaleDateString() + "_\n\n";
    md += "**" + weekPapers.length + " papers** across **" + Object.keys(byEntity).length + " entities**\n\n";

    const highPriority = weekPapers.filter(p => (p.signalScore || 0) >= 70);
    if (highPriority.length > 0) {
      md += "## High-Signal Papers\n\n";
      highPriority.forEach(p => {
        md += "- **" + p.title + "** (" + p.journal + ", Signal: " + p.signalScore + ")\n";
      });
      md += "\n";
    }

    Object.entries(byEntity).sort((a, b) => b[1].length - a[1].length).forEach(([entity, entityPapers]) => {
      md += "## " + entity + " (" + entityPapers.length + " papers)\n\n";
      entityPapers.slice(0, 5).forEach(p => {
        md += "- " + p.title + " \u2014 _" + (p.journal || "Unknown") + "_ " + (p.date || "") + "\n";
      });
      md += "\n";
    });

    navigator.clipboard.writeText(md).then(() => showToast("Weekly digest copied to clipboard"));
  }

  // ── Reading List Export ───────────────────────────────────────────

  function initRLExportListeners() {
    const $btn = document.getElementById("rl-export-btn");
    const $menu = document.getElementById("rl-export-menu");
    if (!$btn || !$menu) return;

    $btn.addEventListener("click", (e) => {
      e.stopPropagation();
      $menu.classList.toggle("open");
    });

    $menu.querySelectorAll(".more-menu-item").forEach(item => {
      item.addEventListener("click", () => {
        const format = item.dataset.format;
        let text = "";
        if (format === "bibtex") text = formatBibTeX(readingList);
        else if (format === "ris") text = formatRIS(readingList);
        else text = formatMarkdownExport(readingList);
        navigator.clipboard.writeText(text).then(() => showToast(format.toUpperCase() + " copied"));
        $menu.classList.remove("open");
      });
    });

    // Search, sort, tag filter for reading list
    const $search = document.getElementById("rl-search");
    const $sort = document.getElementById("rl-sort");
    const $tagFilter = document.getElementById("rl-tag-filter");
    if ($search) $search.addEventListener("input", renderReadingList);
    if ($sort) $sort.addEventListener("change", renderReadingList);
    if ($tagFilter) $tagFilter.addEventListener("change", renderReadingList);
  }

  // ── Digest Panel Listeners ──────────────────────────────────────

  function initDigestPanelListeners() {
    const $copy = document.getElementById("btn-digest-copy");
    const $slides = document.getElementById("btn-digest-slides");

    if ($copy) $copy.addEventListener("click", () => {
      const text = document.getElementById("digest-preview").textContent;
      navigator.clipboard.writeText(text).then(() => showToast("Digest copied"));
    });

    if ($slides) $slides.addEventListener("click", () => {
      const slides = generateSlidesFormat();
      if (!slides) {
        showToast("No papers from this week");
        return;
      }
      navigator.clipboard.writeText(slides).then(() => showToast("Slides format copied"));
    });
  }

  // ── Citation Tracker ──────────────────────────────────────────────

  function loadCitations() {
    sendMsg({ type: "get-citations" }).then(resp => {
      citationData = resp.citations || {};
    });
  }

  function citationSparkline(history) {
    if (!history || history.length < 2) return "";
    const bars = "\u2581\u2582\u2583\u2585";
    const counts = history.map(h => h.count);
    const max = Math.max(1, ...counts);
    return counts.map(c => bars[Math.min(bars.length - 1, Math.floor((c / max) * (bars.length - 1)))]).join("");
  }

  // ── Journal Club Picker ────────────────────────────────────────────

  function getJournalClubCandidates() {
    const now = Date.now();
    const weekMs = 7 * 24 * 60 * 60 * 1000;
    return papers
      .filter(p => (p.fetchedAt || 0) > (now - weekMs))
      .sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0));
  }

  function renderJournalClubPick() {
    const $container = document.getElementById("jc-card-container");
    const $empty = document.getElementById("jc-empty");
    if (!$container) return;

    const candidates = getJournalClubCandidates();
    $container.innerHTML = "";

    if (candidates.length === 0) {
      $empty.style.display = "block";
      return;
    }
    $empty.style.display = "none";

    // Clamp index
    if (journalClubIndex >= candidates.length) journalClubIndex = 0;
    const paper = candidates[journalClubIndex];

    const card = document.createElement("div");
    card.className = "jc-card";

    const title = document.createElement("div");
    title.className = "jc-title";
    title.textContent = paper.title || "Untitled";
    card.appendChild(title);

    const journal = document.createElement("div");
    journal.className = "jc-journal";
    journal.textContent = [paper.journal, paper.date].filter(Boolean).join(" \u2014 ");
    card.appendChild(journal);

    const signal = document.createElement("div");
    signal.className = "jc-signal";
    signal.textContent = "\u2B50 Signal Score: " + (paper.signalScore || 0);
    card.appendChild(signal);

    // Why it's the pick
    const matched = getMatchedEntities(paper);
    if (matched.length > 0) {
      const reason = document.createElement("div");
      reason.className = "jc-reason";
      reason.textContent = "Top scored for: " + matched.map(e => e.name).join(", ");
      card.appendChild(reason);
    }

    // Abstract preview
    if (paper.abstract) {
      const abs = document.createElement("div");
      abs.className = "jc-abstract";
      abs.textContent = paper.abstract;
      card.appendChild(abs);
    }

    // Pick index indicator
    const indicator = document.createElement("div");
    indicator.style.cssText = "font-size:10px;color:#64748b;margin-bottom:8px";
    indicator.textContent = "Pick " + (journalClubIndex + 1) + " of " + candidates.length;
    card.appendChild(indicator);

    // Actions
    const actions = document.createElement("div");
    actions.className = "jc-actions";

    const slackBtn = document.createElement("button");
    slackBtn.className = "btn btn-saffron";
    slackBtn.style.fontSize = "11px";
    slackBtn.textContent = "Copy for Slack";
    slackBtn.addEventListener("click", () => {
      const text = "\uD83D\uDCF0 Journal Club Pick: " + paper.title +
        " \u2014 " + (paper.journal || "Unknown") +
        " (Signal: " + (paper.signalScore || 0) + ") " +
        (paper.url || "");
      navigator.clipboard.writeText(text).then(() => showToast("Copied for Slack"));
    });
    actions.appendChild(slackBtn);

    const pickBtn = document.createElement("button");
    pickBtn.className = "btn";
    pickBtn.style.fontSize = "11px";
    pickBtn.textContent = "Pick Another";
    pickBtn.addEventListener("click", () => {
      journalClubIndex++;
      renderJournalClubPick();
    });
    actions.appendChild(pickBtn);

    const openBtn = document.createElement("button");
    openBtn.className = "btn";
    openBtn.style.fontSize = "11px";
    openBtn.textContent = "Open";
    openBtn.addEventListener("click", () => {
      window.open(paper.url, "_blank");
    });
    actions.appendChild(openBtn);

    card.appendChild(actions);
    $container.appendChild(card);
  }

  // ── Collaboration Finder ───────────────────────────────────────────

  function renderCollaborators() {
    const $list = document.getElementById("collab-list");
    const $empty = document.getElementById("collab-empty");
    if (!$list) return;
    $list.innerHTML = "";

    // Gather all papers that match watched entities
    const entityNames = new Set(watchlist.map(w => w.name.toLowerCase()));
    const matchedPapers = papers.filter(p => {
      if (p.entity && entityNames.has(p.entity.toLowerCase())) return true;
      const title = (p.title || "").toLowerCase();
      return [...entityNames].some(e => title.includes(e));
    });

    if (matchedPapers.length === 0 || !matchedPapers.some(p => p.authors)) {
      $empty.style.display = "block";
      return;
    }
    $empty.style.display = "none";

    // Count author frequency
    const authorMap = {};
    matchedPapers.forEach(p => {
      const authorStr = p.authors || "";
      const authors = authorStr.split(/,\s*/).map(a => a.trim()).filter(Boolean);
      authors.forEach(author => {
        const key = author.toLowerCase();
        if (!authorMap[key]) {
          authorMap[key] = { name: author, count: 0, papers: [] };
        }
        authorMap[key].count++;
        authorMap[key].papers.push(p);
      });
    });

    // Sort by count desc, take top 10
    const topAuthors = Object.values(authorMap)
      .sort((a, b) => b.count - a.count)
      .slice(0, 10);

    topAuthors.forEach((author, i) => {
      const row = document.createElement("div");
      row.className = "collab-row";

      const rank = document.createElement("span");
      rank.className = "collab-rank";
      rank.textContent = (i + 1);
      row.appendChild(rank);

      const info = document.createElement("div");
      info.className = "collab-info";

      const name = document.createElement("div");
      name.className = "collab-name";
      name.textContent = author.name;
      info.appendChild(name);

      // Most recent paper title
      const recentPaper = author.papers.sort((a, b) => (b.fetchedAt || 0) - (a.fetchedAt || 0))[0];
      if (recentPaper) {
        const detail = document.createElement("div");
        detail.className = "collab-detail";
        detail.textContent = truncate(recentPaper.title, 50);
        detail.title = recentPaper.title;
        info.appendChild(detail);
      }

      row.appendChild(info);

      const count = document.createElement("span");
      count.className = "collab-count";
      count.textContent = author.count + " papers";
      row.appendChild(count);

      const actions = document.createElement("div");
      actions.className = "collab-actions";

      const watchBtn = document.createElement("button");
      watchBtn.className = "paper-action";
      watchBtn.textContent = "Watch";
      watchBtn.title = "Watch this author";
      watchBtn.addEventListener("click", () => {
        sendMsg({ type: "add-watch", entity: { name: author.name, type: "author", priority: 5 } }).then(resp => {
          if (resp.ok) {
            showToast("Watching " + author.name);
            refresh();
          } else if (resp.reason === "duplicate") {
            showToast("Already watching " + author.name);
          }
        });
      });
      actions.appendChild(watchBtn);

      const searchBtn = document.createElement("button");
      searchBtn.className = "paper-action";
      searchBtn.textContent = "PubMed";
      searchBtn.title = "Search on PubMed";
      searchBtn.addEventListener("click", () => {
        window.open("https://pubmed.ncbi.nlm.nih.gov/?term=" + encodeURIComponent(author.name + "[author]"), "_blank");
      });
      actions.appendChild(searchBtn);

      row.appendChild(actions);
      $list.appendChild(row);
    });
  }

  // ── Enhanced Weekly Digest (with slides) ───────────────────────────

  function generateDigestContent() {
    const now = Date.now();
    const weekMs = 7 * 24 * 60 * 60 * 1000;
    const weekPapers = papers.filter(p => (p.fetchedAt || 0) > (now - weekMs));
    return weekPapers;
  }

  function generateDigestMarkdown() {
    const weekPapers = generateDigestContent();
    if (weekPapers.length === 0) return null;

    const byEntity = {};
    weekPapers.forEach(p => {
      const ent = p.entity || "Other";
      if (!byEntity[ent]) byEntity[ent] = [];
      byEntity[ent].push(p);
    });

    let md = "# BioKhoj Weekly Digest\n";
    md += "_" + new Date().toLocaleDateString() + "_\n\n";
    md += "**" + weekPapers.length + " papers** across **" + Object.keys(byEntity).length + " entities**\n\n";

    const highPriority = weekPapers.filter(p => (p.signalScore || 0) >= 70);
    if (highPriority.length > 0) {
      md += "## High-Signal Papers\n\n";
      highPriority.forEach(p => {
        md += "- **" + p.title + "** (" + p.journal + ", Signal: " + p.signalScore + ")\n";
      });
      md += "\n";
    }

    Object.entries(byEntity).sort((a, b) => b[1].length - a[1].length).forEach(([entity, entityPapers]) => {
      md += "## " + entity + " (" + entityPapers.length + " papers)\n\n";
      entityPapers.slice(0, 5).forEach(p => {
        md += "- " + p.title + " \u2014 _" + (p.journal || "Unknown") + "_ " + (p.date || "") + "\n";
      });
      md += "\n";
    });

    return md;
  }

  function generateSlidesFormat() {
    const weekPapers = generateDigestContent();
    if (weekPapers.length === 0) return null;

    const now = new Date();
    const weekStart = new Date(now);
    weekStart.setDate(weekStart.getDate() - weekStart.getDay());
    const dateStr = weekStart.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });

    let slides = "\uD83D\uDD2C Lab Meeting \u2014 Week of " + dateStr + "\n\n";

    // Top papers
    const topPapers = [...weekPapers].sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0)).slice(0, 5);
    slides += "TOP PAPERS\n";
    topPapers.forEach(p => {
      const matched = getMatchedEntities(p);
      const keyEntities = matched.map(e => e.name).join(", ");
      slides += "\u2022 " + p.title + " \u2014 " + (p.journal || "Unknown") + " \u2B50 Signal: " + (p.signalScore || 0) + "\n";
      if (keyEntities) {
        slides += "  Key: " + keyEntities + " mentioned\n";
      }
      slides += "\n";
    });

    // Trending entities
    const weekMs = 7 * 24 * 60 * 60 * 1000;
    const twoWeekMs = 14 * 24 * 60 * 60 * 1000;
    const nowTs = Date.now();

    const entityCounts = {};
    const entityCountsPrev = {};
    weekPapers.forEach(p => {
      const ent = p.entity || "Other";
      entityCounts[ent] = (entityCounts[ent] || 0) + 1;
    });
    papers.filter(p => {
      const ft = p.fetchedAt || 0;
      return ft > (nowTs - twoWeekMs) && ft <= (nowTs - weekMs);
    }).forEach(p => {
      const ent = p.entity || "Other";
      entityCountsPrev[ent] = (entityCountsPrev[ent] || 0) + 1;
    });

    const trending = Object.entries(entityCounts)
      .filter(([ent, cnt]) => cnt > (entityCountsPrev[ent] || 0))
      .sort((a, b) => b[1] - a[1]);

    if (trending.length > 0) {
      slides += "TRENDING\n";
      trending.slice(0, 5).forEach(([ent, cnt]) => {
        const prev = entityCountsPrev[ent] || 0;
        slides += "\u2022 " + ent + ": " + cnt + " new papers (\u2191 from " + prev + " last week)\n";
      });
      slides += "\n";
    }

    // Co-mentions: papers matching multiple entities
    const coMentions = weekPapers.filter(p => {
      const matched = getMatchedEntities(p);
      return matched.length >= 2;
    });
    if (coMentions.length > 0) {
      slides += "NEW CO-MENTIONS\n";
      coMentions.slice(0, 3).forEach(p => {
        const matched = getMatchedEntities(p);
        slides += "\u2022 Paper linking " + matched.map(e => e.name).join(" + ") + "\n";
      });
    }

    return slides;
  }

  function openDigestPanel() {
    const md = generateDigestMarkdown();
    if (!md) {
      showToast("No papers from this week");
      return;
    }
    const $preview = document.getElementById("digest-preview");
    $preview.textContent = md;
    openOverlay("digest-panel");
  }

  // ── Event Listeners ────────────────────────────────────────────────

  $btnAdd.addEventListener("click", () => addEntity());
  $addInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") addEntity();
  });

  $btnRefresh.addEventListener("click", checkNow);

  // Pop-out: open sidebar as full page tab
  const $btnPopout = document.getElementById("btn-popout");
  if ($btnPopout) {
    $btnPopout.addEventListener("click", () => {
      if (chrome.tabs) {
        chrome.tabs.create({ url: chrome.runtime.getURL("sidebar.html") });
      } else {
        window.open(location.href, "_blank");
      }
    });
  }

  if ($btnOpenPwa) $btnOpenPwa.addEventListener("click", () => { sendMsg({ type: "open-pwa" }); });
  if ($btnOpenBk) $btnOpenBk.addEventListener("click", () => { sendMsg({ type: "open-pwa" }); });

  $btnTheme.addEventListener("click", toggleTheme);

  // More menu
  $btnMore.addEventListener("click", (e) => {
    e.stopPropagation();
    toggleMoreMenu();
  });

  document.getElementById("menu-trends").addEventListener("click", () => {
    renderTrends();
    openOverlay("trends-panel");
  });
  document.getElementById("menu-reading-list").addEventListener("click", () => {
    renderReadingList();
    openOverlay("reading-list-panel");
  });
  document.getElementById("menu-export").addEventListener("click", () => {
    updateExportPreview();
    openOverlay("export-panel");
  });
  document.getElementById("menu-digest").addEventListener("click", () => {
    openDigestPanel();
  });
  document.getElementById("menu-journal-club").addEventListener("click", () => {
    journalClubIndex = 0;
    renderJournalClubPick();
    openOverlay("journal-club-panel");
  });
  document.getElementById("menu-collaborators").addEventListener("click", () => {
    renderCollaborators();
    openOverlay("collaborators-panel");
  });
  document.getElementById("menu-settings").addEventListener("click", () => {
    applySettingsToUI();
    openOverlay("settings-panel");
  });
  document.getElementById("menu-help").addEventListener("click", () => {
    chrome.tabs.create({ url: chrome.runtime.getURL("help.html") });
  });

  // Settings help link
  var settingsHelpLink = document.getElementById("settings-help-link");
  if (settingsHelpLink) settingsHelpLink.addEventListener("click", (e) => {
    e.preventDefault();
    chrome.tabs.create({ url: chrome.runtime.getURL("help.html") });
  });

  // Close overlay buttons
  document.getElementById("btn-close-trends").addEventListener("click", () => closeOverlay("trends-panel"));
  document.getElementById("btn-close-reading-list").addEventListener("click", () => closeOverlay("reading-list-panel"));
  document.getElementById("btn-close-settings").addEventListener("click", () => closeOverlay("settings-panel"));
  document.getElementById("btn-close-export").addEventListener("click", () => closeOverlay("export-panel"));
  document.getElementById("btn-close-journal-club").addEventListener("click", () => closeOverlay("journal-club-panel"));
  document.getElementById("btn-close-collaborators").addEventListener("click", () => closeOverlay("collaborators-panel"));
  document.getElementById("btn-close-digest").addEventListener("click", () => closeOverlay("digest-panel"));

  // Paper search/sort
  if ($paperSearch) $paperSearch.addEventListener("input", renderPapers);
  if ($paperSort) $paperSort.addEventListener("change", renderPapers);

  // Initialize sub-feature listeners
  initSettingsListeners();
  initPaperDetailListeners();
  initExportListeners();
  initRLExportListeners();
  initDigestPanelListeners();

  // --- Inline tool buttons + responsive collapse ---
  const toolButtons = [
    { inline: document.getElementById("btn-trends-inline"), menu: document.getElementById("menu-trends"), action: () => { renderTrends(); openOverlay("trends-panel"); } },
    { inline: document.getElementById("btn-reading-inline"), menu: document.getElementById("menu-reading-list"), action: () => { renderReadingList(); openOverlay("reading-list-panel"); } },
    { inline: document.getElementById("btn-export-inline"), menu: document.getElementById("menu-export"), action: () => { updateExportPreview(); openOverlay("export-panel"); } },
    { inline: document.getElementById("btn-digest-inline"), menu: document.getElementById("menu-digest"), action: () => { openDigestPanel(); } },
  ];

  // Wire inline buttons
  toolButtons.forEach(tb => {
    if (tb.inline) tb.inline.addEventListener("click", tb.action);
  });

  // Settings inline button
  const btnSettingsInline = document.getElementById("btn-settings-inline");
  if (btnSettingsInline) btnSettingsInline.addEventListener("click", () => { applySettingsToUI(); openOverlay("settings-panel"); });

  // Adaptive layout: hide overflowing inline buttons into More menu
  function adaptHeader() {
    toolButtons.forEach(tb => { if (tb.inline) tb.inline.style.display = ""; if (tb.menu) tb.menu.style.display = "none"; });
    $btnMore.style.display = "none";
    // Also keep settings/help always in menu
    var menuSettings = document.getElementById("menu-settings");
    var menuHelp = document.getElementById("menu-help");
    if (menuSettings) menuSettings.style.display = "";
    if (menuHelp) menuHelp.style.display = "";

    var header = document.querySelector(".header-actions");
    var maxWidth = header.parentElement.clientWidth - document.querySelector(".header-left").offsetWidth - 8;

    if (header.scrollWidth <= maxWidth) return;

    var overflowed = false;
    for (var i = toolButtons.length - 1; i >= 0; i--) {
      if (header.scrollWidth > maxWidth || overflowed) {
        if (toolButtons[i].inline) toolButtons[i].inline.style.display = "none";
        if (toolButtons[i].menu) toolButtons[i].menu.style.display = "";
        $btnMore.style.display = "";
        overflowed = true;
      }
    }
  }

  adaptHeader();
  new ResizeObserver(adaptHeader).observe(document.querySelector(".header"));

  // Auto-refresh when sidebar becomes visible
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) refresh();
  });

  // ── Onboarding chip clicks ────────────────────────────────────────

  document.querySelectorAll(".onboarding-chip").forEach(chip => {
    chip.addEventListener("click", () => {
      const entity = chip.getAttribute("data-entity");
      if (entity) addEntity(entity);
    });
  });

  // ── Keyboard Navigation ───────────────────────────────────────────

  function updatePaperSelection(newIndex) {
    const items = $paperList.querySelectorAll(".paper-item");
    if (items.length === 0) return;

    // Remove old selection
    items.forEach(item => item.classList.remove("paper-selected"));

    // Clamp index
    if (newIndex < 0) newIndex = 0;
    if (newIndex >= items.length) newIndex = items.length - 1;
    selectedPaperIndex = newIndex;

    // Apply new selection
    const selected = items[selectedPaperIndex];
    if (selected) {
      selected.classList.add("paper-selected");
      selected.scrollIntoView({ block: "nearest", behavior: "smooth" });
    }
  }

  function getSelectedPaper() {
    const filtered = getFilteredSortedPapers();
    const shown = filtered.slice(0, MAX_PAPERS_SHOWN);
    return shown[selectedPaperIndex] || null;
  }

  function isAnyOverlayOpen() {
    return document.querySelector(".overlay-panel.visible") !== null;
  }

  document.addEventListener("keydown", (e) => {
    const tag = (e.target.tagName || "").toLowerCase();
    const isInput = (tag === "input" || tag === "textarea" || tag === "select");

    // Tab switching: 1-4 keys (when not in input)
    if (!isInput && e.key >= "1" && e.key <= "4") {
      const tabs = ["recent", "watch", "search", "trends"];
      switchSidebarTab(tabs[parseInt(e.key) - 1]);
      e.preventDefault();
      return;
    }

    // Escape closes any open overlay
    if (e.key === "Escape") {
      const openOverlayEl = document.querySelector(".overlay-panel.visible");
      if (openOverlayEl) {
        openOverlayEl.classList.remove("visible");
        e.preventDefault();
        return;
      }
      // Clear paper selection
      if (selectedPaperIndex >= 0) {
        selectedPaperIndex = -1;
        $paperList.querySelectorAll(".paper-item").forEach(item => item.classList.remove("paper-selected"));
        e.preventDefault();
        return;
      }
    }

    // Don't handle arrow/enter keys when typing in inputs
    if (isInput) return;

    // Don't handle navigation when an overlay is open
    if (isAnyOverlayOpen()) return;

    if (e.key === "ArrowDown" || e.key === "j") {
      e.preventDefault();
      updatePaperSelection(selectedPaperIndex + 1);
    } else if (e.key === "ArrowUp" || e.key === "k") {
      e.preventDefault();
      updatePaperSelection(selectedPaperIndex - 1);
    } else if (e.key === "Enter" && selectedPaperIndex >= 0) {
      e.preventDefault();
      const paper = getSelectedPaper();
      if (paper) openPaperDetail(paper);
    }
  });

  // Tab key cycles focus between main sections
  $addInput.addEventListener("keydown", (e) => {
    if (e.key === "Tab" && !e.shiftKey) {
      const firstPaper = $paperList.querySelector(".paper-item");
      if (firstPaper) {
        e.preventDefault();
        $addInput.blur();
        updatePaperSelection(0);
      }
    }
  });

  // ── Tab Switching ────────────────────────────────────────────────

  let activeTabName = "recent";

  function switchSidebarTab(tabName) {
    activeTabName = tabName;
    // Update tab buttons
    document.querySelectorAll(".sidebar-tab").forEach(btn => {
      btn.classList.toggle("active", btn.dataset.tab === tabName);
    });
    // Show/hide panels
    document.querySelectorAll(".tab-panel").forEach(panel => {
      panel.classList.toggle("active", panel.id === "panel-" + tabName);
      panel.style.display = panel.id === "panel-" + tabName ? "" : "none";
    });
    // Remember last tab
    try { chrome.storage.local.set({ biokhoj_last_tab: tabName }); } catch (e) {}
  }

  // Bind tab clicks
  document.querySelectorAll(".sidebar-tab").forEach(btn => {
    btn.addEventListener("click", () => switchSidebarTab(btn.dataset.tab));
  });

  // Restore last tab
  try {
    chrome.storage.local.get("biokhoj_last_tab", (data) => {
      if (data.biokhoj_last_tab) switchSidebarTab(data.biokhoj_last_tab);
    });
  } catch (e) {}

  // Smart default: switch to Watch after right-click add
  chrome.runtime.onMessage.addListener((msg) => {
    if (msg.type === "watchlist-updated") {
      refresh();
      // If sidebar just opened via right-click watch, show Watch tab
      if (activeTabName === "recent") {
        chrome.storage.session.get("biokhoj_lookup", (data) => {
          if (data && data.biokhoj_lookup) switchSidebarTab("watch");
        });
      }
    }
  });

  // ── Discover / Trending refs ──────────────────────────────────────

  const $trendingList     = document.getElementById("trending-list");
  const $trendingEmpty    = document.getElementById("trending-empty");
  const $btnRefreshTrend  = document.getElementById("btn-refresh-trending");
  const $discoverInput    = document.getElementById("discover-search-input");
  const $btnDiscoverSearch = document.getElementById("btn-discover-search");
  const $discoverResults  = document.getElementById("discover-results");

  // ── Trending bioRxiv ──────────────────────────────────────────────

  function fetchAllTrending(forceRefresh) {
    chrome.runtime.sendMessage({ type: "discover-trending", force: !!forceRefresh }, (resp) => {
      if (resp && resp.results) {
        renderAllTrending(resp.results);
      } else {
        $trendingList.innerHTML = "";
        $trendingEmpty.textContent = resp && resp.error ? resp.error : "No results found.";
        $trendingEmpty.style.display = "";
      }
    });
  }

  const SOURCE_COLORS = {
    "bioRxiv": "#f59e0b",
    "medRxiv": "#06b6d4",
    "PubMed": "#3b82f6",
    "Europe PMC": "#10b981"
  };

  // Deterministic entity color from name hash (matches PWA entityChipColor)
  var ENTITY_COLORS = [
    { bg: "rgba(168,85,247,0.15)", fg: "#c084fc" },  // purple
    { bg: "rgba(96,165,250,0.15)", fg: "#60a5fa" },   // blue
    { bg: "rgba(52,211,153,0.15)", fg: "#34d399" },   // green
    { bg: "rgba(251,146,60,0.15)", fg: "#fb923c" },   // orange
    { bg: "rgba(244,114,182,0.15)", fg: "#f472b6" },  // pink
    { bg: "rgba(34,211,238,0.15)", fg: "#22d3ee" },   // cyan
    { bg: "rgba(250,204,21,0.15)", fg: "#facc15" },   // yellow
    { bg: "rgba(129,140,248,0.15)", fg: "#818cf8" }    // indigo
  ];

  function entityColor(name) {
    var hash = 0;
    for (var i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
    return ENTITY_COLORS[Math.abs(hash) % ENTITY_COLORS.length];
  }

  function hexToRgb(hex) {
    var r = parseInt(hex.slice(1,3), 16);
    var g = parseInt(hex.slice(3,5), 16);
    var b = parseInt(hex.slice(5,7), 16);
    return r + "," + g + "," + b;
  }

  function renderAllTrending(results) {
    $trendingEmpty.style.display = "none";
    $trendingList.innerHTML = "";

    const sourceOrder = ["bioRxiv", "medRxiv", "PubMed", "Europe PMC"];
    let totalItems = 0;

    sourceOrder.forEach(source => {
      const papers = results[source];
      if (!papers || papers.length === 0) return;

      // Source header (clickable toggle)
      const header = document.createElement("div");
      var srcColor = SOURCE_COLORS[source] || "#64748b";
      header.style.cssText = "font-size:12px;font-weight:700;color:" + srcColor + ";text-transform:uppercase;letter-spacing:0.04em;padding:10px 14px 8px;margin-top:" + (totalItems > 0 ? "4px" : "0") + ";display:flex;align-items:center;justify-content:space-between;cursor:pointer;user-select:none;border-bottom:1px solid #0f172a;border-left:3px solid " + srcColor + ";background:rgba(" + hexToRgb(srcColor) + ",0.12);";
      header.innerHTML =
        '<div style="display:flex;align-items:center;gap:6px">' +
          '<span style="width:8px;height:8px;border-radius:50%;background:' + (SOURCE_COLORS[source] || "#64748b") + ';display:inline-block;flex-shrink:0"></span>' +
          escapeHtml(source) +
          ' <span style="color:#475569;font-weight:400">(' + papers.length + ')</span>' +
        '</div>' +
        '<span class="trending-source-arrow" style="font-size:8px;color:#475569;transition:transform 0.15s">&#9660;</span>';
      $trendingList.appendChild(header);

      // Papers container (collapsible)
      const container = document.createElement("div");
      container.className = "trending-source-body";

      papers.slice(0, 10).forEach(p => {
        const li = document.createElement("li");
        li.className = "trending-item";
        const url = p.pmid ? "https://pubmed.ncbi.nlm.nih.gov/" + p.pmid + "/" : (p.doi ? "https://doi.org/" + p.doi : "#");
        li.innerHTML =
          '<div class="trending-title">' + escapeHtml(p.title || "Untitled") + '</div>' +
          '<div class="trending-meta">' +
            '<span class="trending-category">' + escapeHtml(p.category || p.source || "") + '</span>' +
            '<span>' + escapeHtml(p.date || "") + '</span>' +
          '</div>';
        li.addEventListener("click", () => { window.open(url, "_blank"); });
        container.appendChild(li);
        totalItems++;
      });

      $trendingList.appendChild(container);

      // Toggle collapse
      header.addEventListener("click", () => {
        const hidden = container.style.display === "none";
        container.style.display = hidden ? "" : "none";
        const arrow = header.querySelector(".trending-source-arrow");
        if (arrow) arrow.style.transform = hidden ? "" : "rotate(-90deg)";
      });
    });

    // Update count badge
    const badge = document.getElementById("trending-count-badge");
    if (badge) badge.textContent = totalItems > 0 ? "(" + totalItems + ")" : "";

    if (totalItems === 0) {
      $trendingEmpty.textContent = "No trending papers found.";
      $trendingEmpty.style.display = "";
    }
  }

  function escapeHtml(str) {
    const d = document.createElement("div");
    d.textContent = str;
    return d.innerHTML;
  }

  if ($btnRefreshTrend) $btnRefreshTrend.addEventListener("click", (e) => {
    e.stopPropagation();
    $trendingEmpty.textContent = "Refreshing...";
    $trendingEmpty.style.display = "";
    $trendingList.innerHTML = "";
    fetchAllTrending(true);
  });

  // Load trending on startup
  fetchAllTrending(false);

  // ── Unified Database Search ───────────────────────────────────────

  function unifiedSearch(query) {
    if (!query || !query.trim()) return;
    $discoverResults.innerHTML = '<div class="discover-searching">Searching databases...</div>';
    chrome.runtime.sendMessage({ type: "discover-search", query: query.trim() }, (resp) => {
      if (resp && resp.results) {
        renderSearchResults(resp.results);
      } else {
        $discoverResults.innerHTML = '<div class="discover-search-empty">Search failed. Try again.</div>';
      }
    });
  }

  function renderSearchResults(results) {
    $discoverResults.innerHTML = "";
    const sources = [
      { key: "pubmed", label: "PubMed" },
      { key: "gene", label: "NCBI Gene" },
      { key: "clinvar", label: "ClinVar" },
      { key: "clinicaltrials", label: "ClinicalTrials.gov" },
      { key: "uniprot", label: "UniProt" }
    ];
    let anyResults = false;
    sources.forEach(src => {
      const items = results[src.key];
      if (!items || items.length === 0) return;
      anyResults = true;

      // Source header (collapsible)
      const header = document.createElement("div");
      header.className = "discover-source-header";
      header.innerHTML = `<span>${src.label}</span>
        <div style="display:flex;align-items:center;gap:4px">
          <span class="discover-source-count">${items.length}</span>
          <span class="arrow">&#9660;</span>
        </div>`;
      const body = document.createElement("div");
      header.addEventListener("click", () => {
        const hidden = body.style.display === "none";
        body.style.display = hidden ? "" : "none";
        header.querySelector(".arrow").innerHTML = hidden ? "&#9660;" : "&#9654;";
      });
      $discoverResults.appendChild(header);

      items.forEach(item => {
        const div = document.createElement("div");
        div.className = "discover-result-item";
        div.innerHTML = `
          <div class="discover-result-title">${escapeHtml(item.title || "")}</div>
          <div class="discover-result-sub">${escapeHtml(item.subtitle || "")}</div>
          <div class="discover-result-actions">
            ${item.url ? `<a href="${item.url}" target="_blank" class="btn" style="text-decoration:none">Open</a>` : ""}
            <button class="btn discover-watch-btn">Watch</button>
          </div>`;
        const watchBtn = div.querySelector(".discover-watch-btn");
        if (watchBtn) watchBtn.addEventListener("click", () => {
          const name = item.watchName || item.title || "";
          if (name) addEntity(name);
        });
        body.appendChild(div);
      });
      $discoverResults.appendChild(body);
    });
    if (!anyResults) {
      $discoverResults.innerHTML = '<div class="discover-search-empty">No results found.</div>';
    }
  }

  if ($btnDiscoverSearch) $btnDiscoverSearch.addEventListener("click", () => {
    unifiedSearch($discoverInput.value);
  });
  if ($discoverInput) $discoverInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") unifiedSearch($discoverInput.value);
  });

  // ── Help shortcuts tooltip ─────────────────────────────────────────
  const $helpBtn = document.getElementById("btn-help-shortcuts");
  const $shortcutsTip = document.getElementById("shortcuts-tooltip");
  if ($helpBtn && $shortcutsTip) {
    $helpBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      $shortcutsTip.style.display = $shortcutsTip.style.display === "none" ? "block" : "none";
    });
    document.addEventListener("click", () => { $shortcutsTip.style.display = "none"; });
  }

  // (Live refresh now handled in Tab Switching section above)

  // ── Paper Open History ─────────────────────────────────────────────

  const MAX_HISTORY = 15;

  function trackHistory(paper) {
    chrome.storage.local.get("biokhoj_history", (data) => {
      let history = data.biokhoj_history || [];
      // Remove duplicate (same pmid)
      history = history.filter(h => h.pmid !== paper.pmid);
      // Add to front
      history.unshift({
        pmid: paper.pmid,
        title: paper.title || "Untitled",
        url: paper.url || "",
        journal: paper.journal || "",
        openedAt: Date.now()
      });
      // Cap
      if (history.length > MAX_HISTORY) history = history.slice(0, MAX_HISTORY);
      chrome.storage.local.set({ biokhoj_history: history });
      renderHistory(history);
    });
  }

  function renderHistory(history) {
    const section = document.getElementById("history-section");
    const list = document.getElementById("history-list");
    if (!section || !list) return;

    if (!history || history.length === 0) {
      section.style.display = "none";
      return;
    }

    section.style.display = "";
    list.innerHTML = "";

    history.forEach(h => {
      const li = document.createElement("li");
      li.className = "history-item";
      const ago = timeSinceShort(h.openedAt);
      li.innerHTML =
        '<div class="history-title">' + escapeHtml(h.title) + '</div>' +
        '<div class="history-meta">' + escapeHtml(h.journal || "") + (h.journal ? " · " : "") + ago + '</div>';
      li.addEventListener("click", () => { window.open(h.url, "_blank"); });
      list.appendChild(li);
    });
  }

  function timeSinceShort(ts) {
    if (!ts) return "";
    var diff = Math.floor((Date.now() - ts) / 1000);
    if (diff < 60) return "just now";
    if (diff < 3600) return Math.floor(diff / 60) + "m ago";
    if (diff < 86400) return Math.floor(diff / 3600) + "h ago";
    return Math.floor(diff / 86400) + "d ago";
  }

  // History toggle
  const $historyToggle = document.getElementById("history-toggle");
  const $historyList = document.getElementById("history-list");
  const $historyArrow = document.getElementById("history-arrow");
  if ($historyToggle) $historyToggle.addEventListener("click", () => {
    const hidden = $historyList.style.display === "none";
    $historyList.style.display = hidden ? "" : "none";
    if ($historyArrow) $historyArrow.style.transform = hidden ? "" : "rotate(-90deg)";
  });

  // Load history on startup
  chrome.storage.local.get("biokhoj_history", (data) => {
    renderHistory(data.biokhoj_history || []);
  });

  // ── Empty state check button ───────────────────────────────────────
  var $btnEmptyCheck = document.getElementById("btn-empty-check");
  if ($btnEmptyCheck) $btnEmptyCheck.addEventListener("click", () => checkNow());

  // ── Paper Selection Mode ──────────────────────────────────────────

  const $selectModeBtn = document.getElementById("btn-select-mode");
  const $selectActionBar = document.getElementById("select-action-bar");
  const $selectCount = document.getElementById("select-count");
  const $btnClearSelected = document.getElementById("btn-clear-selected");
  const $btnClearAll = document.getElementById("btn-clear-all");
  const $btnCancelSelect = document.getElementById("btn-cancel-select");

  function enterSelectMode() {
    document.body.classList.add("select-mode");
    if ($selectActionBar) $selectActionBar.style.display = "flex";
    if ($selectModeBtn) $selectModeBtn.textContent = "Done";
    updateSelectCount();
  }

  function exitSelectMode() {
    document.body.classList.remove("select-mode");
    if ($selectActionBar) $selectActionBar.style.display = "none";
    if ($selectModeBtn) $selectModeBtn.textContent = "Select";
    // Uncheck all
    document.querySelectorAll(".paper-checkbox").forEach(cb => { cb.checked = false; });
    document.querySelectorAll(".paper-item.selected").forEach(el => { el.classList.remove("selected"); });
  }

  function updateSelectCount() {
    const checked = document.querySelectorAll(".paper-checkbox:checked").length;
    if ($selectCount) $selectCount.textContent = checked + " selected";
    if ($btnClearSelected) $btnClearSelected.style.display = checked > 0 ? "" : "none";
  }

  function getSelectedPmids() {
    const pmids = [];
    document.querySelectorAll(".paper-item.selected").forEach(el => {
      const pmid = el.getAttribute("data-pmid");
      if (pmid) pmids.push(pmid);
    });
    return pmids;
  }

  if ($selectModeBtn) $selectModeBtn.addEventListener("click", () => {
    if (document.body.classList.contains("select-mode")) {
      exitSelectMode();
    } else {
      enterSelectMode();
    }
  });

  if ($btnCancelSelect) $btnCancelSelect.addEventListener("click", () => exitSelectMode());

  if ($btnClearSelected) $btnClearSelected.addEventListener("click", async () => {
    const pmids = getSelectedPmids();
    if (pmids.length === 0) return;
    await sendMsg({ type: "clear-papers", pmids });
    exitSelectMode();
    showToast("Removed " + pmids.length + " paper" + (pmids.length > 1 ? "s" : ""));
    refresh();
  });

  if ($btnClearAll) $btnClearAll.addEventListener("click", async () => {
    if (!confirm("Clear all papers? This cannot be undone.")) return;
    await sendMsg({ type: "clear-papers" });
    exitSelectMode();
    showToast("All papers cleared");
    refresh();
  });

})();
