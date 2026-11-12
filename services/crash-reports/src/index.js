// CutCraft crash reports: a Cloudflare Worker that takes crash reports from CutCraft installs
// whose owners agreed to send them, groups them by crash, and shows Matthew a private dashboard.
// Raw logs are deleted as soon as a crash is dealt with, and after RETAIN_DAYS regardless.
import { validate, setting, MAX_BODY_BYTES } from './rules.js';
import {
  receive, summary, detail, markHandled, markIssue, reopen, deleteSample, forget, cleanup,
} from './store.js';
import { PAGE, SCRIPT } from './page.js';
import {
  AuthError, keyCount, listKeys, removeKey, registerOptions, register, loginOptions, login,
  startSession, hasSession, endSession,
} from './auth.js';

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

/**
 * Who is asking: 'session' (signed in with a security key), 'token' (the admin token, accepted
 * only while no security key is registered), or null.
 */
async function adminFrom(request, env, now) {
  if (await hasSession(request, env.DB, now)) return 'session';
  const bearer = (request.headers.get('authorization') ?? '').replace(/^Bearer\s+/i, '');
  if (bearer && sameToken(bearer, env.ADMIN_TOKEN) && (await keyCount(env.DB)) === 0) return 'token';
  return null;
}

const fail = (e) => (e instanceof AuthError ? problem(e.status, e.message) : problem(400, e.message));
const readJson = (request) => request.json().catch(() => ({}));

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

async function adminApi(request, env, path, url, now, who) {
  const method = request.method;
  if (path === '/admin/api/keys') {
    if (method === 'GET') return json({ keys: await listKeys(env.DB) });
    if (method !== 'POST') return problem(405, 'Use GET or POST.');
    await register(env.DB, url, await readJson(request), now);
    // the first key, added with the token: sign in with a session so the token is no longer needed
    const extra = who === 'token' ? { 'set-cookie': await startSession(env.DB, now) } : {};
    return json({ keys: await listKeys(env.DB) }, 200, extra);
  }
  if (method === 'POST' && path === '/admin/api/keys/options') {
    return json(await registerOptions(env.DB, url, now));
  }
  const key = path.match(/^\/admin\/api\/keys\/([A-Za-z0-9_-]{1,1400})$/);
  if (key && method === 'DELETE') {
    if (!(await removeKey(env.DB, key[1]))) return problem(404, 'No such key.');
    return json({ keys: await listKeys(env.DB) });
  }
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
  if (path.startsWith('/auth/')) {
    try {
      if (method === 'GET' && path === '/auth/status') {
        return json({ keys: await keyCount(env.DB), signedIn: await hasSession(request, env.DB, now) });
      }
      if (method !== 'POST') return problem(405, 'Use POST.', { allow: 'POST' });
      if (path === '/auth/options') return json(await loginOptions(env.DB, url, now));
      if (path === '/auth/verify') {
        return json({ ok: true }, 200, { 'set-cookie': await login(env.DB, url, await readJson(request), now) });
      }
      if (path === '/auth/logout') return json({ ok: true }, 200, { 'set-cookie': await endSession(request, env.DB) });
    } catch (e) {
      return fail(e);
    }
    return problem(404, 'Nothing here.');
  }
  if (path.startsWith('/admin/api/')) {
    const who = await adminFrom(request, env, now);
    if (!who) return problem(401, 'Not signed in.');
    // a session rides on a cookie, so a change must come from this page, not another site
    if (who === 'session' && method !== 'GET' && request.headers.get('origin') !== url.origin) {
      return problem(403, 'Changes must come from the dashboard itself.');
    }
    try {
      return await adminApi(request, env, path, url, now, who);
    } catch (e) {
      return fail(e);
    }
  }
  return problem(404, 'Nothing here.');
}

export default {
  fetch: (request, env) => handle(request, env),
  // daily clean-up (see wrangler.toml)
  scheduled: (_event, env, ctx) => ctx.waitUntil(cleanup(env.DB, settings(env))),
};
