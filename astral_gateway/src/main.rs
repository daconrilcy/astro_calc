use astral_gateway::{serve, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,astral_gateway=debug".into()),
        )
        .init();
    serve(AppConfig::from_env()).await
}
