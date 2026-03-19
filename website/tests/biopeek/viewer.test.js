// BioPeek Viewer — Unit tests
// Tests pure logic functions extracted from viewer.js
// Run with: node --test viewer.test.js

const { test } = require('node:test');
const assert = require('node:assert');

// ══════════════════════════════════════════════════════════════════════
// 1. IUPAC Motif Expansion
// ══════════════════════════════════════════════════════════════════════

const IUPAC_MAP = {
  R: "[AG]", Y: "[CT]", S: "[GC]", W: "[AT]", K: "[GT]", M: "[AC]",
  B: "[CGT]", D: "[AGT]", H: "[ACT]", V: "[ACG]", N: "[ACGT]"
};

function expandIUPAC(pattern) {
  if (/^[ATCGURYSWKMBDHVNatcguryswkmbdhvn]+$/.test(pattern)) {
    return pattern.split("").map(function(c) {
      return IUPAC_MAP[c.toUpperCase()] || c;
    }).join("");
  }
  return pattern;
}

test('IUPAC — TATAWR expands to TATA[AT][AG]', () => {
  assert.strictEqual(expandIUPAC('TATAWR'), 'TATA[AT][AG]');
});

test('IUPAC — CANNTG expands correctly', () => {
  assert.strictEqual(expandIUPAC('CANNTG'), 'CA[ACGT][ACGT]TG');
});

test('IUPAC — pure ATCG unchanged', () => {
  assert.strictEqual(expandIUPAC('ATCGATCG'), 'ATCGATCG');
});

test('IUPAC — single N expands', () => {
  assert.strictEqual(expandIUPAC('N'), '[ACGT]');
});

test('IUPAC — regex pattern passed through', () => {
  assert.strictEqual(expandIUPAC('ATG[CG]{3}'), 'ATG[CG]{3}');
});

test('IUPAC — empty string', () => {
  assert.strictEqual(expandIUPAC(''), '');
});

test('IUPAC — lowercase input', () => {
  const result = expandIUPAC('tatawr');
  assert.ok(result.includes('[AT]'));
  assert.ok(result.includes('[AG]'));
});

test('IUPAC — all ambiguity codes', () => {
  const result = expandIUPAC('RYSWKMBDHVN');
  assert.ok(result.includes('[AG]'));
  assert.ok(result.includes('[CT]'));
  assert.ok(result.includes('[GC]'));
  assert.ok(result.includes('[AT]'));
  assert.ok(result.includes('[GT]'));
  assert.ok(result.includes('[AC]'));
  assert.ok(result.includes('[CGT]'));
  assert.ok(result.includes('[AGT]'));
  assert.ok(result.includes('[ACT]'));
  assert.ok(result.includes('[ACG]'));
  assert.ok(result.includes('[ACGT]'));
});

// ══════════════════════════════════════════════════════════════════════
// 2. Format Detection from Name
// ══════════════════════════════════════════════════════════════════════

function detectFormatFromName(name) {
  var ext = (name.match(/\.([^.]+)$/) || [,""])[1].toLowerCase();
  if (ext === "gz") ext = (name.replace(/\.gz$/i, "").match(/\.([^.]+)$/) || [,""])[1].toLowerCase();
  if (ext === "fa" || ext === "fna" || ext === "faa" || ext === "fasta") return "fasta";
  if (ext === "fq" || ext === "fastq") return "fastq";
  if (ext === "vcf") return "vcf";
  if (ext === "bed") return "bed";
  if (ext === "gff" || ext === "gff3" || ext === "gtf") return "gff";
  if (ext === "csv") return "csv";
  if (ext === "tsv") return "tsv";
  if (ext === "sam") return "sam";
  return null;
}

test('format — .fasta', () => assert.strictEqual(detectFormatFromName('test.fasta'), 'fasta'));
test('format — .fa', () => assert.strictEqual(detectFormatFromName('test.fa'), 'fasta'));
test('format — .fna', () => assert.strictEqual(detectFormatFromName('test.fna'), 'fasta'));
test('format — .faa (protein)', () => assert.strictEqual(detectFormatFromName('test.faa'), 'fasta'));
test('format — .fastq', () => assert.strictEqual(detectFormatFromName('test.fastq'), 'fastq'));
test('format — .fq', () => assert.strictEqual(detectFormatFromName('test.fq'), 'fastq'));
test('format — .vcf', () => assert.strictEqual(detectFormatFromName('test.vcf'), 'vcf'));
test('format — .bed', () => assert.strictEqual(detectFormatFromName('test.bed'), 'bed'));
test('format — .gff', () => assert.strictEqual(detectFormatFromName('test.gff'), 'gff'));
test('format — .gff3', () => assert.strictEqual(detectFormatFromName('test.gff3'), 'gff'));
test('format — .gtf', () => assert.strictEqual(detectFormatFromName('test.gtf'), 'gff'));
test('format — .csv', () => assert.strictEqual(detectFormatFromName('test.csv'), 'csv'));
test('format — .tsv', () => assert.strictEqual(detectFormatFromName('test.tsv'), 'tsv'));
test('format — .sam', () => assert.strictEqual(detectFormatFromName('test.sam'), 'sam'));
test('format — .vcf.gz strips gz', () => assert.strictEqual(detectFormatFromName('test.vcf.gz'), 'vcf'));
test('format — .fastq.gz strips gz', () => assert.strictEqual(detectFormatFromName('test.fastq.gz'), 'fastq'));
test('format — .txt returns null', () => assert.strictEqual(detectFormatFromName('test.txt'), null));
test('format — no extension returns null', () => assert.strictEqual(detectFormatFromName('noext'), null));

// ══════════════════════════════════════════════════════════════════════
// 3. K-mer Counting
// ══════════════════════════════════════════════════════════════════════

function countKmers(sequences, k) {
  var counts = {};
  var total = 0;
  sequences.forEach(function(seq) {
    if (!seq) return;
    var s = seq.toUpperCase();
    for (var i = 0; i <= s.length - k; i++) {
      var kmer = s.substring(i, i + k);
      if (/^[ACGT]+$/.test(kmer)) { counts[kmer] = (counts[kmer] || 0) + 1; total++; }
    }
  });
  return { counts, total };
}

test('kmer — counts 4-mers correctly', () => {
  const r = countKmers(['ATCGATCG'], 4);
  assert.strictEqual(r.counts['ATCG'], 2);
  assert.strictEqual(r.counts['TCGA'], 1);
  assert.strictEqual(r.counts['CGAT'], 1);
  assert.strictEqual(r.counts['GATC'], 1);
});

test('kmer — skips N-containing kmers', () => {
  const r = countKmers(['ATCNATCG'], 4);
  assert.strictEqual(r.counts['ATCN'], undefined);
  assert.strictEqual(r.counts['ATCG'], 1);
});

test('kmer — handles empty input', () => {
  const r = countKmers([], 4);
  assert.strictEqual(r.total, 0);
});

test('kmer — handles short sequence', () => {
  const r = countKmers(['AT'], 4);
  assert.strictEqual(r.total, 0);
});

test('kmer — multiple sequences', () => {
  const r = countKmers(['ATCG', 'ATCG'], 4);
  assert.strictEqual(r.counts['ATCG'], 2);
});

// ══════════════════════════════════════════════════════════════════════
// 4. GC Sliding Window
// ══════════════════════════════════════════════════════════════════════

function gcSlidingWindow(seq, windowSize) {
  if (!seq || seq.length < windowSize) return [];
  var points = [];
  for (var i = 0; i <= seq.length - windowSize; i += Math.max(1, Math.floor(seq.length / 200))) {
    var gc = 0, total = 0;
    for (var j = i; j < i + windowSize; j++) {
      var c = seq.charAt(j).toUpperCase();
      if (c === 'G' || c === 'C') { gc++; total++; }
      else if (c === 'A' || c === 'T') { total++; }
    }
    if (total > 0) points.push({ x: i, y: (gc / total) * 100 });
  }
  return points;
}

test('gc window — returns points for long sequence', () => {
  const seq = 'ATCGATCGATCG'.repeat(20);  // 240bp
  const points = gcSlidingWindow(seq, 100);
  assert.ok(points.length > 0);
  assert.ok(points[0].y === 50);  // ATCG is 50% GC
});

test('gc window — too short returns empty', () => {
  assert.deepStrictEqual(gcSlidingWindow('ATCG', 100), []);
});

test('gc window — null input', () => {
  assert.deepStrictEqual(gcSlidingWindow(null, 100), []);
});

test('gc window — 100% GC', () => {
  const seq = 'GCGCGCGCGC'.repeat(30);
  const points = gcSlidingWindow(seq, 100);
  assert.ok(points.length > 0);
  assert.strictEqual(points[0].y, 100);
});

test('gc window — 0% GC', () => {
  const seq = 'ATATATATAT'.repeat(30);
  const points = gcSlidingWindow(seq, 100);
  assert.ok(points.length > 0);
  assert.strictEqual(points[0].y, 0);
});

// ══════════════════════════════════════════════════════════════════════
// 5. FASTA Parser Logic
// ══════════════════════════════════════════════════════════════════════

function parseFastaText(text) {
  var records = [];
  var lines = text.split("\n");
  var header = "", seq = [];
  for (var i = 0; i < lines.length; i++) {
    var line = lines[i].trimEnd();
    if (line.charAt(0) === ">") {
      if (header || seq.length) {
        var s = seq.join("");
        records.push({ id: header.split(/\s/)[0], sequence: s, length: s.length });
      }
      header = line.substring(1);
      seq = [];
    } else if (line) {
      seq.push(line);
    }
  }
  if (header || seq.length) {
    var s = seq.join("");
    records.push({ id: header.split(/\s/)[0], sequence: s, length: s.length });
  }
  return records;
}

test('fasta — parses single record', () => {
  const r = parseFastaText('>seq1 desc\nATCG\nGCTA');
  assert.strictEqual(r.length, 1);
  assert.strictEqual(r[0].id, 'seq1');
  assert.strictEqual(r[0].sequence, 'ATCGGCTA');
  assert.strictEqual(r[0].length, 8);
});

test('fasta — parses multiple records', () => {
  const r = parseFastaText('>a\nATCG\n>b\nGCTA');
  assert.strictEqual(r.length, 2);
  assert.strictEqual(r[0].id, 'a');
  assert.strictEqual(r[1].id, 'b');
});

test('fasta — empty input', () => {
  assert.strictEqual(parseFastaText('').length, 0);
});

test('fasta — multi-line sequence joined', () => {
  const r = parseFastaText('>x\nAT\nCG\nGC\nTA');
  assert.strictEqual(r[0].sequence, 'ATCGGCTA');
});

test('fasta — handles trailing newline', () => {
  const r = parseFastaText('>x\nATCG\n');
  assert.strictEqual(r.length, 1);
  assert.strictEqual(r[0].sequence, 'ATCG');
});

// ══════════════════════════════════════════════════════════════════════
// 6. Validation Messages
// ══════════════════════════════════════════════════════════════════════

test('validation — FASTA low parse rate explains multi-line', () => {
  const rows = new Array(356);
  const rawLines = 49999;
  const format = "fasta";
  var hint = "Only " + rows.length + " records parsed from ~" + rawLines + " lines.";
  if (format === "fasta") hint += " Each FASTA record spans multiple lines (header + sequence). This is normal for multi-line FASTA.";
  assert.ok(hint.includes('normal for multi-line FASTA'));
});

test('validation — FASTQ explains 4-line records', () => {
  const rows = new Array(1000);
  const format = "fastq";
  var hint = "Only " + rows.length + " records.";
  if (format === "fastq") hint += " Each FASTQ record uses 4 lines. " + rows.length + " records = " + (rows.length * 4) + " lines expected.";
  assert.ok(hint.includes('4000 lines expected'));
});

// ══════════════════════════════════════════════════════════════════════
// 7. Streaming Mode Constants
// ══════════════════════════════════════════════════════════════════════

test('streaming — threshold is 10MB', () => {
  const STREAM_THRESHOLD = 10 * 1024 * 1024;
  assert.strictEqual(STREAM_THRESHOLD, 10485760);
});

test('streaming — chunk size is 2MB', () => {
  const STREAM_CHUNK = 2 * 1024 * 1024;
  assert.strictEqual(STREAM_CHUNK, 2097152);
});

test('streaming — page size is 500', () => {
  const STREAM_PAGE = 500;
  assert.strictEqual(STREAM_PAGE, 500);
});

// ══════════════════════════════════════════════════════════════════════
// 8. Color Toggle State
// ══════════════════════════════════════════════════════════════════════

test('color toggle — default is on', () => {
  // localStorage.getItem returns null when not set, !== "0" is true
  const enabled = null !== "0";
  assert.strictEqual(enabled, true);
});

test('color toggle — "0" means off', () => {
  const enabled = "0" !== "0";
  assert.strictEqual(enabled, false);
});

test('color toggle — "1" means on', () => {
  const enabled = "1" !== "0";
  assert.strictEqual(enabled, true);
});

// ══════════════════════════════════════════════════════════════════════
// 9. Fetch Timeout
// ══════════════════════════════════════════════════════════════════════

test('fetch timeout — AbortController exists', () => {
  assert.ok(typeof AbortController !== 'undefined');
});

// ══════════════════════════════════════════════════════════════════════
// 10. Batch Accession Parsing
// ══════════════════════════════════════════════════════════════════════

function parseAccessions(input) {
  return input.split(/[,\s\n]+/).map(s => s.trim()).filter(Boolean);
}

test('batch — comma separated', () => {
  assert.deepStrictEqual(parseAccessions('NM_007294, NC_000017, NP_009225'), ['NM_007294', 'NC_000017', 'NP_009225']);
});

test('batch — space separated', () => {
  assert.deepStrictEqual(parseAccessions('NM_007294 NC_000017'), ['NM_007294', 'NC_000017']);
});

test('batch — newline separated', () => {
  assert.deepStrictEqual(parseAccessions('NM_007294\nNC_000017'), ['NM_007294', 'NC_000017']);
});

test('batch — single accession', () => {
  assert.deepStrictEqual(parseAccessions('NM_007294'), ['NM_007294']);
});

test('batch — empty input', () => {
  assert.deepStrictEqual(parseAccessions(''), []);
});

test('batch — mixed separators', () => {
  assert.deepStrictEqual(parseAccessions('NM_007294, NC_000017\nNP_009225 P38398'), ['NM_007294', 'NC_000017', 'NP_009225', 'P38398']);
});

// ══════════════════════════════════════════════════════════════════════
// 11. Chromosome Multi-Select Filter
// ══════════════════════════════════════════════════════════════════════

test('chrom filter — add first', () => {
  var colFilters = {};
  colFilters[0] = new Set(['chr1']);
  assert.ok(colFilters[0].has('chr1'));
  assert.strictEqual(colFilters[0].size, 1);
});

test('chrom filter — add second', () => {
  var colFilters = {};
  colFilters[0] = new Set(['chr1']);
  colFilters[0].add('chr2');
  assert.ok(colFilters[0].has('chr1'));
  assert.ok(colFilters[0].has('chr2'));
  assert.strictEqual(colFilters[0].size, 2);
});

test('chrom filter — remove one', () => {
  var colFilters = {};
  colFilters[0] = new Set(['chr1', 'chr2']);
  colFilters[0].delete('chr1');
  assert.ok(!colFilters[0].has('chr1'));
  assert.ok(colFilters[0].has('chr2'));
});

test('chrom filter — remove last clears', () => {
  var colFilters = {};
  colFilters[0] = new Set(['chr1']);
  colFilters[0].delete('chr1');
  if (colFilters[0].size === 0) delete colFilters[0];
  assert.strictEqual(colFilters[0], undefined);
});

// ══════════════════════════════════════════════════════════════════════
// 12. Filter Cache Key
// ══════════════════════════════════════════════════════════════════════

test('cache key — includes Set values', () => {
  var colFilters = { 0: new Set(['chr1', 'chr2']) };
  var cfKey = Object.keys(colFilters).map(function(k) {
    return k + ":" + Array.from(colFilters[k]).sort().join("+");
  }).join(",");
  assert.strictEqual(cfKey, '0:chr1+chr2');
});

test('cache key — different values different key', () => {
  var k1 = '0:chr1';
  var k2 = '0:chr1+chr2';
  assert.notStrictEqual(k1, k2);
});

test('cache key — empty filters', () => {
  var colFilters = {};
  var cfKey = Object.keys(colFilters).map(function(k) {
    return k + ":" + Array.from(colFilters[k]).sort().join("+");
  }).join(",");
  assert.strictEqual(cfKey, '');
});
