// Matches Podbean show titles to RSS feeds through Apple's public podcast directory.
const clean = (s) => s.replace(/[^\p{L}\p{N}\s]/gu, ' ').split(/\s+/).filter(Boolean).join(' ').toLowerCase();

export const normalizeTitle = clean;

const STOP = new Set(['the', 'a', 'an', 'and', '&', 'inc', 'llc', 'ltd', 'corp', 'corporation', 'media', 'studios', 'studio', 'productions', 'production', 'network', 'podcast', 'podcasts', 'group', 'radio']);

export function authorsMatch(a, b) {
  a = (a || '').trim().toLowerCase();
  b = (b || '').trim().toLowerCase();
  if (!a || !b) return false;
  if (a === b) return true;
  const ca = clean(a), cb = clean(b);
  if (ca === cb || ca.includes(cb) || cb.includes(ca)) return true;
  const wa = new Set(ca.split(' ').filter((w) => !STOP.has(w)));
  const wb = new Set(cb.split(' ').filter((w) => !STOP.has(w)));
  const subset = (x, y) => [...x].every((w) => y.has(w));
  return wa.size > 0 && wb.size > 0 && (subset(wa, wb) || subset(wb, wa));
}

const result = (show, status, reason, extra = {}) => ({
  status, original_title: show.title, original_author: show.author || null,
  feed_url: null, matched_title: null, artist: null, artwork_url: null,
  candidates: [], reason, ...extra,
});

const pick = (show, c, reason, candidates = []) =>
  result(show, 'Resolved', reason, { feed_url: c.feed_url, matched_title: c.collection_name, artist: c.artist_name, artwork_url: c.artwork_url, candidates });

async function lookup(title, fetchFn) {
  const url = `https://itunes.apple.com/search?media=podcast&entity=podcast&limit=8&term=${encodeURIComponent(title)}`;
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const r = await fetchFn(url);
      if (r.ok) return (await r.json()).results || [];
      if (![403, 429, 503].includes(r.status)) break;
    } catch (_) { /* retry */ }
    await new Promise((ok) => setTimeout(ok, 500 * (attempt + 1)));
  }
  return null;
}

export async function resolveShow(show, fetchFn = fetch) {
  const raw = await lookup(show.title, fetchFn);
  if (raw === null) return result(show, 'NotFound', 'Directory query timed out or returned no response');
  const cands = raw.filter((r) => r.feedUrl).map((r) => ({
    collection_name: r.collectionName || show.title,
    artist_name: r.artistName || '',
    feed_url: r.feedUrl,
    artwork_url: r.artworkUrl100 || null,
  }));
  if (!cands.length) return result(show, 'NotFound', 'No active RSS feed found in directory');

  const target = show.title.trim().toLowerCase();
  const norm = normalizeTitle(show.title);
  const exact = [], close = [];
  for (const c of cands) {
    const name = c.collection_name.trim().toLowerCase();
    if (name === target || normalizeTitle(c.collection_name) === norm) exact.push(c);
    else if ([`${target}:`, `${target} -`].some((p) => name.startsWith(p)) || [`${name}:`, `${name} -`].some((p) => target.startsWith(p))) close.push(c);
  }

  if (exact.length === 1) return pick(show, exact[0], 'Exact title match');
  if (exact.length > 1) {
    const byAuthor = exact.filter((c) => authorsMatch(show.author, c.artist_name));
    if (byAuthor.length === 1) return pick(show, byAuthor[0], 'Disambiguated duplicate title using publisher metadata', exact);
    return result(show, 'Ambiguous', 'Multiple shows found with this exact title (requires selection)', { candidates: exact });
  }
  if (close.length) {
    const byAuthor = close.filter((c) => authorsMatch(show.author, c.artist_name));
    if (byAuthor.length === 1) return pick(show, byAuthor[0], 'Matched subtitle prefix and publisher', close);
    return result(show, 'Ambiguous', 'Near-match candidates found in directory', { candidates: close });
  }
  return result(show, 'NotFound', 'No matching podcast found in directory');
}

export function suggestedCandidate(r) {
  const author = (r.original_author || '').toLowerCase();
  const byAuthor = author && r.candidates.find((c) => {
    const a = c.artist_name.toLowerCase();
    return a.includes(author) || author.includes(a);
  });
  return byAuthor || r.candidates[0];
}

export function choose(r, c) {
  Object.assign(r, { status: 'Resolved', matched_title: c.collection_name, feed_url: c.feed_url, artist: c.artist_name, artwork_url: c.artwork_url });
}

export async function resolveAll(shows, onResult, fetchFn = fetch, workers = 4) {
  let next = 0;
  const run = async () => {
    while (next < shows.length) {
      const i = next++;
      onResult(i, await resolveShow(shows[i], fetchFn));
    }
  };
  await Promise.all(Array.from({ length: workers }, run));
}
