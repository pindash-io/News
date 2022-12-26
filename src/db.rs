use anyhow::Result;

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OptionalExtension;

use crate::models::{Article, Feed, Folder};

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
                (CASE count(f.d)
                WHEN 0 THEN NULL
                ELSE json_group_array(json_object(
                    'id',
                    f.id,
                    'name',
                    f.name,
                    'url',
                    f.url,
                    'last_seen',
                    f.last_seen,
                    'folder_id',
                    d.id
                ))
            END) AS feeds
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

pub fn create_articles(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Feed { id, .. }: &Feed,
    updated: &str,
    // feeds: &[Entry],
) -> Result<()> {
    let t = conn.transaction()?;

    {
        let mut stmt = t.prepare_cached(
            r#"
            INSERT INTO articles (
                feed_id,
                title,
                content,
                author
            )
            VALUES (
                ?1,
                ?2,
                ?3,
                ?4
            )
            "#,
        )?;
    }

    t.commit()?;
    Ok(())
}
