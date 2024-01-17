use eframe::egui::{Color32, Context};

pub fn points_to_pixels(ctx: &Context, points: f32) -> f32 {
    points / ctx.native_pixels_per_point().unwrap()
}

pub fn flip_colour(col: &mut Color32, col1: &Color32, col2: &Color32) {
    *col = if *col == *col1 { *col2 } else { *col1 };
}
