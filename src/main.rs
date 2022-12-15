#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use eframe::egui;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, OpenFlags};
use rusqlite_migration::{Migrations, M};

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
    let migrations = Migrations::new(vec![M::up(include_str!("../migrations/0-feeds.sql"))]);

    dbg!(&migrations);

    let mut conn = pool.get()?;

    // 2️⃣ Update the database schema, atomically
    dbg!(migrations.to_latest(&mut conn)).unwrap();

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,

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
    name: String,
    age: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("SideBar")
            .resizable(true)
            .default_width(220.)
            .width_range(64.0..=220.)
            .show_animated(ctx, true, |ui| {
                ui.heading("Side Panel");
                ui.separator();
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Central Panel");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
    }
}
