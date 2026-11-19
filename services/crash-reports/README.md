# CutCraft crash reports

A Cloudflare Worker with a D1 database that takes crash reports from CutCraft installs whose
owners agreed to send them, groups them by crash, and shows the maintainer a private dashboard.

- **Grouped by crash.** Reports with the same `signature` (a hash of the panic location) are one
  crash, with a count, how many installs hit it, which versions and which systems.
- **Logs are cleared as they are dealt with.** At most 3 example logs are kept per crash (a report
  from a new version replaces the oldest). Marking a crash *handled*, or saving the GitHub issue it
  became, deletes its logs at once. Every log is deleted after `RETAIN_DAYS` (30) regardless, by a
  daily clean-up. After that only counts remain, which hold nothing about the user or their work.
- **A crash that comes back.** A new report for a *handled* crash marks it *Came back* and starts
  keeping example logs again. Reports for a crash that is a GitHub issue are only counted.
- **No personal data kept.** The `install_id` is stored only as a salted SHA-256 hash (salt made
  once, kept in D1). No IP addresses are stored.
- **Junk is refused.** A report must carry CutCraft's own crash-log layout and a small proof of work
  (see `APP-HOOKUP.md`), and one network (IP) may send 2 reports of a crash and 10 in all a day. IPs
  are never stored: only a hash with a random salt made each day and deleted after it. Refused
  reports are counted by reason on the dashboard.
- **Crash ID worked out here.** The relay groups reports by an ID it makes from the panicked file and
  the panic message (numbers, addresses and paths taken out), so it is the same on every machine and
  version and no client can choose it.
- **Limits.** One report per install per hour; `DAILY_LIMIT` (2000) reports a day across
  everyone; a log over 64 KB or a body over 72 KB is refused.
- **Private dashboard, signed in with security keys.** `GET /` is a sign-in page with no data in
  it. Everything shown comes from `/admin/api/*`, which needs a signed-in session. You sign in by
  touching a registered security key (WebAuthn, e.g. a YubiKey). The admin token only signs in
  until the first key is registered; it is how that first key gets added. Search engines are told
  not to index it.

## What CutCraft sends

`POST /v1/crash`, `Content-Type: application/json`:

| Field | Rule |
| --- | --- |
| `version` | CutCraft version, 1–40 printable characters, e.g. `0.6.4` |
| `os` | 1–40 printable characters, e.g. `Windows 11 23H2 x86_64`, `macOS 14 Sonoma aarch64`, `Debian GNU/Linux 12 x86_64`; just `windows` / `macos` / `linux` when the person switched the computer's details off |
| `title` | plain-language crash title, 1–200 characters |
| `signature` | 8–64 of `A-Z a-z 0-9 _ -` (a hex hash is fine) |
| `install_id` | random id made at install, 8–64 of `A-Z a-z 0-9 -` (a UUID) |
| `log` | the scrubbed crash log, up to 64 KB (UTF-8 bytes) |
| `work` | proof-of-work nonce; see `APP-HOOKUP.md` |

Other fields are ignored. Replies: **202** taken; **400** a field is wrong (don't retry);
**413** too big (don't retry); **429** with `Retry-After` seconds (same install within the hour,
or the day's limit reached); **5xx** try later. No CORS, no cookies.

## Deploying (once)

Needs a Cloudflare account (the free plan is enough) and Node 22 or newer.

```sh
cd services/crash-reports
npm install
npx wrangler login
npx wrangler d1 create cutcraft-crash-reports      # paste the database_id into wrangler.toml
npx wrangler d1 execute cutcraft-crash-reports --remote --file schema.sql
npx wrangler secret put ADMIN_TOKEN                # a long random string, e.g. `openssl rand -hex 32`
npx wrangler deploy
```

`wrangler deploy` prints the address, e.g. `https://cutcraft-crash-reports.<you>.workers.dev`.
CutCraft sends to `<address>/v1/crash`. Open `<address>/`, paste the token, and add your security
keys (at least two, so one is a backup). From then on only the keys sign in.

When a new version adds tables, run the `schema.sql` line again before deploying: it only creates
what is missing.

To change how long logs are kept, edit `RETAIN_DAYS` in `wrangler.toml` and deploy again.

## Security keys

- Sign-in uses WebAuthn with ES256 (every FIDO2 key, YubiKeys included), checked with Web Crypto
  only; no dependency. The key must be touched; its PIN is used if set. Keys are bound to this
  Worker's address, so a key registered on `*.workers.dev` will not work on a custom domain.
- A sign-in lasts 12 hours, in an `HttpOnly`, `Secure`, `SameSite=Strict` cookie; only its hash is
  stored. Changes are refused unless they come from the dashboard page itself.
- A key whose use counter goes backwards is refused, as a sign it was copied.
- **Lost every key?** Clear them with wrangler (needs your Cloudflare login); the token then signs in
  again so you can add new ones:
  `npx wrangler d1 execute cutcraft-crash-reports --remote --command "DELETE FROM passkeys; DELETE FROM sessions"`

Optional extra lock: put the dashboard behind Cloudflare Access (Zero Trust ▸ Access ▸
Applications, path `/` and `/admin/*`, allow only your email). Leave `/v1/crash` outside it, or
CutCraft can't send.

## Dashboard

- Reports per day, by version and by system (Windows, macOS, Linux, then each exact system), for the last 7, 30, 90 or 365 days; the **from** list shows one kind of system on its own.
- Every crash with its status: **New**, **Came back**, **GitHub issue**, **Handled**.
- Click a crash to read its example logs, copy or delete one, and then:
  - **Mark handled and delete logs** (with an optional note such as "fixed in 0.6.5");
  - **Draft GitHub issue** opens a new-issue page on NoobyNull/Cut-Craft filled in with the
    counts and the newest log. Read it for anything personal before posting, then paste the
    issue's address and **Save issue link and delete logs**;
  - **Reopen**, or **Forget this crash completely** (counts too).

The admin API, all needing a signed-in session (or `Authorization: Bearer <ADMIN_TOKEN>` while no
security key is registered):
`GET /admin/api/summary?days=30`, `GET /admin/api/crashes/<signature>`,
`POST /admin/api/crashes/<signature>/handled` `{note?}`, `POST …/issue` `{url}`, `POST …/reopen`,
`DELETE /admin/api/crashes/<signature>`, `DELETE /admin/api/samples/<id>`, `POST /admin/api/cleanup`,
`GET /admin/api/keys`, `POST /admin/api/keys/options`, `POST /admin/api/keys`, `DELETE /admin/api/keys/<id>`.
Sign-in: `GET /auth/status`, `POST /auth/options`, `POST /auth/verify`, `POST /auth/logout`.

## Messages and votes

The same Worker also carries the **messages and opinion polls** CutCraft shows at startup
(Help ▸ Messages from the Cut Craft Team, in 0.6.19 to 0.6.21).

- **Publish one** at `/votes` (signed in as for the dashboard; the dashboard's header links
  to it): a title, a message, and choices one per line (none for a plain message, 2 to 6 for
  a poll). One shows at a time: publishing ends the last. Publishing again with the same
  short name edits it and keeps its votes. **Stop showing it** ends it; **Delete** removes it
  and its votes.
- `GET /v1/announcement` gives CutCraft the live one, `{id, title, body, choices,
  show_results}`, or **204** when there is none.
- `POST /v1/vote` `{poll, choice, install_id, version}` takes one vote per install per poll
  (a later vote replaces it). It answers `{ok: true, totals: [..]}`, the totals only when the
  poll shows results. **400** bad field, **404** no such poll, **410** ended, **429** with
  `Retry-After` when one network has cast `VOTES_PER_IP` (20) votes in that poll today.
- Votes keep only a salted hash of the poll and install id, so votes can't be linked to each
  other across polls or to crash reports. No IPs are kept (the per-network count uses the
  same daily-salted hash as crash reports).
- Admin API: `GET /admin/api/polls`, `POST /admin/api/polls` (publish), `POST
  /admin/api/polls/<id>/end`, `DELETE /admin/api/polls/<id>`.

After pulling this, run the `schema.sql` line from *Deploying* again (it adds the `polls` and
`votes` tables) and deploy.

## Running and testing locally

```sh
npm test                                          # Node 22+, node:sqlite stands in for D1
npx wrangler d1 execute cutcraft-crash-reports --local --file schema.sql
echo 'ADMIN_TOKEN=dev-token' > .dev.vars
npm run dev                                       # http://localhost:8787, /__scheduled runs the clean-up
```
