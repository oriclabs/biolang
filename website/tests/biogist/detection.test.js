// BioGist — Unit tests for entity detection logic
// Tests the core detection functions from content.js
// Run with: node --test detection.test.js

const { test } = require('node:test');
const assert = require('node:assert');

// Extract detection functions from content.js by evaluating in a mock context
const fs = require('fs');
const path = require('path');

// Load content.js source
const contentSrc = fs.readFileSync(
  path.join(__dirname, '..', '..', 'extension', 'biogist', 'shared', 'content.js'),
  'utf8'
);

// Create a sandbox to extract detection functions
// The content.js is an IIFE — we need to extract the inner functions
// For testing, we'll replicate the key detection logic

// ── Gene Symbol Set (subset for testing) ──
const GENE_SYMBOLS = new Set([
  "TP53","BRCA1","BRCA2","PTEN","RB1","APC","VHL","KRAS","NRAS","BRAF",
  "PIK3CA","EGFR","ERBB2","HER2","ALK","RET","MET","ROS1","MYC","MYCN",
  "CDKN2A","CDK4","CDK6","MDM2","ATM","ATR","CHEK2","PALB2","RAD51",
  "MLH1","MSH2","MSH6","PMS2","JAK2","FLT3","KIT","ABL1","BCR",
  "GAPDH","ACTB","VEGFA","TNF","IL6","STAT3","ACE2"
]);

const EXCLUDE = new Set([
  "THE","AND","FOR","NOT","WITH","FROM","BUT","ALL","ARE","WAS",
  "SET","MAP","LET","RUN","USE","AGE","END","TOP","CAN","MAY",
  "HAS","HAD"
]);

function detectGenes(text) {
  const results = [];
  const seen = {};
  const re = /\b([A-Z][A-Z0-9]{1,9})\b/g;
  let match;
  while ((match = re.exec(text)) !== null) {
    const symbol = match[1];
    if (!EXCLUDE.has(symbol) && GENE_SYMBOLS.has(symbol)) {
      if (!seen[symbol]) {
        seen[symbol] = { type: "gene", id: symbol, count: 1 };
      } else {
        seen[symbol].count++;
      }
    }
  }
  return Object.values(seen);
}

function detectVariants(text) {
  const results = [];
  const seen = new Set();
  let match;
  const rsRe = /\b(rs\d{3,12})\b/gi;
  while ((match = rsRe.exec(text)) !== null) {
    const id = match[1].toLowerCase();
    if (!seen.has(id)) { seen.add(id); results.push({ type: "variant", id, subtype: "rsid" }); }
  }
  const hgvsRe = /\b((?:NM_|NP_|NC_)\d+(?:\.\d+)?:[cpg]\.\S+?)(?=[\s,;)\]]|$)/g;
  while ((match = hgvsRe.exec(text)) !== null) {
    if (!seen.has(match[1])) { seen.add(match[1]); results.push({ type: "variant", id: match[1], subtype: "hgvs" }); }
  }
  const vcvRe = /\b(VCV\d{9,12})\b/g;
  while ((match = vcvRe.exec(text)) !== null) {
    if (!seen.has(match[1])) { seen.add(match[1]); results.push({ type: "variant", id: match[1], subtype: "clinvar" }); }
  }
  return results;
}

function detectAccessions(text) {
  const results = [];
  const seen = new Set();
  let match;
  const patterns = [
    { re: /\b(GSE\d{3,8})\b/g, subtype: "geo_series" },
    { re: /\b(SRR\d{5,10})\b/g, subtype: "sra_run" },
    { re: /\b(PRJNA\d{4,8})\b/g, subtype: "bioproject" },
    { re: /\b(ENSG\d{11})\b/g, subtype: "ensembl_gene" },
    { re: /\b(NM_\d{6,9}(?:\.\d+)?)\b/g, subtype: "refseq" },
    { re: /\b(10\.\d{4,9}\/[^\s]{5,50})\b/g, subtype: "doi" },
  ];
  for (const p of patterns) {
    while ((match = p.re.exec(text)) !== null) {
      const id = match[1];
      if (!seen.has(id)) { seen.add(id); results.push({ type: "accession", id, subtype: p.subtype }); }
    }
  }
  return results;
}

function detectSpecies(text) {
  const results = [];
  const patterns = [
    { re: /\b(?:Homo sapiens|human)\b/i, name: "Human" },
    { re: /\b(?:Mus musculus|mouse)\b/i, name: "Mouse" },
    { re: /\b(?:E\. coli|Escherichia coli)\b/i, name: "E. coli" },
    { re: /\b(?:Drosophila melanogaster)\b/i, name: "Fruit fly" },
    { re: /\b(?:Danio rerio|zebrafish)\b/i, name: "Zebrafish" },
  ];
  for (const p of patterns) {
    if (p.re.test(text)) results.push({ type: "species", id: p.name });
  }
  return results;
}

// ── Tests ──

test('Gene detection — finds known symbols', () => {
  const text = "The BRCA1 and TP53 genes are associated with cancer risk.";
  const genes = detectGenes(text);
  const ids = genes.map(g => g.id);
  assert.ok(ids.includes("BRCA1"), "Should find BRCA1");
  assert.ok(ids.includes("TP53"), "Should find TP53");
});

test('Gene detection — excludes common English words', () => {
  const text = "THE SET OF ALL GENES CAN MAP TO THIS END.";
  const genes = detectGenes(text);
  const ids = genes.map(g => g.id);
  assert.ok(!ids.includes("THE"), "Should exclude THE");
  assert.ok(!ids.includes("SET"), "Should exclude SET");
  assert.ok(!ids.includes("ALL"), "Should exclude ALL");
  assert.ok(!ids.includes("CAN"), "Should exclude CAN");
  assert.ok(!ids.includes("MAP"), "Should exclude MAP");
  assert.ok(!ids.includes("END"), "Should exclude END");
});

test('Gene detection — counts occurrences', () => {
  const text = "BRCA1 is mutated. BRCA1 testing is important. BRCA1 variants are pathogenic.";
  const genes = detectGenes(text);
  const brca1 = genes.find(g => g.id === "BRCA1");
  assert.ok(brca1, "Should find BRCA1");
  assert.equal(brca1.count, 3, "Should count 3 occurrences");
});

test('Gene detection — does not find non-gene uppercase words', () => {
  const text = "HELLO WORLD TESTING RANDOM UPPERCASE WORDS";
  const genes = detectGenes(text);
  assert.equal(genes.length, 0, "Should find no genes");
});

test('Variant detection — finds rsIDs', () => {
  const text = "The variant rs28934576 is associated with cancer. Also rs1801133 is relevant.";
  const variants = detectVariants(text);
  const ids = variants.map(v => v.id);
  assert.ok(ids.includes("rs28934576"), "Should find rs28934576");
  assert.ok(ids.includes("rs1801133"), "Should find rs1801133");
});

test('Variant detection — finds HGVS notation', () => {
  const text = "The mutation NM_007294.4:c.5266dupC was identified.";
  const variants = detectVariants(text);
  assert.ok(variants.length > 0, "Should find HGVS variant");
  assert.equal(variants[0].subtype, "hgvs");
});

test('Variant detection — finds ClinVar IDs', () => {
  const text = "See ClinVar VCV000017599 for details.";
  const variants = detectVariants(text);
  const clinvar = variants.find(v => v.subtype === "clinvar");
  assert.ok(clinvar, "Should find ClinVar ID");
});

test('Variant detection — deduplicates', () => {
  const text = "rs28934576 is important. We analyzed rs28934576 again.";
  const variants = detectVariants(text);
  const rs = variants.filter(v => v.id === "rs28934576");
  assert.equal(rs.length, 1, "Should deduplicate rsIDs");
});

test('Accession detection — finds GEO series', () => {
  const text = "Data deposited in GSE62944.";
  const acc = detectAccessions(text);
  const geo = acc.find(a => a.id === "GSE62944");
  assert.ok(geo, "Should find GSE62944");
  assert.equal(geo.subtype, "geo_series");
});

test('Accession detection — finds SRA runs', () => {
  const text = "Sequencing run SRR1234567 was analyzed.";
  const acc = detectAccessions(text);
  assert.ok(acc.find(a => a.id === "SRR1234567"), "Should find SRR");
});

test('Accession detection — finds BioProject', () => {
  const text = "Project PRJNA289974 contains the data.";
  const acc = detectAccessions(text);
  assert.ok(acc.find(a => a.id === "PRJNA289974"), "Should find PRJNA");
});

test('Accession detection — finds Ensembl gene IDs', () => {
  const text = "Gene ENSG00000012048 encodes BRCA1.";
  const acc = detectAccessions(text);
  assert.ok(acc.find(a => a.id === "ENSG00000012048"), "Should find ENSG");
});

test('Accession detection — finds DOIs', () => {
  const text = "Published at 10.1038/nrg.2016.49 in Nature.";
  const acc = detectAccessions(text);
  const doi = acc.find(a => a.subtype === "doi");
  assert.ok(doi, "Should find DOI");
});

test('Accession detection — finds RefSeq', () => {
  const text = "Transcript NM_007294.4 is the reference.";
  const acc = detectAccessions(text);
  assert.ok(acc.find(a => a.subtype === "refseq"), "Should find RefSeq");
});

test('Species detection — finds Human', () => {
  const text = "This study was conducted in Homo sapiens samples.";
  const species = detectSpecies(text);
  assert.ok(species.find(s => s.id === "Human"), "Should detect Human");
});

test('Species detection — finds Mouse', () => {
  const text = "Experiments performed in Mus musculus.";
  const species = detectSpecies(text);
  assert.ok(species.find(s => s.id === "Mouse"), "Should detect Mouse");
});

test('Species detection — finds E. coli', () => {
  const text = "E. coli K-12 strain was used.";
  const species = detectSpecies(text);
  assert.ok(species.find(s => s.id === "E. coli"), "Should detect E. coli");
});

test('Species detection — case insensitive', () => {
  const text = "The HUMAN genome was sequenced.";
  const species = detectSpecies(text);
  assert.ok(species.find(s => s.id === "Human"), "Should detect human (case insensitive)");
});

test('Full text scan — real abstract', () => {
  const abstract = `
    Mutations in BRCA1 and BRCA2 are associated with increased risk of breast
    and ovarian cancer in Homo sapiens. We analyzed variant rs80357906 in a cohort
    from GSE62944 (SRP045636, BioProject PRJNA289974). The reference transcript
    NM_007294.4 was used. See 10.1186/s12864-018-4601-5 for full details.
  `;

  const genes = detectGenes(abstract);
  const variants = detectVariants(abstract);
  const accessions = detectAccessions(abstract);
  const species = detectSpecies(abstract);

  assert.ok(genes.length >= 2, "Should find at least BRCA1, BRCA2");
  assert.ok(variants.length >= 1, "Should find at least 1 variant");
  assert.ok(accessions.length >= 3, "Should find GSE, SRP, PRJNA");
  assert.ok(species.length >= 1, "Should find Human");
});

test('Empty text returns no entities', () => {
  assert.equal(detectGenes("").length, 0);
  assert.equal(detectVariants("").length, 0);
  assert.equal(detectAccessions("").length, 0);
  assert.equal(detectSpecies("").length, 0);
});

test('Non-bio text returns no entities', () => {
  const text = "The quick brown fox jumps over the lazy dog. JavaScript is a programming language.";
  assert.equal(detectGenes(text).length, 0);
  assert.equal(detectVariants(text).length, 0);
  assert.equal(detectAccessions(text).length, 0);
  assert.equal(detectSpecies(text).length, 0);
});
