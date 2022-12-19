CREATE TABLE IF NOT EXISTS sources (
  id INTEGER PRIMARY KEY,
  url TEXT NOT NULL UNIQUE,
  name TEXT,
  description TEXT,
  image_url TEXT,
  created_at INTEGER NOT NULL DEFAULT (
    CAST(
      ROUND((julianday('now') - 2440587.5) * 86400000) As INTEGER
    )
  ),
  updated_at INTEGER NOT NULL DEFAULT (
    CAST(
      ROUND((julianday('now') - 2440587.5) * 86400000) As INTEGER
    )
  )
);
