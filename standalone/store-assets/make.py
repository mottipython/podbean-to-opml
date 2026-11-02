#!/usr/bin/env python3
"""Regenerates the store screenshots, promo tiles and large icons with headless Edge/Chrome."""
import http.server, json, pathlib, shutil, subprocess, tempfile, threading, functools

HERE = pathlib.Path(__file__).parent
SRC = HERE.parent
BROWSER = shutil.which('microsoft-edge') or shutil.which('google-chrome') or shutil.which('chromium')

def cand(n, a, f='https://example.com/feed'):
    return {'collection_name': n, 'artist_name': a, 'feed_url': f, 'artwork_url': None}

def done(t, a, m=None):
    return {'status': 'Resolved', 'original_title': t, 'original_author': None, 'matched_title': m or t, 'artist': a,
            'feed_url': 'https://feeds.example.com/' + ''.join(c for c in t.lower() if c.isalnum())[:18], 'candidates': [], 'reason': ''}

def pick(t, *c):
    return {'status': 'Ambiguous', 'original_title': t, 'original_author': None, 'candidates': list(c), 'reason': ''}

def pend(t):
    return {'status': 'Pending', 'original_title': t, 'original_author': None, 'candidates': [], 'reason': ''}

SHOWS = [('99% Invisible', 'Roman Mars'), ('Decoder Ring', 'Slate Podcasts'), ('Radiolab', 'WNYC Studios'), ('Reveal', 'The Center for Investigative Reporting'),
         ('Short Wave', 'NPR'), ('Planet Money', 'NPR'), ('The Daily', 'The New York Times'), ('Criminal', 'Vox Media Podcast Network'),
         ('Hidden Brain', 'Hidden Brain Media'), ('Freakonomics Radio', 'Freakonomics Network'), ('Reply All', 'Gimlet Media'), ('Song Exploder', 'Hrishikesh Hirway')]
RESOLVED = [done(t, a) for t, a in SHOWS]
AMBIG = [pick('The Habit', cand('The Habit', 'Wondery'), cand('The Habit', 'Sam Harris')), pick('Origins', cand('Origins', 'Origins Media'), cand('Origins', 'Ohio State'))]
SCENES = {
    'working': dict(url='https://www.podbean.com/user-xxxxxxxxxxxx', status=('Found 70 shows. Looking up their feeds...', ''), results=RESOLVED[:7] + [pend(t) for t, _ in SHOWS[7:]]),
    'review': dict(url='https://www.podbean.com/user-xxxxxxxxxxxx', status=('Done. Review the shows marked Choose, then download your OPML.', 'ok'), results=RESOLVED[:3] + AMBIG + RESOLVED[3:]),
    'ready': dict(url='https://www.podbean.com/user-xxxxxxxxxxxx', status=('Done. Review the shows marked Choose, then download your OPML.', 'ok'), results=RESOLVED),
}
STUB = """<script>
const S = %s;
globalThis.chrome = {storage:{local:{get:async()=>({results:S.results}),set:async()=>{}}},runtime:{onMessage:{addListener(){}},sendMessage(){}},tabs:{}};
document.getElementById('profile').value = S.url;
const s = document.getElementById('status'); s.hidden = false; s.textContent = S.status[0]; s.className = 'status ' + S.status[1];
</script>"""

def shoot(url, out, w, h):
    subprocess.run([BROWSER, '--headless=new', '--no-sandbox', '--hide-scrollbars', '--default-background-color=00000000',
                    f'--window-size={w},{h}', f'--screenshot={out}', url], check=True, capture_output=True)

def card(body, css=''):
    return f"""<html><body style="margin:0;font-family:Inter,'Noto Sans','DejaVu Sans',sans-serif;color:#ededef;{css}">{body}</body></html>"""

ICON = '<svg viewBox="0 0 128 128" width="100%" height="100%"><rect width="128" height="128" rx="28" fill="#5e6ad2"/><circle cx="64" cy="64" r="11" fill="#fff"/><path d="M42 40a34 34 0 0 0 0 48M86 40a34 34 0 0 1 0 48M30 30a50 50 0 0 0 0 68M98 30a50 50 0 0 1 0 68" stroke="#fff" stroke-width="7" stroke-linecap="round" fill="none"/></svg>'

def main():
    tmp = pathlib.Path(tempfile.mkdtemp())
    for f in SRC.glob('*'):
        if f.is_file(): shutil.copy(f, tmp)
    page = (SRC / 'app.html').read_text()
    for name, scene in SCENES.items():
        (tmp / f'{name}.html').write_text(page.replace('<script type="module"', STUB % json.dumps(scene) + '<script type="module"'))
    tiles = {
        'logo-300.html': card(f'<div style="width:300px;height:300px">{ICON}</div>', 'background:transparent'),
        'icon-512.html': card(f'<div style="width:512px;height:512px">{ICON}</div>', 'background:transparent'),
        'promo-440x280.html': card(f'<div style="width:440px;height:280px;background:linear-gradient(135deg,#1a1d3a,#5e6ad2);display:flex;align-items:center;gap:20px;padding:0 30px;box-sizing:border-box"><div style="width:96px;height:96px;flex:none">{ICON}</div><div><div style="font-size:26px;font-weight:700;line-height:1.15">Podcast Rescue</div><div style="font-size:15px;opacity:.85;margin-top:6px">Podbean follows to OPML in one click</div></div></div>'),
        'marquee-1400x560.html': card(f'<div style="width:1400px;height:560px;background:linear-gradient(135deg,#1a1d3a,#5e6ad2);display:flex;align-items:center;gap:48px;padding:0 120px;box-sizing:border-box"><div style="width:220px;height:220px;flex:none">{ICON}</div><div><div style="font-size:72px;font-weight:700;line-height:1.05">Podcast Rescue</div><div style="font-size:30px;opacity:.88;margin-top:14px">Take your Podbean podcasts to any app.<br>Private. No account. No server.</div></div></div>'),
    }
    for n, html in tiles.items(): (tmp / n).write_text(html)
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(tmp))
    handler.log_message = lambda *a: None
    srv = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    base = f'http://127.0.0.1:{srv.server_port}/'
    for i, name in enumerate(['working', 'review', 'ready'], 1):
        shoot(base + name + '.html', str(HERE / f'screenshot-{i}-{name}-1280x800.png'), 1280, 800)
    shoot(base + 'logo-300.html', str(HERE / 'logo-300x300.png'), 300, 300)
    shoot(base + 'icon-512.html', str(HERE / 'icon-512x512.png'), 512, 512)
    shoot(base + 'promo-440x280.html', str(HERE / 'promo-small-440x280.png'), 440, 280)
    shoot(base + 'marquee-1400x560.html', str(HERE / 'promo-marquee-1400x560.png'), 1400, 560)
    srv.shutdown()

main()
# Chrome Web Store wants opaque 24-bit PNGs for screenshots and tiles; convert after running:
# python3 -c "from PIL import Image;import glob;[Image.open(f).convert('RGB).save(f) for f in glob.glob('screenshot-*.png')+glob.glob('promo-*.png')]"
