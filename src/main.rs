#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::{
    collections::HashSet,
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use eframe::egui;
use rbr_sync_lib::{stages, Stage};
use tokio::runtime::Runtime;

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
        resizable: false,
        initial_window_size: Some([500.0, 700.0].into()),
        max_window_size: Some([500.0, 700.0].into()),
        ..Default::default()
    };

    // Run the GUI in the main thread.
    eframe::run_native(
        "RBR Sync",
        options,
        Box::new(|cc| Box::new(RbrSync::new(cc))),
    );
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct RbrSync {
    // Sender/Receiver for async notifications.
    #[serde(skip)]
    tx: Sender<Vec<Stage>>,
    #[serde(skip)]
    rx: Receiver<Vec<Stage>>,

    token: String,
    db_id: String,

    fetching: bool,
    stages: Vec<Stage>,

    selected_tags: HashSet<String>,
}

impl Default for RbrSync {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();

        Self {
            tx,
            rx,

            token: "".to_owned(),
            db_id: "".to_owned(),

            fetching: false,
            stages: Vec::new(),

            selected_tags: HashSet::new(),
        }
    }
}

impl eframe::App for RbrSync {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update the counter with the async response.
        if let Ok(stages) = self.rx.try_recv() {
            self.fetching = false;
            self.stages = stages;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.contents(ui, ctx);
                });

            ui.separator();

            egui::ScrollArea::vertical()
                .always_show_scroll(true)
                .max_height(300.)
                .show(ui, |ui| {
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
                            .add(egui::SelectableLabel::new(
                                self.selected_tags.contains(&tag),
                                tag.clone(),
                            ))
                            .clicked()
                        {
                            if !self.selected_tags.insert(tag.clone()) {
                                self.selected_tags.remove(&tag);
                            }
                        }
                    }
                });

            ui.separator();

            if ui.button("Write").clicked() {
                println!("{:?}", self.selected_tags);
            }
        });
    }
}

impl RbrSync {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(2.5);

        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn contents(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("RBR Sync");
        ui.allocate_space(egui::vec2(ui.available_width(), 0.0));

        ui.end_row();

        ui.label("Token: ");
        ui.text_edit_singleline(&mut self.token);

        ui.end_row();

        ui.label("Notion DB ID: ");
        ui.text_edit_singleline(&mut self.db_id);

        ui.end_row();

        if ui.button("Fetch tags").clicked() {
            self.fetching = true;
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

        ui.end_row();
    }
}

fn fetch_stages(token: String, db_id: String, tx: Sender<Vec<Stage>>, ctx: egui::Context) {
    tokio::spawn(async move {
        let stages = stages(token.as_str(), db_id.as_str())
            .await
            .expect("Unable to parse response");

        // After parsing the response, notify the GUI thread of the new value.
        let _ = tx.send(stages);
        ctx.request_repaint();
    });
}
