mod feature;
mod service;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    utils::env::init();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    tracing::info!("Starting Ubuntu App Rating Service");
    utils::env::print_env_if_dev();

    service::build_and_run().await;

    Ok(())
}
