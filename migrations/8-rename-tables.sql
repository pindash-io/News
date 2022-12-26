ALTER TABLE feeds RENAME TO articles;

ALTER TABLE feed_authors RENAME TO article_authors;
ALTER TABLE article_authors RENAME COLUMN f TO t;

ALTER TABLE sources RENAME TO feeds;
ALTER TABLE authors RENAME COLUMN source_id TO article_id;

ALTER TABLE folder_sources RENAME TO folder_feeds;
ALTER TABLE folder_feeds RENAME COLUMN f TO d;
ALTER TABLE folder_feeds RENAME COLUMN s TO f;

DROP INDEX IF EXISTS unique_feeds_url;
DROP INDEX IF EXISTS unique_sources_url;

CREATE UNIQUE INDEX IF NOT EXISTS unique_articles_url ON articles (url);
CREATE UNIQUE INDEX IF NOT EXISTS unique_feeds_url ON feeds (url);