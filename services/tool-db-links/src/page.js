// The human-readable list at "/", built from the same rows the API serves.
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

const STATUS = {
  ok: ['Current', 'ok'],
  manual: ['Set by hand', 'ok'],
  new: ['Not checked yet', 'wait'],
  'needs-link': ['Link to set by hand', 'warn'],
  blocked: ['Site blocks checks', 'warn'],
  'link-missing': ['Link gone from page', 'bad'],
  'download-broken': ['Download broken', 'bad'],
  'page-unreachable': ['Page unreachable', 'bad'],
};

const FORMAT = {
  'vectric-vtdb': 'Vectric (.vtdb)',
  'vectric-tool': 'Vectric (.tool)',
  'fusion-tools': 'Fusion 360 (.tools)',
  'carveco-tdb': 'Carveco (.tdb)',
};

export function page(rows) {
  const items = rows.map((r) => {
    const [label, tone] = STATUS[r.status] ?? [r.status, 'wait'];
    const date = (iso) => (iso ? new Date(iso).toISOString().slice(0, 10) : '—');
    return `<tr>
  <td><a href="${esc(r.page_url)}" rel="noopener">${esc(r.vendor)}</a></td>
  <td>${esc(FORMAT[r.format] ?? r.format)}</td>
  <td>${r.url ? `<a href="${esc(r.url)}" rel="noopener nofollow">${esc(r.filename)}</a>` : '—'}</td>
  <td>${esc(r.version ?? '—')}</td>
  <td><span class="pill ${tone}" title="${esc(r.message)}">${esc(label)}</span></td>
  <td class="d">${date(r.changed_at)}</td>
  <td class="d">${date(r.checked_at)}</td>
</tr>`;
  });
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>CutCraft tool databases</title>
<meta name="description" content="Where each router-bit maker publishes its current tool database, checked weekly.">
<style>
:root{--bg:#fbfaf7;--fg:#1f1d1a;--mute:#6b665e;--line:#e4e0d8;--ok:#2f7a3d;--warn:#a46a00;--bad:#b3261e;--accent:#d9731f}
@media (prefers-color-scheme:dark){:root{--bg:#171614;--fg:#ece8e1;--mute:#a39d93;--line:#34312c;--ok:#6cc47c;--warn:#e0a640;--bad:#f2867e}}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 system-ui,sans-serif}
main{max-width:960px;margin:0 auto;padding:32px 16px}
h1{font-size:24px;margin:0 0 4px}p{color:var(--mute);margin:0 0 20px}
.wrap{overflow-x:auto}table{border-collapse:collapse;width:100%;min-width:640px}
th,td{text-align:left;padding:8px 10px;border-bottom:1px solid var(--line);vertical-align:top}
th{font-size:12px;text-transform:uppercase;letter-spacing:.04em;color:var(--mute)}
a{color:inherit;text-decoration-color:var(--accent)}
.pill{font-size:12px;padding:2px 8px;border-radius:99px;border:1px solid currentColor;white-space:nowrap}
.ok{color:var(--ok)}.warn{color:var(--warn)}.bad{color:var(--bad)}.wait{color:var(--mute)}
.d{white-space:nowrap}
small{display:block;margin-top:20px;color:var(--mute)}
</style></head><body><main>
<h1>Tool databases from bit makers</h1>
<p>Links go straight to each maker's own file. Nothing is copied or hosted here. Checked once a week.</p>
<div class="wrap"><table>
<thead><tr><th>Maker</th><th>For</th><th>File</th><th>Version</th><th>Status</th><th>Changed</th><th>Checked</th></tr></thead>
<tbody>${items.join('\n') || '<tr><td colspan="7">No makers listed yet.</td></tr>'}</tbody>
</table></div>
<small>Vendor values are starting points; check them against your bit and material. JSON for apps: <a href="/v1/tool-databases">/v1/tool-databases</a></small>
</main></body></html>`;
}
