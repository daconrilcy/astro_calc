//! Serveur HTTP du moteur de calcul astral.

pub mod app;
pub mod auth;
pub mod config;
pub mod error;
pub mod reference_status;
pub mod routes;
pub mod schema_registry;
pub mod state;

pub use app::build_app;
pub use config::AppConfig;
pub use routes::{router, serve};
