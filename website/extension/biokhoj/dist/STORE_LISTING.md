# BioKhoj — Store Listings

---

# Chrome Web Store

## Name
BioKhoj

## Short Description (132 chars max)
Literature monitor for biology. Track genes, drugs, and variants across PubMed. Signal scoring ranks papers by relevance. Runs locally.

## Detailed Description

BioKhoj monitors PubMed and bioRxiv for papers matching your research interests. Add genes, drugs, variants, or diseases to a watchlist. The extension checks for new publications every 4 hours and ranks each paper on a 0-100 signal score.

HOW IT WORKS
Type an entity into the watchlist (for example BRCA1, olaparib, or rs1801133). BioKhoj searches PubMed, scores each result based on six factors (recency, journal tier, entity matches, co-mentions, citation velocity, author reputation), and presents papers in a ranked feed. All data stays in your browser.

SIDEBAR TABS
Recent: paper feed with date grouping, signal bars, unread dots, search and sort.
Watch: manage entities, see activity indicators, start with preset packs.
Search: query PubMed, NCBI Gene, ClinVar, ClinicalTrials.gov, and UniProt at once.
Hot: recent preprints from bioRxiv, medRxiv, PubMed, and Europe PMC.

FEATURES
- Signal score breakdown (click the badge to see component scores)
- Right-click selected text to add a gene or variant to your watchlist
- Right-click PubMed or bioRxiv links to save papers to your reading list
- Background checks every 4 hours with notifications for high-scoring papers
- Color-coded entity chips for visual scanning
- Mark read or unread, save, cite, and export papers
- Browsing history of opened papers
- Dark and light themes
- Keyboard shortcuts (Ctrl+Shift+K to toggle, 1-4 for tabs, j/k to navigate)

ALSO AVAILABLE AS A WEB APP
A full-page version is available at https://lang.bio/biokhoj with additional features including trends charts, reading list export (BibTeX, RIS, Markdown, CSV, pandas, R), weekly digest, and journal club tools.

SUPPORTED ENTITY TYPES
Genes (BRCA1, TP53), variants (rs1801133, HGVS notation), drugs (olaparib, trastuzumab), diseases (Alzheimer, DOID identifiers), pathways (GO terms, Reactome IDs), species, cell types, cytobands, and free-text topics.

PRIVACY
All data is stored locally in your browser. API calls go directly from your browser to PubMed and bioRxiv. There is no proxy server, no analytics, no tracking, and no account.

Report issues: https://github.com/oriclabs/biolang/issues
Built by Oric Labs — https://lang.bio

## Category
Productivity

## Language
English

## Single Purpose Description
Monitors PubMed for papers matching a user-defined watchlist of genes, drugs, and variants. Scores and ranks results locally in the browser.

## Privacy Practices

### Does your extension use remote code?
No.

### Data use disclosures
This extension does not collect, transmit, or sell user data. All data is stored locally using the Chrome storage API and is never sent to any external server operated by the developer.

### Permission Justifications

**activeTab**
Required to open the sidebar panel when the user clicks the extension icon. Also used by the context menu actions ("Watch in BioKhoj" and "Lookup in BioKhoj") to read selected text from the current page. This permission is only activated by direct user action (clicking the icon or using the context menu). No page content is accessed without explicit user interaction.

**sidePanel**
Required to display the BioKhoj sidebar as a Chrome side panel. The sidebar contains four tabs (Recent papers, Watchlist, Database search, Trending) and does not read or modify any page content.

**storage**
Required to persist the user's data locally between sessions. Stored data includes: watchlist entries (entity names and types), paper metadata fetched from PubMed (titles, authors, journals, dates, signal scores), user preferences (theme selection, last active tab, notification settings), and browsing history (titles and URLs of papers the user opened). All data resides on the user's device. No data is uploaded, transmitted, or shared. Users can clear all stored data through the extension's Settings panel or through Chrome's built-in extension storage management.

**contextMenus**
Required to add right-click options to the browser context menu. Three menu items are registered:
1. "Watch in BioKhoj" — appears when the user selects text. Classifies the selected text as a biological entity (gene, drug, variant, etc.) and adds it to the local watchlist.
2. "Lookup in BioKhoj" — appears when the user selects text. Opens the sidebar to display information about the selected entity.
3. "Save to BioKhoj reading list" — appears on links pointing to PubMed, bioRxiv, DOI, NCBI Gene, ClinVar, Nature, Science, or Cell. Fetches the paper's metadata from PubMed and saves it locally.

**alarms**
Required to schedule periodic background checks. An alarm fires every 4 hours to search PubMed for new papers matching the user's watchlist. The search runs entirely within the extension's service worker. No user data is sent to any server other than the standard PubMed E-utilities API (eutils.ncbi.nlm.nih.gov).

**notifications**
Required to alert the user when papers with a signal score of 70 or above are found during background checks. Each notification shows the paper title, signal score, matched entity, and journal name. Users can disable notifications in the extension's settings. No notification data is sent to any external service.

### Host permissions
None. This extension does not request access to any host URLs.

### Content scripts
None. This extension does not inject content scripts into any web pages.

## Firefox Notes to Reviewer
No account, API key, or login is required.

To test:
1. Click the extension icon to open the sidebar
2. Type "BRCA1" and click Watch
3. Papers load automatically from PubMed and bioRxiv
4. Switch tabs: Recent, Watch, Search, Hot
5. In Search tab, type "CRISPR" and click Search
6. Right-click selected text for "Watch in BioKhoj"

API calls go directly from the browser to public databases (PubMed E-utilities, bioRxiv API, OpenAlex, ClinicalTrials.gov, UniProt). No proxy server is used.

No data is collected or shared. All storage is local via browser storage API.

## Firefox Categories
Alerts & Updates, Search Tools

## Firefox License
MIT License

---

# Microsoft Edge Add-ons

## Name
BioKhoj

## Short Description
Literature monitor for biology. Track genes, drugs, and variants across PubMed. Signal scoring. Runs locally.

## Detailed Description
(Same as Chrome Web Store description above.)

## Category
Productivity

## Search Terms (7 terms, max 30 chars each, max 21 words total)
1. pubmed literature monitor
2. bioinformatics research
3. gene variant tracker
4. bioRxiv preprint feed
5. scientific paper scoring
6. genomics watchlist
7. biology research radar

## Notes for Certification (testers only, max 2000 chars)

BioKhoj is a literature monitoring sidebar for biology researchers. To test:

1. Click the extension icon to open the sidebar.
2. Type "BRCA1" in the input at the top and click "Watch". The extension will search PubMed and bioRxiv for papers mentioning BRCA1. Results appear in the Recent tab within a few seconds.
3. Try adding more entities: "TP53" (gene), "olaparib" (drug), "rs1801133" (variant).
4. Switch between tabs: Recent (paper feed), Watch (entity list), Search (database lookup), Hot (trending preprints).
5. In the Search tab, type "CRISPR" and click Search. Results from PubMed, NCBI Gene, ClinVar, ClinicalTrials.gov, and UniProt appear grouped by source.
6. Right-click any selected text on a webpage. Context menu shows "Watch in BioKhoj" and "Lookup in BioKhoj".
7. Right-click a PubMed link (e.g. pubmed.ncbi.nlm.nih.gov/12345). Context menu shows "Save to BioKhoj reading list".

No account or API key is required. All API calls go directly from the browser to public databases (PubMed E-utilities, bioRxiv API, OpenAlex, ClinicalTrials.gov, UniProt REST). No data is sent to any server operated by the developer.

The extension uses chrome.storage.local for persistence, alarms for periodic background checks (every 4 hours), and notifications to alert users when high-scoring papers are found. The sidebar panel (sidePanel API) displays all content. No content scripts are injected into any page.

Keyboard shortcut: Ctrl+Shift+K toggles the sidebar.

## Privacy Policy URL
https://lang.bio/privacy

## Support URL
https://github.com/oriclabs/biolang/issues

## Website
https://lang.bio/biokhoj

---

# Firefox Add-ons (AMO)

## Name
BioKhoj

## Summary
Literature monitor for biology. Track genes, drugs, and variants across PubMed. Signal scoring. Runs locally.

## Description
(Same as Chrome Web Store description above.)

## Add-on ID
biokhoj@oriclabs.com

## Category
Search Tools

## Tags
biology, bioinformatics, pubmed, research, genomics, literature

## Support URL
https://github.com/oriclabs/biolang/issues

## Homepage
https://lang.bio/biokhoj

## License
MIT

## Privacy Policy
All data is stored locally in the user's browser. No data is collected, transmitted, or shared with any external server. API calls go directly from the browser to public databases (PubMed, bioRxiv, OpenAlex, ClinicalTrials.gov, UniProt). There is no proxy, no analytics, and no tracking.

---

# Screenshots

Original screenshots: website/biokhoj/screenshots/store/ (~1685x1255)
Submission-ready (1280x800): website/biokhoj/screenshots/store-submit/
To generate resized copies: cd website/biokhoj/screenshots && bash resize-store.sh

Recommended submission order (pick 5 for Chrome, up to 10 for Edge/Firefox):

1. **recent.png** — Recent tab with paper feed, signal bars, unread dots, date groups
2. **watch.png** — Watch tab with color-coded entity chips
3. **search.png** — Search tab with unified database results grouped by source
4. **whatshot.png** — Hot tab with trending preprints from multiple sources
5. **settings.png** — Settings panel with API key, theme, notifications
6. **reading.png** — Reading list overlay with saved papers
7. **trends.png** — Trends overlay with publication charts
8. **digest.png** — Weekly digest summary
9. **export.png** — Export options
