import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { d1 } from './d1-shim.js';
import { handle } from '../src/index.js';
import { bugWorkText, issueBody, quiet, validateBug } from '../src/bugs.js';

const now = new Date('2026-11-28T10:00:00Z');
const PNG = Buffer.from([0x89, 0x50, 0x4e, 0x47]).toString('base64');

/** A pretend GitHub that records each call and answers as the real one does. */
function fakeGitHub({ labelRefused = false, failIssues = false } = {}) {
  const calls = [];
  const fetch = async (url, init) => {
    const body = JSON.parse(init.body);
    calls.push({ url, method: init.method, auth: init.headers.authorization, body });
    const answer = (status, json) => new Response(JSON.stringify(json), { status });
    if (url.includes('/contents/')) {
      const path = url.split('/contents/')[1];
      return answer(201, { content: { html_url: `https://github.com/pw/pics/blob/main/${path}` } });
    }
    if (failIssues) return answer(403, { message: 'Resource not accessible by personal access token' });
    if (labelRefused && body.labels) return answer(422, { message: 'Validation Failed' });
    return answer(201, { html_url: 'https://github.com/pw/app/issues/42' });
  };
  return { calls, fetch };
}

const env = (gh, over = {}) => ({
  DB: d1('schema.sql'), POW_BITS: '0', BUG_TOKEN: 'issue-token', BUG_PICTURE_TOKEN: 'picture-token', BUG_REPO: 'pw/app',
  BUG_PICTURE_REPO: 'pw/pics', GITHUB_FETCH: gh.fetch, ...over,
});
const bug = (over = {}) => ({
  title: 'Apply does nothing', text: 'Clicked Apply.\n\n### Where\n\n- Bit Library',
  install_id: 'install-0001-abcd', pictures: [{ name: 'screenshot.png', data: PNG }], ...over,
});
const send = (e, body, ip = '', at = now) => handle(new Request('https://w.example/v1/bug', {
  method: 'POST', headers: { 'content-type': 'application/json', 'cf-connecting-ip': ip },
  body: typeof body === 'string' ? body : JSON.stringify(body),
}), e, at);

test('a bug report puts its pictures in the pictures repo, then opens the issue', async () => {
  const gh = fakeGitHub();
  const r = await send(env(gh), bug({ reporter: 'Ray-1961' }));
  assert.equal(r.status, 201);
  assert.deepEqual(await r.json(), { url: 'https://github.com/pw/app/issues/42' });
  const [picture, issue] = gh.calls;
  assert.equal(picture.method, 'PUT');
  assert.match(picture.url, /^https:\/\/api\.github\.com\/repos\/pw\/pics\/contents\/2026-11-28\/[0-9a-f]{12}\/1-screenshot\.png$/);
  assert.equal(picture.body.content, PNG);
  assert.equal(picture.auth, 'Bearer picture-token');
  assert.equal(issue.auth, 'Bearer issue-token');
  assert.equal(issue.url, 'https://api.github.com/repos/pw/app/issues');
  assert.equal(issue.body.title, 'Apply does nothing');
  assert.deepEqual(issue.body.labels, ['bug']);
  assert.match(issue.body.body, /!\[screenshot\.png\]\(https:\/\/github\.com\/pw\/pics\/blob\/main\/.*1-screenshot\.png\?raw=true\)/);
  assert.ok(!issue.body.body.includes('Ray-1961'), 'a name sent is never put in the issue');
  assert.match(issue.body.body, /_Sent anonymously from PowerWhittle's Report a Bug\._\n$/);
});

test('a repository without the bug label still gets the issue', async () => {
  const gh = fakeGitHub({ labelRefused: true });
  assert.equal((await send(env(gh), bug({ pictures: [] }))).status, 201);
  assert.equal(gh.calls.length, 2);
  assert.equal(gh.calls[1].body.labels, undefined);
});

test('GitHub refusing the issue is a 502 that says why', async () => {
  const r = await send(env(fakeGitHub({ failIssues: true })), bug());
  assert.equal(r.status, 502);
  assert.match((await r.json()).error, /GitHub answered 403/);
});

test('a report GitHub refused does not use up the network\'s allowance', async () => {
  const e = env(fakeGitHub({ failIssues: true }), { BUG_IP_DAILY: '1' });
  assert.equal((await send(e, bug(), '203.0.113.9')).status, 502);
  e.GITHUB_FETCH = fakeGitHub().fetch;
  assert.equal((await send(e, bug(), '203.0.113.9')).status, 201, 'the retry is taken');
  assert.equal((await send(e, bug(), '203.0.113.9')).status, 429, 'and then counted');
});

test('without its tokens the service says bug reports are not set up', async () => {
  const r = await send(env(fakeGitHub(), { BUG_TOKEN: undefined }), bug());
  assert.equal(r.status, 503);
});

test('the report text cannot ping anyone', () => {
  assert.equal(quiet('ask @someone or mail a@b.c'), 'ask @​someone or mail a@​b.c');
  const body = issueBody({ text: 'cc @everyone' }, []);
  assert.ok(body.includes('@​everyone'));
  assert.ok(body.trimEnd().endsWith("_Sent anonymously from PowerWhittle's Report a Bug._"));
});

test('bad fields are refused before GitHub is called', async () => {
  const gh = fakeGitHub();
  const e = env(gh);
  for (const bad of [
    bug({ title: '' }),
    bug({ title: 'two\nlines' }),
    bug({ install_id: 'x' }),
    bug({ pictures: [{ name: '../evil.png', data: PNG }] }),
    bug({ pictures: [{ name: 'a.png', data: 'not base64!' }] }),
    bug({ pictures: Array(7).fill({ name: 'a.png', data: PNG }) }),
  ]) {
    assert.equal((await send(e, bad)).status, 400, JSON.stringify(bad).slice(0, 80));
  }
  assert.equal((await send(e, '{nope')).status, 400);
  assert.equal(gh.calls.length, 0);
  assert.equal(validateBug(bug({ pictures: [{ name: 'big.png', data: 'A'.repeat(14 * 1024 * 1024) }] })).status, 413);
});

test('each network may send five bug reports a day, then waits until midnight UTC', async () => {
  const e = env(fakeGitHub());
  for (let i = 0; i < 5; i++) assert.equal((await send(e, bug(), '203.0.113.9')).status, 201);
  const r = await send(e, bug(), '203.0.113.9');
  assert.equal(r.status, 429);
  assert.equal(r.headers.get('retry-after'), String(14 * 3600));
  assert.equal((await send(e, bug(), '198.51.100.7')).status, 201, 'another network still can');
  const stored = JSON.stringify(e.DB.raw.prepare('SELECT * FROM ip_counts').all());
  assert.ok(!stored.includes('203.0.113.9'), 'never the IP itself');
});

test('bug reports do not use up a network\'s crash reports', async () => {
  const e = env(fakeGitHub(), { IP_DAILY: '1', STRICT_LOG: '0' });
  await send(e, bug(), '203.0.113.9');
  const crash = await handle(new Request('https://w.example/v1/crash', {
    method: 'POST', headers: { 'content-type': 'application/json', 'cf-connecting-ip': '203.0.113.9' },
    body: JSON.stringify({ version: '0.7.10', os: 'Linux', title: 'Crashed', signature: 'sig-aaaaaaaa', log: 'x', install_id: 'install-0002-abcd' }),
  }), e, now);
  assert.equal(crash.status, 202);
});

test('a bug report needs its proof of work', async () => {
  const e = env(fakeGitHub(), { POW_BITS: '8' });
  const b = bug();
  assert.equal((await send(e, b)).status, 400);
  let nonce = 0;
  for (; ; nonce++) {
    const hash = createHash('sha256').update(await bugWorkText(b, '2026-11-28', nonce.toString(36))).digest();
    if (hash[0] === 0) break;
  }
  assert.equal((await send(e, { ...b, work: nonce.toString(36) })).status, 201);
});

test('the proof of work hashes what the app hashes', async () => {
  // the same text as crates/bug-report/src/relay.rs bug_work_text, for the same report
  const text = await bugWorkText({ title: 't', text: 'b', install_id: 'install-1', pictures: [{ data: 'AQID' }] }, '2026-11-28', 'n');
  const sum = createHash('sha256').update('t\nb').digest('hex');
  assert.equal(text, `cutcraft-bug-v1\n2026-11-28\ninstall-1\n${sum}\nn`);
});
