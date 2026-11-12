-- CutCraft crash reports. Raw logs live only in `samples` and are deleted as soon as a
-- crash is marked handled or turned into a GitHub issue, and in any case after RETAIN_DAYS.
-- Everything else is counts, which hold nothing about the user or their work.

-- One row per distinct crash (same panic location = same signature).
CREATE TABLE IF NOT EXISTS crashes (
  signature   TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  status      TEXT NOT NULL DEFAULT 'new',   -- new | handled | issue | back
  issue_url   TEXT,
  note        TEXT,
  count       INTEGER NOT NULL DEFAULT 0,
  installs    INTEGER NOT NULL DEFAULT 0,     -- distinct installs that hit it (approximate)
  first_seen  TEXT NOT NULL,
  last_seen   TEXT NOT NULL,
  first_version TEXT,
  last_version  TEXT,
  handled_at  TEXT
);
CREATE INDEX IF NOT EXISTS crashes_last_seen ON crashes (last_seen);

-- A few example logs per crash, kept only until the crash is dealt with.
CREATE TABLE IF NOT EXISTS samples (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  signature   TEXT NOT NULL,
  version     TEXT NOT NULL,
  os          TEXT NOT NULL,
  log         TEXT NOT NULL,
  received_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS samples_sig ON samples (signature);
CREATE INDEX IF NOT EXISTS samples_age ON samples (received_at);

-- Counts for the charts: per crash, per day, version and operating system.
CREATE TABLE IF NOT EXISTS daily (
  day         TEXT NOT NULL,                  -- YYYY-MM-DD (UTC)
  signature   TEXT NOT NULL,
  version     TEXT NOT NULL,
  os          TEXT NOT NULL,
  count       INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (day, signature, version, os)
);

-- Which installs already reported which crash, as a salted hash, so `installs` can be counted
-- without keeping the install id. Cleared with the crash's samples.
CREATE TABLE IF NOT EXISTS seen (
  signature   TEXT NOT NULL,
  who         TEXT NOT NULL,
  PRIMARY KEY (signature, who)
);

-- Rate limit: last time each install (salted hash) sent a report. Rows older than a day are deleted.
CREATE TABLE IF NOT EXISTS senders (
  who         TEXT PRIMARY KEY,
  last_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Security keys (WebAuthn, e.g. YubiKeys) that sign in to the dashboard. Once one exists the
-- admin token no longer signs in. Lost every key: DELETE FROM passkeys (with wrangler), and the
-- token works again.
CREATE TABLE IF NOT EXISTS passkeys (
  id           TEXT PRIMARY KEY,            -- credential id, base64url
  name         TEXT NOT NULL,
  public_key   TEXT NOT NULL,               -- P-256 public key as a JWK (JSON)
  sign_count   INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL,
  last_used_at TEXT
);

-- Signed-in dashboard sessions. Only the SHA-256 of the cookie is kept.
CREATE TABLE IF NOT EXISTS sessions (
  id          TEXT PRIMARY KEY,
  created_at  TEXT NOT NULL,
  expires_at  TEXT NOT NULL
);

-- One-use WebAuthn challenges, good for five minutes.
CREATE TABLE IF NOT EXISTS challenges (
  challenge   TEXT PRIMARY KEY,
  purpose     TEXT NOT NULL,                -- login | register
  expires_at  TEXT NOT NULL
);

-- Reports per network per day, for the per-IP allowance. `who` is a hash of the IP with a
-- random salt made each day (meta 'ip_salt:<day>'); rows and salts are deleted after the day.
CREATE TABLE IF NOT EXISTS ip_counts (
  day         TEXT NOT NULL,
  who         TEXT NOT NULL,
  signature   TEXT NOT NULL,
  count       INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (day, who, signature)
);

-- Refused reports per day, by reason and network hash (as ip_counts; never the IP), so the
-- dashboard can show junk arriving. Deleted after 30 days.
CREATE TABLE IF NOT EXISTS refusals (
  day         TEXT NOT NULL,
  reason      TEXT NOT NULL,
  who         TEXT NOT NULL,
  count       INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (day, reason, who)
);
