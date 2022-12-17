CREATE TABLE IF NOT EXISTS sources (
  id INTEGER PRIMARY KEY,
  url TEXT NOT NULL UNIQUE,
  name TEXT,
  description TEXT,
  image_url TEXT,
  created INTEGER NOT NULL
);
