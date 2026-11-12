// CutCraft crash reports: a Cloudflare Worker that takes crash reports from CutCraft installs
// whose owners agreed to send them, groups them by crash, and shows Matthew a private dashboard.
// Raw logs are deleted as soon as a crash is dealt with, and after RETAIN_DAYS regardless.
import { validate, setting, MAX_BODY_BYTES } from './rules.js';
import {
  receive, summary, detail, markHandled, markIssue, reopen, deleteSample, forget, cleanup,
} from './store.js';
import { PAGE, SCRIPT } from './page.js';

const NO_STORE = { 'cache-control': 'no-store' };

function json(body, status = 200, extra = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json; charset=utf-8', ...NO_STORE, ...extra },
  });
}
const problem = (status, error, extra) => json({ error }, status, extra);

/** Constant-time comparison so the token can't be guessed a character at a time. */
function sameToken(given, expected) {
  if (!expected || typeof given !== 'string') return false;
  const a = new TextEncoder().encode(given);
  const b = new TextEncoder().encode(expected);
  let diff = a.length ^ b.length;
  for (let i = 0; i < b.length; i++) diff |= (a[i] ?? 0) ^ b[i];
  return diff === 0;
}

const isAdmin = (request, env) =>
  sameToken((request.headers.get('authorization') ?? '').replace(/^Bearer\s+/i, ''), env.ADMIN_TOKEN);

const settings = (env) => ({
  retainDays: setting(env.RETAIN_DAYS, 30),
  statsDays: setting(env.STATS_DAYS, 400),
  dailyLimit: setting(env.DAILY_LIMIT, 2000),
});

const PAGE_HEADERS = {
  ...NO_STORE,
  'x-robots-tag': 'noindex, nofollow',
  'x-content-type-options': 'nosniff',
  'referrer-policy': 'no-referrer',
  'content-security-policy':
    "default-src 'none'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; img-src data:; " +
    "frame-ancestors 'none'; base-uri 'none'; form-action 'none'",
};

async function takeReport(request, env, now) {
  const declared = Number(request.headers.get('content-length') ?? 0);
  if (declared > MAX_BODY_BYTES) return problem(413, 'Report is too big.');
  const raw = new Uint8Array(await request.arrayBuffer());
  if (raw.length > MAX_BODY_BYTES) return problem(413, 'Report is too big.');
  let body;
  try {
    body = JSON.parse(new TextDecoder().decode(raw));
  } catch {
    return problem(400, 'Report is not valid JSON.');
  }
  const checked = validate(body);
  if (checked.error) return problem(checked.status, checked.error);
  const r = await receive(env.DB, checked.report, settings(env), now);
  if (!r.accepted) {
    const secs = Math.max(1, Math.ceil(r.retryAfterMs / 1000));
    const why = r.reason === 'daily' ? 'Too many reports today.' : 'This install already sent a report in the last hour.';
    return problem(429, why, { 'retry-after': String(secs) });
  }
  return json({ ok: true }, 202);
}

async function adminApi(request, env, path, url, now) {
  const method = request.method;
  if (method === 'GET' && path === '/admin/api/summary') {
    const days = Math.min(setting(url.searchParams.get('days'), 30), 400);
    return json({ ...(await summary(env.DB, days, now)), settings: settings(env) });
  }
  if (method === 'POST' && path === '/admin/api/cleanup') {
    return json(await cleanup(env.DB, settings(env), now));
  }
  const sample = path.match(/^\/admin\/api\/samples\/(\d+)$/);
  if (sample && method === 'DELETE') {
    return (await deleteSample(env.DB, Number(sample[1]))) ? json({ ok: true }) : problem(404, 'No such log.');
  }
  const m = path.match(/^\/admin\/api\/crashes\/([A-Za-z0-9_-]{1,64})(?:\/(handled|issue|reopen))?$/);
  if (!m) return problem(404, 'Unknown admin action.');
  const [, sig, action] = m;
  const notFound = () => problem(404, 'No crash with that signature.');
  if (!action && method === 'GET') {
    const d = await detail(env.DB, sig);
    return d ? json(d) : notFound();
  }
  if (!action && method === 'DELETE') return (await forget(env.DB, sig)) ? json({ ok: true }) : notFound();
  if (method !== 'POST') return problem(405, 'Use POST.');
  const body = await request.json().catch(() => ({}));
  let ok;
  if (action === 'handled') ok = await markHandled(env.DB, sig, typeof body.note === 'string' ? body.note.slice(0, 500) : null, now);
  else if (action === 'issue') ok = await markIssue(env.DB, sig, String(body.url ?? ''), now);
  else ok = await reopen(env.DB, sig);
  return ok ? json(await detail(env.DB, sig)) : notFound();
}

export async function handle(request, env, now = new Date()) {
  const url = new URL(request.url);
  const path = url.pathname.replace(/\/+$/, '') || '/';
  const method = request.method;

  if (path === '/v1/crash') {
    if (method !== 'POST') return problem(405, 'Send crash reports with POST.', { allow: 'POST' });
    return takeReport(request, env, now);
  }
  if (method === 'GET' && path === '/') {
    return new Response(PAGE, { headers: { 'content-type': 'text/html; charset=utf-8', ...PAGE_HEADERS } });
  }
  if (method === 'GET' && path === '/dashboard.js') {
    return new Response(SCRIPT, { headers: { 'content-type': 'text/javascript; charset=utf-8', ...PAGE_HEADERS } });
  }
  if (method === 'GET' && path === '/robots.txt') {
    return new Response('User-agent: *\nDisallow: /\n', { headers: { 'content-type': 'text/plain' } });
  }
  if (path.startsWith('/admin/api/')) {
    if (!isAdmin(request, env)) return problem(401, 'Admin token missing or wrong.');
    try {
      return await adminApi(request, env, path, url, now);
    } catch (e) {
      return problem(400, e.message);
    }
  }
  return problem(404, 'Nothing here.');
}

export default {
  fetch: (request, env) => handle(request, env),
  // daily clean-up (see wrangler.toml)
  scheduled: (_event, env, ctx) => ctx.waitUntil(cleanup(env.DB, settings(env))),
};
