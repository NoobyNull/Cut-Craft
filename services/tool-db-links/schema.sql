-- Vendor tool-database download links. Only links and fingerprints are kept, never the files.
CREATE TABLE IF NOT EXISTS sources (
  id            TEXT PRIMARY KEY,              -- e.g. 'spetool-vectric'
  vendor        TEXT NOT NULL,                 -- shown to people, e.g. 'SpeTool'
  format        TEXT NOT NULL,                 -- 'vectric-vtdb', 'vectric-tool', 'fusion-tools', 'carveco-tdb'
  page_url      TEXT NOT NULL,                 -- the vendor's own download page
  link_match    TEXT,                          -- regex picking the download link out of the page
  version_match TEXT,                          -- regex reading a version from the file name (group 1)
  mode          TEXT NOT NULL DEFAULT 'auto' CHECK (mode IN ('auto', 'manual')),
  enabled       INTEGER NOT NULL DEFAULT 1,
  note          TEXT,
  -- the current link, as last seen
  url           TEXT,
  filename      TEXT,
  version       TEXT,
  sha256        TEXT,
  bytes         INTEGER,
  status        TEXT NOT NULL DEFAULT 'new',   -- ok, manual, needs-link, blocked, link-missing, download-broken, page-unreachable, new
  message       TEXT,
  checked_at    TEXT,
  changed_at    TEXT
);

-- Every version ever seen, newest last.
CREATE TABLE IF NOT EXISTS history (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  source_id TEXT NOT NULL REFERENCES sources (id),
  url       TEXT,
  filename  TEXT,
  version   TEXT,
  sha256    TEXT,
  bytes     INTEGER,
  seen_at   TEXT NOT NULL
);

-- 'last_run': when the last check of all vendors started (at most one a day).
CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT
);
