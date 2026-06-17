use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::bootstrap::env::load_dotenv;

pub async fn connect_from_env() -> Result<PgPool, sqlx::Error> {
    load_dotenv();
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url())
        .await
}

fn database_url() -> String {
    if let Ok(url) = std::env::var("DATABASE_URL") {
        return url;
    }

    let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = std::env::var("POSTGRES_USER").expect("POSTGRES_USER must be set");
    let password = std::env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set");
    let db = std::env::var("POSTGRES_DB").expect("POSTGRES_DB must be set");

    format!("postgres://{user}:{password}@{host}:{port}/{db}")
}
