import assert from 'node:assert/strict';
import { authorsMatch, normalizeTitle, resolveShow, suggestedCandidate } from './resolver.js';
import { buildOpml } from './opml.js';

assert.ok(authorsMatch('The New York Times', 'New York Times'));
assert.ok(authorsMatch('Vox Media', 'Vox'));
assert.ok(!authorsMatch('BBC', 'iHeartPodcasts'));
assert.equal(normalizeTitle('The Daily & Co.'), 'the daily co');

const mk = (...items) => async () => ({ ok: true, json: async () => ({ results: items }) });
const item = (n, a, f) => ({ collectionName: n, artistName: a, feedUrl: f });

let r = await resolveShow({ title: 'Planet Money' }, mk(item('Planet Money', 'NPR', 'https://a/feed')));
assert.equal(r.status, 'Resolved'); assert.equal(r.feed_url, 'https://a/feed');

r = await resolveShow({ title: 'X', author: 'NPR' }, mk(item('X', 'BBC', 'https://b'), item('X', 'NPR', 'https://c')));
assert.equal(r.feed_url, 'https://c');

r = await resolveShow({ title: 'X' }, mk(item('X', 'BBC', 'https://b'), item('X', 'NPR', 'https://c')));
assert.equal(r.status, 'Ambiguous'); assert.equal(suggestedCandidate(r).feed_url, 'https://b');

r = await resolveShow({ title: 'Serial' }, mk(item('Serial: Season 2', 'Serial', 'https://s')));
assert.equal(r.status, 'Ambiguous');
r = await resolveShow({ title: 'Nope' }, mk(item('Totally Other', 'Z', 'https://z')));
assert.equal(r.status, 'NotFound');

const xml = buildOpml([{ status: 'Resolved', original_title: 'A & B', feed_url: 'https://f?a=1&b=2' }, { status: 'NotFound', original_title: 'N' }]);
assert.ok(xml.includes('text="A &amp; B"') && xml.includes('xmlUrl="https://f?a=1&amp;b=2"') && xml.includes('1 subscriptions'));
console.log('ok');
