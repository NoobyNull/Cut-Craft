// One check of one vendor source: fetch its page, find the link, download the file, and say
// what was found. It touches neither the database nor the clock, so tests drive it with a fake fetch.
import { findLink, fileName, versionFrom, isBotCheck, looksLike, sha256 } from './rules.js';

export const USER_AGENT =
  'CutCraft-ToolDB-LinkCheck/1.0 (weekly link check; +https://github.com/NoobyNull/Cut-Craft)';

const MAX_BYTES = 50 * 1024 * 1024;

/**
 * @param {object} source  row from `sources`: page_url, link_match, version_match, format, mode
 * @param {typeof fetch} fetchFn
 * @returns {Promise<{status: string, message: string, url?: string, filename?: string,
 *   version?: string|null, sha256?: string, bytes?: number}>}
 */
export async function checkSource(source, fetchFn) {
  if (source.mode === 'manual') {
    return { status: 'manual', message: 'Set by hand; the vendor blocks automatic checks.' };
  }
  const headers = { 'user-agent': USER_AGENT, accept: 'text/html,*/*' };

  let page;
  let html;
  try {
    page = await fetchFn(source.page_url, { headers, redirect: 'follow' });
    html = await page.text();
  } catch (e) {
    return { status: 'page-unreachable', message: `Could not open the vendor page: ${e.message}` };
  }
  if (isBotCheck(page.status, html)) {
    return {
      status: 'blocked',
      message: 'The vendor site asks visitors to prove they are human, so it cannot be checked automatically.',
    };
  }
  if (!page.ok) {
    return { status: 'page-unreachable', message: `The vendor page answered ${page.status}.` };
  }

  const url = findLink(html, source.page_url, source.link_match);
  if (!url) {
    return { status: 'link-missing', message: 'The vendor page no longer has a matching download link.' };
  }
  const filename = fileName(url);
  const version = versionFrom(filename, source.version_match);

  let file;
  let bytes;
  try {
    file = await fetchFn(url, { headers: { 'user-agent': USER_AGENT }, redirect: 'follow' });
    const len = Number(file.headers.get('content-length') ?? 0);
    if (len > MAX_BYTES) {
      return { status: 'download-broken', message: `The file is ${len} bytes, over the ${MAX_BYTES} limit.`, url, filename, version };
    }
    bytes = new Uint8Array(await file.arrayBuffer());
  } catch (e) {
    return { status: 'download-broken', message: `The download failed: ${e.message}`, url, filename, version };
  }
  if (!file.ok || !looksLike(source.format, bytes)) {
    return {
      status: 'download-broken',
      message: file.ok
        ? 'The link gives a web page instead of a tool database.'
        : `The link answered ${file.status}.`,
      url,
      filename,
      version,
    };
  }
  return { status: 'ok', message: '', url, filename, version, sha256: await sha256(bytes), bytes: bytes.length };
}
