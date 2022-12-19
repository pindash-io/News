use anyhow::{Context, Error, Result};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::models::Folder;

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
                s.url
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

pub fn create_source(
    mut conn: &mut PooledConnection<SqliteConnectionManager>,
    url: String,
    name: String,
    folder_id: u64,
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
        [id, folder_id],
    )?;
    t.commit()?;
    Ok(id)
}

pub fn create_folder(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    name: String,
) -> Result<Folder> {
    let folder = conn.query_row(
        r#"
        INSERT INTO folders (
            name
        )
        VALUES (
            ?1
        )
        RETURNING
            id,
            name 
        "#,
        [name],
        |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                sources: None,
            })
        },
    )?;
    Ok(folder)
}

pub fn delete_folder(
    mut conn: &mut PooledConnection<SqliteConnectionManager>,
    id: u64,
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

pub fn rename_folder(
    conn: &mut PooledConnection<SqliteConnectionManager>,
    name: String,
    id: u64,
) -> Result<usize> {
    let size = conn.execute(
        r#"
        UPDATE
            folders
        SET
            name = ?1
        WHERE
            id = ?2
        "#,
        rusqlite::params![name.clone(), id],
    )?;
    Ok(size)
}
