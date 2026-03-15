// @ts-check
const { test, expect } = require('@playwright/test');
const {
  openViewer,
  loadFixture,
  getFormatBadge,
  getRecordCount,
  getTableHeaders,
  getCellText,
  getVisibleRowCount,
} = require('./helpers');

test.describe('File loading — format detection and parsing', () => {

  test.beforeEach(async ({ page }) => {
    await openViewer(page);
  });

  test('loads FASTA file with correct format and record count', async ({ page }) => {
    await loadFixture(page, 'sample.fasta');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('fasta');

    const count = await getRecordCount(page);
    expect(count).toContain('3');

    const headers = await getTableHeaders(page);
    // FASTA typically has: id, description, sequence, length columns
    expect(headers.some(h => h.toLowerCase().includes('id') || h.toLowerCase().includes('name'))).toBeTruthy();
    expect(headers.some(h => h.toLowerCase().includes('seq') || h.toLowerCase().includes('sequence'))).toBeTruthy();
  });

  test('loads FASTQ file with correct format and record count', async ({ page }) => {
    await loadFixture(page, 'sample.fastq');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('fastq');

    const count = await getRecordCount(page);
    expect(count).toContain('3');

    const headers = await getTableHeaders(page);
    expect(headers.some(h => h.toLowerCase().includes('id') || h.toLowerCase().includes('name'))).toBeTruthy();
    expect(headers.some(h => h.toLowerCase().includes('seq') || h.toLowerCase().includes('sequence'))).toBeTruthy();
    expect(headers.some(h => h.toLowerCase().includes('qual') || h.toLowerCase().includes('quality'))).toBeTruthy();
  });

  test('loads VCF file with correct format and record count', async ({ page }) => {
    await loadFixture(page, 'sample.vcf');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('vcf');

    const count = await getRecordCount(page);
    expect(count).toContain('5');

    const headers = await getTableHeaders(page);
    // VCF standard columns
    expect(headers).toEqual(expect.arrayContaining(
      expect.arrayContaining ? ['CHROM', 'POS', 'REF', 'ALT'].map(c =>
        expect.stringContaining(c)
      ) : []
    ));
    // Simpler check: at least CHROM and POS present
    const joinedHeaders = headers.join(' ').toUpperCase();
    expect(joinedHeaders).toContain('CHROM');
    expect(joinedHeaders).toContain('POS');

    // Spot check first row
    const rows = await getVisibleRowCount(page);
    expect(rows).toBe(5);
  });

  test('loads BED file with correct format and record count', async ({ page }) => {
    await loadFixture(page, 'sample.bed');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('bed');

    const count = await getRecordCount(page);
    expect(count).toContain('5');

    const headers = await getTableHeaders(page);
    const joinedHeaders = headers.join(' ').toLowerCase();
    expect(joinedHeaders).toContain('chrom');
    expect(joinedHeaders).toContain('start');
    expect(joinedHeaders).toContain('end');
  });

  test('loads GFF file with correct format and record count', async ({ page }) => {
    await loadFixture(page, 'sample.gff');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('gff');

    const count = await getRecordCount(page);
    expect(count).toContain('5');

    const headers = await getTableHeaders(page);
    const joinedHeaders = headers.join(' ').toLowerCase();
    expect(joinedHeaders).toContain('source');
    expect(joinedHeaders).toContain('type');
  });

  test('loads CSV file with correct format and record count', async ({ page }) => {
    await loadFixture(page, 'sample.csv');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('csv');

    const count = await getRecordCount(page);
    expect(count).toContain('5');

    const headers = await getTableHeaders(page);
    expect(headers).toContain('gene_id');
    expect(headers).toContain('gene_name');
    expect(headers).toContain('chrom');
    expect(headers).toContain('expression');
  });

  test('loads TSV file with correct format and record count', async ({ page }) => {
    await loadFixture(page, 'sample.tsv');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('tsv');

    const count = await getRecordCount(page);
    expect(count).toContain('5');

    const headers = await getTableHeaders(page);
    expect(headers).toContain('id');
    expect(headers).toContain('chrom');
    expect(headers).toContain('gene');
    expect(headers).toContain('effect');
  });

  test('loads large CSV file (1000 rows) with pagination', async ({ page }) => {
    await loadFixture(page, 'sample_large.csv');

    const format = await getFormatBadge(page);
    expect(format.toLowerCase()).toBe('csv');

    const count = await getRecordCount(page);
    expect(count).toContain('1000');

    // With pageSize=100 default, should show 100 rows
    const rows = await getVisibleRowCount(page);
    expect(rows).toBeLessThanOrEqual(100);
    expect(rows).toBeGreaterThan(0);
  });

});
