pub mod tristate_label {

    use eframe::egui::*;

    pub struct TristateLabel {
        included: bool,
        excluded: bool,
        label: egui::SelectableLabel,
    }

    impl TristateLabel {
        pub fn new(included: bool, excluded: bool, text: impl Into<WidgetText>) -> Self {
            Self {
                included,
                excluded,
                label: SelectableLabel::new(included || excluded, text),
            }
        }
    }

    impl Widget for TristateLabel {
        fn ui(self, ui: &mut Ui) -> Response {
            if self.included {
                ui.style_mut().visuals.selection.bg_fill = green(ui);
            } else if self.excluded {
                ui.style_mut().visuals.selection.bg_fill = red(ui);
            }
            self.label.ui(ui)
        }
    }

    fn green(ui: &mut Ui) -> egui::Color32 {
        let dark_mode = ui.visuals().dark_mode;
        if dark_mode {
            Color32::DARK_GREEN
        } else {
            Color32::LIGHT_GREEN
        }
    }

    fn red(ui: &mut Ui) -> egui::Color32 {
        let dark_mode = ui.visuals().dark_mode;
        if dark_mode {
            Color32::DARK_RED
        } else {
            Color32::LIGHT_RED
        }
    }
}
