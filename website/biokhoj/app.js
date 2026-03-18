// ============================================================================
// BioKhoj — app.js
// Main UI logic for the BioKhoj literature monitoring PWA
// Depends on: Chart.js, window.BioKhojCore, window.BioKhojStorage
// ============================================================================

'use strict';

// ---------------------------------------------------------------------------
// 1. Initialize Storage (delegates to BioKhojStorage)
// ---------------------------------------------------------------------------

const store = window.BioKhojStorage;
store.init(Dexie);

// ---------------------------------------------------------------------------
// 2. State
// ---------------------------------------------------------------------------

let currentTab = 'feed';
let charts = {};
let deferredInstallPrompt = null;
let citationCache = {};
let journalClubIndex = 0;
let watchedAuthors = [];
let selectedPaperIndex = -1;
let lastFailedOp = null;

// ---------------------------------------------------------------------------
// 3. Initialization
// ---------------------------------------------------------------------------

document.addEventListener('DOMContentLoaded', async () => {
  registerServiceWorker();
  listenForInstallPrompt();
  await loadSettings();
  bindTabNavigation();
  bindAddEntity();
  bindSettingsControls();
  bindExportImport();
  bindGlobalSearch();
  bindFeedSort();
  bindReadingSort();
  bindExportDropdown();
  bindFeatureButtons();
  bindDiscoverTab();
  applyTheme();

  const watchlistCount = await store.getWatchlistCount();
  const onboarded = await store.getSetting('onboarded');
  if (watchlistCount === 0 && !onboarded) {
    showOnboardingCard();
  } else if (watchlistCount === 0) {
    await loadDemoData();
  }

  bindKeyboardNavigation();
  await checkImportFromUrl();
  await renderSidebar();
  await renderCurrentTab();
  await runCatchUp();
  scheduleBackgroundChecks();
  checkCitationAlerts();
});

// ---------------------------------------------------------------------------
// 4. Service Worker
// ---------------------------------------------------------------------------

function registerServiceWorker() {
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/biokhoj/sw.js')
      .then((reg) => {
        if ('periodicSync' in reg) {
          reg.periodicSync.register('check-papers-periodic', {
            minInterval: 60 * 60 * 1000
          }).catch(() => {});
        }
      })
      .catch((err) => console.warn('SW registration failed:', err));

    navigator.serviceWorker.addEventListener('message', (event) => {
      if (event.data && event.data.type === 'background-check-papers') {
        triggerManualCheck();
      }
    });
  }
}

// ---------------------------------------------------------------------------
// 5. PWA Install Prompt
// ---------------------------------------------------------------------------

function listenForInstallPrompt() {
  window.addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    deferredInstallPrompt = e;
    const dismissed = localStorage.getItem('biokhoj-install-dismissed');
    if (!dismissed) {
      document.getElementById('pwa-install-banner').classList.remove('hidden');
    }
  });
}

window.installPWA = async function () {
  if (!deferredInstallPrompt) return;
  deferredInstallPrompt.prompt();
  const result = await deferredInstallPrompt.userChoice;
  deferredInstallPrompt = null;
  document.getElementById('pwa-install-banner').classList.add('hidden');
};

window.dismissInstallBanner = function () {
  document.getElementById('pwa-install-banner').classList.add('hidden');
  localStorage.setItem('biokhoj-install-dismissed', '1');
};

// ---------------------------------------------------------------------------
// 6. Tab Navigation
// ---------------------------------------------------------------------------

function bindTabNavigation() {
  document.querySelectorAll('.tab-btn, .mobile-tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => switchTab(btn.dataset.tab));
  });
}

window.switchTab = function (tab) {
  currentTab = tab;

  document.querySelectorAll('.tab-panel').forEach((p) => p.classList.add('hidden'));
  const panel = document.getElementById(`tab-${tab}`);
  if (panel) {
    panel.classList.remove('hidden');
    panel.classList.remove('fade-in');
    void panel.offsetWidth;
    panel.classList.add('fade-in');
  }

  document.querySelectorAll('.tab-btn').forEach((b) => {
    b.classList.remove('tab-active', 'text-saffron-400');
    b.classList.add('text-slate-400');
  });
  const activeDesktop = document.querySelector(`.tab-btn[data-tab="${tab}"]`);
  if (activeDesktop) {
    activeDesktop.classList.add('tab-active', 'text-saffron-400');
    activeDesktop.classList.remove('text-slate-400');
  }

  document.querySelectorAll('.mobile-tab-btn').forEach((b) => {
    b.classList.remove('text-saffron-400');
    b.classList.add('text-slate-500');
  });
  const activeMobile = document.querySelector(`.mobile-tab-btn[data-tab="${tab}"]`);
  if (activeMobile) {
    activeMobile.classList.add('text-saffron-400');
    activeMobile.classList.remove('text-slate-500');
  }

  renderCurrentTab();
};

async function renderCurrentTab() {
  switch (currentTab) {
    case 'feed': await renderFeed(); break;
    case 'discover': await renderDiscover(); break;
    case 'watchlist': await renderWatchlist(); break;
    case 'trends': await renderTrends(); break;
    case 'reading-list': await renderReadingList(); break;
    case 'settings': await renderSettings(); break;
  }
}

// ---------------------------------------------------------------------------
// 7. Demo Data
// ---------------------------------------------------------------------------

async function loadDemoData() {
  const demoEntities = [
    { term: 'TP53', type: 'gene', priority: 'high', tags: ['cancer', 'tumor suppressor'], paused: false, addedAt: Date.now(), lastChecked: 0 },
    { term: 'CRISPR-Cas9', type: 'technique', priority: 'high', tags: ['gene editing'], paused: false, addedAt: Date.now(), lastChecked: 0 },
    { term: 'single-cell RNA-seq', type: 'technique', priority: 'normal', tags: ['transcriptomics'], paused: false, addedAt: Date.now(), lastChecked: 0 },
    { term: 'BRCA1', type: 'gene', priority: 'normal', tags: ['cancer', 'breast cancer'], paused: false, addedAt: Date.now(), lastChecked: 0 },
    { term: 'Alzheimer disease', type: 'disease', priority: 'low', tags: ['neurodegeneration'], paused: false, addedAt: Date.now(), lastChecked: 0 },
  ];
  await store.bulkAddWatchlist(demoEntities);

  const demoPapers = [
    {
      pmid: 'demo-1', title: 'CRISPR-Cas9 mediated TP53 knockout reveals synthetic lethality in pancreatic cancer cells',
      authors: ['Zhang Y', 'Chen L', 'Wang M', 'Kim S', 'Rodriguez A'],
      journal: 'Nature Genetics', date: new Date().toISOString().slice(0, 10),
      abstract: 'We performed genome-wide CRISPR screens in TP53-mutant pancreatic ductal adenocarcinoma cell lines and identified novel synthetic lethal interactions...',
      matchedEntities: ['TP53', 'CRISPR-Cas9'], signalScore: 92,
      signalBreakdown: { entityRelevance: 40, journalTier: 25, recency: 15, novelty: 12 },
      saved: false, starred: false, read: false, savedAt: null, notes: '', paperTags: []
    },
    {
      pmid: 'demo-2', title: 'Single-cell transcriptomic atlas of Alzheimer disease progression in human prefrontal cortex',
      authors: ['Park J', 'Williams R', 'Nakamura T', 'Singh P'],
      journal: 'Cell', date: new Date(Date.now() - 86400000).toISOString().slice(0, 10),
      abstract: 'Using single-cell RNA sequencing of over 500,000 nuclei from post-mortem prefrontal cortex samples spanning Braak stages I-VI, we construct a comprehensive cellular atlas...',
      matchedEntities: ['single-cell RNA-seq', 'Alzheimer disease'], signalScore: 88,
      signalBreakdown: { entityRelevance: 35, journalTier: 25, recency: 15, novelty: 13 },
      saved: false, starred: true, read: false, savedAt: null, notes: '', paperTags: []
    },
    {
      pmid: 'demo-3', title: 'BRCA1 promoter methylation as a predictive biomarker for PARP inhibitor response in triple-negative breast cancer',
      authors: ['Lee H', 'Gupta R', 'Fernandez C'],
      journal: 'Journal of Clinical Oncology', date: new Date(Date.now() - 172800000).toISOString().slice(0, 10),
      abstract: 'In a prospective cohort of 342 patients with triple-negative breast cancer, we evaluated BRCA1 promoter methylation status as a predictor of response to olaparib...',
      matchedEntities: ['BRCA1'], signalScore: 74,
      signalBreakdown: { entityRelevance: 30, journalTier: 20, recency: 14, novelty: 10 },
      saved: false, starred: false, read: false, savedAt: null, notes: '', paperTags: []
    },
    {
      pmid: 'demo-4', title: 'Base editing of TP53 hotspot mutations restores wild-type function in human organoids',
      authors: ['Thompson K', 'Liu X', 'Patel N', 'Johansson S', 'Brown D'],
      journal: 'Science', date: new Date(Date.now() - 259200000).toISOString().slice(0, 10),
      abstract: 'Using adenine base editors delivered via lipid nanoparticles, we demonstrate precise correction of six recurrent TP53 missense mutations in patient-derived colorectal organoids...',
      matchedEntities: ['TP53'], signalScore: 85,
      signalBreakdown: { entityRelevance: 35, journalTier: 25, recency: 13, novelty: 12 },
      saved: true, starred: false, read: true, savedAt: Date.now(), notes: 'Interesting approach for thesis', paperTags: ['base-editing']
    },
  ];
  await store.bulkAddPapers(demoPapers);
}

// ---------------------------------------------------------------------------
// 8. Sidebar Rendering
// ---------------------------------------------------------------------------

async function renderSidebar() {
  const container = document.getElementById('sidebar-watchlist');
  if (!container) return;

  const entities = await store.getActiveWatchlist();
  if (entities.length === 0) {
    container.innerHTML = '<p class="text-xs text-slate-500 py-2">No entities tracked yet</p>';
    return;
  }

  let html = '';
  for (const entity of entities.slice(0, 12)) {
    const newCount = await store.getUnreadPapersForEntity(entity.term);
    // Activity dot: purple = high-signal unread, saffron = new papers, grey = quiet
    const allPapersForEntity = await store.getPapersForEntity(entity.term);
    const hasHighSignal = allPapersForEntity.some(p => !p.read && (p.signalScore || 0) >= 70);
    const dotColor = hasHighSignal ? 'bg-purple-400' : newCount > 0 ? 'bg-saffron-400' : 'bg-slate-600';
    const dotTitle = hasHighSignal ? 'High-signal papers' : newCount > 0 ? 'New papers' : 'Up to date';

    html += `
      <div class="flex items-center justify-between py-1.5 px-2 rounded-lg hover:bg-surface-800 transition cursor-pointer group" onclick="switchTab('feed')">
        <div class="flex items-center gap-2 min-w-0">
          <span class="w-2 h-2 rounded-full ${dotColor} flex-shrink-0${hasHighSignal ? ' animate-pulse' : ''}" title="${dotTitle}"></span>
          <span class="text-sm text-slate-300 truncate">${escapeHtml(entity.term)}</span>
        </div>
        <div class="flex items-center gap-2 flex-shrink-0">
          ${newCount > 0 ? `<span class="text-[10px] font-bold text-saffron-400 bg-saffron-400/10 px-1.5 py-0.5 rounded-full">${newCount}</span>` : '<span class="text-[10px] text-slate-600">—</span>'}
        </div>
      </div>`;
  }
  container.innerHTML = html;

  const lastCheck = await store.getSetting('lastGlobalCheck');
  if (lastCheck) {
    const ago = timeSince(lastCheck);
    document.getElementById('sidebar-last-check').textContent = `Last check: ${ago}`;
  }
}

// ---------------------------------------------------------------------------
// 9. Feed Rendering
// ---------------------------------------------------------------------------

async function renderFeed() {
  const container = document.getElementById('feed-papers');
  const emptyState = document.getElementById('feed-empty');
  const loadingState = document.getElementById('feed-loading');

  selectedPaperIndex = -1;

  const sortBy = document.getElementById('feed-sort').value;
  let papers = await store.getAllPapers();

  if (papers.length === 0) {
    container.innerHTML = '';
    loadingState.classList.add('hidden');
    // Show different message based on watchlist state
    const wlCount = await store.getWatchlistCount();
    if (wlCount > 0) {
      emptyState.innerHTML = `
        <div class="text-5xl mb-4">&#128225;</div>
        <h3 class="text-lg font-semibold text-white mb-2">No papers yet</h3>
        <p class="text-sm text-slate-400 max-w-md mx-auto mb-6">You're watching ${wlCount} entities. Click refresh to check PubMed for new papers.</p>
        <button onclick="triggerManualCheck()" class="px-5 py-2.5 bg-saffron-400 text-surface-900 rounded-lg text-sm font-semibold hover:bg-saffron-300 transition">Check PubMed now</button>`;
    } else {
      emptyState.innerHTML = `
        <div class="text-5xl mb-4">&#128225;</div>
        <h3 class="text-lg font-semibold text-white mb-2">Your feed is empty</h3>
        <p class="text-sm text-slate-400 max-w-md mx-auto mb-6">Add genes, proteins, pathways, or topics to your watchlist and BioKhoj will find matching papers from PubMed.</p>
        <button onclick="switchTab('watchlist')" class="px-5 py-2.5 bg-saffron-400 text-surface-900 rounded-lg text-sm font-semibold hover:bg-saffron-300 transition">Set up watchlist</button>`;
    }
    emptyState.classList.remove('hidden');
    return;
  }

  emptyState.classList.add('hidden');
  loadingState.classList.add('hidden');

  if (sortBy === 'signal') {
    papers.sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0));
  } else if (sortBy === 'date') {
    papers.sort((a, b) => new Date(b.date) - new Date(a.date));
  } else if (sortBy === 'journal') {
    papers.sort((a, b) => (a.journal || '').localeCompare(b.journal || ''));
  }

  const searchTerm = document.getElementById('global-search').value.toLowerCase().trim();
  if (searchTerm) {
    papers = papers.filter(p =>
      p.title.toLowerCase().includes(searchTerm) ||
      (p.authors || []).join(' ').toLowerCase().includes(searchTerm) ||
      (p.journal || '').toLowerCase().includes(searchTerm) ||
      (p.abstract || '').toLowerCase().includes(searchTerm)
    );
  }

  // Signal mute threshold from settings
  const muteThreshold = (await store.getSetting('muteThreshold')) || 0;
  if (muteThreshold > 0) {
    papers = papers.filter(p => (p.signalScore || 0) >= muteThreshold);
  }

  // Feed filters
  if (comentionFilterActive) {
    papers = papers.filter(p => (p.matchedEntities || []).length >= 2);
  }
  if (unreadFilterActive) {
    papers = papers.filter(p => !p.read);
  }
  if (highSignalFilterActive) {
    papers = papers.filter(p => (p.signalScore || 0) >= 70);
  }

  await renderCoMentionAlerts();
  await renderFeedBriefing();

  const subtitle = document.getElementById('feed-subtitle');
  const activeFilters = [];
  if (comentionFilterActive) activeFilters.push('multi-entity');
  if (unreadFilterActive) activeFilters.push('unread');
  if (highSignalFilterActive) activeFilters.push('high signal');
  if (muteThreshold > 0) activeFilters.push('score \u2265' + muteThreshold);
  const filterNote = activeFilters.length > 0 ? ' (' + activeFilters.join(', ') + ')' : '';
  subtitle.textContent = `${papers.length} paper${papers.length !== 1 ? 's' : ''} matching your watchlist${filterNote}`;

  container.innerHTML = papers.map((p) => renderPaperCard(p, false)).join('');
  bindPaperActions();

  // Bind paper card click for multi-select (shift+click or long press)
  document.querySelectorAll('#feed-papers > [data-pmid]').forEach(el => {
    el.addEventListener('click', (e) => {
      if (e.shiftKey) {
        e.preventDefault();
        togglePaperSelect(el.dataset.pmid);
      }
    });
  });
}

async function renderFeedBriefing() {
  const el = document.getElementById('feed-briefing');
  if (!el) return;

  const lastCheck = await store.getSetting('lastGlobalCheck');
  if (!lastCheck) { el.classList.add('hidden'); return; }

  const allPapers = await store.getAllPapers();
  const sinceLastCheck = allPapers.filter(p => {
    const t = p.addedAt || new Date(p.date || 0).getTime() || 0;
    return t > lastCheck;
  });

  if (sinceLastCheck.length === 0 && allPapers.length === 0) {
    el.classList.add('hidden');
    return;
  }

  const items = [];
  const newCount = sinceLastCheck.length;
  const highSignal = sinceLastCheck.filter(p => (p.signalScore || 0) >= 70);
  const unread = allPapers.filter(p => !p.read).length;

  if (newCount > 0) {
    items.push(`<strong class="text-white">${newCount}</strong> new paper${newCount !== 1 ? 's' : ''} since last check`);
  }
  if (highSignal.length > 0) {
    items.push(`<strong class="text-purple-300">${highSignal.length}</strong> high-signal`);
  }
  if (unread > 0 && unread !== newCount) {
    items.push(`${unread} unread total`);
  }

  // Recent co-mentions
  const coMentions = await store.getRecentCoMentions(1);
  if (coMentions.length > 0) {
    items.push(`co-mention: <strong class="text-white">${escapeHtml(coMentions[0].entityA)}</strong> + <strong class="text-white">${escapeHtml(coMentions[0].entityB)}</strong>`);
  }

  if (items.length === 0) {
    el.classList.add('hidden');
    return;
  }

  el.classList.remove('hidden');
  el.innerHTML = `
    <div class="bg-saffron-400/5 border border-saffron-400/20 rounded-lg px-4 py-2.5 flex items-center gap-2">
      <span class="text-saffron-400 text-sm flex-shrink-0">&#9889;</span>
      <span class="text-xs text-slate-400">${items.join(' &middot; ')}</span>
    </div>`;
}

function renderPaperCard(paper, isReadingList) {
  const score = paper.signalScore || 0;
  let signalClass, signalLabel;
  if (score >= 80) { signalClass = 'signal-high'; signalLabel = 'High'; }
  else if (score >= 50) { signalClass = 'signal-normal'; signalLabel = 'Normal'; }
  else { signalClass = 'signal-low'; signalLabel = 'Low'; }

  const authors = (paper.authors || []).slice(0, 3).join(', ') + ((paper.authors || []).length > 3 ? ' et al.' : '');
  const abstractPreview = (paper.abstract || '').slice(0, 180) + ((paper.abstract || '').length > 180 ? '...' : '');
  const entityChips = (paper.matchedEntities || []).map(e => {
    const colors = entityChipColor(e);
    return `<span class="chip ${colors}">${escapeHtml(e)}</span>`;
  }).join('');

  const readOpacity = paper.read ? 'opacity-60' : '';
  const starFill = paper.starred ? 'text-saffron-400' : 'text-slate-600';
  const saveFill = paper.saved ? 'text-saffron-400' : 'text-slate-600';

  const citationBadge = isReadingList ? getCitationBadgeHtml(paper.pmid) : '';
  const backlogBadge = (paper.backlog && !paper.backlogDismissed)
    ? '<span class="inline-flex items-center text-[10px] text-blue-400 bg-blue-400/10 px-1.5 py-0.5 rounded-full font-medium ml-2">backlog</span>'
    : '';

  let notesSection = '';
  if (isReadingList) {
    notesSection = `
      <div class="mt-2 pt-2 border-t border-slate-700/50">
        <textarea data-pmid="${paper.pmid}" class="paper-notes w-full bg-transparent text-xs text-slate-400 placeholder-slate-600 resize-none focus:outline-none" rows="2" placeholder="Add notes...">${escapeHtml(paper.notes || '')}</textarea>
        ${(paper.paperTags || []).length > 0 ? `<div class="flex flex-wrap gap-1 mt-1">${paper.paperTags.map(t => `<span class="chip bg-slate-700/50 text-slate-400">${escapeHtml(t)}</span>`).join('')}</div>` : ''}
      </div>`;
  }

  const fullAbstract = paper.abstract || '';
  const hasFullAbstract = fullAbstract.length > 180;

  return `
    <div class="bg-surface-800 border border-slate-700/50 rounded-xl p-4 hover:border-slate-600 transition ${readOpacity} fade-in" data-pmid="${paper.pmid}">
      <div class="flex items-start gap-3">
        <button class="signal-badge flex-shrink-0 ${signalClass} text-white text-xs font-bold px-2 py-1 rounded-lg cursor-pointer hover:opacity-80 transition" data-pmid="${paper.pmid}" title="Click for breakdown">
          ${score >= 80 ? '&#9733; ' : score < 50 ? '&#9888; ' : ''}${score}
        </button>
        <div class="flex-1 min-w-0">
          <h3 class="text-sm font-semibold text-white leading-snug mb-1">${escapeHtml(paper.title)}</h3>
          <div class="flex flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-slate-400 mb-2">
            <span>${escapeHtml(authors)}</span>
            <span class="text-slate-600">|</span>
            <span class="font-medium text-slate-300">${escapeHtml(paper.journal || 'Preprint')}</span>
            <span class="text-slate-600">|</span>
            <span>${paper.date || 'Unknown'}</span>
            ${citationBadge}
            ${backlogBadge}
          </div>
          <p class="text-xs text-slate-500 leading-relaxed mb-2 abstract-preview" data-pmid="${paper.pmid}">${escapeHtml(abstractPreview)}${hasFullAbstract ? ' <span class="text-saffron-400/70 cursor-pointer hover:text-saffron-400 abstract-expand-btn">more</span>' : ''}</p>
          <p class="text-xs text-slate-400 leading-relaxed mb-2 hidden abstract-full" data-pmid="${paper.pmid}">${escapeHtml(fullAbstract)} <span class="text-saffron-400/70 cursor-pointer hover:text-saffron-400 abstract-collapse-btn">less</span></p>
          <div class="flex items-center justify-between">
            <div class="flex flex-wrap gap-1.5">${entityChips}</div>
            <div class="flex items-center gap-1 flex-shrink-0 ml-2">
              <button class="paper-action p-1.5 rounded hover:bg-surface-700 transition" data-action="read" data-pmid="${paper.pmid}" title="${paper.read ? 'Mark unread' : 'Mark read'}">
                <svg class="w-4 h-4 ${paper.read ? 'text-green-400' : 'text-slate-600'}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
              </button>
              <button class="paper-action p-1.5 rounded hover:bg-surface-700 transition" data-action="star" data-pmid="${paper.pmid}" title="Star">
                <svg class="w-4 h-4 ${starFill}" fill="${paper.starred ? 'currentColor' : 'none'}" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"/></svg>
              </button>
              <button class="paper-action p-1.5 rounded hover:bg-surface-700 transition" data-action="save" data-pmid="${paper.pmid}" title="${paper.saved ? 'Remove from reading list' : 'Save to reading list'}">
                <svg class="w-4 h-4 ${saveFill}" fill="${paper.saved ? 'currentColor' : 'none'}" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z"/></svg>
              </button>
              <button class="paper-action p-1.5 rounded hover:bg-surface-700 transition" data-action="cite" data-pmid="${paper.pmid}" title="Copy citation">
                <svg class="w-4 h-4 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"/></svg>
              </button>
            </div>
          </div>
          ${notesSection}
        </div>
      </div>
    </div>`;
}

function bindPaperActions() {
  document.querySelectorAll('.paper-action').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const pmid = btn.dataset.pmid;
      const action = btn.dataset.action;
      await handlePaperAction(pmid, action);
    });
  });

  document.querySelectorAll('.signal-badge').forEach((btn) => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const pmid = btn.dataset.pmid;
      await showSignalBreakdown(pmid, btn);
    });
  });

  // Abstract expand/collapse
  document.querySelectorAll('.abstract-expand-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const pmid = btn.closest('[data-pmid]').dataset.pmid;
      const card = btn.closest('.bg-surface-800');
      const preview = card.querySelector('.abstract-preview[data-pmid="' + pmid + '"]');
      const full = card.querySelector('.abstract-full[data-pmid="' + pmid + '"]');
      if (preview) preview.classList.add('hidden');
      if (full) full.classList.remove('hidden');
    });
  });
  document.querySelectorAll('.abstract-collapse-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const pmid = btn.closest('[data-pmid]').dataset.pmid;
      const card = btn.closest('.bg-surface-800');
      const preview = card.querySelector('.abstract-preview[data-pmid="' + pmid + '"]');
      const full = card.querySelector('.abstract-full[data-pmid="' + pmid + '"]');
      if (full) full.classList.add('hidden');
      if (preview) preview.classList.remove('hidden');
    });
  });

  document.querySelectorAll('.paper-notes').forEach((textarea) => {
    textarea.addEventListener('change', async () => {
      const pmid = textarea.dataset.pmid;
      const paper = await store.getPaperByPmid(pmid);
      if (paper) {
        await store.updatePaper(paper.id, { notes: textarea.value });
      }
    });
  });
}

async function handlePaperAction(pmid, action) {
  const paper = await store.getPaperByPmid(pmid);
  if (!paper) return;

  switch (action) {
    case 'read':
      await store.updatePaper(paper.id, { read: !paper.read });
      break;
    case 'star':
      await store.updatePaper(paper.id, { starred: !paper.starred });
      break;
    case 'save':
      await store.updatePaper(paper.id, {
        saved: !paper.saved,
        savedAt: !paper.saved ? Date.now() : null
      });
      break;
    case 'cite':
      const citation = `${(paper.authors || []).join(', ')}. ${paper.title}. ${paper.journal || 'Preprint'}. ${paper.date || ''}.${paper.pmid && !paper.pmid.startsWith('demo') ? ' PMID: ' + paper.pmid : ''}`;
      await navigator.clipboard.writeText(citation).catch(() => {});
      showToast('Citation copied to clipboard');
      return;
  }

  await renderCurrentTab();
  await renderSidebar();
}

async function showSignalBreakdown(pmid, anchorEl) {
  const paper = await store.getPaperByPmid(pmid);
  if (!paper || !paper.signalBreakdown) return;

  const popover = document.getElementById('signal-popover');
  const content = document.getElementById('signal-popover-content');

  const bd = paper.signalBreakdown;
  content.innerHTML = `
    <div class="flex justify-between"><span class="text-slate-400">Entity relevance</span><span class="text-white font-medium">${bd.entityRelevance || 0}</span></div>
    <div class="flex justify-between"><span class="text-slate-400">Journal tier</span><span class="text-white font-medium">${bd.journalTier || 0}</span></div>
    <div class="flex justify-between"><span class="text-slate-400">Recency</span><span class="text-white font-medium">${bd.recency || 0}</span></div>
    <div class="flex justify-between"><span class="text-slate-400">Novelty</span><span class="text-white font-medium">${bd.novelty || 0}</span></div>
    <div class="flex justify-between pt-1 border-t border-slate-700 mt-1"><span class="text-slate-300 font-medium">Total</span><span class="text-saffron-400 font-bold">${paper.signalScore}</span></div>`;

  const rect = anchorEl.getBoundingClientRect();
  popover.style.top = `${rect.bottom + 8}px`;
  popover.style.left = `${Math.max(8, rect.left - 60)}px`;
  popover.classList.remove('hidden');

  const closeHandler = (e) => {
    if (!popover.contains(e.target) && e.target !== anchorEl) {
      popover.classList.add('hidden');
      document.removeEventListener('click', closeHandler);
    }
  };
  setTimeout(() => document.addEventListener('click', closeHandler), 10);
}

async function renderCoMentionAlerts() {
  const container = document.getElementById('comention-alerts');
  const recent = await store.getRecentCoMentions(5);
  if (recent.length === 0) {
    container.classList.add('hidden');
    return;
  }

  container.classList.remove('hidden');
  container.innerHTML = `
    <div class="bg-purple-900/20 border border-purple-800/30 rounded-xl p-3">
      <h3 class="text-xs font-semibold text-purple-300 mb-2">Novel Co-mentions Detected</h3>
      <div class="space-y-1">
        ${recent.map(cm => `
          <div class="flex items-center gap-2 text-xs">
            <span class="chip bg-purple-800/40 text-purple-300">${escapeHtml(cm.entityA)}</span>
            <span class="text-purple-500">&harr;</span>
            <span class="chip bg-purple-800/40 text-purple-300">${escapeHtml(cm.entityB)}</span>
            <span class="text-slate-500 ml-auto">${timeSince(cm.discoveredAt)}</span>
          </div>
        `).join('')}
      </div>
    </div>`;
}

// ---------------------------------------------------------------------------
// 10. Watchlist Rendering & Management
// ---------------------------------------------------------------------------

async function renderWatchlist() {
  const container = document.getElementById('watchlist-entities');
  const emptyState = document.getElementById('watchlist-empty');
  const entities = (await store.getWatchlist()).sort((a, b) => (b.addedAt || 0) - (a.addedAt || 0));

  await renderPresetPacks();

  if (entities.length === 0) {
    container.innerHTML = '';
    emptyState.classList.remove('hidden');
    return;
  }

  emptyState.classList.add('hidden');
  container.innerHTML = entities.map((entity) => {
    const priorityBadge = entity.priority === 'high'
      ? '<span class="chip bg-purple-800/40 text-purple-300">high</span>'
      : entity.priority === 'low'
        ? '<span class="chip bg-slate-700/60 text-slate-400">low</span>'
        : '<span class="chip bg-saffron-400/10 text-saffron-400">normal</span>';

    const typeBadge = `<span class="chip bg-slate-700/50 text-slate-400">${escapeHtml(entity.type || 'unknown')}</span>`;
    const tags = (entity.tags || []).map(t => `<span class="chip bg-slate-700/30 text-slate-500">${escapeHtml(t)}</span>`).join('');
    const pausedClass = entity.paused ? 'opacity-50' : '';

    return `
      <div class="bg-surface-800 border border-slate-700/50 rounded-xl p-3 flex items-center justify-between gap-3 ${pausedClass} fade-in" data-entity-id="${entity.id}">
        <div class="flex items-center gap-3 min-w-0 flex-1">
          <span class="w-2 h-2 rounded-full ${entity.priority === 'high' ? 'bg-purple-400' : entity.priority === 'low' ? 'bg-slate-500' : 'bg-saffron-400'} flex-shrink-0"></span>
          <div class="min-w-0">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="text-sm font-medium text-white">${escapeHtml(entity.term)}</span>
              ${typeBadge}
              ${priorityBadge}
              ${entity.paused ? '<span class="chip bg-yellow-800/30 text-yellow-400">paused</span>' : ''}
            </div>
            <div class="flex flex-wrap gap-1 mt-1">${tags}</div>
          </div>
        </div>
        <div class="flex items-center gap-1 flex-shrink-0">
          <button class="entity-action p-1.5 rounded hover:bg-surface-700 transition" data-action="pause" data-id="${entity.id}" title="${entity.paused ? 'Resume' : 'Pause'}">
            <svg class="w-4 h-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="${entity.paused ? 'M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z' : 'M10 9v6m4-6v6'}"/></svg>
          </button>
          <button class="entity-action p-1.5 rounded hover:bg-surface-700 transition" data-action="delete" data-id="${entity.id}" title="Remove">
            <svg class="w-4 h-4 text-slate-500 hover:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/></svg>
          </button>
        </div>
      </div>`;
  }).join('');

  document.querySelectorAll('.entity-action').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const id = parseInt(btn.dataset.id);
      const action = btn.dataset.action;
      if (action === 'delete') {
        await store.deleteWatchlistEntity(id);
      } else if (action === 'pause') {
        const entity = await store.getWatchlistById(id);
        if (entity) await store.updateWatchlistEntity(id, { paused: !entity.paused });
      }
      await renderWatchlist();
      await renderSidebar();
    });
  });
}

function bindAddEntity() {
  const input = document.getElementById('add-entity-input');
  const btn = document.getElementById('add-entity-btn');
  const hint = document.getElementById('entity-type-hint');

  input.addEventListener('input', () => {
    const val = input.value.trim();
    if (val.length > 1 && window.BioKhojCore) {
      const detected = window.BioKhojCore.classifyEntity ? window.BioKhojCore.classifyEntity(val) : { type: guessEntityType(val) };
      hint.classList.remove('hidden');
      hint.querySelector('span').textContent = typeof detected === 'object' ? detected.type : detected;
    } else {
      hint.classList.add('hidden');
    }
  });

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') addEntity();
  });

  btn.addEventListener('click', addEntity);
}

async function addEntity() {
  const input = document.getElementById('add-entity-input');
  const prioritySelect = document.getElementById('add-entity-priority');
  const tagsInput = document.getElementById('entity-tags-input');
  const term = input.value.trim();
  if (!term) return;

  let type = 'topic';
  if (window.BioKhojCore && window.BioKhojCore.classifyEntity) {
    const result = window.BioKhojCore.classifyEntity(term);
    type = (typeof result === 'object' && result.type) ? result.type : (result || 'topic');
  } else {
    type = guessEntityType(term);
  }

  const tags = tagsInput && tagsInput.value
    ? tagsInput.value.split(',').map(t => t.trim()).filter(Boolean)
    : [];

  await store.addWatchlistEntity({
    term,
    type,
    priority: prioritySelect.value,
    tags
  });

  input.value = '';
  if (tagsInput) tagsInput.value = '';
  document.getElementById('entity-type-hint').classList.add('hidden');

  await store.saveSetting('onboarded', true);
  dismissOnboarding();

  await showConceptSuggestions(term, type);
  await renderWatchlist();
  await renderSidebar();

  // Auto-fetch papers for the newly added entity
  const allPapers = await store.getAllPapers();
  if (allPapers.length === 0) {
    // First entity added and no papers yet — fetch immediately
    showToast(`Checking PubMed for ${term}...`);
    const entity = await store.getWatchlistByTerm(term);
    if (entity) {
      await checkPapersForEntity(entity);
      await renderSidebar();
      if (currentTab === 'feed') await renderFeed();
    }
  }
}

async function showConceptSuggestions(term, type) {
  const container = document.getElementById('concept-suggestions');
  const list = document.getElementById('concept-suggestions-list');

  let suggestions = [];
  if (window.BioKhojCore && window.BioKhojCore.getConceptSuggestions) {
    try {
      suggestions = await window.BioKhojCore.getConceptSuggestions(term, type);
    } catch (e) {
      suggestions = [];
    }
  }

  if (!suggestions || suggestions.length === 0) {
    suggestions = getFallbackSuggestions(term, type);
  }

  if (suggestions.length === 0) {
    container.classList.add('hidden');
    return;
  }

  container.classList.remove('hidden');
  list.innerHTML = suggestions.map((s) => `
    <button class="suggestion-chip chip bg-saffron-400/10 text-saffron-400 border border-saffron-400/20 hover:bg-saffron-400/20 transition cursor-pointer" data-term="${escapeHtml(typeof s === 'string' ? s : s.term)}" data-type="${escapeHtml(typeof s === 'string' ? 'topic' : s.type || 'topic')}">
      + ${escapeHtml(typeof s === 'string' ? s : s.term)}
    </button>
  `).join('');

  list.querySelectorAll('.suggestion-chip').forEach((chip) => {
    chip.addEventListener('click', async () => {
      const existing = await store.getWatchlistByTerm(chip.dataset.term);
      if (!existing) {
        await store.addWatchlistEntity({
          term: chip.dataset.term,
          type: chip.dataset.type
        });
      }
      chip.remove();
      if (list.children.length === 0) container.classList.add('hidden');
      await renderWatchlist();
      await renderSidebar();
    });
  });
}

function getFallbackSuggestions(term, type) {
  const map = {
    gene: [
      { term: `${term} pathway`, type: 'pathway' },
      { term: `${term} mutation`, type: 'topic' },
      { term: `${term} inhibitor`, type: 'topic' },
    ],
    protein: [
      { term: `${term} structure`, type: 'topic' },
      { term: `${term} interaction`, type: 'topic' },
    ],
    disease: [
      { term: `${term} biomarker`, type: 'topic' },
      { term: `${term} treatment`, type: 'topic' },
      { term: `${term} genetics`, type: 'topic' },
    ],
  };
  return map[type] || [];
}

function guessEntityType(term) {
  const upper = term.toUpperCase();
  if (/^[A-Z][A-Z0-9]{1,6}$/.test(upper) && !/^(THE|AND|FOR|NOT|BUT)$/.test(upper)) return 'gene';
  if (/kinase|ase$|receptor|protein|antibody/i.test(term)) return 'protein';
  if (/pathway|signaling|cascade/i.test(term)) return 'pathway';
  if (/disease|syndrome|disorder|cancer|tumor|carcinoma/i.test(term)) return 'disease';
  if (/seq$|sequencing|PCR|CRISPR|assay|method|technique|imaging/i.test(term)) return 'technique';
  if (/drug|inhibitor|agonist|antagonist|therapeutic/i.test(term)) return 'drug';
  return 'topic';
}

// ---------------------------------------------------------------------------
// 11. Trends Rendering
// ---------------------------------------------------------------------------

async function renderTrends() {
  const trendData = await store.getAllTrends();
  const emptyState = document.getElementById('trends-empty');

  if (trendData.length === 0) {
    document.getElementById('trends-charts').classList.add('hidden');
    emptyState.classList.remove('hidden');
    return;
  }

  document.getElementById('trends-charts').classList.remove('hidden');
  emptyState.classList.add('hidden');

  await renderTrendsInsights(trendData);
  await renderVolumeChart(trendData);
  await renderJournalChart();
}

async function renderTrendsInsights(trendData) {
  let insightBox = document.getElementById('trends-insights');
  if (!insightBox) {
    insightBox = document.createElement('div');
    insightBox.id = 'trends-insights';
    insightBox.className = 'bg-surface-800 border border-slate-700/50 rounded-xl p-4 mb-4';
    const chartsEl = document.getElementById('trends-charts');
    chartsEl.insertBefore(insightBox, chartsEl.firstChild);
  }

  const papers = await store.getAllPapers();
  const now = Date.now();
  const weekMs = 7 * 24 * 60 * 60 * 1000;
  const monthMs = 30 * 24 * 60 * 60 * 1000;

  const thisWeek = papers.filter(p => (new Date(p.date).getTime() || 0) > now - weekMs);
  const lastMonth = papers.filter(p => {
    const t = new Date(p.date).getTime() || 0;
    return t > now - monthMs && t <= now - weekMs;
  });

  // Entity activity
  const entityCounts = {};
  const entityCountsPrev = {};
  thisWeek.forEach(p => (p.matchedEntities || []).forEach(e => { entityCounts[e] = (entityCounts[e] || 0) + 1; }));
  lastMonth.forEach(p => (p.matchedEntities || []).forEach(e => { entityCountsPrev[e] = (entityCountsPrev[e] || 0) + 1; }));

  const insights = [];

  // Rising entities
  for (const [ent, cnt] of Object.entries(entityCounts)) {
    const prev = entityCountsPrev[ent] || 0;
    if (prev > 0 && cnt > prev) {
      const pctChange = Math.round(((cnt - prev) / prev) * 100);
      if (pctChange >= 20) insights.push(`<strong class="text-white">${escapeHtml(ent)}</strong> up ${pctChange}% this week (${cnt} vs ${prev})`);
    }
  }

  // Top journal this week
  const journalCounts = {};
  thisWeek.forEach(p => { if (p.journal) journalCounts[p.journal] = (journalCounts[p.journal] || 0) + 1; });
  const topJournal = Object.entries(journalCounts).sort((a, b) => b[1] - a[1])[0];
  if (topJournal && topJournal[1] >= 2) {
    insights.push(`<strong class="text-white">${escapeHtml(topJournal[0])}</strong> contributed ${topJournal[1]} papers this week`);
  }

  // Co-mention spikes
  const coMentions = await store.getRecentCoMentions(3);
  if (coMentions.length > 0) {
    insights.push(`Co-mention spike: <strong class="text-white">${escapeHtml(coMentions[0].entityA)}</strong> + <strong class="text-white">${escapeHtml(coMentions[0].entityB)}</strong>`);
  }

  // High signal count
  const highSignal = thisWeek.filter(p => (p.signalScore || 0) >= 70);
  if (highSignal.length > 0) {
    insights.push(`${highSignal.length} high-signal paper${highSignal.length > 1 ? 's' : ''} this week`);
  }

  if (insights.length === 0) {
    insightBox.classList.add('hidden');
    return;
  }

  insightBox.classList.remove('hidden');
  insightBox.innerHTML = `
    <h3 class="text-xs font-semibold text-saffron-400 uppercase tracking-wide mb-2">This Week's Insights</h3>
    <ul class="space-y-1 text-xs text-slate-400">
      ${insights.map(i => `<li class="flex items-start gap-2"><span class="text-saffron-400 mt-0.5">&#9679;</span><span>${i}</span></li>`).join('')}
    </ul>`;
}

async function renderVolumeChart(trendData) {
  const canvas = document.getElementById('chart-volume');
  if (!canvas || typeof Chart === 'undefined') return;

  if (charts.volume) charts.volume.destroy();

  const grouped = {};
  for (const t of trendData) {
    if (!grouped[t.entity]) grouped[t.entity] = [];
    grouped[t.entity].push({ x: t.date, y: t.count });
  }

  const colors = ['#F4C430', '#a855f7', '#0ea5e9', '#22c55e', '#f97316', '#ec4899'];
  const datasets = Object.entries(grouped).map(([entity, points], i) => ({
    label: entity,
    data: points.sort((a, b) => a.x.localeCompare(b.x)),
    borderColor: colors[i % colors.length],
    backgroundColor: colors[i % colors.length] + '20',
    fill: true,
    tension: 0.3,
    pointRadius: 2,
  }));

  charts.volume = new Chart(canvas, {
    type: 'line',
    data: { datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: { legend: { labels: { color: '#94a3b8', font: { size: 11 } } } },
      scales: {
        x: { type: 'category', ticks: { color: '#64748b', font: { size: 10 } }, grid: { color: '#1e293b' } },
        y: { ticks: { color: '#64748b', font: { size: 10 } }, grid: { color: '#1e293b' } }
      }
    }
  });
}

async function renderJournalChart() {
  const canvas = document.getElementById('chart-journals');
  if (!canvas || typeof Chart === 'undefined') return;

  if (charts.journals) charts.journals.destroy();

  const papers = await store.getAllPapers();
  const journalCounts = {};
  for (const p of papers) {
    const j = p.journal || 'Unknown';
    journalCounts[j] = (journalCounts[j] || 0) + 1;
  }

  const sorted = Object.entries(journalCounts).sort((a, b) => b[1] - a[1]).slice(0, 10);
  if (sorted.length === 0) return;

  const colors = ['#F4C430', '#a855f7', '#0ea5e9', '#22c55e', '#f97316', '#ec4899', '#14b8a6', '#f43f5e', '#8b5cf6', '#06b6d4'];

  charts.journals = new Chart(canvas, {
    type: 'doughnut',
    data: {
      labels: sorted.map(s => s[0]),
      datasets: [{ data: sorted.map(s => s[1]), backgroundColor: colors.slice(0, sorted.length), borderWidth: 0 }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: { legend: { position: 'right', labels: { color: '#94a3b8', font: { size: 11 }, padding: 12 } } }
    }
  });
}

// ---------------------------------------------------------------------------
// 12. Reading List
// ---------------------------------------------------------------------------

async function renderReadingList() {
  const container = document.getElementById('reading-list-papers');
  const emptyState = document.getElementById('reading-list-empty');
  const countEl = document.getElementById('reading-list-count');

  let papers = await store.getSavedPapers();

  countEl.textContent = `${papers.length} saved paper${papers.length !== 1 ? 's' : ''}`;

  if (papers.length === 0) {
    container.innerHTML = '';
    emptyState.classList.remove('hidden');
    document.getElementById('reading-tags-filter').classList.add('hidden');
    return;
  }

  emptyState.classList.add('hidden');

  const sortBy = document.getElementById('reading-sort').value;
  if (sortBy === 'saved') papers.sort((a, b) => (b.savedAt || 0) - (a.savedAt || 0));
  else if (sortBy === 'date') papers.sort((a, b) => new Date(b.date) - new Date(a.date));
  else if (sortBy === 'signal') papers.sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0));
  else if (sortBy === 'citations') papers.sort((a, b) => ((citationCache[b.pmid] || {}).currentCount || 0) - ((citationCache[a.pmid] || {}).currentCount || 0));

  const allTags = new Set();
  papers.forEach(p => (p.paperTags || []).forEach(t => allTags.add(t)));
  const tagFilter = document.getElementById('reading-tags-filter');
  if (allTags.size > 0) {
    tagFilter.classList.remove('hidden');
    tagFilter.innerHTML = `<button class="chip bg-saffron-400/10 text-saffron-400 border border-saffron-400/20 cursor-pointer reading-tag-filter" data-tag="">All</button>` +
      Array.from(allTags).map(t => `<button class="chip bg-slate-700/50 text-slate-400 cursor-pointer reading-tag-filter hover:bg-slate-700 transition" data-tag="${escapeHtml(t)}">${escapeHtml(t)}</button>`).join('');
  } else {
    tagFilter.classList.add('hidden');
  }

  const allCitations = await store.getAllCitations();
  allCitations.forEach(c => { citationCache[c.pmid] = c; });

  container.innerHTML = papers.map(p => renderPaperCard(p, true)).join('');
  bindPaperActions();

  fetchCitationsForReadingList().then(async () => {
    const updatedContainer = document.getElementById('reading-list-papers');
    if (updatedContainer && currentTab === 'reading-list') {
      let updatedPapers = await store.getSavedPapers();
      const sort = document.getElementById('reading-sort').value;
      if (sort === 'saved') updatedPapers.sort((a, b) => (b.savedAt || 0) - (a.savedAt || 0));
      else if (sort === 'date') updatedPapers.sort((a, b) => new Date(b.date) - new Date(a.date));
      else if (sort === 'signal') updatedPapers.sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0));
      updatedContainer.innerHTML = updatedPapers.map(p => renderPaperCard(p, true)).join('');
      bindPaperActions();
    }
  });
}

// ---------------------------------------------------------------------------
// 13. Settings
// ---------------------------------------------------------------------------

async function renderSettings() {
  const apiKey = await store.getSetting('ncbiApiKey');
  if (apiKey) document.getElementById('settings-api-key').value = apiKey;
  renderApiBudget();

  // Refresh interval + status
  const refreshInterval = (await store.getSetting('refreshInterval')) || 60;
  const refreshSelect = document.getElementById('settings-refresh-interval');
  if (refreshSelect) refreshSelect.value = String(refreshInterval);
  const lastCheck = await store.getSetting('lastGlobalCheck');
  const checkStatus = document.getElementById('settings-check-status');
  if (checkStatus) {
    const wlCount = await store.getWatchlistCount();
    if (wlCount === 0) {
      checkStatus.textContent = 'No entities watched. Add entities to start checking PubMed.';
    } else {
      const lastText = lastCheck ? timeSince(lastCheck) : 'never';
      const nextMs = lastCheck ? (lastCheck + refreshInterval * 60 * 1000) - Date.now() : 0;
      const nextText = nextMs > 0 ? 'in ' + formatDuration(nextMs) : 'on next app open';
      checkStatus.textContent = `Last check: ${lastText}. Next check: ${nextText}.`;
    }
  }

  // Signal mute threshold
  const muteThreshold = (await store.getSetting('muteThreshold')) || 0;
  const muteSlider = document.getElementById('settings-mute-threshold');
  const muteLabel = document.getElementById('settings-mute-label');
  if (muteSlider) {
    muteSlider.value = muteThreshold;
    muteLabel.textContent = muteThreshold;
  }

  const notifications = await store.getSetting('notifications');
  document.getElementById('settings-notifications').checked = !!notifications;

  const theme = await store.getSetting('theme');
  document.getElementById('settings-theme').checked = theme !== 'light';

  const tiers = await store.getSetting('journalTiers');
  if (tiers) {
    if (tiers.tier1) document.getElementById('tier1-journals').value = tiers.tier1;
    if (tiers.tier2) document.getElementById('tier2-journals').value = tiers.tier2;
    if (tiers.tier3) document.getElementById('tier3-journals').value = tiers.tier3;
  }
}

function bindSettingsControls() {
  document.getElementById('save-api-key-btn').addEventListener('click', async () => {
    const key = document.getElementById('settings-api-key').value.trim();
    await store.saveSetting('ncbiApiKey', key);
    if (window.BioKhojCore && window.BioKhojCore.setNcbiApiKey) {
      window.BioKhojCore.setNcbiApiKey(key);
    }
    showToast('API key saved');
  });

  document.getElementById('settings-notifications').addEventListener('change', async (e) => {
    const enabled = e.target.checked;
    await store.saveSetting('notifications', enabled);
    if (enabled && 'Notification' in window) {
      const perm = await Notification.requestPermission();
      if (perm !== 'granted') {
        e.target.checked = false;
        await store.saveSetting('notifications', false);
        showToast('Notification permission denied');
      }
    }
  });

  document.getElementById('settings-theme').addEventListener('change', async (e) => {
    const dark = e.target.checked;
    await store.saveSetting('theme', dark ? 'dark' : 'light');
    applyTheme();
  });

  document.getElementById('save-tiers-btn').addEventListener('click', async () => {
    await store.saveSetting('journalTiers', {
      tier1: document.getElementById('tier1-journals').value,
      tier2: document.getElementById('tier2-journals').value,
      tier3: document.getElementById('tier3-journals').value,
    });
    showToast('Journal tiers saved');
  });

  // Refresh interval
  const refreshSelect = document.getElementById('settings-refresh-interval');
  if (refreshSelect) {
    refreshSelect.addEventListener('change', async () => {
      const val = parseInt(refreshSelect.value);
      await store.saveSetting('refreshInterval', val);
      const labels = { 30: '30 minutes', 60: '1 hour', 360: '6 hours', 1440: 'daily', 10080: 'weekly' };
      showToast(`Check frequency set to ${labels[val] || val + ' minutes'}`);
    });
  }

  // Signal mute threshold slider
  const muteSlider = document.getElementById('settings-mute-threshold');
  if (muteSlider) {
    muteSlider.addEventListener('input', () => {
      document.getElementById('settings-mute-label').textContent = muteSlider.value;
    });
    muteSlider.addEventListener('change', async () => {
      await store.saveSetting('muteThreshold', parseInt(muteSlider.value));
      showToast(`Papers below signal ${muteSlider.value} will be hidden`);
    });
  }

  document.getElementById('clear-data-btn').addEventListener('click', async () => {
    if (!confirm('Delete all BioKhoj data? This cannot be undone.')) return;
    await store.clearAll();
    citationCache = {};
    showToast('All data cleared');
    await renderCurrentTab();
    await renderSidebar();
  });
}

async function loadSettings() {
  const apiKey = await store.getSetting('ncbiApiKey');
  if (apiKey && window.BioKhojCore && window.BioKhojCore.setNcbiApiKey) {
    window.BioKhojCore.setNcbiApiKey(apiKey);
  }
}

function applyTheme() {
  store.getSetting('theme').then((theme) => {
    if (theme === 'light') {
      document.documentElement.classList.remove('dark');
    } else {
      document.documentElement.classList.add('dark');
    }
  });
}

// ---------------------------------------------------------------------------
// 13b. API Budget Visualizer
// ---------------------------------------------------------------------------

let budgetInterval = null;

function renderApiBudget() {
  if (!window.BioKhojCore || !window.BioKhojCore.getApiBudget) return;
  const budget = window.BioKhojCore.getApiBudget();

  const bar = document.getElementById('budget-bar');
  const label = document.getElementById('budget-label');
  const status = document.getElementById('budget-status');
  const keyStatus = document.getElementById('budget-key-status');
  const rateEl = document.getElementById('budget-rate');
  const hourlyEl = document.getElementById('budget-hourly');
  if (!bar) return;

  rateEl.textContent = budget.perSecond;
  hourlyEl.textContent = budget.limit.toLocaleString();
  label.textContent = `${budget.used.toLocaleString()} / ${budget.limit.toLocaleString()}`;
  bar.style.width = Math.min(100, budget.pct) + '%';

  // Color based on usage
  if (budget.pct >= 80) {
    bar.className = 'h-3 rounded-full transition-all duration-500 bg-red-500';
    status.textContent = 'Approaching limit — slow down or add API key';
    status.className = 'text-xs text-red-400';
  } else if (budget.pct >= 50) {
    bar.className = 'h-3 rounded-full transition-all duration-500 bg-amber-500';
    status.textContent = 'Moderate usage';
    status.className = 'text-xs text-amber-400';
  } else {
    bar.className = 'h-3 rounded-full transition-all duration-500 bg-emerald-500';
    status.textContent = 'All clear';
    status.className = 'text-xs text-emerald-400';
  }

  keyStatus.textContent = budget.hasApiKey ? 'API key active (10 req/s)' : 'No API key (3 req/s)';
  keyStatus.className = budget.hasApiKey ? 'text-xs text-emerald-500' : 'text-xs text-slate-500';

  // Auto-refresh while settings tab is open
  if (currentTab === 'settings' && !budgetInterval) {
    budgetInterval = setInterval(() => {
      if (currentTab === 'settings') renderApiBudget();
      else { clearInterval(budgetInterval); budgetInterval = null; }
    }, 5000);
  }
}

// ---------------------------------------------------------------------------
// 14. Export / Import
// ---------------------------------------------------------------------------

function bindExportImport() {
  document.getElementById('export-watchlist-btn').addEventListener('click', async () => {
    const entities = await store.getWatchlist();
    downloadJSON(entities, 'biokhoj-watchlist.json');
  });

  document.getElementById('import-watchlist-input').addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    try {
      const text = await file.text();
      const data = JSON.parse(text);
      const items = Array.isArray(data) ? data : (data.watchlist || data.entities || []);
      let count = 0;
      for (const item of items) {
        const term = item.term || item.name || item.query;
        if (!term) continue;
        const existing = await store.getWatchlistByTerm(term);
        if (!existing) {
          await store.addWatchlistEntity({
            term,
            type: item.type || 'topic',
            priority: item.priority || 'normal',
            tags: item.tags || []
          });
          count++;
        }
      }
      showToast(`Imported ${count} entities`);
      await renderWatchlist();
      await renderSidebar();
    } catch (err) {
      showToast('Failed to import: invalid JSON');
    }
    e.target.value = '';
  });

  // BioGist import
  document.getElementById('import-biogist-btn').addEventListener('click', async () => {
    const input = prompt('Paste BioGist watchlist JSON or entity list (comma-separated):');
    if (!input) return;
    try {
      let items;
      try {
        items = JSON.parse(input);
        if (!Array.isArray(items)) items = items.watchlist || items.entities || [items];
      } catch {
        items = input.split(',').map(s => ({ term: s.trim(), type: 'topic' }));
      }
      let count = 0;
      for (const item of items) {
        const term = typeof item === 'string' ? item : (item.term || item.name);
        if (!term) continue;
        const existing = await store.getWatchlistByTerm(term);
        if (!existing) {
          await store.addWatchlistEntity({
            term,
            type: (typeof item === 'object' ? item.type : null) || guessEntityType(term)
          });
          count++;
        }
      }
      showToast(`Imported ${count} entities from BioGist`);
      await renderWatchlist();
      await renderSidebar();
    } catch (err) {
      showToast('Failed to import');
    }
  });
}

window.exportReadingList = async function (format) {
  const papers = await store.getSavedPapers();
  document.getElementById('export-dropdown').classList.add('hidden');

  if (papers.length === 0) {
    showToast('No saved papers to export');
    return;
  }

  let content, filename, mimeType;

  switch (format) {
    case 'bibtex':
      content = papers.map((p, i) => {
        const key = (p.authors?.[0]?.split(' ')[0] || 'Unknown') + (p.date?.slice(0, 4) || '') + (i + 1);
        return `@article{${key},\n  title = {${p.title}},\n  author = {${(p.authors || []).join(' and ')}},\n  journal = {${p.journal || 'Preprint'}},\n  year = {${p.date?.slice(0, 4) || ''}},\n  pmid = {${p.pmid || ''}}\n}`;
      }).join('\n\n');
      filename = 'biokhoj-reading-list.bib';
      mimeType = 'application/x-bibtex';
      break;

    case 'ris':
      content = papers.map((p) => {
        const lines = ['TY  - JOUR', `TI  - ${p.title}`];
        (p.authors || []).forEach(a => lines.push(`AU  - ${a}`));
        lines.push(`JO  - ${p.journal || 'Preprint'}`);
        if (p.date) lines.push(`DA  - ${p.date}`);
        if (p.pmid) lines.push(`AN  - ${p.pmid}`);
        if (p.abstract) lines.push(`AB  - ${p.abstract}`);
        lines.push('ER  - ');
        return lines.join('\n');
      }).join('\n\n');
      filename = 'biokhoj-reading-list.ris';
      mimeType = 'application/x-research-info-systems';
      break;

    case 'markdown':
      content = '# BioKhoj Reading List\n\n' + papers.map((p, i) => {
        let md = `${i + 1}. **${p.title}**\n   ${(p.authors || []).join(', ')}. *${p.journal || 'Preprint'}*. ${p.date || ''}.`;
        if (p.pmid && !p.pmid.startsWith('demo')) md += ` PMID: ${p.pmid}`;
        if (p.notes) md += `\n   > ${p.notes}`;
        return md;
      }).join('\n\n');
      filename = 'biokhoj-reading-list.md';
      mimeType = 'text/markdown';
      break;

    case 'pandas':
      content = 'import pandas as pd\n\ndf = pd.DataFrame(' +
        JSON.stringify(papers.map(p => ({
          pmid: p.pmid || '', title: p.title, authors: (p.authors || []).join('; '),
          journal: p.journal || '', date: p.date || '', signal_score: p.signalScore || 0,
          doi: p.doi || '', entities: (p.matchedEntities || []).join('; ')
        })), null, 2) + ')\n\ndf.head()';
      await navigator.clipboard.writeText(content).catch(() => {});
      showToast(`Copied ${papers.length} papers as pandas DataFrame`);
      return;

    case 'r':
      const rows = papers.map(p =>
        `  c(pmid="${(p.pmid || '').replace(/"/g, '\\"')}", title="${p.title.replace(/"/g, '\\"')}", ` +
        `authors="${(p.authors || []).join('; ').replace(/"/g, '\\"')}", ` +
        `journal="${(p.journal || '').replace(/"/g, '\\"')}", ` +
        `date="${p.date || ''}", signal_score=${p.signalScore || 0}, ` +
        `doi="${(p.doi || '').replace(/"/g, '\\"')}")`
      );
      content = 'library(tibble)\n\ndf <- tibble(\n  pmid = c(' +
        papers.map(p => `"${(p.pmid || '').replace(/"/g, '\\"')}"`).join(', ') + '),\n  title = c(' +
        papers.map(p => `"${p.title.replace(/"/g, '\\"')}"`).join(', ') + '),\n  journal = c(' +
        papers.map(p => `"${(p.journal || '').replace(/"/g, '\\"')}"`).join(', ') + '),\n  date = c(' +
        papers.map(p => `"${p.date || ''}"`).join(', ') + '),\n  signal_score = c(' +
        papers.map(p => p.signalScore || 0).join(', ') + ')\n)\n\nhead(df)';
      await navigator.clipboard.writeText(content).catch(() => {});
      showToast(`Copied ${papers.length} papers as R tibble`);
      return;

    case 'csv':
      const csvHeader = 'pmid,title,authors,journal,date,signal_score,doi,entities';
      const csvRows = papers.map(p => {
        const esc = s => `"${(s || '').replace(/"/g, '""')}"`;
        return [esc(p.pmid), esc(p.title), esc((p.authors || []).join('; ')),
          esc(p.journal), esc(p.date), p.signalScore || 0,
          esc(p.doi), esc((p.matchedEntities || []).join('; '))].join(',');
      });
      content = csvHeader + '\n' + csvRows.join('\n');
      filename = 'biokhoj-reading-list.csv';
      mimeType = 'text/csv';
      break;
  }

  downloadFile(content, filename, mimeType);
  showToast(`Exported ${papers.length} papers as ${format.toUpperCase()}`);
};

// ---------------------------------------------------------------------------
// 15. Background Paper Checks
// ---------------------------------------------------------------------------

async function scheduleBackgroundChecks() {
  const wlCount = await store.getWatchlistCount();
  if (wlCount === 0) return; // Nothing to check
  checkStalePapers();
}

// ---------------------------------------------------------------------------
// 15b. Gap-Aware Catch-Up
// ---------------------------------------------------------------------------

async function runCatchUp() {
  if (!window.BioKhojCore || !window.BioKhojCore.searchPubMed) return;

  const gap = await store.detectGap();
  if (!gap.needsCatchUp) return;

  const entities = await store.getActiveWatchlist();
  if (entities.length === 0) return;

  const cappedDays = Math.min(gap.gapDays, store.BACKLOG_MAX_DAYS);
  const windows = store.buildCatchUpWindows(gap.lastChecked);

  // Show progress banner
  showBacklogProgress(cappedDays, 0, 0);

  let totalFound = 0;

  for (let wi = 0; wi < windows.length; wi++) {
    const win = windows[wi];
    for (const entity of entities) {
      try {
        const papers = await window.BioKhojCore.searchPubMed(entity.term, {
          maxResults: 30,
          minDate: win.minDate,
          maxDate: win.maxDate
        });

        if (papers && papers.length > 0) {
          for (const paper of papers) {
            const existing = await store.getPaperByPmid(paper.pmid);
            if (!existing) {
              let signalScore = 50;
              let signalBreakdown = { entityRelevance: 20, journalTier: 10, recency: 10, novelty: 10 };

              if (window.BioKhojCore.computeSignalScore) {
                const watchlist = await store.getWatchlist();
                const tiers = await store.getSetting('journalTiers');
                const result = window.BioKhojCore.computeSignalScore(paper, watchlist, {});
                signalScore = result.score || 50;
                signalBreakdown = result.breakdown || signalBreakdown;
              }

              await store.addPaper({
                pmid: paper.pmid,
                title: paper.title,
                authors: typeof paper.authors === 'string'
                  ? paper.authors.split(', ')
                  : (paper.authors || []),
                journal: paper.journal || '',
                date: paper.date || '',
                abstract: paper.abstract || '',
                matchedEntities: paper.matchedEntities || [entity.term],
                signalScore,
                signalBreakdown,
                backlog: true,
                backlogDismissed: false,
                backlogWindow: `${win.minDate} - ${win.maxDate}`
              });

              totalFound++;
            }
          }

          // Detect co-mentions in backlog papers too
          for (const paper of papers) {
            if (paper.matchedEntities && paper.matchedEntities.length >= 2) {
              const ents = paper.matchedEntities;
              for (let i = 0; i < ents.length; i++) {
                for (let j = i + 1; j < ents.length; j++) {
                  const existing = await store.findCoMention(ents[i], ents[j]);
                  if (!existing) {
                    await store.addCoMention(ents[i], ents[j], paper.pmid);
                  }
                }
              }
            }
          }
        }

        // Rate limit courtesy
        await sleep(350);
      } catch (err) {
        console.warn(`Catch-up failed for ${entity.term} window ${win.minDate}-${win.maxDate}:`, err);
      }
    }

    // Update progress banner after each window
    const pct = Math.round(((wi + 1) / windows.length) * 100);
    showBacklogProgress(cappedDays, totalFound, pct);
  }

  // Update last checked to now
  for (const entity of entities) {
    await store.updateWatchlistEntity(entity.id, { lastChecked: Date.now() });
  }
  await store.saveSetting('lastGlobalCheck', Date.now());

  // Show final backlog banner (or hide if nothing found)
  if (totalFound > 0) {
    showBacklogBanner(cappedDays, totalFound);
  } else {
    hideBacklogBanner();
  }

  // Re-render feed with backlog papers
  await renderSidebar();
  if (currentTab === 'feed') await renderFeed();
}

function showBacklogProgress(days, found, pct) {
  const banner = document.getElementById('backlog-banner');
  if (!banner) return;
  banner.classList.remove('hidden');
  banner.innerHTML = `
    <div class="bg-blue-900/30 border border-blue-700/40 rounded-xl p-4">
      <div class="flex items-center gap-3 mb-2">
        <svg class="w-5 h-5 text-blue-400 animate-spin flex-shrink-0" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path></svg>
        <div class="flex-1">
          <p class="text-sm font-medium text-blue-200">Catching up on ${days} day${days !== 1 ? 's' : ''} of papers...</p>
          <p class="text-xs text-blue-400 mt-0.5">${found} paper${found !== 1 ? 's' : ''} found so far &middot; ${pct}% complete</p>
        </div>
      </div>
      <div class="w-full bg-blue-900/50 rounded-full h-1.5">
        <div class="bg-blue-400 h-1.5 rounded-full transition-all duration-300" style="width: ${pct}%"></div>
      </div>
    </div>`;
}

function showBacklogBanner(days, count) {
  const banner = document.getElementById('backlog-banner');
  if (!banner) return;
  banner.classList.remove('hidden');

  const capped = days >= store.BACKLOG_MAX_DAYS;

  banner.innerHTML = `
    <div class="bg-saffron-400/10 border border-saffron-400/30 rounded-xl p-4">
      <div class="flex items-start gap-3">
        <span class="text-xl flex-shrink-0">&#128232;</span>
        <div class="flex-1">
          <p class="text-sm font-semibold text-saffron-300">Welcome back! You were away ${days} day${days !== 1 ? 's' : ''}.</p>
          <p class="text-xs text-slate-400 mt-1">
            <strong class="text-white">${count} new paper${count !== 1 ? 's' : ''}</strong> found while you were away.
            ${capped ? `<br><span class="text-slate-500">Showing papers from last ${store.BACKLOG_MAX_DAYS} days. Older papers may be missed.</span>` : ''}
          </p>
          <div class="flex flex-wrap gap-2 mt-3">
            <button id="backlog-review-btn" class="px-3 py-1.5 bg-saffron-400 text-surface-900 rounded-lg text-xs font-semibold hover:bg-saffron-300 transition">Review backlog</button>
            <button id="backlog-mark-read-btn" class="px-3 py-1.5 bg-surface-700 text-slate-300 rounded-lg text-xs hover:bg-surface-600 transition">Mark all read</button>
            <button id="backlog-dismiss-btn" class="px-3 py-1.5 text-xs text-slate-500 hover:text-slate-300 transition">Dismiss</button>
          </div>
        </div>
      </div>
    </div>`;

  document.getElementById('backlog-review-btn').addEventListener('click', async () => {
    // Acknowledge the backlog (remove banner) but keep papers unread
    await store.acknowledgeBacklog();
    hideBacklogBanner();
    // Sort feed by date so backlog papers are visible in order
    document.getElementById('feed-sort').value = 'date';
    await renderFeed();
  });

  document.getElementById('backlog-mark-read-btn').addEventListener('click', async () => {
    await store.dismissBacklog();
    hideBacklogBanner();
    await renderFeed();
    await renderSidebar();
    showToast(`${count} backlog papers marked as read`);
  });

  document.getElementById('backlog-dismiss-btn').addEventListener('click', async () => {
    await store.acknowledgeBacklog();
    hideBacklogBanner();
  });
}

function hideBacklogBanner() {
  const banner = document.getElementById('backlog-banner');
  if (banner) {
    banner.classList.add('hidden');
    banner.innerHTML = '';
  }
}

async function checkStalePapers() {
  const allEntities = await store.getActiveWatchlist();
  const refreshInterval = (await store.getSetting('refreshInterval')) || 60; // minutes, default 1 hour
  const staleThreshold = refreshInterval * 60 * 1000;
  const now = Date.now();

  for (const entity of allEntities) {
    if (now - (entity.lastChecked || 0) > staleThreshold) {
      await checkPapersForEntity(entity);
    }
  }

  await renderSidebar();
  if (currentTab === 'feed') await renderFeed();
}

async function checkPapersForEntity(entity) {
  if (!window.BioKhojCore) return;

  try {
    // Search both PubMed and bioRxiv in parallel
    const [pubmedResults, biorxivResults] = await Promise.allSettled([
      window.BioKhojCore.searchPubMed ? window.BioKhojCore.searchPubMed(entity.term, { maxResults: 20, daysBack: 7 }) : [],
      window.BioKhojCore.searchBioRxiv ? window.BioKhojCore.searchBioRxiv(entity.term, 10, 7) : []
    ]);

    const pubmedPapers = pubmedResults.status === 'fulfilled' ? (pubmedResults.value || []) : [];
    const biorxivPapers = biorxivResults.status === 'fulfilled' ? (biorxivResults.value || []) : [];

    // Deduplicate across sources
    const allPapers = window.BioKhojCore.dedupPapers
      ? window.BioKhojCore.dedupPapers(pubmedPapers, biorxivPapers)
      : [...pubmedPapers, ...biorxivPapers];

    if (allPapers.length > 0) {
      for (const paper of allPapers) {
        const paperId = paper.pmid || paper.doi || null;
        if (!paperId) continue;
        const existing = await store.getPaperByPmid(paperId);
        if (!existing) {
          let signalScore = 50;
          let signalBreakdown = { entityRelevance: 20, journalTier: 10, recency: 10, novelty: 10 };

          if (window.BioKhojCore.computeSignalScore) {
            const watchlist = await store.getWatchlist();
            const result = window.BioKhojCore.computeSignalScore(paper, watchlist, {});
            signalScore = result.total || result.score || 50;
            signalBreakdown = result.breakdown || signalBreakdown;
          }

          await store.addPaper({
            pmid: paperId,
            doi: paper.doi || '',
            title: paper.title,
            authors: typeof paper.authors === 'string'
              ? paper.authors.split(', ')
              : (paper.authors || []),
            journal: paper.journal || '',
            date: paper.date || '',
            abstract: paper.abstract || '',
            matchedEntities: paper.matchedEntities || [entity.term],
            signalScore,
            signalBreakdown,
            source: paper.source || 'pubmed'
          });

          // Check for co-mentions
          if (paper.matchedEntities && paper.matchedEntities.length >= 2) {
            const ents = paper.matchedEntities;
            for (let i = 0; i < ents.length; i++) {
              for (let j = i + 1; j < ents.length; j++) {
                const existing = await store.findCoMention(ents[i], ents[j]);
                if (!existing) {
                  await store.addCoMention(ents[i], ents[j], paperId);
                }
              }
            }
          }
        }
      }
    }

    await store.updateWatchlistEntity(entity.id, { lastChecked: Date.now() });
    await store.saveSetting('lastGlobalCheck', Date.now());

    dismissWarningBanner();
  } catch (err) {
    console.warn(`Check failed for ${entity.term}:`, err);
    showWarningBanner('error', entity, err.message);
  }
}

window.triggerManualCheck = async function () {
  const refreshBtn = document.getElementById('sidebar-refresh-btn');
  if (refreshBtn) refreshBtn.classList.add('animate-spin');

  if (currentTab === 'feed') {
    showFeedSkeletons();
    showSidebarSkeletons();
  }

  const entities = await store.getActiveWatchlist();
  for (const entity of entities) {
    await checkPapersForEntity(entity);
    await sleep(350);
  }

  if (refreshBtn) refreshBtn.classList.remove('animate-spin');

  await renderSidebar();
  await renderCurrentTab();
};

window.closeModal = function (id) {
  document.getElementById(id).classList.add('hidden');
};

// ---------------------------------------------------------------------------
// 16. Search, Sort, Export Dropdown Bindings
// ---------------------------------------------------------------------------

function bindGlobalSearch() {
  const input = document.getElementById('global-search');
  let timeout;
  input.addEventListener('input', () => {
    clearTimeout(timeout);
    timeout = setTimeout(() => {
      if (currentTab === 'feed') renderFeed();
    }, 300);
  });
}

function bindFeedSort() {
  document.getElementById('feed-sort').addEventListener('change', () => {
    if (currentTab === 'feed') renderFeed();
  });
}

function bindReadingSort() {
  document.getElementById('reading-sort').addEventListener('change', () => {
    if (currentTab === 'reading-list') renderReadingList();
  });
}

function bindExportDropdown() {
  const btn = document.getElementById('export-btn');
  const dropdown = document.getElementById('export-dropdown');
  btn.addEventListener('click', (e) => {
    e.stopPropagation();
    dropdown.classList.toggle('hidden');
  });
  document.addEventListener('click', () => dropdown.classList.add('hidden'));
}

// ---------------------------------------------------------------------------
// 17. Utility Functions
// ---------------------------------------------------------------------------

function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function timeSince(timestamp) {
  if (!timestamp) return 'never';
  const seconds = Math.floor((Date.now() - timestamp) / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function formatDuration(ms) {
  if (ms <= 0) return 'now';
  const mins = Math.floor(ms / 60000);
  if (mins < 60) return `${mins}m`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ${mins % 60}m`;
  const days = Math.floor(hrs / 24);
  return `${days}d`;
}

function entityChipColor(entity) {
  let hash = 0;
  for (let i = 0; i < entity.length; i++) hash = entity.charCodeAt(i) + ((hash << 5) - hash);
  const colors = [
    'bg-purple-800/40 text-purple-300',
    'bg-blue-800/40 text-blue-300',
    'bg-green-800/40 text-green-300',
    'bg-orange-800/40 text-orange-300',
    'bg-pink-800/40 text-pink-300',
    'bg-cyan-800/40 text-cyan-300',
    'bg-yellow-800/40 text-yellow-300',
    'bg-indigo-800/40 text-indigo-300',
  ];
  return colors[Math.abs(hash) % colors.length];
}

function downloadJSON(data, filename) {
  downloadFile(JSON.stringify(data, null, 2), filename, 'application/json');
}

function downloadFile(content, filename, mimeType) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function showToast(message) {
  const toast = document.createElement('div');
  toast.className = 'fixed bottom-24 sm:bottom-6 left-1/2 -translate-x-1/2 z-50 bg-surface-800 border border-slate-700 text-slate-200 text-sm px-4 py-2 rounded-xl shadow-2xl fade-in';
  toast.textContent = message;
  document.body.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = '0';
    toast.style.transition = 'opacity 0.3s';
    setTimeout(() => toast.remove(), 300);
  }, 2500);
}

// ---------------------------------------------------------------------------
// 17. Citation Tracker
// ---------------------------------------------------------------------------

async function fetchCitationForPaper(paper) {
  const pmid = paper.pmid;
  if (!pmid || pmid.startsWith('demo')) return null;

  const cached = await store.getCitation(pmid);
  if (cached && cached.lastFetched && (Date.now() - cached.lastFetched < 24 * 60 * 60 * 1000)) {
    return cached;
  }

  try {
    let url = `https://api.openalex.org/works/pmid:${pmid}`;
    let resp = await fetch(url);
    if (!resp.ok && paper.doi) {
      url = `https://api.openalex.org/works/doi:${paper.doi}`;
      resp = await fetch(url);
    }
    if (!resp.ok) return cached || null;

    const data = await resp.json();
    const currentCount = data.cited_by_count || 0;

    const history = cached ? (cached.history || []) : [];
    const today = new Date().toISOString().slice(0, 10);
    if (history.length === 0 || history[history.length - 1].date !== today) {
      history.push({ date: today, count: currentCount });
    } else {
      history[history.length - 1].count = currentCount;
    }
    while (history.length > 12) history.shift();

    const record = { pmid, currentCount, history, lastFetched: Date.now() };
    await store.putCitation(record);
    return record;
  } catch (e) {
    console.warn('Citation fetch failed for', pmid, e);
    if (!cached && e.message && e.message.includes('fetch')) {
      showToast('Citation data temporarily unavailable');
    }
    return cached || null;
  }
}

async function fetchCitationsForReadingList() {
  let papers = await store.getSavedPapers();
  citationCache = {};
  for (const paper of papers) {
    const cite = await fetchCitationForPaper(paper);
    if (cite) citationCache[paper.pmid] = cite;
    await sleep(200);
  }
}

function citationSparkline(history) {
  if (!history || history.length < 2) return '';
  const bars = '\u2581\u2582\u2583\u2585';
  const counts = history.map(h => h.count);
  const max = Math.max(1, ...counts);
  return counts.map(c => bars[Math.min(bars.length - 1, Math.floor((c / max) * (bars.length - 1)))]).join('');
}

function getCitationBadgeHtml(pmid) {
  const cite = citationCache[pmid];
  if (!cite || cite.currentCount === undefined) return '';
  let html = `<span class="inline-flex items-center gap-1 ml-2 text-[11px] text-emerald-400 bg-emerald-400/10 px-1.5 py-0.5 rounded-full font-medium">\uD83D\uDCC8 ${cite.currentCount} citations`;
  if (cite.history && cite.history.length >= 2) {
    html += ` <span class="text-emerald-300/70" title="Citation history: ${escapeHtml(cite.history.map(h => h.count).join(' \u2192 '))}">${citationSparkline(cite.history)}</span>`;
  }
  html += '</span>';
  return html;
}

// ---------------------------------------------------------------------------
// 18. Journal Club Picker
// ---------------------------------------------------------------------------

async function getJournalClubCandidates() {
  const now = Date.now();
  const weekMs = 7 * 24 * 60 * 60 * 1000;
  const allPapers = await store.getAllPapers();
  return allPapers
    .filter(p => {
      const paperTime = p.savedAt || new Date(p.date).getTime() || 0;
      return paperTime > (now - weekMs) || (Date.now() - (new Date(p.date).getTime() || 0)) < weekMs;
    })
    .sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0));
}

async function renderJournalClubCard() {
  const container = document.getElementById('journal-club-container');
  if (!container) return;

  const candidates = await getJournalClubCandidates();
  if (candidates.length === 0) {
    container.innerHTML = `
      <div class="bg-surface-800 border border-slate-700/50 rounded-xl p-4 text-center">
        <p class="text-sm text-slate-400">No papers from the last 7 days to pick from.</p>
      </div>`;
    return;
  }

  if (journalClubIndex >= candidates.length) journalClubIndex = 0;
  const paper = candidates[journalClubIndex];
  const score = paper.signalScore || 0;
  const matchedEnts = (paper.matchedEntities || []).join(', ');
  const abstractPreview = (paper.abstract || '').slice(0, 200) + ((paper.abstract || '').length > 200 ? '...' : '');

  container.innerHTML = `
    <div class="bg-gradient-to-br from-purple-900/30 to-saffron-400/10 border border-saffron-400/30 rounded-xl p-4 mb-4">
      <div class="flex items-center justify-between mb-2">
        <div class="flex items-center gap-2">
          <span class="text-lg">\uD83C\uDFC6</span>
          <span class="text-sm font-semibold text-saffron-400">Journal Club Pick</span>
        </div>
        <span class="text-[10px] text-slate-500">Pick ${journalClubIndex + 1} of ${candidates.length}</span>
      </div>
      <h3 class="text-sm font-semibold text-white leading-snug mb-1">${escapeHtml(paper.title)}</h3>
      <div class="flex flex-wrap items-center gap-2 text-xs text-slate-400 mb-2">
        <span class="font-medium text-slate-300">${escapeHtml(paper.journal || 'Preprint')}</span>
        <span>\u2B50 Signal: ${score}</span>
      </div>
      ${matchedEnts ? `<p class="text-xs text-purple-300 mb-2">Top scored for: ${escapeHtml(matchedEnts)}</p>` : ''}
      ${abstractPreview ? `<p class="text-xs text-slate-500 leading-relaxed mb-3">${escapeHtml(abstractPreview)}</p>` : ''}
      <div class="flex flex-wrap gap-2">
        <button id="jc-copy-slack" class="px-3 py-1 bg-saffron-400 text-surface-900 rounded-lg text-xs font-semibold hover:bg-saffron-300 transition">Copy for Slack</button>
        <button id="jc-pick-another" class="px-3 py-1 bg-surface-700 text-slate-300 rounded-lg text-xs hover:bg-surface-600 transition">Pick Another</button>
        <a href="https://pubmed.ncbi.nlm.nih.gov/${paper.pmid}" target="_blank" rel="noopener" class="px-3 py-1 bg-surface-700 text-slate-300 rounded-lg text-xs hover:bg-surface-600 transition">Open</a>
      </div>
    </div>`;

  document.getElementById('jc-copy-slack').addEventListener('click', () => {
    const text = `\uD83D\uDCF0 Journal Club Pick: ${paper.title} \u2014 ${paper.journal || 'Unknown'} (Signal: ${score}) https://pubmed.ncbi.nlm.nih.gov/${paper.pmid}`;
    navigator.clipboard.writeText(text).then(() => showToast('Copied for Slack'));
  });

  document.getElementById('jc-pick-another').addEventListener('click', async () => {
    journalClubIndex++;
    await renderJournalClubCard();
  });
}

// ---------------------------------------------------------------------------
// 19. Collaboration Finder
// ---------------------------------------------------------------------------

async function renderCollaborators() {
  const container = document.getElementById('collaborators-container');
  if (!container) return;

  const allPapers = await store.getAllPapers();
  const entities = await store.getWatchlist();
  const entityNames = new Set(entities.map(e => e.term.toLowerCase()));

  const matchedPapers = allPapers.filter(p => {
    if ((p.matchedEntities || []).some(e => entityNames.has(e.toLowerCase()))) return true;
    const title = (p.title || '').toLowerCase();
    return [...entityNames].some(e => title.includes(e));
  });

  if (matchedPapers.length === 0 || !matchedPapers.some(p => p.authors && p.authors.length > 0)) {
    container.innerHTML = `<p class="text-sm text-slate-500 text-center py-6">No author data available yet. Papers need author information.</p>`;
    return;
  }

  const authorMap = {};
  matchedPapers.forEach(p => {
    (p.authors || []).forEach(author => {
      const key = author.toLowerCase().trim();
      if (!key) return;
      if (!authorMap[key]) authorMap[key] = { name: author, count: 0, papers: [] };
      authorMap[key].count++;
      authorMap[key].papers.push(p);
    });
  });

  const topAuthors = Object.values(authorMap)
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);

  let html = '';
  topAuthors.forEach((author, i) => {
    const recentPaper = author.papers.sort((a, b) => new Date(b.date) - new Date(a.date))[0];
    const recentTitle = recentPaper ? escapeHtml((recentPaper.title || '').slice(0, 60) + (recentPaper.title.length > 60 ? '...' : '')) : '';
    const isWatched = entities.some(e => e.term.toLowerCase() === author.name.toLowerCase());

    html += `
      <div class="flex items-center gap-3 py-2 px-3 rounded-lg hover:bg-surface-700/50 transition">
        <span class="text-xs font-bold text-slate-600 w-5 text-right">${i + 1}</span>
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium text-slate-200">${escapeHtml(author.name)}</div>
          <div class="text-xs text-slate-500 truncate" title="${escapeHtml(recentPaper?.title || '')}">${recentTitle}</div>
        </div>
        <span class="text-xs text-slate-400 flex-shrink-0">${author.count} paper${author.count !== 1 ? 's' : ''}</span>
        <div class="flex gap-1 flex-shrink-0">
          <button class="collab-watch-btn px-2 py-0.5 text-[10px] rounded ${isWatched ? 'bg-saffron-400/20 text-saffron-400' : 'bg-surface-700 text-slate-400 hover:text-saffron-400'} transition" data-author="${escapeHtml(author.name)}" ${isWatched ? 'disabled' : ''}>${isWatched ? 'Watching' : 'Watch'}</button>
          <a href="https://pubmed.ncbi.nlm.nih.gov/?term=${encodeURIComponent(author.name + '[author]')}" target="_blank" rel="noopener" class="px-2 py-0.5 text-[10px] bg-surface-700 text-slate-400 rounded hover:text-blue-400 transition">PubMed</a>
        </div>
      </div>`;
  });

  container.innerHTML = html;

  container.querySelectorAll('.collab-watch-btn:not([disabled])').forEach(btn => {
    btn.addEventListener('click', async () => {
      const authorName = btn.dataset.author;
      const existing = await store.getWatchlistByTerm(authorName);
      if (!existing) {
        await store.addWatchlistEntity({
          term: authorName,
          type: 'author',
          tags: ['collaborator']
        });
        btn.textContent = 'Watching';
        btn.classList.add('bg-saffron-400/20', 'text-saffron-400');
        btn.disabled = true;
        showToast(`Watching ${authorName}`);
      } else {
        showToast(`Already watching ${authorName}`);
      }
    });
  });
}

// ---------------------------------------------------------------------------
// 20. Lab Meeting Prep / Weekly Digest
// ---------------------------------------------------------------------------

async function generateDigestContent() {
  const now = Date.now();
  const weekMs = 7 * 24 * 60 * 60 * 1000;
  const allPapers = await store.getAllPapers();
  return allPapers.filter(p => {
    const paperTime = new Date(p.date).getTime() || 0;
    return paperTime > (now - weekMs);
  });
}

async function generateDigestMarkdown() {
  const weekPapers = await generateDigestContent();
  if (weekPapers.length === 0) return null;

  const entities = await store.getWatchlist();
  const byEntity = {};
  weekPapers.forEach(p => {
    const ents = p.matchedEntities || ['Other'];
    ents.forEach(ent => {
      if (!byEntity[ent]) byEntity[ent] = [];
      byEntity[ent].push(p);
    });
  });

  let md = '# BioKhoj Weekly Digest\n';
  md += '_' + new Date().toLocaleDateString() + '_\n\n';
  md += '**' + weekPapers.length + ' papers** across **' + Object.keys(byEntity).length + ' entities**\n\n';

  const highPriority = weekPapers.filter(p => (p.signalScore || 0) >= 70);
  if (highPriority.length > 0) {
    md += '## High-Signal Papers\n\n';
    highPriority.forEach(p => {
      md += '- **' + p.title + '** (' + (p.journal || 'Preprint') + ', Signal: ' + (p.signalScore || 0) + ')\n';
    });
    md += '\n';
  }

  Object.entries(byEntity).sort((a, b) => b[1].length - a[1].length).forEach(([entity, entityPapers]) => {
    md += '## ' + entity + ' (' + entityPapers.length + ' papers)\n\n';
    entityPapers.slice(0, 5).forEach(p => {
      md += '- ' + p.title + ' \u2014 _' + (p.journal || 'Unknown') + '_ ' + (p.date || '') + '\n';
    });
    md += '\n';
  });

  return md;
}

async function generateSlidesFormat() {
  const weekPapers = await generateDigestContent();
  if (weekPapers.length === 0) return null;

  const allPapers = await store.getAllPapers();

  const now = new Date();
  const weekStart = new Date(now);
  weekStart.setDate(weekStart.getDate() - weekStart.getDay());
  const dateStr = weekStart.toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' });

  let slides = '\uD83D\uDD2C Lab Meeting \u2014 Week of ' + dateStr + '\n\n';

  const topPapers = [...weekPapers].sort((a, b) => (b.signalScore || 0) - (a.signalScore || 0)).slice(0, 5);
  slides += 'TOP PAPERS\n';
  topPapers.forEach(p => {
    const keyEntities = (p.matchedEntities || []).join(', ');
    slides += '\u2022 ' + p.title + ' \u2014 ' + (p.journal || 'Unknown') + ' \u2B50 Signal: ' + (p.signalScore || 0) + '\n';
    if (keyEntities) slides += '  Key: ' + keyEntities + ' mentioned\n';
    slides += '\n';
  });

  const weekMs = 7 * 24 * 60 * 60 * 1000;
  const twoWeekMs = 14 * 24 * 60 * 60 * 1000;
  const nowTs = Date.now();

  const entityCounts = {};
  const entityCountsPrev = {};
  weekPapers.forEach(p => {
    (p.matchedEntities || ['Other']).forEach(ent => {
      entityCounts[ent] = (entityCounts[ent] || 0) + 1;
    });
  });
  allPapers.filter(p => {
    const ft = new Date(p.date).getTime() || 0;
    return ft > (nowTs - twoWeekMs) && ft <= (nowTs - weekMs);
  }).forEach(p => {
    (p.matchedEntities || ['Other']).forEach(ent => {
      entityCountsPrev[ent] = (entityCountsPrev[ent] || 0) + 1;
    });
  });

  const trending = Object.entries(entityCounts)
    .filter(([ent, cnt]) => cnt > (entityCountsPrev[ent] || 0))
    .sort((a, b) => b[1] - a[1]);

  if (trending.length > 0) {
    slides += 'TRENDING\n';
    trending.slice(0, 5).forEach(([ent, cnt]) => {
      const prev = entityCountsPrev[ent] || 0;
      slides += '\u2022 ' + ent + ': ' + cnt + ' new papers (\u2191 from ' + prev + ' last week)\n';
    });
    slides += '\n';
  }

  const coMentions = weekPapers.filter(p => (p.matchedEntities || []).length >= 2);
  if (coMentions.length > 0) {
    slides += 'NEW CO-MENTIONS\n';
    coMentions.slice(0, 3).forEach(p => {
      slides += '\u2022 Paper linking ' + (p.matchedEntities || []).join(' + ') + '\n';
    });
  }

  return slides;
}

async function openDigestModal() {
  const md = await generateDigestMarkdown();
  const modal = document.getElementById('digest-modal');
  const preview = document.getElementById('digest-preview');
  if (!md) {
    showToast('No papers from this week');
    return;
  }
  preview.textContent = md;
  modal.classList.remove('hidden');
}

function bindDigestButtons() {
  const copyBtn = document.getElementById('digest-copy-md');
  const slidesBtn = document.getElementById('digest-copy-slides');
  const closeBtn = document.getElementById('digest-close');

  if (copyBtn) copyBtn.addEventListener('click', async () => {
    const md = await generateDigestMarkdown();
    if (md) {
      await navigator.clipboard.writeText(md);
      showToast('Digest copied as Markdown');
    }
  });

  if (slidesBtn) slidesBtn.addEventListener('click', async () => {
    const slides = await generateSlidesFormat();
    if (slides) {
      await navigator.clipboard.writeText(slides);
      showToast('Slides format copied');
    } else {
      showToast('No papers from this week');
    }
  });

  if (closeBtn) closeBtn.addEventListener('click', () => {
    document.getElementById('digest-modal').classList.add('hidden');
  });
}

// ---------------------------------------------------------------------------
// 21. Feature Integration Init
// ---------------------------------------------------------------------------

function bindFeatureButtons() {
  // Insights dropdown toggle
  const insightsBtn = document.getElementById('insights-dropdown-btn');
  const insightsMenu = document.getElementById('insights-dropdown');
  if (insightsBtn && insightsMenu) {
    insightsBtn.addEventListener('click', (e) => { e.stopPropagation(); insightsMenu.classList.toggle('hidden'); });
    document.addEventListener('click', () => insightsMenu.classList.add('hidden'));
  }

  // Unread filter toggle
  const unreadFilterBtn = document.getElementById('feed-unread-filter-btn');
  if (unreadFilterBtn) unreadFilterBtn.addEventListener('click', () => {
    unreadFilterActive = !unreadFilterActive;
    unreadFilterBtn.classList.toggle('border-saffron-400/50', unreadFilterActive);
    unreadFilterBtn.classList.toggle('text-saffron-400', unreadFilterActive);
    unreadFilterBtn.classList.toggle('bg-saffron-400/10', unreadFilterActive);
    unreadFilterBtn.classList.toggle('border-slate-700', !unreadFilterActive);
    unreadFilterBtn.classList.toggle('text-slate-400', !unreadFilterActive);
    renderFeed();
  });

  // High signal filter toggle
  const highSignalBtn = document.getElementById('feed-highsignal-filter-btn');
  if (highSignalBtn) highSignalBtn.addEventListener('click', () => {
    highSignalFilterActive = !highSignalFilterActive;
    highSignalBtn.classList.toggle('border-purple-500/50', highSignalFilterActive);
    highSignalBtn.classList.toggle('text-purple-300', highSignalFilterActive);
    highSignalBtn.classList.toggle('bg-purple-500/10', highSignalFilterActive);
    highSignalBtn.classList.toggle('border-slate-700', !highSignalFilterActive);
    highSignalBtn.classList.toggle('text-slate-400', !highSignalFilterActive);
    renderFeed();
  });

  const jcBtn = document.getElementById('jc-toggle-btn');
  if (jcBtn) jcBtn.addEventListener('click', async () => {
    const container = document.getElementById('journal-club-container');
    if (container.classList.contains('hidden')) {
      container.classList.remove('hidden');
      jcBtn.textContent = '\uD83C\uDFC6 Hide Pick';
      await renderJournalClubCard();
    } else {
      container.classList.add('hidden');
      jcBtn.textContent = '\uD83C\uDFC6 Journal Club';
    }
  });

  const collabBtn = document.getElementById('collab-toggle-btn');
  if (collabBtn) collabBtn.addEventListener('click', () => {
    const modal = document.getElementById('collaborators-modal');
    modal.classList.remove('hidden');
    renderCollaborators();
  });
  const collabClose = document.getElementById('collab-close');
  if (collabClose) collabClose.addEventListener('click', () => {
    document.getElementById('collaborators-modal').classList.add('hidden');
  });

  const digestBtn = document.getElementById('digest-toggle-btn');
  if (digestBtn) digestBtn.addEventListener('click', () => openDigestModal());

  bindDigestButtons();

  // Co-mention filter toggle
  const comentionFilterBtn = document.getElementById('comention-filter-btn');
  if (comentionFilterBtn) comentionFilterBtn.addEventListener('click', () => {
    comentionFilterActive = !comentionFilterActive;
    if (comentionFilterActive) {
      comentionFilterBtn.classList.add('border-purple-500/50', 'text-purple-300', 'bg-purple-500/10');
      comentionFilterBtn.classList.remove('border-slate-700', 'text-slate-400');
    } else {
      comentionFilterBtn.classList.remove('border-purple-500/50', 'text-purple-300', 'bg-purple-500/10');
      comentionFilterBtn.classList.add('border-slate-700', 'text-slate-400');
    }
    renderFeed();
  });

  // Share view button
  const shareBtn = document.getElementById('share-view-btn');
  if (shareBtn) shareBtn.addEventListener('click', shareCurrentView);

  // Bulk tag button
  const bulkTagBtn = document.getElementById('bulk-tag-btn');
  if (bulkTagBtn) bulkTagBtn.addEventListener('click', bulkTagSelected);
}

// ---------------------------------------------------------------------------
// 21b. Co-mention Filter
// ---------------------------------------------------------------------------

let comentionFilterActive = false;
let unreadFilterActive = false;
let highSignalFilterActive = false;

// ---------------------------------------------------------------------------
// 21c. Share Current View
// ---------------------------------------------------------------------------

async function shareCurrentView() {
  const watchlist = await store.getWatchlist();
  if (watchlist.length === 0) {
    showToast('Nothing to share — watchlist is empty');
    return;
  }
  const payload = {
    v: 1,
    entities: watchlist.map(e => ({ t: e.term, y: e.type, p: e.priority }))
  };
  const encoded = btoa(unescape(encodeURIComponent(JSON.stringify(payload))));
  if (encoded.length > 2000) {
    showToast('Watchlist too large for URL. Use Export instead.');
    return;
  }
  const url = `${window.location.origin}/biokhoj/?import=${encoded}`;
  await navigator.clipboard.writeText(url).catch(() => {});
  showToast('Shareable link copied to clipboard');
}

// Handle incoming shared links
async function checkImportFromUrl() {
  const params = new URLSearchParams(window.location.search);
  const importData = params.get('import');
  if (!importData) return;
  try {
    const json = JSON.parse(decodeURIComponent(escape(atob(importData))));
    if (json.v === 1 && json.entities) {
      let count = 0;
      for (const e of json.entities) {
        const existing = await store.getWatchlistByTerm(e.t);
        if (!existing) {
          await store.addWatchlistEntity({ term: e.t, type: e.y || 'topic', priority: e.p || 'normal' });
          count++;
        }
      }
      if (count > 0) {
        showToast(`Imported ${count} entities from shared link`);
        await renderWatchlist();
        await renderSidebar();
      } else {
        showToast('All entities already in your watchlist');
      }
    }
    // Clean URL
    window.history.replaceState({}, '', window.location.pathname);
  } catch (err) {
    console.warn('Failed to import from URL:', err);
  }
}

// ---------------------------------------------------------------------------
// 21d. Bulk Tagging
// ---------------------------------------------------------------------------

let selectedPaperPmids = new Set();

function togglePaperSelect(pmid) {
  if (selectedPaperPmids.has(pmid)) {
    selectedPaperPmids.delete(pmid);
  } else {
    selectedPaperPmids.add(pmid);
  }
  // Update visual selection
  document.querySelectorAll('#feed-papers > [data-pmid]').forEach(el => {
    if (selectedPaperPmids.has(el.dataset.pmid)) {
      el.classList.add('ring-1', 'ring-saffron-400/50');
    } else {
      el.classList.remove('ring-1', 'ring-saffron-400/50');
    }
  });
  // Show/hide bulk tag button
  const btn = document.getElementById('bulk-tag-btn');
  if (btn) {
    if (selectedPaperPmids.size > 0) {
      btn.classList.remove('hidden');
      btn.textContent = `\uD83C\uDFF7 Tag ${selectedPaperPmids.size} selected`;
    } else {
      btn.classList.add('hidden');
    }
  }
}

async function bulkTagSelected() {
  if (selectedPaperPmids.size === 0) return;
  const tag = prompt('Enter tag for selected papers:');
  if (!tag || !tag.trim()) return;
  const trimmed = tag.trim();
  let count = 0;
  for (const pmid of selectedPaperPmids) {
    const paper = await store.getPaperByPmid(pmid);
    if (paper) {
      const tags = paper.paperTags || [];
      if (!tags.includes(trimmed)) {
        tags.push(trimmed);
        await store.updatePaper(paper.id, { paperTags: tags });
        count++;
      }
    }
  }
  selectedPaperPmids.clear();
  document.querySelectorAll('#feed-papers > [data-pmid]').forEach(el => {
    el.classList.remove('ring-1', 'ring-saffron-400/50');
  });
  const btn = document.getElementById('bulk-tag-btn');
  if (btn) btn.classList.add('hidden');
  showToast(`Tagged ${count} papers with "${trimmed}"`);
}

// ---------------------------------------------------------------------------
// 21e. Preset Entity Packs
// ---------------------------------------------------------------------------

const PRESET_PACKS = [
  {
    name: 'Cancer Genomics',
    icon: '\uD83E\uDDEC',
    entities: [
      { term: 'TP53', type: 'gene' }, { term: 'BRCA1', type: 'gene' }, { term: 'BRCA2', type: 'gene' },
      { term: 'KRAS', type: 'gene' }, { term: 'EGFR', type: 'gene' }, { term: 'PIK3CA', type: 'gene' },
      { term: 'tumor mutational burden', type: 'topic' }, { term: 'immune checkpoint', type: 'topic' }
    ]
  },
  {
    name: 'CRISPR & Gene Editing',
    icon: '\u2702\uFE0F',
    entities: [
      { term: 'CRISPR-Cas9', type: 'technique' }, { term: 'base editing', type: 'technique' },
      { term: 'prime editing', type: 'technique' }, { term: 'guide RNA', type: 'topic' },
      { term: 'off-target effects', type: 'topic' }, { term: 'gene therapy', type: 'topic' }
    ]
  },
  {
    name: 'Single-Cell Genomics',
    icon: '\uD83D\uDD2C',
    entities: [
      { term: 'single-cell RNA-seq', type: 'technique' }, { term: 'spatial transcriptomics', type: 'technique' },
      { term: 'cell atlas', type: 'topic' }, { term: 'UMAP', type: 'technique' },
      { term: 'trajectory inference', type: 'topic' }, { term: 'multiome', type: 'technique' }
    ]
  },
  {
    name: 'Clinical Variants',
    icon: '\uD83C\uDFE5',
    entities: [
      { term: 'pathogenic variant', type: 'topic' }, { term: 'variant of uncertain significance', type: 'topic' },
      { term: 'pharmacogenomics', type: 'topic' }, { term: 'ACMG classification', type: 'topic' },
      { term: 'ClinVar', type: 'topic' }, { term: 'rare disease', type: 'disease' }
    ]
  },
  {
    name: 'Metagenomics',
    icon: '\uD83E\uDDA0',
    entities: [
      { term: '16S rRNA', type: 'technique' }, { term: 'microbiome', type: 'topic' },
      { term: 'shotgun metagenomics', type: 'technique' }, { term: 'antimicrobial resistance', type: 'topic' },
      { term: 'gut-brain axis', type: 'topic' }, { term: 'metatranscriptomics', type: 'technique' }
    ]
  }
];

async function renderPresetPacks() {
  const list = document.getElementById('preset-packs-list');
  if (!list) return;

  // Hide if user already has entities
  const count = await store.getWatchlistCount();
  if (count > 5) {
    document.getElementById('preset-packs').classList.add('hidden');
    return;
  }
  document.getElementById('preset-packs').classList.remove('hidden');

  list.innerHTML = PRESET_PACKS.map(pack => `
    <button class="preset-pack-btn px-3 py-2 bg-surface-800 border border-slate-700 rounded-lg text-xs text-slate-300 hover:border-saffron-400/50 hover:text-saffron-400 transition flex items-center gap-2" data-pack="${escapeHtml(pack.name)}">
      <span>${pack.icon}</span>
      <span>${escapeHtml(pack.name)}</span>
      <span class="text-slate-600">(${pack.entities.length})</span>
    </button>
  `).join('');

  list.querySelectorAll('.preset-pack-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const packName = btn.dataset.pack;
      const pack = PRESET_PACKS.find(p => p.name === packName);
      if (!pack) return;
      let added = 0;
      for (const e of pack.entities) {
        const existing = await store.getWatchlistByTerm(e.term);
        if (!existing) {
          await store.addWatchlistEntity({ term: e.term, type: e.type, priority: 'normal', tags: [packName.toLowerCase()] });
          added++;
        }
      }
      btn.classList.add('opacity-40', 'pointer-events-none');
      btn.innerHTML = `<span>\u2713</span> <span>${escapeHtml(pack.name)}</span>`;
      showToast(`Added ${added} entities from ${pack.name}`);
      await store.saveSetting('onboarded', true);
      await renderWatchlist();
      await renderSidebar();
    });
  });
}

// ---------------------------------------------------------------------------
// 21f. Citation Velocity Alerts
// ---------------------------------------------------------------------------

async function checkCitationAlerts() {
  const watchlist = await store.getWatchlist();
  const papers = await store.getSavedPapers();
  const allCitations = await store.getAllCitations();

  for (const cite of allCitations) {
    if (!cite.history || cite.history.length < 2) continue;

    const recent = cite.history[cite.history.length - 1].count;
    const prev = cite.history[cite.history.length - 2].count;
    const velocity = recent - prev;

    // Alert if velocity > 5 citations since last check (spike detection)
    if (velocity >= 5) {
      const paper = papers.find(p => p.pmid === cite.pmid);
      if (paper) {
        showToast(`Citation spike: "${paper.title.slice(0, 50)}..." gained ${velocity} citations`);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// 22. Onboarding (first-time user)
// ---------------------------------------------------------------------------

function showOnboardingCard() {
  const container = document.getElementById('onboarding-card');
  if (!container) return;

  const quickAddItems = [
    { term: 'BRCA1', type: 'gene' },
    { term: 'TP53', type: 'gene' },
    { term: 'olaparib', type: 'drug' },
    { term: 'rs1801133', type: 'variant' },
    { term: 'CRISPR', type: 'technique' },
  ];

  container.classList.remove('hidden');
  container.innerHTML = `
    <div class="onboarding-card">
      <div class="flex items-start gap-3 mb-3">
        <span class="text-2xl flex-shrink-0">&#128075;</span>
        <div>
          <h3 class="text-base font-semibold text-white mb-1">Welcome to BioKhoj!</h3>
          <p class="text-sm text-slate-400">Try watching a gene: switch to the <button class="text-saffron-400 hover:underline font-medium" onclick="switchTab('watchlist')">Watchlist tab</button> and add <strong class="text-white">BRCA1</strong>, or quick-add from the chips below.</p>
        </div>
      </div>
      <div class="flex flex-wrap gap-2" id="onboarding-chips">
        ${quickAddItems.map(item => `<button class="onboarding-chip" data-term="${escapeHtml(item.term)}" data-type="${escapeHtml(item.type)}">+ ${escapeHtml(item.term)}</button>`).join('')}
      </div>
    </div>`;

  container.querySelectorAll('.onboarding-chip').forEach(chip => {
    chip.addEventListener('click', async () => {
      const term = chip.dataset.term;
      const type = chip.dataset.type;
      const existing = await store.getWatchlistByTerm(term);
      if (!existing) {
        await store.addWatchlistEntity({ term, type });
      }
      chip.style.opacity = '0.4';
      chip.style.pointerEvents = 'none';
      chip.textContent = '\u2713 ' + term;
      showToast(`Watching ${term}`);
      await store.saveSetting('onboarded', true);
      await renderSidebar();
      setTimeout(() => dismissOnboarding(), 800);
    });
  });
}

function dismissOnboarding() {
  const container = document.getElementById('onboarding-card');
  if (container) {
    container.style.opacity = '0';
    container.style.transition = 'opacity 0.3s';
    setTimeout(() => {
      container.classList.add('hidden');
      container.style.opacity = '';
    }, 300);
  }
}

// ---------------------------------------------------------------------------
// 23. Discover Tab
// ---------------------------------------------------------------------------

function bindDiscoverTab() {
  const searchBtn = document.getElementById('discover-search-btn');
  const searchInput = document.getElementById('discover-search-input');
  const refreshBtn = document.getElementById('discover-refresh-trending');

  if (searchBtn) {
    searchBtn.addEventListener('click', () => {
      const query = searchInput.value.trim();
      if (query) unifiedDatabaseSearch(query);
    });
  }
  if (searchInput) {
    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        const query = searchInput.value.trim();
        if (query) unifiedDatabaseSearch(query);
      }
    });
  }
  if (refreshBtn) {
    refreshBtn.addEventListener('click', () => fetchBioRxivTrending(true));
  }
}

async function renderDiscover() {
  await fetchBioRxivTrending(false);
}

async function fetchBioRxivTrending(forceRefresh) {
  const container = document.getElementById('discover-trending');
  const loading = document.getElementById('discover-trending-loading');
  const empty = document.getElementById('discover-trending-empty');

  if (!forceRefresh) {
    const cached = await store.getDiscoverCache('biorxiv_trending');
    if (cached && cached.fetchedAt && (Date.now() - cached.fetchedAt < 6 * 60 * 60 * 1000)) {
      renderTrendingCards(cached.data);
      return;
    }
  }

  container.innerHTML = '';
  loading.classList.remove('hidden');
  empty.classList.add('hidden');

  const refreshBtn = document.getElementById('discover-refresh-trending');
  if (refreshBtn) refreshBtn.querySelector('svg')?.classList.add('animate-spin');

  try {
    const endDate = new Date().toISOString().slice(0, 10);
    const startDate = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().slice(0, 10);
    const resp = await fetch(`https://api.biorxiv.org/details/biorxiv/${startDate}/${endDate}/0/10`);
    if (!resp.ok) throw new Error(`bioRxiv API ${resp.status}`);
    const data = await resp.json();
    const papers = (data.collection || []).slice(0, 10);

    await store.putDiscoverCache('biorxiv_trending', papers);

    loading.classList.add('hidden');
    renderTrendingCards(papers);
  } catch (err) {
    console.warn('bioRxiv trending fetch failed:', err);
    loading.classList.add('hidden');
    empty.classList.remove('hidden');
  }

  if (refreshBtn) refreshBtn.querySelector('svg')?.classList.remove('animate-spin');
}

function renderTrendingCards(papers) {
  const container = document.getElementById('discover-trending');
  const empty = document.getElementById('discover-trending-empty');
  const countBadge = document.getElementById('discover-trending-count');

  if (!papers || papers.length === 0) {
    container.innerHTML = '';
    empty.classList.remove('hidden');
    if (countBadge) countBadge.textContent = '0';
    return;
  }

  empty.classList.add('hidden');
  if (countBadge) countBadge.textContent = papers.length;
  container.innerHTML = papers.map((p) => {
    const title = escapeHtml(p.title || 'Untitled');
    const category = escapeHtml(p.category || 'biology');
    const date = escapeHtml(p.date || '');
    const authors = escapeHtml(
      (p.authors || '').length > 80 ? (p.authors || '').slice(0, 80) + '...' : (p.authors || '')
    );
    const doi = p.doi || '';
    const url = doi ? `https://doi.org/${doi}` : '#';

    const geneMatches = (p.title || '').match(/\b[A-Z][A-Z0-9]{1,5}\b/g) || [];
    const genes = [...new Set(geneMatches.filter(g => !/^(THE|AND|FOR|NOT|BUT|WITH|FROM|THAT|THIS|HAVE|BEEN|WERE|CELL|RNA|DNA|PCR)$/.test(g)))];

    return `
      <div class="bg-surface-800 border border-slate-700/50 rounded-xl p-4 hover:border-slate-600 transition fade-in">
        <div class="flex items-start justify-between gap-2 mb-2">
          <span class="chip bg-emerald-800/40 text-emerald-300 flex-shrink-0">${category}</span>
          <span class="text-[11px] text-slate-500 flex-shrink-0">${date}</span>
        </div>
        <a href="${url}" target="_blank" rel="noopener" class="text-sm font-semibold text-white hover:text-saffron-300 transition leading-snug block mb-1.5">${title}</a>
        <p class="text-xs text-slate-500 mb-2">${authors}</p>
        <div class="flex items-center justify-between gap-2">
          <div class="flex flex-wrap gap-1">
            ${genes.slice(0, 3).map(g => `<span class="chip bg-blue-800/40 text-blue-300">${g}</span>`).join('')}
          </div>
          <div class="flex items-center gap-1 flex-shrink-0">
            ${genes.length > 0 ? `<button class="discover-watch-genes text-[11px] text-saffron-400 hover:text-saffron-300 transition px-2 py-1 rounded hover:bg-saffron-400/10" data-genes="${escapeHtml(genes.join(','))}" title="Watch extracted genes">+ Watch</button>` : ''}
            <a href="${url}" target="_blank" rel="noopener" class="text-[11px] text-slate-400 hover:text-white transition px-2 py-1 rounded hover:bg-surface-700">Open &#8599;</a>
          </div>
        </div>
      </div>`;
  }).join('');

  container.querySelectorAll('.discover-watch-genes').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const genes = btn.dataset.genes.split(',').filter(Boolean);
      let added = 0;
      for (const gene of genes) {
        const existing = await store.getWatchlistByTerm(gene);
        if (!existing) {
          await store.addWatchlistEntity({
            term: gene, type: 'gene', tags: ['discovered']
          });
          added++;
        }
      }
      if (added > 0) {
        showToast(`Added ${added} gene${added > 1 ? 's' : ''} to watchlist`);
        await renderSidebar();
      } else {
        showToast('Genes already in watchlist');
      }
    });
  });
}

async function unifiedDatabaseSearch(query) {
  const resultsContainer = document.getElementById('discover-results');
  const loadingEl = document.getElementById('discover-search-loading');
  const noResults = document.getElementById('discover-no-results');

  resultsContainer.innerHTML = '';
  noResults.classList.add('hidden');
  loadingEl.classList.remove('hidden');

  const encoded = encodeURIComponent(query);

  const [pubmedRes, geneRes, clinvarRes, trialsRes, uniprotRes] = await Promise.allSettled([
    fetchPubMedDiscover(encoded),
    fetchNCBIGeneDiscover(encoded),
    fetchClinVarDiscover(encoded),
    fetchClinicalTrialsDiscover(encoded),
    fetchUniProtDiscover(encoded)
  ]);

  loadingEl.classList.add('hidden');

  const results = {
    pubmed: pubmedRes.status === 'fulfilled' ? pubmedRes.value : [],
    gene: geneRes.status === 'fulfilled' ? geneRes.value : [],
    clinvar: clinvarRes.status === 'fulfilled' ? clinvarRes.value : [],
    trials: trialsRes.status === 'fulfilled' ? trialsRes.value : [],
    uniprot: uniprotRes.status === 'fulfilled' ? uniprotRes.value : [],
  };

  const totalResults = Object.values(results).reduce((sum, arr) => sum + arr.length, 0);
  if (totalResults === 0) {
    noResults.classList.remove('hidden');
    return;
  }

  renderDiscoverResults(results, query);
}

async function fetchPubMedDiscover(query) {
  const searchResp = await fetch(`https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&retmode=json&retmax=5&term=${query}`);
  if (!searchResp.ok) return [];
  const searchData = await searchResp.json();
  const ids = searchData.esearchresult?.idlist || [];
  if (ids.length === 0) return [];

  await sleep(350);
  const summResp = await fetch(`https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&retmode=json&id=${ids.join(',')}`);
  if (!summResp.ok) return [];
  const summData = await summResp.json();
  const result = summData.result || {};

  return ids.map(id => {
    const r = result[id];
    if (!r) return null;
    return {
      id, title: r.title || '', authors: (r.authors || []).map(a => a.name).join(', '),
      journal: r.fulljournalname || r.source || '', date: r.pubdate || '',
      url: `https://pubmed.ncbi.nlm.nih.gov/${id}/`
    };
  }).filter(Boolean);
}

async function fetchNCBIGeneDiscover(query) {
  const searchResp = await fetch(`https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=gene&retmode=json&retmax=3&term=${query}[gene]+AND+human[orgn]`);
  if (!searchResp.ok) return [];
  const searchData = await searchResp.json();
  const ids = searchData.esearchresult?.idlist || [];
  if (ids.length === 0) return [];

  await sleep(350);
  const summResp = await fetch(`https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=gene&retmode=json&id=${ids.join(',')}`);
  if (!summResp.ok) return [];
  const summData = await summResp.json();
  const result = summData.result || {};

  return ids.map(id => {
    const r = result[id];
    if (!r) return null;
    return {
      id, symbol: r.name || '', fullName: r.description || '',
      chromosome: r.chromosome || '', url: `https://www.ncbi.nlm.nih.gov/gene/${id}`
    };
  }).filter(Boolean);
}

async function fetchClinVarDiscover(query) {
  await sleep(350);
  const searchResp = await fetch(`https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=clinvar&retmode=json&retmax=3&term=${query}`);
  if (!searchResp.ok) return [];
  const searchData = await searchResp.json();
  const ids = searchData.esearchresult?.idlist || [];
  if (ids.length === 0) return [];

  await sleep(350);
  const summResp = await fetch(`https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=clinvar&retmode=json&id=${ids.join(',')}`);
  if (!summResp.ok) return [];
  const summData = await summResp.json();
  const result = summData.result || {};

  return ids.map(id => {
    const r = result[id];
    if (!r) return null;
    return {
      id, title: r.title || r.obj_type || '',
      significance: r.clinical_significance?.description || r.germline_classification?.description || '',
      url: `https://www.ncbi.nlm.nih.gov/clinvar/variation/${id}/`
    };
  }).filter(Boolean);
}

async function fetchClinicalTrialsDiscover(query) {
  try {
    const resp = await fetch(`https://clinicaltrials.gov/api/v2/studies?query.term=${query}&pageSize=3`);
    if (!resp.ok) return [];
    const data = await resp.json();
    return (data.studies || []).map(s => {
      const proto = s.protocolSection || {};
      const ident = proto.identificationModule || {};
      const status = proto.statusModule || {};
      const design = proto.designModule || {};
      return {
        id: ident.nctId || '',
        title: ident.briefTitle || ident.officialTitle || '',
        status: status.overallStatus || '',
        phase: (design.phases || []).join(', ') || 'N/A',
        url: `https://clinicaltrials.gov/study/${ident.nctId || ''}`
      };
    });
  } catch (e) {
    console.warn('ClinicalTrials.gov fetch failed:', e);
    return [];
  }
}

async function fetchUniProtDiscover(query) {
  try {
    const resp = await fetch(`https://rest.uniprot.org/uniprotkb/search?query=${query}&size=3&format=json`);
    if (!resp.ok) return [];
    const data = await resp.json();
    return (data.results || []).map(r => {
      const proteinName = r.proteinDescription?.recommendedName?.fullName?.value
        || r.proteinDescription?.submissionNames?.[0]?.fullName?.value || '';
      return {
        id: r.primaryAccession || '',
        entryName: r.uniProtkbId || '',
        proteinName,
        organism: r.organism?.scientificName || '',
        url: `https://www.uniprot.org/uniprotkb/${r.primaryAccession || ''}/entry`
      };
    });
  } catch (e) {
    console.warn('UniProt fetch failed:', e);
    return [];
  }
}

function renderDiscoverResults(results, query) {
  const container = document.getElementById('discover-results');

  const sections = [];

  if (results.pubmed.length > 0) {
    sections.push(renderDiscoverGroup(
      'PubMed', results.pubmed.length, 'bg-blue-600', results.pubmed.map(r => `
        <div class="flex items-start justify-between gap-3 py-2.5 border-b border-slate-700/30 last:border-0">
          <div class="min-w-0 flex-1">
            <a href="${r.url}" target="_blank" rel="noopener" class="text-sm text-white hover:text-blue-300 transition font-medium leading-snug block">${escapeHtml(r.title)}</a>
            <p class="text-xs text-slate-500 mt-0.5">${escapeHtml(r.authors.length > 80 ? r.authors.slice(0, 80) + '...' : r.authors)}</p>
            <div class="flex items-center gap-2 mt-1 text-xs text-slate-400">
              <span class="font-medium text-slate-300">${escapeHtml(r.journal)}</span>
              <span class="text-slate-600">|</span>
              <span>${escapeHtml(r.date)}</span>
            </div>
          </div>
          <div class="flex items-center gap-1 flex-shrink-0">
            <button class="discover-watch-term text-[11px] text-saffron-400 hover:text-saffron-300 px-2 py-1 rounded hover:bg-saffron-400/10 transition" data-term="${escapeHtml(query)}" data-type="topic">Watch</button>
            <a href="${r.url}" target="_blank" rel="noopener" class="text-[11px] text-slate-400 hover:text-white px-2 py-1 rounded hover:bg-surface-700 transition">Open &#8599;</a>
          </div>
        </div>
      `).join('')
    ));
  }

  if (results.gene.length > 0) {
    sections.push(renderDiscoverGroup(
      'NCBI Gene', results.gene.length, 'bg-green-600', results.gene.map(r => `
        <div class="flex items-start justify-between gap-3 py-2.5 border-b border-slate-700/30 last:border-0">
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="text-sm font-bold text-green-300">${escapeHtml(r.symbol)}</span>
              <span class="text-xs text-slate-400">${escapeHtml(r.fullName)}</span>
            </div>
            <p class="text-xs text-slate-500 mt-0.5">Chromosome ${escapeHtml(r.chromosome)}</p>
          </div>
          <div class="flex items-center gap-1 flex-shrink-0">
            <button class="discover-watch-term text-[11px] text-saffron-400 hover:text-saffron-300 px-2 py-1 rounded hover:bg-saffron-400/10 transition" data-term="${escapeHtml(r.symbol)}" data-type="gene">Watch</button>
            <a href="${r.url}" target="_blank" rel="noopener" class="text-[11px] text-slate-400 hover:text-white px-2 py-1 rounded hover:bg-surface-700 transition">Open &#8599;</a>
          </div>
        </div>
      `).join('')
    ));
  }

  if (results.clinvar.length > 0) {
    sections.push(renderDiscoverGroup(
      'ClinVar', results.clinvar.length, 'bg-purple-600', results.clinvar.map(r => `
        <div class="flex items-start justify-between gap-3 py-2.5 border-b border-slate-700/30 last:border-0">
          <div class="min-w-0 flex-1">
            <span class="text-sm text-white font-medium">${escapeHtml(r.title)}</span>
            ${r.significance ? `<p class="text-xs mt-0.5"><span class="chip bg-purple-800/40 text-purple-300">${escapeHtml(r.significance)}</span></p>` : ''}
          </div>
          <div class="flex items-center gap-1 flex-shrink-0">
            <button class="discover-watch-term text-[11px] text-saffron-400 hover:text-saffron-300 px-2 py-1 rounded hover:bg-saffron-400/10 transition" data-term="${escapeHtml(r.title)}" data-type="topic">Watch</button>
            <a href="${r.url}" target="_blank" rel="noopener" class="text-[11px] text-slate-400 hover:text-white px-2 py-1 rounded hover:bg-surface-700 transition">Open &#8599;</a>
          </div>
        </div>
      `).join('')
    ));
  }

  if (results.trials.length > 0) {
    sections.push(renderDiscoverGroup(
      'ClinicalTrials.gov', results.trials.length, 'bg-orange-600', results.trials.map(r => {
        const statusColors = {
          'RECRUITING': 'bg-green-800/40 text-green-300',
          'ACTIVE_NOT_RECRUITING': 'bg-yellow-800/40 text-yellow-300',
          'COMPLETED': 'bg-slate-700/60 text-slate-400',
          'NOT_YET_RECRUITING': 'bg-blue-800/40 text-blue-300',
        };
        const statusClass = statusColors[r.status] || 'bg-slate-700/60 text-slate-400';
        const statusLabel = (r.status || '').replace(/_/g, ' ').toLowerCase().replace(/\b\w/g, c => c.toUpperCase());
        return `
        <div class="flex items-start justify-between gap-3 py-2.5 border-b border-slate-700/30 last:border-0">
          <div class="min-w-0 flex-1">
            <span class="text-sm text-white font-medium leading-snug block">${escapeHtml(r.title)}</span>
            <div class="flex items-center gap-2 mt-1">
              <span class="chip ${statusClass}">${escapeHtml(statusLabel)}</span>
              <span class="text-xs text-slate-500">Phase: ${escapeHtml(r.phase)}</span>
            </div>
          </div>
          <div class="flex items-center gap-1 flex-shrink-0">
            <button class="discover-watch-term text-[11px] text-saffron-400 hover:text-saffron-300 px-2 py-1 rounded hover:bg-saffron-400/10 transition" data-term="${escapeHtml(query)}" data-type="topic">Watch</button>
            <a href="${r.url}" target="_blank" rel="noopener" class="text-[11px] text-slate-400 hover:text-white px-2 py-1 rounded hover:bg-surface-700 transition">Open &#8599;</a>
          </div>
        </div>`;
      }).join('')
    ));
  }

  if (results.uniprot.length > 0) {
    sections.push(renderDiscoverGroup(
      'UniProt', results.uniprot.length, 'bg-cyan-600', results.uniprot.map(r => `
        <div class="flex items-start justify-between gap-3 py-2.5 border-b border-slate-700/30 last:border-0">
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span class="text-sm font-bold text-cyan-300">${escapeHtml(r.entryName)}</span>
              <span class="text-xs text-slate-400">${escapeHtml(r.proteinName)}</span>
            </div>
            <p class="text-xs text-slate-500 mt-0.5 italic">${escapeHtml(r.organism)}</p>
          </div>
          <div class="flex items-center gap-1 flex-shrink-0">
            <button class="discover-watch-term text-[11px] text-saffron-400 hover:text-saffron-300 px-2 py-1 rounded hover:bg-saffron-400/10 transition" data-term="${escapeHtml(r.entryName.split('_')[0] || r.entryName)}" data-type="protein">Watch</button>
            <a href="${r.url}" target="_blank" rel="noopener" class="text-[11px] text-slate-400 hover:text-white px-2 py-1 rounded hover:bg-surface-700 transition">Open &#8599;</a>
          </div>
        </div>
      `).join('')
    ));
  }

  container.innerHTML = sections.join('');
  bindDiscoverWatchButtons();
}

function renderDiscoverGroup(sourceName, count, badgeColor, innerHtml) {
  return `
    <div class="bg-surface-800 border border-slate-700/50 rounded-xl overflow-hidden fade-in">
      <button class="w-full flex items-center justify-between px-4 py-3 hover:bg-surface-700/50 transition" onclick="this.nextElementSibling.classList.toggle('hidden');this.querySelector('.chevron').classList.toggle('rotate-180')">
        <div class="flex items-center gap-2">
          <span class="w-2.5 h-2.5 rounded-full ${badgeColor} flex-shrink-0"></span>
          <span class="text-sm font-semibold text-white">${escapeHtml(sourceName)}</span>
          <span class="text-xs text-slate-500">(${count})</span>
        </div>
        <svg class="chevron w-4 h-4 text-slate-500 transition-transform" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>
      </button>
      <div class="px-4 pb-3">
        ${innerHtml}
      </div>
    </div>`;
}

function bindDiscoverWatchButtons() {
  document.querySelectorAll('.discover-watch-term').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const term = btn.dataset.term;
      const type = btn.dataset.type || 'topic';
      if (!term) return;
      const existing = await store.getWatchlistByTerm(term);
      if (existing) {
        showToast(`"${term}" already in watchlist`);
        return;
      }
      await store.addWatchlistEntity({
        term, type, tags: ['discovered']
      });
      showToast(`Added "${term}" to watchlist`);
      await renderSidebar();
    });
  });
}

// ---------------------------------------------------------------------------
// 24. Skeleton Loading States
// ---------------------------------------------------------------------------

function showFeedSkeletons() {
  const container = document.getElementById('feed-papers');
  if (!container) return;
  const emptyState = document.getElementById('feed-empty');
  if (emptyState) emptyState.classList.add('hidden');

  let html = '';
  for (let i = 0; i < 3; i++) {
    html += `
      <div class="skeleton-card fade-in">
        <div class="flex items-start gap-3">
          <div class="skeleton flex-shrink-0" style="width:40px;height:28px;border-radius:8px;"></div>
          <div class="flex-1 space-y-2">
            <div class="skeleton" style="height:14px;width:85%;"></div>
            <div class="skeleton" style="height:14px;width:60%;"></div>
            <div class="flex gap-3 mt-1">
              <div class="skeleton" style="height:10px;width:100px;"></div>
              <div class="skeleton" style="height:10px;width:80px;"></div>
              <div class="skeleton" style="height:10px;width:60px;"></div>
            </div>
            <div class="skeleton" style="height:10px;width:90%;margin-top:6px;"></div>
            <div class="skeleton" style="height:10px;width:70%;"></div>
            <div class="flex gap-2 mt-2">
              <div class="skeleton skeleton-chip"></div>
              <div class="skeleton skeleton-chip" style="width:56px;"></div>
            </div>
          </div>
        </div>
      </div>`;
  }
  container.innerHTML = html;
}

function showSidebarSkeletons() {
  const container = document.getElementById('sidebar-watchlist');
  if (!container) return;

  let html = '';
  for (let i = 0; i < 4; i++) {
    html += `
      <div class="flex items-center justify-between py-1.5 px-2">
        <div class="flex items-center gap-2">
          <div class="skeleton" style="width:6px;height:6px;border-radius:50%;"></div>
          <div class="skeleton" style="width:${60 + i * 15}px;height:12px;"></div>
        </div>
        <div class="skeleton" style="width:20px;height:12px;"></div>
      </div>`;
  }
  container.innerHTML = html;
}

// ---------------------------------------------------------------------------
// 24. Error Recovery (Warning Banners)
// ---------------------------------------------------------------------------

function showWarningBanner(type, entity, errorMsg) {
  const container = document.getElementById('feed-warning-banner');
  if (!container) return;

  container.classList.remove('hidden');

  if (type === 'rate-limit') {
    lastFailedOp = { type: 'rate-limit', entity };
    container.innerHTML = `
      <div class="warning-banner">
        <span class="text-sm text-yellow-300">&#9203; Rate limited &mdash; will retry in 30s.</span>
        <div class="flex gap-2">
          <button id="warning-dismiss-btn" class="text-xs text-slate-400 hover:text-white transition">Dismiss</button>
        </div>
      </div>`;
    const retryTimer = setTimeout(async () => {
      dismissWarningBanner();
      if (entity) await checkPapersForEntity(entity);
      await renderCurrentTab();
    }, 30000);
    container.querySelector('#warning-dismiss-btn').addEventListener('click', () => {
      clearTimeout(retryTimer);
      dismissWarningBanner();
    });
  } else {
    lastFailedOp = { type: 'error', entity };
    container.innerHTML = `
      <div class="warning-banner error-banner">
        <span class="text-sm text-yellow-300">&#9888;&#65039; PubMed check failed${errorMsg ? ': ' + escapeHtml(errorMsg) : '.'}</span>
        <div class="flex gap-2">
          <button id="warning-retry-btn" class="px-3 py-1 bg-saffron-400 text-surface-900 rounded-lg text-xs font-semibold hover:bg-saffron-300 transition">Retry</button>
          <button id="warning-dismiss-btn" class="text-xs text-slate-400 hover:text-white transition">Dismiss</button>
        </div>
      </div>`;
    container.querySelector('#warning-retry-btn').addEventListener('click', async () => {
      dismissWarningBanner();
      showFeedSkeletons();
      if (entity) {
        await checkPapersForEntity(entity);
        await renderCurrentTab();
      }
    });
    container.querySelector('#warning-dismiss-btn').addEventListener('click', () => {
      dismissWarningBanner();
    });
  }
}

function dismissWarningBanner() {
  const container = document.getElementById('feed-warning-banner');
  if (container) {
    container.classList.add('hidden');
    container.innerHTML = '';
  }
  lastFailedOp = null;
}

// ---------------------------------------------------------------------------
// 25. Keyboard Navigation
// ---------------------------------------------------------------------------

function bindKeyboardNavigation() {
  document.addEventListener('keydown', (e) => {
    const tag = document.activeElement?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') {
      if (e.key === 'Escape') {
        document.activeElement.blur();
        e.preventDefault();
      }
      return;
    }

    if (e.key === '/') {
      e.preventDefault();
      const search = document.getElementById('global-search');
      if (search) search.focus();
      return;
    }

    if (e.key === 'Escape') {
      const modals = ['rate-limit-modal', 'collaborators-modal', 'digest-modal', 'signal-popover'];
      for (const id of modals) {
        const el = document.getElementById(id);
        if (el && !el.classList.contains('hidden')) {
          el.classList.add('hidden');
          e.preventDefault();
          return;
        }
      }
      return;
    }

    if (currentTab === 'feed') {
      const papers = document.querySelectorAll('#feed-papers > [data-pmid]');
      if (papers.length === 0) return;

      if (e.key === 'ArrowDown' || e.key === 'j') {
        e.preventDefault();
        selectedPaperIndex = Math.min(selectedPaperIndex + 1, papers.length - 1);
        updatePaperSelection(papers);
      } else if (e.key === 'ArrowUp' || e.key === 'k') {
        e.preventDefault();
        selectedPaperIndex = Math.max(selectedPaperIndex - 1, 0);
        updatePaperSelection(papers);
      } else if (e.key === 'Enter' && selectedPaperIndex >= 0 && selectedPaperIndex < papers.length) {
        e.preventDefault();
        const pmid = papers[selectedPaperIndex].dataset.pmid;
        if (pmid && !pmid.startsWith('demo')) {
          window.open(`https://pubmed.ncbi.nlm.nih.gov/${pmid}`, '_blank');
        }
      }
    }
  });
}

function updatePaperSelection(papers) {
  papers.forEach((el, i) => {
    if (i === selectedPaperIndex) {
      el.classList.add('paper-selected');
      el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    } else {
      el.classList.remove('paper-selected');
    }
  });
}
