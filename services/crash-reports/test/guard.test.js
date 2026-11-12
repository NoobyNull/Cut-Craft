// Keeping junk out: CutCraft's log layout, the proof of work, the per-network allowance, the
// server's own crash ID, and refusals counted for the dashboard. Production defaults, except a
// smaller proof of work so the tests run fast.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { d1 } from './d1-shim.js';
import { handle } from '../src/index.js';
import { cleanup, summary } from '../src/store.js';
import { workText, zeroBits, crashId, checkWork } from '../src/guard.js';

const BITS = 8;
const env = (over = {}) => ({ DB: d1('schema.sql'), ADMIN_TOKEN: 't', POW_BITS: String(BITS), ...over });
const at = (iso) => new Date(iso);
const rows = (e, sql) => e.DB.raw.prepare(sql).all();
const sha = async (t) => new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(t)));

/** What CutCraft's crash_text writes. */
const crashLog = ({ version = '0.6.4', os = 'linux', file = 'crates/cam/src/pocket.rs', line = 120,
  message = 'index out of bounds: the len is 3 but the index is 7' } = {}) =>
  `Cut Craft ${version} on ${os} crashed: panicked at ${file}:${line}:9:\n${message}\n\nBacktrace:\n` +
  '   0: 0x55d0c0ffee01 - <unknown>\n   1: 0x55d0c0ffee02 - <unknown>\n\nLog:\n10:00:00.000 INFO  cut_craft: started\n';

let n = 0;
const report = (over = {}) => {
  const version = over.version ?? '0.6.4';
  return {
    version, os: 'Linux x86_64', title: 'Crashed while calculating a toolpath', signature: 'app-sent-sig',
    install_id: `install-${++n}-xxxx`, log: crashLog({ version }), ...over,
  };
};

/** Solve the proof of work the way the app will. */
async function solve(r, now, bits = BITS) {
  const day = now.toISOString().slice(0, 10);
  for (let i = 0; ; i++) {
    const nonce = i.toString(36);
    if (zeroBits(await sha(await workText(r, day, nonce))) >= bits) return nonce;
  }
}

const post = async (e, body, now, { ip = '203.0.113.7', work = true } = {}) => {
  const b = work ? { ...body, work: await solve(body, now) } : body;
  const headers = { 'content-type': 'application/json' };
  if (ip) headers['cf-connecting-ip'] = ip;
  return handle(new Request('https://w.example/v1/crash', { method: 'POST', headers, body: JSON.stringify(b) }), e, now);
};

const NOW = at('2026-11-12T10:00:00Z');

test('a real CutCraft crash log with a solved proof of work is taken', async () => {
  const e = env();
  const r = await post(e, report(), NOW);
  assert.equal(r.status, 202, JSON.stringify(await r.clone().json()));
});

test('reports that do not look like CutCraft wrote them are refused', async () => {
  const e = env();
  const bad = [
    report({ version: 'v1', log: crashLog({ version: 'v1' }) }),
    report({ log: 'hello there' }),
    report({ log: crashLog({ version: '9.9.9' }) }), // version in the log disagrees
    report({ log: crashLog().replace('\n\nBacktrace:\n', '\n\n') }),
    report({ log: crashLog().replace('panicked at ', 'fell over at ') }),
  ];
  for (const b of bad) assert.equal((await post(e, b, NOW)).status, 400, b.log.slice(0, 60));
});

test('the proof of work is required, must match the report, and is cheap to check', async () => {
  const e = env();
  assert.equal((await post(e, report(), NOW, { work: false })).status, 400);
  const r = report();
  const nonce = await solve(r, NOW, 16); // 16 bits, so a chance match on another day is 1 in 32768
  const other = { ...report(), work: nonce }; // a nonce solved for a different report
  const res = await handle(new Request('https://w.example/v1/crash', {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(other),
  }), e, NOW);
  assert.ok([400, 202].includes(res.status));
  assert.equal(await checkWork(other, nonce, 16, NOW), false, 'work is tied to its own report');
  assert.equal(await checkWork(r, nonce, 16, NOW), true);
  assert.equal(await checkWork(r, nonce, 16, at('2026-11-13T00:30:00Z')), true, "yesterday's work counts just after midnight");
  assert.equal(await checkWork(r, nonce, 16, at('2026-11-14T10:00:00Z')), false, 'older work does not');
  assert.equal(await checkWork(r, 'NOT-valid!', 16, NOW), false);
});

test('one network: two reports of the same crash a day, ten in all', async () => {
  const e = env();
  assert.equal((await post(e, report(), NOW)).status, 202);
  assert.equal((await post(e, report(), NOW)).status, 202);
  const third = await post(e, report(), NOW);
  assert.equal(third.status, 429);
  assert.equal(third.headers.get('retry-after'), String(14 * 3600));
  assert.equal((await post(e, report(), NOW, { ip: '198.51.100.9' })).status, 202, 'another network is not affected');

  for (let i = 0; i < 8; i++) {
    const msg = `different crash number ${String.fromCharCode(97 + i)}`;
    assert.equal((await post(e, report({ log: crashLog({ file: `crates/x/src/f${'abcdefgh'[i]}.rs`, message: msg }) }), NOW)).status, 202);
  }
  const eleventh = await post(e, report({ log: crashLog({ file: 'crates/x/src/z.rs' }) }), NOW);
  assert.equal(eleventh.status, 429, 'ten a day from one network');
  assert.equal((await post(e, report(), at('2026-11-13T00:00:01Z'))).status, 202, 'a new day starts fresh');
});

test('IPs are never stored, and the day salt and counts are deleted by the clean-up', async () => {
  const e = env();
  await post(e, report(), NOW, { ip: '203.0.113.77' });
  await post(e, report({ log: 'junk' }), NOW, { ip: '203.0.113.77' });
  const dump = JSON.stringify(['ip_counts', 'refusals', 'meta', 'senders', 'seen', 'samples'].map((t) => rows(e, `SELECT * FROM ${t}`)));
  assert.ok(!dump.includes('203.0.113.77'));
  assert.equal(rows(e, "SELECT * FROM meta WHERE key LIKE 'ip_salt:%'").length, 1);
  await cleanup(e.DB, { retainDays: 30, statsDays: 400 }, at('2026-11-13T03:17:00Z'));
  assert.equal(rows(e, "SELECT * FROM meta WHERE key LIKE 'ip_salt:%'").length, 0);
  assert.equal(rows(e, 'SELECT * FROM ip_counts').length, 0);
});

test('the crash ID is the same on every machine and version, and the app cannot choose it', async () => {
  const a = await crashId(crashLog({ version: '0.6.4', line: 120,
    message: 'failed to open /home/matt/Projects/sign.crv: No such file (os error 2) at 0x7ffd1234' }));
  const b = await crashId(crashLog({ version: '0.7.0', os: 'windows', line: 131,
    file: 'C:\\Users\\Kim\\.cargo\\git\\cut-craft\\crates\\cam\\src\\pocket.rs',
    message: 'failed to open C:\\Users\\Kim\\Documents\\box.crv: No such file (os error 3) at 0x1a2b' }));
  assert.match(a, /^cc-[0-9a-f]{12}$/);
  assert.equal(a, b);
  assert.notEqual(a, await crashId(crashLog({ message: 'attempt to divide by zero' })));
  assert.notEqual(a, await crashId(crashLog({ file: 'crates/cam/src/profile.rs',
    message: 'failed to open /x/y.crv: No such file (os error 2) at 0x1' })));
  assert.equal(await crashId('no panic here'), null);

  const e = env();
  await post(e, report({ signature: 'chosen-by-app-1' }), NOW, { ip: '1.1.1.1' });
  await post(e, report({ signature: 'chosen-by-app-2', version: '0.6.5', log: crashLog({ version: '0.6.5', line: 140 }) }), NOW, { ip: '1.1.1.2' });
  const crashes = rows(e, 'SELECT signature, count, installs FROM crashes');
  assert.equal(crashes.length, 1, 'both installs land on one crash');
  assert.match(crashes[0].signature, /^cc-/);
  assert.equal(crashes[0].count, 2);
  assert.equal(crashes[0].installs, 2);
});

test('refused reports are counted by reason and shown in the summary', async () => {
  const e = env();
  await post(e, report({ log: 'junk' }), NOW, { ip: '203.0.113.1' });
  await post(e, report({ log: 'junk' }), NOW, { ip: '203.0.113.2' });
  await post(e, report(), NOW, { work: false });
  await handle(new Request('https://w.example/v1/crash', { method: 'POST', body: '{nope' }), e, NOW);
  const s = await summary(e.DB, 7, NOW);
  const by = Object.fromEntries(s.refused.map((r) => [r.reason, r]));
  assert.equal(by['not a CutCraft log'].count, 2);
  assert.equal(by['not a CutCraft log'].networks, 2);
  assert.equal(by['no proof of work'].count, 1);
  assert.equal(by['not JSON'].count, 1);
  assert.equal(s.total, 0, 'refused reports are not crash reports');
});
