# BioKhoj — Requirements

**Tagline:** Your personal research radar for biology.

**Positioning:** Bloomberg terminal for biology signals — research awareness infrastructure, not a reference manager or paper reader.

**What it does:** Monitors PubMed and bioRxiv for new papers mentioning your watched genes, drugs, variants, and other biological entities. Ranks papers by Signal Score, suggests related concepts, and adapts check frequency to entity activity. Available as a Chrome extension (sidebar) and a PWA (standalone app).

---

## Architecture

| Component | Purpose |
|---|---|
| **PWA** (`lang.bio/biokhoj`) | Standalone app — dashboard, feed, trends, reading list |
| **Extension** (Chrome sidebar) | Quick watch from BioGist, page relevance, alerts |
| **Shared core** (`biokhoj-core.js`) | Watchlist logic, PubMed/bioRxiv API, storage schema |

### Data Sources
- **PubMed E-utilities** — `esearch` + `esummary` for paper metadata (free, 10 req/s with API key)
- **bioRxiv API** — `api.biorxiv.org/details` for preprints (free, no key)
- **OpenAlex API** — citation counts, author data, related works (free, no key)

### Storage
- **IndexedDB** — watchlist, paper cache, read/unread state, trends (PWA)
- **chrome.storage.local** — watchlist mirror, alert state (extension)
- **No server, no account** — 100% client-side, privacy-first

### Background Scheduling
- **PWA:** Service Worker + Periodic Background Sync API
- **Extension:** `chrome.alarms` API (rotates through watchlist per wake cycle)

---

## PWA Features

### 1. Dashboard
- Feed of new papers grouped by watched entity
- Unread count per entity
- Sort by: date, relevance, citation count
- Filter by: entity type, journal, date range

### 2. Watchlist Manager
- Add entities: gene names, drug names, variant IDs, diseases, author names
- Set priority per entity: high / normal / low
- Organize with tags (e.g., "thesis project", "grant proposal", "lab meeting")
- Bulk import from BioGist export (JSON)
- Edit, pause, or remove watched entities

### 3. Paper Feed
- Each paper shows: title, authors, journal, date, abstract preview, **Signal Score badge**
- Highlight which watched entities appear in each paper
- Mark as read / unread / starred
- Quick actions: open paper, copy citation, add to reading list, share
- Sort by: date (default), Signal Score, citation count
- Filter: high signal only, unread only, specific entity

### 4. Signal Score (v1 core differentiator)

Every paper gets a **Signal Score** (0–100) computed locally — no AI needed:

```
signal = recency_weight        (0-25)  # newer = higher
       + citation_velocity     (0-20)  # citations/month since publication
       + journal_tier          (0-15)  # top journals score higher
       + co_mention_novelty    (0-20)  # first time entity A + B appear together
       + entity_match_count    (0-10)  # how many watched entities appear
       + author_reputation     (0-10)  # known authors in the field (via OpenAlex)
```

**Display:**
- Papers with signal >= 70: ★ **High signal** (purple badge)
- Papers with signal 40-69: normal (no badge)
- Papers with signal < 40: ⚠ **Low signal** (grey, collapsed by default)

**Journal tier list** (built-in, editable in settings):
- Tier 1 (15pts): Nature, Science, Cell, NEJM, Lancet, JAMA
- Tier 2 (10pts): Nature Genetics, Nature Medicine, PNAS, Genome Research, etc.
- Tier 3 (5pts): PLOS, BMC, Frontiers, MDPI, etc.
- Unranked (0pts): preprints, unknown journals

**Citation velocity** sourced from OpenAlex (free API, no key).

### 5. Concept Expansion Engine (v1)

When a user watches an entity, BioKhoj suggests related concepts using **MeSH terms** and **OpenAlex related concepts**:

**How it works:**
1. User adds "TP53" to watchlist
2. BioKhoj queries OpenAlex: `GET https://api.openalex.org/concepts?search=TP53`
3. Returns related concepts: MDM2, apoptosis, DNA damage response, Li-Fraumeni syndrome, p21
4. Shows suggestion card: "Also watch?" with one-click add buttons

**Sources for expansion:**
- **OpenAlex concepts API** — related concepts with relevance scores
- **MeSH tree** — parent/child/sibling terms from NCBI
- **Co-mention data** — entities frequently appearing in same papers (from BioKhoj's own cache)

**Display:**
- On first add: "Related entities" card below the watchlist entry
- In Discovery tab: "Suggested for you" section
- Smart dedup: don't suggest entities already in watchlist

**Not AI — just graph lookups.** Feels intelligent, runs locally, no API key needed.

### 6. Trend Charts
- Publication frequency over time per entity (line chart)
- Compare trends across entities on same chart
- Time ranges: 1 month, 3 months, 1 year, 5 years
- Sparkline preview in watchlist

### 7. Discovery Feed
- "Based on your watchlist, you might be interested in..."
- Powered by Concept Expansion Engine (see above)
- Suggest entities that frequently co-appear with your watched entities
- Suggest trending entities in your field
- One-click add to watchlist

### 8. Co-mention Alerts
- Detect novel entity pairs across papers
- "First paper linking [entity A] + [entity B] found"
- Show co-mention frequency over time
- Highlight unexpected co-mentions (entities not previously seen together)

### 9. Author Tracking
- Watch specific authors (by ORCID or name)
- Get notified on new publications
- Author profile: publication count, h-index (via OpenAlex), recent papers

### 10. Journal Filter
- Whitelist/blacklist journals
- Preset journal lists: "Top genomics", "Clinical oncology", "Bioinformatics"
- Impact factor indicator

### 11. Reading List
- Save papers for later reading
- Add personal notes per paper
- Tag papers with custom labels
- Sort by date added, priority, or entity
- Export as BibTeX, RIS, or Markdown

### 12. Weekly Digest
- Auto-generated summary of the week's new papers
- Grouped by entity, sorted by relevance
- Highlights: trending entities, novel co-mentions, top-cited new papers
- Export as Markdown or PDF for lab meetings
- Optional: email digest (requires email input, no account)

### 13. Push Notifications
- "5 new papers for TP53 today"
- Configurable: per entity, per priority, or daily summary only
- Works even when app is closed (Web Push API)
- Notification settings in app

### 14. Offline Reading
- Cache paper abstracts for offline review
- Mark papers for offline before commute/flight
- Sync new papers when back online

### 15. Export & Sharing
- Export watchlist as JSON (import into BioGist or share)
- Export reading list as BibTeX / RIS / Markdown
- Shared watchlist via URL (encoded in query params, no account needed)
- Weekly report export as Markdown / PDF

### 16. BioGist Sync (one-way for v1)
- Import watchlist from BioGist export (JSON) — one-click in BioKhoj
- Export BioKhoj watchlist as JSON (compatible with BioGist import)
- Shared entity format between both tools (`{ type, id }`)
- **v1: one-way only** (BioGist → BioKhoj). Bidirectional sync deferred to v2 to avoid conflict resolution complexity

---

## Extension Features

### 17. Watch from BioGist
- Right-click any entity in BioGist sidebar → "Watch in BioKhoj"
- Context menu on any webpage: "Watch in BioKhoj"
- Bulk watch: select multiple entities in BioGist → "Watch all"

### 18. Quick Watchlist Sidebar
- Sidebar shows watched entities with new paper counts
- Expand entity to see latest 3 papers
- Click paper to open in new tab
- Badge on extension icon: total new papers count

### 19. Page Relevance Badge
- When visiting a paper, show: "This page mentions 4 of your watched entities"
- Highlight which watched entities appear on the page
- One-click add new entities from the page to watchlist

### 20. Paper Alerts
- Badge icon shows total new papers since last check
- Click badge to open sidebar with latest papers
- Notification popup for high-priority entities

### 21. One-Click Watch
- Select any text on any page → right-click → "Watch in BioKhoj"
- Auto-classify entity type (gene, drug, variant, etc.)
- Confirm dialog with entity type and watch settings

---

## Shared Core (`biokhoj-core.js`)

### Data Schema

```javascript
// Watched entity
{
  id: "BRCA1",
  type: "gene",           // gene, drug, variant, disease, author, custom
  priority: "high",        // high, normal, low
  tags: ["thesis"],
  paused: false,
  addedAt: 1710000000000,
  lastChecked: 1710086400000,
  newPaperCount: 3
}

// Cached paper
{
  pmid: "39876543",
  doi: "10.1234/example",
  title: "...",
  authors: ["Smith J", "Doe A"],
  journal: "Nature",
  date: "2026-03-16",
  abstract: "...",
  matchedEntities: ["BRCA1", "olaparib"],
  source: "pubmed",        // pubmed, biorxiv
  read: false,
  starred: false,
  notes: "",
  savedAt: 1710000000000
}

// Co-mention record
{
  entityA: "BRCA1",
  entityB: "sotorasib",
  firstSeen: 1710000000000,
  paperCount: 1,
  papers: ["39876543"]
}

// Trend data point
{
  entity: "BRCA1",
  month: "2026-03",
  count: 47
}
```

### API Wrappers

```javascript
// PubMed search
async function searchPubMed(query, maxResults = 10, minDate = null)

// bioRxiv search
async function searchBioRxiv(query, maxResults = 10, days = 7)

// OpenAlex enrichment
async function enrichPaper(doi)

// Trend data
async function getTrendData(entity, months = 12)
```

### Rate Limiting

PubMed allows 3 req/s without an API key, 10 req/s with `NCBI_API_KEY`. BioKhoj must respect this strictly:

- **Request queue** with 350ms delay between calls (no API key) or 100ms (with key)
- **Priority ordering** — high-priority entities checked first
- **Max 5 entities per wake cycle** — prevents long-running service worker (MV3 kills after 30s)
- **Exponential backoff** on HTTP 429 (rate limited) — wait 2s, 4s, 8s, then skip cycle
- **Optional NCBI API key** in settings — increases throughput from 3 to 10 req/s

### Adaptive Polling (v1)

Not all entities need the same check frequency. BioKhoj adapts automatically:

| Entity Activity | Check Frequency | Logic |
|---|---|---|
| Hot (>5 papers/week) | Every 6 hours | High publication velocity |
| Active (1-5 papers/week) | Daily | Normal activity |
| Moderate (<1 paper/week) | Every 3 days | Low activity |
| Rare (<1 paper/month) | Weekly | Very low activity |
| Clinical trial drug | Daily (forced) | Time-sensitive |

**How it works:**
- Track `papersFoundLastWeek` per entity
- After each check cycle, recalculate polling interval
- Store `nextCheckAt` timestamp per entity
- Background scheduler only checks entities where `now >= nextCheckAt`
- User can override: force "check daily" on any entity

**Benefit:** A user watching 200 entities doesn't hit rate limits — most are checked weekly, only hot ones daily. Reduces API calls by ~80% vs fixed polling.

### Background Check Logic

```
1. On alarm/sync trigger:
2. Get watchlist sorted by: priority DESC, lastChecked ASC (high-priority + oldest first)
3. Pick top N entities (N = 5 per cycle to stay within rate limits)
4. For each entity (with 350ms delay between requests):
   a. Query PubMed: esearch with date filter (since lastChecked)
   b. If 429 response: backoff and stop cycle
   c. Query bioRxiv: /details endpoint with date filter
   d. Deduplicate by DOI
   e. Store new papers in Dexie.js (IndexedDB)
   f. Update newPaperCount
   g. Check for novel co-mentions (indexed query via Dexie)
   h. Update lastChecked timestamp
5. Update badge count
6. Fire push notification if high-priority entity has new papers
```

---

## UI Design

### PWA Layout
- **Header:** BioKhoj logo + search + notification bell + settings gear
- **Left sidebar (desktop):** Watchlist with sparklines + new counts
- **Main area:** Paper feed / trend charts / discovery (tab switching)
- **Mobile:** Bottom tab bar (Feed / Watchlist / Trends / Reading List / Settings)

### Color Scheme
- **Primary: Saffron** (`#F4C430` / amber-400 range) — buttons, highlights, active states, badges
- **Background:** Dark slate (`#0f172a` / `#020617`) for dark mode, `#fafaf9` for light
- **Accents:** Saffron-tinted amber/yellow gradients
- **Text:** Slate-200 on dark, slate-800 on light
- **Entity type colors:** Match BioGist badge colors for consistency
- Light and dark theme support
- Tailwind CSS custom color: `saffron: { 50: '#fffbeb', 100: '#fef3c7', 200: '#fde68a', 300: '#fcd34d', 400: '#F4C430', 500: '#d4a017', 600: '#b8860b', 700: '#92400e', 800: '#78350f', 900: '#451a03' }`

### Extension Layout
- Same sidebar panel pattern as BioGist
- Header: BioKhoj logo + refresh + settings
- Watchlist with expandable paper previews
- Bottom: "Open BioKhoj" link to PWA for full features

---

## Technical Stack

| Component | Technology |
|---|---|
| PWA framework | Vanilla JS (like BioGist — no build step) |
| Storage | IndexedDB via Dexie.js (cleaner API, indexed queries for co-mentions/trends) |
| Charts | Chart.js (lightweight, no build needed) |
| Push notifications | Web Push API + Notification API |
| Background sync | Service Worker + Periodic Background Sync |
| Extension | Chrome MV3, `chrome.alarms`, `chrome.storage` |
| APIs | PubMed E-utilities, bioRxiv API, OpenAlex API |
| Styling | Tailwind CSS (CDN) with saffron (#F4C430) as primary color |

---

## Privacy

- **No server, no account** — all data stored locally
- **No tracking** — no analytics, no telemetry
- **API calls only for paper queries** — PubMed, bioRxiv, OpenAlex (all public, no auth)
- **Shared watchlists** — encoded in URL, no server storage
- **Push notifications** — optional, user-initiated subscription

---

## Permissions (Extension)

| Permission | Justification |
|---|---|
| `activeTab` | Read page text to detect watched entities on current page |
| `sidePanel` | Display BioKhoj sidebar |
| `storage` | Store watchlist, paper cache, settings |
| `alarms` | Schedule periodic background checks |
| `notifications` | Show new paper alerts |
| `contextMenus` | "Watch in BioKhoj" right-click menu |

---

## Out of Scope (v1)

| Feature | Reason | Revisit |
|---|---|---|
| LLM/AI summarization | Privacy contradiction (sends data to API), API key complexity, scope creep | v2 if users request |
| Bidirectional BioGist sync | Conflict resolution complexity for v1 | v2 |
| Full-text PDF indexing | Heavy processing, storage bloat | v2 |
| User accounts / cloud sync | Contradicts privacy-first model | Never (or opt-in v3) |
| Reference manager (full) | Competes with Zotero/Mendeley — stay lightweight | Never |

---

## Development Plan — Single v1 Release (MVP with all features)

All 19 features ship in v1. Two features deferred to v2.

### Build Order (dependency-driven)

**Week 1 — Foundation**
1. Project setup: PWA scaffold, Dexie.js schema, Tailwind CSS with saffron theme
2. Watchlist manager (add/remove/edit/tag/priority)
3. Entity auto-classification (regex from BioGist core + OpenAlex lookup)
4. PubMed + bioRxiv API wrappers with rate limiting queue (350ms delay, backoff)
5. Adaptive polling engine (hot/active/moderate/rare intervals)

**Week 2 — Feed & Intelligence**
6. Paper feed with read/unread/starred
7. Signal Score computation (recency + citation velocity + journal tier + co-mention + entity match + author)
8. Signal Score breakdown popover ("why this score?")
9. Concept Expansion Engine (OpenAlex concepts + MeSH suggestions)
10. Co-mention detection and alerts (indexed via Dexie)
11. bioRxiv dedup logic (DOI match + fuzzy title)

**Week 3 — Research Tools**
12. Trend charts (Chart.js, sparklines in watchlist)
13. Discovery feed (powered by Concept Expansion + co-mention data)
14. Author tracking (OpenAlex author lookup, new paper alerts)
15. Reading list (save/notes/tags/sort)
16. Journal filter (whitelist/blacklist, built-in tier list)
17. Weekly digest (auto-generated Markdown/PDF)
18. Export (BibTeX/RIS/Markdown, watchlist JSON)

**Week 4 — Extension & Polish**
19. Chrome extension sidebar (quick watchlist, paper alerts, badge)
20. Watch from BioGist integration (context menu, bulk watch)
21. Page relevance badge ("This page mentions 4 of your watched entities")
22. One-click watch (select text → right-click → watch)
23. Push notifications (Web Push API, configurable per entity/priority)
24. Rate limit warning modal ("consider API key or lower priorities")
25. First-run demo (pre-loaded watchlist + cached papers)
26. BioGist sync (one-way import/export JSON)
27. Dashboard (stats, unread counts, trending sparklines)

### First-Run Demo (critical for adoption)

Onboarding must deliver value in under 30 seconds:

1. **Pre-loaded watchlist:** 3 trending entities (e.g., TP53, CRISPR-Cas9, semaglutide)
2. **Cached results:** ~15 pre-fetched papers so feed is instant on first open (no loading spinner)
3. **Signal Score visible:** papers already scored, high-signal papers starred
4. **Concept suggestions shown:** "Also watch MDM2? apoptosis? GLP-1?" cards visible
5. **Gentle prompt:** "Add your own entities or keep these to explore"
6. **One-click clear:** "Start fresh" button removes demo data

**Why:** Researchers hate empty states. Showing a working feed with real papers immediately proves value.

### Deferred to v2

| Feature | Reason |
|---|---|
| Offline reading | PWA service worker caching handles basic offline automatically |
| Shared watchlists (URL-encoded) | No users to share with at launch — add when there's a community |

---

## File Structure

```
website/extension/biokhoj/
├── REQUIREMENTS.md          # This file
├── STORE_LISTING.md         # Chrome Web Store listing
├── shared/
│   ├── biokhoj-core.js      # Shared logic (API, watchlist, storage)
│   ├── sidebar.html         # Extension sidebar
│   ├── sidebar.js           # Extension sidebar logic
│   ├── icons/               # Extension icons
│   └── screenshots/         # Store screenshots
├── chrome/
│   ├── manifest.json        # Chrome MV3 manifest
│   └── background.js        # Service worker (alarms, notifications)
└── pwa/
    ├── index.html           # PWA main page
    ├── app.js               # PWA app logic
    ├── sw.js                # Service worker (background sync, push, offline)
    ├── manifest.json        # PWA manifest
    ├── styles.css            # Styles
    └── charts.js            # Trend visualization
```
