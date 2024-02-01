use eframe::egui::Context;

/// Converts pixels to logical points.
pub fn pixels_to_points(ctx: &Context, pixels: f32) -> f32 {
    ctx.native_pixels_per_point()
        .map_or(pixels, |ppp| pixels / ppp)
}
