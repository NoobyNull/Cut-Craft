import { test } from 'node:test';
import assert from 'node:assert/strict';
import { d1 } from './d1-shim.js';
import { handle } from '../src/index.js';

const TOKEN = 'a-long-admin-token-for-tests';
const env = (over = {}) => ({ DB: d1('schema.sql'), ADMIN_TOKEN: TOKEN, ...over });
const at = (iso) => new Date(iso);
const NOW = at('2026-11-20T10:00:00Z');
const req = (e, method, path, body, { now = NOW, token, ip } = {}) => handle(new Request(`https://w.example${path}`, {
  method,
  headers: {
    'content-type': 'application/json',
    ...(token ? { authorization: `Bearer ${token}` } : {}),
    ...(ip ? { 'cf-connecting-ip': ip } : {}),
  },
  body: body ? JSON.stringify(body) : undefined,
}), e, now);
const admin = (e, method, path, body, now = NOW) => req(e, method, path, body, { now, token: TOKEN });
const NAME_VOTE = {
  id: 'name-vote', title: 'Help choose a name', body: 'Should Cut Craft keep its name?',
  choices: ['CutCraft', 'PowerWhittle'], show_results: true,
};
const vote = (e, choice, install = 'install-0001', over = {}) =>
  req(e, 'POST', '/v1/vote', { poll: 'name-vote', choice, install_id: install, version: '0.6.19', ...over }, { ip: over.ip });

test('with nothing published the app is told there is nothing to show', async () => {
  const r = await req(env(), 'GET', '/v1/announcement');
  assert.equal(r.status, 204);
});

test('a published vote is what the app is shown', async () => {
  const e = env();
  assert.equal((await admin(e, 'POST', '/admin/api/polls', NAME_VOTE)).status, 200);
  const r = await req(e, 'GET', '/v1/announcement');
  assert.equal(r.status, 200);
  assert.deepEqual(await r.json(), {
    id: 'name-vote', title: 'Help choose a name', body: 'Should Cut Craft keep its name?',
    choices: ['CutCraft', 'PowerWhittle'], show_results: true,
  });
});

test('publishing needs the admin', async () => {
  const r = await req(env(), 'POST', '/admin/api/polls', NAME_VOTE);
  assert.equal(r.status, 401);
});

test('a plain message has no choices', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', { id: 'hello', title: 'Hello', body: 'Thanks for testing.' });
  const a = await (await req(e, 'GET', '/v1/announcement')).json();
  assert.deepEqual(a.choices, []);
});

test('bad announcements are refused with a reason', async () => {
  const e = env();
  for (const bad of [
    { ...NAME_VOTE, id: 'Has Spaces' },
    { ...NAME_VOTE, title: '' },
    { ...NAME_VOTE, choices: ['Only one'] },
    { ...NAME_VOTE, choices: ['a', 'b', 'c', 'd', 'e', 'f', 'g'] },
    { ...NAME_VOTE, choices: ['same', 'same'] },
  ]) {
    const r = await admin(e, 'POST', '/admin/api/polls', bad);
    assert.equal(r.status, 400, JSON.stringify(bad));
    assert.ok((await r.json()).error);
  }
});

test('publishing a new one ends the last, and an ended one is not shown', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  await admin(e, 'POST', '/admin/api/polls', { id: 'second', title: 'Two', body: 'b' });
  assert.equal((await (await req(e, 'GET', '/v1/announcement')).json()).id, 'second');
  assert.equal((await admin(e, 'POST', '/admin/api/polls/second/end')).status, 200);
  assert.equal((await req(e, 'GET', '/v1/announcement')).status, 204);
});

test('an announcement stops showing at its end date', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', { ...NAME_VOTE, ends_at: '2026-12-01T00:00:00Z' });
  assert.equal((await req(e, 'GET', '/v1/announcement', null, { now: at('2026-11-30T23:00:00Z') })).status, 200);
  assert.equal((await req(e, 'GET', '/v1/announcement', null, { now: at('2026-12-01T00:00:01Z') })).status, 204);
});

test('one vote per install, which can be changed', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  assert.equal((await vote(e, 0, 'install-0001')).status, 200);
  assert.equal((await vote(e, 0, 'install-0002')).status, 200);
  assert.equal((await vote(e, 1, 'install-0003')).status, 200);
  const changed = await vote(e, 1, 'install-0001');
  assert.equal(changed.status, 200);
  assert.deepEqual((await changed.json()).totals, [1, 2]);
  const results = await (await admin(e, 'GET', '/admin/api/polls')).json();
  assert.deepEqual(results.polls[0].totals, [1, 2]);
  assert.equal(results.polls[0].live, true);
});

test('totals are only sent back when the vote shows results', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', { ...NAME_VOTE, show_results: false });
  const body = await (await vote(e, 0)).json();
  assert.equal(body.ok, true);
  assert.equal(body.totals, undefined);
});

test('the install id is never stored as sent', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  await vote(e, 0, 'my-very-own-install-id');
  assert.ok(!JSON.stringify(e.DB.raw.prepare('SELECT * FROM votes').all()).includes('my-very-own-install-id'));
});

test('votes for a wrong choice, an ended vote or a plain message are refused', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  assert.equal((await vote(e, 2)).status, 400);
  assert.equal((await vote(e, -1)).status, 400);
  assert.equal((await vote(e, 0, 'bad id!')).status, 400);
  assert.equal((await vote(e, 0, 'install-0001', { poll: 'nope' })).status, 404);
  await admin(e, 'POST', '/admin/api/polls/name-vote/end');
  assert.equal((await vote(e, 0)).status, 410);
  await admin(e, 'POST', '/admin/api/polls', { id: 'hello', title: 'Hello', body: 'b' });
  assert.equal((await vote(e, 0, 'install-0001', { poll: 'hello' })).status, 400);
});

test('one network can only cast a few votes a day', async () => {
  const e = env({ VOTES_PER_IP: '3' });
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  for (let i = 0; i < 3; i++) assert.equal((await vote(e, 0, `install-000${i}`, { ip: '203.0.113.9' })).status, 200);
  const r = await vote(e, 0, 'install-0009', { ip: '203.0.113.9' });
  assert.equal(r.status, 429);
  assert.ok(Number(r.headers.get('retry-after')) > 0);
  assert.equal((await vote(e, 0, 'install-0010', { ip: '198.51.100.1' })).status, 200);
});

test('a vote and its votes can be deleted', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  await vote(e, 0);
  assert.equal((await admin(e, 'DELETE', '/admin/api/polls/name-vote')).status, 200);
  assert.equal(e.DB.raw.prepare('SELECT * FROM votes').all().length, 0);
  assert.equal((await admin(e, 'GET', '/admin/api/polls').then((r) => r.json())).polls.length, 0);
});

test('the votes page is served, holding no data', async () => {
  const r = await req(env(), 'GET', '/votes');
  assert.equal(r.status, 200);
  const html = await r.text();
  assert.ok(html.includes('/votes.js'));
  assert.equal((await req(env(), 'GET', '/votes.js')).status, 200);
});

test('votes never use up a network\'s crash-report allowance', async () => {
  const e = env({ VOTES_PER_IP: '20', IP_DAILY: '10', POW_BITS: '0', STRICT_LOG: '0' });
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  for (let i = 0; i < 12; i++) assert.equal((await vote(e, 0, `install-00${10 + i}`, { ip: '203.0.113.9' })).status, 200);
  const crash = await handle(new Request('https://w.example/v1/crash', {
    method: 'POST', headers: { 'content-type': 'application/json', 'cf-connecting-ip': '203.0.113.9' },
    body: JSON.stringify({ version: '0.6.19', os: 'Linux x86_64', title: 'Crashed', signature: 'sig-aaaaaaaa', log: 'x', install_id: 'install-crash-1' }),
  }), e, NOW);
  assert.equal(crash.status, 202);
});

test('a vote with votes keeps its choices; text can still change', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  await vote(e, 0);
  const swapped = await admin(e, 'POST', '/admin/api/polls', { ...NAME_VOTE, choices: ['PowerWhittle', 'CutCraft'] });
  assert.equal(swapped.status, 400);
  assert.equal((await admin(e, 'POST', '/admin/api/polls', { ...NAME_VOTE, title: 'New title' })).status, 200);
  const polls = (await (await admin(e, 'GET', '/admin/api/polls')).json()).polls;
  assert.deepEqual([polls[0].title, polls[0].totals], ['New title', [1, 0]]);
});

test('a vote too big is refused even without a declared length', async () => {
  const e = env();
  await admin(e, 'POST', '/admin/api/polls', NAME_VOTE);
  const big = JSON.stringify({ poll: 'name-vote', choice: 0, install_id: 'install-0001', version: '0.6.19' }) + ' '.repeat(5000);
  const r = await handle(new Request('https://w.example/v1/vote', { method: 'POST', body: big }), e, NOW);
  assert.equal(r.status, 413);
});
