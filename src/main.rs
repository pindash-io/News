#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{
    env, fs,
    ops::Deref,
    path::PathBuf,
    str::FromStr,
    sync::{
        mpsc::{self, Sender},
        Arc, RwLock,
    },
    thread,
};

use anyhow::{Error, Result};
use eframe::egui;
use image::EncodableLayout;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OpenFlags};
use rusqlite_migration::{Migrations, M};

use pindash_news::*;

const APP_NAME: &str = "PinDash News";

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
                PRAGMA busy_timeout = 5000;
            "#,
        )?;
        Ok(())
    });

    let pool = r2d2::Pool::new(db)?;

    let folders = Arc::new(RwLock::new(Vec::new()));

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .thread_name(APP_NAME)
        .build()?;

    let folders_writer = folders.clone();
    rt.block_on(async {
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
        ]);

        let mut conn = pool.get()?;
        migrations.to_latest(&mut conn)?;

        let folders = db::fetch_folders(&mut conn)?;
        tracing::info!("{:?}", folders);

        if let Ok(mut fd) = folders_writer.write() {
            fd.extend_from_slice(&folders);
        }

        drop(conn);
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
                    Message::Feed(action, feed) => {
                        tracing::info!("{:?} {:?}", action, feed);
                        match action {
                            Action::Create => {
                                db::create_feed(&mut conn, feed).ok().and_then(|id| {
                                    folders_writer.write().ok().and_then(|mut folders| {
                                        folders
                                            .iter_mut()
                                            .find_map(|f| {
                                                if f.id == feed.folder_id {
                                                    Some(f)
                                                } else {
                                                    None
                                                }
                                            })
                                            .map(|f| {
                                                let mut feed = feed.to_owned();
                                                feed.id = id;
                                                f.feeds.get_or_insert_with(Vec::new).push(feed)
                                            })
                                    })
                                });
                            }
                            Action::Update => {
                                db::update_feed(&mut conn, feed).ok().and_then(
                                    |(prev_folder_id, changed)| {
                                        folders_writer.write().ok().map(|mut folders| {
                                            // dont change folder
                                            if prev_folder_id == 0 {
                                                // update
                                                if changed > 0 {
                                                    folders
                                                        .iter_mut()
                                                        .find_map(|f| {
                                                            if f.id == feed.folder_id {
                                                                Some(f)
                                                            } else {
                                                                None
                                                            }
                                                        })
                                                        .and_then(|f| f.feeds.as_mut())
                                                        .and_then(|feeds| {
                                                            feeds.iter_mut().find_map(|s| {
                                                                if s.id == feed.id {
                                                                    Some(s)
                                                                } else {
                                                                    None
                                                                }
                                                            })
                                                        })
                                                        .map(|s| {
                                                            *s = feed.to_owned();
                                                        });
                                                }
                                            } else {
                                                folders
                                                    .iter_mut()
                                                    .filter(|f| {
                                                        f.id == prev_folder_id
                                                            || f.id == feed.folder_id
                                                    })
                                                    .for_each(|f| {
                                                        if f.id == prev_folder_id {
                                                            // delete from prev folder
                                                            f.feeds.as_mut().map(|feeds| {
                                                                feeds.retain(|s| s.id != feed.id)
                                                            });
                                                        } else {
                                                            // push to new folder
                                                            f.feeds
                                                                .get_or_insert_with(Vec::new)
                                                                .push(feed.to_owned())
                                                        }
                                                    })
                                            }
                                        })
                                    },
                                );
                            }
                            Action::Delete => {
                                db::delete_feed(&mut conn, feed).ok().and_then(|_| {
                                    folders_writer.write().ok().and_then(|mut folders| {
                                        folders
                                            .iter_mut()
                                            .find_map(|f| {
                                                if f.id == feed.folder_id {
                                                    Some(f)
                                                } else {
                                                    None
                                                }
                                            })
                                            .and_then(|f| f.feeds.as_mut())
                                            .map(|feeds| {
                                                feeds.retain(|s| s.id != feed.id);
                                            })
                                    })
                                });
                            }
                            Action::Fetch => {
                                let url = feed.url.to_string();
                                tokio::task::spawn(async move {
                                    let content = reqwest::get(url).await?.bytes().await?;
                                    let feed_rs::model::Feed {
                                        feed_type,
                                        id,
                                        title,
                                        description,
                                        updated,
                                        logo,
                                        icon,
                                        authors,
                                        entries,
                                        links,
                                        categories,
                                        contributors,
                                        published,
                                        ttl,
                                        generator,
                                        language,
                                        rating,
                                        rights,
                                    } = feed_rs::parser::parse(content.as_bytes())?;

                                    tracing::info!("{:?}", feed_type);
                                    tracing::info!("{:?}", id);
                                    tracing::info!("{:?}", title);
                                    tracing::info!("{:?}", description);
                                    tracing::info!("{:?}", updated);
                                    tracing::info!("{:?}", logo);
                                    tracing::info!("{:?}", icon);
                                    tracing::info!("{:?}", entries.len());

                                    tracing::info!("{:?}", links);
                                    tracing::info!("{:?}", categories);
                                    tracing::info!("{:?}", contributors);
                                    tracing::info!("{:?}", published);
                                    tracing::info!("{:?}", ttl);
                                    tracing::info!("{:?}", generator);
                                    tracing::info!("{:?}", language);
                                    tracing::info!("{:?}", rating);
                                    tracing::info!("{:?}", rights);
                                    Ok::<(), Error>(())
                                });
                            }
                            _ => {}
                        }
                    }
                    Message::Folder(action, folder) => {
                        tracing::info!("{:?} {:?}", action, folder);
                        match action {
                            Action::Create => {
                                db::create_folder(&mut conn, folder).ok().and_then(|id| {
                                    folders_writer.write().ok().map(|mut folders| {
                                        let mut folder = folder.to_owned();
                                        folder.id = id;
                                        folders.push(folder)
                                    })
                                });
                            }
                            Action::Update => {
                                db::rename_folder(&mut conn, folder)
                                    .ok()
                                    .filter(|n| *n == 1)
                                    .and_then(|_| {
                                        folders_writer.write().ok().map(|mut folders| {
                                            folders
                                                .iter_mut()
                                                .find_map(|f| {
                                                    if f.id == folder.id {
                                                        Some(f)
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .map(|f| {
                                                    f.name = folder.name.to_owned();
                                                })
                                        })
                                    });
                            }
                            Action::Delete => {
                                // mv other folder's feeds to folder 1
                                db::delete_folder(&mut conn, folder).ok().and_then(|_| {
                                    folders_writer.write().ok().map(|mut folders| {
                                        let mut tmp = folders
                                            .iter()
                                            .enumerate()
                                            .filter_map(|(i, f)| {
                                                if f.id == 1 || f.id == folder.id {
                                                    Some((i, f.id))
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect::<Vec<_>>();

                                        tmp.sort_by_key(|&(_, id)| id);

                                        folders.remove(tmp[1].0).feeds.map(move |feeds| {
                                            folders[tmp[0].0]
                                                .feeds
                                                .get_or_insert_with(Vec::new)
                                                .extend_from_slice(&feeds)
                                        });
                                    })
                                });
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            Ok::<(), Error>(())
        });

        drop(rt);
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
            APP_NAME,
            options,
            Box::new(|_cc| Box::new(ui::App::new(store))),
        );
    });

    tracing::info!("app exit!");

    drop(rt);
    Ok(())
}
