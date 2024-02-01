use eframe::{
    egui::{widgets::Button, Align, Color32, Layout, Rect, Rounding, Shape, Stroke, Ui},
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

/// Adds a button on the given [`Ui`] at the given region with the given text
/// that executes `on_click` when the button is clicked.
pub fn add_button_to_region<T: FnOnce()>(ui: &mut Ui, region: Rect, text: &str, on_click: T) {
    let button = Button::new(text).min_size(region.size());
    if ui
        .child_ui(region, Layout::left_to_right(Align::Center))
        .add(button)
        .clicked()
    {
        on_click();
    }
}
