// Signing in to the dashboard. Security keys (WebAuthn, e.g. YubiKeys) are the login. The
// admin token only works until the first key is registered: it is how the first key gets added.
// A sign-in makes a session cookie (HttpOnly, SameSite=Strict); only its hash is stored.
//
// Lost every key? Clear them with wrangler (needs the Cloudflare login), and the token works again:
//   npx wrangler d1 execute cutcraft-crash-reports --remote --command "DELETE FROM passkeys; DELETE FROM sessions"
import { b64url, verifyRegistration, verifyAssertion } from './webauthn.js';

export const COOKIE = '__Host-cc_session';
export const SESSION_HOURS = 12;
const CHALLENGE_MS = 5 * 60 * 1000;
/** Unanswered sign-in requests allowed at once, so anyone can't fill the table. */
const MAX_OPEN_CHALLENGES = 50;
const NAME_CHARS = 60;

export class AuthError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
const hashOf = async (text) => hex(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text)));
const random = (n) => b64url.encode(crypto.getRandomValues(new Uint8Array(n)));
const site = (url) => ({ origin: url.origin, rpId: url.hostname });

export async function keyCount(db) {
  return (await db.prepare('SELECT COUNT(*) AS n FROM passkeys').first()).n;
}

export async function listKeys(db) {
  const { results } = await db.prepare(
    'SELECT id, name, created_at, last_used_at FROM passkeys ORDER BY created_at',
  ).all();
  return results;
}

export async function removeKey(db, id) {
  const r = await db.prepare('DELETE FROM passkeys WHERE id = ?').bind(id).run();
  return r.meta.changes > 0;
}

async function newChallenge(db, purpose, now) {
  await db.prepare('DELETE FROM challenges WHERE expires_at <= ?').bind(now.toISOString()).run();
  const open = await db.prepare('SELECT COUNT(*) AS n FROM challenges').first();
  if (open.n >= MAX_OPEN_CHALLENGES) throw new AuthError(429, 'Too many sign-in attempts. Wait a few minutes.');
  const challenge = random(32);
  await db.prepare('INSERT INTO challenges (challenge, purpose, expires_at) VALUES (?, ?, ?)')
    .bind(challenge, purpose, new Date(now.getTime() + CHALLENGE_MS).toISOString()).run();
  return challenge;
}

/** The challenge the browser answered, used up so the same answer can't be sent twice. */
async function takeChallenge(db, clientDataJSON, purpose, now) {
  let challenge;
  try {
    challenge = JSON.parse(new TextDecoder().decode(b64url.decode(clientDataJSON))).challenge;
  } catch {
    throw new AuthError(400, "The browser's answer could not be read.");
  }
  if (typeof challenge !== 'string') throw new AuthError(400, "The browser's answer could not be read.");
  const r = await db.prepare('DELETE FROM challenges WHERE challenge = ? AND purpose = ? AND expires_at > ?')
    .bind(challenge, purpose, now.toISOString()).run();
  if (r.meta.changes !== 1) throw new AuthError(400, 'That request has expired. Try again.');
  return challenge;
}

/** The fixed WebAuthn user handle for the one admin, made once. */
async function userHandle(db) {
  const row = await db.prepare("SELECT value FROM meta WHERE key = 'webauthn_user'").first();
  if (row) return row.value;
  await db.prepare("INSERT OR IGNORE INTO meta (key, value) VALUES ('webauthn_user', ?)").bind(random(16)).run();
  return (await db.prepare("SELECT value FROM meta WHERE key = 'webauthn_user'").first()).value;
}

/** What the page passes to navigator.credentials.create() to add a key. */
export async function registerOptions(db, url, now) {
  const { results } = await db.prepare('SELECT id FROM passkeys').all();
  return {
    challenge: await newChallenge(db, 'register', now),
    rp: { id: url.hostname, name: 'CutCraft crash reports' },
    user: { id: await userHandle(db), name: 'admin', displayName: 'CutCraft crash reports admin' },
    exclude: results.map((r) => r.id),
    timeout: 120000,
  };
}

/** Check the new key and keep it. */
export async function register(db, url, body, now) {
  const challenge = await takeChallenge(db, body?.clientDataJSON, 'register', now);
  let cred;
  try {
    cred = await verifyRegistration(body, { challenge, ...site(url) });
  } catch (e) {
    throw new AuthError(400, e.message);
  }
  const name = (typeof body.name === 'string' ? body.name.trim().slice(0, NAME_CHARS) : '') || 'Security key';
  const known = await db.prepare('SELECT 1 FROM passkeys WHERE id = ?').bind(cred.credentialId).first();
  if (known) throw new AuthError(409, 'That key is already registered.');
  await db.prepare('INSERT INTO passkeys (id, name, public_key, sign_count, created_at) VALUES (?, ?, ?, ?, ?)')
    .bind(cred.credentialId, name, JSON.stringify(cred.publicKey), cred.signCount, now.toISOString()).run();
}

/** What the page passes to navigator.credentials.get() to sign in. */
export async function loginOptions(db, url, now) {
  const { results } = await db.prepare('SELECT id FROM passkeys').all();
  if (!results.length) throw new AuthError(409, 'No security key is registered yet. Sign in with the admin token.');
  return {
    challenge: await newChallenge(db, 'login', now),
    rpId: url.hostname,
    allow: results.map((r) => r.id),
    timeout: 120000,
  };
}

/** Check the key's signature; on success returns the Set-Cookie value for a new session. */
export async function login(db, url, body, now) {
  const challenge = await takeChallenge(db, body?.clientDataJSON, 'login', now);
  const key = typeof body.id === 'string'
    ? await db.prepare('SELECT id, public_key, sign_count FROM passkeys WHERE id = ?').bind(body.id).first()
    : null;
  if (!key) throw new AuthError(400, "That key isn't registered here.");
  let result;
  try {
    result = await verifyAssertion(body, {
      challenge, ...site(url), publicKey: JSON.parse(key.public_key), signCount: key.sign_count,
    });
  } catch (e) {
    throw new AuthError(400, e.message);
  }
  await db.prepare('UPDATE passkeys SET sign_count = ?, last_used_at = ? WHERE id = ?')
    .bind(result.signCount, now.toISOString(), key.id).run();
  return startSession(db, now);
}

/** A new session; returns the Set-Cookie value. */
export async function startSession(db, now) {
  await db.prepare('DELETE FROM sessions WHERE expires_at <= ?').bind(now.toISOString()).run();
  const raw = random(32);
  await db.prepare('INSERT INTO sessions (id, created_at, expires_at) VALUES (?, ?, ?)')
    .bind(await hashOf(raw), now.toISOString(), new Date(now.getTime() + SESSION_HOURS * 3600000).toISOString()).run();
  return `${COOKIE}=${raw}; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=${SESSION_HOURS * 3600}`;
}

const cookieValue = (request) =>
  (request.headers.get('cookie') ?? '').match(new RegExp(`(?:^|;\\s*)${COOKIE}=([A-Za-z0-9_-]{20,100})`))?.[1];

/** Whether the request carries a live session cookie. */
export async function hasSession(request, db, now) {
  const raw = cookieValue(request);
  if (!raw) return false;
  const row = await db.prepare('SELECT expires_at FROM sessions WHERE id = ?').bind(await hashOf(raw)).first();
  return Boolean(row) && row.expires_at > now.toISOString();
}

/** End the session; returns the Set-Cookie value that clears the cookie. */
export async function endSession(request, db) {
  const raw = cookieValue(request);
  if (raw) await db.prepare('DELETE FROM sessions WHERE id = ?').bind(await hashOf(raw)).run();
  return `${COOKIE}=; Path=/; Secure; HttpOnly; SameSite=Strict; Max-Age=0`;
}
