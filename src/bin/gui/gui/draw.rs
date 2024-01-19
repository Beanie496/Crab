use eframe::{
    egui::{Color32, Rect, Rounding, Shape, Stroke, Ui},
    epaint::RectShape,
};

/// Paints the area on `ui` defined by `rect` the colour `colour`.
pub fn paint_area_with_colour(ui: &Ui, rect: Rect, colour: Color32) {
    ui.painter().add(Shape::Rect(RectShape::new(
        rect,
        Rounding::ZERO,
        colour,
        Stroke::default(),
    )));
}
