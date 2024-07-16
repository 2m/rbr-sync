#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{
    collections::HashSet,
    process::Command,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use eframe::egui;
use egui_notify::Toasts;
use ini::{Ini, WriteOption};
use rbr_sync_lib::{stages, Stage};
use tokio::runtime::Runtime;

mod widgets;
pub use crate::widgets::tristate_label;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
    let rt = Runtime::new().expect("Unable to create Runtime");

    // Enter the runtime so that `tokio::spawn` is available immediately.
    let _enter = rt.enter();

    // Execute the runtime in its own thread.
    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            resizable: Some(true),
            inner_size: Some([500.0, 400.0].into()),
            min_inner_size: Some([500.0, 400.0].into()),
            icon: Some(std::sync::Arc::new(
                eframe::icon_data::from_png_bytes(&include_bytes!("../icon.png")[..]).unwrap(),
            )),
            ..egui::ViewportBuilder::default()
        },
        ..Default::default()
    };

    // Run the GUI in the main thread.
    eframe::run_native(
        "RBR Sync",
        options,
        Box::new(|cc| Ok(Box::new(RbrSync::new(cc)))),
    )
    .unwrap();
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct RbrSync {
    // Sender/Receiver for async notifications.
    #[serde(skip)]
    tx: Sender<Result<Vec<Stage>, rbr_sync_lib::AppError>>,
    #[serde(skip)]
    rx: Receiver<Result<Vec<Stage>, rbr_sync_lib::AppError>>,

    #[serde(skip)]
    toasts: Toasts,

    token: String,
    token_plaintext: bool,

    db_id: String,

    fetching: bool,
    stages: Vec<Stage>,

    include_tags: HashSet<String>,
    exclude_tags: HashSet<String>,

    #[serde(skip)]
    favorites_file: String,
}

impl Default for RbrSync {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            tx,
            rx,

            toasts: Toasts::default(),

            token: "".to_owned(),
            token_plaintext: false,

            db_id: "".to_owned(),

            fetching: false,
            stages: Vec::new(),

            include_tags: HashSet::new(),
            exclude_tags: HashSet::new(),

            favorites_file: favorites_file(),
        }
    }
}

impl eframe::App for RbrSync {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(result) = self.rx.try_recv() {
            self.fetching = false;
            match result {
                Ok(stages) => self.stages = stages,
                Err(error) => {
                    println!("{}", error);
                    self.toasts.error(format!("{}", error));
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.inputs(ui);
            ui.separator();

            self.buttons(ui, ctx);
            ui.separator();

            self.outputs(ui);
        });

        self.toasts.show(ctx);
    }
}

pub fn favorites_file() -> String {
    let fav_path = if cfg!(target_os = "windows") {
        let stdout = Command::new("reg")
            .args([
                "query",
                "HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\Rallysimfans RBR",
                "/v",
                "InstallPath",
            ])
            .output()
            .expect("failed to execute process")
            .stdout;
        let reg_output = String::from_utf8(stdout).expect("Unable to parse output");
        let fav_dir = reg_output
            .split("REG_SZ")
            .last()
            .expect("part not found")
            .trim();
        format!("{}\\rsfdata\\cache\\", fav_dir)
    } else {
        "".to_owned()
    };
    format!("{fav_path}favorites.ini")
}

impl RbrSync {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn filtered_stages(&self) -> Vec<&Stage> {
        let mut filtered = self
            .stages
            .iter()
            .filter(|stage| self.include_tags.iter().any(|tag| stage.tags.contains(tag)))
            .filter(|stage| {
                self.exclude_tags
                    .iter()
                    .all(|tag| !stage.tags.contains(tag))
            })
            .collect::<Vec<&Stage>>();
        filtered.sort_by(|a, b| a.title.as_str().cmp(b.title.as_str()));
        filtered
    }

    fn inputs(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("my_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::widgets::global_dark_light_mode_switch(ui);
                    ui.heading("RBR Sync");
                });
                ui.label(built_info::PKG_VERSION);

                ui.end_row();

                ui.label("Token: ");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.token).password(!self.token_plaintext),
                    );

                    if ui
                        .add(egui::SelectableLabel::new(self.token_plaintext, "👁"))
                        .on_hover_text("Show/hide token")
                        .clicked()
                    {
                        self.token_plaintext = !self.token_plaintext;
                    }
                });

                ui.end_row();

                ui.label("Notion DB ID: ");
                ui.text_edit_singleline(&mut self.db_id);

                ui.end_row();

                ui.label("Favorites file: ");
                ui.label(self.favorites_file.as_str());

                ui.end_row();
            });
    }

    fn buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if ui.button("Fetch tags").clicked() {
                self.fetching = true;
                self.stages = Vec::new();
                fetch_stages(
                    self.token.clone(),
                    self.db_id.clone(),
                    self.tx.clone(),
                    ctx.clone(),
                )
            }
            if self.fetching {
                ui.spinner();
            }
            if ui
                .button(format!("Write {} stages", self.filtered_stages().len()))
                .clicked()
            {
                write_stages(self);
            }
        });
    }

    fn outputs(&mut self, ui: &mut egui::Ui) {
        use egui_extras::Size;

        egui_extras::StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(12.0))
            .size(Size::remainder())
            .vertical(|mut strip| {
                strip.cell(|ui| self.tags(ui));
                strip.cell(|ui| {
                    ui.separator();
                });
                strip.cell(|ui| self.stages(ui));
            });
    }

    fn tags(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut unique_tags = self
                        .stages
                        .iter()
                        .flat_map(|s| s.tags.clone())
                        .collect::<HashSet<String>>()
                        .into_iter()
                        .collect::<Vec<String>>();
                    unique_tags.sort();

                    for tag in unique_tags {
                        if ui
                            .add(tristate_label::TristateLabel::new(
                                self.include_tags.contains(&tag),
                                self.exclude_tags.contains(&tag),
                                tag.clone(),
                            ))
                            .clicked()
                        {
                            if self.include_tags.contains(&tag) {
                                self.include_tags.remove(&tag);
                                self.exclude_tags.insert(tag.clone());
                            } else if self.exclude_tags.contains(&tag) {
                                self.exclude_tags.remove(&tag);
                            } else {
                                self.include_tags.insert(tag.clone());
                            }
                        }
                    }
                });
            });
    }

    fn stages(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.push_id(777, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    use egui_extras::Column;

                    let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

                    egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::auto())
                        .column(Column::remainder())
                        .header(text_height, |mut header| {
                            header.col(|ui| {
                                ui.strong("#");
                            });
                            header.col(|ui| {
                                ui.strong("ID");
                            });
                            header.col(|ui| {
                                ui.strong("Stage");
                            });
                            header.col(|ui| {
                                ui.strong("Labels");
                            });
                        })
                        .body(|body| {
                            body.rows(text_height, self.filtered_stages().len(), |mut row| {
                                let idx = row.index();
                                row.col(|ui| {
                                    ui.label((idx + 1).to_string());
                                });
                                row.col(|ui| {
                                    ui.label(self.filtered_stages()[idx].id.to_string());
                                });
                                row.col(|ui| {
                                    ui.label(self.filtered_stages()[idx].title.clone());
                                });
                                row.col(|ui| {
                                    for tag in self.filtered_stages()[idx].tags.clone() {
                                        ui.label(tag);
                                    }
                                });
                            });
                        });
                });
            });
        });
    }
}

fn fetch_stages(
    token: String,
    db_id: String,
    tx: Sender<Result<Vec<Stage>, rbr_sync_lib::AppError>>,
    ctx: egui::Context,
) {
    tokio::spawn(async move {
        let stages = stages(token.as_str(), db_id.as_str()).await;

        // After parsing the response, notify the GUI thread of the new value.
        let _ = tx.send(stages);
        ctx.request_repaint();
    });
}

fn write_stages(rbr_sync: &RbrSync) {
    let mut favorites = Ini::load_from_file(rbr_sync.favorites_file.clone()).unwrap_or_default();
    favorites.delete(Some("FavoriteStages"));

    for stage in rbr_sync.filtered_stages() {
        favorites
            .with_section(Some("FavoriteStages"))
            .set(stage.id.to_string(), "f");
    }

    favorites
        .write_to_file_opt(
            rbr_sync.favorites_file.clone(),
            WriteOption {
                kv_separator: " = ",
                ..Default::default()
            },
        )
        .unwrap();
}
