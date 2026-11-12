// Keeping junk out of /v1/crash. CutCraft is open source, so no secret in the app can prove a
// report is real; anything built into it can be pulled out. Instead each report has to:
//  - look exactly like what CutCraft writes (a release version, its crash-log layout);
//  - carry a small proof of work: a nonce whose hash starts with POW_BITS zero bits. One report
//    costs the app a fraction of a second; thousands cost a spammer real time. Nothing to steal;
//  - stay inside a per-IP allowance: IP_PER_CRASH reports of one crash and IP_DAILY in all, per
//    day. IPs are never stored: only a hash with a random salt made each day and deleted after.
import { dayOf } from './rules.js';

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
const sha256 = async (text) => new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text)));

const VERSION = /^\d{1,4}\.\d{1,4}\.\d{1,4}(?:-[0-9A-Za-z.]{1,20})?$/;

/** Whether the report looks like one CutCraft wrote. Returns an error message, or null. */
export function checkShape(report) {
  if (!VERSION.test(report.version)) return 'version is not a CutCraft release number.';
  const head = `Cut Craft ${report.version} on `;
  if (!report.log.startsWith(head)) return 'log does not start the way a CutCraft crash log does.';
  const firstLine = report.log.slice(0, report.log.indexOf('\n') === -1 ? undefined : report.log.indexOf('\n'));
  if (!firstLine.includes(' crashed: ')) return 'log does not say what crashed.';
  if (!report.log.includes('\nBacktrace:\n')) return 'log has no backtrace.';
  return null;
}

/** The text the app hashes, nonce last: the day (UTC), install id, SHA-256 of the log. */
export async function workText(report, day, nonce) {
  const logHash = hex(await sha256(report.log));
  return `cutcraft-crash-v1\n${day}\n${report.install_id}\n${logHash}\n${nonce}`;
}

export function zeroBits(bytes) {
  let n = 0;
  for (const b of bytes) {
    if (b === 0) { n += 8; continue; }
    return n + Math.clz32(b) - 24;
  }
  return n;
}

/**
 * Whether `nonce` is a proof of work for this report, made today or yesterday (UTC), so a crash
 * sent just after midnight still counts. bits = 0 turns the check off.
 */
export async function checkWork(report, nonce, bits, now) {
  if (bits <= 0) return true;
  if (typeof nonce !== 'string' || !/^[0-9a-z]{1,24}$/.test(nonce)) return false;
  for (const day of [dayOf(now.toISOString()), dayOf(new Date(now.getTime() - 86400000).toISOString())]) {
    if (zeroBits(await sha256(await workText(report, day, nonce))) >= bits) return true;
  }
  return false;
}

/** Today's salt for IP hashes: random, made once a day, deleted by the clean-up after the day. */
async function ipSalt(db, day) {
  const key = `ip_salt:${day}`;
  const row = await db.prepare('SELECT value FROM meta WHERE key = ?').bind(key).first();
  if (row) return row.value;
  await db.prepare('INSERT OR IGNORE INTO meta (key, value) VALUES (?, ?)')
    .bind(key, hex(crypto.getRandomValues(new Uint8Array(32)))).run();
  return (await db.prepare('SELECT value FROM meta WHERE key = ?').bind(key).first()).value;
}

const ipHash = async (db, ip, day) => hex(await sha256(`${await ipSalt(db, day)}:${ip}`)).slice(0, 32);

/**
 * Milliseconds this IP must wait before this crash is taken from it, or 0. No IP (only in tests
 * and local runs; Cloudflare always sets it) means no limit.
 */
export async function ipWait(db, ip, signature, { ipPerCrash, ipDaily }, now) {
  if (!ip) return 0;
  const day = dayOf(now.toISOString());
  const who = await ipHash(db, ip, day);
  const row = await db.prepare(
    `SELECT COALESCE(SUM(count), 0) AS total,
            COALESCE(SUM(CASE WHEN signature = ? THEN count ELSE 0 END), 0) AS same
     FROM ip_counts WHERE day = ? AND who = ?`,
  ).bind(signature, day, who).first();
  if (row.total < ipDaily && row.same < ipPerCrash) return 0;
  return Date.parse(`${day}T00:00:00Z`) + 86400000 - now.getTime();
}

/** Count a report this IP sent. */
export async function countIp(db, ip, signature, now) {
  if (!ip) return;
  const day = dayOf(now.toISOString());
  await db.prepare(
    `INSERT INTO ip_counts (day, who, signature, count) VALUES (?, ?, ?, 1)
     ON CONFLICT (day, who, signature) DO UPDATE SET count = count + 1`,
  ).bind(day, await ipHash(db, ip, day), signature).run();
}

const PANIC = /^Cut Craft \S+ on [^\n]*? crashed: panicked at (.+?):\d+:\d+:\n([\s\S]*?)\n\nBacktrace:\n/;

/** A source path as it reads on every machine: from `crates/` (or the Rust `library/`) on. */
function samePath(path) {
  const p = path.replace(/\\/g, '/');
  const at = p.search(/(?:^|\/)(crates|library)\//);
  return at === -1 ? p.split('/').slice(-3).join('/') : p.slice(p[at] === '/' ? at + 1 : at);
}

/** A panic message with what differs between machines and runs taken out. */
function sameMessage(text) {
  return text
    .replace(/(?:[A-Za-z]:)?(?:[\\/][^\s"'`:,;)\]}]+){2,}/g, '<path>')
    .replace(/0x[0-9a-fA-F]+/g, '<hex>')
    .replace(/\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b/g, '<uuid>')
    .replace(/\d+(?:\.\d+)?/g, '<n>')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 400);
}

/**
 * The crash's ID, worked out here from the log alone so no client can choose it: the file that
 * panicked and the panic message, with line numbers, numbers, addresses and paths taken out. The
 * same panic gives the same ID on every machine and version. (Release builds strip function
 * names, so the backtrace is only addresses and can't be used.) Null if the log has no panic.
 */
export async function crashId(log) {
  const m = log.match(PANIC);
  if (!m) return null;
  return `cc-${hex(await sha256(`${samePath(m[1])}\n${sameMessage(m[2])}`)).slice(0, 12)}`;
}

/** Count a refused report by reason, per network hash, for the dashboard. Never the IP. */
export async function countRefusal(db, ip, reason, now) {
  const day = dayOf(now.toISOString());
  let who = ip ? await ipHash(db, ip, day) : '-';
  const many = await db.prepare('SELECT COUNT(*) AS n FROM refusals WHERE day = ?').bind(day).first();
  if (many.n >= 5000) who = 'many'; // a flood from many networks: keep counting, stop adding rows
  await db.prepare(
    `INSERT INTO refusals (day, reason, who, count) VALUES (?, ?, ?, 1)
     ON CONFLICT (day, reason, who) DO UPDATE SET count = count + 1`,
  ).bind(day, reason, who).run();
}
