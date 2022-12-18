#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::ops::Div;
use std::path::Path;
use std::path::PathBuf;
use std::vec;

use anyhow::Result;
use eframe::egui;
use eframe::egui::style::Margin;
use eframe::egui::Button;
use eframe::egui::Sense;
use eframe::epaint::ahash::{HashMap, HashMapExt};
use eframe::epaint::ColorImage;
use eframe::epaint::Rect;
use eframe::epaint::Shape;
use egui_extras::RetainedImage;
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
        PRAGMA journal_mode = wal;
        PRAGMA foreign_keys = on;
        PRAGMA synchronous = normal;
        "#,
        )?;
        Ok(())
    });
    tracing::info!("{:?}", config_dir);
    tracing::info!("{:?}", db);

    let pool = r2d2::Pool::new(db)?;

    // 1️⃣ Define migrations
    let migrations = Migrations::new(vec![M::up(include_str!("../migrations/0-sources.sql"))]);

    dbg!(&migrations);

    let mut conn = pool.get()?;

    // 2️⃣ Update the database schema, atomically
    dbg!(migrations.to_latest(&mut conn)).unwrap();

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
        Box::new(|_cc| Box::new(App::default())),
    );

    Ok(())
}

struct App {
    icons: HashMap<&'static str, RetainedImage>,

    windows: Vec<Box<dyn Window>>,
    open: BTreeSet<String>,
}

impl Default for App {
    fn default() -> Self {
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

        let windows: Vec<Box<dyn Window>> = vec![
            Box::new(WindowAddFeed::default()),
            Box::new(WindowAddFolder::default()),
        ];
        let open = BTreeSet::default();

        Self {
            icons,
            windows,
            open,
        }
    }
}

impl App {
    pub fn windows(&mut self, ctx: &egui::Context, size: egui::Vec2) {
        let Self { windows, open, .. } = self;
        for window in windows {
            let mut is_open = open.contains(window.name());
            if is_open {
                window.show(ctx, &mut is_open, size);
            }
            if window.is_closed() {
                is_open = false;
            }
            set_open(open, window.name(), is_open);
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
                            dbg!("feed");
                            set_open(&mut self.open, WindowAddFeed::NAME, true);
                            ui.close_menu()
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
                            dbg!("folder");
                            set_open(&mut self.open, WindowAddFolder::NAME, true);
                            ui.close_menu()
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
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Central Panel");
            ui.horizontal(|ui| {
                // let name_label = ui.label("Your name: ");
                // ui.text_edit_singleline(&mut self.name)
                //     .labelled_by(name_label.id);
            });
            // ui.add(egui::slider::new(&mut self.age, 0..=120).text("age"));
            // if ui.button("click each year").clicked() {
            //     self.age += 1;
            // }
        });
    }
}

fn set_open(open: &mut BTreeSet<String>, key: &'static str, is_open: bool) {
    if is_open {
        if !open.contains(key) {
            open.insert(key.to_owned());
        }
    } else {
        open.remove(key);
    }
}
