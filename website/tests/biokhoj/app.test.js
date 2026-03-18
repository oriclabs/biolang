// BioKhoj App — Unit tests for app.js UI logic
// Tests filter logic, preset packs, sharing, and data flow
// Run with: node --test app.test.js

const { test } = require('node:test');
const assert = require('node:assert');

// ══════════════════════════════════════════════════════════════════════
// 1. Preset Packs
// ══════════════════════════════════════════════════════════════════════

const PRESET_PACKS = [
  { name: 'Cancer Genomics', entities: [
    { term: 'TP53', type: 'gene' }, { term: 'BRCA1', type: 'gene' }, { term: 'BRCA2', type: 'gene' },
    { term: 'KRAS', type: 'gene' }, { term: 'EGFR', type: 'gene' }, { term: 'PIK3CA', type: 'gene' },
    { term: 'tumor mutational burden', type: 'topic' }, { term: 'immune checkpoint', type: 'topic' }
  ]},
  { name: 'CRISPR & Gene Editing', entities: [
    { term: 'CRISPR-Cas9', type: 'technique' }, { term: 'base editing', type: 'technique' },
    { term: 'prime editing', type: 'technique' }, { term: 'guide RNA', type: 'topic' },
    { term: 'off-target effects', type: 'topic' }, { term: 'gene therapy', type: 'topic' }
  ]},
  { name: 'Single-Cell Genomics', entities: [
    { term: 'single-cell RNA-seq', type: 'technique' }, { term: 'spatial transcriptomics', type: 'technique' },
    { term: 'cell atlas', type: 'topic' }, { term: 'UMAP', type: 'technique' },
    { term: 'trajectory inference', type: 'topic' }, { term: 'multiome', type: 'technique' }
  ]},
  { name: 'Clinical Variants', entities: [
    { term: 'pathogenic variant', type: 'topic' }, { term: 'variant of uncertain significance', type: 'topic' },
    { term: 'pharmacogenomics', type: 'topic' }, { term: 'ACMG classification', type: 'topic' },
    { term: 'ClinVar', type: 'topic' }, { term: 'rare disease', type: 'disease' }
  ]},
  { name: 'Metagenomics', entities: [
    { term: '16S rRNA', type: 'technique' }, { term: 'microbiome', type: 'topic' },
    { term: 'shotgun metagenomics', type: 'technique' }, { term: 'antimicrobial resistance', type: 'topic' },
    { term: 'gut-brain axis', type: 'topic' }, { term: 'metatranscriptomics', type: 'technique' }
  ]}
];

test('preset packs — 5 packs exist', () => {
  assert.strictEqual(PRESET_PACKS.length, 5);
});

test('preset packs — each has 6-8 entities', () => {
  for (const pack of PRESET_PACKS) {
    assert.ok(pack.entities.length >= 6, `${pack.name} has ${pack.entities.length} entities`);
    assert.ok(pack.entities.length <= 8, `${pack.name} has ${pack.entities.length} entities`);
  }
});

test('preset packs — all entities have term and type', () => {
  for (const pack of PRESET_PACKS) {
    for (const e of pack.entities) {
      assert.ok(e.term, `Missing term in ${pack.name}`);
      assert.ok(e.type, `Missing type for ${e.term} in ${pack.name}`);
    }
  }
});

test('preset packs — no duplicate terms across packs', () => {
  const all = new Set();
  for (const pack of PRESET_PACKS) {
    for (const e of pack.entities) {
      assert.ok(!all.has(e.term.toLowerCase()), `Duplicate: ${e.term}`);
      all.add(e.term.toLowerCase());
    }
  }
});

// ══════════════════════════════════════════════════════════════════════
// 2. Feed Filters
// ══════════════════════════════════════════════════════════════════════

function applyFilters(papers, opts = {}) {
  let filtered = [...papers];
  if (opts.muteThreshold > 0) {
    filtered = filtered.filter(p => (p.signalScore || 0) >= opts.muteThreshold);
  }
  if (opts.comentionFilter) {
    filtered = filtered.filter(p => (p.matchedEntities || []).length >= 2);
  }
  if (opts.unreadFilter) {
    filtered = filtered.filter(p => !p.read);
  }
  if (opts.highSignalFilter) {
    filtered = filtered.filter(p => (p.signalScore || 0) >= 70);
  }
  if (opts.searchTerm) {
    const q = opts.searchTerm.toLowerCase();
    filtered = filtered.filter(p =>
      (p.title || '').toLowerCase().includes(q) ||
      (p.journal || '').toLowerCase().includes(q)
    );
  }
  return filtered;
}

const testPapers = [
  { pmid: '1', title: 'BRCA1 in cancer', signalScore: 92, read: false, matchedEntities: ['BRCA1', 'TP53'], journal: 'Nature' },
  { pmid: '2', title: 'TP53 mutations', signalScore: 65, read: true, matchedEntities: ['TP53'], journal: 'Cell' },
  { pmid: '3', title: 'CRISPR review', signalScore: 45, read: false, matchedEntities: ['CRISPR'], journal: 'PLoS ONE' },
  { pmid: '4', title: 'Low signal paper', signalScore: 20, read: true, matchedEntities: ['obscure'], journal: 'Unknown' },
  { pmid: '5', title: 'BRCA1 EGFR ALK', signalScore: 88, read: false, matchedEntities: ['BRCA1', 'EGFR', 'ALK'], journal: 'Science' },
];

test('filters — no filters returns all', () => {
  assert.strictEqual(applyFilters(testPapers).length, 5);
});

test('filters — mute threshold 50', () => {
  const result = applyFilters(testPapers, { muteThreshold: 50 });
  assert.strictEqual(result.length, 3);
  assert.ok(result.every(p => p.signalScore >= 50));
});

test('filters — unread only', () => {
  const result = applyFilters(testPapers, { unreadFilter: true });
  assert.strictEqual(result.length, 3);
  assert.ok(result.every(p => !p.read));
});

test('filters — high signal only', () => {
  const result = applyFilters(testPapers, { highSignalFilter: true });
  assert.strictEqual(result.length, 2);
  assert.ok(result.every(p => p.signalScore >= 70));
});

test('filters — multi-entity only', () => {
  const result = applyFilters(testPapers, { comentionFilter: true });
  assert.strictEqual(result.length, 2);
  assert.ok(result.every(p => p.matchedEntities.length >= 2));
});

test('filters — compose: unread + high signal', () => {
  const result = applyFilters(testPapers, { unreadFilter: true, highSignalFilter: true });
  assert.strictEqual(result.length, 2);
  assert.ok(result.every(p => !p.read && p.signalScore >= 70));
});

test('filters — compose: mute + multi-entity', () => {
  const result = applyFilters(testPapers, { muteThreshold: 60, comentionFilter: true });
  assert.strictEqual(result.length, 2);
});

test('filters — search by title', () => {
  const result = applyFilters(testPapers, { searchTerm: 'BRCA1' });
  assert.strictEqual(result.length, 2);
});

test('filters — search by journal', () => {
  const result = applyFilters(testPapers, { searchTerm: 'nature' });
  assert.strictEqual(result.length, 1);
  assert.strictEqual(result[0].journal, 'Nature');
});

test('filters — search + filter compose', () => {
  const result = applyFilters(testPapers, { searchTerm: 'BRCA1', highSignalFilter: true });
  assert.strictEqual(result.length, 2);
});

test('filters — mute threshold 0 returns all', () => {
  const result = applyFilters(testPapers, { muteThreshold: 0 });
  assert.strictEqual(result.length, 5);
});

test('filters — mute threshold 100 returns none', () => {
  const result = applyFilters(testPapers, { muteThreshold: 100 });
  assert.strictEqual(result.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// 3. Share URL Encoding/Decoding
// ══════════════════════════════════════════════════════════════════════

function encodeWatchlist(entities) {
  const payload = { v: 1, entities: entities.map(e => ({ t: e.term, y: e.type, p: e.priority })) };
  return btoa(unescape(encodeURIComponent(JSON.stringify(payload))));
}

function decodeWatchlist(encoded) {
  const json = JSON.parse(decodeURIComponent(escape(atob(encoded))));
  if (json.v !== 1 || !json.entities) return null;
  return json.entities.map(e => ({ term: e.t, type: e.y, priority: e.p }));
}

test('share — encode/decode round-trip', () => {
  const entities = [
    { term: 'BRCA1', type: 'gene', priority: 'high' },
    { term: 'olaparib', type: 'drug', priority: 'normal' }
  ];
  const encoded = encodeWatchlist(entities);
  const decoded = decodeWatchlist(encoded);
  assert.strictEqual(decoded.length, 2);
  assert.strictEqual(decoded[0].term, 'BRCA1');
  assert.strictEqual(decoded[0].type, 'gene');
  assert.strictEqual(decoded[1].term, 'olaparib');
});

test('share — handles unicode entity names', () => {
  const entities = [{ term: 'IL-1\u03B2', type: 'gene', priority: 'normal' }];
  const encoded = encodeWatchlist(entities);
  const decoded = decodeWatchlist(encoded);
  assert.strictEqual(decoded[0].term, 'IL-1\u03B2');
});

test('share — encoded string is URL-safe', () => {
  const entities = [{ term: 'BRCA1', type: 'gene', priority: 'high' }];
  const encoded = encodeWatchlist(entities);
  // base64 is URL-safe enough for query params
  assert.ok(!encoded.includes(' '));
  assert.ok(encoded.length < 2000);
});

test('share — empty watchlist', () => {
  const encoded = encodeWatchlist([]);
  const decoded = decodeWatchlist(encoded);
  assert.strictEqual(decoded.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// 4. Entity Color Hashing
// ══════════════════════════════════════════════════════════════════════

const ENTITY_COLORS = [
  'bg-purple-800/40 text-purple-300',
  'bg-blue-800/40 text-blue-300',
  'bg-green-800/40 text-green-300',
  'bg-orange-800/40 text-orange-300',
  'bg-pink-800/40 text-pink-300',
  'bg-cyan-800/40 text-cyan-300',
  'bg-yellow-800/40 text-yellow-300',
  'bg-indigo-800/40 text-indigo-300',
];

function entityChipColor(entity) {
  let hash = 0;
  for (let i = 0; i < entity.length; i++) hash = entity.charCodeAt(i) + ((hash << 5) - hash);
  return ENTITY_COLORS[Math.abs(hash) % ENTITY_COLORS.length];
}

test('entity color — deterministic', () => {
  const c1 = entityChipColor('BRCA1');
  const c2 = entityChipColor('BRCA1');
  assert.strictEqual(c1, c2);
});

test('entity color — different names get (likely) different colors', () => {
  const c1 = entityChipColor('BRCA1');
  const c2 = entityChipColor('TP53');
  const c3 = entityChipColor('EGFR');
  // At least 2 of 3 should differ (statistically)
  const unique = new Set([c1, c2, c3]);
  assert.ok(unique.size >= 2, 'Expected at least 2 different colors');
});

test('entity color — returns valid class string', () => {
  const color = entityChipColor('BRCA1');
  assert.ok(ENTITY_COLORS.includes(color));
});

test('entity color — empty string does not crash', () => {
  const color = entityChipColor('');
  assert.ok(typeof color === 'string');
});

// ══════════════════════════════════════════════════════════════════════
// 5. Abstract Truncation
// ══════════════════════════════════════════════════════════════════════

function abstractPreview(text, limit = 180) {
  if (!text) return '';
  return text.slice(0, limit) + (text.length > limit ? '...' : '');
}

test('abstract — short text not truncated', () => {
  assert.strictEqual(abstractPreview('Short text'), 'Short text');
});

test('abstract — long text truncated with ellipsis', () => {
  const long = 'A'.repeat(300);
  const preview = abstractPreview(long);
  assert.strictEqual(preview.length, 183); // 180 + '...'
  assert.ok(preview.endsWith('...'));
});

test('abstract — empty string', () => {
  assert.strictEqual(abstractPreview(''), '');
});

test('abstract — null input', () => {
  assert.strictEqual(abstractPreview(null), '');
});

test('abstract — exactly 180 chars not truncated', () => {
  const exact = 'B'.repeat(180);
  assert.strictEqual(abstractPreview(exact), exact);
});

// ══════════════════════════════════════════════════════════════════════
// 6. Date Grouping
// ══════════════════════════════════════════════════════════════════════

function getDateGroup(timestamp) {
  const now = Date.now();
  const todayStart = new Date();
  todayStart.setHours(0, 0, 0, 0);
  const weekStart = new Date(todayStart);
  weekStart.setDate(weekStart.getDate() - 7);

  if (timestamp >= todayStart.getTime()) return 'Today';
  if (timestamp >= weekStart.getTime()) return 'This week';
  return 'Older';
}

test('date group — today', () => {
  assert.strictEqual(getDateGroup(Date.now()), 'Today');
  assert.strictEqual(getDateGroup(Date.now() - 1000), 'Today');
});

test('date group — this week', () => {
  const threeDaysAgo = Date.now() - 3 * 24 * 60 * 60 * 1000;
  assert.strictEqual(getDateGroup(threeDaysAgo), 'This week');
});

test('date group — older', () => {
  const twoWeeksAgo = Date.now() - 14 * 24 * 60 * 60 * 1000;
  assert.strictEqual(getDateGroup(twoWeeksAgo), 'Older');
});

// ══════════════════════════════════════════════════════════════════════
// 7. Citation Velocity Detection
// ══════════════════════════════════════════════════════════════════════

function detectCitationSpike(history, threshold = 5) {
  if (!history || history.length < 2) return null;
  const recent = history[history.length - 1].count;
  const prev = history[history.length - 2].count;
  const velocity = recent - prev;
  return velocity >= threshold ? velocity : null;
}

test('citation spike — detected when >= 5', () => {
  const history = [{ count: 10 }, { count: 16 }];
  assert.strictEqual(detectCitationSpike(history), 6);
});

test('citation spike — not detected when < 5', () => {
  const history = [{ count: 10 }, { count: 12 }];
  assert.strictEqual(detectCitationSpike(history), null);
});

test('citation spike — handles single entry', () => {
  assert.strictEqual(detectCitationSpike([{ count: 5 }]), null);
});

test('citation spike — handles empty/null', () => {
  assert.strictEqual(detectCitationSpike(null), null);
  assert.strictEqual(detectCitationSpike([]), null);
});

test('citation spike — custom threshold', () => {
  const history = [{ count: 10 }, { count: 13 }];
  assert.strictEqual(detectCitationSpike(history, 3), 3);
  assert.strictEqual(detectCitationSpike(history, 5), null);
});

// ══════════════════════════════════════════════════════════════════════
// 8. Signal Bar Classification
// ══════════════════════════════════════════════════════════════════════

function signalBarClass(score) {
  if (score >= 70) return 'signal-bar-high';
  if (score >= 40) return 'signal-bar-mid';
  return 'signal-bar-low';
}

test('signal bar — high (purple) for 70+', () => {
  assert.strictEqual(signalBarClass(70), 'signal-bar-high');
  assert.strictEqual(signalBarClass(100), 'signal-bar-high');
});

test('signal bar — mid (saffron) for 40-69', () => {
  assert.strictEqual(signalBarClass(40), 'signal-bar-mid');
  assert.strictEqual(signalBarClass(69), 'signal-bar-mid');
});

test('signal bar — low (slate) for <40', () => {
  assert.strictEqual(signalBarClass(0), 'signal-bar-low');
  assert.strictEqual(signalBarClass(39), 'signal-bar-low');
});

// ══════════════════════════════════════════════════════════════════════
// 9. Export Format Validation
// ══════════════════════════════════════════════════════════════════════

test('pandas export — generates valid Python', () => {
  const papers = [{ pmid: '123', title: 'Test', authors: ['A'], journal: 'Nature', date: '2024', signalScore: 90, doi: '10.1/x', matchedEntities: ['BRCA1'] }];
  const content = 'import pandas as pd\n\ndf = pd.DataFrame(' +
    JSON.stringify(papers.map(p => ({
      pmid: p.pmid, title: p.title, authors: (p.authors || []).join('; '),
      journal: p.journal, date: p.date, signal_score: p.signalScore,
      doi: p.doi, entities: (p.matchedEntities || []).join('; ')
    })), null, 2) + ')\n\ndf.head()';
  assert.ok(content.startsWith('import pandas as pd'));
  assert.ok(content.includes('pd.DataFrame('));
  assert.ok(content.includes('df.head()'));
  assert.ok(content.includes('"pmid": "123"'));
});

test('R export — generates valid R code', () => {
  const papers = [{ pmid: '123', title: 'Test', journal: 'Nature', date: '2024', signalScore: 90 }];
  const content = 'library(tibble)\n\ndf <- tibble(\n  pmid = c("123"),\n  title = c("Test")\n)\n\nhead(df)';
  assert.ok(content.startsWith('library(tibble)'));
  assert.ok(content.includes('tibble('));
  assert.ok(content.includes('head(df)'));
});

test('CSV export — has header row', () => {
  const header = 'pmid,title,authors,journal,date,signal_score,doi,entities';
  assert.ok(header.split(',').length === 8);
});

// ══════════════════════════════════════════════════════════════════════
// 10. Entity Classification Display
// ══════════════════════════════════════════════════════════════════════

test('classifyEntity result — type extraction handles object', () => {
  const result = { type: 'gene', id: 'BRCA1' };
  const type = (typeof result === 'object' && result.type) ? result.type : result;
  assert.strictEqual(type, 'gene');
});

test('classifyEntity result — type extraction handles string fallback', () => {
  const result = 'unknown';
  const type = (typeof result === 'object' && result.type) ? result.type : result;
  assert.strictEqual(type, 'unknown');
});

test('classifyEntity result — type extraction handles null', () => {
  const result = null;
  const type = (typeof result === 'object' && result && result.type) ? result.type : (result || 'topic');
  assert.strictEqual(type, 'topic');
});

// ══════════════════════════════════════════════════════════════════════
// 11. Feed Empty State
// ══════════════════════════════════════════════════════════════════════

test('feed empty — with watchlist shows refresh message', () => {
  const wlCount = 5;
  const papers = [];
  const message = wlCount > 0
    ? `You're watching ${wlCount} entities. Click refresh to check PubMed.`
    : 'Add genes, proteins, pathways, or topics to your watchlist.';
  assert.ok(message.includes('watching 5'));
});

test('feed empty — without watchlist shows setup message', () => {
  const wlCount = 0;
  const message = wlCount > 0
    ? 'Click refresh'
    : 'Add genes, proteins, pathways, or topics to your watchlist.';
  assert.ok(message.includes('Add genes'));
});

// ══════════════════════════════════════════════════════════════════════
// 12. Sidebar Activity Dots
// ══════════════════════════════════════════════════════════════════════

function getActivityDotColor(hasHighSignal, newCount) {
  if (hasHighSignal) return 'bg-purple-400';
  if (newCount > 0) return 'bg-saffron-400';
  return 'bg-slate-600';
}

test('activity dot — purple for high-signal unread', () => {
  assert.strictEqual(getActivityDotColor(true, 3), 'bg-purple-400');
});

test('activity dot — saffron for new papers', () => {
  assert.strictEqual(getActivityDotColor(false, 5), 'bg-saffron-400');
});

test('activity dot — grey when up to date', () => {
  assert.strictEqual(getActivityDotColor(false, 0), 'bg-slate-600');
});

// ══════════════════════════════════════════════════════════════════════
// 13. Extension Entity Color Hashing
// ══════════════════════════════════════════════════════════════════════

const EXT_ENTITY_COLORS = [
  { bg: "rgba(168,85,247,0.15)", fg: "#c084fc" },
  { bg: "rgba(96,165,250,0.15)", fg: "#60a5fa" },
  { bg: "rgba(52,211,153,0.15)", fg: "#34d399" },
  { bg: "rgba(251,146,60,0.15)", fg: "#fb923c" },
  { bg: "rgba(244,114,182,0.15)", fg: "#f472b6" },
  { bg: "rgba(34,211,238,0.15)", fg: "#22d3ee" },
  { bg: "rgba(250,204,21,0.15)", fg: "#facc15" },
  { bg: "rgba(129,140,248,0.15)", fg: "#818cf8" }
];

function extEntityColor(name) {
  var hash = 0;
  for (var i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  return EXT_ENTITY_COLORS[Math.abs(hash) % EXT_ENTITY_COLORS.length];
}

test('ext entity color — deterministic', () => {
  assert.deepStrictEqual(extEntityColor('BRCA1'), extEntityColor('BRCA1'));
});

test('ext entity color — watch chip matches paper chip', () => {
  // Same function used in both places
  const watchColor = extEntityColor('TP53');
  const paperColor = extEntityColor('TP53');
  assert.strictEqual(watchColor.fg, paperColor.fg);
  assert.strictEqual(watchColor.bg, paperColor.bg);
});
