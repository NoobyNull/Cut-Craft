// Reading and writing the D1 database. Each function takes the D1 binding, so tests can pass
// a stand-in backed by node:sqlite and run the same SQL.
import { checkSource } from './check.js';
import { isNewVersion, waitBeforeCheck } from './rules.js';

const PUBLIC_FIELDS =
  'id, vendor, format, page_url, url, filename, version, sha256, bytes, status, message, checked_at, changed_at, note';

/** What anyone may read: every enabled source with its current link. */
export async function publicList(db) {
  const { results } = await db
    .prepare(`SELECT ${PUBLIC_FIELDS} FROM sources WHERE enabled = 1 ORDER BY vendor, format`)
    .all();
  return results;
}

export async function publicOne(db, id) {
  return db.prepare(`SELECT ${PUBLIC_FIELDS} FROM sources WHERE enabled = 1 AND id = ?`).bind(id).first();
}

export async function historyOf(db, id) {
  const { results } = await db
    .prepare('SELECT url, filename, version, sha256, bytes, seen_at FROM history WHERE source_id = ? ORDER BY id DESC')
    .bind(id)
    .all();
  return results;
}

async function lastRun(db) {
  const row = await db.prepare("SELECT value FROM meta WHERE key = 'last_run'").first();
  return row ? row.value : null;
}

/**
 * Check every enabled vendor, unless a check already started in the last 24 hours.
 * @returns {Promise<{ran: false, retryAfterMs: number} | {ran: true, results: object[]}>}
 */
export async function checkAll(db, fetchFn, now = new Date()) {
  const wait = waitBeforeCheck(await lastRun(db), now.getTime());
  if (wait > 0) return { ran: false, retryAfterMs: wait };
  const nowIso = now.toISOString();
  await db.prepare("INSERT OR REPLACE INTO meta (key, value) VALUES ('last_run', ?)").bind(nowIso).run();

  const { results: sources } = await db.prepare('SELECT * FROM sources WHERE enabled = 1').all();
  const out = [];
  for (const source of sources) {
    const found = await checkSource(source, fetchFn);
    const changed = await record(db, source, found, nowIso);
    out.push({ id: source.id, status: found.status, changed, message: found.message });
  }
  return { ran: true, results: out };
}

/** Store one check's result. Returns true when it was a new version. */
export async function record(db, source, found, nowIso) {
  if (found.status === 'manual') {
    await db.prepare('UPDATE sources SET status = ?, checked_at = ? WHERE id = ?')
      .bind(source.url ? 'manual' : 'needs-link', nowIso, source.id).run();
    return false;
  }
  if (found.status !== 'ok') {
    // keep the last good link so CutCraft can still offer it, but say what went wrong
    await db.prepare('UPDATE sources SET status = ?, message = ?, checked_at = ? WHERE id = ?')
      .bind(found.status, found.message, nowIso, source.id).run();
    return false;
  }
  const isNew = isNewVersion(source, found);
  const stmts = [
    db.prepare(
      `UPDATE sources SET url = ?, filename = ?, version = ?, sha256 = ?, bytes = ?, status = 'ok',
       message = NULL, checked_at = ?, changed_at = CASE WHEN ? THEN ? ELSE changed_at END WHERE id = ?`,
    ).bind(found.url, found.filename, found.version, found.sha256, found.bytes, nowIso, isNew ? 1 : 0, nowIso, source.id),
  ];
  if (isNew) {
    stmts.push(
      db.prepare('INSERT INTO history (source_id, url, filename, version, sha256, bytes, seen_at) VALUES (?, ?, ?, ?, ?, ?, ?)')
        .bind(source.id, found.url, found.filename, found.version, found.sha256, found.bytes, nowIso),
    );
  }
  await db.batch(stmts);
  return isNew;
}

const EDITABLE = ['vendor', 'format', 'page_url', 'link_match', 'version_match', 'mode', 'enabled', 'note'];

/** Add or change a source's definition (admin only). */
export async function upsertSource(db, id, body) {
  if (!/^[a-z0-9][a-z0-9-]{0,62}$/.test(id)) throw new Error('id must be lowercase letters, digits and hyphens');
  const fields = EDITABLE.filter((k) => body[k] !== undefined);
  for (const k of ['link_match', 'version_match']) {
    if (body[k]) new RegExp(body[k]); // throws on a bad pattern before it is stored
  }
  const existing = await db.prepare('SELECT id FROM sources WHERE id = ?').bind(id).first();
  if (!existing) {
    for (const k of ['vendor', 'format', 'page_url']) {
      if (!body[k]) throw new Error(`a new source needs ${k}`);
    }
    if ((body.mode ?? 'auto') === 'auto' && !body.link_match) throw new Error('an automatic source needs link_match');
    await db.prepare(`INSERT INTO sources (id, ${fields.join(', ')}) VALUES (?, ${fields.map(() => '?').join(', ')})`)
      .bind(id, ...fields.map((k) => body[k])).run();
  } else if (fields.length) {
    await db.prepare(`UPDATE sources SET ${fields.map((k) => `${k} = ?`).join(', ')} WHERE id = ?`)
      .bind(...fields.map((k) => body[k]), id).run();
  }
}

/** Set a manual source's link by hand, for vendors whose site blocks automatic checks. */
export async function setManualLink(db, id, body, nowIso) {
  const url = new URL(body.url); // throws on a bad address
  if (url.protocol !== 'https:') throw new Error('the link must be https');
  const source = await db.prepare('SELECT * FROM sources WHERE id = ?').bind(id).first();
  if (!source) throw new Error(`no source ${id}`);
  const filename = body.filename ?? decodeURIComponent(url.pathname.split('/').pop());
  const isNew = source.url !== url.href || source.version !== (body.version ?? null);
  const stmts = [
    db.prepare(`UPDATE sources SET url = ?, filename = ?, version = ?, sha256 = NULL, bytes = NULL,
      status = 'manual', message = NULL, changed_at = ? WHERE id = ?`)
      .bind(url.href, filename, body.version ?? null, isNew ? nowIso : source.changed_at, id),
  ];
  if (isNew) {
    stmts.push(db.prepare('INSERT INTO history (source_id, url, filename, version, seen_at) VALUES (?, ?, ?, ?, ?)')
      .bind(id, url.href, filename, body.version ?? null, nowIso));
  }
  await db.batch(stmts);
  return isNew;
}
