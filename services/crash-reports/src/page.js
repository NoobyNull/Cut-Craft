// The private dashboard. The page itself holds no data: it asks for a security key (or, until
// the first key is registered, the admin token) and then reads everything from /admin/api/*,
// so anyone who finds the address sees only a sign-in box.
// Logs are untrusted text from users' machines, so they are only ever set with textContent.

export const PAGE = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<title>CutCraft crash reports</title>
<style>
:root {
  --bg: #f6f5f2; --surface: #ffffff; --ink: #1d1d1b; --ink-2: #55534e; --ink-3: #8a877f;
  --line: #e4e1da; --accent: #c8611b; --bar: #2f6db5; --bar-hover: #1f4f88;
  --new: #b4410f; --new-bg: #fbe6d9; --back: #8e1d1d; --back-bg: #f8dcdc;
  --issue: #2c5aa0; --issue-bg: #dde8f7; --handled: #2f6b3a; --handled-bg: #dcefdf;
  --code-bg: #f1efea;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #161614; --surface: #1f1f1c; --ink: #ecebe6; --ink-2: #b5b2aa; --ink-3: #85827a;
    --line: #34332f; --accent: #e88443; --bar: #6aa1e0; --bar-hover: #9cc2ef;
    --new: #f2a273; --new-bg: #3d2416; --back: #f19a9a; --back-bg: #421c1c;
    --issue: #9cbcf0; --issue-bg: #1c2b44; --handled: #93cf9d; --handled-bg: #1b3220;
    --code-bg: #262622;
  }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--ink);
  font: 15px/1.45 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; }
header { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; padding: 14px 20px;
  background: var(--surface); border-bottom: 1px solid var(--line); }
header h1 { font-size: 17px; margin: 0; margin-right: auto; }
header h1 span { color: var(--accent); }
main { max-width: 1180px; margin: 0 auto; padding: 20px 16px 60px; }
button, select, input { font: inherit; color: inherit; }
button { background: var(--surface); border: 1px solid var(--line); border-radius: 8px; padding: 6px 12px; cursor: pointer; }
button:hover { border-color: var(--ink-3); }
button.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
button.danger { color: var(--back); }
button:disabled { opacity: .5; cursor: default; }
select, input[type=text], input[type=password], input[type=url] { background: var(--surface);
  border: 1px solid var(--line); border-radius: 8px; padding: 6px 10px; }
:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.card { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 16px 18px; }
.signin { max-width: 420px; margin: 60px auto; }
.signin h2 { margin-top: 0; font-size: 18px; }
.signin p { color: var(--ink-2); margin: 0 0 14px; }
.signin input[type=password] { width: 100%; margin-bottom: 10px; }
.row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
.muted { color: var(--ink-3); }
.small { font-size: 13px; }
.error { color: var(--back); min-height: 1.4em; }
.tiles { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; margin-bottom: 14px; }
.tile .label { color: var(--ink-2); font-size: 13px; }
.tile .value { font-size: 28px; font-weight: 600; font-variant-numeric: tabular-nums; }
.tile .sub { color: var(--ink-3); font-size: 13px; }
.grid2 { display: grid; grid-template-columns: 2fr 1fr 1fr; gap: 12px; margin-bottom: 14px; }
@media (max-width: 860px) { .grid2 { grid-template-columns: 1fr; } }
h3 { font-size: 14px; margin: 0 0 10px; color: var(--ink-2); font-weight: 600; }
.chart svg { width: 100%; height: 170px; display: block; overflow: visible; }
.chart rect.bar { fill: var(--bar); }
.chart rect.bar:hover, .chart rect.bar.on { fill: var(--bar-hover); }
.chart text { fill: var(--ink-3); font-size: 11px; }
.chart line { stroke: var(--line); }
.tip { position: fixed; pointer-events: none; background: var(--ink); color: var(--bg); font-size: 12px;
  padding: 4px 8px; border-radius: 6px; display: none; z-index: 5; white-space: nowrap; }
.hbar { display: grid; grid-template-columns: minmax(70px, 40%) 1fr auto; gap: 8px; align-items: center; margin: 5px 0; font-size: 13px; }
.hbar .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.hbar .track { height: 10px; }
.hbar .fill { height: 10px; background: var(--bar); border-radius: 0 4px 4px 0; min-width: 2px; }
.hbar .n { font-variant-numeric: tabular-nums; color: var(--ink-2); }
.table-wrap { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 14px; }
th { text-align: left; color: var(--ink-2); font-weight: 600; font-size: 12px; padding: 6px 8px; border-bottom: 1px solid var(--line); white-space: nowrap; }
td { padding: 8px; border-bottom: 1px solid var(--line); vertical-align: top; }
td.num, th.num { text-align: right; font-variant-numeric: tabular-nums; }
tbody tr { cursor: pointer; }
tbody tr:hover { background: var(--code-bg); }
tbody tr.sel { background: var(--code-bg); box-shadow: inset 3px 0 0 var(--accent); }
.pill { display: inline-block; font-size: 12px; font-weight: 600; padding: 1px 8px; border-radius: 999px; white-space: nowrap; }
.pill.new { color: var(--new); background: var(--new-bg); }
.pill.back { color: var(--back); background: var(--back-bg); }
.pill.issue { color: var(--issue); background: var(--issue-bg); }
.pill.handled { color: var(--handled); background: var(--handled-bg); }
.detail { margin-top: 14px; }
.detail h2 { font-size: 18px; margin: 0 0 4px; word-break: break-word; }
.detail .sig { font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 12px; color: var(--ink-3); }
.actions { display: flex; flex-wrap: wrap; gap: 8px; margin: 14px 0; }
.actions input { flex: 1 1 260px; }
.log { margin-top: 12px; border: 1px solid var(--line); border-radius: 10px; overflow: hidden; }
.log .head { display: flex; gap: 10px; align-items: center; padding: 6px 10px; background: var(--code-bg); font-size: 13px; }
.log .head span { margin-right: auto; }
.log pre { margin: 0; padding: 10px; max-height: 360px; overflow: auto; font-size: 12px; line-height: 1.4;
  font-family: ui-monospace, Menlo, Consolas, monospace; white-space: pre-wrap; word-break: break-word; }
.note { font-size: 13px; color: var(--ink-2); background: var(--code-bg); border-radius: 8px; padding: 8px 10px; margin-top: 10px; }
.empty { color: var(--ink-3); padding: 18px 0; text-align: center; }
[hidden] { display: none !important; }
.keys { margin-bottom: 14px; }
.keys.warn { border-color: var(--accent); }
.keylist { list-style: none; margin: 0 0 10px; padding: 0; }
.keylist li { display: flex; gap: 10px; align-items: center; padding: 6px 0; border-bottom: 1px solid var(--line); font-size: 14px; }
.keylist li span { margin-right: auto; }
.keys input[type=text] { flex: 1 1 220px; }
</style>
</head>
<body>
<header>
  <h1><span>CutCraft</span> crash reports</h1>
  <div class="row" id="controls" hidden>
    <label class="small muted" for="range">Show</label>
    <select id="range">
      <option value="7">Last 7 days</option>
      <option value="30" selected>Last 30 days</option>
      <option value="90">Last 90 days</option>
      <option value="365">Last year</option>
    </select>
    <button id="refresh">Refresh</button>
    <button id="signout">Sign out</button>
  </div>
</header>
<main>
  <section class="card signin" id="signin">
    <h2>Sign in</h2>
    <div id="key-signin" hidden>
      <p>Insert your security key, click the button, then touch the key.</p>
      <button class="primary" id="key-go">Sign in with security key</button>
    </div>
    <div id="token-signin" hidden>
      <p>No security key is registered yet. Paste the admin token once, then add your keys.
        After that, only the keys sign in.</p>
      <input type="password" id="token" autocomplete="off" aria-label="Admin token">
      <div class="row">
        <button class="primary" id="go" style="margin-left:auto">Sign in</button>
      </div>
    </div>
    <div class="error small" id="signin-error" role="alert"></div>
  </section>

  <section id="dash" hidden>
    <div class="card keys" id="keys">
      <h3>Security keys</h3>
      <p class="small" id="keys-note"></p>
      <ul class="keylist" id="keylist"></ul>
      <div class="row">
        <input type="text" id="key-name" maxlength="60" placeholder="Name, e.g. YubiKey 5C or Bitwarden" aria-label="Key name">
        <button id="key-add">Add a security key</button>
      </div>
      <div class="error small" id="keys-error" role="alert"></div>
    </div>
    <div class="tiles">
      <div class="card tile"><div class="label">Reports</div><div class="value" id="t-total">–</div><div class="sub" id="t-total-sub"></div></div>
      <div class="card tile"><div class="label">Crashes needing a look</div><div class="value" id="t-open">–</div><div class="sub" id="t-open-sub"></div></div>
      <div class="card tile"><div class="label">Logs held</div><div class="value" id="t-logs">–</div><div class="sub" id="t-logs-sub"></div></div>
    </div>
    <div class="grid2">
      <div class="card chart"><h3>Reports per day</h3><div id="perday"></div></div>
      <div class="card"><h3>By version</h3><div id="byversion"></div></div>
      <div class="card"><h3>By system</h3><div id="byos"></div></div>
    </div>
    <div class="card">
      <h3>Crashes</h3>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Status</th><th>Crash</th><th class="num" id="th-recent">Reports</th><th class="num">All time</th>
            <th class="num">Installs</th><th>Versions</th><th>Last seen</th><th class="num">Logs</th></tr></thead>
          <tbody id="crashes"></tbody>
        </table>
      </div>
      <div class="empty" id="no-crashes" hidden>No crashes reported yet.</div>
    </div>
    <section class="card detail" id="detail" hidden></section>
    <p class="small muted" id="footer"></p>
  </section>
</main>
<div class="tip" id="tip"></div>
<script src="/dashboard.js"></script>
</body>
</html>`;

export const SCRIPT = String.raw`(() => {
const $ = (id) => document.getElementById(id);
const KEY = 'cutcraft-crash-token';
const STATUS = { new: 'New', back: 'Came back', issue: 'GitHub issue', handled: 'Handled' };
const REPO_ISSUES = 'https://github.com/NoobyNull/Cut-Craft/issues/new';
let token = null;
let selected = null;
let lastSummary = null;

// The admin token is only for adding the first key, so it is kept for this tab at most.
function stored() {
  try { return sessionStorage.getItem(KEY); } catch { return null; }
}
function store(t) {
  try { sessionStorage.setItem(KEY, t); } catch { /* storage blocked: this page only */ }
}
function forgetToken() {
  try { sessionStorage.removeItem(KEY); localStorage.removeItem(KEY); } catch {}
}

// WebAuthn sends bytes; the server speaks base64url.
const toB64 = (buf) => {
  let s = '';
  for (const b of new Uint8Array(buf)) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
};
const fromB64 = (str) => {
  const bin = atob(str.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((str.length + 3) % 4));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
};
const keyProblem = (e) => e && e.name === 'NotAllowedError'
  ? 'The key was not used (cancelled or timed out). Try again.'
  : (e && e.name === 'InvalidStateError' ? 'That key is already registered.' : (e && e.message) || 'Something went wrong.');

function el(tag, props = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === 'class') e.className = v;
    else if (k === 'text') e.textContent = v;
    else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else e.setAttribute(k, v);
  }
  for (const kid of kids) if (kid != null) e.append(kid);
  return e;
}

async function api(path, opts = {}) {
  const r = await fetch(path, {
    ...opts,
    headers: token
      ? { authorization: 'Bearer ' + token, 'content-type': 'application/json' }
      : { 'content-type': 'application/json' },
  });
  if (r.status === 401) {
    signOut(token ? 'That token was not accepted.' : 'Please sign in again.');
    throw new Error('signed out');
  }
  const body = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(body.error || ('Request failed (' + r.status + ')'));
  return body;
}

const when = (iso) => iso ? new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : '–';
const ago = (iso) => {
  if (!iso) return '–';
  const s = (Date.now() - Date.parse(iso)) / 1000;
  if (s < 90) return 'just now';
  if (s < 5400) return Math.round(s / 60) + ' min ago';
  if (s < 129600) return Math.round(s / 3600) + ' h ago';
  return Math.round(s / 86400) + ' days ago';
};
const kb = (n) => n < 1024 ? n + ' B' : (n / 1024).toFixed(n < 10240 ? 1 : 0) + ' KB';
const pill = (s) => el('span', { class: 'pill ' + s, text: STATUS[s] || s });

function showTip(ev, text) {
  const t = $('tip');
  t.textContent = text;
  t.style.display = 'block';
  t.style.left = Math.min(ev.clientX + 12, innerWidth - t.offsetWidth - 8) + 'px';
  t.style.top = (ev.clientY - 32) + 'px';
}
const hideTip = () => { $('tip').style.display = 'none'; };

function perDayChart(s) {
  const box = $('perday');
  box.replaceChildren();
  const counts = new Map(s.perDay.map((d) => [d.day, d.count]));
  const days = [];
  const start = Date.parse(s.from + 'T00:00:00Z');
  for (let i = 0; i < s.days; i++) {
    const d = new Date(start + i * 86400000).toISOString().slice(0, 10);
    days.push({ day: d, count: counts.get(d) || 0 });
  }
  const max = Math.max(1, ...days.map((d) => d.count));
  const W = 600, H = 150, top = 8, base = H - 18;
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H);
  svg.setAttribute('preserveAspectRatio', 'none');
  svg.setAttribute('role', 'img');
  svg.setAttribute('aria-label', 'Reports per day, ' + s.total + ' in total');
  const mk = (tag, attrs) => {
    const e = document.createElementNS(ns, tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    return e;
  };
  svg.append(mk('line', { x1: 0, x2: W, y1: base, y2: base }));
  const step = W / days.length;
  const bw = Math.max(1, step - 2);
  days.forEach((d, i) => {
    const h = d.count ? Math.max(2, (d.count / max) * (base - top)) : 0;
    const x = i * step + 1;
    // invisible full-height hit target, so empty days still show their tooltip
    const hit = mk('rect', { x: i * step, y: top, width: step, height: base - top, fill: 'transparent' });
    const bar = mk('rect', { class: 'bar', x, y: base - h, width: bw, height: h, rx: Math.min(2, bw / 2) });
    const label = new Date(d.day + 'T00:00:00Z').toLocaleDateString(undefined, { month: 'short', day: 'numeric', timeZone: 'UTC' });
    const tip = label + ': ' + d.count + (d.count === 1 ? ' report' : ' reports');
    for (const t of [hit, bar]) {
      t.addEventListener('mousemove', (ev) => { bar.classList.add('on'); showTip(ev, tip); });
      t.addEventListener('mouseleave', () => { bar.classList.remove('on'); hideTip(); });
    }
    svg.append(bar, hit);
  });
  const lab = (x, text, anchor) => { const t = mk('text', { x, y: H - 3, 'text-anchor': anchor }); t.textContent = text; svg.append(t); };
  const fmt = (d) => new Date(d + 'T00:00:00Z').toLocaleDateString(undefined, { month: 'short', day: 'numeric', timeZone: 'UTC' });
  lab(0, fmt(days[0].day), 'start');
  lab(W, fmt(days[days.length - 1].day), 'end');
  const peak = mk('text', { x: 0, y: top + 2, 'text-anchor': 'start' });
  peak.textContent = 'most in a day: ' + max;
  if (s.total) svg.append(peak);
  box.append(svg);
}

function hbars(target, rows, key) {
  const box = $(target);
  box.replaceChildren();
  if (!rows.length) { box.append(el('div', { class: 'empty', text: 'Nothing yet' })); return; }
  const max = Math.max(...rows.map((r) => r.count));
  const shown = rows.slice(0, 8);
  const rest = rows.slice(8).reduce((n, r) => n + r.count, 0);
  if (rest) shown.push({ [key]: 'Other', count: rest });
  for (const r of shown) {
    const fill = el('div', { class: 'fill' });
    fill.style.width = (100 * r.count / max) + '%';
    box.append(el('div', { class: 'hbar' },
      el('span', { class: 'name', title: r[key], text: r[key] }),
      el('div', { class: 'track' }, fill),
      el('span', { class: 'n', text: String(r.count) })));
  }
}

function render(s) {
  lastSummary = s;
  const open = s.crashes.filter((c) => c.status === 'new' || c.status === 'back');
  $('t-total').textContent = s.total;
  $('t-total-sub').textContent = 'in the last ' + s.days + ' days';
  $('t-open').textContent = open.length;
  const back = open.filter((c) => c.status === 'back').length;
  $('t-open-sub').textContent = s.crashes.length + ' different crashes in all' + (back ? ', ' + back + ' came back' : '');
  $('t-logs').textContent = s.held.logs;
  if (s.held.logs) {
    const gone = new Date(Date.parse(s.held.oldest) + s.settings.retainDays * 86400000);
    $('t-logs-sub').textContent = kb(s.held.bytes) + '; oldest deleted ' + gone.toLocaleDateString();
  } else {
    $('t-logs-sub').textContent = 'none stored';
  }
  $('th-recent').textContent = s.days + ' days';
  perDayChart(s);
  hbars('byversion', s.byVersion, 'version');
  hbars('byos', s.byOs, 'os');

  const body = $('crashes');
  body.replaceChildren();
  $('no-crashes').hidden = s.crashes.length > 0;
  for (const c of s.crashes) {
    const versions = c.first_version === c.last_version ? c.last_version : c.first_version + ' – ' + c.last_version;
    const tr = el('tr', { tabindex: '0', onclick: () => openCrash(c.signature),
      onkeydown: (e) => { if (e.key === 'Enter') openCrash(c.signature); } },
      el('td', {}, pill(c.status)),
      el('td', { text: c.title }),
      el('td', { class: 'num', text: String(c.recent) }),
      el('td', { class: 'num', text: String(c.count) }),
      el('td', { class: 'num', text: String(c.installs) }),
      el('td', { class: 'small', text: versions || '–' }),
      el('td', { class: 'small', title: when(c.last_seen), text: ago(c.last_seen) }),
      el('td', { class: 'num', text: String(c.samples) }));
    if (c.signature === selected) tr.classList.add('sel');
    body.append(tr);
  }
  $('footer').textContent = 'Example logs are deleted when a crash is marked handled or linked to a GitHub issue, ' +
    'and after ' + s.settings.retainDays + ' days in any case. Up to ' + s.settings.dailyLimit + ' reports are accepted a day.';
}

async function load() {
  try {
    render(await api('/admin/api/summary?days=' + $('range').value));
    if (selected) await openCrash(selected, true);
  } catch (e) {
    if (e.message !== 'signed out') $('footer').textContent = 'Could not load: ' + e.message;
  }
}

const FENCE = String.fromCharCode(96).repeat(3);
function issueDraftUrl(d) {
  const log = d.samples[0] ? d.samples[0].log : '';
  const tail = log.length > 5000 ? '…\n' + log.slice(-5000) : log;
  const body = [
    '**Crash:** ' + d.title,
    '**Seen:** ' + d.count + ' times on ' + d.installs + ' installs, versions ' + d.first_version + ' to ' + d.last_version,
    '**Systems:** ' + d.byOs.map((o) => o.os + ' (' + o.count + ')').join(', '),
    '**Signature:** ' + d.signature,
    '', 'Log from a crash report (check it for anything personal before posting):', '', FENCE, tail, FENCE,
  ].join('\n');
  return REPO_ISSUES + '?' + new URLSearchParams({ title: 'Crash: ' + d.title, body, labels: 'crash' });
}

async function act(sig, path, body, confirmText) {
  if (confirmText && !confirm(confirmText)) return;
  try {
    await api('/admin/api/crashes/' + encodeURIComponent(sig) + path, { method: path ? 'POST' : 'DELETE', body: JSON.stringify(body || {}) });
    if (!path) { selected = null; $('detail').hidden = true; }
    await load();
  } catch (e) {
    if (e.message !== 'signed out') alert(e.message);
  }
}

async function openCrash(sig, quiet) {
  selected = sig;
  for (const tr of $('crashes').children) tr.classList.remove('sel');
  let d;
  try { d = await api('/admin/api/crashes/' + encodeURIComponent(sig)); } catch (e) {
    if (!quiet && e.message !== 'signed out') alert(e.message);
    return;
  }
  if (lastSummary) {
    const i = lastSummary.crashes.findIndex((c) => c.signature === sig);
    if (i >= 0) $('crashes').children[i].classList.add('sel');
  }
  const box = $('detail');
  box.hidden = false;
  box.replaceChildren();
  box.append(
    el('div', { class: 'row' }, pill(d.status), el('span', { class: 'sig', text: d.signature })),
    el('h2', { text: d.title }),
    el('div', { class: 'small muted', text: d.count + ' reports from ' + d.installs + ' installs. First seen ' + when(d.first_seen) +
      ' (' + d.first_version + '), last ' + when(d.last_seen) + ' (' + d.last_version + ').' }),
    el('div', { class: 'small muted', text: 'Versions: ' + d.byVersion.map((v) => v.version + ' ×' + v.count).join(', ') +
      '. Systems: ' + d.byOs.map((o) => o.os + ' ×' + o.count).join(', ') + '.' }));
  if (d.issue_url) {
    box.append(el('div', { class: 'small' }, 'GitHub issue: ', el('a', { href: d.issue_url, target: '_blank', rel: 'noopener noreferrer', text: d.issue_url })));
  }
  if (d.note) box.append(el('div', { class: 'note', text: 'Note: ' + d.note }));

  const actions = el('div', { class: 'actions' });
  if (d.status === 'new' || d.status === 'back') {
    const note = el('input', { type: 'text', placeholder: 'Note, e.g. "fixed in 0.6.4" (optional)', 'aria-label': 'Note' });
    actions.append(note, el('button', { class: 'primary', text: 'Mark handled and delete logs',
      onclick: () => act(d.signature, '/handled', { note: note.value.trim() || undefined }) }));
  }
  if (d.status !== 'issue') {
    const link = el('input', { type: 'url', placeholder: 'https://github.com/NoobyNull/Cut-Craft/issues/123', 'aria-label': 'GitHub issue address' });
    const draft = el('button', { text: 'Draft GitHub issue', onclick: () => window.open(issueDraftUrl(d), '_blank', 'noopener') });
    if (!d.samples.length) draft.title = 'No example log left to include';
    actions.append(el('div', { class: 'row', style: 'flex-basis:100%' }, draft, link,
      el('button', { text: 'Save issue link and delete logs', onclick: () => act(d.signature, '/issue', { url: link.value.trim() }) })));
  }
  if (d.status !== 'new') actions.append(el('button', { text: 'Reopen', onclick: () => act(d.signature, '/reopen') }));
  actions.append(el('button', { class: 'danger', text: 'Forget this crash completely',
    onclick: () => act(d.signature, '', null, 'Delete this crash, its logs and all its counts? This cannot be undone.') }));
  box.append(actions);

  if (!d.samples.length) {
    box.append(el('div', { class: 'empty', text: d.status === 'new' || d.status === 'back'
      ? 'No example logs held.' : 'Logs were deleted when this crash was dealt with. New reports are only counted.' }));
  }
  for (const s of d.samples) {
    box.append(el('div', { class: 'log' },
      el('div', { class: 'head' },
        el('span', { text: s.version + ' on ' + s.os + ', ' + when(s.received_at) + ' (' + kb(new TextEncoder().encode(s.log).length) + ')' }),
        el('button', { text: 'Copy', onclick: () => navigator.clipboard.writeText(s.log) }),
        el('button', { class: 'danger', text: 'Delete', onclick: async () => {
          if (!confirm('Delete this log?')) return;
          try { await api('/admin/api/samples/' + s.id, { method: 'DELETE' }); await load(); } catch (e) { alert(e.message); }
        } })),
      el('pre', { text: s.log })));
  }
}

function signedIn() {
  $('signin').hidden = true;
  $('dash').hidden = false;
  $('controls').hidden = false;
  load();
  loadKeys();
}
async function signOut(msg) {
  token = null;
  forgetToken();
  $('dash').hidden = true;
  $('controls').hidden = true;
  $('signin').hidden = false;
  $('signin-error').textContent = msg || '';
  await showSignin();
}
async function showSignin() {
  let keys = 0;
  try { keys = (await (await fetch('/auth/status')).json()).keys; } catch {}
  $('key-signin').hidden = !keys;
  $('token-signin').hidden = Boolean(keys);
}

async function keySignin() {
  $('signin-error').textContent = '';
  $('key-go').disabled = true;
  try {
    const opts = await (await fetch('/auth/options', { method: 'POST' })).json();
    if (opts.error) throw new Error(opts.error);
    const cred = await navigator.credentials.get({ publicKey: {
      challenge: fromB64(opts.challenge),
      rpId: opts.rpId,
      allowCredentials: opts.allow.map((id) => ({ type: 'public-key', id: fromB64(id) })),
      userVerification: 'preferred',
      timeout: opts.timeout,
    } });
    const r = await fetch('/auth/verify', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        id: cred.id,
        clientDataJSON: toB64(cred.response.clientDataJSON),
        authenticatorData: toB64(cred.response.authenticatorData),
        signature: toB64(cred.response.signature),
      }),
    });
    const body = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(body.error || 'Sign-in failed.');
    signedIn();
  } catch (e) {
    $('signin-error').textContent = keyProblem(e);
  } finally {
    $('key-go').disabled = false;
  }
}

function renderKeys(keys) {
  const list = $('keylist');
  list.replaceChildren(...keys.map((k) => el('li', {},
    el('span', { text: k.name }),
    el('span', { class: 'small muted', text: 'added ' + when(k.created_at) + (k.last_used_at ? ', last used ' + ago(k.last_used_at) : ', not used yet') }),
    el('button', { class: 'danger', text: 'Remove', onclick: () => removeKey(k, keys.length) }),
  )));
  $('keys').classList.toggle('warn', keys.length < 2);
  $('keys-note').textContent = keys.length === 0
    ? 'You signed in with the admin token. Add your security keys now: once one is added, only keys can sign in.'
    : keys.length === 1
      ? 'Only one key is registered. Add a second as a backup, so losing one does not lock you out.'
      : 'Any of these keys can sign in.';
}
async function loadKeys() {
  try { renderKeys((await api('/admin/api/keys')).keys); } catch (e) {
    if (e.message !== 'signed out') $('keys-error').textContent = e.message;
  }
}
async function addKey() {
  $('keys-error').textContent = '';
  $('key-add').disabled = true;
  try {
    const opts = await api('/admin/api/keys/options', { method: 'POST' });
    const cred = await navigator.credentials.create({ publicKey: {
      challenge: fromB64(opts.challenge),
      rp: opts.rp,
      user: { id: fromB64(opts.user.id), name: opts.user.name, displayName: opts.user.displayName },
      pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
      // 'preferred' so password managers such as Bitwarden offer to save it; a YubiKey keeps it too
      authenticatorSelection: { residentKey: 'preferred', requireResidentKey: false, userVerification: 'preferred' },
      excludeCredentials: opts.exclude.map((id) => ({ type: 'public-key', id: fromB64(id) })),
      attestation: 'none',
      timeout: opts.timeout,
    } });
    const out = await api('/admin/api/keys', { method: 'POST', body: JSON.stringify({
      name: $('key-name').value,
      clientDataJSON: toB64(cred.response.clientDataJSON),
      attestationObject: toB64(cred.response.attestationObject),
    }) });
    // the first key comes with a session, so the token is not needed any more
    token = null;
    forgetToken();
    $('key-name').value = '';
    renderKeys(out.keys);
  } catch (e) {
    if (e.message !== 'signed out') $('keys-error').textContent = keyProblem(e);
  } finally {
    $('key-add').disabled = false;
  }
}
async function removeKey(k, count) {
  const warn = count === 1
    ? 'Remove "' + k.name + '"? It is the only key, so the admin token will sign in again.'
    : 'Remove "' + k.name + '"? It will no longer sign in.';
  if (!confirm(warn)) return;
  try {
    renderKeys((await api('/admin/api/keys/' + encodeURIComponent(k.id), { method: 'DELETE' })).keys);
  } catch (e) {
    if (e.message !== 'signed out') $('keys-error').textContent = e.message;
  }
}

$('go').addEventListener('click', () => {
  const t = $('token').value.trim();
  if (!t) { $('signin-error').textContent = 'Paste the token first.'; return; }
  token = t;
  store(t);
  $('token').value = '';
  signedIn();
});
$('token').addEventListener('keydown', (e) => { if (e.key === 'Enter') $('go').click(); });
$('key-go').addEventListener('click', keySignin);
$('key-add').addEventListener('click', addKey);
$('signout').addEventListener('click', async () => {
  try { await fetch('/auth/logout', { method: 'POST' }); } catch {}
  signOut();
});
$('refresh').addEventListener('click', load);
$('range').addEventListener('change', load);

(async () => {
  token = stored();
  let status = { keys: 0, signedIn: false };
  try { status = await (await fetch('/auth/status')).json(); } catch {}
  if (status.signedIn || (token && !status.keys)) signedIn();
  else { if (token) forgetToken(); token = null; signOut(); }
})();
})();`;
