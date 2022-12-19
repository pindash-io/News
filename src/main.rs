#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::env;
use std::fs;
use std::ops::Div;
use std::path::Path;
use std::path::PathBuf;
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
use eframe::egui::Sense;
use eframe::epaint::ahash::{HashMap, HashMapExt};
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
    tracing::info!("{:?}", config_dir);
    tracing::info!("{:?}", db);

    let pool = r2d2::Pool::new(db)?;

    // 1️⃣ Define migrations
    let migrations = Migrations::new(vec![
        M::up(include_str!("../migrations/0-sources.sql")),
        M::up(include_str!("../migrations/1-folders.sql")),
        M::up(include_str!("../migrations/2-default-folder.sql")),
    ]);

    dbg!(&migrations);

    let mut conn = pool.get()?;
    // 2️⃣ Update the database schema, atomically
    dbg!(migrations.to_latest(&mut conn)?);

    let mut stmt = conn.prepare(
        r#"
        SELECT
            id,
            name
        FROM
            folders
        ORDER BY id ASC
    "#,
    )?;

    let folders = stmt
        .query_map([], |row| {
            Ok(models::Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                sources: None,
            })
        })
        .map(|rows| rows.filter_map(Result::ok).collect::<Vec<_>>())?;

    tracing::info!("{:?}", folders);
    // drop(conn);

    let folders = Arc::new(RwLock::new(folders));
    let (tx, rx) = mpsc::channel::<Messge>();

    let folders_writer = folders.clone();
    thread::spawn(move || {
        let mut conn = pool.get()?;
        loop {
            for m in rx.iter() {
                tracing::info!("{:?}", m);
                match m {
                    Messge::NewSource(url, name, folder_id) => {
                        if let Ok(id) = db::create_source(&mut conn, url, name, folder_id) {
                            tracing::info!("{}", id);
                        }
                    }
                    Messge::NewFolder(name) => {
                        if let Ok(folder) = db::create_folder(&mut conn, name) {
                            if let Ok(mut folders) = folders_writer.write() {
                                folders.push(folder);
                            }
                        }
                    }
                    Messge::DeleteFolder(_, id) => {
                        if let Ok(()) = db::delete_folder(&mut conn, id) {
                            if let Ok(mut folders) = folders_writer.write() {
                                folders.retain(|f| f.id != id);
                            }
                        }
                    }
                    Messge::RenameFolder(name, id) => {
                        if db::rename_folder(&mut conn, name.clone(), id)
                            .ok()
                            .filter(|n| *n == 1)
                            .is_some()
                        {
                            if let Ok(mut folders) = folders_writer.write() {
                                if let Some(folder) = folders.iter_mut().find_map(|folder| {
                                    if folder.id == id {
                                        Some(folder)
                                    } else {
                                        None
                                    }
                                }) {
                                    folder.name = name;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok::<(), Error>(())
    });

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
        Box::new(|_cc| Box::new(App::new(store))),
    );

    Ok(())
}

struct App {
    icons: HashMap<&'static str, RetainedImage>,

    windows: Vec<Box<dyn windows::Window>>,
    open: HashMap<&'static str, Option<Messge>>,

    store: Store,
}

impl App {
    pub fn new(store: Store) -> Self {
        let mut icons = HashMap::<&'static str, RetainedImage>::new();

        icons.insert(
            "rss",
            RetainedImage::from_svg_bytes_with_size(
                "rss",
                include_bytes!("../icons/rss.svg"),
                egui_extras::image::FitTo::Size(24, 24),
            )
            .unwrap(),
        );
        icons.insert(
            "folder",
            RetainedImage::from_svg_bytes_with_size(
                "folder",
                include_bytes!("../icons/folder.svg"),
                egui_extras::image::FitTo::Size(24, 24),
            )
            .unwrap(),
        );
        icons.insert(
            "plus",
            RetainedImage::from_svg_bytes_with_size(
                "plus",
                include_bytes!("../icons/plus.svg"),
                egui_extras::image::FitTo::Original,
            )
            .unwrap(),
        );

        let windows: Vec<Box<dyn windows::Window>> = vec![
            Box::new(windows::WindowAddFeed::default()),
            Box::new(windows::WindowAddFolder::default()),
            Box::new(windows::WindowDeleteFolder::default()),
            Box::new(windows::WindowRenameFolder::default()),
        ];
        let open = HashMap::default();

        Self {
            store,
            icons,
            windows,
            open,
        }
    }

    pub fn windows(&mut self, ctx: &egui::Context, size: egui::Vec2) {
        let Self { windows, open, .. } = self;
        for window in windows {
            let mut is_open = open.contains_key(window.name());
            if is_open {
                let data = open.get(window.name()).cloned().unwrap();
                window.show(&self.store, ctx, &mut is_open, size, data);
            }
            if window.is_closed() {
                is_open = false;
            }
            set_open(open, window.name(), is_open, None);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.windows(ctx, frame.info().window_info.size);

        egui::TopBottomPanel::top("Navbar")
            // .exact_height(38.)
            .exact_height(28.)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(70.);
                    let img = self.icons.get("plus").unwrap();
                    ui.menu_button_image(img.texture_id(ctx), img.size_vec2() * 0.5, "Add", |ui| {
                        let img = self.icons.get("rss").unwrap();
                        if ui
                            .add(Button::image_and_text(
                                img.texture_id(ctx),
                                img.size_vec2() * 0.5,
                                "Feed",
                            ))
                            .clicked()
                        {
                            ui.close_menu();
                            set_open(
                                &mut self.open,
                                windows::WindowAddFeed::NAME,
                                true,
                                Some(Messge::RefreshFolders),
                            );
                        }
                        let img = self.icons.get("folder").unwrap();
                        if ui
                            .add(Button::image_and_text(
                                img.texture_id(ctx),
                                img.size_vec2() * 0.5,
                                "Folder",
                            ))
                            .clicked()
                        {
                            ui.close_menu();
                            set_open(&mut self.open, windows::WindowAddFolder::NAME, true, None);
                        }
                    });
                });
            });

        egui::SidePanel::left("SideBar")
            .resizable(true)
            .default_width(220.)
            .width_range(64.0..=220.)
            .show_animated(ctx, true, |ui| {
                ui.horizontal(|ui| {
                    let img = self.icons.get("rss").unwrap();
                    ui.image(img.texture_id(ctx), img.size_vec2());
                    ui.heading("Feeds");
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show_viewport(ui, |ui, rect| {
                        ui.set_width(rect.width());
                        ui.set_height(rect.height());
                        let open = &mut self.open;
                        let Store { folders, sender } = &self.store;
                        if let Ok(folders) = folders.try_read() {
                            folders.iter().for_each(move |folder| {
                                // ui.collapsing(folder.name.to_string(), |ui| {
                                //     ui.group(|ui| {});
                                // });

                                let id = ui.make_persistent_id(folder.name.to_string());
                                egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    id,
                                    false,
                                )
                                .show_header(ui, |ui| {
                                    ui.label(folder.name.to_string()).context_menu(|ui| {
                                        ui.button("Mark as read");
                                        ui.separator();
                                        if ui.button("Rename").clicked() {
                                            ui.close_menu();
                                            set_open(
                                                open,
                                                windows::WindowRenameFolder::NAME,
                                                true,
                                                Some(Messge::RenameFolder(
                                                    folder.name.to_string(),
                                                    folder.id,
                                                )),
                                            );
                                        }
                                        if folder.id > 1 {
                                            if ui.button("Delete").clicked() {
                                                ui.close_menu();
                                                set_open(
                                                    open,
                                                    windows::WindowDeleteFolder::NAME,
                                                    true,
                                                    Some(Messge::DeleteFolder(
                                                        folder.name.to_string(),
                                                        folder.id,
                                                    )),
                                                );
                                            }
                                        }
                                    });
                                })
                                .body(|ui| {});
                            });
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Central Panel");
            ui.horizontal(|ui| {});
        });
    }
}

fn set_open(
    open: &mut HashMap<&'static str, Option<Messge>>,
    key: &'static str,
    is_open: bool,
    data: Option<Messge>,
) {
    if is_open {
        if !open.is_empty() {
            open.clear();
        }
        open.insert(key, data);
    } else {
        open.remove(key);
    }
}
