// Messages and opinion polls shown inside CutCraft. Matthew publishes one from the /votes page;
// CutCraft asks GET /v1/announcement at startup and shows it once. A poll's buttons POST
// /v1/vote. One vote per install per poll (changeable), kept only as a salted hash.
import { salt } from './store.js';
import { ipWait, countIp } from './guard.js';

export const MAX_CHOICES = 6;
const ID = /^[a-z0-9][a-z0-9-]{1,39}$/;
const INSTALL = /^[A-Za-z0-9-]{8,64}$/;
const VERSION = /^[\x20-\x7e]{1,40}$/;

export class PollError extends Error {
  constructor(status, message, extra = {}) {
    super(message);
    this.status = status;
    this.extra = extra;
  }
}

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
async function voter(db, poll, installId) {
  const data = new TextEncoder().encode(`${await salt(db)}:vote:${poll}:${installId}`);
  return hex(await crypto.subtle.digest('SHA-256', data)).slice(0, 32);
}

const text = (v, name, max) => {
  if (typeof v !== 'string' || !v.trim()) throw new PollError(400, `${name} is missing.`);
  const t = v.trim();
  if (t.length > max) throw new PollError(400, `${name} must be at most ${max} characters.`);
  return t;
};

/** Check what the votes page sends. Returns the row to store. */
export function checkPoll(body) {
  if (!body || typeof body !== 'object') throw new PollError(400, 'Expected a JSON object.');
  const id = String(body.id ?? '').trim();
  if (!ID.test(id)) throw new PollError(400, 'The short name must be 2 to 40 lower-case letters, digits or dashes.');
  const title = text(body.title, 'The title', 80);
  const message = text(body.body, 'The message', 1200);
  const raw = body.choices ?? [];
  if (!Array.isArray(raw)) throw new PollError(400, 'choices must be a list.');
  const choices = raw.map((c) => (typeof c === 'string' ? c.trim() : '')).filter(Boolean);
  if (choices.length === 1) throw new PollError(400, 'A vote needs at least two choices (or none, for a plain message).');
  if (choices.length > MAX_CHOICES) throw new PollError(400, `At most ${MAX_CHOICES} choices.`);
  if (choices.some((c) => c.length > 40)) throw new PollError(400, 'Each choice must be at most 40 characters.');
  if (new Set(choices.map((c) => c.toLowerCase())).size !== choices.length) throw new PollError(400, 'Two choices are the same.');
  let endsAt = null;
  if (body.ends_at) {
    const t = Date.parse(body.ends_at);
    if (Number.isNaN(t)) throw new PollError(400, 'The end date could not be read.');
    endsAt = new Date(t).toISOString();
  }
  return { id, title, body: message, choices, show_results: body.show_results !== false, ends_at: endsAt };
}

const isLive = (row, now) => !row.ended_at && (!row.ends_at || row.ends_at > now.toISOString());

/** Publish (or republish, keeping its votes) and end every other live one. */
export async function publish(db, body, now) {
  const p = checkPoll(body);
  const stamp = now.toISOString();
  // votes are kept by choice number, so changing the choices would move them to other choices
  const old = await db.prepare('SELECT choices FROM polls WHERE id = ?').bind(p.id).first();
  const votes = old && (await db.prepare('SELECT COUNT(*) AS n FROM votes WHERE poll = ?').bind(p.id).first()).n;
  if (votes && old.choices !== JSON.stringify(p.choices)) {
    throw new PollError(400, 'This vote already has votes, so its choices can\'t change. Publish it under a new short name instead.');
  }
  await db.batch([
    db.prepare('UPDATE polls SET ended_at = ? WHERE ended_at IS NULL AND id <> ?').bind(stamp, p.id),
    db.prepare(
      `INSERT INTO polls (id, title, body, choices, show_results, created_at, ends_at, ended_at)
       VALUES (?, ?, ?, ?, ?, ?, ?, NULL)
       ON CONFLICT (id) DO UPDATE SET title = excluded.title, body = excluded.body,
         choices = excluded.choices, show_results = excluded.show_results,
         ends_at = excluded.ends_at, ended_at = NULL`,
    ).bind(p.id, p.title, p.body, JSON.stringify(p.choices), p.show_results ? 1 : 0, stamp, p.ends_at),
  ]);
  return p;
}

const shape = (row) => ({
  id: row.id, title: row.title, body: row.body, choices: JSON.parse(row.choices),
  show_results: row.show_results === 1,
});

/** What CutCraft shows now, or null. */
export async function current(db, now) {
  const row = await db.prepare(
    `SELECT * FROM polls WHERE ended_at IS NULL AND (ends_at IS NULL OR ends_at > ?)
     ORDER BY created_at DESC LIMIT 1`,
  ).bind(now.toISOString()).first();
  return row ? shape(row) : null;
}

async function totals(db, id, count) {
  const { results } = await db.prepare('SELECT choice, COUNT(*) AS n FROM votes WHERE poll = ? GROUP BY choice').bind(id).all();
  const out = new Array(count).fill(0);
  for (const r of results) if (r.choice < count) out[r.choice] = r.n;
  return out;
}

/** Take one install's vote. Returns {ok, totals?}. */
export async function castVote(db, body, ip, { votesPerIp }, now) {
  if (!body || typeof body !== 'object') throw new PollError(400, 'Expected a JSON object.');
  const { poll, choice, install_id: install, version } = body;
  if (typeof install !== 'string' || !INSTALL.test(install)) throw new PollError(400, 'install_id must be 8 to 64 of A-Z a-z 0-9 -.');
  if (typeof version !== 'string' || !VERSION.test(version)) throw new PollError(400, 'version is missing.');
  const row = typeof poll === 'string' && ID.test(poll)
    ? await db.prepare('SELECT * FROM polls WHERE id = ?').bind(poll).first()
    : null;
  if (!row) throw new PollError(404, 'No such vote.');
  const p = shape(row);
  if (p.choices.length === 0) throw new PollError(400, 'That is a message, not a vote.');
  if (!isLive(row, now)) throw new PollError(410, 'That vote has ended.');
  if (!Number.isInteger(choice) || choice < 0 || choice >= p.choices.length) throw new PollError(400, 'No such choice.');
  const key = `vote:${p.id}`;
  const wait = await ipWait(db, ip, key, { ipPerCrash: votesPerIp, ipDaily: Infinity }, now);
  if (wait > 0) {
    throw new PollError(429, 'Too many votes from this network today.', { 'retry-after': String(Math.ceil(wait / 1000)) });
  }
  await db.prepare(
    `INSERT INTO votes (poll, who, choice, version, voted_at) VALUES (?, ?, ?, ?, ?)
     ON CONFLICT (poll, who) DO UPDATE SET choice = excluded.choice, version = excluded.version, voted_at = excluded.voted_at`,
  ).bind(p.id, await voter(db, p.id, install), choice, version, now.toISOString()).run();
  await countIp(db, ip, key, now);
  return p.show_results ? { ok: true, totals: await totals(db, p.id, p.choices.length) } : { ok: true };
}

/** Every poll, newest first, with its totals, for the votes page. */
export async function listPolls(db, now) {
  const { results } = await db.prepare('SELECT * FROM polls ORDER BY created_at DESC').all();
  const out = [];
  for (const row of results) {
    const p = shape(row);
    const t = await totals(db, p.id, p.choices.length);
    out.push({
      ...p, created_at: row.created_at, ends_at: row.ends_at, ended_at: row.ended_at,
      live: isLive(row, now), totals: t, voters: t.reduce((a, b) => a + b, 0),
    });
  }
  return out;
}

export async function endPoll(db, id, now) {
  const r = await db.prepare('UPDATE polls SET ended_at = ? WHERE id = ? AND ended_at IS NULL').bind(now.toISOString(), id).run();
  if (r.meta.changes) return true;
  return !!(await db.prepare('SELECT id FROM polls WHERE id = ?').bind(id).first());
}

export async function deletePoll(db, id) {
  const [, gone] = await db.batch([
    db.prepare('DELETE FROM votes WHERE poll = ?').bind(id),
    db.prepare('DELETE FROM polls WHERE id = ?').bind(id),
  ]);
  return gone.meta.changes > 0;
}
