use eframe::{
    egui::{self, Color32, Frame, Id, Vec2, ViewportBuilder},
    run_simple_native, Error, NativeOptions,
};

fn main() -> Result<(), Error> {
    let title = "Crab - A chess engine";

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(title)
            .with_decorations(false)
            .with_inner_size(Vec2::new(1920.0, 1080.0)),
        ..Default::default()
    };

    run_simple_native(title, options, move |ctx, _frame| {
        let ppp = ctx.native_pixels_per_point().unwrap();
        let board_area_width = 880.0 / ppp;
        let info_box_width = (1920.0 - 880.0) / ppp;
        egui::SidePanel::left(Id::new(0))
            .resizable(false)
            .show_separator_line(false)
            .exact_width(board_area_width)
            .frame(Frame::none().fill(Color32::WHITE))
            .show(ctx, |_ui| {});
        egui::SidePanel::right(Id::new(1))
            .resizable(false)
            .show_separator_line(false)
            .exact_width(info_box_width)
            .frame(Frame::none().fill(Color32::RED))
            .show(ctx, |_ui| {});
    })
}
