// CutCraft tool-database links: a Cloudflare Worker that remembers where each bit maker
// publishes its current tool database, checks the vendors once a week, and serves the list.
// Anyone may read; only the holder of ADMIN_TOKEN may change it.
import { publicList, publicOne, historyOf, checkAll, upsertSource, setManualLink } from './store.js';
import { page } from './page.js';

const JSON_HEADERS = {
  'content-type': 'application/json; charset=utf-8',
  'access-control-allow-origin': '*',
  'cache-control': 'public, max-age=300',
};

function json(body, status = 200, extra = {}) {
  return new Response(JSON.stringify(body, null, 2), { status, headers: { ...JSON_HEADERS, ...extra } });
}

function problem(status, message, extra) {
  return json({ error: message }, status, { 'cache-control': 'no-store', ...extra });
}

/** Constant-time comparison so the token can't be guessed a character at a time. */
function sameToken(given, expected) {
  if (!expected || typeof given !== 'string') return false;
  const a = new TextEncoder().encode(given);
  const b = new TextEncoder().encode(expected);
  let diff = a.length ^ b.length;
  for (let i = 0; i < b.length; i++) diff |= (a[i] ?? 0) ^ b[i];
  return diff === 0;
}

function isAdmin(request, env) {
  const auth = request.headers.get('authorization') ?? '';
  return sameToken(auth.replace(/^Bearer\s+/i, ''), env.ADMIN_TOKEN);
}

export async function handle(request, env, fetchFn = fetch) {
  const url = new URL(request.url);
  const path = url.pathname.replace(/\/+$/, '') || '/';
  const method = request.method;

  if (method === 'OPTIONS') {
    return new Response(null, {
      status: 204,
      headers: { 'access-control-allow-origin': '*', 'access-control-allow-methods': 'GET', 'access-control-max-age': '86400' },
    });
  }

  if (method === 'GET' || method === 'HEAD') {
    if (path === '/') {
      return new Response(page(await publicList(env.DB)), {
        headers: { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'public, max-age=300' },
      });
    }
    if (path === '/v1/tool-databases') {
      return json({ sources: await publicList(env.DB) });
    }
    const one = path.match(/^\/v1\/tool-databases\/([a-z0-9-]+)$/);
    if (one) {
      const source = await publicOne(env.DB, one[1]);
      return source ? json(source) : problem(404, `No tool database called ${one[1]}.`);
    }
    const hist = path.match(/^\/v1\/tool-databases\/([a-z0-9-]+)\/history$/);
    if (hist) {
      return (await publicOne(env.DB, hist[1]))
        ? json({ id: hist[1], versions: await historyOf(env.DB, hist[1]) })
        : problem(404, `No tool database called ${hist[1]}.`);
    }
    return problem(404, 'Nothing here. The list is at /v1/tool-databases.');
  }

  // everything below changes the list
  if (!path.startsWith('/admin/')) return problem(405, 'This list is read-only.');
  if (!isAdmin(request, env)) return problem(401, 'Admin token missing or wrong.');

  try {
    if (method === 'POST' && path === '/admin/check') {
      const r = await checkAll(env.DB, fetchFn);
      if (!r.ran) {
        const secs = Math.ceil(r.retryAfterMs / 1000);
        return problem(429, `Vendors were checked less than a day ago. Try again in ${Math.ceil(secs / 3600)} hours.`, {
          'retry-after': String(secs),
        });
      }
      return json(r, 200, { 'cache-control': 'no-store' });
    }
    const src = path.match(/^\/admin\/sources\/([a-z0-9-]+)$/);
    if (src && method === 'PUT') {
      await upsertSource(env.DB, src[1], await request.json());
      return json(await publicOne(env.DB, src[1]) ?? { id: src[1], enabled: 0 }, 200, { 'cache-control': 'no-store' });
    }
    if (src && method === 'DELETE') {
      await upsertSource(env.DB, src[1], { enabled: 0 });
      return json({ id: src[1], enabled: 0 }, 200, { 'cache-control': 'no-store' });
    }
    const link = path.match(/^\/admin\/sources\/([a-z0-9-]+)\/link$/);
    if (link && method === 'PUT') {
      const changed = await setManualLink(env.DB, link[1], await request.json(), new Date().toISOString());
      return json({ changed, source: await publicOne(env.DB, link[1]) }, 200, { 'cache-control': 'no-store' });
    }
  } catch (e) {
    return problem(400, e.message);
  }
  return problem(404, 'Unknown admin action.');
}

export default {
  fetch: (request, env) => handle(request, env),
  // the weekly cron in wrangler.toml; checkAll itself refuses a second run within a day
  scheduled: (_event, env, ctx) => ctx.waitUntil(checkAll(env.DB, fetch)),
};
