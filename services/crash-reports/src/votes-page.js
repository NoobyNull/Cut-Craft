// The private Messages and votes page (/votes). Like the dashboard it holds no data: it reads
// /admin/api/polls with the dashboard's sign-in (security-key cookie, or the admin token kept
// in this tab). Text people typed is only ever set with textContent.

export const VOTES_PAGE = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<title>CutCraft messages and votes</title>
<style>
:root { --bg: #f6f5f2; --surface: #fff; --ink: #1d1d1b; --ink-2: #55534e; --ink-3: #8a877f;
  --line: #e4e1da; --accent: #c8611b; --bar: #2f6db5; --live: #2f6b3a; --live-bg: #dcefdf; --bad: #8e1d1d; }
@media (prefers-color-scheme: dark) {
  :root { --bg: #161614; --surface: #1f1f1c; --ink: #ecebe6; --ink-2: #b5b2aa; --ink-3: #85827a;
    --line: #34332f; --accent: #e88443; --bar: #6aa1e0; --live: #93cf9d; --live-bg: #1b3220; --bad: #f19a9a; }
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--ink); font: 15px/1.45 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; }
header { display: flex; align-items: center; gap: 12px; padding: 14px 20px; background: var(--surface); border-bottom: 1px solid var(--line); }
header h1 { font-size: 17px; margin: 0; margin-right: auto; }
header h1 span { color: var(--accent); }
a { color: var(--bar); }
main { max-width: 860px; margin: 0 auto; padding: 20px 16px 60px; display: grid; gap: 14px; }
.card { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 16px 18px; }
h2 { font-size: 16px; margin: 0 0 4px; }
h3 { font-size: 14px; margin: 0 0 10px; color: var(--ink-2); }
button, input, textarea { font: inherit; color: inherit; }
button { background: var(--surface); border: 1px solid var(--line); border-radius: 8px; padding: 6px 12px; cursor: pointer; }
button.primary { background: var(--accent); border-color: var(--accent); color: #fff; }
button.danger { color: var(--bad); }
input[type=text], input[type=datetime-local], textarea { width: 100%; background: var(--surface); border: 1px solid var(--line); border-radius: 8px; padding: 6px 10px; }
textarea { min-height: 80px; resize: vertical; }
label { display: block; font-size: 13px; color: var(--ink-2); margin: 10px 0 4px; }
label.inline { display: flex; gap: 8px; align-items: center; }
.row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; margin-top: 12px; }
.muted { color: var(--ink-3); } .small { font-size: 13px; }
.error { color: var(--bad); min-height: 1.4em; }
.pill { font-size: 12px; padding: 1px 8px; border-radius: 99px; background: var(--line); color: var(--ink-2); }
.pill.live { background: var(--live-bg); color: var(--live); }
.message { white-space: pre-wrap; margin: 6px 0 10px; }
.hbar { display: grid; grid-template-columns: minmax(80px, 30%) 1fr auto; gap: 8px; align-items: center; margin: 6px 0; }
.track { height: 12px; background: var(--line); border-radius: 6px; overflow: hidden; }
.fill { height: 12px; background: var(--bar); }
.n { font-variant-numeric: tabular-nums; color: var(--ink-2); white-space: nowrap; }
:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
</head>
<body>
<header><h1><span>CutCraft</span> messages and votes</h1><a href="/">Crash reports</a></header>
<main>
  <section class="card" id="signin" hidden>
    <h2>Sign in first</h2>
    <p class="muted">Sign in on the <a href="/">crash reports page</a> in this tab, then come back with its <b>Messages and votes</b> link.</p>
  </section>
  <section id="app" hidden>
    <div class="card">
      <h2>Show a message or a vote in CutCraft</h2>
      <p class="small muted">CutCraft shows it once at startup to everyone running 0.6.19 to 0.6.21 with messages switched on, and keeps it under Help. Publishing ends the one showing now. Publishing again with the same short name changes it and keeps its votes.</p>
      <label for="f-title">Title</label>
      <input type="text" id="f-title" maxlength="80" placeholder="What should Cut Craft be called?">
      <label for="f-body">Message</label>
      <textarea id="f-body" maxlength="1200" placeholder="Just an opinion poll: we'd like to know what you think. Nothing has been decided."></textarea>
      <label for="f-choices">Choices, one per line (leave empty for a plain message, at most 6)</label>
      <textarea id="f-choices" placeholder="CutCraft&#10;PowerWhittle"></textarea>
      <label for="f-id">Short name (lower-case, no spaces)</label>
      <input type="text" id="f-id" maxlength="40" placeholder="name-poll">
      <label for="f-ends">Stop showing on (optional)</label>
      <input type="datetime-local" id="f-ends">
      <label class="inline"><input type="checkbox" id="f-results" checked> Show voters how the vote is going after they vote</label>
      <div class="row"><button class="primary" id="publish">Publish</button><span class="error small" id="f-error" role="alert"></span></div>
    </div>
    <div id="polls"></div>
  </section>
</main>
<script src="/votes.js"></script>
</body>
</html>`;

export const VOTES_SCRIPT = String.raw`(() => {
const $ = (id) => document.getElementById(id);
let token = null;
try { token = sessionStorage.getItem('cutcraft-crash-token'); } catch {}

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
  const headers = { 'content-type': 'application/json' };
  if (token) headers.authorization = 'Bearer ' + token;
  const r = await fetch(path, { ...opts, headers });
  if (r.status === 401) { $('app').hidden = true; $('signin').hidden = false; throw new Error('signed out'); }
  const body = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(body.error || ('Request failed (' + r.status + ')'));
  return body;
}
const when = (iso) => iso ? new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : '';

function pollCard(p) {
  const status = p.live ? el('span', { class: 'pill live', text: 'Showing now' })
    : el('span', { class: 'pill', text: p.ended_at ? 'Ended ' + when(p.ended_at) : 'Ended ' + when(p.ends_at) });
  const card = el('div', { class: 'card' },
    el('div', { class: 'row', style: 'margin-top:0' }, el('h2', { text: p.title }), status),
    el('div', { class: 'small muted', text: p.id + ' · published ' + when(p.created_at) + (p.ends_at && p.live ? ' · stops ' + when(p.ends_at) : '') }),
    el('p', { class: 'message', text: p.body }));
  if (p.choices.length) {
    const most = Math.max(1, ...p.totals);
    card.append(el('h3', { text: p.voters + (p.voters === 1 ? ' vote' : ' votes') + (p.show_results ? '' : ' (voters do not see these)') }));
    p.choices.forEach((c, i) => {
      const n = p.totals[i];
      const pct = p.voters ? Math.round((n / p.voters) * 100) : 0;
      card.append(el('div', { class: 'hbar' }, el('span', { text: c }),
        el('div', { class: 'track' }, el('div', { class: 'fill', style: 'width:' + (n / most) * 100 + '%' })),
        el('span', { class: 'n', text: n + ' (' + pct + '%)' })));
    });
  } else {
    card.append(el('div', { class: 'small muted', text: 'A plain message, no vote.' }));
  }
  const row = el('div', { class: 'row' });
  if (p.live) row.append(el('button', { text: 'Stop showing it', onclick: () => act('POST', p.id + '/end') }));
  row.append(el('button', { text: 'Copy into the form', onclick: () => fill(p) }));
  row.append(el('button', { class: 'danger', text: 'Delete it and its votes', onclick: () => {
    if (confirm('Delete "' + p.title + '" and all its votes? This cannot be undone.')) act('DELETE', p.id);
  } }));
  card.append(row);
  return card;
}
function fill(p) {
  $('f-title').value = p.title; $('f-body').value = p.body; $('f-choices').value = p.choices.join('\n');
  $('f-id').value = p.id; $('f-results').checked = p.show_results;
  // the form's local date and time, so a scheduled end is kept unless it is cleared
  const end = p.live && p.ends_at ? new Date(p.ends_at) : null;
  $('f-ends').value = end ? new Date(end.getTime() - end.getTimezoneOffset() * 60000).toISOString().slice(0, 16) : '';
  scrollTo({ top: 0, behavior: 'smooth' });
}
async function act(method, path) {
  try { await api('/admin/api/polls/' + path, { method }); load(); } catch (e) { if (e.message !== 'signed out') alert(e.message); }
}
async function load() {
  const { polls } = await api('/admin/api/polls');
  $('signin').hidden = true; $('app').hidden = false;
  const box = $('polls');
  box.replaceChildren(...polls.map(pollCard));
  if (!polls.length) box.append(el('p', { class: 'muted', text: 'Nothing published yet.' }));
}
$('publish').addEventListener('click', async () => {
  $('f-error').textContent = '';
  const ends = $('f-ends').value;
  const body = {
    id: $('f-id').value.trim(), title: $('f-title').value, body: $('f-body').value,
    choices: $('f-choices').value.split('\n').map((s) => s.trim()).filter(Boolean),
    show_results: $('f-results').checked, ends_at: ends ? new Date(ends).toISOString() : null,
  };
  try { await api('/admin/api/polls', { method: 'POST', body: JSON.stringify(body) }); load(); }
  catch (e) { if (e.message !== 'signed out') $('f-error').textContent = e.message; }
});
load().catch(() => {});
})();`;
