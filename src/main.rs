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
    time::Duration,
};

use anyhow::{Error, Result};
use eframe::egui;
use image::EncodableLayout;
use once_cell::sync::Lazy;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OpenFlags};
use rusqlite_migration::{Migrations, M};

use pindash_news::*;

const APP_NAME: &str = "PinDash News";

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    use reqwest::header;
    let mut headers = header::HeaderMap::new();
    // headers.insert(
    //     "Cache-Control",
    //     header::HeaderValue::from_static("max-age=0"),
    // );
    // headers.insert("Connection", header::HeaderValue::from_static("keep-alive"));
    reqwest::Client::builder()
        // Cache-Control: max-age=0
        // Connection: keep-alive
        // .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
        .default_headers(headers)
        // .use_native_tls()
        .trust_dns(true)
        .gzip(true)
        .deflate(true)
        .brotli(true)
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Cant build a reqwest client")
});

fn main() -> Result<()> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let home = env::var("HOME")?;
    let config_dir: PathBuf = (home + "/.config/pindash").into();

    if !config_dir.exists() {
        fs::create_dir(config_dir.clone())?;
    }

    // https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/
    // https://developer.apple.com/documentation/xcode/reducing-disk-writes
    // https://www.theunterminatedstring.com/sqlite-vacuuming/
    let db = SqliteConnectionManager::file(config_dir.join("news.db")).with_init(|c| {
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
            M::up(include_str!(
                "../migrations/12-feeds-rename-published-to-updated.sql"
            )),
            M::up(include_str!(
                "../migrations/13-feeds-add-unique-index-feed_id-url.sql"
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
            while rx.changed().await.is_ok() {
                let msg = rx.borrow();
                tracing::info!("{:?}", &msg);
                match msg.deref() {
                    Message::Feed(action, feed) => {
                        let Ok(mut conn) = pool.get() else {
                            continue;
                        };
                        tracing::info!("{:?} {:?}", action, feed);
                        match action {
                            Action::Create => {
                                db::create_feed(&mut conn, feed).ok().and_then(|id| {
                                    folders_writer.write().ok().and_then(|mut folders| {
                                        folders.iter_mut().find(|f| f.id == feed.folder_id).map(
                                            |f| {
                                                let mut feed = feed.to_owned();
                                                feed.id = id;
                                                f.feeds.get_or_insert_with(Vec::new).push(feed)
                                            },
                                        )
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
                                                        .find(|f| f.id == feed.folder_id)
                                                        .and_then(|f| f.feeds.as_mut())
                                                        .and_then(|feeds| {
                                                            feeds
                                                                .iter_mut()
                                                                .find(|f| f.id == feed.id)
                                                        })
                                                        .map(|f| {
                                                            *f = feed.to_owned();
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
                                                                feeds.retain(|f| f.id != feed.id)
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
                                            .find(|f| f.id == feed.folder_id)
                                            .and_then(|f| f.feeds.as_mut())
                                            .map(|feeds| feeds.retain(|f| f.id != feed.id))
                                    })
                                });
                            }
                            Action::Fetch => {
                                let mut feed = feed.to_owned();
                                // if `fetching` or `inserting`, pass
                                if feed.status {
                                    continue;
                                }
                                let folder_id = feed.folder_id;
                                let feed_id = feed.id;

                                // first fetch
                                let articles = feed
                                    .articles
                                    .is_none()
                                    .then(|| db::find_articles_by_feed(&mut conn, &feed).ok())
                                    .flatten();

                                folders_writer.write().ok().and_then(|mut folders| {
                                    folders
                                        .iter_mut()
                                        .find(|f| f.id == folder_id)
                                        .and_then(|f| f.feeds.as_mut())
                                        .and_then(|feeds| {
                                            feeds.iter_mut().find(|f| f.id == feed_id)
                                        })
                                        .map(|f| {
                                            if articles.as_ref().filter(|a| !a.is_empty()).is_some()
                                                && f.articles.is_none()
                                            {
                                                f.last_seen = articles
                                                    .as_ref()
                                                    .and_then(|a| a.last())
                                                    .map(|a| a.created)
                                                    .unwrap();
                                                f.articles = articles;
                                                feed.last_seen = f.last_seen;
                                            }
                                            f.status = true;
                                        })
                                });

                                let url = feed.url.clone();
                                let folders_writer = folders_writer.clone();
                                tokio::task::spawn(async move {
                                    let Some(data) = async move {
                                        let result = CLIENT.get(&url).send().await;
                                        let mut data = None;
                                        if let Ok(resp) = result {
                                            data = resp.bytes().await.ok();
                                        }
                                        data
                                    }
                                    .await else {
                                        tracing::info!("Feeds fetch failed");
                                        folders_writer.write().ok().map(|mut folders| {
                                            folders
                                                .iter_mut()
                                                .find(|f| f.id == folder_id)
                                                .and_then(|f| f.feeds.as_mut())
                                                .and_then(|feeds| {
                                                    feeds.iter_mut().find(|f| f.id == feed_id)
                                                })
                                                .map(|f| f.status = false)
                                        });
                                        return Ok::<(), Error>(())
                                    };

                                    let feed_rs::model::Feed {
                                        id,
                                        feed_type,
                                        title,
                                        description,
                                        mut entries,
                                        published,
                                        updated,
                                        authors,
                                        links,
                                        ..
                                        // logo,
                                        // icon,
                                        // categories,
                                        // contributors,
                                        // published,
                                        // ttl,
                                        // language,
                                        // rating,
                                        // rights,
                                        // generator,
                                    } = feed_rs::parser::parse(data.as_ref())?;

                                    let published = entries
                                        .first()
                                        .and_then(|e| e.published.or(e.updated))
                                        .or(published)
                                        .or(updated)
                                        .map(|t| t.timestamp_millis())
                                        .unwrap_or(feed.last_seen);

                                    let flag = published > feed.last_seen;

                                    tracing::info!(
                                        "{}: has new entries {}, last_seen = {}, published = {}",
                                        feed.name,
                                        flag,
                                        feed.last_seen,
                                        published
                                    );

                                    if !flag {
                                        folders_writer.write().ok().map(|mut folders| {
                                            folders
                                                .iter_mut()
                                                .find(|f| f.id == folder_id)
                                                .and_then(|f| f.feeds.as_mut())
                                                .and_then(|feeds| {
                                                    feeds.iter_mut().find(|f| f.id == feed_id)
                                                })
                                                .map(|f| f.status = false)
                                        });
                                        return Ok::<(), Error>(());
                                    }

                                    let site = links
                                        .iter()
                                        .find_map(|link| {
                                            if !link.href.ends_with(".xml")
                                                && !link.href.ends_with(".atom")
                                            {
                                                Some(link.href.to_owned())
                                            } else {
                                                None
                                            }
                                        })
                                        .or_else(|| {
                                            // Fixed, https://go.dev/blog
                                            if id.contains(',') {
                                                None
                                            } else {
                                                url::Url::parse(&id)
                                                    .map(|link| link.as_str().to_owned())
                                                    .ok()
                                            }
                                        })
                                        .unwrap_or_else(|| {
                                            let url = links
                                                .first()
                                                .map(|link| link.href.to_owned())
                                                .unwrap_or(feed.url.to_owned());

                                            url.trim_end_matches("rss.xml")
                                                .trim_end_matches("atom.xml")
                                                .trim_end_matches("index.xml")
                                                .trim_end_matches("feed.xml")
                                                .trim_end_matches("feed.atom")
                                                .to_string()
                                        });

                                    let published = db::update_feed_ext_and_upsert_articles(
                                        &mut conn,
                                        &feed,
                                        &site,
                                        feed_type,
                                        title.map(|t| t.content),
                                        description.map(|t| t.content),
                                        published,
                                        authors,
                                        {
                                            // insert, order by asc
                                            entries.reverse();
                                            entries
                                        },
                                    )?;

                                    let articles = db::find_articles_by_feed(&mut conn, &feed).ok();

                                    folders_writer.write().ok().map(|mut folders| {
                                        folders
                                            .iter_mut()
                                            .find(|f| f.id == folder_id)
                                            .and_then(|f| f.feeds.as_mut())
                                            .and_then(|feeds| {
                                                feeds.iter_mut().find(|f| f.id == feed_id)
                                            })
                                            .map(|f| {
                                                if let Some(a) = f.articles.as_mut() {
                                                    a.extend_from_slice(
                                                        &articles.unwrap_or_default(),
                                                    );
                                                } else {
                                                    f.articles = articles;
                                                }
                                                f.last_seen = published;
                                                f.status = false;
                                            })
                                    });

                                    tracing::info!("{site}: fetched feeds {published}");

                                    Ok::<(), Error>(())
                                });
                            }
                            _ => {}
                        }
                    }
                    Message::Folder(action, folder) => {
                        let Ok(mut conn) = pool.get() else {
                            continue;
                        };
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
            Box::new(|cc| Box::new(ui::App::new(&cc, store))),
        );
    });

    tracing::info!("app exit!");

    drop(rt);
    Ok(())
}
