// Pure rules for incoming reports: field checks and the one-report-per-hour limit.

export const MAX_LOG_BYTES = 64 * 1024;
export const MAX_BODY_BYTES = 72 * 1024;
export const HOUR_MS = 60 * 60 * 1000;
export const SAMPLES_PER_CRASH = 3;

const FIELDS = {
  version: { max: 40, re: /^[\x20-\x7e]+$/ },
  os: { max: 40, re: /^[\x20-\x7e]+$/ },
  title: { max: 200, re: /^[^\x00-\x08\x0b-\x1f\x7f]+$/ },
  signature: { min: 8, max: 64, re: /^[A-Za-z0-9_-]+$/ },
  install_id: { min: 8, max: 64, re: /^[A-Za-z0-9-]+$/ },
};

/**
 * Check a report's fields. Returns {report} when it is good, or {error, status} saying what's wrong.
 * Unknown fields are ignored so a newer CutCraft can add fields without breaking older relays.
 */
export function validate(body) {
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return { status: 400, error: 'Expected a JSON object.' };
  }
  const report = {};
  for (const [name, rule] of Object.entries(FIELDS)) {
    const v = body[name];
    if (typeof v !== 'string' || !v.trim()) return { status: 400, error: `${name} is missing.` };
    const t = v.trim();
    if (t.length < (rule.min ?? 1) || t.length > rule.max) {
      return { status: 400, error: `${name} must be ${rule.min ?? 1} to ${rule.max} characters.` };
    }
    if (!rule.re.test(t)) return { status: 400, error: `${name} has characters that aren't allowed.` };
    report[name] = t;
  }
  if (typeof body.log !== 'string' || !body.log.length) return { status: 400, error: 'log is missing.' };
  if (new TextEncoder().encode(body.log).length > MAX_LOG_BYTES) {
    return { status: 413, error: `log is over ${MAX_LOG_BYTES / 1024} KB.` };
  }
  report.log = body.log;
  return { report };
}

/** Milliseconds this install must still wait before another report is accepted (0 = may send). */
export function waitBeforeNext(lastAtIso, nowMs) {
  if (!lastAtIso) return 0;
  const last = Date.parse(lastAtIso);
  if (Number.isNaN(last)) return 0;
  return Math.max(0, last + HOUR_MS - nowMs);
}

/** YYYY-MM-DD in UTC. */
export const dayOf = (iso) => iso.slice(0, 10);

/** The ISO time `days` before `now`. */
export const daysBefore = (now, days) => new Date(now.getTime() - days * 86400000).toISOString();

/** A positive whole number from a Worker variable, or the fallback. */
export function setting(value, fallback) {
  const n = Number.parseInt(value, 10);
  return Number.isFinite(n) && n > 0 ? n : fallback;
}
