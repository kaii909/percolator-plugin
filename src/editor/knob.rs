use super::theme::*;
use nice_plug::prelude::*;
use egui::{self, vec2, Align, Layout, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2};
use std::f32::consts::PI;

const ARC_START: f32 = -0.75 * PI;
const ARC_SWEEP: f32 = 1.5 * PI;

// angle 0 points up, positive angles go clockwise
fn arc_point(center: Pos2, radius: f32, angle: f32) -> Pos2 {
    center + radius * vec2(angle.sin(), -angle.cos())
}

fn arc_points(center: Pos2, radius: f32, from: f32, to: f32) -> Vec<Pos2> {
    let steps = (((to - from).abs() / 0.1).ceil() as usize).max(2);
    (0..=steps)
        .map(|i| arc_point(center, radius, from + (to - from) * i as f32 / steps as f32))
        .collect()
}

pub fn knob<P: Param>(ui: &mut Ui, param: &P, setter: &ParamSetter, size: f32) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());

    if response.drag_started() {
        setter.begin_set_parameter(param);
    }
    if response.dragged() {
        let speed = if ui.input(|i| i.modifiers.shift) { 0.0015 } else { 0.006 };
        let next = (param.unmodulated_normalized_value() - response.drag_delta().y * speed)
            .clamp(0.0, 1.0);
        setter.set_parameter_normalized(param, next);
        response.mark_changed();
    }
    if response.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if response.double_clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, param.default_plain_value());
        setter.end_set_parameter(param);
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = rect.center();
        let radius = size * 0.5 - 6.0;
        let value = param.unmodulated_normalized_value();
        let angle = ARC_START + ARC_SWEEP * value;
        let accent = if response.hovered() || response.dragged() { ACCENT_HOT } else { ACCENT };

        painter.add(Shape::line(
            arc_points(center, radius, ARC_START, ARC_START + ARC_SWEEP),
            Stroke::new(ARC_WIDTH, TRACK),
        ));
        painter.add(Shape::line(
            arc_points(center, radius, ARC_START, angle),
            Stroke::new(ARC_WIDTH, accent),
        ));
        painter.circle_filled(center, radius - 8.0, BODY);
        painter.line_segment(
            [arc_point(center, radius * 0.35, angle), arc_point(center, radius - 10.0, angle)],
            Stroke::new(POINTER_WIDTH, POINTER),
        );
    }

    response
}

pub fn labeled_knob<P: Param>(ui: &mut Ui, param: &P, setter: &ParamSetter, size: f32) {
    ui.vertical(|ui| {
        ui.set_width(size + 16.0);
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.colored_label(TEXT, param.name());
            knob(ui, param, setter, size);
            let value = param.unmodulated_normalized_value();
            ui.colored_label(TEXT, param.normalized_value_to_string(value, true));
        });
    });
}
