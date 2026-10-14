use crate::PercolatorParams;
use egui::{Margin, RichText, Vec2};
use nice_plug::{context::gui::GuiContext, editor::dpi::LogicalSize, prelude::*};
use nice_plug_egui::{NiceEguiApp, resizable_window::ResizableWindow, widgets::ParamSlider};
use std::sync::Arc;

mod fonts;
mod knob;
mod theme;

pub(crate) const MIN_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(300.0, 300.0);
pub(crate) const RESIZE_HINT: ResizeHint =
    ResizeHint::resizable().with_min_logical_size(MIN_WINDOW_SIZE);
pub(crate) const INITIAL_ZOOM_FACTOR: f32 = 1.0;

#[allow(dead_code)]
fn param_row<P: Param>(ui: &mut egui::Ui, param: &P, setter: &ParamSetter) {
    ui.label(param.name());
    ui.add(ParamSlider::for_param(param, setter).with_width(180.0));
    ui.end_row();
}

fn ui_header(text: &str, size: f32, ui: &mut egui::Ui) {
    ui.label(RichText::new(text).heading().size(size));
}

/// Here you can store any state you need for your GUI.
///
/// This state persists across editor openings.
pub struct PercolatorEditor {
    pub(crate) open_state: Option<OpenEditorState>,
    pub(crate) zoom_factor: f32,
    pub(crate) track_info: Option<TrackInfo>,

    pub(crate) params: Arc<PercolatorParams>,
}

/// Editor-related state that only exists while an editor window is open.
pub struct OpenEditorState {
    _egui_ctx: egui::Context,
    nice_gui_ctx: GuiContext,
}

impl OpenEditorState {
    fn new(egui_ctx: egui::Context, nice_gui_ctx: GuiContext) -> Self {
        Self {
            _egui_ctx: egui_ctx,
            nice_gui_ctx,
        }
    }
}

impl NiceEguiApp for PercolatorEditor {
    fn build(
        &mut self,
        egui_ctx: egui::Context,
        nice_gui_ctx: GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        fonts::install(&egui_ctx);
        self.open_state = Some(OpenEditorState::new(egui_ctx, nice_gui_ctx));
        Ok(())
    }

    /// Called when the editor is closed. This is needed because the plugin editor window
    /// can be opened and closed multiple times.
    ///
    /// If your app holds onto an `egui::Context` context, then all copies of that context
    /// should be dropped so that egui can properly clean up.
    fn editor_closed(&mut self) {
        self.open_state = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(state) = self.open_state.as_mut() else {
            return;
        };

        let setter = state.nice_gui_ctx.param_setter();

        ResizableWindow::new("res-wind")
            .min_size(Vec2::new(MIN_WINDOW_SIZE.width, MIN_WINDOW_SIZE.height))
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(Margin::same(5))
                    .show(ui, |ui| {
                        // Display the track information
                        let track_info = self.track_info.clone().unwrap_or_default();
                        let name = track_info.name();
                        if name.is_empty() {
                            ui.label("Track name: (unknown)");
                        } else {
                            ui.label(format!("Track name: {name}"));
                        }
                        if let Some(color) = track_info.color() {
                            let (r, g, b, a) = color.rgba();

                            ui.horizontal(|ui| {
                                ui.label("Track color: ");
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense::hover(),
                                );

                                ui.painter().rect_filled(
                                    rect,
                                    2.0,
                                    egui::Color32::from_rgba_unmultiplied(r, g, b, a),
                                );
                            });
                        }

                        // UI =============================================================================================

                        ui_header("harmonic percolator", 22.0, ui);
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            knob::labeled_knob(ui, &self.params.harmonics, &setter, 90.0);
                            knob::labeled_knob(ui, &self.params.balance, &setter, 90.0);
                            knob::labeled_knob(ui, &self.params.character, &setter, 90.0);
                        });

                        // ================================================================================================

                        let before = self.zoom_factor;
                        egui::ComboBox::from_label("zoom factor")
                            .selected_text(format!(
                                "{}%",
                                (self.zoom_factor * 100.0).round() as u32
                            ))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.zoom_factor, 0.5, "50%");
                                ui.selectable_value(&mut self.zoom_factor, 0.75, "75%");
                                ui.selectable_value(&mut self.zoom_factor, 1.0, "100%");
                                ui.selectable_value(&mut self.zoom_factor, 1.25, "125%");
                                ui.selectable_value(&mut self.zoom_factor, 1.5, "150%");
                                ui.selectable_value(&mut self.zoom_factor, 1.75, "175%");
                                ui.selectable_value(&mut self.zoom_factor, 2.0, "200%");
                            });
                        if self.zoom_factor != before {
                            ui.set_zoom_factor(self.zoom_factor);
                        }
                    });
            });
    }

    fn track_info_changed(&mut self, info: TrackInfo) {
        self.track_info = Some(info);
    }
}
