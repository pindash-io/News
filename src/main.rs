#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::env;
use std::fs;
use std::ops::Deref;
use std::ops::Div;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{
    mpsc::{self, Sender},
    Arc, RwLock,
};
use std::thread;
use std::vec;

use anyhow::{Error, Result};
use eframe::egui;
use eframe::egui::style::Margin;
use eframe::egui::Button;
use eframe::egui::ImageButton;
use eframe::egui::Sense;
use eframe::epaint::ColorImage;
use eframe::epaint::Rect;
use eframe::epaint::Shape;
use egui_extras::RetainedImage;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OpenFlags};
use rusqlite_migration::{Migrations, M};

use pindash_news::*;

fn main() -> Result<()> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let home = env::var("HOME")?;
    let config_dir: PathBuf = (home + "/.config/pindash").into();

    if !config_dir.exists() {
        fs::create_dir(config_dir.clone())?;
    }

    // https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/
    let db = SqliteConnectionManager::file(config_dir.join("news.db")).with_init(|c| {
        c.execute_batch(
            r#"
                PRAGMA synchronous = NORMAL;
                PRAGMA journal_mode = WAL;
                PRAGMA foreign_keys = ON;
                "#,
        )?;
        Ok(())
    });

    let pool = r2d2::Pool::new(db)?;

    let folders = Arc::new(RwLock::new(Vec::new()));

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let folders_writer = folders.clone();
    rt.block_on(async {
        let migrations = Migrations::new(vec![
            M::up(include_str!("../migrations/0-sources.sql")),
            M::up(include_str!("../migrations/1-folders.sql")),
            M::up(include_str!("../migrations/2-default-folder.sql")),
            M::up(include_str!("../migrations/3-feeds.sql")),
            M::up(include_str!("../migrations/4-seed.sql")),
        ]);

        let mut conn = pool.get()?;
        migrations.to_latest(&mut conn)?;

        let folders = db::fetch_folders(&mut conn)?;
        tracing::info!("{:?}", folders);

        if let Ok(mut fd) = folders_writer.write() {
            fd.extend_from_slice(&folders);
        }

        Ok::<(), Error>(())
    });

    let (tx, mut rx) = tokio::sync::watch::channel::<Message>(Message::Normal);

    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(async move {
            let mut conn = pool.get()?;

            while rx.changed().await.is_ok() {
                let msg = rx.borrow();
                tracing::info!("{:?}", &msg);
                match msg.deref() {
                    Message::NewSource(url, name, folder_id) => {
                        if let Ok(id) = db::create_source(
                            &mut conn,
                            url.to_string(),
                            name.to_string(),
                            *folder_id,
                        ) {
                            if let Ok(mut folders) = folders_writer.write() {
                                if let Some(folder) = folders.iter_mut().find_map(|folder| {
                                    if folder.id == *folder_id {
                                        Some(folder)
                                    } else {
                                        None
                                    }
                                }) {
                                    if let Some(sources) = folder.sources.as_mut() {
                                        sources.push(models::Source {
                                            id,
                                            name: name.to_string(),
                                            url: url.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Message::NewFolder(name) => {
                        if let Ok(folder) = db::create_folder(&mut conn, name.to_string()) {
                            if let Ok(mut folders) = folders_writer.write() {
                                folders.push(folder);
                            }
                        }
                    }
                    Message::DeleteFolder(_, id) => {
                        if let Ok(()) = db::delete_folder(&mut conn, *id) {
                            if let Ok(mut folders) = folders_writer.write() {
                                folders.retain(|f| f.id != *id);
                            }
                        }
                    }
                    Message::RenameFolder(name, id) => {
                        if db::rename_folder(&mut conn, name.clone(), *id)
                            .ok()
                            .filter(|n| *n == 1)
                            .is_some()
                        {
                            if let Ok(mut folders) = folders_writer.write() {
                                if let Some(folder) = folders.iter_mut().find_map(|folder| {
                                    if folder.id == *id {
                                        Some(folder)
                                    } else {
                                        None
                                    }
                                }) {
                                    folder.name = name.to_string();
                                }
                            }
                        }
                    }
                    Message::DeleteSource(_, id, folder_id) => {
                        if let Ok(()) = db::delete_source(&mut conn, *id) {
                            if let Ok(mut folders) = folders_writer.write() {
                                if let Some(folder) = folders.iter_mut().find_map(|folder| {
                                    if folder.id == *folder_id {
                                        Some(folder)
                                    } else {
                                        None
                                    }
                                }) {
                                    if let Some(sources) = folder.sources.as_mut() {
                                        sources.retain(|s| s.id != *id);
                                    }
                                }
                            }
                        }
                    }
                    Message::EditSource(url, name, id, folder_id, prev_folder_id) => {
                        if let Ok(()) = db::update_source(
                            &mut conn,
                            url.to_string(),
                            name.to_string(),
                            *id,
                            prev_folder_id.is_some().then_some(*folder_id),
                        ) {
                            // need opt!
                            if let Ok(mut folders) = folders_writer.write() {
                                // remove
                                if let Some(pfid) = prev_folder_id {
                                    // remove
                                    if let Some(folder) = folders.iter_mut().find_map(|folder| {
                                        if folder.id == *pfid {
                                            Some(folder)
                                        } else {
                                            None
                                        }
                                    }) {
                                        if let Some(sources) = folder.sources.as_mut() {
                                            sources.retain(|s| s.id != *id);
                                        }
                                    }
                                    // push
                                    if let Some(folder) = folders.iter_mut().find_map(|folder| {
                                        if folder.id == *folder_id {
                                            Some(folder)
                                        } else {
                                            None
                                        }
                                    }) {
                                        if let Some(sources) = folder.sources.as_mut() {
                                            sources.push(models::Source {
                                                id: *id,
                                                name: name.to_string(),
                                                url: url.to_string(),
                                            });
                                        }
                                    }
                                } else {
                                    // update
                                    if let Some(folder) = folders.iter_mut().find_map(|folder| {
                                        if folder.id == *folder_id {
                                            Some(folder)
                                        } else {
                                            None
                                        }
                                    }) {
                                        if let Some(sources) = folder.sources.as_mut() {
                                            if let Some(source) =
                                                sources.iter_mut().find_map(|source| {
                                                    if source.id == *id {
                                                        Some(source)
                                                    } else {
                                                        None
                                                    }
                                                })
                                            {
                                                source.name = name.to_string();
                                                source.url = url.to_string();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Message::FetchFeedsBySource(url, id) => {
                        let url = url.to_string();
                        tokio::task::spawn(async move {
                            let content = reqwest::get(url).await?.text().await?;
                            let feed = syndication::Feed::from_str(&content)
                                .map_err(|e| anyhow::anyhow!(e))?;

                            // match feed {
                            //     syndication::Feed()
                            // }

                            Ok::<(), Error>(())
                        });
                    }
                    _ => {}
                }
            }

            Ok::<(), Error>(())
        });
        Ok::<(), Error>(())
    });

    rt.block_on(async {
        let store = Store::new(tx, folders);
        let options = eframe::NativeOptions {
            drag_and_drop_support: true,
            fullsize_content: true,

            initial_window_size: Some(egui::vec2(1280.0, 1024.0)),

            #[cfg(feature = "wgpu")]
            renderer: eframe::Renderer::Wgpu,

            ..Default::default()
        };
        eframe::run_native(
            "PinDash News",
            options,
            Box::new(|_cc| Box::new(ui::App::new(store))),
        );
    });
    Ok(())
}
