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
5. **signature** = a hash of the panic location (`file:line:col`, the first frame in a Cut Craft crate),
   **not** of the message. The same crash should group together across machines. Hex is fine.
6. Endpoint as a constant that can be overridden (an env var, e.g. `CUTCRAFT_CRASH_URL`) for testing.

## Payload (JSON, `Content-Type: application/json`, no auth, no cookies)

| Field | Rule |
| --- | --- |
| `version` | 1–40 printable ASCII, e.g. `0.6.4` (`CARGO_PKG_VERSION`) |
| `os` | 1–40 printable ASCII, e.g. `Windows 11 x64` |
| `title` | plain-language crash title, 1–200 chars, no control characters |
| `signature` | 8–64 of `A-Z a-z 0-9 _ -` |
| `install_id` | 8–64 of `A-Z a-z 0-9 -` (a UUID) |
| `log` | scrubbed crash log, at most 64 KB UTF-8; the whole body at most 72 KB |

Values are trimmed, and unknown fields are ignored. Trim the log to the last 64 KB rather than letting
the server refuse it.

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
