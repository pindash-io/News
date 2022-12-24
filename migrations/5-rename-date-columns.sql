ALTER TABLE sources RENAME COLUMN created_at TO created;
ALTER TABLE sources RENAME COLUMN updated_at TO updated;
ALTER TABLE sources RENAME COLUMN last_seen_at TO last_seen;

ALTER TABLE folders RENAME COLUMN created_at TO created;
ALTER TABLE folders RENAME COLUMN updated_at TO updated;

ALTER TABLE feeds RENAME COLUMN created_at TO created;
