// Generate sample_large.csv with 1000 rows of gene expression data
const fs = require('fs');
const path = require('path');

const chromosomes = [
  'chr1', 'chr2', 'chr3', 'chr4', 'chr5', 'chr6', 'chr7', 'chr8', 'chr9', 'chr10',
  'chr11', 'chr12', 'chr13', 'chr14', 'chr15', 'chr16', 'chr17', 'chr18', 'chr19',
  'chr20', 'chr21', 'chr22', 'chrX', 'chrY'
];

const geneNames = [
  'BRCA1', 'TP53', 'EGFR', 'KRAS', 'MYC', 'RB1', 'PTEN', 'APC', 'VHL', 'BRAF',
  'PIK3CA', 'CDH1', 'SMAD4', 'ARID1A', 'ATM', 'CDKN2A', 'ERBB2', 'FGFR1', 'IDH1', 'JAK2',
  'KIT', 'MET', 'NF1', 'NRAS', 'PALB2', 'RAD51', 'RET', 'ROS1', 'STK11', 'TSC1'
];

function seededRandom(seed) {
  let s = seed;
  return function () {
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    return s / 0x7fffffff;
  };
}

const rand = seededRandom(42);

const header = 'gene_id,gene_name,chrom,start,end,expression,pvalue,log2fc,padj,biotype';
const rows = [header];

for (let i = 1; i <= 1000; i++) {
  const geneId = 'ENSG' + String(i).padStart(8, '0');
  const geneName = geneNames[Math.floor(rand() * geneNames.length)] + '_' + i;
  const chrom = chromosomes[Math.floor(rand() * chromosomes.length)];
  const start = Math.floor(rand() * 200000000) + 1000000;
  const end = start + Math.floor(rand() * 100000) + 1000;
  const expression = (rand() * 50).toFixed(2);
  const pvalue = (rand() * 0.1).toFixed(6);
  const log2fc = ((rand() - 0.5) * 8).toFixed(3);
  const padj = (parseFloat(pvalue) * (1 + rand())).toFixed(6);
  const biotype = rand() > 0.2 ? 'protein_coding' : rand() > 0.5 ? 'lncRNA' : 'pseudogene';

  rows.push(`${geneId},${geneName},${chrom},${start},${end},${expression},${pvalue},${log2fc},${padj},${biotype}`);
}

const outputPath = path.join(__dirname, 'sample_large.csv');
fs.writeFileSync(outputPath, rows.join('\n') + '\n');
console.log(`Generated ${outputPath} with ${rows.length - 1} data rows`);
