use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OptionalExtension;
use rusqlite_migration::{Migrations, M};

use crate::models::{Article, Entry, Feed, FeedType, Folder, Person};

// https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/
// https://developer.apple.com/documentation/xcode/reducing-disk-writes
// https://www.theunterminatedstring.com/sqlite-vacuuming/
pub fn init(
    dir: PathBuf,
    folders: Arc<RwLock<Vec<Folder>>>,
) -> Result<Pool<SqliteConnectionManager>> {
    let pool = Pool::new(
        SqliteConnectionManager::file(dir.join("news.db")).with_init(|c| {
            c.execute_batch(
                r#"
                PRAGMA auto_vacuum = INCREMENTAL;
                PRAGMA synchronous = NORMAL;
                PRAGMA journal_mode = WAL;
                PRAGMA foreign_keys = ON;
                PRAGMA busy_timeout = 5000;
            "#,
            )?;
            Ok(())
        }),
    )?;

    migrate(&mut pool.get()?, folders)?;

    Ok(pool)
}

fn migrate(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    folders: Arc<RwLock<Vec<Folder>>>,
) -> Result<()> {
    let migrations = Migrations::new(vec![
        M::up(include_str!("../migrations/0-sources.sql")),
        M::up(include_str!("../migrations/1-folders.sql")),
        M::up(include_str!("../migrations/2-default-folder.sql")),
        M::up(include_str!("../migrations/3-feeds.sql")),
        M::up(include_str!("../migrations/4-seed.sql")),
        M::up(include_str!("../migrations/5-rename-date-columns.sql")),
        M::up(include_str!("../migrations/6-feeds-add-url-published.sql")),
        M::up(include_str!("../migrations/7-authors.sql")),
        M::up(include_str!("../migrations/8-rename-tables.sql")),
        M::up(include_str!(
            "../migrations/9-authors-rename-article_id-to-feed_id.sql"
        )),
        M::up(include_str!(
            "../migrations/10-articles-rename-source_id-to-feed_id.sql"
        )),
        M::up(include_str!(
            "../migrations/11-feeds-add-site-type-title.sql"
        )),
        M::up(include_str!(
            "../migrations/12-feeds-rename-published-to-updated.sql"
        )),
        M::up(include_str!(
            "../migrations/13-feeds-add-unique-index-feed_id-url.sql"
        )),
    ]);

    migrations.to_latest(conn)?;

    let results = fetch_folders(conn)?;

    if let Ok(mut fd) = folders.write() {
        fd.extend_from_slice(&results);
    }

    Ok(())
}

pub fn fetch_folders(conn: &mut PooledConnection<SqliteConnectionManager>) -> Result<Vec<Folder>> {
    let folders = conn
        .prepare_cached(
            r#"
            WITH d AS (
                SELECT
                    id,
                    name
                FROM
                    folders
            ),
            df AS (
                SELECT
                    d,
                    f
                FROM
                    folder_feeds
            ),
            f AS (
                SELECT
                    f.id,
                    f.name,
                    f.url,
                    f.site,
                    f.last_seen,
                    df.d
                FROM
                    feeds AS f
                INNER JOIN
                    df
                ON
                    df.f = f.id
            )
            SELECT
                d.id,
                d.name,
                (
                    CASE count(f.d)
                    WHEN 0 THEN NULL
                    ELSE json_group_array(
                        json_object(
                            'id',
                            f.id,
                            'name',
                            f.name,
                            'url',
                            f.url,
                            'site',
                            f.site,
                            'last_seen',
                            f.last_seen,
                            'folder_id',
                            d.id
                        )
                    )
                    END
                ) AS feeds
            FROM
                d
            LEFT JOIN
                f
            ON
                f.d = d.id 
            GROUP BY
                d.id
            "#,
        )?
        .query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                feeds: row
                    .get::<_, Option<serde_json::Value>>(2)?
                    .and_then(|v| serde_json::from_value(v).ok()),
            })
        })
        .map(|rows| rows.filter_map(Result::ok).collect::<Vec<_>>())?;
    Ok(folders)
}

pub fn create_folder(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Folder { name, .. }: &Folder,
) -> Result<u64> {
    let id = conn.query_row(
        r#"
        INSERT INTO folders (
            name
        )
        VALUES (
            ?1
        )
        RETURNING
            id
        "#,
        [name],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn rename_folder(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Folder { id, name, .. }: &Folder,
) -> Result<usize> {
    let changed = conn.execute(
        r#"
        UPDATE
            folders
        SET
            name = ?1
        WHERE
            id = ?2
        "#,
        rusqlite::params![name, id],
    )?;
    Ok(changed)
}

pub fn delete_folder(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Folder { id, .. }: &Folder,
) -> Result<()> {
    let t = conn.transaction()?;
    t.execute(
        r#"
        UPDATE
            folder_feeds
        SET
            d = 1
        WHERE
            d = ?1
        "#,
        [id],
    )?;
    t.execute(
        r#"
        DELETE FROM
            folders
        WHERE
            id = ?1
        "#,
        [id],
    )?;
    t.commit()?;
    Ok(())
}

pub fn create_feed(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Feed {
        url,
        name,
        folder_id,
        ..
    }: &Feed,
) -> Result<u64> {
    let t = conn.transaction()?;
    let id = t.query_row(
        r#"
        INSERT INTO feeds (
            url,
            name
        )
        VALUES (
            ?1,
            ?2
        )
        RETURNING
            id
        "#,
        [url, name],
        |row| row.get(0),
    )?;
    t.execute(
        r#"
        INSERT INTO folder_feeds (
            f,
            d
        )
        VALUES (
            ?1,
            ?2
        )
        "#,
        [id, *folder_id],
    )?;
    t.commit()?;
    Ok(id)
}

pub fn delete_feed(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Feed { id, .. }: &Feed,
) -> Result<usize> {
    let t = conn.transaction()?;
    let changed = t.execute(
        r#"
        DELETE FROM
            feeds
        WHERE
            id = ?1
        "#,
        [id],
    )?;
    t.commit()?;
    Ok(changed)
}

pub fn update_feed(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Feed {
        id,
        name,
        url,
        folder_id,
        ..
    }: &Feed,
) -> Result<(u64, usize)> {
    let t = conn.transaction()?;
    let prev_folder_id = t
        .query_row(
            r#"
            SELECT
                d
            FROM
                folder_feeds
            WHERE
                f = ?1
            AND
                d <> ?2
            LIMIT 1
            "#,
            [id, folder_id],
            |row| row.get(0),
        )
        .optional()
        .map(|v| v.unwrap_or(0))?;
    if prev_folder_id != 0 {
        t.execute(
            r#"
            UPDATE
                folder_feeds
            SET
                d = ?2
            WHERE
                f = ?1
            "#,
            [id, folder_id],
        )?;
    }
    let changed = t.execute(
        r#"
        UPDATE
            feeds
        SET
            url = ?1,
            name = ?2
        WHERE
            id = ?3
        "#,
        rusqlite::params![url, name, id],
    )?;
    t.commit()?;
    Ok((prev_folder_id, changed))
}

pub fn update_feed_ext_and_upsert_articles(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Feed { id, .. }: &Feed,
    site: &String,
    kind: FeedType,
    title: Option<String>,
    description: Option<String>,
    published: i64,
    authors: Vec<Person>,
    articles: Vec<Entry>,
) -> Result<i64> {
    update_feed_ext(conn, id, site, kind, title, description)?;

    upsert_articles(conn, id, site, published, authors, articles)
}

fn update_feed_ext(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    id: &u64,
    site: &String,
    kind: FeedType,
    title: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let t = conn.transaction()?;

    {
        let mut stmt = t.prepare_cached(
            r#"
            UPDATE
                feeds 
            SET
                site = ?1,
                type = ?2,
                title = ?3,
                description = ?4
            WHERE
                id = ?5
            "#,
        )?;
        stmt.execute(rusqlite::params![
            site,
            {
                use FeedType::*;
                match kind {
                    Atom => "Atom",
                    JSON => "JSON",
                    RSS0 => "RSS0",
                    RSS1 => "RSS1",
                    RSS2 => "RSS2",
                }
            },
            title,
            description,
            id
        ])?;
    }

    t.commit()?;
    Ok(())
}

fn upsert_articles(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    id: &u64,
    site: &String,
    feed_published: i64,
    authors: Vec<Person>,
    articles: Vec<Entry>,
) -> Result<i64> {
    let t = conn.transaction()?;

    {
        let mut stmt = t.prepare_cached(
            r#"
            INSERT INTO articles (
                feed_id,
                url,
                title,
                content,
                created,
                updated 
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4,
                ?5,
                ?6
            )
            ON CONFLICT(feed_id, url) DO 
            UPDATE
            SET
                title = EXCLUDED.title,
                content = EXCLUDED.content,
                created = ifnull(EXCLUDED.created, articles.created),
                updated = ifnull(EXCLUDED.updated, ifnull(articles.updated, articles.created))
            RETURNING
                id
            "#,
        )?;

        let mut sa = t.prepare_cached(
            r#"
            INSERT INTO authors (
                feed_id,
                name,
                email,
                url
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4
            )
            ON CONFLICT(feed_id, name) DO 
            UPDATE
            SET
                name = EXCLUDED.name,
                email = EXCLUDED.email,
                url = EXCLUDED.url
            RETURNING
                id
            "#,
        )?;

        let mut saa = t.prepare_cached(
            r#"
            INSERT INTO article_authors (
                t,
                a
            )
            VALUES (
                ?1,
                ?2
            )
            ON CONFLICT(t, a) DO 
            NOTHING
            "#,
        )?;

        for article in articles {
            let updated = article.updated.map(|t| t.timestamp_millis());
            let published = article
                .published
                .map(|t| t.timestamp_millis())
                .or_else(|| updated)
                .unwrap_or(feed_published);

            let updated = if let Some(updated) = updated {
                updated
            } else {
                published
            };

            let article_id: u64 = stmt.query_row(
                rusqlite::params![
                    id,
                    article
                        .links
                        .first()
                        .map(|link| link.href.to_owned())
                        // sometimes `article.id` is not a link
                        .or(Some(article.id))
                        .map(|path| if path.starts_with(site) {
                            path
                        } else {
                            let mut url = String::new();
                            url.push_str(site.trim_end_matches('/'));
                            url.push('/');
                            url.push_str(path.trim_start_matches('/'));
                            url
                        })
                        .unwrap(),
                    article.title.map(|t| t.content.trim().to_owned()),
                    article
                        .content
                        .and_then(|t| t.body)
                        .or_else(|| article.summary.map(|t| t.content)),
                    published,
                    updated,
                ],
                |row| row.get(0),
            )?;

            if article.authors.is_empty() {
                if let Some(author) = authors.first() {
                    let author_id: u64 = sa.query_row(
                        rusqlite::params![id, author.name, author.email, author.uri],
                        |row| row.get(0),
                    )?;
                    saa.execute([article_id, author_id])?;
                }
            } else {
                for author in article.authors {
                    let author_id: u64 = sa.query_row(
                        rusqlite::params![id, author.name, author.email, author.uri],
                        |row| row.get(0),
                    )?;
                    saa.execute([article_id, author_id])?;
                }
            }
        }
    }

    {
        let mut stmt = t.prepare_cached(
            r#"
            UPDATE
                feeds
            SET
                last_seen = ?1
            WHERE
                id = ?2
            "#,
        )?;

        stmt.execute(rusqlite::params![feed_published, id])?;
    }

    t.commit()?;
    Ok(feed_published)
}

pub fn find_articles_by_feed(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    feed: &Feed,
) -> Result<Vec<Article>> {
    let articles = conn
        .prepare_cached(
            r#"
            SELECT
                id,
                url,
                title,
                ifnull(content, ''),
                created,
                updated,
                feed_id,
                (
                    SELECT 
                        (CASE count(a.id)
                        WHEN 0 THEN NULL
                        ELSE json_group_array(
                            json_object(
                                'id',
                                a.id,
                                'name',
                                a.name
                            )
                        )
                        END)
                    FROM
                        authors as a
                    JOIN
                        article_authors AS aa
                    ON
                        aa.a = a.id
                    AND
                        aa.t = t.id
                    GROUP BY
                        aa.t
                    ORDER BY
                        aa.a
                ) AS authors
            FROM
                articles AS t
            WHERE
                feed_id = ?1
            AND
                updated > ?2
            ORDER BY
                id
            "#,
        )?
        .query_map(
            rusqlite::params![
                feed.id,
                feed.articles
                    .is_none()
                    .then_some(0)
                    .unwrap_or(feed.last_seen)
            ],
            |row| {
                Ok(Article {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    created: row.get(4)?,
                    updated: row.get(5)?,
                    feed_id: row.get(6)?,
                    authors: row
                        .get::<_, Option<serde_json::Value>>(7)?
                        .and_then(|v| serde_json::from_value(v).ok()),
                })
            },
        )
        .map(|rows| rows.filter_map(Result::ok).collect::<Vec<_>>())?;

    Ok(articles)
}
