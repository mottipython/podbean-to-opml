const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&apos;');

export function buildOpml(results) {
  const done = results.filter((r) => r.status === 'Resolved').sort((a, b) => a.original_title.toLowerCase().localeCompare(b.original_title.toLowerCase()));
  const open = results.length - done.length;
  const lines = [
    '<?xml version="1.0" encoding="UTF-8"?>', '<opml version="1.0">', '  <head>',
    '    <title>Rescued Podcast Subscriptions</title>',
    `    <dateCreated>${new Date().toUTCString()}</dateCreated>`, '  </head>', '  <body>',
  ];
  if (open) lines.push(`    <!-- Note: ${open} subscriptions could not be safely resolved. See report for details. -->`);
  for (const r of done) {
    const t = esc(r.original_title);
    lines.push(`    <outline text="${t}" title="${t}" type="rss" xmlUrl="${esc(r.feed_url)}" htmlUrl="" />`);
  }
  lines.push('  </body>', '</opml>', '');
  return lines.join('\n');
}

export function buildReport(results) {
  const out = ['# Podcast Rescue - Unresolved Subscriptions Report', ''];
  for (const r of results.filter((x) => x.status !== 'Resolved')) {
    out.push(`${r.status === 'Ambiguous' ? 'AMBIGUOUS' : 'NOT FOUND'}: ${r.original_title}${r.original_author ? ` (${r.original_author})` : ''}`);
    for (const c of r.candidates) out.push(`  - ${c.collection_name} by ${c.artist_name}: ${c.feed_url}`);
  }
  return out.join('\n') + '\n';
}
