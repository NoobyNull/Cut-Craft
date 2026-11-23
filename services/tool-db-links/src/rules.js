// Pure helpers: find a vendor's download link on its page, read a version from the file name,
// and tell a real tool database from an error page. No network, no database, so they test fast.

/** Every href on the page, with HTML entities decoded and made absolute against the page. */
export function links(html, pageUrl) {
  const out = [];
  const re = /href\s*=\s*["']([^"']+)["']/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    const raw = m[1].replace(/&amp;/g, '&').replace(/&#x2F;/gi, '/').replace(/&#47;/g, '/');
    try {
      out.push(new URL(raw, pageUrl).href);
    } catch {
      // a broken href on the vendor's page is not our problem
    }
  }
  return out;
}

/** The first link on the page whose address matches the source's pattern, or null. */
export function findLink(html, pageUrl, linkMatch) {
  const re = new RegExp(linkMatch, 'i');
  return links(html, pageUrl).find((u) => re.test(u)) ?? null;
}

/** The file name at the end of a download address, without the query string. */
export function fileName(url) {
  const path = new URL(url).pathname;
  return decodeURIComponent(path.slice(path.lastIndexOf('/') + 1));
}

/**
 * The version printed in the file name, such as "20251015" or "09-25-2024", using the
 * source's pattern (first capture group). Null when the vendor does not put one there.
 */
export function versionFrom(name, versionMatch) {
  if (!versionMatch) return null;
  const m = new RegExp(versionMatch, 'i').exec(name);
  return m ? (m[1] ?? m[0]) : null;
}

/** True when the page is a bot check (Cloudflare "Just a moment...") rather than the vendor's page. */
export function isBotCheck(status, html) {
  return (status === 403 || status === 503) && /just a moment|challenges\.cloudflare\.com|cf-chl/i.test(html);
}

const SQLITE = 'SQLite format 3\u0000';

/**
 * Whether the downloaded bytes look like the promised kind of file. A vendor's broken link
 * usually answers with an HTML page and status 200 or 404, which must not count as a new version.
 */
export function looksLike(format, bytes) {
  const head = new TextDecoder('latin1').decode(bytes.slice(0, 64));
  if (/^\s*(<!doctype|<html|<\?xml)/i.test(head)) return false;
  if (format === 'vectric-vtdb') return head.startsWith(SQLITE);
  return bytes.length > 0;
}

/** Lowercase hex SHA-256 of the bytes. */
export async function sha256(bytes) {
  const d = await crypto.subtle.digest('SHA-256', bytes);
  return [...new Uint8Array(d)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Whether a check found a new version. The hash decides when there is one: a vendor that
 * re-uploads the same file under a new address (SpeTool's ?v= stamp) has not released anything.
 */
export function isNewVersion(previous, found) {
  if (!previous || !previous.sha256) return true;
  return previous.sha256 !== found.sha256;
}

export const DAY_MS = 24 * 60 * 60 * 1000;

/** Milliseconds until another check is allowed, 0 when one may run now. At most one a day. */
export function waitBeforeCheck(lastCheckedIso, nowMs) {
  if (!lastCheckedIso) return 0;
  const since = nowMs - Date.parse(lastCheckedIso);
  return since >= DAY_MS ? 0 : DAY_MS - since;
}
