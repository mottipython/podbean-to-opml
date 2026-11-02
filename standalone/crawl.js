// Runs on podbean.com pages. Reads the followed podcasts of the logged-in profile when the app page asks.
(() => {
  const api = globalThis.browser ?? globalThis.chrome;

  async function crawl() {
    const seen = new Set();
    const shows = [];
    const parse = (doc) => {
      const heading = [...doc.querySelectorAll('h2')].find((h) => /Followed Podcasts/.test(h.textContent));
      if (!heading) return 0;
      let added = 0;
      heading.parentElement.parentElement.querySelectorAll('a[href*="/podcast-detail/"], a[title], .card-title').forEach((el) => {
        const title = (el.getAttribute('title') || el.textContent || '').replace(/\s+/g, ' ').trim();
        const key = title.toLowerCase();
        if (title.length >= 2 && title.length < 120 && !seen.has(key)) {
          seen.add(key);
          shows.push({ title, author: null, source: 'podbean' });
          added++;
        }
      });
      return added;
    };
    parse(document);
    for (let page = 2; page <= 30; page++) {
      api.runtime.sendMessage({ type: 'progress', page, count: shows.length });
      const r = await fetch(`${location.pathname}?page2=${page}&pageSize=9`, { credentials: 'include' });
      if (!r.ok || parse(new DOMParser().parseFromString(await r.text(), 'text/html')) === 0) break;
    }
    return shows;
  }

  api.runtime.onMessage.addListener((msg, _sender, reply) => {
    if (msg?.type !== 'crawl') return;
    crawl().then((shows) => reply({ shows }), (e) => reply({ error: String(e) }));
    return true;
  });
})();
