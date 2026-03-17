# BioPeek — Store Listing (Chrome Web Store + Microsoft Edge Add-ons)

## Name
BioPeek

## Short Description (132 chars max)
View FASTA, FASTQ, VCF, BED, GFF, and CSV files in your browser. Quality heatmaps, variant tables, motif search. 100% local.

## Description

BioPeek is a browser-based file viewer for bioinformatics researchers. Open common genomics file formats and instantly see their contents — no upload, no server, no install.

HOW IT WORKS
Drag and drop a file onto BioPeek, paste content, or open from a URL. The file is parsed and displayed in your browser using JavaScript and WebAssembly. Everything runs locally.

WHAT IT OPENS
BioPeek reads sequence files, variant files, interval files, feature annotations, and tabular data. It supports gzipped files with automatic decompression. Each format gets a tailored view — sequences show base coloring, quality files show a heatmap, and variant files show a filterable table.

HIGHLIGHTS
BioPeek handles large files by streaming chunks instead of loading everything into memory. You can search for DNA motifs using regex, navigate by genomic coordinates, copy data in multiple formats, and toggle between dark and light themes. Quality scores are visualized as a color gradient so you can spot problems at a glance.

PRIVACY
All file parsing happens entirely in your browser. Your files never leave your device. No data is uploaded, stored, or shared. No analytics, no tracking, no account required.

Report issues: https://github.com/oriclabs/biolang/issues
Built by Oric Labs — https://lang.bio

## Category
Developer Tools

## Language
English


# Chrome Web Store — Privacy Practices

## Single purpose description
View and inspect bioinformatics data files entirely in the browser. Parses common genomics formats locally using JavaScript and WebAssembly — no data is uploaded to any server.

## Permission justifications

### activeTab
Used to detect when a bioinformatics file is opened in the browser (by file extension) so BioPeek can offer to display it in the viewer. Only activated by explicit user action (clicking the extension icon or using the context menu).

### storage
Stores user preferences (theme setting, recent file names only — not file contents) locally on the user's device. Data can be cleared at any time through browser settings.

### contextMenus
Adds right-click menu options: "Open in BioPeek" for file links on web pages, and "Analyze selection" for selected text that may contain sequence data. These allow users to quickly send bio data to the viewer without copy-pasting.

### downloads
Used to detect when a user downloads a bioinformatics file (by file extension such as .fasta, .vcf, .bed). When detected, BioPeek offers to open the downloaded file directly in the viewer. No downloads are initiated or modified by the extension — it only observes download completion events to offer the viewing option.

## Host permission justification
No host permissions are required. BioPeek processes files locally and does not access external websites.

## Remote code
This extension does not use remote code. All JavaScript and WebAssembly runs from files bundled within the extension package.

## Data use disclosure

### What data do you collect?
None. BioPeek does not collect personally identifiable information, health information, financial information, authentication information, personal communications, location data, web history, user activity, or website content.

### Certification
- Data is NOT sold to third parties
- Data is NOT used for purposes unrelated to the extension's functionality
- Data is NOT used for creditworthiness or lending purposes

## Privacy policy URL
https://lang.bio/privacy


# Microsoft Edge Add-ons — Additional Fields

## Notes for certification
BioPeek is a file viewer extension that opens bioinformatics data files (FASTA, FASTQ, VCF, BED, GFF, CSV) in the browser. All parsing runs locally in JavaScript/WebAssembly — no data is uploaded to any server. The extension uses minimal permissions (activeTab for file detection, storage for theme preference). No remote code, no analytics, no tracking. Previously published as BLViewer on Chrome Web Store.

## Testing instructions
1. Click the BioPeek icon or navigate to the viewer page
2. Drag and drop any FASTA, FASTQ, VCF, BED, or CSV file
3. The file is parsed and displayed with format-specific visualizations
4. Try the motif search, coordinate navigation, and copy features
5. Toggle between dark and light themes

## Website
https://lang.bio/viewer

## Support URL
https://github.com/oriclabs/biolang/issues

## Privacy policy URL
https://lang.bio/privacy


# Search Terms

## Chrome Web Store (7 terms, max 30 chars each, max 21 words)
1. **bioinformatics file viewer**
2. **FASTA FASTQ VCF viewer**
3. **genomics data browser**
4. **sequence quality heatmap**
5. **BED GFF CSV bio viewer**
6. **variant table chromosome**
7. **DNA sequence motif search**

## Edge Add-ons (same terms)
1. bioinformatics file viewer
2. FASTA FASTQ VCF viewer
3. genomics data browser
4. sequence quality heatmap
5. BED GFF CSV bio viewer
6. variant table chromosome
7. DNA sequence motif search
