use anyhow::Result;

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::models::Folder;

pub fn create_source(
    mut conn: &mut PooledConnection<SqliteConnectionManager>,
    url: String,
    name: String,
    folder_id: u64,
) -> Result<u64> {
    let t = conn.transaction()?;
    let id = dbg!(t.query_row(
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
    ))?;
    dbg!(t.execute(
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
    ))?;
    dbg!(t.commit())?;
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
