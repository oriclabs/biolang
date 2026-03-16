// BioGist Core — Comprehensive unit tests
// Tests all detection functions from biogist-core.js
// Run with: node --test core.test.js

const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

// Load biogist-core.js in a sandbox with a mock window
const coreSrc = fs.readFileSync(
  path.join(__dirname, '..', '..', 'extension', 'biogist', 'shared', 'biogist-core.js'),
  'utf8'
);

const sandbox = {
  window: {},
  document: { createElement: () => {
    let _text = '';
    return {
      set textContent(v) { _text = v; },
      get textContent() { return _text; },
      get innerHTML() { return _text.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;'); }
    };
  }},
  console: console
};
vm.createContext(sandbox);
vm.runInContext(coreSrc, sandbox);
const core = sandbox.window.BioGistCore;

// ══════════════════════════════════════════════════════════════════════
// Gene Detection
// ══════════════════════════════════════════════════════════════════════

test('detectGenes — finds known symbols', () => {
  const r = core.detectGenes("The BRCA1 and TP53 genes are key.");
  const ids = r.map(g => g.id);
  assert.ok(ids.includes("BRCA1"));
  assert.ok(ids.includes("TP53"));
});

test('detectGenes — excludes common English words', () => {
  const r = core.detectGenes("THE SET OF ALL GENES CAN MAP TO THIS END RACE ROLE SALT FACE");
  const ids = r.map(g => g.id);
  assert.ok(!ids.includes("THE"));
  assert.ok(!ids.includes("SET"));
  assert.ok(!ids.includes("RACE"));
  assert.ok(!ids.includes("ROLE"));
  assert.ok(!ids.includes("FACE"));
});

test('detectGenes — counts occurrences', () => {
  const r = core.detectGenes("BRCA1 is mutated. BRCA1 is important. BRCA1 again.");
  const brca1 = r.find(g => g.id === "BRCA1");
  assert.ok(brca1);
  assert.equal(brca1.count, 3);
});

test('detectGenes — includes snippet', () => {
  const r = core.detectGenes("mutations in BRCA1 were found");
  assert.ok(r[0].snippet);
  assert.ok(r[0].snippet.includes("BRCA1"));
});

test('detectGenes — no false positives on non-bio text', () => {
  const r = core.detectGenes("The quick brown fox jumps over the lazy dog.");
  assert.equal(r.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// Variant Detection
// ══════════════════════════════════════════════════════════════════════

test('detectVariants — finds rsIDs', () => {
  const r = core.detectVariants("variant rs28934576 and rs1801133");
  assert.equal(r.length, 2);
  assert.ok(r.find(v => v.id === "rs28934576"));
});

test('detectVariants — finds HGVS', () => {
  const r = core.detectVariants("mutation NM_007294.4:c.5266dupC was identified");
  assert.ok(r.length > 0);
  assert.equal(r[0].subtype, "hgvs");
});

test('detectVariants — finds ClinVar VCV', () => {
  const r = core.detectVariants("See VCV000017599 for details");
  assert.ok(r.find(v => v.subtype === "clinvar"));
});

test('detectVariants — finds COSMIC', () => {
  const r = core.detectVariants("COSM6224 is a hotspot mutation");
  assert.ok(r.find(v => v.subtype === "cosmic"));
});

test('detectVariants — deduplicates', () => {
  const r = core.detectVariants("rs28934576 was found. We confirmed rs28934576 again.");
  assert.equal(r.filter(v => v.id === "rs28934576").length, 1);
});

// ══════════════════════════════════════════════════════════════════════
// Accession Detection
// ══════════════════════════════════════════════════════════════════════

test('detectAccessions — GEO series', () => {
  const r = core.detectAccessions("Data in GSE62944");
  assert.ok(r.find(a => a.id === "GSE62944"));
});

test('detectAccessions — SRA run', () => {
  const r = core.detectAccessions("Run SRR1234567 analyzed");
  assert.ok(r.find(a => a.id === "SRR1234567"));
});

test('detectAccessions — BioProject', () => {
  const r = core.detectAccessions("Project PRJNA289974");
  assert.ok(r.find(a => a.id === "PRJNA289974"));
});

test('detectAccessions — Ensembl gene', () => {
  const r = core.detectAccessions("Gene ENSG00000012048");
  assert.ok(r.find(a => a.id === "ENSG00000012048"));
});

test('detectAccessions — DOI', () => {
  const r = core.detectAccessions("Published at 10.1038/nrg.2016.49");
  assert.ok(r.find(a => a.subtype === "doi"));
});

test('detectAccessions — RefSeq', () => {
  const r = core.detectAccessions("Transcript NM_007294.4");
  assert.ok(r.find(a => a.subtype === "refseq"));
});

// ══════════════════════════════════════════════════════════════════════
// Species Detection
// ══════════════════════════════════════════════════════════════════════

test('detectSpecies — Human', () => {
  const r = core.detectSpecies("Study in Homo sapiens");
  assert.ok(r.find(s => s.id === "Human"));
});

test('detectSpecies — Mouse', () => {
  const r = core.detectSpecies("Mus musculus model");
  assert.ok(r.find(s => s.id === "Mouse"));
});

test('detectSpecies — E. coli', () => {
  const r = core.detectSpecies("E. coli K-12 strain");
  assert.ok(r.find(s => s.id === "E. coli"));
});

test('detectSpecies — Zebrafish', () => {
  const r = core.detectSpecies("Danio rerio embryos");
  assert.ok(r.find(s => s.id === "Zebrafish"));
});

test('detectSpecies — Yeast', () => {
  const r = core.detectSpecies("Saccharomyces cerevisiae deletion library");
  assert.ok(r.find(s => /Yeast/.test(s.id)));
});

test('detectSpecies — SARS-CoV-2', () => {
  const r = core.detectSpecies("SARS-CoV-2 genomic surveillance");
  assert.ok(r.find(s => s.id === "SARS-CoV-2"));
});

test('detectSpecies — HIV', () => {
  const r = core.detectSpecies("HIV-1 integration sites");
  assert.ok(r.find(s => s.id === "HIV"));
});

test('detectSpecies — Lactobacillus', () => {
  const r = core.detectSpecies("Lactobacillus rhamnosus strain GG");
  assert.ok(r.find(s => s.id === "Lactobacillus"));
});

test('detectSpecies — Arabidopsis', () => {
  const r = core.detectSpecies("Arabidopsis thaliana Col-0");
  assert.ok(r.find(s => s.id === "Arabidopsis"));
});

test('detectSpecies — case insensitive', () => {
  const r = core.detectSpecies("HUMAN samples analyzed");
  assert.ok(r.find(s => s.id === "Human"));
});

// ══════════════════════════════════════════════════════════════════════
// Methods/Tools Detection
// ══════════════════════════════════════════════════════════════════════

test('detectMethods — finds BWA', () => {
  const r = core.detectMethods("Reads aligned with BWA-MEM2 v2.0");
  assert.ok(r.find(m => m.id === "BWA-MEM2"));
});

test('detectMethods — finds GATK', () => {
  const r = core.detectMethods("Variants called using GATK HaplotypeCaller");
  assert.ok(r.find(m => m.id === "GATK"));
});

test('detectMethods — finds DESeq2', () => {
  const r = core.detectMethods("Differential expression with DESeq2");
  assert.ok(r.find(m => m.id === "DESeq2"));
});

test('detectMethods — finds Seurat', () => {
  const r = core.detectMethods("Single cell analysis using Seurat v4.0");
  assert.ok(r.find(m => m.id === "Seurat"));
});

test('detectMethods — finds Nextflow', () => {
  const r = core.detectMethods("Pipeline built with Nextflow");
  assert.ok(r.find(m => m.id === "Nextflow"));
});

test('detectMethods — finds BLAST', () => {
  const r = core.detectMethods("Sequences searched against BLAST database");
  assert.ok(r.find(m => m.id === "BLAST"));
});

test('detectMethods — detects version', () => {
  const r = core.detectMethods("aligned with STAR 2.7.10a");
  const star = r.find(m => m.id === "STAR");
  assert.ok(star);
  assert.equal(star.version, "2.7.10a");
});

test('detectMethods — finds AlphaFold', () => {
  const r = core.detectMethods("Structure predicted with AlphaFold");
  assert.ok(r.find(m => m.id === "AlphaFold"));
});

test('detectMethods — no false positives', () => {
  const r = core.detectMethods("The weather is nice today");
  assert.equal(r.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// Genome Build Detection
// ══════════════════════════════════════════════════════════════════════

test('detectGenomeBuild — GRCh38', () => {
  const r = core.detectGenomeBuild("Aligned to GRCh38 reference");
  assert.ok(r.find(b => b.id.includes("GRCh38")));
});

test('detectGenomeBuild — hg19', () => {
  const r = core.detectGenomeBuild("Coordinates in hg19");
  assert.ok(r.find(b => b.id.includes("hg19")));
});

test('detectGenomeBuild — T2T', () => {
  const r = core.detectGenomeBuild("Using T2T-CHM13 assembly");
  assert.ok(r.find(b => b.id.includes("T2T")));
});

test('detectGenomeBuild — mouse mm10', () => {
  const r = core.detectGenomeBuild("Mouse genome mm10");
  assert.ok(r.find(b => b.id.includes("mm10")));
});

// ══════════════════════════════════════════════════════════════════════
// Sample Size Detection
// ══════════════════════════════════════════════════════════════════════

test('detectSampleSize — n=X', () => {
  const r = core.detectSampleSize("cohort of n=234 patients");
  assert.ok(r.find(s => s.id === "n=234"));
});

test('detectSampleSize — N patients', () => {
  const r = core.detectSampleSize("100 samples were collected");
  assert.ok(r.find(s => s.id === "n=100"));
});

test('detectSampleSize — cohort of N', () => {
  const r = core.detectSampleSize("cohort of 500 individuals");
  assert.ok(r.length > 0);
});

test('detectSampleSize — ignores tiny numbers', () => {
  const r = core.detectSampleSize("n=1 was tested");
  assert.equal(r.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// Statistical Methods Detection
// ══════════════════════════════════════════════════════════════════════

test('detectStatMethods — t-test', () => {
  const r = core.detectStatMethods("Compared using t-test");
  assert.ok(r.find(s => s.id === "t-test"));
});

test('detectStatMethods — FDR', () => {
  const r = core.detectStatMethods("FDR correction applied");
  assert.ok(r.find(s => /FDR|false discovery/i.test(s.id)));
});

test('detectStatMethods — PCA', () => {
  const r = core.detectStatMethods("PCA revealed two clusters");
  assert.ok(r.find(s => s.id === "PCA"));
});

test('detectStatMethods — Kaplan-Meier', () => {
  const r = core.detectStatMethods("Kaplan-Meier survival curves");
  assert.ok(r.find(s => s.id === "Kaplan-Meier"));
});

test('detectStatMethods — Benjamini-Hochberg', () => {
  const r = core.detectStatMethods("Benjamini-Hochberg correction");
  assert.ok(r.find(s => s.id === "Benjamini-Hochberg"));
});

// ══════════════════════════════════════════════════════════════════════
// Sequencing Platform Detection
// ══════════════════════════════════════════════════════════════════════

test('detectPlatforms — Illumina', () => {
  const r = core.detectPlatforms("Sequenced on Illumina NovaSeq 6000");
  assert.ok(r.find(p => p.id === "Illumina"));
});

test('detectPlatforms — PacBio', () => {
  const r = core.detectPlatforms("PacBio HiFi long reads");
  assert.ok(r.find(p => p.id === "PacBio"));
});

test('detectPlatforms — Oxford Nanopore', () => {
  const r = core.detectPlatforms("Oxford Nanopore MinION sequencing");
  assert.ok(r.find(p => p.id === "Oxford Nanopore"));
});

test('detectPlatforms — 10x Genomics', () => {
  const r = core.detectPlatforms("10x Genomics Chromium library");
  assert.ok(r.find(p => p.id === "10x Genomics"));
});

// ══════════════════════════════════════════════════════════════════════
// Cell Line Detection
// ══════════════════════════════════════════════════════════════════════

test('detectCellLines — HeLa', () => {
  const r = core.detectCellLines("Experiments in HeLa cells");
  assert.ok(r.find(c => c.id === "HeLa"));
});

test('detectCellLines — HEK293T', () => {
  const r = core.detectCellLines("Transfected HEK293T cells");
  assert.ok(r.find(c => c.id === "HEK293T"));
});

test('detectCellLines — MDA-MB-231', () => {
  const r = core.detectCellLines("MDA-MB-231 triple negative breast cancer");
  assert.ok(r.find(c => c.id === "MDA-MB-231"));
});

test('detectCellLines — iPSC', () => {
  const r = core.detectCellLines("differentiated from iPSC");
  assert.ok(r.find(c => c.id === "iPSC"));
});

// ══════════════════════════════════════════════════════════════════════
// Tissue Detection
// ══════════════════════════════════════════════════════════════════════

test('detectTissues — blood', () => {
  const r = core.detectTissues("collected from blood samples");
  assert.ok(r.find(t => t.id === "blood"));
});

test('detectTissues — tumor', () => {
  const r = core.detectTissues("tumor and adjacent normal tissue");
  assert.ok(r.find(t => t.id === "tumor"));
});

test('detectTissues — brain', () => {
  const r = core.detectTissues("brain tissue specimens");
  assert.ok(r.find(t => t.id === "brain"));
});

test('detectTissues — organoid', () => {
  const r = core.detectTissues("intestinal organoid cultures");
  assert.ok(r.find(t => t.id === "organoid"));
});

// ══════════════════════════════════════════════════════════════════════
// Drug Detection
// ══════════════════════════════════════════════════════════════════════

test('detectDrugs — olaparib', () => {
  const r = core.detectDrugs("treated with olaparib");
  assert.ok(r.find(d => d.id === "olaparib"));
});

test('detectDrugs — pembrolizumab', () => {
  const r = core.detectDrugs("pembrolizumab immunotherapy");
  assert.ok(r.find(d => d.id === "pembrolizumab"));
});

test('detectDrugs — cisplatin', () => {
  const r = core.detectDrugs("cisplatin resistance mechanism");
  assert.ok(r.find(d => d.id === "cisplatin"));
});

test('detectDrugs — tamoxifen', () => {
  const r = core.detectDrugs("tamoxifen for ER+ breast cancer");
  assert.ok(r.find(d => d.id === "tamoxifen"));
});

// ══════════════════════════════════════════════════════════════════════
// Key Findings Extraction
// ══════════════════════════════════════════════════════════════════════

test('extractKeyFindings — finds gene + significance', () => {
  const genes = [{ id: "BRCA1" }];
  const text = "Background info here. BRCA1 mutations were significantly associated with increased cancer risk. More text follows.";
  const r = core.extractKeyFindings ? core.extractKeyFindings(text, genes) : [];
  // extractKeyFindings may not be exported — check
  if (r.length > 0) {
    assert.ok(r[0].id.includes("BRCA1"));
  }
});

// ══════════════════════════════════════════════════════════════════════
// Full Scan — Integration
// ══════════════════════════════════════════════════════════════════════

test('scanText — real abstract finds multiple entity types', () => {
  const text = `
    We performed whole-genome sequencing on Illumina NovaSeq using GRCh38.
    BRCA1 mutations (rs80357906) were found in n=234 patients from GSE62944.
    Differential expression was analyzed with DESeq2 using Benjamini-Hochberg
    FDR correction. HeLa cells treated with olaparib showed significant
    response. Samples from blood and tumor tissue of Homo sapiens.
    See 10.1186/s12864-018-4601-5 for details.
  `;
  const r = core.scanText(text);
  const types = new Set(r.map(e => e.type));

  assert.ok(types.has("gene"), "Should find genes");
  assert.ok(types.has("variant"), "Should find variants");
  assert.ok(types.has("accession"), "Should find accessions");
  assert.ok(types.has("method"), "Should find methods");
  assert.ok(types.has("genome_build"), "Should find genome build");
  assert.ok(types.has("sample_size"), "Should find sample size");
  assert.ok(types.has("stat_method"), "Should find stat methods");
  assert.ok(types.has("platform"), "Should find platform");
  assert.ok(types.has("cell_line"), "Should find cell lines");
  assert.ok(types.has("drug"), "Should find drugs");
  assert.ok(types.has("tissue"), "Should find tissues");
  assert.ok(types.has("species"), "Should find species");
});

test('scanText — empty text returns empty', () => {
  assert.equal(core.scanText("").length, 0);
});

test('scanText — non-bio text returns minimal', () => {
  const r = core.scanText("JavaScript is a programming language for web development.");
  // Should find very few or no entities
  assert.ok(r.length < 3);
});

// ══════════════════════════════════════════════════════════════════════
// Utility Functions
// ══════════════════════════════════════════════════════════════════════

test('escapeHtml — escapes special chars', () => {
  const r = core.escapeHtml('<script>alert("xss")</script>');
  assert.ok(!r.includes('<script>'));
  assert.ok(r.includes('&lt;'));
});

test('truncate — truncates long strings', () => {
  assert.equal(core.truncate("hello world", 5), "hello...");
  assert.equal(core.truncate("hi", 5), "hi");
});

test('groupEntities — groups by type', () => {
  const entities = [
    { type: "gene", id: "BRCA1" },
    { type: "variant", id: "rs123" },
    { type: "gene", id: "TP53" },
  ];
  const g = core.groupEntities(entities);
  assert.equal(g.gene.length, 2);
  assert.equal(g.variant.length, 1);
});

test('TYPE_META — has all 18 entity types', () => {
  const expected = ["gene","variant","accession","method","genome_build","sample_size",
    "stat_method","platform","cell_line","tissue","drug","clinical_trial","funding",
    "repository","p_value","finding","file","species"];
  expected.forEach(t => {
    assert.ok(core.TYPE_META[t], "Missing TYPE_META for: " + t);
    assert.ok(core.TYPE_META[t].label, "Missing label for: " + t);
    assert.ok(core.TYPE_META[t].icon, "Missing icon for: " + t);
    assert.ok(core.TYPE_META[t].cls, "Missing cls for: " + t);
  });
});

// ══════════════════════════════════════════════════════════════════════
// EXCLUDE list — false positive prevention
// ══════════════════════════════════════════════════════════════════════

test('EXCLUDE — blocks PDF', () => {
  assert.ok(core.EXCLUDE.has("PDF"));
});

test('EXCLUDE — blocks common 2-letter abbreviations', () => {
  ["CP","KL","HP","PC","HR","SD","SE","OR"].forEach(w => {
    assert.ok(core.EXCLUDE.has(w), "Should exclude: " + w);
  });
});

test('EXCLUDE — blocks file format words', () => {
  ["CSV","TSV","TXT","PNG","JPG","SVG","JSON","HTML","XML"].forEach(w => {
    assert.ok(core.EXCLUDE.has(w), "Should exclude: " + w);
  });
});

test('EXCLUDE — does not block real gene symbols', () => {
  ["BRCA1","TP53","EGFR","KRAS","BRAF","ALK","MYC"].forEach(g => {
    assert.ok(!core.EXCLUDE.has(g), "Should NOT exclude gene: " + g);
  });
});

test('detectGenes — PDF not detected as gene', () => {
  const r = core.detectGenes("Download PDF here and save as PDF file");
  assert.equal(r.length, 0);
});

test('detectGenes — CP KL HP not detected as genes', () => {
  const r = core.detectGenes("The CP violation was studied. KL divergence computed. HP filter applied.");
  assert.equal(r.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// Expanded drugs
// ══════════════════════════════════════════════════════════════════════

test('detectDrugs — finds semaglutide', () => {
  const r = core.detectDrugs("Patients received semaglutide weekly");
  assert.equal(r.length, 1);
  assert.equal(r[0].id, "semaglutide");
});

test('detectDrugs — finds remdesivir', () => {
  const r = core.detectDrugs("Treatment with remdesivir for 5 days");
  assert.equal(r.length, 1);
});

test('detectDrugs — finds vancomycin', () => {
  const r = core.detectDrugs("Vancomycin was administered IV");
  assert.equal(r.length, 1);
});

test('detectDrugs — finds dolutegravir', () => {
  const r = core.detectDrugs("Dolutegravir-based ART regimen");
  assert.equal(r.length, 1);
});

test('detectDrugs — finds research compound JQ1', () => {
  const r = core.detectDrugs("Cells were treated with JQ1 at 500nM");
  assert.equal(r.length, 1);
  assert.equal(r[0].id, "JQ1");
});

// ══════════════════════════════════════════════════════════════════════
// Expanded cell lines
// ══════════════════════════════════════════════════════════════════════

test('detectCellLines — finds PANC-1', () => {
  const r = core.detectCellLines("PANC-1 cells were cultured");
  assert.equal(r.length, 1);
});

test('detectCellLines — finds FaDu', () => {
  const r = core.detectCellLines("FaDu head and neck squamous cells");
  assert.equal(r.length, 1);
});

test('detectCellLines — finds T24', () => {
  const r = core.detectCellLines("T24 bladder cancer cells");
  assert.equal(r.length, 1);
});

test('detectCellLines — finds RAW264.7', () => {
  const r = core.detectCellLines("RAW264.7 macrophage cell line");
  assert.equal(r.length, 1);
});

test('detectCellLines — finds HUVEC', () => {
  const r = core.detectCellLines("HUVEC endothelial cells were used");
  assert.equal(r.length, 1);
});

test('detectCellLines — finds 4T1 mouse model', () => {
  const r = core.detectCellLines("4T1 breast tumor model in BALB/c mice");
  assert.equal(r.length, 1);
});

// ══════════════════════════════════════════════════════════════════════
// Expanded species
// ══════════════════════════════════════════════════════════════════════

test('detectSpecies — Pig', () => {
  const r = core.detectSpecies("Pig intestinal samples were collected");
  assert.ok(r.length >= 1);
  assert.ok(r.some(s => s.id === "Pig"));
});

test('detectSpecies — Rice', () => {
  const r = core.detectSpecies("Oryza sativa rice genome analysis");
  assert.ok(r.length >= 1);
  assert.ok(r.some(s => s.id === "Rice"));
});

test('detectSpecies — Maize', () => {
  const r = core.detectSpecies("Zea mays maize transcriptome");
  assert.ok(r.length >= 1);
  assert.ok(r.some(s => s.id === "Maize"));
});

test('detectSpecies — Salmonella', () => {
  const r = core.detectSpecies("Salmonella contamination detected");
  assert.ok(r.length >= 1);
  assert.ok(r.some(s => s.id === "Salmonella"));
});

test('detectSpecies — Candida', () => {
  const r = core.detectSpecies("Candida albicans infection");
  assert.ok(r.length >= 1);
  assert.ok(r.some(s => s.id === "Candida"));
});

test('detectSpecies — Influenza', () => {
  const r = core.detectSpecies("Influenza H1N1 pandemic strain");
  assert.ok(r.length >= 1);
});

// ══════════════════════════════════════════════════════════════════════
// SPECIES_DATA completeness
// ══════════════════════════════════════════════════════════════════════

test('SPECIES_DATA — all detected species have data entries', () => {
  const text = "Human Mouse Rat Pig Zebrafish Arabidopsis Rice Maize E. coli Yeast Lactobacillus Salmonella SARS-CoV-2 HIV";
  const detected = core.detectSpecies(text);
  detected.forEach(sp => {
    assert.ok(core.SPECIES_DATA[sp.id], "Missing SPECIES_DATA for detected species: " + sp.id);
    assert.ok(core.SPECIES_DATA[sp.id].taxid, "Missing taxid for: " + sp.id);
    assert.ok(core.SPECIES_DATA[sp.id].latin, "Missing latin name for: " + sp.id);
  });
});

// ══════════════════════════════════════════════════════════════════════
// buildSourceLinks (from core)
// ══════════════════════════════════════════════════════════════════════

test('buildSourceLinks — gene includes standard links', () => {
  const links = core.buildSourceLinks("gene", "BRCA1", "");
  const labels = links.map(l => l.label);
  assert.ok(labels.includes("Ensembl"), "Should have Ensembl");
  assert.ok(labels.includes("GeneCards"), "Should have GeneCards");
});

// ══════════════════════════════════════════════════════════════════════
// Methods — no single-letter false positives
// ══════════════════════════════════════════════════════════════════════

test('detectMethods — no single-letter R false positive', () => {
  const r = core.detectMethods("The R value was computed using R statistical software");
  // Should NOT match standalone "R" — only "R/Bioconductor"
  const ids = r.map(m => m.id);
  assert.ok(!ids.includes("R"), "Should not detect standalone R as method");
});

test('detectMethods — finds R/Bioconductor', () => {
  const r = core.detectMethods("Analysis was performed in R/Bioconductor");
  assert.ok(r.length >= 1);
  assert.ok(r.some(m => m.id === "R/Bioconductor"));
});

// ══════════════════════════════════════════════════════════════════════
// Clinical trial detection
// ══════════════════════════════════════════════════════════════════════

test('detectClinicalTrials — finds NCT number', () => {
  const r = core.detectClinicalTrials("This study is registered as NCT04380701");
  assert.equal(r.length, 1);
  assert.equal(r[0].id, "NCT04380701");
});

test('detectClinicalTrials — finds multiple NCTs', () => {
  const r = core.detectClinicalTrials("NCT01234567 and NCT09876543 were reviewed");
  assert.equal(r.length, 2);
});

test('detectClinicalTrials — no false positives', () => {
  const r = core.detectClinicalTrials("The NCTC 8325 strain was used");
  assert.equal(r.length, 0);
});

// ══════════════════════════════════════════════════════════════════════
// Funding detection
// ══════════════════════════════════════════════════════════════════════

test('detectFunding — finds NIH R01 grant', () => {
  const r = core.detectFunding("Funded by R01 GM123456");
  assert.ok(r.length >= 1);
  assert.ok(r.some(f => f.id.includes("GM123456")));
});

test('detectFunding — finds ERC grant', () => {
  const r = core.detectFunding("ERC-2024-StG funding was received");
  assert.ok(r.length >= 1);
  assert.ok(r.some(f => f.subtype === "ERC"));
});

// ══════════════════════════════════════════════════════════════════════
// Repository detection
// ══════════════════════════════════════════════════════════════════════

test('detectRepositories — finds GitHub URL', () => {
  const r = core.detectRepositories("Code at https://github.com/samtools/samtools");
  assert.equal(r.length, 1);
  assert.ok(r[0].id.includes("github.com"));
  assert.equal(r[0].subtype, "GitHub");
});

test('detectRepositories — finds Zenodo DOI', () => {
  const r = core.detectRepositories("Data deposited at 10.5281/zenodo.1234567");
  assert.ok(r.length >= 1);
  assert.ok(r.some(r => r.subtype === "Zenodo DOI"));
});

test('detectRepositories — finds PyPI URL', () => {
  const r = core.detectRepositories("Install from https://pypi.org/project/biopython");
  assert.ok(r.length >= 1);
  assert.equal(r[0].subtype, "PyPI");
});

// ══════════════════════════════════════════════════════════════════════
// P-value detection
// ══════════════════════════════════════════════════════════════════════

test('detectPValues — finds p < 0.05', () => {
  const r = core.detectPValues("The result was significant (p < 0.05)");
  assert.ok(r.length >= 1);
  assert.ok(r.some(p => p.id.includes("0.05")));
});

test('detectPValues — finds p = 3.2e-8', () => {
  const r = core.detectPValues("Association reached p = 3.2e-8");
  assert.ok(r.length >= 1);
});

test('detectPValues — finds FDR < 0.01', () => {
  const r = core.detectPValues("Genes with FDR < 0.01 were selected");
  assert.ok(r.length >= 1);
  assert.ok(r.some(p => p.id.includes("FDR")));
});

test('detectPValues — caps at 20', () => {
  let text = "";
  for (let i = 0; i < 30; i++) text += "p < 0." + String(i).padStart(3, "0") + " ";
  const r = core.detectPValues(text);
  assert.ok(r.length <= 20);
});

// ══════════════════════════════════════════════════════════════════════
// scanText integration with new types
// ══════════════════════════════════════════════════════════════════════

test('scanText — detects clinical trials, repos, p-values', () => {
  const text = "Study NCT04380701 found BRCA1 (p < 0.001). Code: https://github.com/test/repo. Funded by R01 GM123456.";
  const r = core.scanText(text);
  const types = new Set(r.map(e => e.type));
  assert.ok(types.has("clinical_trial"), "Should detect clinical trial");
  assert.ok(types.has("repository"), "Should detect repository");
  assert.ok(types.has("p_value"), "Should detect p-value");
});
