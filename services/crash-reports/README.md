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

## Bug reports

`POST /v1/bug` takes Report a Bug's reports from anyone, GitHub account or not. It puts the
pictures in `BUG_PICTURE_REPO` (a public repository of their own, so the token never touches
code) and opens the issue in `BUG_REPO`, labelled `bug`, with the `BUG_PICTURE_TOKEN` and `BUG_TOKEN` secrets.
Nothing of a bug report is kept here but a count per network hash, for the caps.

| Field | Rule |
| --- | --- |
| `title` | 1–200 characters on one line |
| `text` | the issue's text, up to 60 KB; every `@name` in it is made so it pings nobody. Reports are anonymous: any `reporter` field is ignored and nothing names the sender |
| `install_id` | as for crashes |
| `pictures` | up to 6 of `{name, data}`: `name` of `A-Z a-z 0-9 . _ -`, `data` base64, up to 10 MB each |
| `work` | proof of work over `cutcraft-bug-v1\n<day>\n<install_id>\n<sha256 of title\ntext>\n<nonce>`, `POW_BITS` zero bits |

Replies: **201** `{url}` of the issue; **400**/**413** refused as it is; **429** with
`Retry-After` (`BUG_IP_DAILY` a network a day, `BUG_DAILY_LIMIT` in all); **502** GitHub refused
it (the error says why); **503** bug reports are not set up (a token missing). On any failure the
app opens the report in the browser instead. Big pictures cost CPU time: on the free plan's
10 ms a request may be cut off, which the app treats as any other failure.

Setting up (once): make the public repository named in `BUG_PICTURE_REPO` with a first commit
(a README). Then two fine-grained tokens, owner Power-Whittle, because a token's permissions
cover every repository it names:

- `BUG_PICTURE_TOKEN`: repository `BUG_PICTURE_REPO` only, **Contents: Read and write**.
- `BUG_TOKEN`: repository `BUG_REPO` only, **Issues: Read and write**. No Contents, so it can't
  touch code.

```sh
npx wrangler secret put BUG_PICTURE_TOKEN
npx wrangler secret put BUG_TOKEN
npx wrangler deploy
```

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
  - **Draft GitHub issue** opens a new-issue page on Power-Whittle/PowerWhittle filled in with the
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

## Running and testing locally

```sh
npm test                                          # Node 22+, node:sqlite stands in for D1
npx wrangler d1 execute cutcraft-crash-reports --local --file schema.sql
echo 'ADMIN_TOKEN=dev-token' > .dev.vars
npm run dev                                       # http://localhost:8787, /__scheduled runs the clean-up
```
