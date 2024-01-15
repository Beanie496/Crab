use eframe::egui::Context;

pub fn points_to_pixels(ctx: &Context, points: f32) -> f32 {
    points / ctx.native_pixels_per_point().unwrap()
}
