import { test } from 'node:test';
import assert from 'node:assert/strict';
import { findLink, fileName, versionFrom, isBotCheck, looksLike, isNewVersion, waitBeforeCheck, DAY_MS } from '../src/rules.js';

test('finds the Vectric link among the other formats and decodes &amp;', () => {
  const html = `<a href="https://cdn.shopify.com/f/SpeTool-Fusion-360-251015.tools?v=1">x</a>
    <a href='https://cdn.shopify.com/f/SpeTool-Vectric-Tool-Files-Database-20251015.vtdb?v=17&amp;x=2'>y</a>`;
  assert.equal(
    findLink(html, 'https://spetools.com/pages/a', 'cdn\\.shopify\\.com/.*Vectric[^/]*\\.vtdb'),
    'https://cdn.shopify.com/f/SpeTool-Vectric-Tool-Files-Database-20251015.vtdb?v=17&x=2',
  );
});

test('relative links are made absolute against the vendor page', () => {
  const html = '<a href="/content/cnc-libary/Amana-Tool-Vectric-09-25-2024.tool">t</a>';
  assert.equal(
    findLink(html, 'https://toolstoday.com/learn/page', 'Amana-Tool-Vectric[^/]*\\.tool$'),
    'https://toolstoday.com/content/cnc-libary/Amana-Tool-Vectric-09-25-2024.tool',
  );
});

test('no matching link gives null', () => {
  assert.equal(findLink('<a href="/a.pdf">', 'https://x.com/', '\\.vtdb'), null);
});

test('file name drops the query; version comes from the pattern', () => {
  const name = fileName('https://cdn.shopify.com/f/SpeTool-Vectric-Tool-Files-Database-20251015.vtdb?v=1');
  assert.equal(name, 'SpeTool-Vectric-Tool-Files-Database-20251015.vtdb');
  assert.equal(versionFrom(name, '(\\d{8})\\.vtdb'), '20251015');
  assert.equal(versionFrom(name, null), null);
  assert.equal(versionFrom('IDC.vtdb', '(\\d{8})'), null);
});

test('a Cloudflare challenge is a bot check, a normal 403 is not', () => {
  assert.ok(isBotCheck(403, '<title>Just a moment...</title>'));
  assert.ok(!isBotCheck(403, 'Forbidden'));
  assert.ok(!isBotCheck(200, '<title>Just a moment...</title>'));
});

test('an HTML error page is not a tool database, even with status 200', () => {
  const html = new TextEncoder().encode('<!DOCTYPE html><title>Download link not available</title>');
  assert.ok(!looksLike('vectric-vtdb', html));
  assert.ok(!looksLike('vectric-tool', html));
  assert.ok(looksLike('vectric-vtdb', new TextEncoder().encode('SQLite format 3\u0000...')));
  assert.ok(!looksLike('vectric-vtdb', new TextEncoder().encode('PK\u0003\u0004 zip')));
  assert.ok(looksLike('vectric-tool', new Uint8Array([1, 2, 3])));
});

test('same fingerprint under a new address is not a new version', () => {
  assert.ok(!isNewVersion({ sha256: 'aa', url: 'u?v=1' }, { sha256: 'aa', url: 'u?v=2' }));
  assert.ok(isNewVersion({ sha256: 'aa' }, { sha256: 'bb' }));
  assert.ok(isNewVersion({ sha256: null }, { sha256: 'bb' }));
});

test('checks are at most one a day', () => {
  const now = Date.parse('2026-11-11T12:00:00Z');
  assert.equal(waitBeforeCheck(null, now), 0);
  assert.equal(waitBeforeCheck('2026-11-10T12:00:00Z', now), 0);
  assert.equal(waitBeforeCheck('2026-11-11T06:00:00Z', now), (DAY_MS * 3) / 4);
});
