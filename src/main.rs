mod app;
mod assets;
mod audio;
mod dispatcher;
mod fx;
mod input;
mod level_loader;
mod levels;
mod lighting_lab;
mod menu;
mod overlay;
mod render;
mod screenshots;
mod settings;
mod text;
mod ui;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    )
    .init();
    let screenshots = screenshot_arguments();
    let mut app = app::App::new(screenshots)?;
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn screenshot_arguments() -> Option<screenshots::ScreenshotArgs> {
    let mut arguments = std::env::args().skip_while(|argument| argument != "--screenshots");
    arguments.next()?;
    let mut values = arguments.filter(|argument| !argument.starts_with("--"));
    Some(screenshots::ScreenshotArgs {
        output: values
            .next()
            .unwrap_or_else(|| "target/screenshots".into())
            .into(),
        preset: values.next(),
        level: values.next(),
        shot: values.next(),
    })
}
