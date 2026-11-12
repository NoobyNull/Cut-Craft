// Security-key sign-in, end to end, against a software security key that does what a YubiKey
// does: makes a P-256 key pair, answers challenges with ECDSA signatures and counts its uses.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { d1 } from './d1-shim.js';
import { handle } from '../src/index.js';
import { b64url, cbor, derToRaw } from '../src/webauthn.js';

const TOKEN = 'a-long-admin-token-for-tests';
const ORIGIN = 'https://w.example';
const env = () => ({ DB: d1('schema.sql'), ADMIN_TOKEN: TOKEN });
const sha = async (b) => new Uint8Array(await crypto.subtle.digest('SHA-256', b));
const utf8 = (t) => new TextEncoder().encode(t);
const join = (...parts) => Uint8Array.from(parts.flatMap((p) => [...p]));

/** Just enough CBOR writing for a key's answers. */
function enc(v) {
  const head = (major, n) => (n < 24 ? [(major << 5) | n] : n < 256 ? [(major << 5) | 24, n] : [(major << 5) | 25, n >> 8, n & 255]);
  if (typeof v === 'number') return v >= 0 ? head(0, v) : head(1, -1 - v);
  if (typeof v === 'string') { const b = utf8(v); return [...head(3, b.length), ...b]; }
  if (v instanceof Uint8Array) return [...head(2, v.length), ...v];
  const out = head(5, v.size);
  for (const [k, x] of v) out.push(...enc(k), ...enc(x));
  return out;
}

/** ECDSA raw r||s to DER, as real keys send it. */
function der(raw) {
  const int = (b) => {
    let i = 0;
    while (i < 31 && b[i] === 0) i++;
    let v = [...b.slice(i)];
    if (v[0] & 0x80) v = [0, ...v];
    return [2, v.length, ...v];
  };
  const r = int(raw.slice(0, 32));
  const s = int(raw.slice(32));
  return Uint8Array.from([0x30, r.length + s.length, ...r, ...s]);
}

class SoftKey {
  static async make() {
    const k = new SoftKey();
    k.pair = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify']);
    k.id = crypto.getRandomValues(new Uint8Array(16));
    k.counter = 0;
    return k;
  }

  async authData(rpId, withKey) {
    const counter = new Uint8Array(4);
    new DataView(counter.buffer).setUint32(0, this.counter);
    const flags = 0x01 | 0x04 | (this.synced ? 0x18 : 0) | (withKey ? 0x40 : 0);
    const base = join(await sha(utf8(rpId)), [flags], counter);
    if (!withKey) return base;
    const jwk = await crypto.subtle.exportKey('jwk', this.pair.publicKey);
    const cose = new Map([[1, 2], [3, -7], [-1, 1], [-2, b64url.decode(jwk.x)], [-3, b64url.decode(jwk.y)]]);
    return join(base, new Uint8Array(16), [0, this.id.length], this.id, enc(cose));
  }

  async create(opts, origin = ORIGIN) {
    const clientData = utf8(JSON.stringify({ type: 'webauthn.create', challenge: opts.challenge, origin }));
    const att = new Map([['fmt', 'none'], ['attStmt', new Map()], ['authData', await this.authData(opts.rp.id, true)]]);
    return { clientDataJSON: b64url.encode(clientData), attestationObject: b64url.encode(Uint8Array.from(enc(att))) };
  }

  async get(opts, { origin = ORIGIN, rpId = opts.rpId } = {}) {
    if (!this.synced) this.counter += 1;
    const clientData = utf8(JSON.stringify({ type: 'webauthn.get', challenge: opts.challenge, origin }));
    const authData = await this.authData(rpId, false);
    const raw = new Uint8Array(await crypto.subtle.sign(
      { name: 'ECDSA', hash: 'SHA-256' }, this.pair.privateKey, join(authData, await sha(clientData)),
    ));
    return {
      id: b64url.encode(this.id),
      clientDataJSON: b64url.encode(clientData),
      authenticatorData: b64url.encode(authData),
      signature: b64url.encode(der(raw)),
    };
  }
}

const call = (e, method, path, { body, token, cookie, origin = ORIGIN } = {}) => {
  const headers = { 'content-type': 'application/json', origin };
  if (token) headers.authorization = `Bearer ${token}`;
  if (cookie) headers.cookie = cookie;
  return handle(new Request(`${ORIGIN}${path}`, { method, headers, body: body ? JSON.stringify(body) : undefined }), e);
};
const cookieOf = (r) => r.headers.get('set-cookie')?.split(';')[0];

/** Add a key with the token (the first key) or a session; returns the session cookie it gave, if any. */
async function addKey(e, key, auth, name = 'YubiKey') {
  const opts = await (await call(e, 'POST', '/admin/api/keys/options', auth)).json();
  const r = await call(e, 'POST', '/admin/api/keys', { ...auth, body: { name, ...(await key.create(opts)) } });
  assert.equal(r.status, 200, JSON.stringify(await r.clone().json()));
  return cookieOf(r);
}

async function signIn(e, key, how = {}) {
  const opts = await (await call(e, 'POST', '/auth/options')).json();
  return call(e, 'POST', '/auth/verify', { body: await key.get(opts, how) });
}

test('the token adds the first key, then only keys sign in', async () => {
  const e = env();
  assert.deepEqual(await (await call(e, 'GET', '/auth/status')).json(), { keys: 0, signedIn: false });
  assert.equal((await call(e, 'GET', '/admin/api/summary', { token: TOKEN })).status, 200);
  assert.equal((await call(e, 'POST', '/auth/options')).status, 409, 'no key to sign in with yet');

  const key = await SoftKey.make();
  const cookie = await addKey(e, key, { token: TOKEN });
  assert.match(cookie, /^__Host-cc_session=/, 'adding the first key signs you in');
  const set = (await call(e, 'GET', '/auth/status', { cookie })).headers;
  assert.equal(set.get('cache-control'), 'no-store');
  assert.deepEqual(await (await call(e, 'GET', '/auth/status', { cookie })).json(), { keys: 1, signedIn: true });

  assert.equal((await call(e, 'GET', '/admin/api/summary', { token: TOKEN })).status, 401, 'the token is retired');
  assert.equal((await call(e, 'GET', '/admin/api/summary', { cookie })).status, 200);

  const r = await signIn(e, key);
  assert.equal(r.status, 200);
  const flags = r.headers.get('set-cookie');
  assert.match(flags, /HttpOnly/);
  assert.match(flags, /Secure/);
  assert.match(flags, /SameSite=Strict/);
  assert.equal((await call(e, 'GET', '/admin/api/summary', { cookie: cookieOf(r) })).status, 200);
  const stored = e.DB.raw.prepare('SELECT id FROM sessions').all().map((s) => s.id);
  assert.ok(!stored.includes(cookieOf(r).split('=')[1]), 'only a hash of the cookie is kept');
});

test('a second key can be added from a session, and either signs in', async () => {
  const e = env();
  const first = await SoftKey.make();
  const cookie = await addKey(e, first, { token: TOKEN }, 'Keyring');
  const second = await SoftKey.make();
  assert.equal(await addKey(e, second, { cookie }, 'Backup in the drawer'), undefined, 'no new session needed');
  const keys = (await (await call(e, 'GET', '/admin/api/keys', { cookie })).json()).keys;
  assert.deepEqual(keys.map((k) => k.name), ['Keyring', 'Backup in the drawer']);
  assert.equal((await signIn(e, second)).status, 200);
  assert.equal((await signIn(e, first)).status, 200);
});

test('the same key cannot be registered twice', async () => {
  const e = env();
  const key = await SoftKey.make();
  const cookie = await addKey(e, key, { token: TOKEN });
  const opts = await (await call(e, 'POST', '/admin/api/keys/options', { cookie })).json();
  assert.deepEqual(opts.exclude, [b64url.encode(key.id)]);
  const r = await call(e, 'POST', '/admin/api/keys', { cookie, body: await key.create(opts) });
  assert.equal(r.status, 409);
});

test('answers for another site, a replayed answer and a forged signature are refused', async () => {
  const e = env();
  const key = await SoftKey.make();
  await addKey(e, key, { token: TOKEN });

  assert.equal((await signIn(e, key, { origin: 'https://evil.example' })).status, 400);
  assert.equal((await signIn(e, key, { rpId: 'evil.example' })).status, 400);

  const opts = await (await call(e, 'POST', '/auth/options')).json();
  const answer = await key.get(opts);
  assert.equal((await call(e, 'POST', '/auth/verify', { body: answer })).status, 200);
  assert.equal((await call(e, 'POST', '/auth/verify', { body: answer })).status, 400, 'a challenge is used once');

  const impostor = await SoftKey.make();
  impostor.id = key.id;
  impostor.counter = 100;
  const forged = await signIn(e, impostor);
  assert.equal(forged.status, 400);
  assert.match((await forged.json()).error, /signature/);

  const stranger = await SoftKey.make();
  assert.equal((await signIn(e, stranger)).status, 400, 'an unregistered key');
});

test('a counter that goes backwards is refused as a copied key', async () => {
  const e = env();
  const key = await SoftKey.make();
  await addKey(e, key, { token: TOKEN });
  assert.equal((await signIn(e, key)).status, 200);
  key.counter = 0;
  const r = await signIn(e, key);
  assert.equal(r.status, 400);
  assert.match((await r.json()).error, /copied/);
});

test('changes with a session must come from the dashboard itself', async () => {
  const e = env();
  const cookie = await addKey(e, await SoftKey.make(), { token: TOKEN });
  assert.equal((await call(e, 'POST', '/admin/api/cleanup', { cookie, origin: 'https://evil.example' })).status, 403);
  assert.equal((await call(e, 'POST', '/admin/api/cleanup', { cookie })).status, 200);
  assert.equal((await call(e, 'GET', '/admin/api/summary', { cookie, origin: 'https://evil.example' })).status, 200,
    'reading is not a change (and the browser will not send the cookie cross-site anyway)');
});

test('signing out ends the session, and sessions expire', async () => {
  const e = env();
  const cookie = await addKey(e, await SoftKey.make(), { token: TOKEN });
  const out = await call(e, 'POST', '/auth/logout', { cookie });
  assert.match(out.headers.get('set-cookie'), /Max-Age=0/);
  assert.equal((await call(e, 'GET', '/admin/api/summary', { cookie })).status, 401);

  const key = await SoftKey.make();
  const e2 = env();
  const c2 = await addKey(e2, key, { token: TOKEN });
  e2.DB.raw.exec("UPDATE sessions SET expires_at = '2000-01-01T00:00:00.000Z'");
  assert.equal((await call(e2, 'GET', '/admin/api/summary', { cookie: c2 })).status, 401);
});

test('removing the last key lets the token sign in again', async () => {
  const e = env();
  const key = await SoftKey.make();
  const cookie = await addKey(e, key, { token: TOKEN });
  const r = await call(e, 'DELETE', `/admin/api/keys/${b64url.encode(key.id)}`, { cookie });
  assert.deepEqual((await r.json()).keys, []);
  assert.equal((await call(e, 'GET', '/admin/api/summary', { token: TOKEN })).status, 200);
});

test('unanswered sign-in requests are capped', async () => {
  const e = env();
  await addKey(e, await SoftKey.make(), { token: TOKEN });
  let last;
  for (let i = 0; i < 51; i++) last = await call(e, 'POST', '/auth/options');
  assert.equal(last.status, 429);
});

test('the WebAuthn helpers refuse malformed input', () => {
  assert.throws(() => b64url.decode('not base64!'));
  assert.throws(() => cbor(Uint8Array.of(0x5f)), 'indefinite lengths are not read');
  assert.throws(() => cbor(Uint8Array.of(0x58, 10, 1)), 'cut-off bytes');
  assert.throws(() => derToRaw(Uint8Array.of(0x30, 2, 2, 0)));
  const raw = Uint8Array.from({ length: 64 }, (_, i) => (i === 0 || i === 32 ? 0x80 : i));
  assert.deepEqual(derToRaw(der(raw)), raw);
});

test('a synced passkey (Bitwarden) with backup flags and a counter stuck at 0 signs in every time', async () => {
  const e = env();
  const yubikey = await SoftKey.make();
  const cookie = await addKey(e, yubikey, { token: TOKEN }, 'YubiKey 5C');
  const bitwarden = await SoftKey.make();
  bitwarden.synced = true; // BE and BS set, signCount always 0, as Bitwarden does
  await addKey(e, bitwarden, { cookie }, 'Bitwarden');
  for (let i = 0; i < 3; i++) assert.equal((await signIn(e, bitwarden)).status, 200);
  assert.equal((await signIn(e, yubikey)).status, 200);
  const names = (await (await call(e, 'GET', '/admin/api/keys', { cookie })).json()).keys.map((k) => k.name);
  assert.deepEqual(names, ['YubiKey 5C', 'Bitwarden']);
});
