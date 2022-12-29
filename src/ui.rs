#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::collections::HashMap;
use std::vec;

use eframe::egui;
use egui_extras::RetainedImage;

use crate::*;

pub struct App {
    icons: HashMap<&'static str, RetainedImage>,

    windows: Vec<Box<dyn windows::Window>>,
    open: HashMap<&'static str, Option<Message>>,

    store: Store,

    feed: models::Feed,
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
            Box::new(windows::feed::AddWindow::default()),
            Box::new(windows::feed::DeleteWindow::default()),
            Box::new(windows::feed::EditWindow::default()),
            Box::new(windows::folder::AddWindow::default()),
            Box::new(windows::folder::DeleteWindow::default()),
            Box::new(windows::folder::EditWindow::default()),
        ];
        let open = HashMap::default();

        Self {
            store,
            icons,
            windows,
            open,
            feed: models::Feed::default(),
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
                    ui.menu_image_button(img.texture_id(ctx), img.size_vec2() * 0.5, |ui| {
                        let img = self.icons.get("rss").unwrap();
                        if ui
                            .add(egui::Button::image_and_text(
                                img.texture_id(ctx),
                                img.size_vec2() * 0.5,
                                "Feed",
                            ))
                            .clicked()
                        {
                            ui.close_menu();
                            set_open(
                                &mut self.open,
                                windows::feed::AddWindow::NAME,
                                true,
                                Some(Message::RefreshFolders),
                            );
                        }
                        let img = self.icons.get("folder").unwrap();
                        if ui
                            .add(egui::Button::image_and_text(
                                img.texture_id(ctx),
                                img.size_vec2() * 0.5,
                                "Folder",
                            ))
                            .clicked()
                        {
                            ui.close_menu();
                            set_open(
                                &mut self.open,
                                windows::folder::AddWindow::NAME,
                                true,
                                Some(Message::Normal),
                            );
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
                        let current_feed= &mut self.feed;
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
                        //     let id = current_feed.id;
                        //                             ui
                        //                                 .selectable_value(
                        //                                     &mut current_feed.id,
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
                                                windows::folder::EditWindow::NAME,
                                                true,
                                                Some(Message::Folder(
                                                    Action::Update,
                                                    folder.clone_without_feeds()
                                                )),
                                            );
                                        }
                                        if folder.id > 1 {
                                            if ui.button("Delete").clicked() {
                                                ui.close_menu();
                                                set_open(
                                                    open,
                                                    windows::folder::DeleteWindow::NAME,
                                                    true,
                                                    Some(Message::Folder(
                                                        Action::Delete,
                                                        folder.clone_without_feeds()
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
                                            if let Some(feeds) = &folder.feeds{
                                                feeds.iter().for_each(|feed| {
                                                    if ui
                                                        .selectable_value(
                                                            &mut current_feed.id,
                                                            feed.id,
                                                            feed.name.to_string(),
                                                        )
                                                        .context_menu(|ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.image(
                                                                    link_img.texture_id(ctx),
                                                                    link_img.size_vec2() * 0.5,
                                                                );
                                                                ui.label(feed.name.to_string());
                                                            });
                                                            ui.separator();
                                                            ui.button("Mark as read");
                                                            ui.separator();
                                                            if ui.button("Edit").clicked() {
                                                                ui.close_menu();
                                                                set_open(
                                                                    open,
                                                                    windows::feed::EditWindow::NAME,
                                                                    true,
                                                                    Some(Message::Feed(
                                                                        Action::Update,
                                                                        feed.clone()
                                                                    )),
                                                                );
                                                            }
                                                            if ui.button("Delete").clicked() {
                                                                ui.close_menu();
                                                                set_open(
                                                                    open,
                                                                    windows::feed::DeleteWindow::NAME,
                                                                    true,
                                                                    Some(Message::Feed(
                                                                        Action::Delete,
                                                                        feed.clone()
                                                                    )),
                                                                );
                                                            }
                                                        })
                                                        .changed()
                                                    {
                                                        *current_feed = feed.clone();
                                                        if let Err(e) = sender.send(Message::Feed (
                                                            Action::Fetch,
                                                            feed.clone(),
                                                        )) {
                                                            tracing::error!("{}", e);
                                                        }
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
                let mut name = self.feed.name.to_string();
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
                .default_width(280.)
                .width_range(128.0..=360.)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show_viewport(ui, |ui, rect| {
                            ui.set_width(rect.width());
                            ui.set_height(rect.height());
                            ui.label("left");
                        });
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label("content");
                });
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
