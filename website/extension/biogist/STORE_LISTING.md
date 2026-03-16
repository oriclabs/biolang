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
