// BioKhoj Service Worker
const CACHE_NAME = 'biokhoj-v1';
const STATIC_ASSETS = [
  '/biokhoj/',
  '/biokhoj/index.html',
  '/biokhoj/app.js',
  '/biokhoj/manifest.json',
  '/js/biokhoj-core.js',
  '/js/biokhoj-storage.js'
];

const API_CACHE_NAME = 'biokhoj-api-v1';
const API_CACHE_MAX_AGE = 30 * 60 * 1000; // 30 minutes

// Install: cache static assets
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => {
      return cache.addAll(STATIC_ASSETS).catch((err) => {
        console.warn('[SW] Some static assets failed to cache:', err);
        // Cache what we can individually
        return Promise.allSettled(
          STATIC_ASSETS.map((url) => cache.add(url).catch(() => {}))
        );
      });
    })
  );
  self.skipWaiting();
});

// Activate: clean old caches
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      return Promise.all(
        keys
          .filter((key) => key !== CACHE_NAME && key !== API_CACHE_NAME)
          .map((key) => caches.delete(key))
      );
    })
  );
  self.clients.claim();
});

// Fetch: network-first for API, cache-first for static
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);

  // Only handle http/https — skip chrome-extension:// and other schemes
  if (!url.protocol.startsWith('http')) return;

  // NCBI/API requests: network-first with cache fallback
  if (url.hostname.includes('ncbi.nlm.nih.gov') ||
      url.hostname.includes('eutils.be-md.ncbi.nlm.nih.gov') ||
      url.pathname.startsWith('/api/')) {
    event.respondWith(networkFirstWithCache(event.request));
    return;
  }

  // CDN resources: cache-first
  if (url.hostname.includes('cdn.') || url.hostname.includes('cdnjs.') ||
      url.hostname.includes('unpkg.com')) {
    event.respondWith(cacheFirstWithNetwork(event.request));
    return;
  }

  // Static assets: cache-first with network fallback
  if (event.request.method === 'GET') {
    event.respondWith(cacheFirstWithNetwork(event.request));
    return;
  }
});

async function networkFirstWithCache(request) {
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(API_CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch (err) {
    const cached = await caches.match(request);
    if (cached) return cached;
    return new Response(
      JSON.stringify({ error: 'offline', message: 'No cached data available' }),
      { status: 503, headers: { 'Content-Type': 'application/json' } }
    );
  }
}

async function cacheFirstWithNetwork(request) {
  const cached = await caches.match(request);
  if (cached) return cached;

  try {
    const response = await fetch(request);
    if (response.ok && request.method === 'GET') {
      const cache = await caches.open(CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch (err) {
    // Offline fallback for navigation requests
    if (request.mode === 'navigate') {
      const fallback = await caches.match('/biokhoj/index.html');
      if (fallback) return fallback;
    }
    return new Response('Offline', { status: 503 });
  }
}

// Background sync for paper checks
self.addEventListener('sync', (event) => {
  if (event.tag === 'check-papers') {
    event.waitUntil(backgroundPaperCheck());
  }
});

async function backgroundPaperCheck() {
  // Notify the client to run a paper check
  const clients = await self.clients.matchAll({ type: 'window' });
  for (const client of clients) {
    client.postMessage({ type: 'background-check-papers' });
  }
}

// Periodic background sync (where supported)
self.addEventListener('periodicsync', (event) => {
  if (event.tag === 'check-papers-periodic') {
    event.waitUntil(backgroundPaperCheck());
  }
});

// Push notification handler
self.addEventListener('push', (event) => {
  let data = { title: 'BioKhoj', body: 'New papers found for your watchlist' };
  try {
    if (event.data) data = event.data.json();
  } catch (e) {
    // Use defaults
  }

  const options = {
    body: data.body || 'New papers match your watchlist entities',
    icon: '/biokhoj/icons/icon-192.png',
    badge: '/biokhoj/icons/icon-72.png',
    tag: 'biokhoj-papers',
    renotify: true,
    data: { url: '/biokhoj/' },
    actions: [
      { action: 'view', title: 'View Feed' },
      { action: 'dismiss', title: 'Dismiss' }
    ]
  };

  event.waitUntil(self.registration.showNotification(data.title || 'BioKhoj', options));
});

// Notification click
self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  if (event.action === 'dismiss') return;

  event.waitUntil(
    self.clients.matchAll({ type: 'window' }).then((clientList) => {
      for (const client of clientList) {
        if (client.url.includes('/biokhoj') && 'focus' in client) {
          return client.focus();
        }
      }
      return self.clients.openWindow('/biokhoj/');
    })
  );
});
