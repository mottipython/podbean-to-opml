import { resolveAll, suggestedCandidate, choose } from './resolver.js';
import { buildOpml, buildReport } from './opml.js';

const api = globalThis.browser ?? globalThis.chrome;
const $ = (id) => document.getElementById(id);
let results = [];

const save = () => api.storage.local.set({ results });
const PROFILE = /^https:\/\/([\w-]+\.)*podbean\.com\/user-[\w-]+/;

function status(text, kind = '') {
  const el = $('status');
  el.hidden = !text;
  el.textContent = text;
  el.className = 'status ' + kind;
}

function render() {
  $('results').hidden = !results.length;
  const count = (s) => results.filter((r) => r.status === s).length;
  const pending = results.filter((r) => r.status === 'Pending').length;
  $('summary').textContent = `${results.length} shows: ${count('Resolved')} resolved, ${count('Ambiguous')} to choose, ${count('NotFound')} not found${pending ? `, ${pending} pending` : ''}`;
  $('accept').hidden = !count('Ambiguous');
  $('export').disabled = !count('Resolved');
  const rows = $('rows');
  rows.replaceChildren(...results.map((r, i) => {
    const tr = document.createElement('tr');
    const name = document.createElement('td');
    name.textContent = r.original_title;
    const match = document.createElement('td');
    if (r.status === 'Ambiguous') {
      const sel = document.createElement('select');
      sel.append(new Option('Choose the right show...', ''));
      r.candidates.forEach((c, j) => sel.append(new Option(`${c.collection_name} by ${c.artist_name}`, j)));
      sel.onchange = () => { if (sel.value !== '') { choose(r, r.candidates[+sel.value]); save(); render(); } };
      match.append(sel);
    } else if (r.status === 'Resolved') {
      match.textContent = `${r.matched_title} by ${r.artist}`;
      const feed = document.createElement('small');
      feed.textContent = r.feed_url;
      match.append(feed);
    } else {
      match.textContent = r.reason || '';
    }
    const tag = document.createElement('td');
    const span = document.createElement('span');
    span.className = 'tag ' + r.status;
    span.textContent = { NotFound: 'Not found', Ambiguous: 'Choose' }[r.status] || r.status;
    tag.append(span);
    tr.append(name, match, tag);
    return tr;
  }));
}

function download(name, text, type) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([text], { type }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

async function crawl(url) {
  const tab = await api.tabs.create({ url, active: false });
  try {
    await new Promise((ok) => {
      const done = (id, info) => { if (id === tab.id && info.status === 'complete') { api.tabs.onUpdated.removeListener(done); ok(); } };
      api.tabs.onUpdated.addListener(done);
      setTimeout(ok, 20000);
    });
    for (let attempt = 0; attempt < 20; attempt++) {
      try {
        return await api.tabs.sendMessage(tab.id, { type: 'crawl' });
      } catch (_) {
        await new Promise((ok) => setTimeout(ok, 500));
      }
    }
    throw new Error('The Podbean page did not respond. Reload the extension and try again.');
  } finally {
    api.tabs.remove(tab.id).catch(() => {});
  }
}

async function sync() {
  const url = $('profile').value.trim().split('#')[0];
  if (!PROFILE.test(url)) return status('Paste your Podbean profile link, like https://www.podbean.com/user-xxxxxxxx', 'err');
  $('sync').disabled = true;
  status('Opening your profile and reading followed podcasts...');
  try {
    const reply = await crawl(url);
    if (reply.error) throw new Error(reply.error);
    if (!reply.shows.length) throw new Error('No followed podcasts found. Log in to Podbean in this browser and use your own profile link.');
    results = reply.shows.map((s) => ({ status: 'Pending', original_title: s.title, original_author: s.author, candidates: [], reason: '' }));
    render();
    status(`Found ${results.length} shows. Looking up their feeds...`);
    await resolveAll(reply.shows, (i, r) => { results[i] = r; if (i % 5 === 0) render(); });
    await save();
    render();
    status('Done. Review the shows marked Choose, then download your OPML.', 'ok');
  } catch (e) {
    status(e.message, 'err');
  } finally {
    $('sync').disabled = false;
  }
}

api.runtime.onMessage.addListener((msg) => {
  if (msg?.type === 'progress') status(`Reading followed podcasts: page ${msg.page}, ${msg.count} shows so far...`);
});

$('sync').onclick = sync;
$('profile').onkeydown = (e) => { if (e.key === 'Enter') sync(); };
$('accept').onclick = () => { results.forEach((r) => { if (r.status === 'Ambiguous' && r.candidates.length) choose(r, suggestedCandidate(r)); }); save(); render(); };
$('export').onclick = () => download('podcasts_rescue.opml', buildOpml(results), 'text/x-opml');
$('report').onclick = () => download('unresolved_report.txt', buildReport(results), 'text/plain');
$('clear').onclick = () => { results = []; save(); render(); status(''); };

api.storage.local.get('results').then((d) => { results = d.results || []; render(); });
