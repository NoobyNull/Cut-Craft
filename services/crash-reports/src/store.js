// Reading and writing the D1 database. Each function takes the D1 binding, so tests can pass
// a stand-in backed by node:sqlite and run the same SQL.
import { waitBeforeNext, dayOf, daysBefore, SAMPLES_PER_CRASH } from './rules.js';

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');

/** A random salt made once and kept in the database, so install ids are never stored as sent. */
async function salt(db) {
  const row = await db.prepare("SELECT value FROM meta WHERE key = 'salt'").first();
  if (row) return row.value;
  const fresh = hex(crypto.getRandomValues(new Uint8Array(32)));
  await db.prepare("INSERT OR IGNORE INTO meta (key, value) VALUES ('salt', ?)").bind(fresh).run();
  return (await db.prepare("SELECT value FROM meta WHERE key = 'salt'").first()).value;
}

async function hashInstall(db, installId) {
  const data = new TextEncoder().encode(`${await salt(db)}:${installId}`);
  return hex(await crypto.subtle.digest('SHA-256', data)).slice(0, 32);
}

/**
 * Take one crash report. Returns {accepted: true} or {accepted: false, retryAfterMs, reason}.
 * The raw install id is hashed and never stored. The log is kept only while the crash is open
 * and fewer than SAMPLES_PER_CRASH examples are held (a new version replaces the oldest).
 */
export async function receive(db, report, { dailyLimit }, now = new Date()) {
  const nowIso = now.toISOString();
  const day = dayOf(nowIso);
  const who = await hashInstall(db, report.install_id);

  const sender = await db.prepare('SELECT last_at FROM senders WHERE who = ?').bind(who).first();
  const wait = waitBeforeNext(sender?.last_at, now.getTime());
  if (wait > 0) return { accepted: false, retryAfterMs: wait, reason: 'hourly' };

  const today = await db.prepare('SELECT COALESCE(SUM(count), 0) AS n FROM daily WHERE day = ?').bind(day).first();
  if (today.n >= dailyLimit) {
    const midnight = Date.parse(`${day}T00:00:00Z`) + 86400000;
    return { accepted: false, retryAfterMs: midnight - now.getTime(), reason: 'daily' };
  }

  const { signature, title, version, os, log } = report;
  await db.batch([
    db.prepare('INSERT INTO senders (who, last_at) VALUES (?, ?) ON CONFLICT (who) DO UPDATE SET last_at = excluded.last_at')
      .bind(who, nowIso),
    db.prepare(
      `INSERT INTO crashes (signature, title, count, first_seen, last_seen, first_version, last_version)
       VALUES (?, ?, 1, ?, ?, ?, ?)
       ON CONFLICT (signature) DO UPDATE SET
         count = count + 1, last_seen = excluded.last_seen, last_version = excluded.last_version,
         status = CASE WHEN status = 'handled' THEN 'back' ELSE status END,
         handled_at = CASE WHEN status = 'handled' THEN NULL ELSE handled_at END`,
    ).bind(signature, title, nowIso, nowIso, version, version),
    db.prepare(
      `INSERT INTO daily (day, signature, version, os, count) VALUES (?, ?, ?, ?, 1)
       ON CONFLICT (day, signature, version, os) DO UPDATE SET count = count + 1`,
    ).bind(day, signature, version, os),
  ]);

  const seen = await db.prepare('INSERT OR IGNORE INTO seen (signature, who) VALUES (?, ?)').bind(signature, who).run();
  if (seen.meta?.changes) {
    await db.prepare('UPDATE crashes SET installs = installs + 1 WHERE signature = ?').bind(signature).run();
  }

  const crash = await db.prepare('SELECT status FROM crashes WHERE signature = ?').bind(signature).first();
  if (crash.status === 'new' || crash.status === 'back') await keepSample(db, signature, version, os, log, nowIso);
  return { accepted: true };
}

async function keepSample(db, signature, version, os, log, nowIso) {
  const { results } = await db
    .prepare('SELECT id, version FROM samples WHERE signature = ? ORDER BY received_at, id')
    .bind(signature).all();
  const insert = db.prepare('INSERT INTO samples (signature, version, os, log, received_at) VALUES (?, ?, ?, ?, ?)')
    .bind(signature, version, os, log, nowIso);
  if (results.length < SAMPLES_PER_CRASH) return insert.run();
  if (results.some((s) => s.version === version)) return; // already have an example from this version
  await db.batch([db.prepare('DELETE FROM samples WHERE id = ?').bind(results[0].id), insert]);
}

/** Everything the dashboard's overview needs for the last `days` days. */
export async function summary(db, days, now = new Date()) {
  const from = dayOf(daysBefore(now, days - 1));
  const q = (sql, ...args) => db.prepare(sql).bind(...args).all().then((r) => r.results);
  const [perDay, byVersion, byOs, crashes, held] = await Promise.all([
    q('SELECT day, SUM(count) AS count FROM daily WHERE day >= ? GROUP BY day ORDER BY day', from),
    q('SELECT version, SUM(count) AS count FROM daily WHERE day >= ? GROUP BY version ORDER BY count DESC', from),
    q('SELECT os, SUM(count) AS count FROM daily WHERE day >= ? GROUP BY os ORDER BY count DESC', from),
    q(`SELECT c.signature, c.title, c.status, c.issue_url, c.note, c.count, c.installs, c.first_seen, c.last_seen,
              c.first_version, c.last_version,
              (SELECT COALESCE(SUM(d.count), 0) FROM daily d WHERE d.signature = c.signature AND d.day >= ?) AS recent,
              (SELECT COUNT(*) FROM samples s WHERE s.signature = c.signature) AS samples
       FROM crashes c ORDER BY CASE c.status WHEN 'back' THEN 0 WHEN 'new' THEN 1 WHEN 'issue' THEN 2 ELSE 3 END,
                               c.last_seen DESC`, from),
    q('SELECT COUNT(*) AS logs, COALESCE(SUM(LENGTH(log)), 0) AS bytes, MIN(received_at) AS oldest FROM samples'),
  ]);
  const total = perDay.reduce((n, d) => n + d.count, 0);
  return { days, from, total, perDay, byVersion, byOs, crashes, held: held[0] };
}

/** One crash with its example logs and its counts by version and system. */
export async function detail(db, signature) {
  const crash = await db.prepare('SELECT * FROM crashes WHERE signature = ?').bind(signature).first();
  if (!crash) return null;
  const q = (sql) => db.prepare(sql).bind(signature).all().then((r) => r.results);
  const [samples, byVersion, byOs] = await Promise.all([
    q('SELECT id, version, os, log, received_at FROM samples WHERE signature = ? ORDER BY received_at DESC'),
    q('SELECT version, SUM(count) AS count FROM daily WHERE signature = ? GROUP BY version ORDER BY count DESC'),
    q('SELECT os, SUM(count) AS count FROM daily WHERE signature = ? GROUP BY os ORDER BY count DESC'),
  ]);
  return { ...crash, samples, byVersion, byOs };
}

async function exists(db, signature) {
  return !!(await db.prepare('SELECT 1 AS x FROM crashes WHERE signature = ?').bind(signature).first());
}

/** Mark a crash dealt with. Its logs are deleted at once; only the counts stay. */
export async function markHandled(db, signature, note, now = new Date()) {
  if (!(await exists(db, signature))) return false;
  await db.batch([
    db.prepare("UPDATE crashes SET status = 'handled', handled_at = ?, note = COALESCE(?, note) WHERE signature = ?")
      .bind(now.toISOString(), note ?? null, signature),
    db.prepare('DELETE FROM samples WHERE signature = ?').bind(signature),
  ]);
  return true;
}

/** Record the GitHub issue a crash became. Its logs are deleted at once. */
export async function markIssue(db, signature, issueUrl, now = new Date()) {
  const url = new URL(issueUrl); // throws on a bad address
  if (url.protocol !== 'https:' || url.hostname !== 'github.com' || !/\/issues\/\d+$/.test(url.pathname)) {
    throw new Error('Paste the address of a GitHub issue, like https://github.com/owner/repo/issues/123.');
  }
  if (!(await exists(db, signature))) return false;
  await db.batch([
    db.prepare("UPDATE crashes SET status = 'issue', issue_url = ?, handled_at = ? WHERE signature = ?")
      .bind(url.href, now.toISOString(), signature),
    db.prepare('DELETE FROM samples WHERE signature = ?').bind(signature),
  ]);
  return true;
}

/** Open a crash again so new reports keep example logs. */
export async function reopen(db, signature) {
  if (!(await exists(db, signature))) return false;
  await db.prepare("UPDATE crashes SET status = 'new', handled_at = NULL WHERE signature = ?").bind(signature).run();
  return true;
}

/** Delete one example log (for example one that holds something it shouldn't). */
export async function deleteSample(db, id) {
  const r = await db.prepare('DELETE FROM samples WHERE id = ?').bind(id).run();
  return !!r.meta?.changes;
}

/** Remove a crash and everything about it, counts included. */
export async function forget(db, signature) {
  if (!(await exists(db, signature))) return false;
  await db.batch(['samples', 'daily', 'seen', 'crashes'].map((t) =>
    db.prepare(`DELETE FROM ${t} WHERE signature = ?`).bind(signature)));
  return true;
}

/**
 * The daily clean-up: logs older than retainDays, rate-limit rows older than a day, install
 * hashes for crashes not seen in retainDays, and daily counts older than statsDays.
 */
export async function cleanup(db, { retainDays, statsDays }, now = new Date()) {
  const results = await db.batch([
    db.prepare('DELETE FROM samples WHERE received_at < ?').bind(daysBefore(now, retainDays)),
    db.prepare('DELETE FROM senders WHERE last_at < ?').bind(daysBefore(now, 1)),
    db.prepare('DELETE FROM seen WHERE signature IN (SELECT signature FROM crashes WHERE last_seen < ?)')
      .bind(daysBefore(now, retainDays)),
    db.prepare('DELETE FROM daily WHERE day < ?').bind(dayOf(daysBefore(now, statsDays))),
  ]);
  const n = (i) => results?.[i]?.meta?.changes ?? 0;
  await db.prepare("INSERT OR REPLACE INTO meta (key, value) VALUES ('last_cleanup', ?)").bind(now.toISOString()).run();
  return { logs: n(0), senders: n(1), installs: n(2), dailyRows: n(3) };
}
