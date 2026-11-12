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
