mod app;
mod cli;
mod command;
mod options;
mod preview;
mod regex_builder;
mod search;
mod theme;
mod ui;

fn main() -> std::process::ExitCode {
    match cli::run() {
        Ok(code) => std::process::ExitCode::from(code),
        Err(error) => {
            eprintln!("rg-studio: {error}");
            std::process::ExitCode::from(2)
        }
    }
}

fn run_gui(command: command::CommandState) -> eframe::Result {
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
        Box::new(move |creation_context| {
            theme::apply(&creation_context.egui_ctx, true);
            Ok(Box::new(app::RgStudio {
                command,
                ..app::RgStudio::default()
            }))
        }),
    )
}
