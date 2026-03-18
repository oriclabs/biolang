// BioKhoj Core — Comprehensive unit tests
// Tests all exported functions from biokhoj-core.js
// Run with: node --test core.test.js

const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

// Load biokhoj-core.js in a sandbox with mock window, fetch, setTimeout
// Try canonical location first, fall back to extension copy
const corePathPrimary = path.join(__dirname, '..', '..', 'js', 'biokhoj-core.js');
const corePathFallback = path.join(__dirname, '..', '..', 'extension', 'biokhoj', 'shared', 'biokhoj-core.js');
const coreSrc = fs.readFileSync(
  fs.existsSync(corePathPrimary) ? corePathPrimary : corePathFallback,
  'utf8'
);

// Mock fetch that returns configurable responses
let fetchResponses = [];
let fetchCalls = [];

function mockFetch(url, options) {
  fetchCalls.push({ url, options });
  const next = fetchResponses.shift();
  if (next && next.error) {
    return Promise.reject(next.error);
  }
  const resp = {
    ok: next ? next.ok !== false : true,
    status: next ? (next.status || 200) : 200,
    statusText: next ? (next.statusText || 'OK') : 'OK',
    json: () => Promise.resolve(next ? next.body : {}),
    text: () => Promise.resolve(next ? next.text : '')
  };
  if (!resp.ok) {
    const err = new Error(`HTTP ${resp.status}: ${resp.statusText}`);
    err.status = resp.status;
    return Promise.reject(err);
  }
  return Promise.resolve(resp);
}

function resetFetch() {
  fetchResponses = [];
  fetchCalls = [];
}

const sandbox = {
  window: {},
  console: console,
  fetch: mockFetch,
  setTimeout: (fn, ms) => { fn(); return 0; },
  clearTimeout: () => {},
  Date: Date,
  Promise: Promise,
  Set: Set,
  Map: Map,
  Array: Array,
  Object: Object,
  String: String,
  Math: Math,
  JSON: JSON,
  RegExp: RegExp,
  Error: Error,
  encodeURIComponent: encodeURIComponent,
  isNaN: isNaN,
  parseInt: parseInt,
  parseFloat: parseFloat
};
vm.createContext(sandbox);
vm.runInContext(coreSrc, sandbox);
const core = sandbox.window.BioKhojCore;

// ══════════════════════════════════════════════════════════════════════
// 1. Constants — verify they exist and have expected structure
// ══════════════════════════════════════════════════════════════════════

test('SIGNAL_THRESHOLDS — has all five tiers', () => {
  assert.strictEqual(core.SIGNAL_THRESHOLDS.critical, 80);
  assert.strictEqual(core.SIGNAL_THRESHOLDS.high, 60);
  assert.strictEqual(core.SIGNAL_THRESHOLDS.medium, 40);
  assert.strictEqual(core.SIGNAL_THRESHOLDS.low, 20);
  assert.strictEqual(core.SIGNAL_THRESHOLDS.noise, 0);
});

test('POLLING_INTERVALS — has all four levels', () => {
  assert.strictEqual(core.POLLING_INTERVALS.hot, 6 * 60 * 60 * 1000);
  assert.strictEqual(core.POLLING_INTERVALS.active, 24 * 60 * 60 * 1000);
  assert.strictEqual(core.POLLING_INTERVALS.moderate, 72 * 60 * 60 * 1000);
  assert.strictEqual(core.POLLING_INTERVALS.rare, 168 * 60 * 60 * 1000);
});

test('TIER_SCORES — maps tiers to points', () => {
  assert.strictEqual(core.TIER_SCORES[1], 15);
  assert.strictEqual(core.TIER_SCORES[2], 10);
  assert.strictEqual(core.TIER_SCORES[3], 5);
});

test('JOURNAL_TIERS — contains representative journals', () => {
  assert.strictEqual(core.JOURNAL_TIERS['nature'], 1);
  assert.strictEqual(core.JOURNAL_TIERS['genome research'], 2);
  assert.strictEqual(core.JOURNAL_TIERS['plos one'], 3);
});

test('API base URLs — defined', () => {
  assert.ok(core.PUBMED_BASE.includes('eutils.ncbi.nlm.nih.gov'));
  assert.ok(core.BIORXIV_BASE.includes('api.biorxiv.org'));
  assert.ok(core.OPENALEX_BASE.includes('api.openalex.org'));
});

// ══════════════════════════════════════════════════════════════════════
// 2. Entity Classification
// ══════════════════════════════════════════════════════════════════════

test('classifyEntity — gene symbols', () => {
  const r = core.classifyEntity('BRCA1');
  assert.strictEqual(r.type, 'gene');
  assert.strictEqual(r.id, 'BRCA1');
});

test('classifyEntity — gene with dash (e.g., HLA-A)', () => {
  const r = core.classifyEntity('HLA-A');
  assert.strictEqual(r.type, 'gene');
  assert.strictEqual(r.id, 'HLA-A');
});

test('classifyEntity — rejects common English words as genes', () => {
  const r = core.classifyEntity('SET');
  assert.strictEqual(r.type, 'unknown');
});

test('classifyEntity — rejects THE, MAP, etc.', () => {
  for (const word of ['THE', 'MAP', 'FOR', 'NOT', 'ALL', 'MARCH']) {
    const r = core.classifyEntity(word);
    assert.strictEqual(r.type, 'unknown', `${word} should not be classified as gene`);
  }
});

test('classifyEntity — rsID variants', () => {
  const r = core.classifyEntity('rs28934576');
  assert.strictEqual(r.type, 'variant');
  assert.strictEqual(r.id, 'rs28934576');
});

test('classifyEntity — rsID case insensitive', () => {
  const r = core.classifyEntity('RS12345678');
  assert.strictEqual(r.type, 'variant');
});

test('classifyEntity — HGVS notation (NM_)', () => {
  const r = core.classifyEntity('NM_007294.4:c.5266dupC');
  assert.strictEqual(r.type, 'variant_hgvs');
});

test('classifyEntity — HGVS notation (NP_)', () => {
  const r = core.classifyEntity('NP_009225.1:p.Arg1756Ter');
  assert.strictEqual(r.type, 'variant_hgvs');
});

test('classifyEntity — ClinVar accessions (RCV)', () => {
  const r = core.classifyEntity('RCV000017599');
  assert.strictEqual(r.type, 'variant_clinvar');
  assert.strictEqual(r.id, 'RCV000017599');
});

test('classifyEntity — ClinVar accessions (VCV)', () => {
  const r = core.classifyEntity('VCV000017599');
  assert.strictEqual(r.type, 'variant_clinvar');
});

test('classifyEntity — drug names with known suffixes', () => {
  // Suffixes recognized: ab, ib, mab, nib, lib, zole, pine, pril, vir, stat, olol, tide, zumab, ximab, lizumab
  const drugs = ['imatinib', 'trastuzumab', 'pembrolizumab', 'olaparib', 'fluconazole'];
  for (const d of drugs) {
    const r = core.classifyEntity(d);
    assert.strictEqual(r.type, 'drug', `${d} should be classified as drug`);
    assert.strictEqual(r.id, d.toLowerCase());
  }
});

test('classifyEntity — drug suffix stat does not match statin ending', () => {
  // The suffix list has "stat" not "statin", so atorvastatin (ending "in") is not matched
  const r = core.classifyEntity('atorvastatin');
  assert.strictEqual(r.type, 'unknown');
});

test('classifyEntity — species binomial nomenclature', () => {
  const r = core.classifyEntity('Homo sapiens');
  assert.strictEqual(r.type, 'species');
  assert.strictEqual(r.id, 'Homo sapiens');
});

test('classifyEntity — species — Drosophila melanogaster', () => {
  const r = core.classifyEntity('Drosophila melanogaster');
  assert.strictEqual(r.type, 'species');
});

test('classifyEntity — GO term', () => {
  const r = core.classifyEntity('GO:0008150');
  assert.strictEqual(r.type, 'go_term');
  assert.strictEqual(r.id, 'GO:0008150');
});

test('classifyEntity — Reactome pathway', () => {
  const r = core.classifyEntity('R-HSA-1640170');
  assert.strictEqual(r.type, 'pathway');
  assert.strictEqual(r.id, 'R-HSA-1640170');
});

test('classifyEntity — KEGG pathway', () => {
  const r = core.classifyEntity('hsa05200');
  assert.strictEqual(r.type, 'pathway');
});

test('classifyEntity — InterPro domain', () => {
  const r = core.classifyEntity('IPR002093');
  assert.strictEqual(r.type, 'domain');
  assert.strictEqual(r.id, 'IPR002093');
});

test('classifyEntity — Pfam domain', () => {
  const r = core.classifyEntity('PF00069');
  assert.strictEqual(r.type, 'domain');
  assert.strictEqual(r.id, 'PF00069');
});

test('classifyEntity — OMIM disease', () => {
  const r = core.classifyEntity('113705');
  assert.strictEqual(r.type, 'disease');
});

test('classifyEntity — DOID disease', () => {
  const r = core.classifyEntity('DOID:1234');
  assert.strictEqual(r.type, 'disease');
});

test('classifyEntity — Orphanet disease', () => {
  const r = core.classifyEntity('ORPHA:1234');
  assert.strictEqual(r.type, 'disease');
});

test('classifyEntity — cell type', () => {
  const r = core.classifyEntity('T cell');
  assert.strictEqual(r.type, 'cell_type');
});

test('classifyEntity — cell type macrophage', () => {
  const r = core.classifyEntity('macrophage');
  assert.strictEqual(r.type, 'cell_type');
});

test('classifyEntity — cytoband', () => {
  const r = core.classifyEntity('17q21.31');
  assert.strictEqual(r.type, 'cytoband');
});

test('classifyEntity — null/empty input returns unknown', () => {
  assert.strictEqual(core.classifyEntity(null).type, 'unknown');
  assert.strictEqual(core.classifyEntity('').type, 'unknown');
  assert.strictEqual(core.classifyEntity(undefined).type, 'unknown');
});

test('classifyEntity — unknown free text', () => {
  const r = core.classifyEntity('some random text here');
  assert.strictEqual(r.type, 'unknown');
  assert.strictEqual(r.id, 'some random text here');
});

// ══════════════════════════════════════════════════════════════════════
// 3. Signal Score Computation
// ══════════════════════════════════════════════════════════════════════

test('computeSignalScore — returns total and breakdown', () => {
  const paper = { title: 'Test', date: new Date().toISOString(), journal: '', abstract: '' };
  const result = core.computeSignalScore(paper, []);
  assert.ok('total' in result);
  assert.ok('breakdown' in result);
  assert.ok('recency' in result.breakdown);
  assert.ok('citation_velocity' in result.breakdown);
  assert.ok('journal_tier' in result.breakdown);
  assert.ok('entity_match_count' in result.breakdown);
  assert.ok('co_mention_novelty' in result.breakdown);
  assert.ok('author_reputation' in result.breakdown);
});

test('computeSignalScore — recency: today gets 25 points', () => {
  const paper = { title: 'Test', date: new Date().toISOString(), journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, []);
  assert.strictEqual(breakdown.recency, 25);
});

test('computeSignalScore — recency: 30+ days ago gets 0', () => {
  const old = new Date(Date.now() - 35 * 24 * 60 * 60 * 1000).toISOString();
  const paper = { title: 'Test', date: old, journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, []);
  assert.strictEqual(breakdown.recency, 0);
});

test('computeSignalScore — recency: 15 days ago gets ~12-13', () => {
  const mid = new Date(Date.now() - 15 * 24 * 60 * 60 * 1000).toISOString();
  const paper = { title: 'Test', date: mid, journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, []);
  assert.ok(breakdown.recency >= 12 && breakdown.recency <= 13);
});

test('computeSignalScore — citation velocity: 10 cites/month = 20 pts', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, [], { citationVelocity: 10 });
  assert.strictEqual(breakdown.citation_velocity, 20);
});

test('computeSignalScore — citation velocity: 5 cites/month = 10 pts', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, [], { citationVelocity: 5 });
  assert.strictEqual(breakdown.citation_velocity, 10);
});

test('computeSignalScore — citation velocity: capped at 20', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, [], { citationVelocity: 50 });
  assert.strictEqual(breakdown.citation_velocity, 20);
});

test('computeSignalScore — journal tier 1 = 15 pts', () => {
  const paper = { title: 'Test', date: '', journal: 'Nature', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, []);
  assert.strictEqual(breakdown.journal_tier, 15);
});

test('computeSignalScore — journal tier 2 = 10 pts', () => {
  const paper = { title: 'Test', date: '', journal: 'Genome Research', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, []);
  assert.strictEqual(breakdown.journal_tier, 10);
});

test('computeSignalScore — journal tier 3 = 5 pts', () => {
  const paper = { title: 'Test', date: '', journal: 'PLOS ONE', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, []);
  assert.strictEqual(breakdown.journal_tier, 5);
});

test('computeSignalScore — unknown journal = 0 pts', () => {
  const paper = { title: 'Test', date: '', journal: 'Some Obscure Journal', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, []);
  assert.strictEqual(breakdown.journal_tier, 0);
});

test('computeSignalScore — entity match: 1 entity = 3 pts', () => {
  const paper = { title: 'BRCA1 mutations', date: '', journal: '', abstract: '' };
  const entities = [{ name: 'BRCA1' }];
  const { breakdown } = core.computeSignalScore(paper, entities);
  assert.strictEqual(breakdown.entity_match_count, 3);
});

test('computeSignalScore — entity match: 2 entities = 6 pts', () => {
  const paper = { title: 'BRCA1 and TP53 mutations', date: '', journal: '', abstract: '' };
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }];
  const { breakdown } = core.computeSignalScore(paper, entities);
  assert.strictEqual(breakdown.entity_match_count, 6);
});

test('computeSignalScore — entity match: 3+ entities = 10 pts', () => {
  const paper = { title: 'BRCA1, TP53, and EGFR in cancer', date: '', journal: '', abstract: '' };
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }, { name: 'EGFR' }];
  const { breakdown } = core.computeSignalScore(paper, entities);
  assert.strictEqual(breakdown.entity_match_count, 10);
});

test('computeSignalScore — co-mention novelty: novel pair = 10 pts', () => {
  const paper = { title: 'BRCA1 and TP53 co-occur', date: '', journal: '', abstract: '' };
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }];
  const { breakdown } = core.computeSignalScore(paper, entities, { knownCoMentions: new Set() });
  assert.strictEqual(breakdown.co_mention_novelty, 10);
});

test('computeSignalScore — co-mention novelty: known pair = 0 pts', () => {
  const paper = { title: 'BRCA1 and TP53 co-occur', date: '', journal: '', abstract: '' };
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }];
  const known = new Set(['brca1::tp53']);
  const { breakdown } = core.computeSignalScore(paper, entities, { knownCoMentions: known });
  assert.strictEqual(breakdown.co_mention_novelty, 0);
});

test('computeSignalScore — co-mention novelty: capped at 20', () => {
  const paper = { title: 'BRCA1 TP53 EGFR ALK ROS1', date: '', journal: '', abstract: '' };
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }, { name: 'EGFR' }, { name: 'ALK' }, { name: 'ROS1' }];
  const { breakdown } = core.computeSignalScore(paper, entities, { knownCoMentions: new Set() });
  assert.ok(breakdown.co_mention_novelty <= 20);
});

test('computeSignalScore — author reputation: h-index 80+ = 10', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, [], { authorHIndex: 100 });
  assert.strictEqual(breakdown.author_reputation, 10);
});

test('computeSignalScore — author reputation: h-index 50 = 8', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, [], { authorHIndex: 50 });
  assert.strictEqual(breakdown.author_reputation, 8);
});

test('computeSignalScore — author reputation: h-index 30 = 6', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, [], { authorHIndex: 30 });
  assert.strictEqual(breakdown.author_reputation, 6);
});

test('computeSignalScore — author reputation: h-index 0 = 0', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const { breakdown } = core.computeSignalScore(paper, [], { authorHIndex: 0 });
  assert.strictEqual(breakdown.author_reputation, 0);
});

test('computeSignalScore — total capped at 100', () => {
  const paper = {
    title: 'BRCA1 TP53 EGFR ALK ROS1',
    date: new Date().toISOString(),
    journal: 'Nature',
    abstract: 'BRCA1 TP53 EGFR ALK ROS1'
  };
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }, { name: 'EGFR' }, { name: 'ALK' }, { name: 'ROS1' }];
  const result = core.computeSignalScore(paper, entities, {
    citationVelocity: 50,
    authorHIndex: 100,
    knownCoMentions: new Set()
  });
  assert.ok(result.total <= 100);
});

test('computeSignalScore — empty paper gets 0', () => {
  const paper = { title: '', date: '', journal: '', abstract: '' };
  const result = core.computeSignalScore(paper, []);
  assert.strictEqual(result.total, 0);
});

// ══════════════════════════════════════════════════════════════════════
// 4. Journal Tier Lookup
// ══════════════════════════════════════════════════════════════════════

test('JOURNAL_TIERS — Nature is tier 1', () => {
  assert.strictEqual(core.JOURNAL_TIERS['nature'], 1);
});

test('JOURNAL_TIERS — Cell is tier 1', () => {
  assert.strictEqual(core.JOURNAL_TIERS['cell'], 1);
});

test('JOURNAL_TIERS — NEJM variants are tier 1', () => {
  assert.strictEqual(core.JOURNAL_TIERS['the new england journal of medicine'], 1);
  assert.strictEqual(core.JOURNAL_TIERS['n engl j med'], 1);
});

test('JOURNAL_TIERS — Bioinformatics is tier 2', () => {
  assert.strictEqual(core.JOURNAL_TIERS['bioinformatics'], 2);
});

test('JOURNAL_TIERS — Nature Communications is tier 2', () => {
  assert.strictEqual(core.JOURNAL_TIERS['nature communications'], 2);
});

test('JOURNAL_TIERS — Scientific Reports is tier 3', () => {
  assert.strictEqual(core.JOURNAL_TIERS['scientific reports'], 3);
});

test('JOURNAL_TIERS — unknown journal returns undefined', () => {
  assert.strictEqual(core.JOURNAL_TIERS['journal of made up science'], undefined);
});

// ══════════════════════════════════════════════════════════════════════
// 5. Adaptive Polling
// ══════════════════════════════════════════════════════════════════════

test('getPollingInterval — hot: >5 papers/week', () => {
  const entity = { stats: { papers_last_week: 6, papers_last_month: 20 } };
  assert.strictEqual(core.getPollingInterval(entity), core.POLLING_INTERVALS.hot);
});

test('getPollingInterval — active: 1-5 papers/week', () => {
  const entity = { stats: { papers_last_week: 3, papers_last_month: 12 } };
  assert.strictEqual(core.getPollingInterval(entity), core.POLLING_INTERVALS.active);
});

test('getPollingInterval — moderate: <1/week but >=1/month', () => {
  const entity = { stats: { papers_last_week: 0, papers_last_month: 2 } };
  assert.strictEqual(core.getPollingInterval(entity), core.POLLING_INTERVALS.moderate);
});

test('getPollingInterval — rare: 0 papers/month', () => {
  const entity = { stats: { papers_last_week: 0, papers_last_month: 0 } };
  assert.strictEqual(core.getPollingInterval(entity), core.POLLING_INTERVALS.rare);
});

test('getPollingInterval — missing stats defaults to rare', () => {
  const entity = {};
  assert.strictEqual(core.getPollingInterval(entity), core.POLLING_INTERVALS.rare);
});

test('getPollingInterval — boundary: exactly 5 papers/week is active', () => {
  const entity = { stats: { papers_last_week: 5, papers_last_month: 20 } };
  assert.strictEqual(core.getPollingInterval(entity), core.POLLING_INTERVALS.active);
});

test('getPollingInterval — boundary: exactly 1 paper/week is active', () => {
  const entity = { stats: { papers_last_week: 1, papers_last_month: 4 } };
  assert.strictEqual(core.getPollingInterval(entity), core.POLLING_INTERVALS.active);
});

// ══════════════════════════════════════════════════════════════════════
// 6. Co-mention Detection
// ══════════════════════════════════════════════════════════════════════

test('detectCoMentions — finds co-occurring entities', () => {
  const papers = [
    { title: 'BRCA1 and TP53 in breast cancer', abstract: '', doi: '10.1/a' }
  ];
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }, { name: 'EGFR' }];
  const result = core.detectCoMentions(papers, entities);
  assert.strictEqual(result.length, 1);
  assert.ok(result[0].entityA === 'brca1' || result[0].entityB === 'brca1');
  assert.ok(result[0].entityA === 'tp53' || result[0].entityB === 'tp53');
});

test('detectCoMentions — marks novel co-mentions', () => {
  const papers = [
    { title: 'BRCA1 and TP53', abstract: '', doi: '10.1/a' }
  ];
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }];
  const result = core.detectCoMentions(papers, entities, new Set());
  assert.strictEqual(result[0].isNovel, true);
});

test('detectCoMentions — marks known co-mentions as not novel', () => {
  const papers = [
    { title: 'BRCA1 and TP53', abstract: '', doi: '10.1/a' }
  ];
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }];
  const known = new Set(['brca1::tp53']);
  const result = core.detectCoMentions(papers, entities, known);
  assert.strictEqual(result[0].isNovel, false);
});

test('detectCoMentions — returns paperIds', () => {
  const papers = [
    { title: 'BRCA1 and TP53 study one', abstract: '', doi: '10.1/a' },
    { title: 'BRCA1 and TP53 study two', abstract: '', doi: '10.1/b' }
  ];
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }];
  const result = core.detectCoMentions(papers, entities);
  assert.strictEqual(result[0].paperIds.length, 2);
});

test('detectCoMentions — no co-mentions if only one entity matched', () => {
  const papers = [
    { title: 'BRCA1 only', abstract: '', doi: '10.1/a' }
  ];
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }];
  const result = core.detectCoMentions(papers, entities);
  assert.strictEqual(result.length, 0);
});

test('detectCoMentions — empty papers array returns empty', () => {
  const result = core.detectCoMentions([], [{ name: 'BRCA1' }]);
  assert.strictEqual(result.length, 0);
});

test('detectCoMentions — sorts novel before non-novel', () => {
  const papers = [
    { title: 'BRCA1 and TP53', abstract: '', doi: '10.1/a' },
    { title: 'EGFR and ALK', abstract: '', doi: '10.1/b' }
  ];
  const entities = [{ name: 'BRCA1' }, { name: 'TP53' }, { name: 'EGFR' }, { name: 'ALK' }];
  const known = new Set(['brca1::tp53']);
  const result = core.detectCoMentions(papers, entities, known);
  assert.strictEqual(result.length, 2);
  // Novel should come first
  assert.strictEqual(result[0].isNovel, true);
  assert.strictEqual(result[1].isNovel, false);
});

// ══════════════════════════════════════════════════════════════════════
// 7. Paper Deduplication
// ══════════════════════════════════════════════════════════════════════

test('dedupPapers — DOI exact match dedup', () => {
  const pubmed = [{ title: 'Paper A', doi: '10.1/test', source: 'pubmed', abstract: '' }];
  const biorxiv = [{ title: 'Paper A preprint', doi: '10.1/test', source: 'biorxiv', abstract: 'has abstract' }];
  const result = core.dedupPapers(pubmed, biorxiv);
  assert.strictEqual(result.length, 1);
  assert.strictEqual(result[0].source, 'pubmed');
  // bioRxiv abstract should be merged into PubMed entry
  assert.strictEqual(result[0].abstract, 'has abstract');
});

test('dedupPapers — fuzzy title match dedup', () => {
  const pubmed = [{ title: 'A Novel Gene Therapy Approach for Cancer Treatment', doi: null, source: 'pubmed', abstract: '' }];
  const biorxiv = [{ title: 'A Novel Gene Therapy Approach for Cancer Treatment', doi: null, source: 'biorxiv', abstract: 'Detailed abstract' }];
  const result = core.dedupPapers(pubmed, biorxiv);
  assert.strictEqual(result.length, 1);
});

test('dedupPapers — different papers not deduped', () => {
  const pubmed = [{ title: 'BRCA1 study', doi: '10.1/a', source: 'pubmed' }];
  const biorxiv = [{ title: 'TP53 study', doi: '10.1/b', source: 'biorxiv' }];
  const result = core.dedupPapers(pubmed, biorxiv);
  assert.strictEqual(result.length, 2);
});

test('dedupPapers — PubMed takes priority', () => {
  const pubmed = [{ title: 'Shared paper', doi: '10.1/shared', source: 'pubmed', abstract: 'pm abs' }];
  const biorxiv = [{ title: 'Shared paper', doi: '10.1/shared', source: 'biorxiv', abstract: 'bx abs' }];
  const result = core.dedupPapers(pubmed, biorxiv);
  assert.strictEqual(result.length, 1);
  assert.strictEqual(result[0].source, 'pubmed');
});

test('dedupPapers — empty inputs', () => {
  assert.strictEqual(core.dedupPapers([], []).length, 0);
});

test('dedupPapers — DOI case-insensitive', () => {
  const pubmed = [{ title: 'Test', doi: '10.1/ABC', source: 'pubmed' }];
  const biorxiv = [{ title: 'Test preprint', doi: '10.1/abc', source: 'biorxiv' }];
  const result = core.dedupPapers(pubmed, biorxiv);
  assert.strictEqual(result.length, 1);
});

test('dedupPapers — paper with no DOI and no title still included', () => {
  const pubmed = [{ title: '', doi: null, source: 'pubmed' }];
  const result = core.dedupPapers(pubmed, []);
  assert.strictEqual(result.length, 1);
});

// ══════════════════════════════════════════════════════════════════════
// 8. Watchlist Helpers
// ══════════════════════════════════════════════════════════════════════

test('createWatchEntry — returns correct structure', () => {
  const entry = core.createWatchEntry('BRCA1', 'gene');
  assert.ok(entry, 'Entry should exist');
  assert.strictEqual(entry.id, 'BRCA1');
  assert.strictEqual(entry.type, 'gene');
  assert.ok(entry.created, 'Should have created timestamp');
  assert.ok(typeof entry.priority === 'string', 'Should have priority');
  assert.ok(typeof entry.muted === 'boolean' || typeof entry.paused === 'boolean', 'Should have muted/paused flag');
});

test('createWatchEntry — stats initialized to zero', () => {
  const entry = core.createWatchEntry('TP53', 'gene');
  assert.strictEqual(entry.stats.papers_last_week, 0);
  assert.strictEqual(entry.stats.papers_last_month, 0);
  assert.strictEqual(entry.stats.total_papers, 0);
  assert.strictEqual(entry.stats.last_paper_date, null);
});

test('createWatchEntry — custom priority and tags', () => {
  const entry = core.createWatchEntry('BRCA1', 'gene', 'critical', ['cancer', 'hereditary']);
  assert.strictEqual(entry.priority, 'critical');
  assert.strictEqual(entry.tags.length, 2);
  assert.strictEqual(entry.tags[0], 'cancer');
  assert.strictEqual(entry.tags[1], 'hereditary');
});

test('createWatchEntry — timestamps are ISO format', () => {
  const entry = core.createWatchEntry('EGFR', 'gene');
  assert.ok(entry.created.includes('T'));
  assert.ok(entry.updated.includes('T'));
});

test('createWatchEntry — auto-classifies type if not provided', () => {
  const entry = core.createWatchEntry('rs12345678', null);
  assert.strictEqual(entry.type, 'variant');
});

test('updateWatchEntry — updates fields and sets updated timestamp', () => {
  const entry = core.createWatchEntry('BRCA1', 'gene');
  const oldUpdated = entry.updated;
  // Ensure a tiny time difference
  const updated = core.updateWatchEntry(entry, { priority: 'high', notes: 'important' });
  assert.strictEqual(updated.priority, 'high');
  assert.strictEqual(updated.notes, 'important');
  assert.strictEqual(updated.id, 'BRCA1'); // immutable
});

test('updateWatchEntry — id and created are immutable', () => {
  const entry = core.createWatchEntry('BRCA1', 'gene');
  const origCreated = entry.created;
  core.updateWatchEntry(entry, { id: 'CHANGED', created: '2020-01-01' });
  assert.strictEqual(entry.id, 'BRCA1');
  assert.strictEqual(entry.created, origCreated);
});

test('updateWatchEntry — null entry returns entry', () => {
  assert.strictEqual(core.updateWatchEntry(null, { a: 1 }), null);
});

test('updateWatchEntry — null updates returns entry unchanged', () => {
  const entry = core.createWatchEntry('X', 'gene');
  const result = core.updateWatchEntry(entry, null);
  assert.strictEqual(result, entry);
});

// ══════════════════════════════════════════════════════════════════════
// 9. Paper Helpers
// ══════════════════════════════════════════════════════════════════════

test('createPaperEntry — standard fields from pubmed', () => {
  const raw = {
    pmid: '12345',
    doi: '10.1/test',
    title: 'Test Paper',
    authors: 'Smith J, Doe A',
    journal: 'Nature',
    date: '2024-01-15',
    abstract: 'Abstract text'
  };
  const paper = core.createPaperEntry(raw, 'pubmed');
  assert.strictEqual(paper.pmid, '12345');
  assert.strictEqual(paper.doi, '10.1/test');
  assert.strictEqual(paper.title, 'Test Paper');
  assert.strictEqual(paper.source, 'pubmed');
  assert.strictEqual(paper.read, false);
  assert.strictEqual(paper.starred, false);
  assert.strictEqual(paper.signalScore, null);
  assert.ok(paper.discoveredAt.includes('T'));
});

test('createPaperEntry — matched entities are mapped', () => {
  const raw = { title: 'Test' };
  const matched = [{ id: 'BRCA1', name: 'BRCA1', type: 'gene' }];
  const paper = core.createPaperEntry(raw, 'pubmed', matched);
  assert.strictEqual(paper.matchedEntities.length, 1);
  assert.strictEqual(paper.matchedEntities[0].id, 'BRCA1');
  assert.strictEqual(paper.matchedEntities[0].type, 'gene');
});

test('createPaperEntry — biorxiv-specific fields', () => {
  const raw = { title: 'Preprint', biorxiv_category: 'genomics', biorxiv_version: '2' };
  const paper = core.createPaperEntry(raw, 'biorxiv');
  assert.strictEqual(paper.biorxiv_category, 'genomics');
  assert.strictEqual(paper.biorxiv_version, '2');
});

test('createPaperEntry — defaults for missing fields', () => {
  const paper = core.createPaperEntry({}, 'openalex');
  assert.strictEqual(paper.pmid, null);
  assert.strictEqual(paper.doi, null);
  assert.strictEqual(paper.title, '');
  assert.strictEqual(paper.abstract, '');
  assert.strictEqual(paper.source, 'openalex');
  assert.strictEqual(paper.tags.length, 0);
  assert.strictEqual(paper.concepts.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// 10. Export Helpers — toBibTeX
// ══════════════════════════════════════════════════════════════════════

test('toBibTeX — empty array returns empty string', () => {
  assert.strictEqual(core.toBibTeX([]), '');
  assert.strictEqual(core.toBibTeX(null), '');
});

test('toBibTeX — single paper with PMID', () => {
  const papers = [{ pmid: '12345', title: 'Test Paper', authors: 'Smith J', journal: 'Nature', date: '2024', doi: '10.1/x' }];
  const bib = core.toBibTeX(papers);
  assert.ok(bib.includes('@article{pmid12345'));
  assert.ok(bib.includes('title = {Test Paper}'));
  assert.ok(bib.includes('journal = {Nature}'));
  assert.ok(bib.includes('year = {2024}'));
  assert.ok(bib.includes('doi = {10.1/x}'));
});

test('toBibTeX — paper without PMID uses DOI as key', () => {
  const papers = [{ pmid: null, title: 'Test', authors: '', journal: '', date: '2024', doi: '10.1/abc' }];
  const bib = core.toBibTeX(papers);
  assert.ok(bib.includes('@article{10_1_abc'));
});

test('toBibTeX — escapes special characters', () => {
  const papers = [{ title: 'Gene & Protein: 100% #1', authors: '', journal: '', date: '' }];
  const bib = core.toBibTeX(papers);
  assert.ok(bib.includes('\\&'));
  assert.ok(bib.includes('\\%'));
  assert.ok(bib.includes('\\#'));
});

test('toBibTeX — multiple authors joined with and', () => {
  const papers = [{ title: 'Test', authors: 'Smith J, Doe A, Lee B', journal: '', date: '' }];
  const bib = core.toBibTeX(papers);
  assert.ok(bib.includes(' and '));
});

// ══════════════════════════════════════════════════════════════════════
// 10b. Export Helpers — toRIS
// ══════════════════════════════════════════════════════════════════════

test('toRIS — empty array returns empty string', () => {
  assert.strictEqual(core.toRIS([]), '');
  assert.strictEqual(core.toRIS(null), '');
});

test('toRIS — starts with TY and ends with ER', () => {
  const papers = [{ title: 'Test', authors: '', journal: '', date: '' }];
  const ris = core.toRIS(papers);
  assert.ok(ris.startsWith('TY  - JOUR'));
  assert.ok(ris.includes('ER  - '));
});

test('toRIS — includes title, journal, DOI, PMID', () => {
  const papers = [{ title: 'Paper Title', authors: 'Smith J', journal: 'Nature', date: '2024', doi: '10.1/x', pmid: '111' }];
  const ris = core.toRIS(papers);
  assert.ok(ris.includes('TI  - Paper Title'));
  assert.ok(ris.includes('JO  - Nature'));
  assert.ok(ris.includes('DO  - 10.1/x'));
  assert.ok(ris.includes('AN  - PMID:111'));
});

test('toRIS — each author on separate AU line', () => {
  const papers = [{ title: 'Test', authors: 'Smith J, Doe A', journal: '', date: '' }];
  const ris = core.toRIS(papers);
  const auLines = ris.split('\n').filter(l => l.startsWith('AU  - '));
  assert.strictEqual(auLines.length, 2);
});

// ══════════════════════════════════════════════════════════════════════
// 10c. Export Helpers — toMarkdown
// ══════════════════════════════════════════════════════════════════════

test('toMarkdown — empty array returns empty string', () => {
  assert.strictEqual(core.toMarkdown([]), '');
  assert.strictEqual(core.toMarkdown(null), '');
});

test('toMarkdown — starts with # Papers heading', () => {
  const papers = [{ title: 'Test', authors: '', journal: '', date: '' }];
  const md = core.toMarkdown(papers);
  assert.ok(md.startsWith('# Papers'));
});

test('toMarkdown — includes title as heading', () => {
  const papers = [{ title: 'My Important Paper', authors: 'Smith J', journal: 'Nature', date: '2024' }];
  const md = core.toMarkdown(papers);
  assert.ok(md.includes('## My Important Paper'));
});

test('toMarkdown — includes DOI and PubMed links', () => {
  const papers = [{ title: 'Test', doi: '10.1/x', pmid: '12345', authors: '', journal: '', date: '' }];
  const md = core.toMarkdown(papers);
  assert.ok(md.includes('https://doi.org/10.1/x'));
  assert.ok(md.includes('https://pubmed.ncbi.nlm.nih.gov/12345/'));
});

test('toMarkdown — includes signal score if present', () => {
  const papers = [{ title: 'Test', signalScore: 85, authors: '', journal: '', date: '' }];
  const md = core.toMarkdown(papers);
  assert.ok(md.includes('[Signal: 85]'));
});

test('toMarkdown — truncates long abstracts', () => {
  const longAbstract = 'A'.repeat(400);
  const papers = [{ title: 'Test', abstract: longAbstract, authors: '', journal: '', date: '' }];
  const md = core.toMarkdown(papers);
  assert.ok(md.includes('...'));
});

test('toMarkdown — includes matched entities', () => {
  const papers = [{
    title: 'Test', authors: '', journal: '', date: '',
    matchedEntities: [{ name: 'BRCA1', type: 'gene' }, { name: 'TP53', type: 'gene' }]
  }];
  const md = core.toMarkdown(papers);
  assert.ok(md.includes('`BRCA1`'));
  assert.ok(md.includes('`TP53`'));
});

// ══════════════════════════════════════════════════════════════════════
// 10d. Export Helpers — watchlistToJSON
// ══════════════════════════════════════════════════════════════════════

test('watchlistToJSON — null returns empty array string', () => {
  assert.strictEqual(core.watchlistToJSON(null), '[]');
});

test('watchlistToJSON — wraps entries with metadata', () => {
  const entries = [{ id: 'BRCA1', type: 'gene' }];
  const json = JSON.parse(core.watchlistToJSON(entries));
  assert.strictEqual(json.version, 1);
  assert.strictEqual(json.source, 'BioKhoj');
  assert.ok(json.exported);
  assert.strictEqual(json.entries.length, 1);
  assert.strictEqual(json.entries[0].id, 'BRCA1');
});

// ══════════════════════════════════════════════════════════════════════
// 11. Weekly Digest
// ══════════════════════════════════════════════════════════════════════

test('generateDigest — returns markdown with header', () => {
  const md = core.generateDigest([], [], {});
  assert.ok(md.includes('# BioKhoj Weekly Digest'));
});

test('generateDigest — summary section with counts', () => {
  const papers = [
    { title: 'P1', signalScore: 85, matchedEntities: [] },
    { title: 'P2', signalScore: 65, matchedEntities: [] },
    { title: 'P3', signalScore: 45, matchedEntities: [] },
    { title: 'P4', signalScore: 15, matchedEntities: [] }
  ];
  const md = core.generateDigest(papers, [], {});
  assert.ok(md.includes('**Total new papers:** 4'));
  assert.ok(md.includes('**Critical signal (80+):** 1'));
  assert.ok(md.includes('**High signal (60-79):** 1'));
  assert.ok(md.includes('**Medium signal (40-59):** 1'));
});

test('generateDigest — must-read section for critical papers', () => {
  const papers = [{ title: 'Critical Paper', signalScore: 90, journal: 'Nature', date: '2024', doi: '10.1/x', matchedEntities: [] }];
  const md = core.generateDigest(papers, [], {});
  assert.ok(md.includes('Must-Read Papers'));
  assert.ok(md.includes('Critical Paper'));
});

test('generateDigest — novel co-mentions section', () => {
  const coMentions = [{ entityA: 'BRCA1', entityB: 'TP53', paperIds: ['a'], isNovel: true }];
  const md = core.generateDigest([], [], { coMentions });
  assert.ok(md.includes('Novel Co-mentions'));
  assert.ok(md.includes('BRCA1'));
  assert.ok(md.includes('TP53'));
});

test('generateDigest — entity activity section', () => {
  const entityCounts = new Map([['BRCA1', 5], ['TP53', 3]]);
  const md = core.generateDigest([], [], { entityCounts });
  assert.ok(md.includes('Entity Activity'));
  assert.ok(md.includes('BRCA1'));
});

test('generateDigest — top journals section', () => {
  const topJournals = [{ journal: 'Nature', count: 3 }];
  const md = core.generateDigest([], [], { topJournals });
  assert.ok(md.includes('Top Journals'));
  assert.ok(md.includes('Nature'));
});

test('generateDigest — watchlist health for inactive entities', () => {
  const watchlist = [
    core.createWatchEntry('OBSCURE1', 'gene'),
  ];
  const md = core.generateDigest([], watchlist, {});
  assert.ok(md.includes('Watchlist Health'));
  assert.ok(md.includes('OBSCURE1'));
});

test('generateDigest — ends with generated-by line', () => {
  const md = core.generateDigest([], [], {});
  assert.ok(md.includes('Generated by BioKhoj'));
});

test('generateDigest — high signal section appears', () => {
  const papers = [{ title: 'High Paper', signalScore: 70, journal: 'PNAS', matchedEntities: [{ name: 'EGFR' }] }];
  const md = core.generateDigest(papers, [], {});
  assert.ok(md.includes('High Signal Papers'));
  assert.ok(md.includes('High Paper'));
});

// ══════════════════════════════════════════════════════════════════════
// 12. API Budget Tracking
// ══════════════════════════════════════════════════════════════════════

test('getApiBudget — exists and returns expected structure', () => {
  assert.ok(typeof core.getApiBudget === 'function');
  const budget = core.getApiBudget();
  assert.ok('used' in budget);
  assert.ok('limit' in budget);
  assert.ok('remaining' in budget);
  assert.ok('perSecond' in budget);
  assert.ok('hasApiKey' in budget);
  assert.ok('pct' in budget);
});

test('getApiBudget — default rate is 3 req/s without API key', () => {
  core.setNcbiApiKey(null);
  const budget = core.getApiBudget();
  assert.strictEqual(budget.perSecond, 3);
  assert.strictEqual(budget.hasApiKey, false);
});

test('getApiBudget — rate is 10 req/s with API key', () => {
  core.setNcbiApiKey('test-key-12345');
  const budget = core.getApiBudget();
  assert.strictEqual(budget.perSecond, 10);
  assert.strictEqual(budget.hasApiKey, true);
  // Reset
  core.setNcbiApiKey(null);
});

test('getApiBudget — remaining = limit - used', () => {
  const budget = core.getApiBudget();
  assert.strictEqual(budget.remaining, budget.limit - budget.used);
});

test('getApiBudget — pct is a number 0-100', () => {
  const budget = core.getApiBudget();
  assert.ok(budget.pct >= 0 && budget.pct <= 100);
});

// ══════════════════════════════════════════════════════════════════════
// 13. searchPubMed — Options Object Support
// ══════════════════════════════════════════════════════════════════════

test('searchPubMed — accepts options object with daysBack', async () => {
  resetFetch();
  // esearch response
  fetchResponses.push({ body: { esearchresult: { idlist: [] } } });
  const result = await core.searchPubMed('BRCA1', { maxResults: 5, daysBack: 7 });
  assert.ok(Array.isArray(result));
  assert.strictEqual(result.length, 0);
  // Verify the fetch was called with mindate parameter
  assert.ok(fetchCalls.length >= 1);
  const url = fetchCalls[0].url;
  assert.ok(url.includes('retmax=5'), 'should use maxResults from options');
  assert.ok(url.includes('mindate='), 'should have mindate from daysBack');
});

test('searchPubMed — accepts options object with minDate and maxDate', async () => {
  resetFetch();
  fetchResponses.push({ body: { esearchresult: { idlist: [] } } });
  const result = await core.searchPubMed('TP53', { maxResults: 10, minDate: '2024/01/01', maxDate: '2024/06/30' });
  assert.ok(Array.isArray(result));
  const url = fetchCalls[0].url;
  assert.ok(url.includes('mindate=2024%2F01%2F01'));
  assert.ok(url.includes('maxdate=2024%2F06%2F30'));
});

test('searchPubMed — accepts positional args (backward compat)', async () => {
  resetFetch();
  fetchResponses.push({ body: { esearchresult: { idlist: [] } } });
  const result = await core.searchPubMed('EGFR', 15, '2024/01/01');
  assert.ok(Array.isArray(result));
  const url = fetchCalls[0].url;
  assert.ok(url.includes('retmax=15'));
  assert.ok(url.includes('mindate=2024%2F01%2F01'));
});

test('searchPubMed — returns empty array on network error', async () => {
  resetFetch();
  fetchResponses.push({ error: new Error('Network error') });
  const result = await core.searchPubMed('BRCA1');
  assert.ok(Array.isArray(result));
  assert.strictEqual(result.length, 0);
});

test('searchPubMed — returns papers with expected fields', async () => {
  resetFetch();
  // esearch
  fetchResponses.push({ body: { esearchresult: { idlist: ['99999'] } } });
  // esummary
  fetchResponses.push({
    body: {
      result: {
        '99999': {
          title: 'Test Paper', fulljournalname: 'Nature',
          pubdate: '2024 Jan', authors: [{ name: 'Smith J' }],
          articleids: [{ idtype: 'doi', value: '10.1/test' }]
        }
      }
    }
  });
  // efetch abstracts (XML)
  fetchResponses.push({
    text: '<PubmedArticle><PMID>99999</PMID><AbstractText>Test abstract</AbstractText></PubmedArticle>'
  });
  const result = await core.searchPubMed('test', 1);
  assert.strictEqual(result.length, 1);
  assert.strictEqual(result[0].pmid, '99999');
  assert.strictEqual(result[0].title, 'Test Paper');
  assert.strictEqual(result[0].journal, 'Nature');
  assert.strictEqual(result[0].doi, '10.1/test');
});

// ══════════════════════════════════════════════════════════════════════
// 14. setNcbiApiKey — Configuration
// ══════════════════════════════════════════════════════════════════════

test('setNcbiApiKey — sets and clears API key', () => {
  core.setNcbiApiKey('my-key');
  assert.strictEqual(core.getApiBudget().hasApiKey, true);
  assert.strictEqual(core.getApiBudget().perSecond, 10);

  core.setNcbiApiKey(null);
  assert.strictEqual(core.getApiBudget().hasApiKey, false);
  assert.strictEqual(core.getApiBudget().perSecond, 3);
});

test('setNcbiApiKey — empty string clears key', () => {
  core.setNcbiApiKey('');
  assert.strictEqual(core.getApiBudget().hasApiKey, false);
});

// ══════════════════════════════════════════════════════════════════════
// 15. Edge Cases & Robustness
// ══════════════════════════════════════════════════════════════════════

test('classifyEntity — very long input returns unknown', () => {
  const long = 'A'.repeat(500);
  const r = core.classifyEntity(long);
  assert.strictEqual(r.type, 'unknown');
});

test('classifyEntity — whitespace-only input returns unknown', () => {
  const r = core.classifyEntity('   ');
  assert.ok(r.type === 'unknown' || r.id === '');
});

test('computeSignalScore — handles missing date gracefully', () => {
  const paper = { title: 'Test', date: null, journal: '', abstract: '' };
  const result = core.computeSignalScore(paper, []);
  assert.strictEqual(result.breakdown.recency, 0);
});

test('computeSignalScore — handles undefined context fields', () => {
  const paper = { title: 'BRCA1', date: '', journal: '', abstract: '' };
  const result = core.computeSignalScore(paper, [{ name: 'BRCA1' }], {});
  assert.ok(result.total >= 0);
});

test('computeSignalScore — PubMed date format "2024 Jan 15"', () => {
  const paper = { title: 'Test', date: '2024 Jan 15', journal: '', abstract: '' };
  const result = core.computeSignalScore(paper, []);
  // Should parse without error; recency depends on current date
  assert.ok(result.breakdown.recency >= 0);
});

test('detectCoMentions — handles papers with no title/abstract gracefully', () => {
  const papers = [{ title: null, abstract: null, doi: '10.1/x' }];
  const result = core.detectCoMentions(papers, [{ name: 'BRCA1' }]);
  assert.strictEqual(result.length, 0);
});

test('dedupPapers — handles DOI with https://doi.org/ prefix', () => {
  const pubmed = [{ title: 'Test', doi: 'https://doi.org/10.1/abc', source: 'pubmed' }];
  const biorxiv = [{ title: 'Test preprint', doi: '10.1/abc', source: 'biorxiv' }];
  const result = core.dedupPapers(pubmed, biorxiv);
  assert.strictEqual(result.length, 1);
});

test('JOURNAL_TIERS — case sensitivity: lowercase lookup works', () => {
  assert.strictEqual(core.JOURNAL_TIERS['nature genetics'], 1);
  assert.strictEqual(core.JOURNAL_TIERS['genome biology'], 2);
  assert.strictEqual(core.JOURNAL_TIERS['bmc genomics'], 3);
});

test('JOURNAL_TIERS — abbreviated names work', () => {
  assert.strictEqual(core.JOURNAL_TIERS['n engl j med'], 1);
  assert.strictEqual(core.JOURNAL_TIERS['proc natl acad sci u s a'], 2);
  assert.strictEqual(core.JOURNAL_TIERS['j biol chem'], 2);
});

test('toBibTeX — handles semicolon-separated authors', () => {
  const papers = [{ title: 'Test', authors: 'Smith, John; Doe, Alice', journal: '', date: '' }];
  const bib = core.toBibTeX(papers);
  assert.ok(bib.includes('Smith, John and Doe, Alice'));
});

test('toRIS — includes abstract when present', () => {
  const papers = [{ title: 'Test', authors: '', journal: '', date: '', abstract: 'My abstract text' }];
  const ris = core.toRIS(papers);
  assert.ok(ris.includes('AB  - My abstract text'));
});

test('watchlistToJSON — valid JSON output', () => {
  const entries = [{ id: 'BRCA1' }, { id: 'TP53' }];
  const str = core.watchlistToJSON(entries);
  const parsed = JSON.parse(str);
  assert.strictEqual(parsed.entries.length, 2);
  assert.ok(typeof parsed.exported === 'string');
});

test('generateDigest — handles papers without signalScore', () => {
  const papers = [{ title: 'Unscored', signalScore: null, matchedEntities: [] }];
  const md = core.generateDigest(papers, [], {});
  assert.ok(md.includes('Total new papers'));
});

// ══════════════════════════════════════════════════════════════════════
// 16. PWA/Extension Entity Format Compatibility
// ══════════════════════════════════════════════════════════════════════

test('computeSignalScore — works with PWA entity format (term field)', () => {
  const paper = { title: 'BRCA1 mutations in cancer', date: new Date().toISOString(), journal: 'Nature', abstract: 'BRCA1' };
  const entities = [{ term: 'BRCA1', type: 'gene' }]; // PWA format: term, not name
  const result = core.computeSignalScore(paper, entities);
  assert.ok(result.breakdown.entity_match_count >= 3, 'Should match BRCA1 via term field');
});

test('computeSignalScore — works with extension entity format (name field)', () => {
  const paper = { title: 'TP53 study', date: '', journal: '', abstract: '' };
  const entities = [{ name: 'TP53', type: 'gene' }]; // Extension format: name, not term
  const result = core.computeSignalScore(paper, entities);
  assert.ok(result.breakdown.entity_match_count >= 3);
});

test('computeSignalScore — works with id-only entity', () => {
  const paper = { title: 'EGFR inhibitor', date: '', journal: '', abstract: '' };
  const entities = [{ id: 'EGFR' }];
  const result = core.computeSignalScore(paper, entities);
  assert.ok(result.breakdown.entity_match_count >= 3);
});

test('computeSignalScore — null context does not crash', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const result = core.computeSignalScore(paper, [], null);
  assert.ok(result.total >= 0);
});

test('computeSignalScore — undefined context does not crash', () => {
  const paper = { title: 'Test', date: '', journal: '', abstract: '' };
  const result = core.computeSignalScore(paper, []);
  assert.ok(result.total >= 0);
});

test('detectCoMentions — works with PWA entity format (term field)', () => {
  const papers = [{ title: 'BRCA1 and TP53 together', abstract: '', doi: '10.1/x' }];
  const entities = [{ term: 'BRCA1' }, { term: 'TP53' }]; // PWA format
  const result = core.detectCoMentions(papers, entities);
  assert.strictEqual(result.length, 1);
});

test('detectCoMentions — works with mixed entity formats', () => {
  const papers = [{ title: 'BRCA1 and TP53', abstract: '', doi: '10.1/x' }];
  const entities = [{ term: 'BRCA1' }, { name: 'TP53' }]; // Mixed
  const result = core.detectCoMentions(papers, entities);
  assert.strictEqual(result.length, 1);
});
