// Runs on podbean.com only when the cockpit opened the tab with #podrescue=<cockpit origin>.
(async function () {
  const m = location.hash.match(/podrescue=([^&]+)/);
  if (!m) return;
  const cockpit = decodeURIComponent(m[1]);
  if (!/^http:\/\/(127\.0\.0\.1|localhost)(:\d+)?$/.test(cockpit)) return;
  history.replaceState(null, '', location.pathname + location.search);

  const bar = document.createElement('div');
  bar.style.cssText = 'position:fixed;top:16px;right:16px;z-index:2147483647;background:#111317;color:#ededef;padding:14px 18px;border-radius:8px;font:13px system-ui,sans-serif;border:1px solid rgba(255,255,255,.15);max-width:340px;box-shadow:0 12px 30px rgba(0,0,0,.8)';
  bar.textContent = 'Podcast Rescue: reading your followed podcasts...';
  document.body.appendChild(bar);

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

  try {
    parse(document);
    for (let page = 2; page <= 30; page++) {
      bar.textContent = `Podcast Rescue: page ${page} (${shows.length} shows)...`;
      const r = await fetch(`${location.pathname}?page2=${page}&pageSize=9`, { credentials: 'include' });
      if (!r.ok || parse(new DOMParser().parseFromString(await r.text(), 'text/html')) === 0) break;
    }
    if (!shows.length) {
      bar.textContent = 'Podcast Rescue: no followed podcasts found. Open your own profile page while logged in.';
      return;
    }
    const res = await fetch(`${cockpit}/api/auto-ingest`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ shows, source: 'podbean' }),
    });
    const data = await res.json();
    bar.textContent = `Podcast Rescue: sent ${data.count} shows. You can close this tab.`;
  } catch (e) {
    bar.textContent = 'Podcast Rescue: sync failed (' + e + '). Is the app still running?';
  }
})();
