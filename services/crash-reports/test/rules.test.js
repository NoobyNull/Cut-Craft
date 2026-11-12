import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validate, waitBeforeNext, setting, MAX_LOG_BYTES, HOUR_MS } from '../src/rules.js';

const good = () => ({
  version: '0.6.4', os: 'Windows 10 x64', title: 'Crashed while drawing the 3D view',
  signature: 'a1b2c3d4e5f60718', log: 'thread main panicked', install_id: '6f1c2a9e-1111-4c4c-9a9a-123456789abc',
});

test('a good report passes and unknown fields are ignored', () => {
  const r = validate({ ...good(), future_field: 1 });
  assert.equal(r.error, undefined);
  assert.deepEqual(Object.keys(r.report).sort(), ['install_id', 'log', 'os', 'signature', 'title', 'version']);
});

test('each missing or malformed field is named', () => {
  for (const f of ['version', 'os', 'title', 'signature', 'install_id', 'log']) {
    const b = good();
    delete b[f];
    assert.match(validate(b).error, new RegExp(f));
  }
  assert.match(validate({ ...good(), signature: 'short' }).error, /signature/);
  assert.match(validate({ ...good(), signature: '../../etc/passwd' }).error, /signature/);
  assert.match(validate({ ...good(), install_id: 'has spaces in it' }).error, /install_id/);
  assert.match(validate({ ...good(), title: 'x'.repeat(201) }).error, /title/);
  assert.equal(validate([]).status, 400);
  assert.equal(validate(null).status, 400);
});

test('a log over 64 KB is refused with 413, counting bytes not characters', () => {
  assert.equal(validate({ ...good(), log: 'x'.repeat(MAX_LOG_BYTES) }).error, undefined);
  assert.equal(validate({ ...good(), log: 'x'.repeat(MAX_LOG_BYTES + 1) }).status, 413);
  assert.equal(validate({ ...good(), log: 'é'.repeat(MAX_LOG_BYTES / 2 + 1) }).status, 413);
});

test('one report per install per hour', () => {
  const t = Date.parse('2026-11-12T10:00:00Z');
  assert.equal(waitBeforeNext(null, t), 0);
  assert.equal(waitBeforeNext('2026-11-12T09:30:00Z', t), HOUR_MS / 2);
  assert.equal(waitBeforeNext('2026-11-12T09:00:00Z', t), 0);
  assert.equal(waitBeforeNext('garbage', t), 0);
});

test('settings fall back when unset or nonsense', () => {
  assert.equal(setting(undefined, 30), 30);
  assert.equal(setting('45', 30), 45);
  assert.equal(setting('-1', 30), 30);
  assert.equal(setting('abc', 30), 30);
});
