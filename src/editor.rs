use crate::ElysieraParams;
use std::sync::Arc;
use nih_plug::editor::Editor;
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};

mod param_knob;

pub fn default_state() -> Arc<EguiState> {
    EguiState::from_size(300, 180)
}

pub fn editor(
    params: Arc<ElysieraParams>
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        |egui_ctx, _| {
            let mut fonts = egui::FontDefinitions::default();

            fonts.font_data.insert(
                "Geist".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/Geist-Regular.otf")),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Geist".to_owned());

            egui_ctx.set_fonts(fonts);
        },
        move |egui_ctx, setter, _state| {
            egui::CentralPanel::default().show(egui_ctx, |ui| {
                ui.label("Mix");
                ui.add(param_knob::ParamSlider::for_param(&params.mix, setter));
            });
        }
    )
}