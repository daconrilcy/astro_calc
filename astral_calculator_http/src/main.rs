use astral_calculator_http::{serve, AppConfig};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,astral_calculator_http=debug".into()),
        )
        .init();

    let config = AppConfig::from_env();
    if let Err(err) = serve(config).await {
        eprintln!("astral_calculator_http failed: {err}");
        std::process::exit(1);
    }
}
