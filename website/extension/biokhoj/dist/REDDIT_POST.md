# BioKhoj — Free browser extension that monitors PubMed for your genes/drugs/variants and ranks papers by relevance

Hi r/bioinformatics,

**BioKhoj** ("khoj" means "search/discovery" in Hindi) is a free browser sidebar that monitors PubMed for your genes, drugs, and variants — and ranks new papers by relevance.

## What it does

You add entities to a watchlist — genes (BRCA1, TP53), drugs (olaparib), variants (rs1801133), diseases, pathways, whatever you're tracking. BioKhoj checks PubMed every 4 hours and **scores each paper 0-100** based on six factors:

- Recency
- Journal tier
- Entity match strength
- Co-mentions with your other watched entities
- Citation velocity
- Author reputation

Papers show up in a ranked feed in your browser sidebar. High-scoring papers get notifications so you don't miss them.

## Why this exists

Keeping up with the literature is a universal problem in research. PubMed alerts are email-based, unranked, and noisy. Manually checking the same searches every day is tedious. BioKhoj turns that daily ritual into a background process — you set your watchlist once and papers come to you, ranked by how relevant they are to your specific research interests.

## Key features

- **Watchlist** — track genes, drugs, variants, diseases, pathways, species, cell types, or any free-text topic
- **Signal scoring** — 0-100 relevance score with breakdown (click the badge to see component scores)
- **4 sidebar tabs** — Recent feed, Watchlist manager, Multi-database search, Trending preprints
- **Multi-database search** — query PubMed, NCBI Gene, ClinVar, ClinicalTrials.gov, and UniProt simultaneously
- **Trending** — browse recent preprints from bioRxiv, medRxiv, PubMed, and Europe PMC
- **Background checks** — notifications for high-scoring papers every 4 hours
- **Right-click integration** — select text on any page → "Watch in BioKhoj" adds it to your watchlist
- **Reading list** — save, cite, and export papers (BibTeX, RIS, Markdown, CSV)
- **Keyboard shortcuts** — Ctrl+Shift+K to toggle, 1-4 for tabs, j/k to navigate papers
- **Dark and light themes**
- **Fully local** — all data in your browser. No account, no server, no tracking

## Also available as a web app

If you prefer a full-page view, there's a PWA at [lang.bio/biokhoj](https://lang.bio/biokhoj) with additional features: trends charts, weekly digest, journal club tools, and more export formats.

## Install links

- **Chrome**: [Chrome Web Store](https://chromewebstore.google.com/detail/biokhoj/joabebeooikdbjnheamlbcnjlodoaflb)
- **Firefox**: [Firefox Add-ons](https://addons.mozilla.org/addon/biokhoj)
- **Edge**: [Edge Add-ons](https://microsoftedge.microsoft.com/addons/detail/pehdloipblabliilcdagdgimodipepap)

## Privacy

Zero data collection. API calls go directly from your browser to PubMed/bioRxiv — no proxy server in between. No analytics. No account required. You can verify this yourself — the extension has no backend.

## Limitations

- Checks run **while the browser is open** — it's not a server-side service. When you close Chrome, checks pause. Next time you open the browser, it picks up and runs the check for the configured period.
- Signal scoring is heuristic-based, not ML — works well for most cases but won't be perfect for niche topics with low publication volume.
- NCBI rate limits apply (3 requests/sec without API key, 10/sec with one). If you have a large watchlist, set your NCBI API key in settings for faster updates.

## Feedback welcome

This is a free tool built by [ORIC Labs](https://oriclabs.com). I'd love to hear:
- What entities do you track? How many?
- Do you use PubMed alerts currently? What's broken about them?
- What would make this more useful for your workflow?

Report bugs or request features: [GitHub Issues](https://github.com/oriclabs/biolang/issues)

---

*Part of the BioLang ecosystem — [lang.bio](https://lang.bio)*
