#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::env;
use std::fs;
use std::ops::Deref;
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
use eframe::egui::ImageButton;
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
            Box::new(|_cc| Box::new(App::new(store))),
        );
    });
    Ok(())
}

struct App {
    icons: HashMap<&'static str, RetainedImage>,

    windows: Vec<Box<dyn windows::Window>>,
    open: HashMap<&'static str, Option<Message>>,

    store: Store,

    source: models::Source,
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
        icons.insert(
            "link",
            RetainedImage::from_svg_bytes_with_size(
                "link",
                include_bytes!("../icons/link.svg"),
                egui_extras::image::FitTo::Original,
            )
            .unwrap(),
        );

        let windows: Vec<Box<dyn windows::Window>> = vec![
            Box::new(windows::WindowAddFeed::default()),
            Box::new(windows::WindowAddFolder::default()),
            Box::new(windows::WindowDeleteFolder::default()),
            Box::new(windows::WindowRenameFolder::default()),
            Box::new(windows::WindowDeleteSource::default()),
            Box::new(windows::WindowEditSource::default()),
        ];
        let open = HashMap::default();

        Self {
            store,
            icons,
            windows,
            open,
            source: models::Source::default(),
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
                                Some(Message::RefreshFolders),
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
                        let folder_img = self.icons.get("folder").unwrap();
                        let link_img = self.icons.get("link").unwrap();
                        let open = &mut self.open;
                        let current_source = &mut self.source;
                        let Store { folders, sender, .. } = &self.store;

                        // fn circle_icon(ui: &mut egui::Ui, openness: f32, response: &egui::Response) {
                        //     let stroke = ui.style().interact(&response).fg_stroke;
                        //     let radius = egui::lerp(2.0..=3.0, openness);
                        //     ui.painter().circle_filled(response.rect.center(), radius, stroke.color);
                        // }
                        // let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                        //     ui.ctx(),
                        //     ui.make_persistent_id("my_collapsing_state"),
                        //     false,
                        // );
                        // // let header_res = ui.horizontal(|ui| {
                        //    let header_res =          ui.with_layout(
                        //                 egui::Layout::top_down_justified(egui::Align::LEFT),
                        //                 |ui| {
                        //                     ui.horizontal(|ui| {
                        //     state.show_toggle_button(ui, circle_icon);
                        //     let id = current_source.id;
                        //                             ui
                        //                                 .selectable_value(
                        //                                     &mut current_source.id,
                        //                                     id,
                        //                                     "dsfldsl sdfjlds ",
                        //                                 );
                        // });
                        //                     });
                        // state.show_body_indented(&header_res.response, ui, |ui| ui.label("Body"));

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
                                        ui.horizontal(|ui| {
                                            ui.image(
                                                folder_img.texture_id(ctx),
                                                folder_img.size_vec2() * 0.5,
                                            );
                                            ui.label(folder.name.to_string());
                                        });
                                        ui.separator();
                                        ui.button("Mark as read");
                                        ui.separator();
                                        if ui.button("Rename").clicked() {
                                            ui.close_menu();
                                            set_open(
                                                open,
                                                windows::WindowRenameFolder::NAME,
                                                true,
                                                Some(Message::RenameFolder(
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
                                                    Some(Message::DeleteFolder(
                                                        folder.name.to_string(),
                                                        folder.id,
                                                    )),
                                                );
                                            }
                                        }
                                    });
                                })
                                .body(|ui| {
                                    ui.with_layout(
                                        egui::Layout::top_down_justified(egui::Align::LEFT),
                                        |ui| {
                                            if let Some(sources) = &folder.sources {
                                                sources.iter().for_each(|source| {
                                                    if ui
                                                        .selectable_value(
                                                            &mut current_source.id,
                                                            source.id,
                                                            source.name.to_string(),
                                                        )
                                                        .context_menu(|ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.image(
                                                                    link_img.texture_id(ctx),
                                                                    link_img.size_vec2() * 0.5,
                                                                );
                                                                ui.label(source.name.to_string());
                                                            });
                                                            ui.separator();
                                                            ui.button("Mark as read");
                                                            ui.separator();
                                                            if ui.button("Edit").clicked() {
                                                                ui.close_menu();
                                                                set_open(
                                                                    open,
                                                                    windows::WindowEditSource::NAME,
                                                                    true,
                                                                    Some(Message::EditSource(
                                                                        source.url.to_string(),
                                                                        source.name.to_string(),
                                                                        source.id,
                                                                        folder.id,
                                                                        None
                                                                    )),
                                                                );
                                                            }
                                                            if ui.button("Delete").clicked() {
                                                                ui.close_menu();
                                                                set_open(
                                                                    open,
                                                                    windows::WindowDeleteSource::NAME,
                                                                    true,
                                                                    Some(Message::DeleteSource(
                                                                        source.name.to_string(),
                                                                        source.id,
                                                                        folder.id
                                                                    )),
                                                                );
                                                            }
                                                        })
                                                        .changed()
                                                    {
                                                        *current_source = source.clone();
                                                        sender.send(Message::FetchFeedsBySource (
                                                            source.url.to_string(),
                                                            source.id,
                                                        ));
                                                    }
                                                });
                                            }
                                        },
                                    );
                                });
                            });
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut name = self.source.name.to_string();
                if name.is_empty() {
                    name.push_str("Feeds");
                }
                let link_img = self.icons.get("link").unwrap();
                ui.image(link_img.texture_id(ctx), link_img.size_vec2() * 0.5);
                ui.heading(name);
            });

            ui.separator();

            egui::SidePanel::left("Feeds SideBar")
                .resizable(true)
                .default_width(360.)
                .width_range(128.0..=360.)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {});
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {});
            });
        });
    }
}

fn set_open(
    open: &mut HashMap<&'static str, Option<Message>>,
    key: &'static str,
    is_open: bool,
    data: Option<Message>,
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
