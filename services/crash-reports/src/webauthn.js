// WebAuthn (security keys such as YubiKeys) with nothing but the Web Crypto API.
//
// Only ES256 (ECDSA on P-256) is accepted: every FIDO2 security key supports it, and one
// algorithm is less to get wrong. Attestation is not checked. The page asks for 'none', and a
// key is trusted because whoever registered it was already signed in.

export const b64url = {
  encode(bytes) {
    let s = '';
    for (const b of new Uint8Array(bytes)) s += String.fromCharCode(b);
    return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  },
  decode(str) {
    if (typeof str !== 'string' || !/^[A-Za-z0-9_-]*$/.test(str)) throw new Error('Expected base64url text.');
    const bin = atob(str.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((str.length + 3) % 4));
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
  },
};

const utf8 = (text) => new TextEncoder().encode(text);
export const sha256 = async (bytes) => new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));

function same(a, b) {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
  return diff === 0;
}

function concat(a, b) {
  const out = new Uint8Array(a.length + b.length);
  out.set(a);
  out.set(b, a.length);
  return out;
}

/** Decode one CBOR item from the start of `bytes`. Returns {value, length}. Maps become Map. */
export function cbor(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let p = 0;
  const need = (n) => {
    if (n < 0 || p + n > bytes.length) throw new Error('The key sent cut-off data.');
  };
  const arg = (info) => {
    if (info < 24) return info;
    if (info === 24) { need(1); return bytes[p++]; }
    if (info === 25) { need(2); const x = view.getUint16(p); p += 2; return x; }
    if (info === 26) { need(4); const x = view.getUint32(p); p += 4; return x; }
    throw new Error('The key sent data this server does not read.');
  };
  const item = (depth) => {
    if (depth > 8) throw new Error('The key sent data nested too deeply.');
    need(1);
    const head = bytes[p++];
    const major = head >> 5;
    const info = head & 31;
    if (major === 7) {
      if (info === 20) return false;
      if (info === 21) return true;
      if (info === 22) return null;
      throw new Error('The key sent data this server does not read.');
    }
    const n = arg(info);
    switch (major) {
      case 0: return n;
      case 1: return -1 - n;
      case 2: { need(n); const out = bytes.slice(p, p + n); p += n; return out; }
      case 3: { need(n); const out = new TextDecoder().decode(bytes.subarray(p, p + n)); p += n; return out; }
      case 4: {
        if (n > 256) throw new Error('The key sent too much data.');
        const out = [];
        for (let i = 0; i < n; i++) out.push(item(depth + 1));
        return out;
      }
      case 5: {
        if (n > 256) throw new Error('The key sent too much data.');
        const out = new Map();
        for (let i = 0; i < n; i++) {
          const k = item(depth + 1);
          out.set(k, item(depth + 1));
        }
        return out;
      }
      default: throw new Error('The key sent data this server does not read.');
    }
  };
  const value = item(0);
  return { value, length: p };
}

/** The authenticator data: which site, whether the key was touched / PIN given, counter, new key. */
export function parseAuthData(a) {
  if (!(a instanceof Uint8Array) || a.length < 37) throw new Error('The key sent too little data.');
  const flags = a[32];
  const out = {
    rpIdHash: a.slice(0, 32),
    up: (flags & 0x01) !== 0,
    uv: (flags & 0x04) !== 0,
    signCount: new DataView(a.buffer, a.byteOffset + 33, 4).getUint32(0),
  };
  if (flags & 0x40) {
    if (a.length < 55) throw new Error('The key sent too little data.');
    const len = (a[53] << 8) | a[54];
    if (len < 1 || a.length < 55 + len) throw new Error('The key sent too little data.');
    out.credentialId = a.slice(55, 55 + len);
    out.publicKey = cbor(a.subarray(55 + len)).value;
  }
  return out;
}

/** A COSE EC2 P-256 ES256 key as a JWK; anything else is refused. */
export function coseToJwk(cose) {
  const x = cose instanceof Map ? cose.get(-2) : null;
  const y = cose instanceof Map ? cose.get(-3) : null;
  if (!(cose instanceof Map) || cose.get(1) !== 2 || cose.get(3) !== -7 || cose.get(-1) !== 1
    || !(x instanceof Uint8Array) || !(y instanceof Uint8Array) || x.length !== 32 || y.length !== 32) {
    throw new Error('This key offered a kind of signature this dashboard does not accept (it needs ES256).');
  }
  return { kty: 'EC', crv: 'P-256', x: b64url.encode(x), y: b64url.encode(y) };
}

/** WebAuthn sends ECDSA signatures in DER; Web Crypto wants the two 32-byte numbers side by side. */
export function derToRaw(der) {
  let p = 0;
  const byte = () => {
    if (p >= der.length) throw new Error('The signature is malformed.');
    return der[p++];
  };
  if (byte() !== 0x30) throw new Error('The signature is malformed.');
  const total = byte();
  if (total & 0x80 || p + total !== der.length) throw new Error('The signature is malformed.');
  const int = () => {
    if (byte() !== 0x02) throw new Error('The signature is malformed.');
    const n = byte();
    if (n < 1 || n > 33 || p + n > der.length) throw new Error('The signature is malformed.');
    let v = der.slice(p, p + n);
    p += n;
    while (v.length > 32 && v[0] === 0) v = v.slice(1);
    if (v.length > 32) throw new Error('The signature is malformed.');
    const out = new Uint8Array(32);
    out.set(v, 32 - v.length);
    return out;
  };
  const r = int();
  const s = int();
  if (p !== der.length) throw new Error('The signature is malformed.');
  return concat(r, s);
}

function checkClientData(bytes, type, challenge, origin) {
  let c;
  try {
    c = JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    throw new Error("The browser's answer could not be read.");
  }
  if (c?.type !== type) throw new Error('The browser answered a different request.');
  if (c.challenge !== challenge) throw new Error('That request has expired. Try again.');
  if (c.origin !== origin) throw new Error('The key was used from a different site.');
  if (c.crossOrigin === true) throw new Error('The key was used from inside another site.');
}

async function checkSite(auth, rpId) {
  if (!same(auth.rpIdHash, await sha256(utf8(rpId)))) throw new Error('The key answered for a different site.');
  if (!auth.up) throw new Error('The key was not touched.');
}

const importKey = (jwk) =>
  crypto.subtle.importKey('jwk', jwk, { name: 'ECDSA', namedCurve: 'P-256' }, false, ['verify']);

/**
 * Check what navigator.credentials.create() gave back (fields base64url-encoded).
 * Returns {credentialId, publicKey (JWK), signCount}.
 */
export async function verifyRegistration({ clientDataJSON, attestationObject }, { challenge, origin, rpId }) {
  checkClientData(b64url.decode(clientDataJSON), 'webauthn.create', challenge, origin);
  const att = cbor(b64url.decode(attestationObject)).value;
  const authData = att instanceof Map ? att.get('authData') : null;
  if (!(authData instanceof Uint8Array)) throw new Error('The key sent no authenticator data.');
  const auth = parseAuthData(authData);
  await checkSite(auth, rpId);
  if (!auth.credentialId) throw new Error('The key did not send a new credential.');
  const publicKey = coseToJwk(auth.publicKey);
  await importKey(publicKey); // refuses a point that is not on the curve
  return { credentialId: b64url.encode(auth.credentialId), publicKey, signCount: auth.signCount };
}

/**
 * Check what navigator.credentials.get() gave back against the stored key.
 * Returns {signCount}. Throws if anything is off, including a counter that did not go up.
 */
export async function verifyAssertion(
  { clientDataJSON, authenticatorData, signature },
  { challenge, origin, rpId, publicKey, signCount },
) {
  const clientBytes = b64url.decode(clientDataJSON);
  checkClientData(clientBytes, 'webauthn.get', challenge, origin);
  const authBytes = b64url.decode(authenticatorData);
  const auth = parseAuthData(authBytes);
  await checkSite(auth, rpId);
  const signed = concat(authBytes, await sha256(clientBytes));
  const ok = await crypto.subtle.verify(
    { name: 'ECDSA', hash: 'SHA-256' }, await importKey(publicKey), derToRaw(b64url.decode(signature)), signed,
  );
  if (!ok) throw new Error("The key's signature did not match.");
  if ((auth.signCount !== 0 || signCount !== 0) && auth.signCount <= signCount) {
    throw new Error("This key's counter went backwards, so it may have been copied. Sign-in refused.");
  }
  return { signCount: auth.signCount };
}
