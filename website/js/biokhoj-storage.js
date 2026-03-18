// BioKhoj Storage — Abstraction layer over IndexedDB (Dexie)
// No DOM APIs, no rendering — pure data operations
// Both PWA and extension import this same module
// Usage: const storage = window.BioKhojStorage;

(function () {
  'use strict';

  // ══════════════════════════════════════════════════════════════════════
  // Database Setup
  // ══════════════════════════════════════════════════════════════════════

  let db = null;

  function initDB(Dexie) {
    if (db) return db;
    db = new Dexie('BioKhojDB');
    db.version(1).stores({
      watchlist: '++id, term, type, priority, paused, addedAt',
      papers: '++id, pmid, title, signalScore, date, saved, starred, read, *matchedEntities',
      coMentions: '++id, entityA, entityB, pmid, discoveredAt',
      trends: '++id, entity, date, count',
      settings: 'key'
    });
    db.version(2).stores({
      watchlist: '++id, term, type, priority, paused, addedAt',
      papers: '++id, pmid, title, signalScore, date, saved, starred, read, *matchedEntities',
      coMentions: '++id, entityA, entityB, pmid, discoveredAt',
      trends: '++id, entity, date, count',
      settings: 'key',
      citations: 'pmid'
    });
    db.version(3).stores({
      watchlist: '++id, term, type, priority, paused, addedAt',
      papers: '++id, pmid, title, signalScore, date, saved, starred, read, *matchedEntities',
      coMentions: '++id, entityA, entityB, pmid, discoveredAt',
      trends: '++id, entity, date, count',
      settings: 'key',
      citations: 'pmid',
      discover_cache: 'key'
    });
    return db;
  }

  function getDB() {
    if (!db) throw new Error('BioKhojStorage: call init(Dexie) first');
    return db;
  }

  // ══════════════════════════════════════════════════════════════════════
  // Settings
  // ══════════════════════════════════════════════════════════════════════

  async function getSetting(key) {
    const row = await getDB().settings.get(key);
    return row ? row.value : null;
  }

  async function saveSetting(key, value) {
    await getDB().settings.put({ key, value });
  }

  // ══════════════════════════════════════════════════════════════════════
  // Watchlist
  // ══════════════════════════════════════════════════════════════════════

  async function getWatchlist() {
    return getDB().watchlist.toArray();
  }

  async function getActiveWatchlist() {
    return (await getDB().watchlist.toArray()).filter(e => !e.paused);
  }

  async function getWatchlistCount() {
    return getDB().watchlist.count();
  }

  async function getWatchlistById(id) {
    return getDB().watchlist.get(id);
  }

  async function getWatchlistByTerm(term) {
    return getDB().watchlist.where('term').equalsIgnoreCase(term).first();
  }

  async function addWatchlistEntity(entity) {
    const record = {
      term: entity.term,
      type: entity.type || 'topic',
      priority: entity.priority || 'normal',
      tags: entity.tags || [],
      paused: false,
      addedAt: Date.now(),
      lastChecked: 0,
      ...entity
    };
    return getDB().watchlist.add(record);
  }

  async function updateWatchlistEntity(id, changes) {
    return getDB().watchlist.update(id, changes);
  }

  async function deleteWatchlistEntity(id) {
    return getDB().watchlist.delete(id);
  }

  async function bulkAddWatchlist(entities) {
    return getDB().watchlist.bulkAdd(entities);
  }

  // ══════════════════════════════════════════════════════════════════════
  // Papers
  // ══════════════════════════════════════════════════════════════════════

  async function getAllPapers() {
    return getDB().papers.toArray();
  }

  async function getPaperByPmid(pmid) {
    return getDB().papers.where('pmid').equals(pmid).first();
  }

  async function getPapersForEntity(term) {
    return getDB().papers.where('matchedEntities').equals(term).toArray();
  }

  async function getUnreadPapersForEntity(term) {
    return getDB().papers.where('matchedEntities').equals(term).and(p => !p.read).count();
  }

  async function getSavedPapers() {
    let papers = await getDB().papers.where('saved').equals(1).toArray();
    // Dexie stores booleans; also check truthy
    if (papers.length === 0) {
      papers = (await getDB().papers.toArray()).filter(p => p.saved);
    }
    return papers;
  }

  async function addPaper(paper) {
    const record = {
      saved: false,
      starred: false,
      read: false,
      savedAt: null,
      notes: '',
      paperTags: [],
      ...paper
    };
    return getDB().papers.add(record);
  }

  async function updatePaper(id, changes) {
    return getDB().papers.update(id, changes);
  }

  async function bulkAddPapers(papers) {
    return getDB().papers.bulkAdd(papers);
  }

  // ══════════════════════════════════════════════════════════════════════
  // Co-Mentions
  // ══════════════════════════════════════════════════════════════════════

  async function getRecentCoMentions(limit) {
    return getDB().coMentions.orderBy('discoveredAt').reverse().limit(limit || 5).toArray();
  }

  async function findCoMention(entityA, entityB) {
    try {
      return await getDB().coMentions
        .where('[entityA+entityB]')
        .equals([entityA, entityB])
        .first();
    } catch {
      return null;
    }
  }

  async function addCoMention(entityA, entityB, pmid) {
    return getDB().coMentions.add({
      entityA,
      entityB,
      pmid,
      discoveredAt: Date.now()
    });
  }

  // ══════════════════════════════════════════════════════════════════════
  // Trends
  // ══════════════════════════════════════════════════════════════════════

  async function getAllTrends() {
    return getDB().trends.toArray();
  }

  // ══════════════════════════════════════════════════════════════════════
  // Citations
  // ══════════════════════════════════════════════════════════════════════

  async function getCitation(pmid) {
    return getDB().citations.get(pmid);
  }

  async function putCitation(record) {
    return getDB().citations.put(record);
  }

  async function getAllCitations() {
    return getDB().citations.toArray();
  }

  // ══════════════════════════════════════════════════════════════════════
  // Discover Cache
  // ══════════════════════════════════════════════════════════════════════

  async function getDiscoverCache(key) {
    try {
      return await getDB().discover_cache.get(key);
    } catch {
      return null;
    }
  }

  async function putDiscoverCache(key, data) {
    return getDB().discover_cache.put({ key, data, fetchedAt: Date.now() });
  }

  // ══════════════════════════════════════════════════════════════════════
  // Backlog / Catch-Up
  // ══════════════════════════════════════════════════════════════════════

  const BACKLOG_MAX_DAYS = 30;

  /**
   * Detect how many days since last check. Returns { gapDays, lastChecked, needsCatchUp }.
   */
  async function detectGap() {
    const lastChecked = await getSetting('lastGlobalCheck');
    if (!lastChecked) return { gapDays: 0, lastChecked: null, needsCatchUp: false };
    const gapMs = Date.now() - lastChecked;
    const gapDays = Math.floor(gapMs / (24 * 60 * 60 * 1000));
    return {
      gapDays,
      lastChecked,
      needsCatchUp: gapDays > 1
    };
  }

  /**
   * Build date windows (7-day chunks) to cover the gap.
   * Returns array of { minDate: 'YYYY/MM/DD', maxDate: 'YYYY/MM/DD' }
   */
  function buildCatchUpWindows(lastCheckedTs) {
    const now = Date.now();
    const gapMs = now - lastCheckedTs;
    const gapDays = Math.min(Math.floor(gapMs / (24 * 60 * 60 * 1000)), BACKLOG_MAX_DAYS);

    const windows = [];
    const windowSize = 7; // days per chunk
    const startDate = new Date(now - gapDays * 24 * 60 * 60 * 1000);

    for (let offset = 0; offset < gapDays; offset += windowSize) {
      const winStart = new Date(startDate.getTime() + offset * 24 * 60 * 60 * 1000);
      const winEndDays = Math.min(offset + windowSize, gapDays);
      const winEnd = new Date(startDate.getTime() + winEndDays * 24 * 60 * 60 * 1000);

      windows.push({
        minDate: formatPubMedDate(winStart),
        maxDate: formatPubMedDate(winEnd)
      });
    }
    return windows;
  }

  function formatPubMedDate(d) {
    return `${d.getFullYear()}/${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')}`;
  }

  /**
   * Get all papers marked as backlog that haven't been dismissed.
   */
  async function getBacklogPapers() {
    const all = await getDB().papers.toArray();
    return all.filter(p => p.backlog && !p.backlogDismissed);
  }

  /**
   * Mark all backlog papers as dismissed (user clicked "Mark all read" or "Dismiss").
   */
  async function dismissBacklog() {
    const backlog = await getBacklogPapers();
    for (const p of backlog) {
      await getDB().papers.update(p.id, { backlogDismissed: true, read: true });
    }
  }

  /**
   * Mark all backlog papers as reviewed (keep unread, just remove backlog banner).
   */
  async function acknowledgeBacklog() {
    const backlog = await getBacklogPapers();
    for (const p of backlog) {
      await getDB().papers.update(p.id, { backlogDismissed: true });
    }
  }

  // ══════════════════════════════════════════════════════════════════════
  // Bulk Operations
  // ══════════════════════════════════════════════════════════════════════

  async function clearAll() {
    const d = getDB();
    await d.watchlist.clear();
    await d.papers.clear();
    await d.coMentions.clear();
    await d.trends.clear();
    await d.settings.clear();
    await d.citations.clear();
    await d.discover_cache.clear();
  }

  // ══════════════════════════════════════════════════════════════════════
  // Public API
  // ══════════════════════════════════════════════════════════════════════

  window.BioKhojStorage = {
    init: initDB,
    getDB,

    // Settings
    getSetting,
    saveSetting,

    // Watchlist
    getWatchlist,
    getActiveWatchlist,
    getWatchlistCount,
    getWatchlistById,
    getWatchlistByTerm,
    addWatchlistEntity,
    updateWatchlistEntity,
    deleteWatchlistEntity,
    bulkAddWatchlist,

    // Papers
    getAllPapers,
    getPaperByPmid,
    getPapersForEntity,
    getUnreadPapersForEntity,
    getSavedPapers,
    addPaper,
    updatePaper,
    bulkAddPapers,

    // Co-Mentions
    getRecentCoMentions,
    findCoMention,
    addCoMention,

    // Trends
    getAllTrends,

    // Citations
    getCitation,
    putCitation,
    getAllCitations,

    // Discover Cache
    getDiscoverCache,
    putDiscoverCache,

    // Backlog / Catch-Up
    BACKLOG_MAX_DAYS,
    detectGap,
    buildCatchUpWindows,
    getBacklogPapers,
    dismissBacklog,
    acknowledgeBacklog,

    // Bulk
    clearAll
  };
})();
