use std::sync::Arc;
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::{egui::{
    self, emath::{self, Rot2}, vec2, Key, Response, Sense, Stroke, TextEdit, TextStyle, Ui, Vec2, Widget,
    WidgetText,
}, widgets::util};
use itertools::Itertools;
use lazy_static::lazy_static;
use parking_lot::Mutex;

/// When shift+dragging a parameter, one pixel dragged corresponds to this much change in the
/// noramlized parameter.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.0015;

lazy_static! {
    static ref DRAG_NORMALIZED_START_VALUE_MEMORY_ID: egui::Id = egui::Id::new((file!(), 0));
    static ref DRAG_AMOUNT_MEMORY_ID: egui::Id = egui::Id::new((file!(), 1));
    static ref VALUE_ENTRY_MEMORY_ID: egui::Id = egui::Id::new((file!(), 2));
}

/// A slider widget similar to [`egui::widgets::Slider`] that knows about NIH-plug parameters ranges
/// and can get values for it. The slider supports double click and control click to reset,
/// shift+drag for granular dragging, text value entry by clicking on the value text.
///
/// TODO: Vertical orientation
/// TODO: Check below for more input methods that should be added
/// TODO: Decouple the logic from the drawing so we can also do things like nobs without having to
///       repeat everything
/// TODO: Add WidgetInfo annotations for accessibility
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct ParamSlider<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,

    draw_value: bool,
    diameter: f32,

    /// Will be set in the `ui()` function so we can request keyboard input focus on Alt+click.
    keyboard_focus_id: Option<egui::Id>,
}

impl<'a, P: Param> ParamSlider<'a, P> {
    /// Create a new slider for a parameter. Use the other methods to modify the slider before
    /// passing it to [`Ui::add()`].
    pub fn for_param(param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Self {
            param,
            setter,

            draw_value: true,
            diameter: 32.0,

            keyboard_focus_id: None,
        }
    }

    /// Don't draw the text slider's current value after the slider.
    pub fn without_value(mut self) -> Self {
        self.draw_value = false;
        self
    }

    /// Set a custom diameter for the slider knob.
    pub fn with_diameter(mut self, diameter: f32) -> Self {
        self.diameter = diameter;
        self
    }

    fn plain_value(&self) -> P::Plain {
        self.param.modulated_plain_value()
    }

    fn normalized_value(&self) -> f32 {
        self.param.modulated_normalized_value()
    }

    fn string_value(&self) -> String {
        self.param.to_string()
    }

    /// Enable the keyboard entry part of the widget.
    fn begin_keyboard_entry(&self, ui: &Ui) {
        ui.memory_mut(|mem| mem.request_focus(self.keyboard_focus_id.unwrap()));

        // Always initialize the field to the current value, that seems nicer than having to
        // being typing from scratch
        let value_entry_mutex = ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_default::<Arc<Mutex<String>>>(*VALUE_ENTRY_MEMORY_ID)
                .clone()
        });
        *value_entry_mutex.lock() = self.string_value();
    }

    fn keyboard_entry_active(&self, ui: &Ui) -> bool {
        ui.memory(|mem| mem.has_focus(self.keyboard_focus_id.unwrap()))
    }

    fn begin_drag(&self) {
        self.setter.begin_set_parameter(self.param);
    }

    fn set_normalized_value(&self, normalized: f32) {
        // This snaps to the nearest plain value if the parameter is stepped in some way.
        // TODO: As an optimization, we could add a `const CONTINUOUS: bool` to the parameter to
        //       avoid this normalized->plain->normalized conversion for parameters that don't need
        //       it
        let value = self.param.preview_plain(normalized);
        if value != self.plain_value() {
            self.setter.set_parameter(self.param, value);
        }
    }

    /// Begin and end drag still need to be called when using this. Returns `false` if the string
    /// could no tbe parsed.
    fn set_from_string(&self, string: &str) -> bool {
        match self.param.string_to_normalized_value(string) {
            Some(normalized_value) => {
                self.set_normalized_value(normalized_value);
                true
            }
            None => false,
        }
    }

    /// Begin and end drag still need to be called when using this..
    fn reset_param(&self) {
        self.setter
            .set_parameter(self.param, self.param.default_plain_value());
    }

    fn granular_drag(&self, ui: &Ui, drag_delta: Vec2) {
        // Remember the intial position when we started with the granular drag. This value gets
        // reset whenever we have a normal itneraction with the slider.
        let start_value = if Self::get_drag_amount_memory(ui) == 0.0 {
            Self::set_drag_normalized_start_value_memory(ui, self.normalized_value());
            self.normalized_value()
        } else {
            Self::get_drag_normalized_start_value_memory(ui)
        };

        let total_drag_distance = drag_delta.x + Self::get_drag_amount_memory(ui);
        Self::set_drag_amount_memory(ui, total_drag_distance);

        self.set_normalized_value(
            (start_value + (total_drag_distance * GRANULAR_DRAG_MULTIPLIER)).clamp(0.0, 1.0),
        );
    }

    fn end_drag(&self) {
        self.setter.end_set_parameter(self.param);
    }

    fn get_drag_normalized_start_value_memory(ui: &Ui) -> f32 {
        ui.memory(|mem| {
            mem.data
                .get_temp(*DRAG_NORMALIZED_START_VALUE_MEMORY_ID)
                .unwrap_or(0.5)
        })
    }

    fn set_drag_normalized_start_value_memory(ui: &Ui, amount: f32) {
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(*DRAG_NORMALIZED_START_VALUE_MEMORY_ID, amount)
        });
    }

    fn get_drag_amount_memory(ui: &Ui) -> f32 {
        ui.memory(|mem| mem.data.get_temp(*DRAG_AMOUNT_MEMORY_ID).unwrap_or(0.0))
    }

    fn set_drag_amount_memory(ui: &Ui, amount: f32) {
        ui.memory_mut(|mem| mem.data.insert_temp(*DRAG_AMOUNT_MEMORY_ID, amount));
    }

    fn paint_arc(&self, ui: &Ui,
        center: egui::Pos2,
        inner_radius: f32,
        outer_radius: f32,
        start_angle: f32,
        end_angle: f32,
        fill: egui::Color32,
        stroke: Stroke,
    ) {
        const RESOLUTION: usize = 32;

        let generate_arc_points = |radius| {
            (0..=32).map(move |i| {
                let angle = egui::lerp(start_angle..=end_angle, i as f32 / RESOLUTION as f32);
                center + Vec2::angled(angle) * radius
            })
        };

        let inner_radius = inner_radius.max(0.1);

        let outer_arc = generate_arc_points(outer_radius).collect::<Vec<_>>();
        let inner_arc = generate_arc_points(inner_radius).collect::<Vec<_>>();

        outer_arc
            .iter()
            .zip(inner_arc.iter())
            .tuple_windows()
            .for_each(|((outer_1, inner_1), (outer_2, inner_2))| {
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![*outer_1, *inner_1, *inner_2, *outer_2],
                    fill,
                    Stroke::new(1.0, fill),
                ));
            });

        let outline_points: Vec<egui::Pos2> = outer_arc
            .iter()
            .chain(inner_arc.iter().rev())
            .copied()
            .collect();

        ui.painter().add(egui::Shape::closed_line(outline_points, stroke));
    }

    fn slider_ui(&self, ui: &Ui, response: &mut Response) {
        // Handle user input
        // TODO: Optionally (since it can be annoying) add scrolling behind a builder option
        if response.drag_started() {
            // When beginning a drag or dragging normally, reset the memory used to keep track of
            // our granular drag
            self.begin_drag();
            Self::set_drag_amount_memory(ui, 0.0);
        }
        if let Some(click_pos) = response.interact_pointer_pos() {
            if ui.input(|i| i.modifiers.command) {
                // Like double clicking, Ctrl+Click should reset the parameter
                self.reset_param();
                response.mark_changed();
            // // FIXME: This releases the focus again when you release the mouse button without
            // //        moving the mouse a bit for some reason
            // } else if ui.input().modifiers.alt && self.draw_value {
            //     // Allow typing in the value on an Alt+Click. Right now this is shown as part of the
            //     // value field, so it only makes sense when we're drawing that.
            //     self.begin_keyboard_entry(ui);
            } else if ui.input(|i| i.modifiers.shift) {
                // And shift dragging should switch to a more granulra input method
                self.granular_drag(ui, response.drag_delta());
                response.mark_changed();
            } else {
                let proportion =
                    emath::remap_clamp(click_pos.x, response.rect.x_range(), 0.0..=1.0) as f64;
                self.set_normalized_value(proportion as f32);
                response.mark_changed();
                Self::set_drag_amount_memory(ui, 0.0);
            }
        }
        if response.double_clicked() {
            self.reset_param();
            response.mark_changed();
        }
        if response.drag_released() {
            self.end_drag();
        }

        // And finally draw the thing
        if ui.is_rect_visible(response.rect) {
            let visuals = *ui.style().interact(&response);

            let orientation = Rot2::from_angle(std::f32::consts::TAU * 0.75);
            let center_angle = (orientation * Vec2::RIGHT).angle();
            let spread_angle = (std::f32::consts::TAU * 0.5) * 1.0_f32.clamp(0.0, 1.0);

            let (min_angle, max_angle) = (
                center_angle - spread_angle,
                center_angle + spread_angle,
            );

            let outer_radius = self.diameter / 2.0;
            let inner_radius = outer_radius * (1.0 - 0.66_f32.clamp(0.0, 1.0));

            self.paint_arc(
                ui,
                response.rect.center(),
                inner_radius,
                outer_radius,
                min_angle,
                max_angle,
                ui.style().visuals.faint_bg_color,
                ui.style().visuals.window_stroke(),
            );

            self.paint_arc(
                ui,
                response.rect.center(),
                (inner_radius - visuals.expansion).max(0.0),
                outer_radius + visuals.expansion,
                egui::remap_clamp(0.0, 0.0..=1.0, min_angle..=max_angle),
                egui::remap_clamp(self.normalized_value(), 0.0..=1.0, min_angle..=max_angle),
                visuals.bg_fill,
                visuals.fg_stroke,
            );
        }
    }

    fn value_ui(&self, ui: &mut Ui) {
        let visuals = ui.visuals().widgets.inactive;
        let should_draw_frame = ui.visuals().button_frame;
        let padding = ui.spacing().button_padding;

        // Either show the parameter's label, or show a text entry field if the parameter's label
        // has been clicked on
        let keyboard_focus_id = self.keyboard_focus_id.unwrap();
        if self.keyboard_entry_active(ui) {
            let value_entry_mutex = ui.memory_mut(|mem| {
                mem.data
                    .get_temp_mut_or_default::<Arc<Mutex<String>>>(*VALUE_ENTRY_MEMORY_ID)
                    .clone()
            });
            let mut value_entry = value_entry_mutex.lock();

            ui.add(
                TextEdit::singleline(&mut *value_entry)
                    .id(keyboard_focus_id)
                    .font(TextStyle::Monospace),
            );
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                // Cancel when pressing escape
                ui.memory_mut(|mem| mem.surrender_focus(keyboard_focus_id));
            } else if ui.input(|i| i.key_pressed(Key::Enter)) {
                // And try to set the value by string when pressing enter
                self.begin_drag();
                self.set_from_string(&value_entry);
                self.end_drag();

                ui.memory_mut(|mem| mem.surrender_focus(keyboard_focus_id));
            }
        } else {
            let text = WidgetText::from(self.string_value()).into_galley(
                ui,
                None,
                ui.available_width() - (padding.x * 2.0),
                TextStyle::Button,
            );

            let response = ui.allocate_response(text.size() + (padding * 2.0), Sense::click());
            if response.clicked() {
                self.begin_keyboard_entry(ui);
            }

            if ui.is_rect_visible(response.rect) {
                if should_draw_frame {
                    let fill = visuals.bg_fill;
                    let stroke = visuals.bg_stroke;
                    ui.painter().rect(
                        response.rect.expand(visuals.expansion),
                        visuals.rounding,
                        fill,
                        stroke,
                    );
                }

                let text_pos = ui
                    .layout()
                    .align_size_within_rect(text.size(), response.rect.shrink2(padding))
                    .min;

                ui.painter().add(egui::epaint::TextShape::new(
                    text_pos,
                    text,
                    visuals.fg_stroke.color,
                ));
            }
        }
    }
}

impl<P: Param> Widget for ParamSlider<'_, P> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let width = self.diameter;

        ui.vertical(|ui| {
            // Allocate space, but add some padding on the top and bottom to make it look a bit slimmer.
            let height = ui
                .text_style_height(&TextStyle::Body)
                .max(ui.spacing().interact_size.y * 0.8);
            let slider_height = ui.painter().round_to_pixel(height * 0.8);
            let mut response = ui
                .vertical(|ui| {
                    ui.allocate_space(vec2(width, (height - slider_height) / 2.0));
                    let response = ui.allocate_response(
                        vec2(width, slider_height),
                        Sense::click_and_drag(),
                    );
                    let (kb_edit_id, _) =
                        ui.allocate_space(vec2(width, (height - slider_height) / 2.0));
                    // Allocate an automatic ID for keeping track of keyboard focus state
                    // FIXME: There doesn't seem to be a way to generate IDs in the public API, not sure how
                    //        you're supposed to do this
                    self.keyboard_focus_id = Some(kb_edit_id);

                    response
                })
                .inner;

            self.slider_ui(ui, &mut response);
            if self.draw_value {
                self.value_ui(ui);
            }

            response
        })
        .inner
    }
}