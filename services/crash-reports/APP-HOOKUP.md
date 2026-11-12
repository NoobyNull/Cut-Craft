# Hook CutCraft up to the crash reporter

For the CutCraft project. The crash-report relay from PR #268 (`services/crash-reports/`) is
**deployed and live** (2026-11-12). What's left is the app side: CutCraft sending its crash logs to it.

- Relay: https://cutcraft-crash-reports.mattyax.workers.dev
- App endpoint: `POST https://cutcraft-crash-reports.mattyax.workers.dev/v1/crash`
- D1 database `cutcraft-crash-reports` (id `4c4b6d9c-95b9-4784-8f03-9afa441b5331`, already in
  `wrangler.toml` on PR #268). Schema applied. Daily clean-up runs at 03:17 UTC.
- Dashboard: open the relay address and paste the admin token. Matthew has it in
  `~/Projects/MyMiniApp.dev/Cutcraft/crash-reports-ADMIN_TOKEN.txt`. It must never go in the repo or the app.

## What the app already has

- `crates/bug-report/src/logs.rs` writes `crash.log` on a panic (version, OS, panic, backtrace,
  last 200 log lines), and `take_crash(dir)` hands it to the next start once.
- The next start already offers that log as a GitHub issue through `gh`. The relay is for
  installs without `gh`, and for counting crashes across everyone.
- `ureq` 3 is already a dependency (`crates/updater`, `crates/machines`), so no new HTTP crate is needed.

## What to build

1. **Opt-in.** Ask once, in plain words, whether to send crash reports automatically. Store the answer
   in settings, with a way to change it later. Send nothing until the user says yes.
2. **install_id.** A random UUID made once and stored in settings. The server keeps only a salted hash.
3. **On the next start after a crash** (never from inside the panicking process), if the user opted in,
   build the report from the `take_crash` text and POST it. If the send fails with 5xx or a network
   error, keep the report on disk and try again on a later start.
4. **Scrub the log before sending.** Replace the home folder with `~`, and strip the user name and the
   names of the user's project files. Backtrace paths from the build machine are fine.
5. **signature**: the relay now works out the crash ID itself (see below), so any stable value is fine,
   e.g. a hash of the panic location.
6. Endpoint as a constant that can be overridden (an env var, e.g. `CUTCRAFT_CRASH_URL`) for testing.

## What the relay checks (so junk is refused)

- **The log must be the one `crash_text` writes**, unchanged in layout: it starts
  `Cut Craft <version> on <os> crashed: panicked at <file>:<line>:<col>:` followed by the panic
  message, then a blank line and `Backtrace:`. `<version>` must equal the `version` field and look
  like a release (`0.6.4`, `0.7.0-beta.1`). Scrub paths *inside* the message and log lines, but keep
  this layout.
- **The relay works out the crash ID itself** from the panicked file (from `crates/` on) and the panic
  message, with numbers, addresses and paths taken out. It is the same on every machine and version.
  The `signature` the app sends is still required (any 8–64 `A-Za-z0-9_-`) but no longer used for grouping.
- **Proof of work** (`work` field, required). Find a nonce (lowercase base-36 text, 1–24 chars, e.g.
  a counter in base 36) such that SHA-256 of this UTF-8 text starts with at least **22 zero bits**:

  ```
  cutcraft-crash-v1\n<YYYY-MM-DD, today in UTC>\n<install_id>\n<hex SHA-256 of log>\n<nonce>
  ```

  (`\n` is a newline; the hex is lowercase.) About 4 million tries: a fraction of a second in release
  Rust, so do it on a background thread when sending. Work made yesterday (UTC) is still accepted. The
  number of bits is `POW_BITS` in `wrangler.toml`; if it is raised, old app versions still send the
  smaller proof and get 400, so only raise it together with an app update.
- **Per network**: 2 reports of the same crash and 10 in all per IP per day, on top of the existing
  1 per install per hour and 2000 a day overall. Over it: 429 with `Retry-After`. IPs are not stored,
  only a hash with a random salt made each day and deleted after the day.

## Payload (JSON, `Content-Type: application/json`, no auth, no cookies)

| Field | Rule |
| --- | --- |
| `version` | 1–40 printable ASCII, e.g. `0.6.4` (`CARGO_PKG_VERSION`) |
| `os` | 1–40 printable ASCII, e.g. `Windows 11 x64` |
| `title` | plain-language crash title, 1–200 chars, no control characters |
| `signature` | 8–64 of `A-Z a-z 0-9 _ -` |
| `install_id` | 8–64 of `A-Z a-z 0-9 -` (a UUID) |
| `log` | scrubbed crash log, at most 64 KB UTF-8; the whole body at most 72 KB |
| `work` | the proof-of-work nonce (see above) |

Values are trimmed, and unknown fields are ignored. If the log is over 64 KB, cut lines from the start of its `Log:` section (keep
the first line and the backtrace) rather than letting the server refuse it.

## Replies

- **202**: accepted. Delete the queued report.
- **400 / 413**: bad or too big. Don't retry. Drop it and note it in the local log.
- **429**: wait for `Retry-After` seconds. The limit is 1 report per install per hour, and 2000 a day overall.
- **5xx / network error**: keep the report queued and retry on a later start.

## Testing

Unit-test the report builder against the rules above (lengths, character sets, the 64 KB trim,
scrubbing, a stable signature). **Don't send test reports to the live relay**, because they show up on
the real dashboard. For an end-to-end check, run the Worker locally: in `services/crash-reports`,
`npm run dev` serves http://localhost:8787. Point `CUTCRAFT_CRASH_URL` there.
