use eframe::egui::Context;

/// Converts logical points to physical pixels.
pub fn points_to_pixels(ctx: &Context, points: f32) -> f32 {
    ctx.native_pixels_per_point()
        .map_or(points, |ppp| points / ppp)
}
