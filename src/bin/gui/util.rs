use super::{SquareColour, SquareColourType, SQUARE_COLOURS};

use eframe::egui::Context;

pub fn points_to_pixels(ctx: &Context, points: f32) -> f32 {
    points / ctx.native_pixels_per_point().unwrap()
}

pub fn flip_square_colour(col: &mut SquareColour) {
    *col = if col.square_type == SquareColourType::Light {
        SQUARE_COLOURS.dark
    } else {
        SQUARE_COLOURS.light
    };
}
