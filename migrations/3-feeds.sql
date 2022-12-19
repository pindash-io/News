CREATE TABLE IF NOT EXISTS feeds (
  id INTEGER PRIMARY KEY,
  source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE ON UPDATE CASCADE,
  title TEXT,
  content TEXT,
  author TEXT,
  created_at INTEGER NOT NULL DEFAULT (
    CAST(
      ROUND((julianday('now') - 2440587.5) * 86400000) As INTEGER
    )
  )
);
