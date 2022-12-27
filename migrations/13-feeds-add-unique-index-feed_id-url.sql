DROP INDEX IF EXISTS unique_articles_url;
CREATE UNIQUE INDEX IF NOT EXISTS unique_index_articles_feed_id_url ON articles (feed_id, url);
