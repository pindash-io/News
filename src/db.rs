use anyhow::Result;

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OptionalExtension;

use crate::models::{Folder, Source};

pub fn fetch_folders(conn: &mut PooledConnection<SqliteConnectionManager>) -> Result<Vec<Folder>> {
    let folders = conn
        .prepare_cached(
            r#"
            WITH f AS (
                SELECT
                    id,
                    name
                FROM
                    folders
            ),
            fs AS (
                SELECT
                    f,
                    s
                FROM
                    folder_sources
            ),
            s AS (
                SELECT
                    s.id,
                    s.name,
                    s.url,
                    s.last_seen,
                    fs.f AS fid
                FROM
                    sources AS s
                INNER JOIN
                    fs
                ON
                    fs.s = s.id
            )
            SELECT
                f.id,
                f.name,
                (CASE count(s.fid)
                WHEN 0 THEN NULL
                ELSE json_group_array(json_object(
                    'id',
                    s.id,
                    'name',
                    s.name,
                    'url',
                    s.url,
                    'last_seen',
                    s.last_seen,
                    'folder_id',
                    f.id
                ))
            END) AS folders 
            FROM
                f
            LEFT JOIN
                s
            ON
                s.fid = f.id 
            GROUP BY
                f.id
            "#,
        )?
        .query_map([], |row| {
            let id = row.get(0)?;
            let name = row.get(1)?;
            let sources = row
                .get::<_, Option<serde_json::Value>>(2)?
                .and_then(|v| serde_json::from_value(v).ok());
            Ok(Folder { id, name, sources })
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
            folder_sources
        SET
            f = 1
        WHERE
            f = ?1
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

pub fn create_source(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Source {
        url,
        name,
        folder_id,
        ..
    }: &Source,
) -> Result<u64> {
    let t = conn.transaction()?;
    let id = t.query_row(
        r#"
        INSERT INTO sources (
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
        INSERT INTO folder_sources (
            s,
            f
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

pub fn delete_source(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Source { id, .. }: &Source,
) -> Result<usize> {
    let t = conn.transaction()?;
    let changed = t.execute(
        r#"
        DELETE FROM
            sources
        WHERE
            id = ?1
        "#,
        [id],
    )?;
    t.commit()?;
    Ok(changed)
}

pub fn update_source(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    Source {
        id,
        name,
        url,
        folder_id,
        ..
    }: &Source,
) -> Result<(u64, usize)> {
    let t = conn.transaction()?;
    let prev_folder_id = t
        .query_row(
            r#"
            SELECT
                f
            FROM
                folder_sources
            WHERE
                s = ?1
            AND
                f <> ?2
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
                folder_sources
            SET
                f = ?2
            WHERE
                s = ?1
            "#,
            [id, folder_id],
        )?;
    }
    let changed = t.execute(
        r#"
        UPDATE
            sources
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

pub fn create_feed(conn: &mut PooledConnection<SqliteConnectionManager>) -> Result<()> {
    Ok(())
}
