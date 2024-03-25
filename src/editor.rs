use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::ParamSlider;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

use crate::ElysieraParams;

#[derive(Lens)]
struct Data {
    params: Arc<ElysieraParams>,
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (300, 500))
}

macro_rules! param {
    ($cx:expr, $id:ident, $name:expr) => {
        VStack::new($cx, |cx| {
            ParamSlider::new(cx, Data::params, |p| &p.$id)
                .width(Pixels(50.0))
                .height(Pixels(50.0));
            Label::new(cx, $name)
                .font_size(16.0)
                .height(Pixels(32.0));
        })
        .child_space(Stretch(1.0))
        .row_between(Pixels(8.0));
    };
}

const FONT_REGULAR: &[u8] = include_bytes!("../assets/Geist-Regular.otf");

pub(crate) fn create(
    params: Arc<ElysieraParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {

        cx.add_font_mem(FONT_REGULAR);
        cx.set_default_font(&["Geist"]);

        Data {
            params: params.clone(),
        }.build(cx);

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                param!(cx, mix, "Mix");
                param!(cx, post_gain, "Post Gain");
                param!(cx, pre_gain, "Pre Gain");
            });
            HStack::new(cx, |cx| {
                param!(cx, low_decay, "Low Decay");
                param!(cx, lf_crossover, "Low Crossover");
                param!(cx, mid_decay, "Mid Decay");
                param!(cx, hf_damping, "HF Damping");
                param!(cx, reverb_delay, "Reverb Delay");
                param!(cx, reverb_mix, "Reverb Mix");
            });
        });
    })
}