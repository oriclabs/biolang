// BioKhoj — Background Service Worker
// Research radar: monitors PubMed for new papers matching watched entities.

const ALARM_NAME = "biokhoj-check";
const ALARM_INTERVAL_MINUTES = 240; // 4 hours
const PUBMED_DELAY_MS = 350;
const MAX_PAPERS = 500;
const PUBMED_BASE = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";

// ── Installation & Alarm Setup ──────────────────────────────────────

chrome.runtime.onInstalled.addListener(() => {
  chrome.alarms.create(ALARM_NAME, { periodInMinutes: ALARM_INTERVAL_MINUTES });

  chrome.contextMenus.create({
    id: "biokhoj-watch",
    title: "Watch in BioKhoj",
    contexts: ["selection"]
  });

  chrome.contextMenus.create({
    id: "biokhoj-lookup",
    title: "Lookup in BioKhoj",
    contexts: ["selection"]
  });

  chrome.contextMenus.create({
    id: "biokhoj-save-link",
    title: "Save to BioKhoj reading list",
    contexts: ["link"],
    targetUrlPatterns: [
      "*://pubmed.ncbi.nlm.nih.gov/*",
      "*://doi.org/*",
      "*://www.biorxiv.org/*",
      "*://www.medrxiv.org/*",
      "*://www.ncbi.nlm.nih.gov/gene/*",
      "*://www.ncbi.nlm.nih.gov/clinvar/*",
      "*://www.nature.com/*",
      "*://www.science.org/*",
      "*://www.cell.com/*"
    ]
  });
});

// Open sidebar on action click
chrome.action.onClicked.addListener(async (tab) => {
  await chrome.sidePanel.open({ tabId: tab.id });
});

// ── Entity Classification ───────────────────────────────────────────

function classifyEntity(text) {
  const trimmed = (text || "").trim();
  if (!trimmed) return null;

  // Variant patterns: rs IDs, HGVS-like, p. notation
  if (/^rs\d+$/i.test(trimmed)) return { name: trimmed, type: "variant" };
  if (/^(c|g|p|m)\.\d/i.test(trimmed)) return { name: trimmed, type: "variant" };
  if (/^(NM_|NP_|NC_)\d+.*:[cpg]\./i.test(trimmed)) return { name: trimmed, type: "variant" };

  // Drug patterns: common suffixes
  if (/(?:mab|nib|lib|vir|tin|ide|ine|ase|pril|olol|statin|sartan|dipine|cycline|mycin|cillin)$/i.test(trimmed)) {
    return { name: trimmed, type: "drug" };
  }

  // Disease patterns: common suffixes/keywords
  if (/(?:oma|emia|itis|osis|pathy|trophy|plasia|carcinoma|lymphoma|leukemia|syndrome)$/i.test(trimmed)) {
    return { name: trimmed, type: "disease" };
  }

  // Pathway patterns
  if (/(?:pathway|signaling|cascade|axis)$/i.test(trimmed)) {
    return { name: trimmed, type: "pathway" };
  }

  // Gene-like: uppercase letters/numbers, 2-10 chars
  if (/^[A-Z][A-Z0-9]{1,9}$/.test(trimmed)) return { name: trimmed, type: "gene" };

  // Default: treat as gene
  return { name: trimmed, type: "gene" };
}

// ── PubMed Search ───────────────────────────────────────────────────

async function searchPubMed(query, maxResults = 5) {
  try {
    const searchUrl = `${PUBMED_BASE}/esearch.fcgi?db=pubmed&retmode=json&retmax=${maxResults}&sort=date&term=${encodeURIComponent(query)}`;
    const searchResp = await fetch(searchUrl);
    if (!searchResp.ok) return [];
    const searchData = await searchResp.json();
    const ids = searchData.esearchresult?.idlist || [];
    if (ids.length === 0) return [];

    await sleep(PUBMED_DELAY_MS);

    const summaryUrl = `${PUBMED_BASE}/esummary.fcgi?db=pubmed&retmode=json&id=${ids.join(",")}`;
    const summaryResp = await fetch(summaryUrl);
    if (!summaryResp.ok) return [];
    const summaryData = await summaryResp.json();
    const result = summaryData.result || {};

    return ids.map(id => {
      const doc = result[id];
      if (!doc) return null;
      return {
        pmid: id,
        title: doc.title || "Untitled",
        journal: doc.fulljournalname || doc.source || "",
        date: doc.pubdate || doc.epubdate || "",
        authors: (doc.authors || []).map(a => a.name).slice(0, 3).join(", "),
        url: `https://pubmed.ncbi.nlm.nih.gov/${id}/`
      };
    }).filter(Boolean);
  } catch (e) {
    console.warn("BioKhoj: PubMed search failed:", e);
    return [];
  }
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// ── Signal Score ────────────────────────────────────────────────────
// Simple heuristic score based on journal prestige keywords and recency

function computeSignalScore(paper) {
  let score = 50;
  const title = (paper.title || "").toLowerCase();
  const journal = (paper.journal || "").toLowerCase();

  // High-impact journal boost
  const topJournals = ["nature", "science", "cell", "lancet", "nejm", "new england", "jama", "bmj", "pnas"];
  if (topJournals.some(j => journal.includes(j))) score += 20;

  // Mid-tier journal boost
  const midJournals = ["plos", "genome", "nucleic acids", "bioinformatics", "molecular", "cancer"];
  if (midJournals.some(j => journal.includes(j))) score += 10;

  // Recency boost (papers from this year)
  const thisYear = new Date().getFullYear().toString();
  if ((paper.date || "").includes(thisYear)) score += 10;

  // Title keyword boost
  const hotWords = ["crispr", "single-cell", "gwas", "novel", "therapeutic", "clinical trial", "phase", "breakthrough"];
  if (hotWords.some(w => title.includes(w))) score += 10;

  return Math.min(100, Math.max(0, score));
}

// ── Periodic Check ──────────────────────────────────────────────────

chrome.alarms.onAlarm.addListener(async (alarm) => {
  if (alarm.name === ALARM_NAME) {
    await runCheck();
  }
});

async function runCheck() {
  const data = await chrome.storage.local.get(["biokhoj_watchlist", "biokhoj_papers", "biokhoj_settings"]);
  const watchlist = data.biokhoj_watchlist || [];
  if (watchlist.length === 0) return;

  // Sort by priority (desc) then lastChecked (asc) — check highest priority + stalest first
  const sorted = [...watchlist].sort((a, b) => {
    const pDiff = (b.priority || 0) - (a.priority || 0);
    if (pDiff !== 0) return pDiff;
    return (a.lastChecked || 0) - (b.lastChecked || 0);
  });

  const toCheck = sorted.slice(0, 5);
  const existingPapers = data.biokhoj_papers || [];
  const existingPmids = new Set(existingPapers.map(p => p.pmid));
  const newPapers = [];
  const now = Date.now();

  for (const entity of toCheck) {
    // Search PubMed
    const pubmedPapers = await searchPubMed(entity.name, 5);
    await sleep(PUBMED_DELAY_MS);

    // Search bioRxiv
    let biorxivPapers = [];
    try {
      const end = new Date();
      const start = new Date(end.getTime() - 7 * 24 * 60 * 60 * 1000);
      const fmt = d => d.toISOString().slice(0, 10);
      const resp = await fetch(`https://api.biorxiv.org/details/biorxiv/${fmt(start)}/${fmt(end)}/0/5`);
      if (resp.ok) {
        const data = await resp.json();
        const query = entity.name.toLowerCase();
        biorxivPapers = (data.collection || [])
          .filter(p => ((p.title || "") + " " + (p.abstract || "")).toLowerCase().includes(query))
          .slice(0, 3)
          .map(p => ({
            pmid: null,
            title: p.title || "Untitled",
            journal: "bioRxiv",
            date: p.date || "",
            authors: (p.authors || "").substring(0, 80),
            url: p.doi ? "https://doi.org/" + p.doi : "#",
            doi: p.doi || "",
            source: "biorxiv"
          }));
      }
    } catch (e) { /* bioRxiv is optional */ }

    const allPapers = [...pubmedPapers, ...biorxivPapers];

    for (const paper of allPapers) {
      const paperId = paper.pmid || paper.doi || null;
      if (!paperId || existingPmids.has(paperId)) continue;
      existingPmids.add(paperId);
      newPapers.push({
        ...paper,
        pmid: paperId,
        entity: entity.name,
        entityType: entity.type,
        signalScore: computeSignalScore(paper),
        read: false,
        fetchedAt: now
      });
    }

    // Update lastChecked on the original watchlist entry
    entity.lastChecked = now;
  }

  // Merge into watchlist (update lastChecked)
  const updatedWatchlist = watchlist.map(w => {
    const checked = toCheck.find(t => t.name === w.name && t.type === w.type);
    return checked ? { ...w, lastChecked: checked.lastChecked } : w;
  });

  // Merge papers, cap at MAX_PAPERS
  const allPapers = [...newPapers, ...existingPapers].slice(0, MAX_PAPERS);

  await chrome.storage.local.set({
    biokhoj_watchlist: updatedWatchlist,
    biokhoj_papers: allPapers
  });

  // Update badge
  const unreadCount = allPapers.filter(p => !p.read).length;
  updateBadge(unreadCount);

  // Notify for high-priority new papers
  if (newPapers.length > 0) {
    const highPriority = newPapers.filter(p => p.signalScore >= 70);
    if (highPriority.length > 0) {
      // Rich notification with top paper details
      const top = highPriority.sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0))[0];
      const entities = [...new Set(highPriority.map(p => p.entity))];
      const entityText = entities.slice(0, 3).join(", ");
      const title = highPriority.length === 1
        ? `\u26A1${top.signalScore} — ${top.entity} in ${top.journal || "new paper"}`
        : `${highPriority.length} high-signal papers for ${entityText}`;
      const message = highPriority.length === 1
        ? top.title.slice(0, 100)
        : highPriority.slice(0, 3).map(p => `\u26A1${p.signalScore} ${p.title.slice(0, 50)}`).join("\n");
      chrome.notifications.create("biokhoj-new-" + now, {
        type: "basic",
        iconUrl: "icons/icon128.png",
        title: title,
        message: message
      });
    }
  }

  // Also update citation counts for reading list papers
  await fetchCitationsForReadingList();
}

// ── Citation Tracker (OpenAlex) ─────────────────────────────────────

async function fetchCitationsForReadingList() {
  try {
    const data = await chrome.storage.local.get(["biokhoj_reading_list", "biokhoj_citations"]);
    const readingList = data.biokhoj_reading_list || [];
    const citations = data.biokhoj_citations || {};
    const now = Date.now();

    // Only check papers that have a PMID and haven't been checked in 24h
    const toCheck = readingList.filter(p => {
      if (!p.pmid) return false;
      const entry = citations[p.pmid];
      if (entry && entry.lastChecked && (now - entry.lastChecked) < 24 * 60 * 60 * 1000) return false;
      return true;
    }).slice(0, 5); // max 5 per run to avoid rate limits

    for (const paper of toCheck) {
      try {
        // Try DOI first, fall back to PMID search
        let url = `https://api.openalex.org/works/pmid:${paper.pmid}`;
        const resp = await fetch(url, { headers: { "Accept": "application/json" } });
        if (resp.ok) {
          const work = await resp.json();
          const count = work.cited_by_count || 0;
          const entry = citations[paper.pmid] || { history: [] };
          entry.history.push({ count, date: now });
          // Keep last 12 data points
          if (entry.history.length > 12) entry.history = entry.history.slice(-12);
          entry.lastChecked = now;
          entry.currentCount = count;
          citations[paper.pmid] = entry;
        }
        await sleep(500); // rate limit courtesy
      } catch (e) {
        console.warn("BioKhoj: citation fetch failed for", paper.pmid, e);
      }
    }

    await chrome.storage.local.set({ biokhoj_citations: citations });
  } catch (e) {
    console.warn("BioKhoj: citation tracker error:", e);
  }
}

function updateBadge(count) {
  chrome.action.setBadgeText({ text: count > 0 ? String(count) : "" });
  chrome.action.setBadgeBackgroundColor({
    color: count > 10 ? "#F4C430" : count > 0 ? "#7c3aed" : "#475569"
  });
}

// ── Context Menu ────────────────────────────────────────────────────

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  // ── Lookup: show entity info in sidebar ──
  if (info.menuItemId === "biokhoj-lookup") {
    const text = (info.selectionText || "").trim();
    if (!text) return;
    const entity = classifyEntity(text);
    // Open sidebar first (must be in direct user gesture), then store data
    if (tab && tab.id) chrome.sidePanel.open({ tabId: tab.id }).catch(() => {});
    chrome.storage.session.set({ biokhoj_lookup: { term: entity.name, type: entity.type, ts: Date.now() } });
    return;
  }

  // ── Save link to reading list ──
  if (info.menuItemId === "biokhoj-save-link") {
    const url = info.linkUrl || "";
    if (!url) return;
    // Extract paper info from URL
    let pmid = null, title = url;
    const pubmedMatch = url.match(/pubmed\.ncbi\.nlm\.nih\.gov\/(\d+)/);
    if (pubmedMatch) pmid = pubmedMatch[1];
    const doiMatch = url.match(/doi\.org\/(.+)/);

    // Try to fetch metadata if it's a PubMed link
    if (pmid) {
      try {
        const papers = await searchPubMed(pmid, 1);
        if (papers.length > 0) {
          const paper = papers[0];
          const data = await chrome.storage.local.get("biokhoj_papers");
          const existing = (data.biokhoj_papers || []);
          if (!existing.some(p => p.pmid === pmid)) {
            existing.unshift({
              ...paper,
              signalScore: computeSignalScore(paper),
              read: false,
              saved: true,
              savedAt: Date.now(),
              fetchedAt: Date.now()
            });
            await chrome.storage.local.set({ biokhoj_papers: existing.slice(0, MAX_PAPERS) });
          }
          chrome.notifications.create("biokhoj-saved-" + Date.now(), {
            type: "basic", iconUrl: "icons/icon128.png",
            title: "Saved to BioKhoj",
            message: paper.title.slice(0, 80)
          });
          notifySidebarRefresh();
          return;
        }
      } catch (e) { /* fall through */ }
    }

    // Fallback: save with URL as identifier
    chrome.notifications.create("biokhoj-saved-" + Date.now(), {
      type: "basic", iconUrl: "icons/icon128.png",
      title: "BioKhoj",
      message: "Open BioKhoj to save this paper manually: " + url.slice(0, 60)
    });
    return;
  }

  // ── Watch selected text ──
  if (info.menuItemId !== "biokhoj-watch") return;

  const text = (info.selectionText || "").trim();
  if (!text) return;

  const entity = classifyEntity(text);
  if (!entity) return;

  // Open sidebar immediately (must be in direct user gesture context)
  if (tab && tab.id) {
    chrome.sidePanel.open({ tabId: tab.id }).catch(() => {});
  }

  const data = await chrome.storage.local.get("biokhoj_watchlist");
  const watchlist = data.biokhoj_watchlist || [];

  // Skip duplicates (case-insensitive, name only)
  if (watchlist.some(w => w.name.toLowerCase() === entity.name.toLowerCase())) {
    chrome.notifications.create("biokhoj-dup-" + Date.now(), {
      type: "basic",
      iconUrl: "icons/icon128.png",
      title: "BioKhoj",
      message: `${entity.name} is already on your watchlist.`
    });
    return;
  }

  watchlist.push({
    name: entity.name,
    type: entity.type,
    priority: 5,
    addedAt: Date.now(),
    lastChecked: 0
  });

  await chrome.storage.local.set({ biokhoj_watchlist: watchlist });

  chrome.notifications.create("biokhoj-add-" + Date.now(), {
    type: "basic",
    iconUrl: "icons/icon128.png",
    title: "BioKhoj",
    message: `Now watching ${entity.name} (${entity.type})`
  });

  // Notify any open sidebar/popup to refresh
  notifySidebarRefresh();
});

// Broadcast refresh to all extension views (sidebar, popup)
function notifySidebarRefresh() {
  chrome.runtime.sendMessage({ type: "watchlist-updated" }).catch(() => {});
}

// ── Message Handlers ────────────────────────────────────────────────

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {

  if (msg.type === "clear-papers") {
    // Clear all or selected papers by pmid list
    chrome.storage.local.get("biokhoj_papers", (data) => {
      let papers = data.biokhoj_papers || [];
      if (msg.pmids && msg.pmids.length > 0) {
        const toRemove = new Set(msg.pmids);
        papers = papers.filter(p => !toRemove.has(p.pmid));
      } else {
        papers = [];
      }
      chrome.storage.local.set({ biokhoj_papers: papers }, () => {
        updateBadge(papers.filter(p => !p.read).length);
        sendResponse({ ok: true, remaining: papers.length });
      });
    });
    return true;
  }

  if (msg.type === "get-watchlist") {
    chrome.storage.local.get("biokhoj_watchlist", (data) => {
      sendResponse({ watchlist: data.biokhoj_watchlist || [] });
    });
    return true;
  }

  if (msg.type === "add-watch") {
    chrome.storage.local.get("biokhoj_watchlist", (data) => {
      const watchlist = data.biokhoj_watchlist || [];
      const entity = msg.entity;
      if (!entity || !entity.name) { sendResponse({ ok: false }); return; }

      // Skip duplicates (case-insensitive, name only — type may vary)
      if (watchlist.some(w => w.name.toLowerCase() === entity.name.toLowerCase())) {
        sendResponse({ ok: false, reason: "duplicate" });
        return;
      }

      watchlist.push({
        name: entity.name,
        type: entity.type,
        priority: entity.priority || 5,
        addedAt: Date.now(),
        lastChecked: 0
      });

      chrome.storage.local.set({ biokhoj_watchlist: watchlist }, () => {
        notifySidebarRefresh();
        sendResponse({ ok: true, watchlist });
      });
    });
    return true;
  }

  if (msg.type === "remove-watch") {
    chrome.storage.local.get("biokhoj_watchlist", (data) => {
      const removeName = (msg.name || "").toLowerCase();
      const watchlist = (data.biokhoj_watchlist || []).filter(
        w => w.name.toLowerCase() !== removeName
      );
      chrome.storage.local.set({ biokhoj_watchlist: watchlist }, () => {
        notifySidebarRefresh();
        sendResponse({ ok: true, watchlist });
      });
    });
    return true;
  }

  if (msg.type === "get-new-papers") {
    chrome.storage.local.get("biokhoj_papers", (data) => {
      const papers = data.biokhoj_papers || [];
      const since = msg.since || 0;
      const filtered = since > 0 ? papers.filter(p => p.fetchedAt > since) : papers;
      sendResponse({ papers: filtered });
    });
    return true;
  }

  if (msg.type === "get-paper-count") {
    chrome.storage.local.get("biokhoj_papers", (data) => {
      const papers = data.biokhoj_papers || [];
      const unread = papers.filter(p => !p.read).length;
      sendResponse({ total: papers.length, unread });
    });
    return true;
  }

  if (msg.type === "mark-read") {
    chrome.storage.local.get("biokhoj_papers", (data) => {
      const papers = (data.biokhoj_papers || []).map(p => {
        if (p.pmid === msg.pmid) return { ...p, read: msg.read !== undefined ? msg.read : !p.read };
        return p;
      });
      chrome.storage.local.set({ biokhoj_papers: papers }, () => {
        const unread = papers.filter(p => !p.read).length;
        updateBadge(unread);
        sendResponse({ ok: true });
      });
    });
    return true;
  }

  if (msg.type === "check-now") {
    runCheck().then(() => {
      sendResponse({ ok: true });
    }).catch((e) => {
      sendResponse({ ok: false, error: e.message });
    });
    return true;
  }

  if (msg.type === "get-citations") {
    chrome.storage.local.get("biokhoj_citations", (data) => {
      sendResponse({ citations: data.biokhoj_citations || {} });
    });
    return true;
  }

  if (msg.type === "refresh-citations") {
    fetchCitationsForReadingList().then(() => {
      chrome.storage.local.get("biokhoj_citations", (data) => {
        sendResponse({ ok: true, citations: data.biokhoj_citations || {} });
      });
    }).catch((e) => {
      sendResponse({ ok: false, error: e.message });
    });
    return true;
  }

  if (msg.type === "open-pwa") {
    chrome.tabs.create({ url: "https://lang.bio/biokhoj" });
    sendResponse({ ok: true });
    return true;
  }

  // ── Discover: bioRxiv Trending ──────────────────────────────────
  if (msg.type === "discover-trending") {
    fetchAllTrending(msg.force).then(results => {
      sendResponse({ results });
    }).catch(e => {
      sendResponse({ results: {}, error: e.message });
    });
    return true;
  }

  // ── Discover: Unified Database Search ───────────────────────────
  if (msg.type === "discover-search") {
    runUnifiedSearch(msg.query).then(results => {
      sendResponse({ results });
    }).catch(e => {
      sendResponse({ results: null, error: e.message });
    });
    return true;
  }
});

// ── Discover: bioRxiv Trending (cached 6h) ──────────────────────────

let _trendingCache = null;
let _trendingCacheTime = 0;
const TRENDING_CACHE_TTL = 6 * 60 * 60 * 1000; // 6 hours

async function fetchTrendingBioRxiv(force) {
  const now = Date.now();
  if (!force && _trendingCache && (now - _trendingCacheTime) < TRENDING_CACHE_TTL) {
    return _trendingCache;
  }
  // Also check chrome.storage cache
  if (!force) {
    try {
      const stored = await chrome.storage.local.get("biokhoj_trending");
      if (stored.biokhoj_trending && stored.biokhoj_trending.papers &&
          (now - (stored.biokhoj_trending.ts || 0)) < TRENDING_CACHE_TTL) {
        _trendingCache = stored.biokhoj_trending.papers;
        _trendingCacheTime = stored.biokhoj_trending.ts;
        return _trendingCache;
      }
    } catch (e) { /* proceed to fetch */ }
  }

  // Compute date range: last 7 days
  const end = new Date();
  const start = new Date(end.getTime() - 7 * 24 * 60 * 60 * 1000);
  const fmt = d => d.toISOString().slice(0, 10);
  const url = `https://api.biorxiv.org/details/biorxiv/${fmt(start)}/${fmt(end)}/0/15`;

  try {
    const resp = await fetch(url);
    if (!resp.ok) throw new Error("bioRxiv API error " + resp.status);
    const data = await resp.json();
    const collection = data.collection || [];
    const papers = collection.map(p => ({
      title: p.title || "Untitled",
      category: p.category || "biology",
      date: p.date || "",
      doi: p.doi || "",
      authors: p.authors || ""
    }));
    _trendingCache = papers;
    _trendingCacheTime = now;
    await chrome.storage.local.set({ biokhoj_trending: { papers, ts: now } });
    return papers;
  } catch (e) {
    console.warn("BioKhoj: bioRxiv trending fetch failed:", e);
    throw e;
  }
}

// ── Fetch medRxiv trending ──────────────────────────────────────────

async function fetchTrendingMedRxiv() {
  const end = new Date();
  const start = new Date(end.getTime() - 7 * 24 * 60 * 60 * 1000);
  const fmt = d => d.toISOString().slice(0, 10);
  const url = `https://api.biorxiv.org/details/medrxiv/${fmt(start)}/${fmt(end)}/0/10`;
  try {
    const resp = await fetch(url);
    if (!resp.ok) return [];
    const data = await resp.json();
    return (data.collection || []).map(p => ({
      title: p.title || "Untitled",
      category: p.category || "clinical",
      date: p.date || "",
      doi: p.doi || "",
      authors: p.authors || "",
      source: "medRxiv"
    }));
  } catch (e) { return []; }
}

// ── Fetch PubMed trending (most recent high-impact) ────────────────

async function fetchTrendingPubMed() {
  try {
    const searchResp = await fetch("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&retmode=json&retmax=10&sort=relevance&term=genomics+OR+bioinformatics+OR+CRISPR+OR+cancer+genomics&datetype=pdat&reldate=7");
    if (!searchResp.ok) return [];
    const searchData = await searchResp.json();
    const ids = searchData.esearchresult && searchData.esearchresult.idlist;
    if (!ids || ids.length === 0) return [];

    await new Promise(r => setTimeout(r, 350));
    const summResp = await fetch("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&retmode=json&id=" + ids.join(","));
    if (!summResp.ok) return [];
    const summData = await summResp.json();
    const result = summData.result || {};

    return ids.map(id => {
      const a = result[id];
      if (!a) return null;
      return {
        title: (a.title || "").replace(/<\/?[^>]+>/g, ""),
        category: a.fulljournalname || a.source || "",
        date: (a.pubdate || "").split(" ")[0] || "",
        doi: "",
        authors: (a.authors || []).slice(0, 2).map(au => au.name).join(", "),
        source: "PubMed",
        pmid: id
      };
    }).filter(Boolean);
  } catch (e) { return []; }
}

// ── Fetch from EuropePMC (open access preprints & papers) ──────────

async function fetchTrendingEuropePMC() {
  try {
    const resp = await fetch("https://www.ebi.ac.uk/europepmc/webservices/rest/search?query=genomics+OR+bioinformatics&format=json&pageSize=10&sort=DATE_PUBLISHED+desc");
    if (!resp.ok) return [];
    const data = await resp.json();
    return (data.resultList && data.resultList.result || []).map(p => ({
      title: p.title || "Untitled",
      category: p.journalTitle || p.source || "",
      date: p.firstPublicationDate || "",
      doi: p.doi || "",
      authors: (p.authorString || "").substring(0, 60),
      source: "Europe PMC"
    }));
  } catch (e) { return []; }
}

// ── Fetch all trending sources ─────────────────────────────────────

let _allTrendingCache = null;
let _allTrendingCacheTime = 0;

async function fetchAllTrending(force) {
  const now = Date.now();
  if (!force && _allTrendingCache && (now - _allTrendingCacheTime) < TRENDING_CACHE_TTL) {
    return _allTrendingCache;
  }
  if (!force) {
    try {
      const stored = await chrome.storage.local.get("biokhoj_all_trending");
      if (stored.biokhoj_all_trending && (now - (stored.biokhoj_all_trending.ts || 0)) < TRENDING_CACHE_TTL) {
        _allTrendingCache = stored.biokhoj_all_trending.results;
        _allTrendingCacheTime = stored.biokhoj_all_trending.ts;
        return _allTrendingCache;
      }
    } catch (e) {}
  }

  const [biorxiv, medrxiv, pubmed, europepmc] = await Promise.allSettled([
    fetchTrendingBioRxiv(true),
    fetchTrendingMedRxiv(),
    fetchTrendingPubMed(),
    fetchTrendingEuropePMC()
  ]);

  const results = {
    bioRxiv: (biorxiv.status === "fulfilled" ? biorxiv.value : []).map(p => ({ ...p, source: "bioRxiv" })),
    medRxiv: medrxiv.status === "fulfilled" ? medrxiv.value : [],
    PubMed: pubmed.status === "fulfilled" ? pubmed.value : [],
    "Europe PMC": europepmc.status === "fulfilled" ? europepmc.value : []
  };

  _allTrendingCache = results;
  _allTrendingCacheTime = now;
  await chrome.storage.local.set({ biokhoj_all_trending: { results, ts: now } });
  return results;
}

// ── Discover: Unified Database Search ───────────────────────────────

async function runUnifiedSearch(query) {
  if (!query || !query.trim()) return {};
  const q = query.trim();
  const results = {};

  // Run all searches in parallel, with NCBI calls staggered by 350ms
  const [pubmedRes, clinTrialsRes, uniprotRes] = await Promise.allSettled([
    searchNCBISequential(q),
    searchClinicalTrials(q),
    searchUniProt(q)
  ]);

  // NCBI results (sequential within, but parallel with others)
  if (pubmedRes.status === "fulfilled" && pubmedRes.value) {
    Object.assign(results, pubmedRes.value);
  }
  if (clinTrialsRes.status === "fulfilled") results.clinicaltrials = clinTrialsRes.value || [];
  if (uniprotRes.status === "fulfilled") results.uniprot = uniprotRes.value || [];

  return results;
}

// NCBI databases share rate limit — query them sequentially with 350ms gaps
async function searchNCBISequential(query) {
  const results = {};
  const enc = encodeURIComponent(query);

  // PubMed (top 3)
  try {
    const sr = await fetch(`${PUBMED_BASE}/esearch.fcgi?db=pubmed&retmode=json&retmax=3&sort=date&term=${enc}`);
    if (sr.ok) {
      const sd = await sr.json();
      const ids = sd.esearchresult?.idlist || [];
      if (ids.length > 0) {
        await sleep(PUBMED_DELAY_MS);
        const smr = await fetch(`${PUBMED_BASE}/esummary.fcgi?db=pubmed&retmode=json&id=${ids.join(",")}`);
        if (smr.ok) {
          const smd = await smr.json();
          const r = smd.result || {};
          results.pubmed = ids.map(id => {
            const doc = r[id];
            if (!doc) return null;
            return {
              title: doc.title || "Untitled",
              subtitle: `${doc.fulljournalname || doc.source || ""} - ${doc.pubdate || ""}`,
              url: `https://pubmed.ncbi.nlm.nih.gov/${id}/`,
              watchName: query
            };
          }).filter(Boolean);
        }
      }
    }
  } catch (e) { console.warn("BioKhoj discover: PubMed error", e); }

  await sleep(PUBMED_DELAY_MS);

  // NCBI Gene (top 2)
  try {
    const sr = await fetch(`${PUBMED_BASE}/esearch.fcgi?db=gene&retmode=json&retmax=2&term=${enc}`);
    if (sr.ok) {
      const sd = await sr.json();
      const ids = sd.esearchresult?.idlist || [];
      if (ids.length > 0) {
        await sleep(PUBMED_DELAY_MS);
        const smr = await fetch(`${PUBMED_BASE}/esummary.fcgi?db=gene&retmode=json&id=${ids.join(",")}`);
        if (smr.ok) {
          const smd = await smr.json();
          const r = smd.result || {};
          results.gene = ids.map(id => {
            const doc = r[id];
            if (!doc) return null;
            return {
              title: doc.name || doc.nomenclaturesymbol || "Unknown",
              subtitle: `${doc.description || doc.nomenclaturename || ""} - ${doc.organism?.scientificname || ""}`,
              url: `https://www.ncbi.nlm.nih.gov/gene/${id}`,
              watchName: doc.nomenclaturesymbol || doc.name || query
            };
          }).filter(Boolean);
        }
      }
    }
  } catch (e) { console.warn("BioKhoj discover: Gene error", e); }

  await sleep(PUBMED_DELAY_MS);

  // ClinVar (top 2)
  try {
    const sr = await fetch(`${PUBMED_BASE}/esearch.fcgi?db=clinvar&retmode=json&retmax=2&term=${enc}`);
    if (sr.ok) {
      const sd = await sr.json();
      const ids = sd.esearchresult?.idlist || [];
      if (ids.length > 0) {
        await sleep(PUBMED_DELAY_MS);
        const smr = await fetch(`${PUBMED_BASE}/esummary.fcgi?db=clinvar&retmode=json&id=${ids.join(",")}`);
        if (smr.ok) {
          const smd = await smr.json();
          const r = smd.result || {};
          results.clinvar = ids.map(id => {
            const doc = r[id];
            if (!doc) return null;
            const sig = doc.clinical_significance?.description || doc.germline_classification?.description || "";
            return {
              title: doc.title || doc.variation_set?.[0]?.variation_name || "Variant",
              subtitle: sig ? `Clinical significance: ${sig}` : "",
              url: `https://www.ncbi.nlm.nih.gov/clinvar/variation/${id}/`,
              watchName: query
            };
          }).filter(Boolean);
        }
      }
    }
  } catch (e) { console.warn("BioKhoj discover: ClinVar error", e); }

  return results;
}

async function searchClinicalTrials(query) {
  try {
    const url = `https://clinicaltrials.gov/api/v2/studies?query.term=${encodeURIComponent(query)}&pageSize=2`;
    const resp = await fetch(url);
    if (!resp.ok) return [];
    const data = await resp.json();
    const studies = data.studies || [];
    return studies.map(s => {
      const proto = s.protocolSection || {};
      const id = proto.identificationModule || {};
      const status = proto.statusModule || {};
      const design = proto.designModule || {};
      const nctId = id.nctId || "";
      const phases = design.phases ? design.phases.join(", ") : "";
      return {
        title: id.briefTitle || id.officialTitle || "Untitled",
        subtitle: [status.overallStatus, phases].filter(Boolean).join(" - "),
        url: nctId ? `https://clinicaltrials.gov/study/${nctId}` : "",
        watchName: query
      };
    });
  } catch (e) {
    console.warn("BioKhoj discover: ClinicalTrials error", e);
    return [];
  }
}

async function searchUniProt(query) {
  try {
    const url = `https://rest.uniprot.org/uniprotkb/search?query=${encodeURIComponent(query)}&size=2&format=json`;
    const resp = await fetch(url);
    if (!resp.ok) return [];
    const data = await resp.json();
    const entries = data.results || [];
    return entries.map(e => {
      const recName = e.proteinDescription?.recommendedName?.fullName?.value || "";
      const org = e.organism?.scientificName || "";
      return {
        title: e.uniProtkbId || e.primaryAccession || "Unknown",
        subtitle: [recName, org].filter(Boolean).join(" - "),
        url: `https://www.uniprot.org/uniprotkb/${e.primaryAccession || ""}`,
        watchName: e.uniProtkbId || query
      };
    });
  } catch (e) {
    console.warn("BioKhoj discover: UniProt error", e);
    return [];
  }
}
