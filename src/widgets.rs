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
                ui.style_mut().visuals.selection.bg_fill = Color32::DARK_GREEN;
            } else if self.excluded {
                ui.style_mut().visuals.selection.bg_fill = Color32::DARK_RED;
            }
            self.label.ui(ui)
        }
    }
}
