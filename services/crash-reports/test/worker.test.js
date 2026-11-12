import { test } from 'node:test';
import assert from 'node:assert/strict';
import { d1 } from './d1-shim.js';
import { handle } from '../src/index.js';
import { cleanup } from '../src/store.js';

const TOKEN = 'a-long-admin-token-for-tests';
// These tests are about storage, so the junk guards (format, proof of work) are off here;
// test/guard.test.js covers them with the production defaults.
const env = (over = {}) => ({ DB: d1('schema.sql'), ADMIN_TOKEN: TOKEN, POW_BITS: '0', STRICT_LOG: '0', ...over });
const at = (iso) => new Date(iso);
let n = 0;
const report = (over = {}) => ({
  version: '0.6.4', os: 'Linux x86_64', title: 'Crashed while calculating a toolpath',
  signature: 'sig-aaaaaaaa', log: 'panicked at crates/cam/src/pocket.rs:120:9', install_id: `install-${++n}-xxxx`, ...over,
});
const post = (e, body, now, raw) => handle(new Request('https://w.example/v1/crash', {
  method: 'POST', headers: { 'content-type': 'application/json' }, body: raw ?? JSON.stringify(body),
}), e, now);
const admin = (e, method, path, body, now, token = TOKEN) => handle(new Request(`https://w.example${path}`, {
  method, headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
  body: body ? JSON.stringify(body) : undefined,
}), e, now);
const rows = (e, sql) => e.DB.raw.prepare(sql).all();

test('a report is accepted with 202 and grouped by signature', async () => {
  const e = env();
  const now = at('2026-11-12T10:00:00Z');
  assert.equal((await post(e, report(), now)).status, 202);
  assert.equal((await post(e, report(), now)).status, 202);
  assert.equal((await post(e, report({ signature: 'sig-bbbbbbbb', title: 'Other' }), now)).status, 202);
  const crashes = rows(e, 'SELECT signature, count, installs, status FROM crashes ORDER BY signature');
  assert.deepEqual(crashes.map((c) => [c.signature, c.count, c.installs, c.status]),
    [['sig-aaaaaaaa', 2, 2, 'new'], ['sig-bbbbbbbb', 1, 1, 'new']]);
});

test('the install id is never stored as sent', async () => {
  const e = env();
  await post(e, report({ install_id: 'my-very-own-install-id' }), at('2026-11-12T10:00:00Z'));
  const dump = JSON.stringify([rows(e, 'SELECT * FROM senders'), rows(e, 'SELECT * FROM seen'), rows(e, 'SELECT * FROM samples')]);
  assert.ok(!dump.includes('my-very-own-install-id'));
});

test('the same install is limited to one report an hour', async () => {
  const e = env();
  const id = 'same-install-0001';
  assert.equal((await post(e, report({ install_id: id }), at('2026-11-12T10:00:00Z'))).status, 202);
  const r = await post(e, report({ install_id: id }), at('2026-11-12T10:20:00Z'));
  assert.equal(r.status, 429);
  assert.equal(r.headers.get('retry-after'), String(40 * 60));
  assert.equal((await post(e, report({ install_id: id }), at('2026-11-12T11:00:00Z'))).status, 202);
});

test('the daily cap refuses reports until midnight UTC', async () => {
  const e = env({ DAILY_LIMIT: '2' });
  const now = at('2026-11-12T22:00:00Z');
  await post(e, report(), now);
  await post(e, report(), now);
  const r = await post(e, report(), now);
  assert.equal(r.status, 429);
  assert.equal(r.headers.get('retry-after'), String(2 * 3600));
  assert.equal((await post(e, report(), at('2026-11-13T00:00:01Z'))).status, 202);
});

test('bad, oversized and wrong-method requests are refused', async () => {
  const e = env();
  const now = at('2026-11-12T10:00:00Z');
  assert.equal((await post(e, null, now, '{not json')).status, 400);
  assert.equal((await post(e, report({ signature: 'x' }), now)).status, 400);
  assert.equal((await post(e, report({ log: 'x'.repeat(65 * 1024) }), now)).status, 413);
  assert.equal((await post(e, null, now, 'x'.repeat(80 * 1024))).status, 413);
  assert.equal((await handle(new Request('https://w.example/v1/crash'), e, now)).status, 405);
  assert.equal(rows(e, 'SELECT * FROM crashes').length, 0);
});

test('only three example logs are kept, and a new version replaces the oldest', async () => {
  const e = env();
  const t = (m) => at(`2026-11-12T10:0${m}:00Z`);
  for (let i = 0; i < 5; i++) await post(e, report({ log: `log ${i}` }), t(i));
  assert.deepEqual(rows(e, 'SELECT log FROM samples ORDER BY id').map((r) => r.log), ['log 0', 'log 1', 'log 2']);
  await post(e, report({ version: '0.6.5', log: 'newer' }), t(6));
  assert.deepEqual(rows(e, 'SELECT log FROM samples ORDER BY id').map((r) => r.log), ['log 1', 'log 2', 'newer']);
  assert.equal(rows(e, 'SELECT count FROM crashes')[0].count, 6);
});

test('marking handled deletes the logs; a repeat crash comes back and keeps logs again', async () => {
  const e = env();
  await post(e, report(), at('2026-11-12T10:00:00Z'));
  const r = await admin(e, 'POST', '/admin/api/crashes/sig-aaaaaaaa/handled', { note: 'fixed in 0.6.5' }, at('2026-11-12T11:00:00Z'));
  assert.equal(r.status, 200);
  const d = await r.json();
  assert.equal(d.status, 'handled');
  assert.equal(d.note, 'fixed in 0.6.5');
  assert.equal(d.samples.length, 0);
  assert.equal(rows(e, 'SELECT * FROM samples').length, 0);

  await post(e, report({ version: '0.6.5', log: 'still here' }), at('2026-11-13T10:00:00Z'));
  const back = await (await admin(e, 'GET', '/admin/api/crashes/sig-aaaaaaaa')).json();
  assert.equal(back.status, 'back');
  assert.equal(back.samples[0].log, 'still here');
  assert.equal(back.count, 2);
});

test('turning a crash into a GitHub issue deletes its logs and stops keeping new ones', async () => {
  const e = env();
  await post(e, report(), at('2026-11-12T10:00:00Z'));
  const bad = await admin(e, 'POST', '/admin/api/crashes/sig-aaaaaaaa/issue', { url: 'https://example.com/x' });
  assert.equal(bad.status, 400);
  const r = await admin(e, 'POST', '/admin/api/crashes/sig-aaaaaaaa/issue', { url: 'https://github.com/NoobyNull/Cut-Craft/issues/300' });
  assert.equal(r.status, 200);
  assert.equal((await r.json()).issue_url, 'https://github.com/NoobyNull/Cut-Craft/issues/300');
  await post(e, report(), at('2026-11-12T12:00:00Z'));
  assert.equal(rows(e, 'SELECT * FROM samples').length, 0);
  assert.equal(rows(e, 'SELECT count, status FROM crashes')[0].count, 2);
  assert.equal(rows(e, 'SELECT status FROM crashes')[0].status, 'issue');
});

test('reopen, delete one log, and forget a crash completely', async () => {
  const e = env();
  await post(e, report({ log: 'one' }), at('2026-11-12T10:00:00Z'));
  await post(e, report({ log: 'two' }), at('2026-11-12T10:01:00Z'));
  const id = rows(e, "SELECT id FROM samples WHERE log = 'one'")[0].id;
  assert.equal((await admin(e, 'DELETE', `/admin/api/samples/${id}`)).status, 200);
  assert.equal((await admin(e, 'DELETE', `/admin/api/samples/${id}`)).status, 404);
  await admin(e, 'POST', '/admin/api/crashes/sig-aaaaaaaa/handled', {});
  assert.equal((await (await admin(e, 'POST', '/admin/api/crashes/sig-aaaaaaaa/reopen')).json()).status, 'new');
  assert.equal((await admin(e, 'DELETE', '/admin/api/crashes/sig-aaaaaaaa')).status, 200);
  for (const t of ['crashes', 'samples', 'daily', 'seen']) assert.equal(rows(e, `SELECT * FROM ${t}`).length, 0, t);
  assert.equal((await admin(e, 'GET', '/admin/api/crashes/sig-aaaaaaaa')).status, 404);
});

test('the daily clean-up deletes logs after RETAIN_DAYS and old rate-limit rows', async () => {
  const e = env();
  await post(e, report({ log: 'old' }), at('2026-10-01T10:00:00Z'));
  await post(e, report({ log: 'recent' }), at('2026-11-10T10:00:00Z'));
  const r = await cleanup(e.DB, { retainDays: 30, statsDays: 400 }, at('2026-11-12T10:00:00Z'));
  assert.equal(r.logs, 1);
  assert.deepEqual(rows(e, 'SELECT log FROM samples').map((x) => x.log), ['recent']);
  assert.equal(rows(e, 'SELECT * FROM senders').length, 0);
  assert.equal(rows(e, 'SELECT count FROM crashes')[0].count, 2, 'counts survive clean-up');
});

test('the summary gives totals, per-day counts and breakdowns for the range', async () => {
  const e = env();
  await post(e, report({ os: 'Windows 11 x64' }), at('2026-11-01T10:00:00Z'));
  await post(e, report(), at('2026-11-12T09:00:00Z'));
  await post(e, report({ version: '0.6.3', signature: 'sig-cccccccc', title: 'Other' }), at('2026-11-12T09:30:00Z'));
  await post(e, report(), at('2026-09-01T10:00:00Z'));
  const s = await (await admin(e, 'GET', '/admin/api/summary?days=30', null, at('2026-11-12T10:00:00Z'))).json();
  assert.equal(s.total, 3);
  assert.deepEqual(s.perDay, [{ day: '2026-11-01', count: 1 }, { day: '2026-11-12', count: 2 }]);
  assert.deepEqual(s.byVersion, [{ version: '0.6.4', count: 2 }, { version: '0.6.3', count: 1 }]);
  assert.equal(s.crashes.length, 2);
  assert.equal(s.crashes.find((c) => c.signature === 'sig-aaaaaaaa').recent, 2);
  assert.equal(s.crashes.find((c) => c.signature === 'sig-aaaaaaaa').count, 3);
  assert.equal(s.held.logs, 4);
  assert.equal(s.settings.retainDays, 30);
});

test('the dashboard API needs the admin token; the page itself holds no data', async () => {
  const e = env();
  await post(e, report({ title: 'Secret project crash' }), at('2026-11-12T10:00:00Z'));
  assert.equal((await admin(e, 'GET', '/admin/api/summary', null, undefined, 'wrong')).status, 401);
  assert.equal((await handle(new Request('https://w.example/admin/api/summary'), e)).status, 401);
  assert.equal((await handle(new Request('https://w.example/admin/api/summary'), { ...e, ADMIN_TOKEN: undefined })).status, 401);
  const page = await handle(new Request('https://w.example/'), e);
  assert.equal(page.status, 200);
  assert.match(page.headers.get('content-security-policy'), /frame-ancestors 'none'/);
  assert.equal(page.headers.get('cache-control'), 'no-store');
  assert.ok(!(await page.text()).includes('Secret project'));
  const js = await handle(new Request('https://w.example/dashboard.js'), e);
  assert.match(js.headers.get('content-type'), /javascript/);
  assert.ok(!(await js.text()).includes('innerHTML'), 'logs must never be inserted as HTML');
});
