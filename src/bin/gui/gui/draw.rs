use eframe::{
    egui::{Color32, Rect, Rounding, Shape, Stroke, Ui},
    epaint::RectShape,
};

/// Paints the area on `ui` defined by `rect` the color `color`.
pub fn paint_area_with_color(ui: &Ui, rect: Rect, color: Color32) {
    ui.painter().add(Shape::Rect(RectShape::new(
        rect,
        Rounding::ZERO,
        color,
        Stroke::default(),
    )));
}
