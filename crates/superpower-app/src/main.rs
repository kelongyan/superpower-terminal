fn main() {
    tracing_subscriber::fmt::init();

    let config = superpower_app::config::Config::load_from_file(
        &superpower_app::config::Config::config_path(),
    );

    tracing::info!("SuperPower Terminal starting...");
    tracing::info!(
        "Shell: {}, Font: {} ({})",
        config.shell.program,
        config.font.family,
        config.font.size
    );

    superpower_app::event_loop::run();
}
