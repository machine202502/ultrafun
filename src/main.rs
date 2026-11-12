mod animation;
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
mod model;
mod overlay;
mod remote_debug;
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
    if let Some(name) = validate_level_argument() {
        println!("{}", levels::validate_level(&name)?);
        return Ok(());
    }
    let screenshots = screenshot_arguments();
    let remote_debug = std::env::args().any(|argument| argument == "--remote-debug");
    let make_previews = std::env::args().any(|argument| argument == "--make-previews");
    let mut app = app::App::new(screenshots, remote_debug, make_previews)?;
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn validate_level_argument() -> Option<String> {
    let mut arguments = std::env::args().skip_while(|argument| argument != "--validate-level");
    arguments.next()?;
    arguments.next()
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
