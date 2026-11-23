import { test } from 'node:test';
import assert from 'node:assert/strict';
import { d1, fakeFetch } from './d1-shim.js';
import { handle } from '../src/index.js';
import { checkAll } from '../src/store.js';

const SPE_PAGE = 'https://spetools.com/pages/spetool-tool-file-database';
const IDC_PAGE = 'https://www.idcwoodcraft.com/pages/database-downloads';
const AMANA_PAGE = 'https://www.amanatool.com/cnc-tool-databases-for-fusion-360tm-vectric';
const TT_PAGE = 'https://toolstoday.com/learn/download-free-vectric-tool-files-for-amana-cnc-bits';
const SPE_FILE = 'https://cdn.shopify.com/s/files/1/SpeTool-Vectric-Tool-Files-Database-20251015.vtdb?v=1';
const IDC_FILE = 'https://app.filemonk.io/rails/blob/IDCWoodcraftVectricToolDatabaseLibrary.vtdb';
const TT_FILE = 'https://toolstoday.com/content/cnc-libary/Amana-Tool-Vectric-09-25-2024.tool';

function vendors(over = {}) {
  return fakeFetch({
    [SPE_PAGE]: { body: `<a href="${SPE_FILE}">Vectric</a>` },
    [SPE_FILE]: { body: 'SQLITE' },
    [IDC_PAGE]: { body: `<a href="${IDC_FILE}">Vectric</a>` },
    [IDC_FILE]: { status: 404, body: '<!DOCTYPE html><title>Download link not available</title>' },
    [TT_PAGE]: { body: '<a href="/content/cnc-libary/Amana-Tool-Vectric-09-25-2024.tool">t</a>' },
    [TT_FILE]: { body: 'binary tool file' },
    ...over,
  });
}

const env = () => ({ DB: d1('schema.sql', 'seed.sql'), ADMIN_TOKEN: 's3cret-token' });
const get = (e, path) => handle(new Request(`https://w.example${path}`), e);
const admin = (e, method, path, body, token = 's3cret-token', f) =>
  handle(new Request(`https://w.example${path}`, {
    method,
    headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  }), e, f);

test('a weekly check records each vendor and never contacts the blocked one', async () => {
  const e = env();
  const f = vendors();
  const r = await checkAll(e.DB, f, new Date('2026-11-16T14:00:00Z'));
  assert.ok(r.ran);
  const by = Object.fromEntries(r.results.map((x) => [x.id, x]));
  assert.equal(by['spetool-vectric'].status, 'ok');
  assert.equal(by['spetool-vectric'].changed, true);
  assert.equal(by['idc-woodcraft-vectric'].status, 'download-broken');
  assert.equal(by['amana-vectric'].status, 'manual');
  assert.equal(by['amana-vectric-toolstoday'].status, 'ok');
  assert.ok(!f.calls.includes(AMANA_PAGE));

  const list = await (await get(e, '/v1/tool-databases')).json();
  const spe = list.sources.find((s) => s.id === 'spetool-vectric');
  assert.equal(spe.version, '20251015');
  assert.equal(spe.url, SPE_FILE);
  assert.match(spe.sha256, /^[0-9a-f]{64}$/);
  const tt = list.sources.find((s) => s.id === 'amana-vectric-toolstoday');
  assert.equal(tt.version, '09-25-2024');
});

test('a second check within a day is refused, the next day it runs', async () => {
  const e = env();
  await checkAll(e.DB, vendors(), new Date('2026-11-16T14:00:00Z'));
  const again = await checkAll(e.DB, vendors(), new Date('2026-11-16T20:00:00Z'));
  assert.equal(again.ran, false);
  assert.equal(again.retryAfterMs, 18 * 3600 * 1000);
  assert.ok((await checkAll(e.DB, vendors(), new Date('2026-11-17T14:00:00Z'))).ran);
});

test('an unchanged file is not a new version; a changed file is, and history keeps both', async () => {
  const e = env();
  await checkAll(e.DB, vendors(), new Date('2026-11-16T14:00:00Z'));
  const same = await checkAll(e.DB, vendors(), new Date('2026-11-23T14:00:00Z'));
  assert.equal(same.results.find((x) => x.id === 'spetool-vectric').changed, false);

  const NEW = 'https://cdn.shopify.com/s/files/1/SpeTool-Vectric-Tool-Files-Database-20260301.vtdb?v=2';
  const f = vendors({
    [SPE_PAGE]: { body: `<a href="${NEW}">Vectric</a>` },
    [NEW]: { body: new TextEncoder().encode('SQLite format 3\u0000 a newer file') },
  });
  const later = await checkAll(e.DB, f, new Date('2026-11-30T14:00:00Z'));
  assert.equal(later.results.find((x) => x.id === 'spetool-vectric').changed, true);
  const h = await (await get(e, '/v1/tool-databases/spetool-vectric/history')).json();
  assert.deepEqual(h.versions.map((v) => v.version), ['20260301', '20251015']);
});

test('a broken check keeps the last good link and says what is wrong', async () => {
  const e = env();
  await checkAll(e.DB, vendors(), new Date('2026-11-16T14:00:00Z'));
  await checkAll(e.DB, vendors({ [SPE_PAGE]: { body: '<p>moved</p>' } }), new Date('2026-11-23T14:00:00Z'));
  const spe = await (await get(e, '/v1/tool-databases/spetool-vectric')).json();
  assert.equal(spe.status, 'link-missing');
  assert.equal(spe.url, SPE_FILE);
});

test('a Cloudflare challenge is reported as blocked', async () => {
  const e = env();
  await admin(e, 'PUT', '/admin/sources/amana-vectric', { mode: 'auto', link_match: '\\.vtdb' });
  const f = vendors({ [AMANA_PAGE]: { status: 403, body: '<title>Just a moment...</title>' } });
  const r = await checkAll(e.DB, f, new Date('2026-11-16T14:00:00Z'));
  assert.equal(r.results.find((x) => x.id === 'amana-vectric').status, 'blocked');
});

test('the public cannot change anything', async () => {
  const e = env();
  for (const [m, p] of [['POST', '/admin/check'], ['PUT', '/admin/sources/x'], ['DELETE', '/admin/sources/spetool-vectric']]) {
    assert.equal((await admin(e, m, p, { vendor: 'x' }, 'wrong')).status, 401);
    assert.equal((await handle(new Request(`https://w.example${p}`, { method: m }), e)).status, 401);
  }
  assert.equal((await handle(new Request('https://w.example/v1/tool-databases', { method: 'POST' }), e)).status, 405);
  assert.equal((await admin({ ...e, ADMIN_TOKEN: undefined }, 'POST', '/admin/check', null, '')).status, 401);
});

test('admin check now: runs once, then 429 with Retry-After', async () => {
  const e = env();
  assert.equal((await admin(e, 'POST', '/admin/check', null, undefined, vendors())).status, 200);
  const r = await admin(e, 'POST', '/admin/check', null, undefined, vendors());
  assert.equal(r.status, 429);
  assert.ok(Number(r.headers.get('retry-after')) > 23 * 3600);
});

test('admin sets Amana by hand; repeating the same link is not a new version', async () => {
  const e = env();
  const body = { url: 'https://www.amanatool.com/media/Amana-Vectric-v12.25.vtdb', version: '12.25' };
  const r1 = await (await admin(e, 'PUT', '/admin/sources/amana-vectric/link', body)).json();
  assert.equal(r1.changed, true);
  assert.equal(r1.source.status, 'manual');
  assert.equal(r1.source.filename, 'Amana-Vectric-v12.25.vtdb');
  const r2 = await (await admin(e, 'PUT', '/admin/sources/amana-vectric/link', body)).json();
  assert.equal(r2.changed, false);
  assert.equal((await admin(e, 'PUT', '/admin/sources/amana-vectric/link', { url: 'http://x.com/a.vtdb' })).status, 400);
});

test('admin adds a vendor; bad ids and patterns are refused', async () => {
  const e = env();
  const ok = await admin(e, 'PUT', '/admin/sources/whiteside-vectric', {
    vendor: 'Whiteside', format: 'vectric-vtdb', page_url: 'https://www.whitesiderouterbits.com/pages/vectric-tool-files', link_match: '\\.vtdb',
  });
  assert.equal(ok.status, 200);
  assert.equal((await ok.json()).status, 'new');
  assert.equal((await admin(e, 'PUT', '/admin/sources/Bad_Id', { vendor: 'x' })).status, 404);
  assert.equal((await admin(e, 'PUT', '/admin/sources/new-one', { vendor: 'x', format: 'f', page_url: 'https://x', link_match: '(' })).status, 400);
  assert.equal((await admin(e, 'PUT', '/admin/sources/new-two', { vendor: 'x', format: 'f', page_url: 'https://x' })).status, 400);
  assert.equal((await admin(e, 'DELETE', '/admin/sources/whiteside-vectric')).status, 200);
  const list = await (await get(e, '/v1/tool-databases')).json();
  assert.ok(!list.sources.some((s) => s.id === 'whiteside-vectric'));
});

test('the page lists vendors, escapes their text, and the API allows any origin', async () => {
  const e = env();
  await admin(e, 'PUT', '/admin/sources/spetool-vectric', { vendor: 'Spe<script>' });
  const html = await (await get(e, '/')).text();
  assert.match(html, /IDC Woodcraft/);
  assert.ok(!html.includes('<script>'));
  const r = await get(e, '/v1/tool-databases');
  assert.equal(r.headers.get('access-control-allow-origin'), '*');
  assert.equal((await get(e, '/v1/tool-databases/nope')).status, 404);
});

test('a hand-set vendor with no link yet asks for one', async () => {
  const e = env();
  await checkAll(e.DB, vendors(), new Date('2026-11-16T14:00:00Z'));
  const amana = await (await get(e, '/v1/tool-databases/amana-vectric')).json();
  assert.equal(amana.status, 'needs-link');
  assert.match(await (await get(e, '/')).text(), /Link to set by hand/);
});
