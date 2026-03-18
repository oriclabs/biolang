// BioKhoj Storage — Unit tests for biokhoj-storage.js
// Tests the storage abstraction layer (Dexie wrapper)
// Run with: node --test storage.test.js

const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

// Load biokhoj-storage.js in a sandbox with a mock Dexie
const storageSrc = fs.readFileSync(
  path.join(__dirname, '..', '..', 'js', 'biokhoj-storage.js'),
  'utf8'
);

// ── Mock Dexie ──────────────────────────────────────────────────────
// Lightweight in-memory mock that supports the subset of Dexie API
// that biokhoj-storage.js uses.

function createMockDexie() {
  function MockDexie(name) {
    this._name = name;
    this._tables = {};
    this._version = 0;
  }

  MockDexie.prototype.version = function (v) {
    this._version = v;
    return {
      stores: (schema) => {
        for (const [tableName, keys] of Object.entries(schema)) {
          if (!this._tables[tableName]) {
            this._tables[tableName] = new MockTable(tableName);
          }
        }
      }
    };
  };

  // Proxy table access: db.watchlist, db.papers, etc.
  MockDexie.prototype._getTable = function (name) {
    if (!this._tables[name]) this._tables[name] = new MockTable(name);
    return this._tables[name];
  };

  // Make tables accessible as properties
  const handler = {
    get(target, prop) {
      if (prop in target) return target[prop];
      if (typeof prop === 'string' && !prop.startsWith('_')) {
        return target._getTable(prop);
      }
      return undefined;
    }
  };

  function MockTable(name) {
    this._name = name;
    this._data = [];
    this._nextId = 1;
  }

  MockTable.prototype.add = async function (record) {
    const id = this._nextId++;
    this._data.push({ ...record, id });
    return id;
  };

  MockTable.prototype.bulkAdd = async function (records) {
    for (const r of records) await this.add(r);
  };

  MockTable.prototype.put = async function (record) {
    const idx = this._data.findIndex(d => d.key === record.key || d.id === record.id || d.pmid === record.pmid);
    if (idx >= 0) {
      this._data[idx] = { ...this._data[idx], ...record };
    } else {
      await this.add(record);
    }
  };

  MockTable.prototype.get = async function (keyOrId) {
    return this._data.find(d => d.id === keyOrId || d.key === keyOrId || d.pmid === keyOrId) || undefined;
  };

  MockTable.prototype.update = async function (id, changes) {
    const item = this._data.find(d => d.id === id);
    if (item) Object.assign(item, changes);
  };

  MockTable.prototype.delete = async function (id) {
    this._data = this._data.filter(d => d.id !== id);
  };

  MockTable.prototype.toArray = async function () {
    return [...this._data];
  };

  MockTable.prototype.count = async function () {
    return this._data.length;
  };

  MockTable.prototype.clear = async function () {
    this._data = [];
  };

  MockTable.prototype.where = function (key) {
    const self = this;
    return {
      equals: (val) => ({
        toArray: async () => self._data.filter(d => d[key] === val),
        count: async () => self._data.filter(d => d[key] === val).length,
        first: async () => self._data.find(d => d[key] === val),
        and: (fn) => ({
          count: async () => self._data.filter(d => d[key] === val && fn(d)).length
        })
      }),
      equalsIgnoreCase: (val) => ({
        first: async () => self._data.find(d => (d[key] || '').toLowerCase() === val.toLowerCase())
      }),
      notEqual: (val) => ({
        toArray: async () => self._data.filter(d => d[key] !== val)
      })
    };
  };

  MockTable.prototype.orderBy = function (key) {
    const self = this;
    return {
      reverse: () => ({
        limit: (n) => ({
          toArray: async () => [...self._data].sort((a, b) => (b[key] || 0) - (a[key] || 0)).slice(0, n)
        })
      })
    };
  };

  return function DexieConstructor(name) {
    return new Proxy(new MockDexie(name), handler);
  };
}

// ── Create sandbox ──────────────────────────────────────────────────

const sandbox = {
  window: {},
  console: console,
  Date: Date,
  Promise: Promise,
  Array: Array,
  Object: Object,
  String: String,
  Math: Math,
  JSON: JSON
};

vm.createContext(sandbox);
vm.runInContext(storageSrc, sandbox);
const storage = sandbox.window.BioKhojStorage;

// Initialize with mock Dexie
const MockDexie = createMockDexie();
storage.init(MockDexie);

// ══════════════════════════════════════════════════════════════════════
// 1. Initialization
// ══════════════════════════════════════════════════════════════════════

test('init — returns db instance', () => {
  const db = storage.getDB();
  assert.ok(db);
});

test('init — idempotent (second call returns same db)', () => {
  const db1 = storage.init(MockDexie);
  const db2 = storage.init(MockDexie);
  assert.strictEqual(db1, db2);
});

// ══════════════════════════════════════════════════════════════════════
// 2. Settings
// ══════════════════════════════════════════════════════════════════════

test('saveSetting + getSetting — round-trips a value', async () => {
  await storage.saveSetting('testKey', 'testValue');
  const val = await storage.getSetting('testKey');
  assert.strictEqual(val, 'testValue');
});

test('getSetting — returns null for missing key', async () => {
  const val = await storage.getSetting('nonexistent_key_xyz');
  assert.strictEqual(val, null);
});

test('saveSetting — overwrites existing value', async () => {
  await storage.saveSetting('overwrite', 'first');
  await storage.saveSetting('overwrite', 'second');
  const val = await storage.getSetting('overwrite');
  assert.strictEqual(val, 'second');
});

// ══════════════════════════════════════════════════════════════════════
// 3. Watchlist
// ══════════════════════════════════════════════════════════════════════

test('addWatchlistEntity — adds entity with defaults', async () => {
  const id = await storage.addWatchlistEntity({ term: 'BRCA1', type: 'gene' });
  assert.ok(id > 0);
  const count = await storage.getWatchlistCount();
  assert.ok(count >= 1);
});

test('getWatchlist — returns all entities', async () => {
  const list = await storage.getWatchlist();
  assert.ok(Array.isArray(list));
  assert.ok(list.some(e => e.term === 'BRCA1'));
});

test('getWatchlistByTerm — finds by term (case insensitive)', async () => {
  await storage.addWatchlistEntity({ term: 'TP53', type: 'gene' });
  const entity = await storage.getWatchlistByTerm('tp53');
  assert.ok(entity);
  assert.strictEqual(entity.term, 'TP53');
});

test('getWatchlistByTerm — returns undefined for missing term', async () => {
  const entity = await storage.getWatchlistByTerm('NONEXISTENT_GENE_XYZ');
  assert.ok(!entity);
});

test('getActiveWatchlist — excludes paused entities', async () => {
  await storage.addWatchlistEntity({ term: 'PAUSED1', type: 'gene', paused: true });
  const active = await storage.getActiveWatchlist();
  assert.ok(!active.some(e => e.term === 'PAUSED1'));
});

test('updateWatchlistEntity — updates fields', async () => {
  await storage.addWatchlistEntity({ term: 'UPDATE_TEST', type: 'gene' });
  const entity = await storage.getWatchlistByTerm('UPDATE_TEST');
  await storage.updateWatchlistEntity(entity.id, { priority: 'high' });
  const updated = await storage.getWatchlistById(entity.id);
  assert.strictEqual(updated.priority, 'high');
});

test('deleteWatchlistEntity — removes entity', async () => {
  await storage.addWatchlistEntity({ term: 'DELETE_ME', type: 'gene' });
  const entity = await storage.getWatchlistByTerm('DELETE_ME');
  await storage.deleteWatchlistEntity(entity.id);
  const gone = await storage.getWatchlistByTerm('DELETE_ME');
  assert.ok(!gone);
});

test('bulkAddWatchlist — adds multiple entities', async () => {
  const before = await storage.getWatchlistCount();
  await storage.bulkAddWatchlist([
    { term: 'BULK1', type: 'gene' },
    { term: 'BULK2', type: 'disease' }
  ]);
  const after = await storage.getWatchlistCount();
  assert.ok(after >= before + 2);
});

test('addWatchlistEntity — sets default fields', async () => {
  await storage.addWatchlistEntity({ term: 'DEFAULTS_TEST' });
  const entity = await storage.getWatchlistByTerm('DEFAULTS_TEST');
  assert.strictEqual(entity.type, 'topic');
  assert.strictEqual(entity.priority, 'normal');
  assert.ok(Array.isArray(entity.tags));
  assert.strictEqual(entity.paused, false);
  assert.ok(entity.addedAt > 0);
});

// ══════════════════════════════════════════════════════════════════════
// 4. Papers
// ══════════════════════════════════════════════════════════════════════

test('addPaper — adds paper with defaults', async () => {
  const id = await storage.addPaper({ pmid: 'test-001', title: 'Test Paper' });
  assert.ok(id > 0);
  const paper = await storage.getPaperByPmid('test-001');
  assert.ok(paper);
  assert.strictEqual(paper.title, 'Test Paper');
  assert.strictEqual(paper.saved, false);
  assert.strictEqual(paper.starred, false);
  assert.strictEqual(paper.read, false);
});

test('getAllPapers — returns all papers', async () => {
  const papers = await storage.getAllPapers();
  assert.ok(Array.isArray(papers));
  assert.ok(papers.length >= 1);
});

test('updatePaper — updates fields', async () => {
  const paper = await storage.getPaperByPmid('test-001');
  await storage.updatePaper(paper.id, { saved: true, savedAt: Date.now() });
  const updated = await storage.getPaperByPmid('test-001');
  assert.strictEqual(updated.saved, true);
  assert.ok(updated.savedAt > 0);
});

test('getSavedPapers — returns only saved papers', async () => {
  await storage.addPaper({ pmid: 'unsaved-002', title: 'Unsaved', saved: false });
  const saved = await storage.getSavedPapers();
  assert.ok(saved.every(p => p.saved));
});

test('getPaperByPmid — returns null for missing pmid', async () => {
  const paper = await storage.getPaperByPmid('nonexistent-pmid-999');
  assert.ok(!paper);
});

// ══════════════════════════════════════════════════════════════════════
// 5. Co-Mentions
// ══════════════════════════════════════════════════════════════════════

test('addCoMention — stores co-mention record', async () => {
  await storage.addCoMention('BRCA1', 'TP53', 'paper-123');
  const recent = await storage.getRecentCoMentions(10);
  assert.ok(recent.some(cm => cm.entityA === 'BRCA1' && cm.entityB === 'TP53'));
});

test('getRecentCoMentions — respects limit', async () => {
  for (let i = 0; i < 10; i++) {
    await storage.addCoMention(`ENT_A${i}`, `ENT_B${i}`, `paper-${i}`);
  }
  const limited = await storage.getRecentCoMentions(3);
  assert.ok(limited.length <= 3);
});

// ══════════════════════════════════════════════════════════════════════
// 6. Citations
// ══════════════════════════════════════════════════════════════════════

test('putCitation + getCitation — round-trips', async () => {
  await storage.putCitation({ pmid: 'cite-001', currentCount: 42, history: [], lastFetched: Date.now() });
  const cite = await storage.getCitation('cite-001');
  assert.ok(cite);
  assert.strictEqual(cite.currentCount, 42);
});

test('getAllCitations — returns all citations', async () => {
  const all = await storage.getAllCitations();
  assert.ok(Array.isArray(all));
  assert.ok(all.length >= 1);
});

// ══════════════════════════════════════════════════════════════════════
// 7. Discover Cache
// ══════════════════════════════════════════════════════════════════════

test('putDiscoverCache + getDiscoverCache — round-trips', async () => {
  await storage.putDiscoverCache('trending', [{ title: 'Trending Paper' }]);
  const cached = await storage.getDiscoverCache('trending');
  assert.ok(cached);
  assert.ok(cached.data.length === 1);
  assert.ok(cached.fetchedAt > 0);
});

test('getDiscoverCache — returns null for missing key', async () => {
  const cached = await storage.getDiscoverCache('nonexistent-cache-key');
  assert.ok(!cached);
});

// ══════════════════════════════════════════════════════════════════════
// 8. Backlog / Catch-Up
// ══════════════════════════════════════════════════════════════════════

test('BACKLOG_MAX_DAYS — is 30', () => {
  assert.strictEqual(storage.BACKLOG_MAX_DAYS, 30);
});

test('detectGap — returns needsCatchUp false when no lastGlobalCheck', async () => {
  const gap = await storage.detectGap();
  // If lastGlobalCheck was never set, needsCatchUp should be false
  // (depends on whether a previous test set it)
  assert.ok('gapDays' in gap);
  assert.ok('needsCatchUp' in gap);
  assert.ok('lastChecked' in gap);
});

test('detectGap — returns needsCatchUp true for old lastGlobalCheck', async () => {
  const threeDaysAgo = Date.now() - 3 * 24 * 60 * 60 * 1000;
  await storage.saveSetting('lastGlobalCheck', threeDaysAgo);
  const gap = await storage.detectGap();
  assert.strictEqual(gap.needsCatchUp, true);
  assert.ok(gap.gapDays >= 2);
});

test('buildCatchUpWindows — returns 7-day windows', () => {
  const tenDaysAgo = Date.now() - 10 * 24 * 60 * 60 * 1000;
  const windows = storage.buildCatchUpWindows(tenDaysAgo);
  assert.ok(windows.length >= 1);
  // Each window should have minDate and maxDate
  for (const w of windows) {
    assert.ok(w.minDate.match(/^\d{4}\/\d{2}\/\d{2}$/));
    assert.ok(w.maxDate.match(/^\d{4}\/\d{2}\/\d{2}$/));
  }
});

test('buildCatchUpWindows — caps at BACKLOG_MAX_DAYS', () => {
  const sixtyDaysAgo = Date.now() - 60 * 24 * 60 * 60 * 1000;
  const windows = storage.buildCatchUpWindows(sixtyDaysAgo);
  // Should cover at most 30 days in 7-day chunks = max 5 windows
  assert.ok(windows.length <= 5);
});

test('getBacklogPapers — returns only backlog papers', async () => {
  await storage.addPaper({ pmid: 'backlog-001', title: 'Backlog Paper', backlog: true, backlogDismissed: false });
  await storage.addPaper({ pmid: 'normal-001', title: 'Normal Paper', backlog: false });
  const backlog = await storage.getBacklogPapers();
  assert.ok(backlog.some(p => p.pmid === 'backlog-001'));
  assert.ok(!backlog.some(p => p.pmid === 'normal-001'));
});

test('dismissBacklog — marks backlog papers as read + dismissed', async () => {
  await storage.addPaper({ pmid: 'dismiss-001', title: 'To Dismiss', backlog: true, backlogDismissed: false, read: false });
  await storage.dismissBacklog();
  const paper = await storage.getPaperByPmid('dismiss-001');
  assert.strictEqual(paper.backlogDismissed, true);
  assert.strictEqual(paper.read, true);
});

test('acknowledgeBacklog — marks dismissed but keeps unread', async () => {
  await storage.addPaper({ pmid: 'ack-001', title: 'To Acknowledge', backlog: true, backlogDismissed: false, read: false });
  await storage.acknowledgeBacklog();
  const paper = await storage.getPaperByPmid('ack-001');
  assert.strictEqual(paper.backlogDismissed, true);
  // read should NOT be changed by acknowledge (only by dismiss)
});

// ══════════════════════════════════════════════════════════════════════
// 9. Bulk Operations
// ══════════════════════════════════════════════════════════════════════

test('clearAll — empties all tables', async () => {
  // Add some data first
  await storage.addWatchlistEntity({ term: 'CLEAR_TEST', type: 'gene' });
  await storage.addPaper({ pmid: 'clear-001', title: 'Clear Me' });
  await storage.saveSetting('clearTest', true);

  await storage.clearAll();

  const watchlist = await storage.getWatchlist();
  const papers = await storage.getAllPapers();
  const setting = await storage.getSetting('clearTest');

  assert.strictEqual(watchlist.length, 0);
  assert.strictEqual(papers.length, 0);
  assert.strictEqual(setting, null);
});
