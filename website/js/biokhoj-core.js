// BioKhoj Core — Shared literature surveillance, scoring, and export logic
// No browser extension APIs (chrome.*, browser.*) — works in extension AND PWA
// Usage: const core = window.BioKhojCore;

(function() {
  "use strict";

  // ════════════════════════════════════════════════════════════════════
  // Constants
  // ════════════════════════════════════════════════════════════════════

  const PUBMED_BASE = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";
  const BIORXIV_BASE = "https://api.biorxiv.org";
  const OPENALEX_BASE = "https://api.openalex.org";

  // Signal score thresholds for categorizing papers
  const SIGNAL_THRESHOLDS = {
    critical: 80,   // must-read, immediate relevance
    high: 60,       // very relevant, read soon
    medium: 40,     // relevant, queue for later
    low: 20,        // tangential, skim abstract
    noise: 0        // below threshold, skip
  };

  // Adaptive polling intervals in milliseconds
  const POLLING_INTERVALS = {
    hot:      6  * 60 * 60 * 1000,  //   6 hours — >5 papers/week
    active:   24 * 60 * 60 * 1000,  //  24 hours — 1-5 papers/week
    moderate: 72 * 60 * 60 * 1000,  //  72 hours — <1 paper/week
    rare:    168 * 60 * 60 * 1000   // 168 hours — <1 paper/month
  };

  // Default rate limit delay (no NCBI API key)
  const DEFAULT_RATE_DELAY_MS = 350;
  // Rate limit delay with NCBI API key
  const KEYED_RATE_DELAY_MS = 100;

  // ════════════════════════════════════════════════════════════════════
  // Journal Tier List (~100 journals across 3 tiers)
  // Tier 1 = 15 pts, Tier 2 = 10 pts, Tier 3 = 5 pts
  // ════════════════════════════════════════════════════════════════════

  const JOURNAL_TIERS = {
    // ── Tier 1 (15 points) — Top-tier multidisciplinary and flagship journals ──
    "nature": 1,
    "science": 1,
    "cell": 1,
    "the new england journal of medicine": 1,
    "new england journal of medicine": 1,
    "n engl j med": 1,
    "the lancet": 1,
    "lancet": 1,
    "jama": 1,
    "nature medicine": 1,
    "nature genetics": 1,
    "nature biotechnology": 1,
    "nature methods": 1,
    "nature cell biology": 1,
    "nature immunology": 1,
    "nature neuroscience": 1,
    "nature chemical biology": 1,
    "nature structural & molecular biology": 1,
    "nature structural and molecular biology": 1,
    "nature reviews genetics": 1,
    "nature reviews molecular cell biology": 1,
    "nature reviews cancer": 1,
    "nature reviews immunology": 1,
    "nature reviews drug discovery": 1,
    "cell stem cell": 1,
    "cell metabolism": 1,
    "cell host & microbe": 1,
    "molecular cell": 1,
    "immunity": 1,
    "cancer cell": 1,
    "neuron": 1,
    "science translational medicine": 1,
    "science immunology": 1,
    "annual review of biochemistry": 1,
    "annual review of genomics and human genetics": 1,

    // ── Tier 2 (10 points) — High-impact specialty journals ──
    "genome research": 2,
    "genome biology": 2,
    "nucleic acids research": 2,
    "the american journal of human genetics": 2,
    "american journal of human genetics": 2,
    "am j hum genet": 2,
    "plos genetics": 2,
    "elife": 2,
    "proceedings of the national academy of sciences": 2,
    "proc natl acad sci u s a": 2,
    "pnas": 2,
    "embo journal": 2,
    "the embo journal": 2,
    "genes & development": 2,
    "genes and development": 2,
    "journal of clinical investigation": 2,
    "j clin invest": 2,
    "blood": 2,
    "journal of experimental medicine": 2,
    "j exp med": 2,
    "cancer discovery": 2,
    "clinical cancer research": 2,
    "journal of clinical oncology": 2,
    "j clin oncol": 2,
    "bioinformatics": 2,
    "briefings in bioinformatics": 2,
    "nature communications": 2,
    "science advances": 2,
    "current biology": 2,
    "developmental cell": 2,
    "cell reports": 2,
    "cell systems": 2,
    "molecular biology and evolution": 2,
    "mol biol evol": 2,
    "human molecular genetics": 2,
    "hum mol genet": 2,
    "genetics": 2,
    "plos computational biology": 2,
    "plant cell": 2,
    "the plant cell": 2,
    "trends in genetics": 2,
    "trends in biochemical sciences": 2,
    "trends in cell biology": 2,
    "trends in molecular medicine": 2,
    "annual review of genetics": 2,
    "annual review of cell and developmental biology": 2,
    "journal of biological chemistry": 2,
    "j biol chem": 2,

    // ── Tier 3 (5 points) — Solid, widely-read journals ──
    "plos one": 3,
    "plos biology": 3,
    "bmc genomics": 3,
    "bmc bioinformatics": 3,
    "bmc biology": 3,
    "scientific reports": 3,
    "frontiers in genetics": 3,
    "frontiers in immunology": 3,
    "frontiers in microbiology": 3,
    "frontiers in oncology": 3,
    "frontiers in cell and developmental biology": 3,
    "giga science": 3,
    "gigascience": 3,
    "peerj": 3,
    "gene": 3,
    "genomics": 3,
    "human genetics": 3,
    "journal of molecular biology": 3,
    "j mol biol": 3,
    "febs letters": 3,
    "biochemical and biophysical research communications": 3,
    "biochem biophys res commun": 3,
    "molecular biology of the cell": 3,
    "journal of cell biology": 3,
    "j cell biol": 3,
    "journal of cell science": 3,
    "dna research": 3,
    "g3: genes|genomes|genetics": 3,
    "g3 genes genomes genetics": 3,
    "genetica": 3,
    "cytogenetic and genome research": 3,
    "heredity": 3,
    "mutation research": 3,
    "epigenetics": 3,
    "epigenetics & chromatin": 3,
    "journal of proteome research": 3,
    "proteomics": 3,
    "molecular & cellular proteomics": 3,
    "molecular and cellular proteomics": 3,
    "journal of proteomics": 3,
    "analytical chemistry": 3,
    "anal chem": 3,
    "journal of the american chemical society": 3,
    "j am chem soc": 3,
    "acs chemical biology": 3,
    "nature protocols": 3,
    "methods": 3
  };

  // Tier score values
  const TIER_SCORES = { 1: 15, 2: 10, 3: 5 };

  // ════════════════════════════════════════════════════════════════════
  // Entity Classification Patterns
  // Reuses BioGist-style regex patterns for biological entities
  // ════════════════════════════════════════════════════════════════════

  const ENTITY_PATTERNS = {
    // Gene symbols: 2-6 uppercase letters/digits, optional dash+digit
    gene: /^[A-Z][A-Z0-9]{1,5}(-[A-Z0-9]{1,2})?$/,

    // rsID variants: rs followed by digits
    rsid: /^rs\d{3,12}$/i,

    // HGVS notation: NM/NP followed by underscore, accession, colon, variant
    hgvs: /^(NM_|NP_|NC_|ENST)\d+\.\d+:[a-z]\.\S+$/i,

    // ClinVar variants: clinvar-style
    clinvar: /^(RCV|VCV)\d{6,}$/i,

    // Drugs: often lowercase or mixed case, may contain hyphens
    drug: /^[a-z][a-z0-9-]{2,30}(ab|ib|mab|nib|lib|zole|pine|pril|vir|stat|olol|tide|zumab|ximab|lizumab)?$/i,

    // Species: binomial nomenclature (Genus species)
    species: /^[A-Z][a-z]+ [a-z]+$/,

    // Pathway identifiers: KEGG, Reactome, GO
    pathway_kegg: /^(hsa|mmu|dme|cel|sce)\d{5}$/,
    pathway_reactome: /^R-HSA-\d+$/,
    pathway_go: /^GO:\d{7}$/,

    // Protein domains: InterPro, Pfam
    interpro: /^IPR\d{6}$/,
    pfam: /^PF\d{5}$/,

    // Disease identifiers: OMIM, DOID, Orphanet
    omim: /^(OMIM:)?\d{6}$/,
    doid: /^DOID:\d+$/,
    orphanet: /^ORPHA(NET)?:\d+$/i,

    // Cell types
    cell_type: /^(T cell|B cell|NK cell|macrophage|neutrophil|monocyte|dendritic cell|fibroblast|epithelial|endothelial|astrocyte|neuron|hepatocyte|cardiomyocyte|stem cell|iPSC|organoid)/i,

    // Chromosome regions
    cytoband: /^(chr)?\d{1,2}[pq]\d{1,2}(\.\d{1,2})?$/i
  };

  // Common English words that are also gene symbols — exclude from gene detection
  const GENE_EXCLUDE = new Set([
    "THE","AND","FOR","NOT","WITH","FROM","BUT","ALL","ARE","WAS","WERE",
    "HAS","HAD","BEEN","HAVE","THAT","THIS","WILL","CAN","MAY","SET",
    "MAP","LET","RUN","USE","AGE","END","TOP","ACE","HER","HIS",
    "REST","CAST","TANK","WARS","IMPACT","CHANCE","CLOCK","CHIP",
    "MARCH","APRIL","JUNE","TRAP","MARK","PAGE","CELL","GENE",
    "LARGE","SMALL","FAST","LONG","LIGHT","CAR","RING","POLE"
  ]);

  // ════════════════════════════════════════════════════════════════════
  // Rate Limiting Queue
  // Single-concurrency queue with configurable delay and exponential
  // backoff on HTTP 429 responses.
  // ════════════════════════════════════════════════════════════════════

  let _ncbiApiKey = null;
  let _queueRunning = false;
  const _queue = [];

  // Search budget tracking
  const _apiBudget = {
    calls: [],       // timestamps of API calls in current hour window
    limit: 3,        // requests per second (3 without key, 10 with key)
    hourlyEstimate: 3 * 3600, // max calls per hour
  };

  function _trackApiCall() {
    const now = Date.now();
    _apiBudget.calls.push(now);
    // Keep only calls from the last hour
    const oneHourAgo = now - 3600000;
    _apiBudget.calls = _apiBudget.calls.filter(t => t > oneHourAgo);
  }

  function getApiBudget() {
    const now = Date.now();
    const oneHourAgo = now - 3600000;
    const recentCalls = _apiBudget.calls.filter(t => t > oneHourAgo);
    const perSecLimit = _ncbiApiKey ? 10 : 3;
    const hourlyLimit = perSecLimit * 3600;
    return {
      used: recentCalls.length,
      limit: hourlyLimit,
      remaining: Math.max(0, hourlyLimit - recentCalls.length),
      perSecond: perSecLimit,
      hasApiKey: !!_ncbiApiKey,
      pct: Math.round((recentCalls.length / hourlyLimit) * 100)
    };
  }

  /**
   * Set the NCBI API key for faster rate limits (100ms vs 350ms).
   * @param {string|null} key - NCBI API key or null to clear
   */
  function setNcbiApiKey(key) {
    _ncbiApiKey = key || null;
    _apiBudget.limit = _ncbiApiKey ? 10 : 3;
    _apiBudget.hourlyEstimate = _apiBudget.limit * 3600;
  }

  /**
   * Get current rate limit delay based on API key presence.
   * @returns {number} Delay in milliseconds
   */
  function _getRateDelay() {
    return _ncbiApiKey ? KEYED_RATE_DELAY_MS : DEFAULT_RATE_DELAY_MS;
  }

  /**
   * Sleep for the given number of milliseconds.
   * @param {number} ms
   * @returns {Promise<void>}
   */
  function _sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  /**
   * Enqueue an async function to be executed with rate limiting.
   * All API calls should go through this to respect NCBI rate limits.
   * Max concurrency: 1. Exponential backoff on 429 (up to 3 retries).
   * @param {Function} fn - Async function that performs the API call
   * @returns {Promise<*>} Result of fn()
   */
  function enqueueApiCall(fn) {
    return new Promise((resolve, reject) => {
      _queue.push({ fn, resolve, reject });
      _processQueue();
    });
  }

  /**
   * Process the rate-limited queue sequentially.
   * @private
   */
  async function _processQueue() {
    if (_queueRunning) return;
    _queueRunning = true;

    while (_queue.length > 0) {
      const { fn, resolve, reject } = _queue.shift();
      let retries = 0;
      const maxRetries = 3;

      while (retries <= maxRetries) {
        try {
          const result = await fn();
          resolve(result);
          break;
        } catch (err) {
          // Exponential backoff on rate limit (429) or server error (5xx)
          const status = err && err.status;
          if ((status === 429 || (status >= 500 && status < 600)) && retries < maxRetries) {
            const backoff = Math.pow(2, retries) * 1000;
            console.warn(`[BioKhoj] Rate limited (${status}), retrying in ${backoff}ms (attempt ${retries + 1}/${maxRetries})`);
            await _sleep(backoff);
            retries++;
          } else {
            reject(err);
            break;
          }
        }
      }

      // Wait between calls to respect rate limits
      await _sleep(_getRateDelay());
    }

    _queueRunning = false;
  }

  /**
   * Rate-limited fetch wrapper. Throws an object with { status, message }
   * on HTTP error so the queue can detect 429s.
   * @param {string} url
   * @param {object} [options]
   * @returns {Promise<Response>}
   */
  async function _rateFetch(url, options) {
    _trackApiCall();
    const resp = await fetch(url, options);
    if (!resp.ok) {
      const err = new Error(`HTTP ${resp.status}: ${resp.statusText}`);
      err.status = resp.status;
      throw err;
    }
    return resp;
  }

  // ════════════════════════════════════════════════════════════════════
  // PubMed API Wrapper (NCBI E-utilities)
  // ════════════════════════════════════════════════════════════════════

  /**
   * Search PubMed for papers matching a query.
   * Uses esearch to get PMIDs, then esummary to get metadata.
   * @param {string} query - PubMed search query
   * @param {number} [maxResults=20] - Maximum papers to return
   * @param {string} [minDate=null] - Minimum date in YYYY/MM/DD format
   * @returns {Promise<Array>} Array of paper objects
   */
  async function searchPubMed(query, maxResultsOrOpts = 20, minDate = null) {
    // Accept positional args or options object: searchPubMed(q, {maxResults, minDate, maxDate, daysBack})
    let maxResults = 20, maxDate = null;
    if (typeof maxResultsOrOpts === 'object' && maxResultsOrOpts !== null) {
      const opts = maxResultsOrOpts;
      maxResults = opts.maxResults || 20;
      minDate = opts.minDate || null;
      maxDate = opts.maxDate || null;
      if (!minDate && opts.daysBack) {
        const d = new Date(Date.now() - opts.daysBack * 24 * 60 * 60 * 1000);
        minDate = `${d.getFullYear()}/${String(d.getMonth()+1).padStart(2,'0')}/${String(d.getDate()).padStart(2,'0')}`;
      }
    } else {
      maxResults = maxResultsOrOpts || 20;
    }
    try {
      // Step 1: esearch to get PMIDs
      const pmids = await enqueueApiCall(async () => {
        let url = `${PUBMED_BASE}/esearch.fcgi?db=pubmed&retmode=json&retmax=${maxResults}&term=${encodeURIComponent(query)}`;
        if (_ncbiApiKey) url += `&api_key=${encodeURIComponent(_ncbiApiKey)}`;
        if (minDate) url += `&mindate=${encodeURIComponent(minDate)}&datetype=edat`;
        if (maxDate) url += `&maxdate=${encodeURIComponent(maxDate)}&datetype=edat`;
        url += "&sort=date"; // newest first

        const resp = await _rateFetch(url);
        const data = await resp.json();
        return (data.esearchresult && data.esearchresult.idlist) || [];
      });

      if (pmids.length === 0) return [];

      // Step 2: esummary to get paper details
      const papers = await enqueueApiCall(async () => {
        let url = `${PUBMED_BASE}/esummary.fcgi?db=pubmed&retmode=json&id=${pmids.join(",")}`;
        if (_ncbiApiKey) url += `&api_key=${encodeURIComponent(_ncbiApiKey)}`;

        const resp = await _rateFetch(url);
        const data = await resp.json();
        const result = data.result || {};
        const entries = [];

        for (const pmid of pmids) {
          const rec = result[pmid];
          if (!rec) continue;

          // Extract DOI from articleids array
          let doi = null;
          if (rec.articleids) {
            const doiEntry = rec.articleids.find(a => a.idtype === "doi");
            if (doiEntry) doi = doiEntry.value;
          }

          // Build authors string
          const authors = (rec.authors || []).map(a => a.name).join(", ");

          entries.push({
            pmid: pmid,
            title: rec.title || "",
            authors: authors,
            journal: rec.fulljournalname || rec.source || "",
            date: rec.pubdate || rec.sortpubdate || "",
            abstract: "", // esummary does not include abstracts
            doi: doi,
            source: "pubmed"
          });
        }

        return entries;
      });

      // Step 3: Optionally fetch abstracts via efetch (batch)
      if (papers.length > 0) {
        try {
          const abstracts = await _fetchPubMedAbstracts(pmids);
          for (const paper of papers) {
            if (abstracts[paper.pmid]) {
              paper.abstract = abstracts[paper.pmid];
            }
          }
        } catch (_) {
          // Abstracts are optional — proceed without them
        }
      }

      return papers;
    } catch (err) {
      console.error("[BioKhoj] PubMed search failed:", err);
      return [];
    }
  }

  /**
   * Fetch abstracts for a list of PMIDs via efetch XML.
   * @private
   * @param {string[]} pmids
   * @returns {Promise<Object>} Map of pmid -> abstract text
   */
  async function _fetchPubMedAbstracts(pmids) {
    return enqueueApiCall(async () => {
      let url = `${PUBMED_BASE}/efetch.fcgi?db=pubmed&retmode=xml&rettype=abstract&id=${pmids.join(",")}`;
      if (_ncbiApiKey) url += `&api_key=${encodeURIComponent(_ncbiApiKey)}`;

      const resp = await _rateFetch(url);
      const xml = await resp.text();
      const abstracts = {};

      // Parse abstracts from XML using regex (avoids DOMParser dependency issues)
      const articlePattern = /<PubmedArticle>([\s\S]*?)<\/PubmedArticle>/g;
      let match;
      while ((match = articlePattern.exec(xml)) !== null) {
        const article = match[1];
        const pmidMatch = article.match(/<PMID[^>]*>(\d+)<\/PMID>/);
        const absMatch = article.match(/<AbstractText[^>]*>([\s\S]*?)<\/AbstractText>/g);

        if (pmidMatch && absMatch) {
          // Concatenate all AbstractText elements (structured abstracts have multiple)
          const fullAbstract = absMatch
            .map(a => a.replace(/<\/?AbstractText[^>]*>/g, "").trim())
            .join(" ");
          abstracts[pmidMatch[1]] = fullAbstract;
        }
      }

      return abstracts;
    });
  }

  // ════════════════════════════════════════════════════════════════════
  // bioRxiv API Wrapper
  // ════════════════════════════════════════════════════════════════════

  /**
   * Search bioRxiv/medRxiv for recent preprints.
   * The bioRxiv API uses date ranges rather than keyword search,
   * so we fetch recent papers and filter client-side.
   * @param {string} query - Search terms to filter by
   * @param {number} [maxResults=20] - Maximum papers to return
   * @param {number} [days=30] - How many days back to search
   * @returns {Promise<Array>} Array of paper objects
   */
  async function searchBioRxiv(query, maxResults = 20, days = 30) {
    try {
      const papers = await enqueueApiCall(async () => {
        const now = new Date();
        const start = new Date(now.getTime() - days * 24 * 60 * 60 * 1000);
        const startStr = _formatDate(start);
        const endStr = _formatDate(now);

        // bioRxiv content API: /details/{server}/{start}/{end}/{cursor}
        const url = `${BIORXIV_BASE}/details/biorxiv/${startStr}/${endStr}/0/json`;
        const resp = await _rateFetch(url);
        const data = await resp.json();
        return (data.collection || []);
      });

      // Filter by query terms (title + abstract)
      const queryTerms = query.toLowerCase().split(/\s+/).filter(t => t.length > 2);
      const filtered = papers.filter(p => {
        const text = ((p.title || "") + " " + (p.abstract || "")).toLowerCase();
        return queryTerms.some(term => text.includes(term));
      });

      // Convert to standard format and limit results
      return filtered.slice(0, maxResults).map(p => ({
        pmid: null,
        title: p.title || "",
        authors: p.authors || "",
        journal: "bioRxiv",
        date: p.date || "",
        abstract: p.abstract || "",
        doi: p.doi || null,
        source: "biorxiv",
        biorxiv_category: p.category || "",
        biorxiv_version: p.version || "1"
      }));
    } catch (err) {
      console.error("[BioKhoj] bioRxiv search failed:", err);
      return [];
    }
  }

  /**
   * Format a Date as YYYY-MM-DD for bioRxiv API.
   * @private
   * @param {Date} d
   * @returns {string}
   */
  function _formatDate(d) {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }

  // ════════════════════════════════════════════════════════════════════
  // OpenAlex API Wrapper
  // ════════════════════════════════════════════════════════════════════

  /**
   * Search OpenAlex for works matching a query. Returns citation data.
   * @param {string} query - Search query
   * @param {number} [maxResults=20] - Maximum results
   * @returns {Promise<Array>} Array of { id, title, cited_by_count, publication_date, doi, concepts }
   */
  async function searchOpenAlex(query, maxResults = 20) {
    try {
      return await enqueueApiCall(async () => {
        const url = `${OPENALEX_BASE}/works?search=${encodeURIComponent(query)}&per_page=${maxResults}&sort=publication_date:desc&mailto=biokhoj@biolang.org`;
        const resp = await _rateFetch(url);
        const data = await resp.json();

        return (data.results || []).map(w => ({
          id: w.id || "",
          title: w.title || "",
          cited_by_count: w.cited_by_count || 0,
          publication_date: w.publication_date || "",
          doi: w.doi ? w.doi.replace("https://doi.org/", "") : null,
          concepts: (w.concepts || []).map(c => ({
            name: c.display_name,
            score: c.score || 0
          })),
          cited_by_count_recent: (w.counts_by_year && w.counts_by_year[0])
            ? w.counts_by_year[0].cited_by_count
            : 0
        }));
      });
    } catch (err) {
      console.error("[BioKhoj] OpenAlex search failed:", err);
      return [];
    }
  }

  /**
   * Get related concepts for an entity name via OpenAlex Concepts API.
   * Useful for concept expansion (e.g., "BRCA1" -> DNA repair, breast cancer).
   * @param {string} entityName - Entity to look up
   * @returns {Promise<Array>} Array of { name, type, relevance, openalex_id }
   */
  async function getRelatedConcepts(entityName) {
    try {
      return await enqueueApiCall(async () => {
        const url = `${OPENALEX_BASE}/concepts?search=${encodeURIComponent(entityName)}&per_page=10&mailto=biokhoj@biolang.org`;
        const resp = await _rateFetch(url);
        const data = await resp.json();

        const results = [];
        for (const concept of (data.results || [])) {
          results.push({
            name: concept.display_name || "",
            type: "concept",
            relevance: concept.relevance_score || 0,
            openalex_id: concept.id || ""
          });

          // Include related concepts from the concept's own related list
          if (concept.related_concepts) {
            for (const rc of concept.related_concepts.slice(0, 5)) {
              results.push({
                name: rc.display_name || "",
                type: "related_concept",
                relevance: rc.score || 0,
                openalex_id: rc.id || ""
              });
            }
          }
        }

        return results;
      });
    } catch (err) {
      console.error("[BioKhoj] OpenAlex concepts failed:", err);
      return [];
    }
  }

  /**
   * Get author information from OpenAlex for reputation scoring.
   * @param {string} authorName - Author name to search
   * @returns {Promise<Object|null>} { name, h_index, works_count, cited_by_count, openalex_id }
   */
  async function getAuthorInfo(authorName) {
    try {
      return await enqueueApiCall(async () => {
        const url = `${OPENALEX_BASE}/authors?search=${encodeURIComponent(authorName)}&per_page=1&mailto=biokhoj@biolang.org`;
        const resp = await _rateFetch(url);
        const data = await resp.json();

        if (!data.results || data.results.length === 0) return null;

        const a = data.results[0];
        return {
          name: a.display_name || "",
          h_index: (a.summary_stats && a.summary_stats.h_index) || 0,
          works_count: a.works_count || 0,
          cited_by_count: a.cited_by_count || 0,
          openalex_id: a.id || ""
        };
      });
    } catch (err) {
      console.error("[BioKhoj] OpenAlex author lookup failed:", err);
      return null;
    }
  }

  // ════════════════════════════════════════════════════════════════════
  // Signal Score Computation
  // ════════════════════════════════════════════════════════════════════

  /**
   * Compute a composite signal score (0-100) for a paper based on
   * recency, citation velocity, journal quality, entity matches,
   * co-mention novelty, and author reputation.
   *
   * @param {Object} paper - Paper object (from searchPubMed/searchBioRxiv)
   * @param {Array} watchedEntities - Array of { id, name, type, ... } being watched
   * @param {Object} [context={}] - Additional context:
   *   - authorHIndex: number (OpenAlex h-index of first/corresponding author)
   *   - citationVelocity: number (citations per month)
   *   - knownCoMentions: Set of "entityA::entityB" pairs already seen
   * @returns {Object} { total, breakdown: { recency, citation_velocity, journal_tier, co_mention_novelty, entity_match_count, author_reputation } }
   */
  function computeSignalScore(paper, watchedEntities, context) {
    context = context || {};
    const breakdown = {
      recency: 0,
      citation_velocity: 0,
      journal_tier: 0,
      co_mention_novelty: 0,
      entity_match_count: 0,
      author_reputation: 0
    };

    // ── Recency (0-25) ──
    // Full 25 for today, decays linearly over 30 days to 0
    const pubDate = _parseDate(paper.date);
    if (pubDate) {
      const daysSince = (Date.now() - pubDate.getTime()) / (1000 * 60 * 60 * 24);
      breakdown.recency = Math.max(0, Math.round(25 * (1 - daysSince / 30)));
    }

    // ── Citation Velocity (0-20) ──
    // Citations per month, capped at 20
    const velocity = context.citationVelocity || 0;
    if (velocity > 0) {
      // 10+ citations/month = full 20 points; scale linearly below
      breakdown.citation_velocity = Math.min(20, Math.round(velocity * 2));
    }

    // ── Journal Tier (0-15) ──
    const journalName = (paper.journal || "").toLowerCase().trim();
    if (journalName && JOURNAL_TIERS[journalName] !== undefined) {
      breakdown.journal_tier = TIER_SCORES[JOURNAL_TIERS[journalName]] || 0;
    }

    // ── Entity Match Count (0-10) ──
    // How many watched entities appear in title + abstract
    const paperText = ((paper.title || "") + " " + (paper.abstract || "")).toLowerCase();
    const matchedEntities = [];
    for (const entity of watchedEntities) {
      const name = String(entity.term || entity.name || entity.id || "").toLowerCase();
      if (name && paperText.includes(name)) {
        matchedEntities.push(entity);
      }
    }
    // 1 entity = 3 pts, 2 = 6, 3+ = 10
    if (matchedEntities.length >= 3) {
      breakdown.entity_match_count = 10;
    } else if (matchedEntities.length === 2) {
      breakdown.entity_match_count = 6;
    } else if (matchedEntities.length === 1) {
      breakdown.entity_match_count = 3;
    }

    // ── Co-mention Novelty (0-20) ──
    // First time two watched entities co-appear in a paper
    const knownCoMentions = context.knownCoMentions || new Set();
    let novelCoMentionCount = 0;
    for (let i = 0; i < matchedEntities.length; i++) {
      for (let j = i + 1; j < matchedEntities.length; j++) {
        const pair = _coMentionKey(matchedEntities[i], matchedEntities[j]);
        if (!knownCoMentions.has(pair)) {
          novelCoMentionCount++;
        }
      }
    }
    // Each novel co-mention = 10 pts, capped at 20
    breakdown.co_mention_novelty = Math.min(20, novelCoMentionCount * 10);

    // ── Author Reputation (0-10) ──
    // Based on h-index from OpenAlex
    const hIndex = context.authorHIndex || 0;
    if (hIndex >= 80) {
      breakdown.author_reputation = 10;
    } else if (hIndex >= 50) {
      breakdown.author_reputation = 8;
    } else if (hIndex >= 30) {
      breakdown.author_reputation = 6;
    } else if (hIndex >= 15) {
      breakdown.author_reputation = 4;
    } else if (hIndex >= 5) {
      breakdown.author_reputation = 2;
    }

    const total = breakdown.recency
      + breakdown.citation_velocity
      + breakdown.journal_tier
      + breakdown.co_mention_novelty
      + breakdown.entity_match_count
      + breakdown.author_reputation;

    return {
      total: Math.min(100, total),
      breakdown: breakdown
    };
  }

  /**
   * Create a canonical key for a co-mention pair (alphabetically sorted).
   * @private
   */
  function _coMentionKey(entityA, entityB) {
    const a = String(entityA.term || entityA.name || entityA.id || "").toLowerCase();
    const b = String(entityB.term || entityB.name || entityB.id || "").toLowerCase();
    return a < b ? `${a}::${b}` : `${b}::${a}`;
  }

  /**
   * Parse a date string into a Date object. Handles multiple formats.
   * @private
   * @param {string} dateStr
   * @returns {Date|null}
   */
  function _parseDate(dateStr) {
    if (!dateStr) return null;
    // Try ISO / standard parsing first
    const d = new Date(dateStr);
    if (!isNaN(d.getTime())) return d;
    // PubMed format: "2024 Jan 15" or "2024 Jan"
    const pubmedMatch = dateStr.match(/^(\d{4})\s+(\w+)\s*(\d{1,2})?/);
    if (pubmedMatch) {
      const monthStr = pubmedMatch[2];
      const day = pubmedMatch[3] || "1";
      const attempt = new Date(`${monthStr} ${day}, ${pubmedMatch[1]}`);
      if (!isNaN(attempt.getTime())) return attempt;
    }
    return null;
  }

  // ════════════════════════════════════════════════════════════════════
  // Entity Auto-Classification
  // ════════════════════════════════════════════════════════════════════

  /**
   * Classify a text string into a biological entity type using regex patterns.
   * Returns the most specific match found.
   * @param {string} text - Text to classify
   * @returns {Object} { type: string, id: string } or { type: "unknown", id: text }
   */
  function classifyEntity(text) {
    if (!text || typeof text !== "string") {
      return { type: "unknown", id: text || "" };
    }

    const trimmed = text.trim();

    // Check structured identifiers first (most specific)
    if (ENTITY_PATTERNS.rsid.test(trimmed)) {
      return { type: "variant", id: trimmed.toLowerCase() };
    }
    if (ENTITY_PATTERNS.hgvs.test(trimmed)) {
      return { type: "variant_hgvs", id: trimmed };
    }
    if (ENTITY_PATTERNS.clinvar.test(trimmed)) {
      return { type: "variant_clinvar", id: trimmed.toUpperCase() };
    }
    if (ENTITY_PATTERNS.pathway_go.test(trimmed)) {
      return { type: "go_term", id: trimmed.toUpperCase() };
    }
    if (ENTITY_PATTERNS.pathway_reactome.test(trimmed)) {
      return { type: "pathway", id: trimmed.toUpperCase() };
    }
    if (ENTITY_PATTERNS.pathway_kegg.test(trimmed)) {
      return { type: "pathway", id: trimmed };
    }
    if (ENTITY_PATTERNS.interpro.test(trimmed)) {
      return { type: "domain", id: trimmed.toUpperCase() };
    }
    if (ENTITY_PATTERNS.pfam.test(trimmed)) {
      return { type: "domain", id: trimmed.toUpperCase() };
    }
    if (ENTITY_PATTERNS.omim.test(trimmed)) {
      return { type: "disease", id: trimmed };
    }
    if (ENTITY_PATTERNS.doid.test(trimmed)) {
      return { type: "disease", id: trimmed.toUpperCase() };
    }
    if (ENTITY_PATTERNS.orphanet.test(trimmed)) {
      return { type: "disease", id: trimmed.toUpperCase() };
    }
    if (ENTITY_PATTERNS.cytoband.test(trimmed)) {
      return { type: "cytoband", id: trimmed };
    }

    // Species (binomial nomenclature)
    if (ENTITY_PATTERNS.species.test(trimmed)) {
      return { type: "species", id: trimmed };
    }

    // Cell types (multi-word, check before gene)
    if (ENTITY_PATTERNS.cell_type.test(trimmed)) {
      return { type: "cell_type", id: trimmed };
    }

    // Drug names (check before gene to avoid misclassifying drug names)
    if (ENTITY_PATTERNS.drug.test(trimmed) && trimmed.length > 4) {
      // Only classify as drug if it ends with a known drug suffix
      const drugSuffixes = /(?:ab|ib|mab|nib|lib|zole|pine|pril|vir|stat|olol|tide|zumab|ximab|lizumab)$/i;
      if (drugSuffixes.test(trimmed)) {
        return { type: "drug", id: trimmed.toLowerCase() };
      }
    }

    // Gene symbols (uppercase, 2-6 chars, not in exclusion list)
    const upper = trimmed.toUpperCase();
    if (ENTITY_PATTERNS.gene.test(upper) && !GENE_EXCLUDE.has(upper)) {
      return { type: "gene", id: upper };
    }

    // Fallback: free text (could be a disease name, technique, etc.)
    return { type: "unknown", id: trimmed };
  }

  // ════════════════════════════════════════════════════════════════════
  // Concept Expansion (via OpenAlex)
  // ════════════════════════════════════════════════════════════════════

  /**
   * Get concept suggestions for expanding a watchlist entity.
   * Queries OpenAlex concepts API and returns related terms.
   * @param {string} entityName - Entity to expand
   * @param {string} [entityType="unknown"] - Entity type from classifyEntity
   * @returns {Promise<Array>} Array of { name, type, relevance }
   */
  async function getConceptSuggestions(entityName, entityType = "unknown") {
    try {
      const concepts = await getRelatedConcepts(entityName);

      // Enrich with type classification
      return concepts.map(c => ({
        name: c.name,
        type: c.type === "concept" ? _inferConceptType(c.name, entityType) : c.type,
        relevance: c.relevance
      }));
    } catch (err) {
      console.error("[BioKhoj] Concept expansion failed:", err);
      return [];
    }
  }

  /**
   * Infer a more specific type for an OpenAlex concept based on context.
   * @private
   */
  function _inferConceptType(conceptName, parentType) {
    const lower = conceptName.toLowerCase();
    if (/cancer|tumor|carcinoma|leukemia|lymphoma|sarcoma/.test(lower)) return "disease";
    if (/pathway|signaling|transduction/.test(lower)) return "pathway";
    if (/gene|protein|receptor|kinase|transcription/.test(lower)) return "gene_family";
    if (/cell|neuron|lymphocyte|macrophage/.test(lower)) return "cell_type";
    if (/drug|therapy|treatment|inhibitor/.test(lower)) return "drug_class";
    if (/species|organism|bacteria|virus/.test(lower)) return "species";
    return "concept";
  }

  // ════════════════════════════════════════════════════════════════════
  // Adaptive Polling
  // ════════════════════════════════════════════════════════════════════

  /**
   * Determine the optimal polling interval for an entity based on its
   * recent publication activity.
   * @param {Object} entity - Watch entry with { id, name, stats }
   *   stats should include: { papers_last_week, papers_last_month }
   * @returns {number} Polling interval in milliseconds
   */
  function getPollingInterval(entity) {
    const stats = entity.stats || {};
    const papersPerWeek = stats.papers_last_week || 0;
    const papersPerMonth = stats.papers_last_month || 0;

    if (papersPerWeek > 5) {
      return POLLING_INTERVALS.hot;       // 6 hours
    }
    if (papersPerWeek >= 1) {
      return POLLING_INTERVALS.active;    // 24 hours
    }
    if (papersPerMonth >= 1) {
      return POLLING_INTERVALS.moderate;  // 72 hours
    }
    return POLLING_INTERVALS.rare;        // 168 hours (weekly)
  }

  // ════════════════════════════════════════════════════════════════════
  // Co-mention Detection
  // ════════════════════════════════════════════════════════════════════

  /**
   * Detect co-mentions of watched entities across a set of papers.
   * Identifies which pairs of entities co-occur in the same paper
   * and flags novel co-mentions (not previously seen).
   *
   * @param {Array} papers - Array of paper objects
   * @param {Array} watchedEntities - Array of { id, name, type, ... }
   * @param {Set} [knownPairs=new Set()] - Set of "entityA::entityB" keys already known
   * @returns {Array} Array of { entityA, entityB, paperIds, isNovel }
   */
  function detectCoMentions(papers, watchedEntities, knownPairs = new Set()) {
    // Build index: for each paper, which entities are mentioned?
    const paperEntities = new Map(); // paperId -> Set of entity names

    for (const paper of papers) {
      const paperId = paper.doi || paper.pmid || paper.title;
      if (!paperId) continue;

      const text = ((paper.title || "") + " " + (paper.abstract || "")).toLowerCase();
      const matched = new Set();

      for (const entity of watchedEntities) {
        const name = String(entity.term || entity.name || entity.id || "").toLowerCase();
        if (name && text.includes(name)) {
          matched.add(entity.term || entity.name || entity.id);
        }
      }

      if (matched.size >= 2) {
        paperEntities.set(paperId, matched);
      }
    }

    // Build co-mention pairs across all papers
    const pairPapers = new Map(); // "A::B" -> Set of paperIds

    for (const [paperId, entities] of paperEntities) {
      const entityList = Array.from(entities).sort();
      for (let i = 0; i < entityList.length; i++) {
        for (let j = i + 1; j < entityList.length; j++) {
          const key = `${entityList[i].toLowerCase()}::${entityList[j].toLowerCase()}`;
          if (!pairPapers.has(key)) {
            pairPapers.set(key, new Set());
          }
          pairPapers.get(key).add(paperId);
        }
      }
    }

    // Convert to result array
    const results = [];
    for (const [key, paperIdSet] of pairPapers) {
      const [a, b] = key.split("::");
      results.push({
        entityA: a,
        entityB: b,
        paperIds: Array.from(paperIdSet),
        isNovel: !knownPairs.has(key)
      });
    }

    // Sort by novelty first, then by paper count descending
    results.sort((a, b) => {
      if (a.isNovel !== b.isNovel) return a.isNovel ? -1 : 1;
      return b.paperIds.length - a.paperIds.length;
    });

    return results;
  }

  // ════════════════════════════════════════════════════════════════════
  // Paper Deduplication
  // ════════════════════════════════════════════════════════════════════

  /**
   * Merge and deduplicate papers from multiple sources.
   * Matches by DOI (exact) and title (fuzzy).
   * PubMed entries take priority when duplicates are found.
   *
   * @param {Array} pubmedPapers - Papers from PubMed
   * @param {Array} biorxivPapers - Papers from bioRxiv
   * @returns {Array} Deduplicated array of papers
   */
  function dedupPapers(pubmedPapers, biorxivPapers) {
    const seen = new Map(); // doi or normalized title -> paper
    const result = [];

    // PubMed papers first (higher priority)
    for (const paper of pubmedPapers) {
      const key = _dedupKey(paper);
      if (key && !seen.has(key)) {
        seen.set(key, paper);
        result.push(paper);
      } else if (!key) {
        // No DOI and no title — include anyway
        result.push(paper);
      }
    }

    // bioRxiv papers — skip if duplicate found
    for (const paper of biorxivPapers) {
      const key = _dedupKey(paper);
      if (key && seen.has(key)) {
        // Duplicate — merge any additional data into the existing entry
        const existing = seen.get(key);
        if (!existing.abstract && paper.abstract) {
          existing.abstract = paper.abstract;
        }
        if (!existing.doi && paper.doi) {
          existing.doi = paper.doi;
        }
        continue;
      }

      // Check fuzzy title match against all existing papers
      const normalized = _normalizeTitle(paper.title);
      let isDup = false;
      if (normalized) {
        for (const existing of result) {
          if (_fuzzyTitleMatch(normalized, _normalizeTitle(existing.title))) {
            isDup = true;
            // Merge fields
            if (!existing.abstract && paper.abstract) {
              existing.abstract = paper.abstract;
            }
            break;
          }
        }
      }

      if (!isDup) {
        if (key) seen.set(key, paper);
        result.push(paper);
      }
    }

    return result;
  }

  /**
   * Generate a dedup key for a paper (DOI preferred, then normalized title).
   * @private
   */
  function _dedupKey(paper) {
    if (paper.doi) {
      return "doi:" + paper.doi.toLowerCase().replace(/^https?:\/\/doi\.org\//, "");
    }
    const norm = _normalizeTitle(paper.title);
    if (norm) return "title:" + norm;
    return null;
  }

  /**
   * Normalize a title for fuzzy matching: lowercase, strip punctuation,
   * collapse whitespace.
   * @private
   */
  function _normalizeTitle(title) {
    if (!title) return "";
    return title.toLowerCase()
      .replace(/[^\w\s]/g, "")
      .replace(/\s+/g, " ")
      .trim();
  }

  /**
   * Fuzzy title match using Jaccard similarity on word sets.
   * Threshold: 0.85 (very high overlap required).
   * @private
   */
  function _fuzzyTitleMatch(normA, normB) {
    if (!normA || !normB) return false;
    if (normA === normB) return true;

    const wordsA = new Set(normA.split(" ").filter(w => w.length > 2));
    const wordsB = new Set(normB.split(" ").filter(w => w.length > 2));

    if (wordsA.size === 0 || wordsB.size === 0) return false;

    let intersection = 0;
    for (const w of wordsA) {
      if (wordsB.has(w)) intersection++;
    }

    const union = wordsA.size + wordsB.size - intersection;
    return union > 0 && (intersection / union) >= 0.85;
  }

  // ════════════════════════════════════════════════════════════════════
  // Watchlist Data Helpers
  // ════════════════════════════════════════════════════════════════════

  /**
   * Create a new watchlist entry.
   * @param {string} id - Entity identifier (e.g., gene symbol, rsID)
   * @param {string} type - Entity type from classifyEntity()
   * @param {string} [priority="medium"] - Watch priority: "critical", "high", "medium", "low"
   * @param {string[]} [tags=[]] - User-defined tags
   * @returns {Object} Watch entry
   */
  function createWatchEntry(id, type, priority = "medium", tags = []) {
    return {
      id: id,
      name: id,   // Display name, may be overridden
      type: type || classifyEntity(id).type,
      priority: priority,
      tags: Array.isArray(tags) ? tags : [],
      created: new Date().toISOString(),
      updated: new Date().toISOString(),
      lastChecked: null,
      stats: {
        papers_last_week: 0,
        papers_last_month: 0,
        total_papers: 0,
        last_paper_date: null
      },
      aliases: [],      // Alternative names/symbols for this entity
      notes: "",        // User notes
      muted: false,     // Temporarily silence notifications
      autoExpand: true  // Use concept expansion for this entity
    };
  }

  /**
   * Update fields on a watch entry, preserving the update timestamp.
   * @param {Object} entry - Existing watch entry
   * @param {Object} updates - Fields to update
   * @returns {Object} Updated entry (mutated in place and returned)
   */
  function updateWatchEntry(entry, updates) {
    if (!entry || !updates) return entry;

    for (const [key, value] of Object.entries(updates)) {
      if (key === "id" || key === "created") continue; // immutable fields
      entry[key] = value;
    }
    entry.updated = new Date().toISOString();
    return entry;
  }

  // ════════════════════════════════════════════════════════════════════
  // Paper Data Helpers
  // ════════════════════════════════════════════════════════════════════

  /**
   * Create a standardized paper entry from raw API data.
   * @param {Object} rawData - Raw paper data from any source
   * @param {string} source - "pubmed", "biorxiv", or "openalex"
   * @param {Array} [matchedEntities=[]] - Entities from the watchlist that matched
   * @returns {Object} Normalized paper entry
   */
  function createPaperEntry(rawData, source, matchedEntities = []) {
    return {
      // Core metadata
      pmid: rawData.pmid || null,
      doi: rawData.doi || null,
      title: rawData.title || "",
      authors: rawData.authors || "",
      journal: rawData.journal || "",
      date: rawData.date || rawData.publication_date || "",
      abstract: rawData.abstract || "",
      source: source,

      // Scoring
      signalScore: null,     // Computed later via computeSignalScore
      scoreBreakdown: null,  // Detailed breakdown

      // Entity matching
      matchedEntities: matchedEntities.map(e => ({
        id: e.id || e.name,
        name: e.name || e.id,
        type: e.type
      })),

      // Tracking
      discoveredAt: new Date().toISOString(),
      read: false,
      starred: false,
      notes: "",
      tags: [],

      // bioRxiv-specific
      biorxiv_category: rawData.biorxiv_category || null,
      biorxiv_version: rawData.biorxiv_version || null,

      // OpenAlex enrichment
      cited_by_count: rawData.cited_by_count || 0,
      concepts: rawData.concepts || []
    };
  }

  // ════════════════════════════════════════════════════════════════════
  // Export Helpers
  // ════════════════════════════════════════════════════════════════════

  /**
   * Convert an array of papers to BibTeX format.
   * @param {Array} papers
   * @returns {string} BibTeX string
   */
  function toBibTeX(papers) {
    if (!papers || papers.length === 0) return "";

    return papers.map((p, i) => {
      const key = p.pmid
        ? `pmid${p.pmid}`
        : p.doi
          ? p.doi.replace(/[^a-zA-Z0-9]/g, "_")
          : `paper${i + 1}`;

      const authorBib = _formatAuthorsBibTeX(p.authors);
      const year = _extractYear(p.date);

      const fields = [];
      if (authorBib) fields.push(`  author = {${authorBib}}`);
      fields.push(`  title = {${_escapeBibTeX(p.title)}}`);
      if (p.journal) fields.push(`  journal = {${_escapeBibTeX(p.journal)}}`);
      if (year) fields.push(`  year = {${year}}`);
      if (p.doi) fields.push(`  doi = {${p.doi}}`);
      if (p.pmid) fields.push(`  pmid = {${p.pmid}}`);

      return `@article{${key},\n${fields.join(",\n")}\n}`;
    }).join("\n\n");
  }

  /**
   * Convert an array of papers to RIS format.
   * @param {Array} papers
   * @returns {string} RIS string
   */
  function toRIS(papers) {
    if (!papers || papers.length === 0) return "";

    return papers.map(p => {
      const lines = ["TY  - JOUR"];
      if (p.title) lines.push(`TI  - ${p.title}`);

      // Split authors and add each as AU
      const authorList = _splitAuthors(p.authors);
      for (const a of authorList) {
        lines.push(`AU  - ${a}`);
      }

      if (p.journal) lines.push(`JO  - ${p.journal}`);

      const year = _extractYear(p.date);
      if (year) lines.push(`PY  - ${year}`);
      if (p.date) lines.push(`DA  - ${p.date}`);
      if (p.doi) lines.push(`DO  - ${p.doi}`);
      if (p.pmid) lines.push(`AN  - PMID:${p.pmid}`);
      if (p.abstract) lines.push(`AB  - ${p.abstract}`);

      lines.push("ER  - ");
      return lines.join("\n");
    }).join("\n\n");
  }

  /**
   * Convert an array of papers to Markdown format.
   * @param {Array} papers
   * @returns {string} Markdown string
   */
  function toMarkdown(papers) {
    if (!papers || papers.length === 0) return "";

    const lines = ["# Papers", ""];

    for (const p of papers) {
      const score = p.signalScore !== null ? ` [Signal: ${p.signalScore}]` : "";
      const doi = p.doi ? ` ([DOI](https://doi.org/${p.doi}))` : "";
      const pmid = p.pmid ? ` ([PubMed](https://pubmed.ncbi.nlm.nih.gov/${p.pmid}/))` : "";

      lines.push(`## ${p.title}${score}`);
      lines.push("");
      if (p.authors) lines.push(`**Authors:** ${p.authors}`);
      if (p.journal) lines.push(`**Journal:** ${p.journal}`);
      if (p.date) lines.push(`**Date:** ${p.date}`);
      lines.push(`**Links:**${doi}${pmid}`);

      if (p.matchedEntities && p.matchedEntities.length > 0) {
        const entities = p.matchedEntities.map(e => `\`${e.name}\``).join(", ");
        lines.push(`**Matched entities:** ${entities}`);
      }

      if (p.abstract) {
        lines.push("");
        lines.push(`> ${p.abstract.slice(0, 300)}${p.abstract.length > 300 ? "..." : ""}`);
      }

      lines.push("");
      lines.push("---");
      lines.push("");
    }

    return lines.join("\n");
  }

  /**
   * Export a watchlist to JSON.
   * @param {Array} watchlist - Array of watch entries
   * @returns {string} Pretty-printed JSON
   */
  function watchlistToJSON(watchlist) {
    if (!watchlist) return "[]";
    return JSON.stringify({
      version: 1,
      exported: new Date().toISOString(),
      source: "BioKhoj",
      entries: watchlist
    }, null, 2);
  }

  // ── BibTeX / RIS helpers ──

  /**
   * Format author string for BibTeX (comma-separated -> "Last, First and Last, First").
   * @private
   */
  function _formatAuthorsBibTeX(authorStr) {
    if (!authorStr) return "";
    const authors = _splitAuthors(authorStr);
    return authors.join(" and ");
  }

  /**
   * Split an author string into individual author names.
   * Handles "Smith J, Doe A" and "Smith, John; Doe, Alice" formats.
   * @private
   */
  function _splitAuthors(authorStr) {
    if (!authorStr) return [];
    // Try semicolon split first (common in structured data)
    if (authorStr.includes(";")) {
      return authorStr.split(";").map(a => a.trim()).filter(Boolean);
    }
    // PubMed format: "Smith J, Doe A, ..." — split on ", " but be careful
    // with "Last, First" format. Heuristic: if most segments are short (<=20 chars),
    // treat commas as author separators.
    const parts = authorStr.split(",").map(s => s.trim()).filter(Boolean);
    if (parts.length > 1 && parts.every(p => p.length < 30)) {
      return parts;
    }
    return [authorStr];
  }

  /**
   * Escape special BibTeX characters.
   * @private
   */
  function _escapeBibTeX(str) {
    if (!str) return "";
    return str
      .replace(/&/g, "\\&")
      .replace(/%/g, "\\%")
      .replace(/#/g, "\\#")
      .replace(/_/g, "\\_")
      .replace(/\{/g, "\\{")
      .replace(/\}/g, "\\}");
  }

  /**
   * Extract a 4-digit year from a date string.
   * @private
   */
  function _extractYear(dateStr) {
    if (!dateStr) return null;
    const match = dateStr.match(/\b(19|20)\d{2}\b/);
    return match ? match[0] : null;
  }

  // ════════════════════════════════════════════════════════════════════
  // Weekly Digest Generator
  // ════════════════════════════════════════════════════════════════════

  /**
   * Generate a formatted weekly digest in Markdown.
   * Summarizes new papers, top signals, trends, and co-mentions.
   *
   * @param {Array} papers - Papers discovered this period
   * @param {Array} watchlist - Current watchlist entries
   * @param {Object} [trends={}] - Trend data:
   *   - coMentions: Array from detectCoMentions()
   *   - entityCounts: Map of entity -> paper count
   *   - topJournals: Array of { journal, count }
   * @returns {string} Formatted Markdown digest
   */
  function generateDigest(papers, watchlist, trends = {}) {
    const now = new Date();
    const dateStr = now.toISOString().split("T")[0];
    const lines = [];

    lines.push(`# BioKhoj Weekly Digest - ${dateStr}`);
    lines.push("");

    // ── Summary stats ──
    const scoredPapers = papers.filter(p => p.signalScore !== null);
    const critical = scoredPapers.filter(p => p.signalScore >= SIGNAL_THRESHOLDS.critical);
    const high = scoredPapers.filter(p => p.signalScore >= SIGNAL_THRESHOLDS.high && p.signalScore < SIGNAL_THRESHOLDS.critical);
    const medium = scoredPapers.filter(p => p.signalScore >= SIGNAL_THRESHOLDS.medium && p.signalScore < SIGNAL_THRESHOLDS.high);

    lines.push("## Summary");
    lines.push("");
    lines.push(`- **Total new papers:** ${papers.length}`);
    lines.push(`- **Critical signal (80+):** ${critical.length}`);
    lines.push(`- **High signal (60-79):** ${high.length}`);
    lines.push(`- **Medium signal (40-59):** ${medium.length}`);
    lines.push(`- **Watched entities:** ${watchlist.length}`);
    lines.push("");

    // ── Critical papers (must-read) ──
    if (critical.length > 0) {
      lines.push("## Must-Read Papers (Signal 80+)");
      lines.push("");
      for (const p of critical.slice(0, 10)) {
        const doi = p.doi ? ` [DOI](https://doi.org/${p.doi})` : "";
        const score = p.signalScore || 0;
        lines.push(`### ${p.title}`);
        lines.push(`- **Score:** ${score} | **Journal:** ${p.journal || "N/A"} | **Date:** ${p.date || "N/A"}`);
        if (p.matchedEntities && p.matchedEntities.length > 0) {
          lines.push(`- **Matches:** ${p.matchedEntities.map(e => e.name).join(", ")}`);
        }
        if (doi) lines.push(`- ${doi}`);
        lines.push("");
      }
    }

    // ── High signal papers ──
    if (high.length > 0) {
      lines.push("## High Signal Papers (60-79)");
      lines.push("");
      for (const p of high.slice(0, 15)) {
        const entities = (p.matchedEntities || []).map(e => e.name).join(", ");
        lines.push(`- **${p.title}** (Score: ${p.signalScore}) — ${p.journal || ""} ${entities ? "| " + entities : ""}`);
      }
      lines.push("");
    }

    // ── Novel co-mentions ──
    const coMentions = trends.coMentions || [];
    const novelCoMentions = coMentions.filter(cm => cm.isNovel);
    if (novelCoMentions.length > 0) {
      lines.push("## Novel Co-mentions");
      lines.push("");
      lines.push("First-time co-occurrence of watched entities in a paper:");
      lines.push("");
      for (const cm of novelCoMentions.slice(0, 10)) {
        lines.push(`- **${cm.entityA}** + **${cm.entityB}** (${cm.paperIds.length} paper${cm.paperIds.length > 1 ? "s" : ""})`);
      }
      lines.push("");
    }

    // ── Entity activity ──
    const entityCounts = trends.entityCounts || new Map();
    if (entityCounts.size > 0) {
      lines.push("## Entity Activity");
      lines.push("");
      const sorted = Array.from(entityCounts.entries()).sort((a, b) => b[1] - a[1]);
      for (const [entity, count] of sorted.slice(0, 20)) {
        const bar = "\u2588".repeat(Math.min(count, 20));
        lines.push(`- \`${entity}\`: ${count} papers ${bar}`);
      }
      lines.push("");
    }

    // ── Top journals ──
    const topJournals = trends.topJournals || [];
    if (topJournals.length > 0) {
      lines.push("## Top Journals This Week");
      lines.push("");
      for (const j of topJournals.slice(0, 10)) {
        lines.push(`- **${j.journal}**: ${j.count} papers`);
      }
      lines.push("");
    }

    // ── Watchlist health ──
    const inactive = watchlist.filter(e => {
      const stats = e.stats || {};
      return (stats.papers_last_month || 0) === 0;
    });
    if (inactive.length > 0) {
      lines.push("## Watchlist Health");
      lines.push("");
      lines.push(`${inactive.length} entities had zero papers this month. Consider reviewing:`);
      lines.push("");
      for (const e of inactive.slice(0, 10)) {
        lines.push(`- \`${e.name}\` (${e.type}) — last checked: ${e.lastChecked || "never"}`);
      }
      lines.push("");
    }

    lines.push("---");
    lines.push(`*Generated by BioKhoj on ${now.toISOString()}*`);

    return lines.join("\n");
  }

  // ════════════════════════════════════════════════════════════════════
  // Public API — exposed on window.BioKhojCore
  // ════════════════════════════════════════════════════════════════════

  window.BioKhojCore = {
    // Constants
    PUBMED_BASE,
    BIORXIV_BASE,
    OPENALEX_BASE,
    SIGNAL_THRESHOLDS,
    POLLING_INTERVALS,
    JOURNAL_TIERS,
    TIER_SCORES,

    // Configuration
    setNcbiApiKey,

    // Rate limiting & budget
    enqueueApiCall,
    getApiBudget,

    // API wrappers
    searchPubMed,
    searchBioRxiv,
    searchOpenAlex,
    getRelatedConcepts,
    getAuthorInfo,

    // Signal scoring
    computeSignalScore,

    // Entity classification and expansion
    classifyEntity,
    getConceptSuggestions,

    // Adaptive polling
    getPollingInterval,

    // Co-mention detection
    detectCoMentions,

    // Deduplication
    dedupPapers,

    // Watchlist helpers
    createWatchEntry,
    updateWatchEntry,

    // Paper helpers
    createPaperEntry,

    // Export
    toBibTeX,
    toRIS,
    toMarkdown,
    watchlistToJSON,

    // Digest
    generateDigest
  };

})();
