mod app;
mod assets;
mod audio;
mod dispatcher;
mod fx;
mod input;
mod level_loader;
mod levels;
mod lighting_lab;
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
    let screenshot_output = screenshot_output_from_arguments();
    let mut app = app::App::new(screenshot_output)?;
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn screenshot_output_from_arguments() -> Option<std::path::PathBuf> {
    let mut arguments = std::env::args().skip_while(|argument| argument != "--screenshots");
    arguments.next()?;
    Some(
        arguments
            .next()
            .unwrap_or_else(|| "target/screenshots".into())
            .into(),
    )
}
