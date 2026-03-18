# BioGist — Chrome Web Store Listing

## Name
BioGist

## Short Description (132 chars max)
Get the biological gist of any research paper. Auto-detect genes, variants, drugs, species & more. Inline PubMed search. 100% local.

## Description

BioGist is a research sidebar that scans any webpage for biological entities and provides instant context — no uploads, no server, no account.

HOW IT WORKS
Visit a paper on PubMed, bioRxiv, Nature, or any scientific website. Open the sidebar and click Scan. BioGist detects 18 types of biological entities on the page. Click any entity for details, database links, and related papers from PubMed.

WHAT IT DETECTS
BioGist recognizes genes, variants, dataset accessions, species, bioinformatics methods, genome builds, sample sizes, statistical methods, sequencing platforms, cell lines, tissues, drugs, clinical trial IDs, funding sources, code repositories, p-values, file links, and key findings — 18 entity types in total. Gene detection uses the full HGNC set of 44,959 human gene symbols.

DETAIL PANELS
Click any detected entity to see rich details pulled from public databases. Gene panels show summaries from NCBI and UniProt. Variant panels show allele frequencies from gnomAD and clinical significance from ClinVar. DOI panels show citation counts, authors, and open access links via OpenAlex, with citation export in APA, Vancouver, Harvard, BibTeX, or RIS format.

INLINE PUBMED SEARCH
Click "Find Related Papers" in any entity detail to see the top 5 PubMed results directly in the sidebar — titles, authors, journal, and year — without leaving the page.

DATABASE LINKS
Every entity links out to relevant databases including NCBI, Ensembl, UniProt, GeneCards, OMIM, ClinicalTrials.gov, DrugBank, Cellosaurus, bio.tools, NIH Reporter, PubMed, and Google Scholar.

RESEARCH TOOLS
- Compare two papers side by side to see shared and unique entities
- Co-occurrence matrix showing which entity types appear together across tabs
- Persistent entity history with search and type filter across sessions
- Batch scan up to 20 URLs at once
- Personal annotation notes on any entity
- Export as plain text, JSON, Markdown, CSV, or BibTeX with scope and type filters

ADDITIONAL FEATURES
- Adaptive toolbar that collapses into a menu on narrow sidebars
- Multi-tab scanning with per-tab result storage
- Pin important entities across sessions
- Keyboard shortcuts for sidebar toggle and scan
- Right-click context menu for quick lookups
- Paste button for scanning text from PDFs
- Entity type toggles to show or hide any of the 18 types
- Light and dark themes
- 24-hour local caching of API results

PRIVACY
All entity detection runs locally in your browser. No page content is uploaded anywhere. The extension only contacts public APIs (NCBI, UniProt, myvariant.info, CrossRef, OpenAlex) when you click an entity to view details. No analytics, no tracking, no account required.

Report issues: https://github.com/oriclabs/biolang/issues
Built by Oric Labs — https://lang.bio

## Category
Developer Tools

## Language
English


# Privacy Practices

## Single purpose description
Detect biological entities on webpages and show contextual information from public bioinformatics databases. Detects 18 entity types including genes, variants, species, drugs, and clinical trials — all processed locally.

## Permission justifications

### activeTab
Used to read the visible text content of the current webpage when the user clicks "Scan" to detect biological entities. Only activated by explicit user action.

### sidePanel
Displays the BioGist sidebar panel where detected entities, details, PubMed results, and export options are shown.

### storage
Stores user preferences (theme, pinned entities, entity type toggles, annotation notes, entity history) and caches API responses locally for 24 hours to reduce redundant network requests. All data stays on the user's device.

### contextMenus
Adds "Look up in BioGist" and "Scan selection in BioGist" to the right-click menu, allowing users to scan selected text or look up specific terms.

### scripting
Used together with activeTab to inject the content script into the current page when the user clicks "Scan". The script is only injected on demand — never in the background. No host permissions are declared; injection is limited to the active tab at the moment the user initiates a scan.

### tabs
Used to track which tab's scan results to display in the sidebar, detect tab switches, populate the tab dropdown selector, and manage batch scan operations.

### webNavigation
Used to detect page navigation events so that stale entity highlights are cleaned up when the user navigates to a new page within the same tab.

## Remote code
This extension does not use remote code. All JavaScript runs from files bundled within the extension package. The only network requests are to public bioinformatics APIs (NCBI, UniProt, myvariant.info, CrossRef, OpenAlex) when the user explicitly requests entity details, PubMed results, or DOI citations.

## Data use disclosure

### What data do you collect?
None of the listed categories apply. BioGist does not collect personally identifiable information, health information, financial information, authentication information, personal communications, location data, web history, user activity, or website content.

### Certification
- Data is NOT sold to third parties
- Data is NOT used for purposes unrelated to the extension's functionality
- Data is NOT used for creditworthiness or lending purposes

## Privacy policy URL
https://lang.bio/privacy

## Firefox Notes to Reviewer
No account, API key, or login is required.

To test:
1. Navigate to any PubMed paper (e.g. pubmed.ncbi.nlm.nih.gov/35882136)
2. Click the BioGist icon to open the sidebar
3. Click "Scan" to detect genes, variants, drugs, and species
4. Click any detected entity for details from NCBI/UniProt/ClinVar
5. Right-click selected text for "Look up in BioGist"
6. Try Compare, Export, and History features

Entity detection runs locally using bundled JavaScript and the HGNC gene symbol list. Network requests only occur when the user clicks an entity to fetch details from public APIs (NCBI, UniProt, myvariant.info, CrossRef, OpenAlex).

No data is collected or shared. All innerHTML uses escapeHtml() sanitization.

## Firefox Categories
Search Tools, Web Development

## Firefox License
MIT License


# Microsoft Edge Add-ons

## Short Description
Biological entity detector for research papers. Auto-detect genes, variants, drugs, species on any webpage. Inline PubMed search. Local.

## Category
Productivity

## Search Terms (7 terms, max 30 chars each, max 21 words total)
1. gene variant detector sidebar
2. bioinformatics research paper
3. PubMed inline search
4. genomics entity recognition
5. biological data annotation
6. NCBI UniProt ClinVar lookup
7. science paper analysis tool

## Notes for Certification (less than 2000 chars)

BioGist is a sidebar extension that scans webpages for biological entities and shows contextual details from public databases. To test:

1. Navigate to any PubMed paper (e.g. pubmed.ncbi.nlm.nih.gov/35882136).
2. Click the BioGist icon to open the sidebar.
3. Click "Scan" to detect entities on the page. Genes, variants, drugs, species, and 14 other entity types are detected.
4. Click any detected entity (e.g. a gene name) to see details from NCBI, UniProt, gnomAD, and ClinVar.
5. Click "Find Related Papers" in any entity detail to see PubMed results in the sidebar.
6. Right-click selected text on any page. Context menu shows "Look up in BioGist" and "Scan selection in BioGist".
7. Try the Compare, Export, and History features from the toolbar.

No account or API key is required. Entity detection runs locally using bundled JavaScript. Network requests are made only when the user clicks an entity to fetch details from public APIs (NCBI E-utilities, UniProt REST, myvariant.info, CrossRef, OpenAlex). No page content or user data is uploaded to any server operated by the developer.

The extension uses activeTab + scripting to read page text on demand (user clicks Scan), sidePanel for the sidebar UI, storage for preferences and 24-hour API cache, contextMenus for right-click options, tabs for multi-tab tracking, and webNavigation for cleanup on page navigation.

Keyboard shortcut: Ctrl+Shift+G toggles the sidebar.

## Website
https://lang.bio/biogist

## Support URL
https://github.com/oriclabs/biolang/issues

## Privacy Policy URL
https://lang.bio/privacy


# Firefox Add-ons

## Add-on ID
biogist@oriclabs.com

## Category
Search Tools

## Tags
biology, bioinformatics, genomics, genes, research, pubmed

## License
MIT
