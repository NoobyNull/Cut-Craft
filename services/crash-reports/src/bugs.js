// Bug reports from Report a Bug, for people who aren't collaborators on the repository (or
// have no GitHub account at all). The app sends the report's text and its pictures here; the
// pictures go to a public pictures repository (BUG_PICTURE_REPO, with BUG_PICTURE_TOKEN) and
// the report becomes an issue in BUG_REPO (with BUG_TOKEN). Two tokens, because a fine-grained
// token's permissions cover every repository it names: the pictures token can write only the
// pictures repository, and the issue token only BUG_REPO's issues, so neither can touch code.
// Nothing about a bug report is stored here but its count, for the per-network and daily caps.
import { dayOf, setting } from './rules.js';
import { zeroBits } from './guard.js';

export const BUG_MAX_BODY_BYTES = 40 * 1024 * 1024;
export const MAX_TEXT_BYTES = 60 * 1024;
export const MAX_PICTURE_BYTES = 10 * 1024 * 1024;
export const MAX_PICTURES = 6;
/** The ip_counts "signature" bug reports are counted under (crash IDs all start `cc-`). */
export const BUG_COUNT = 'bug-report';
const GITHUB_LOGIN = /^[A-Za-z0-9](?:[A-Za-z0-9]|-(?=[A-Za-z0-9])){0,38}$/;
const PICTURE_NAME = /^[A-Za-z0-9._-]{1,80}$/;
const FOOTER = "_Sent from PowerWhittle's Report a Bug._";

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
const sha256 = async (text) => new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text)));

/** Bytes a base64 string decodes to. */
const decodedBytes = (b64) => Math.floor((b64.length * 3) / 4) - (b64.endsWith('==') ? 2 : b64.endsWith('=') ? 1 : 0);

/** Check a bug report's fields: {bug} when good, or {error, status}. */
export function validateBug(body) {
  if (!body || typeof body !== 'object' || Array.isArray(body)) return { status: 400, error: 'Expected a JSON object.' };
  const title = typeof body.title === 'string' ? body.title.trim() : '';
  if (!title || title.length > 200 || /[\x00-\x1f\x7f]/.test(title)) {
    return { status: 400, error: 'title must be 1 to 200 characters on one line.' };
  }
  if (typeof body.text !== 'string') return { status: 400, error: 'text is missing.' };
  if (new TextEncoder().encode(body.text).length > MAX_TEXT_BYTES) {
    return { status: 413, error: `text is over ${MAX_TEXT_BYTES / 1024} KB.` };
  }
  const install = body.install_id;
  if (typeof install !== 'string' || !/^[A-Za-z0-9-]{8,64}$/.test(install)) {
    return { status: 400, error: 'install_id must be 8 to 64 letters, digits or dashes.' };
  }
  let reporter = null;
  if (body.reporter != null && body.reporter !== '') {
    const name = String(body.reporter).trim().replace(/^@/, '');
    if (!GITHUB_LOGIN.test(name)) return { status: 400, error: 'reporter is not a GitHub user name.' };
    reporter = name;
  }
  const list = body.pictures ?? [];
  if (!Array.isArray(list) || list.length > MAX_PICTURES) {
    return { status: 400, error: `pictures must be a list of at most ${MAX_PICTURES}.` };
  }
  const pictures = [];
  for (const p of list) {
    if (!p || typeof p.name !== 'string' || !PICTURE_NAME.test(p.name)) {
      return { status: 400, error: 'a picture name may hold only letters, digits, dots, dashes and underscores.' };
    }
    // only the shape is checked here: reading megabytes would cost more CPU time than a
    // Worker may have, and GitHub refuses anything that isn't base64 anyway
    if (typeof p.data !== 'string' || !p.data || p.data.length % 4 !== 0 || /[^A-Za-z0-9+/=]/.test(p.data.slice(0, 64))) {
      return { status: 400, error: `${p.name} is not base64.` };
    }
    if (decodedBytes(p.data) > MAX_PICTURE_BYTES) {
      return { status: 413, error: `${p.name} is over ${MAX_PICTURE_BYTES / 1024 / 1024} MB.` };
    }
    pictures.push({ name: p.name, data: p.data });
  }
  return { bug: { title, text: body.text, install_id: install, reporter, pictures } };
}

/**
 * The text the app hashes for a bug report's proof of work, nonce last. The pictures are left
 * out: hashing megabytes would cost the Worker more CPU time than it may have, and the
 * per-network cap holds whatever pictures come with a proof.
 */
export async function bugWorkText(bug, day, nonce) {
  const all = `${bug.title}\n${bug.text}`;
  return `cutcraft-bug-v1\n${day}\n${bug.install_id}\n${hex(await sha256(all))}\n${nonce}`;
}

/** Whether `nonce` proves work for this bug report, made today or yesterday (UTC). */
export async function checkBugWork(bug, nonce, bits, now) {
  if (bits <= 0) return true;
  if (typeof nonce !== 'string' || !/^[0-9a-z]{1,24}$/.test(nonce)) return false;
  for (const day of [dayOf(now.toISOString()), dayOf(new Date(now.getTime() - 86400000).toISOString())]) {
    if (zeroBits(await sha256(await bugWorkText(bug, day, nonce))) >= bits) return true;
  }
  return false;
}

/**
 * `text` with every @name made into one that pings nobody (a zero-width space after the @), so
 * the relay can't be used to notify people. Only the reporter's own name, which the relay adds,
 * pings.
 */
export function quiet(text) {
  return text.replace(/@(?=[A-Za-z0-9])/g, '@​');
}

/** The issue's text: the report, its pictures, who sent it, then the footer. */
export function issueBody(bug, links) {
  let body = quiet(bug.text.trim() || '_No description._');
  if (links.length) {
    body += '\n\n### Pictures\n\n';
    body += links.map(({ name, url }) => `![${name}](${url}?raw=true)`).join('\n');
  }
  body += '\n\n';
  if (bug.reporter) body += `Reported by @${bug.reporter}.\n\n`;
  return `${body}${FOOTER}\n`;
}

/** Calls the GitHub REST API with the relay's token; the parsed answer, or throws. */
async function github(env, token, method, path, body) {
  const call = env.GITHUB_FETCH ?? fetch;
  const r = await call(`https://api.github.com${path}`, {
    method,
    headers: {
      authorization: `Bearer ${token}`,
      accept: 'application/vnd.github+json',
      'x-github-api-version': '2022-11-28',
      'user-agent': 'PowerWhittle-bug-relay',
      'content-type': 'application/json',
    },
    body: JSON.stringify(body),
  });
  const answer = await r.json().catch(() => ({}));
  if (!r.ok) {
    const e = new Error(`GitHub answered ${r.status}${answer.message ? `: ${answer.message}` : ''}`);
    e.status = r.status;
    throw e;
  }
  return answer;
}

/** Puts the pictures in the pictures repository; their names and addresses. */
async function uploadPictures(env, bug, now) {
  if (!bug.pictures.length) return [];
  const folder = `${dayOf(now.toISOString())}/${hex(crypto.getRandomValues(new Uint8Array(6)))}`;
  const links = [];
  for (const [i, p] of bug.pictures.entries()) {
    const answer = await github(env, env.BUG_PICTURE_TOKEN, 'PUT', `/repos/${env.BUG_PICTURE_REPO}/contents/${folder}/${i + 1}-${p.name}`, {
      message: `Picture for bug report: ${bug.title}`.slice(0, 200),
      content: p.data,
    });
    links.push({ name: p.name, url: answer.content.html_url });
  }
  return links;
}

/** Opens the issue, labelled `bug` (without the label when the repository refuses it). */
async function openIssue(env, bug, links) {
  const issue = { title: bug.title, body: issueBody(bug, links) };
  try {
    return await github(env, env.BUG_TOKEN, 'POST', `/repos/${env.BUG_REPO}/issues`, { ...issue, labels: ['bug'] });
  } catch (e) {
    if (e.status !== 422) throw e;
    return github(env, env.BUG_TOKEN, 'POST', `/repos/${env.BUG_REPO}/issues`, issue);
  }
}

/** Seconds until midnight UTC when this network or everyone has sent enough today, or 0. */
async function bugWait(db, who, ipDaily, dailyLimit, now) {
  const day = dayOf(now.toISOString());
  const row = await db.prepare(
    `SELECT COALESCE(SUM(count), 0) AS total,
            COALESCE(SUM(CASE WHEN who = ? THEN count ELSE 0 END), 0) AS mine
     FROM ip_counts WHERE day = ? AND signature = ?`,
  ).bind(who, day, BUG_COUNT).first();
  if (row.total < dailyLimit && row.mine < ipDaily) return 0;
  return Math.ceil((Date.parse(`${day}T00:00:00Z`) + 86400000 - now.getTime()) / 1000);
}

export const bugSettings = (env) => ({
  ipDaily: setting(env.BUG_IP_DAILY, 5),
  dailyLimit: setting(env.BUG_DAILY_LIMIT, 50),
  powBits: env.POW_BITS === '0' ? 0 : setting(env.POW_BITS, 22),
});

/**
 * Takes a bug report: checks it, its proof of work and the caps, then makes the issue.
 * `ipHash` turns the sender's IP into the day's salted hash; `count` records one more report,
 * counted before GitHub is called so a burst can't slip past the caps, and `uncount` takes it
 * back when GitHub refuses, so a report that wasn't filed doesn't use up the allowance.
 */
export async function takeBug(request, env, now, { ipHash, count, uncount, refuse, json }) {
  if (!env.BUG_TOKEN || !env.BUG_PICTURE_TOKEN || !env.BUG_REPO || !env.BUG_PICTURE_REPO) {
    return json({ error: 'Bug reports are not set up on this service.' }, 503);
  }
  const declared = Number(request.headers.get('content-length') ?? 0);
  if (declared > BUG_MAX_BODY_BYTES) return refuse('too big', 413, 'Bug report is too big.');
  const raw = new Uint8Array(await request.arrayBuffer());
  if (raw.length > BUG_MAX_BODY_BYTES) return refuse('too big', 413, 'Bug report is too big.');
  let body;
  try {
    body = JSON.parse(new TextDecoder().decode(raw));
  } catch {
    return refuse('not JSON', 400, 'Bug report is not valid JSON.');
  }
  const checked = validateBug(body);
  if (checked.error) return refuse(checked.status === 413 ? 'too big' : 'bad fields', checked.status, checked.error);
  const { bug } = checked;
  const s = bugSettings(env);
  if (!(await checkBugWork(bug, body.work, s.powBits, now))) {
    return refuse('no proof of work', 400, 'work is missing or wrong (see APP-HOOKUP.md).');
  }
  const who = await ipHash(now);
  const wait = await bugWait(env.DB, who, s.ipDaily, s.dailyLimit, now);
  if (wait > 0) {
    return refuse('bug limit', 429, 'Too many bug reports today; try again tomorrow.', { 'retry-after': String(wait) });
  }
  await count(BUG_COUNT, now);
  let issue;
  try {
    issue = await openIssue(env, bug, await uploadPictures(env, bug, now));
  } catch (e) {
    await uncount(BUG_COUNT, now);
    return json({ error: `The bug report could not be filed: ${e.message}` }, 502);
  }
  return json({ url: issue.html_url }, 201);
}
