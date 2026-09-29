#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

mod app;
mod command;
mod options;
mod preview;
mod regex_builder;
mod search;
mod theme;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 640.0])
            .with_min_inner_size([700.0, 440.0])
            .with_clamp_size_to_monitor_size(true),
        ..Default::default()
    };

    eframe::run_native(
        "rg studio",
        options,
        Box::new(|creation_context| {
            theme::apply(&creation_context.egui_ctx, true);
            Ok(Box::<app::RgStudio>::default())
        }),
    )
}
